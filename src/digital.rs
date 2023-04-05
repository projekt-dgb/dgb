use crate::{python::PyVm, AnpassungSeite, Konfiguration, PdfFile, Rect};
use chrono::{DateTime, Utc};
use image::ImageError;
use lopdf::content::Operation;
use lopdf::Error as LoPdfError;
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::io::Error as IoError;
use std::{fmt, fs};
use wry::webview::WebView;

pub struct PdfSpace;
pub type PdfPoint = euclid::Point2D<f32, PdfSpace>;
pub type PdfMatrix = euclid::Transform2D<f32, PdfSpace, PdfSpace>;

/// Alle Fehler, die im Programm passieren können
#[derive(Debug)]
pub enum Fehler {
    // Seite X kann mit pdftotext nicht gelesen werden
    FalscheSeitenZahl(u32),
    // Kann Seite X nicht klassifizieren
    UnbekannterSeitentyp,
    // Fehler beim Auslesen des Titelblatts
    Titelblatt(TitelblattFehler),
    // Datei ist kein PDF
    Pdf(LoPdfError),
    // Fehler bei Bildbearbeitung
    Bild(String, ImageError),
    // Fehler bei Lese- / Schreibvorgang
    Io(String, IoError), // String = FilePath
    // Ungültiges hOCR Format
    HocrUngueltig(String, &'static str),
}

impl From<LoPdfError> for Fehler {
    fn from(e: LoPdfError) -> Fehler {
        Fehler::Pdf(e)
    }
}

impl From<TitelblattFehler> for Fehler {
    fn from(e: TitelblattFehler) -> Fehler {
        Fehler::Titelblatt(e)
    }
}

impl fmt::Display for Fehler {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Fehler::FalscheSeitenZahl(seite) => {
                write!(f, "Seite {} kann mit pdftotext nicht gelesen werden", seite)
            }
            Fehler::UnbekannterSeitentyp => {
                write!(f, "Kann Seite nicht klassifizieren")
            }
            Fehler::Titelblatt(e) => write!(f, "Fehler beim Auslesen des Titelblatts: {}", e),
            Fehler::Pdf(e) => write!(f, "Fehler im PDF: {}", e),
            Fehler::Bild(pfad, e) => write!(f, "Bildbearbeitungsfehler: \"{}\": {}", pfad, e),
            Fehler::Io(pfad, e) => write!(
                f,
                "Fehler beim Lesen / Schreiben vom Pfad \"{}\": {}",
                pfad, e
            ),
            Fehler::HocrUngueltig(hocr, e) => write!(f, "HOCR ungueltig:\r\n{}\r\n:{}", hocr, e),
        }
    }
}

// Funktion, die prüft, ob die Eingabedatei ein PDF ist und die Seitenzahlen zurückgibt
pub fn lese_seitenzahlen(pdf_bytes: &[u8]) -> Result<Vec<u32>, Fehler> {
    let pdf = lopdf::Document::load_mem(pdf_bytes)?;
    Ok(pdf.get_pages().keys().copied().collect())
}

pub fn get_seiten_dimensionen(pdf_bytes: &[u8]) -> Result<BTreeMap<u32, (f32, f32)>, Fehler> {
    const PT_TO_MM: f32 = 2.835;

    let pdf = lopdf::Document::load_mem(pdf_bytes)?;
    let pages = pdf
        .get_pages()
        .into_iter()
        .filter_map(|(page_num, page_obj)| {
            let dict = pdf.get_dictionary(page_obj).ok()?;
            let contents = dict.get(b"MediaBox").ok()?;
            let array = contents.as_array().ok()?;

            let width = if let Ok(o) = array[2].as_f64() {
                o as f32
            } else if let Ok(o) = array[2].as_i64() {
                o as f32
            } else {
                return None;
            } / PT_TO_MM;

            let height = if let Ok(o) = array[3].as_f64() {
                o as f32
            } else if let Ok(o) = array[3].as_i64() {
                o as f32
            } else {
                return None;
            } / PT_TO_MM;

            Some((page_num, (width, height)))
        })
        .collect();

    Ok(pages)
}

#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct Titelblatt {
    pub amtsgericht: String,
    pub grundbuch_von: String,
    pub blatt: usize,
}

#[derive(Debug, Copy, Clone)]
pub enum TitelblattFehler {
    KeinAmtsgericht,
    KeinGbBezirk,
    KeinGbBlatt,
}

impl fmt::Display for TitelblattFehler {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TitelblattFehler::KeinAmtsgericht => write!(f, "Kein Amtsgericht auf Titelblatt!"),
            TitelblattFehler::KeinGbBezirk => write!(f, "Kein \"Grundbuch von\" auf Titelblatt!"),
            TitelblattFehler::KeinGbBlatt => write!(f, "Kein Grundbuchblattnummer auf Titelblatt!"),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HocrLayout {
    #[serde(default)]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub seiten: BTreeMap<String, HocrSeite>,
}

impl HocrLayout {
    pub fn init_from_dimensionen(dim: &BTreeMap<u32, (f32, f32)>) -> Self {
        Self {
            seiten: dim
                .iter()
                .map(|(s, dim)| {
                    (
                        s.to_string(),
                        HocrSeite {
                            breite_mm: dim.0,
                            hoehe_mm: dim.1,
                            parsed: ParsedHocr {
                                bounds: Rect {
                                    min_x: 0.0,
                                    min_y: 0.0,
                                    max_x: dim.0,
                                    max_y: dim.1,
                                },
                                careas: Vec::new(),
                            },
                            rote_linien: Vec::new(),
                        },
                    )
                })
                .collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.seiten.is_empty()
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct HocrSeite {
    pub breite_mm: f32,
    pub hoehe_mm: f32,
    pub parsed: ParsedHocr,
    #[serde(default)]
    pub rote_linien: Vec<Linie>,
}

#[derive(Debug, Clone)]
pub struct ParsedLinie {
    pub punkte_point: Vec<Punkt>,
    pub ctm_transforms: Vec<PdfMatrix>,
    pub page_height_mm: f32,
}

impl ParsedLinie {
    pub fn get_linie(&self) -> Linie {
        let punkte = self
            .punkte_point
            .iter()
            .map(|p| {
                let mut p = PdfPoint::new(p.x, p.y);
                for c in self.ctm_transforms.iter() {
                    p = c.transform_point(p);
                }
                Punkt {
                    x: p.x * 0.352778,
                    y: self.page_height_mm - p.y * 0.352778,
                }
            })
            .collect();
        Linie { punkte }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Linie {
    pub punkte: Vec<Punkt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Punkt {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeiteParsed {
    pub typ: SeitenTyp,
    pub texte: Vec<Vec<Textblock>>,
}

impl HocrSeite {
    pub fn get_textbloecke(
        &self,
        seite: &str,
        seiten_typ: SeitenTyp,
        anpassungen_seite: &BTreeMap<String, AnpassungSeite>,
    ) -> SeiteParsed {
        let spalten = seiten_typ.get_columns(anpassungen_seite.get(seite));
        let zeilen = anpassungen_seite
            .get(seite)
            .map(|ap| ap.get_zeilen())
            .unwrap_or_default();

        let map = &[
            ('⁰', '0'),
            ('¹', '1'),
            ('²', '2'),
            ('³', '3'),
            ('⁴', '4'),
            ('⁵', '5'),
            ('⁶', '6'),
            ('⁷', '7'),
            ('⁸', '8'),
            ('⁹', '9'),
            ('ı', '1'),
            ('₀', '0'),
            ('₁', '1'),
            ('₂', '2'),
            ('₃', '3'),
            ('₄', '4'),
            ('₅', '5'),
            ('₆', '6'),
            ('₇', '7'),
            ('₈', '8'),
            ('₉', '9'),
        ];
        let transform_map = map.iter().copied().collect::<BTreeMap<_, _>>();

        let allowed_chars = &[
            '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', '.', '-', ' ',
        ];
        let allowed_chars = allowed_chars.iter().copied().collect::<BTreeSet<_>>();

        let mut texte: Vec<Vec<Textblock>> = spalten
            .iter()
            .enumerate()
            .map(|(col_idx, col)| {
                let mut zeilen = zeilen.values().copied().collect::<Vec<_>>();
                zeilen.push(col.max_y);
                zeilen.sort_by(|a, b| a.partial_cmp(&b).unwrap_or(core::cmp::Ordering::Equal));
                zeilen
                    .iter()
                    .enumerate()
                    .filter_map(|(zeile_id, max_y)| {
                        let min_y = if zeile_id == 0 {
                            col.min_y
                        } else {
                            zeilen.get(zeile_id - 1).copied()?
                        };

                        let select_rect = Rect {
                            min_x: col.min_x,
                            max_x: col.max_x,
                            min_y,
                            max_y: *max_y,
                        };
                        let zeilen = self.get_words_within_bounds(&select_rect);
                        let mut text = zeilen
                            .join("\r\n")
                            .trim()
                            .to_string()
                            .trim_end_matches('|')
                            .trim_start_matches('|')
                            .replace(" | ", " ")
                            .replace("[ ", " ")
                            .replace("] ", " ")
                            .trim()
                            .to_string();

                        if col.is_number_column {
                            text = text
                                .chars()
                                .filter_map(|mut c| {
                                    if let Some(transform) = transform_map.get(&c) {
                                        c = *transform;
                                    }
                                    if c == ',' {
                                        c = '.'
                                    }

                                    if allowed_chars.contains(&c) {
                                        Some(c)
                                    } else {
                                        None
                                    }
                                })
                                .collect()
                        }
                        Some(Textblock {
                            text,
                            start_x: col.min_x,
                            end_x: col.max_x,
                            start_y: min_y,
                            end_y: *max_y,
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        // Leere Zeilen ignorieren
        let zellen_leer =
            texte
                .iter()
                .flat_map(|sp| {
                    sp.iter().enumerate().filter_map(|(zeile, zelle)| {
                        if zelle.ist_leer() {
                            Some(zeile)
                        } else {
                            None
                        }
                    })
                })
                .collect::<BTreeSet<_>>();

        let zeilen_leer = zellen_leer
            .iter()
            .filter_map(|z| {
                if texte.iter().all(|spalte| {
                    spalte
                        .get(*z)
                        .map(|zelle| zelle.ist_leer())
                        .unwrap_or(false)
                }) {
                    Some(z)
                } else {
                    None
                }
            })
            .collect::<BTreeSet<_>>();

        for spalte in texte.iter_mut() {
            *spalte = spalte
                .clone()
                .into_iter()
                .enumerate()
                .filter_map(|(i, s)| {
                    if zeilen_leer.contains(&i) {
                        None
                    } else {
                        Some(s)
                    }
                })
                .collect();
        }

        SeiteParsed {
            typ: seiten_typ,
            texte,
        }
    }

    pub fn overlaps_any_word(&self, rect: &Rect) -> bool {
        let self_width_mm = self.breite_mm;
        let self_height_mm = self.hoehe_mm;
        let self_width_px = self.parsed.bounds.max_x;
        let self_height_px = self.parsed.bounds.max_y;

        let rect_projected_into_px = Rect {
            min_x: rect.min_x / self_width_mm * self_width_px,
            min_y: rect.min_y / self_height_mm * self_height_px,
            max_x: rect.max_x / self_width_mm * self_width_px,
            max_y: rect.max_y / self_height_mm * self_height_px,
        };

        self.parsed.careas.iter().any(|ca| {
            ca.paragraphs.iter().any(|pa| {
                pa.lines.iter().any(|li| {
                    li.words
                        .iter()
                        .any(|wo| wo.bounds.overlaps(&rect_projected_into_px))
                })
            })
        })
    }

    pub fn ist_eintrag_geroetet(&self, rect: &Rect) -> bool {
        false // TODO
    }

    pub fn get_words_within_bounds(&self, rect: &Rect) -> Vec<String> {
        let mut zeilen = Vec::new();

        let self_width_mm = self.breite_mm;
        let self_height_mm = self.hoehe_mm;
        let self_width_px = self.parsed.bounds.max_x;
        let self_height_px = self.parsed.bounds.max_y;

        let rect_projected_into_px = Rect {
            min_x: rect.min_x / self_width_mm * self_width_px,
            min_y: rect.min_y / self_height_mm * self_height_px,
            max_x: rect.max_x / self_width_mm * self_width_px,
            max_y: rect.max_y / self_height_mm * self_height_px,
        };

        for ca in self.parsed.careas.iter() {
            for pa in ca.paragraphs.iter() {
                if !pa.bounds.overlaps(&rect_projected_into_px) {
                    continue;
                }

                for li in pa.lines.iter() {
                    let mut include_word = false;
                    let mut words_until_max_x = Vec::new();

                    for w in li.words.iter() {
                        if !w.bounds.overlaps(&rect_projected_into_px) {
                            continue;
                        }
                        words_until_max_x.push(w.text.clone());
                    }

                    zeilen.push(words_until_max_x.join(" "));
                }

                zeilen.push(String::new());
            }
        }

        if zeilen.last().cloned() == Some(String::new()) {
            zeilen.pop();
        }

        zeilen
    }
}

#[derive(Default)]
struct GraphicsState {
    ctms: Vec<PdfMatrix>,
    children: Vec<GraphicsState>,
}

pub fn get_rote_linien(pdf_bytes: &[u8]) -> Result<BTreeMap<String, Vec<Linie>>, Fehler> {
    let seiten_dimensionen = get_seiten_dimensionen(pdf_bytes)?;
    let pdf = lopdf::Document::load_mem(pdf_bytes)?;
    let result = Ok(pdf
        .get_pages()
        .into_iter()
        .filter_map(|(page_num, page_obj)| {
            let (_breite_mm, hoehe_mm) = seiten_dimensionen.get(&page_num)?;
            let content = pdf.get_and_decode_page_content(page_obj).ok()?;
            let mut operations_reverse = content.operations.clone();
            operations_reverse.reverse();

            let line_end_operation_indexes = content
                .operations
                .iter()
                .enumerate()
                .filter_map(|(idx, op)| {
                    if op.operator == OP_PATH_PAINT_STROKE {
                        Some(idx)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            let cm_operations = line_end_operation_indexes
                .iter()
                .enumerate()
                .filter_map(|(line_idx, end_idx)| {
                    let mut ctm = PdfMatrix::identity();
                    let mut graphics_states = Vec::new();

                    for op in &content.operations[..*end_idx] {
                        match op.operator.as_str() {
                            "cm" => {
                                let cm0 = op.operands.get(0)?;
                                let cm1 = op.operands.get(1)?;
                                let cm2 = op.operands.get(2)?;
                                let cm3 = op.operands.get(3)?;
                                let cm4 = op.operands.get(4)?;
                                let cm5 = op.operands.get(5)?;

                                let cm0 = (cm0
                                    .as_f64()
                                    .ok()
                                    .or_else(|| cm0.as_i64().ok().map(|r| r as f64))?)
                                    as f32;
                                let cm1 = (cm1
                                    .as_f64()
                                    .ok()
                                    .or_else(|| cm1.as_i64().ok().map(|r| r as f64))?)
                                    as f32;
                                let cm2 = (cm2
                                    .as_f64()
                                    .ok()
                                    .or_else(|| cm2.as_i64().ok().map(|r| r as f64))?)
                                    as f32;
                                let cm3 = (cm3
                                    .as_f64()
                                    .ok()
                                    .or_else(|| cm3.as_i64().ok().map(|r| r as f64))?)
                                    as f32;
                                let cm4 = (cm4
                                    .as_f64()
                                    .ok()
                                    .or_else(|| cm4.as_i64().ok().map(|r| r as f64))?)
                                    as f32;
                                let cm5 = (cm5
                                    .as_f64()
                                    .ok()
                                    .or_else(|| cm5.as_i64().ok().map(|r| r as f64))?)
                                    as f32;

                                ctm = PdfMatrix::new(cm0, cm1, cm2, cm3, cm4, cm5).then(&ctm);
                            }
                            "q" => {
                                graphics_states.push(ctm.clone());
                            }
                            "Q" => {
                                ctm = graphics_states.pop().unwrap_or(PdfMatrix::identity());
                            }
                            _ => {}
                        }
                    }

                    Some((line_idx, vec![ctm]))
                })
                .collect::<BTreeMap<_, _>>();

            let linien = line_end_operation_indexes
                .iter()
                .enumerate()
                .filter_map(|(line_idx, end_idx)| {
                    let last_m_operator_idx_reverse = operations_reverse
                        .iter()
                        .enumerate()
                        .skip(operations_reverse.len().saturating_sub(*end_idx))
                        .find_map(|(op_reverse_idx, op)| {
                            if op.operator == "m" {
                                Some(op_reverse_idx)
                            } else {
                                None
                            }
                        })?;

                    let points = content.operations[(content.operations.len()
                        - last_m_operator_idx_reverse)
                        .saturating_sub(1)
                        ..*end_idx]
                        .iter()
                        .filter_map(|op| match op.operator.as_str() {
                            "m" => Some((op.operands.get(0)?, op.operands.get(1)?)),
                            "l" => Some((op.operands.get(0)?, op.operands.get(1)?)),
                            _ => None,
                        })
                        .filter_map(|(x, y)| {
                            // pt to mm
                            let x = (x
                                .as_f64()
                                .ok()
                                .or_else(|| x.as_i64().ok().map(|r| r as f64))?)
                                as f32;
                            let y = (y
                                .as_f64()
                                .ok()
                                .or_else(|| y.as_i64().ok().map(|r| r as f64))?)
                                as f32;
                            Some(Punkt { x, y })
                        })
                        .collect::<Vec<_>>();

                    if points.is_empty() {
                        None
                    } else {
                        Some(ParsedLinie {
                            punkte_point: points,
                            ctm_transforms: cm_operations
                                .get(&line_idx)
                                .cloned()
                                .unwrap_or_default(),
                            page_height_mm: *hoehe_mm,
                        })
                    }
                })
                .collect::<Vec<_>>();

            let linien = linien
                .into_iter()
                .map(|l| l.get_linie())
                .collect::<Vec<_>>();

            Some((page_num.to_string(), linien))
        })
        .collect());

    result
}

// Funktion, die das Titelblatt ausliest
pub fn lese_titelblatt(pdf_bytes: &[u8]) -> Result<Titelblatt, Fehler> {
    return Ok(Titelblatt {
        amtsgericht: "Oranienburg".to_string(),
        grundbuch_von: "Vehlefanz".to_string(),
        blatt: 456,
    });
    /*
    let temp_dir = std::env::temp_dir();
    let _ = fs::create_dir_all(temp_dir.clone())
        .map_err(|e| Fehler::Io(format!("{}", temp_dir.clone().display()), e))?;

    let temp_pdf_path = temp_dir.clone().join("temp.pdf");
    let pdftotxt_output_path = temp_dir.clone().join(format!("pdftotext-01.txt"));

    // Blah.pdf -> /tmp/temp.pdf
    let _ = fs::remove_file(temp_pdf_path.clone());
    fs::write(temp_pdf_path.clone(), pdf_bytes)
        .map_err(|e| Fehler::Io(format!("{}", temp_pdf_path.display()), e))?;

    // pdftotext -q -layout -enc UTF-8 -eol unix -nopgbrk -f 1 -l 1 /tmp/temp.pdf /pdftotxt-1.txt
    let _ = get_pdftotext_command()
    .arg("-q")
    .arg("-layout")
    .arg("-enc")
    .arg("UTF-8")
    .arg("-eol")
    .arg("unix")
    .arg("-nopgbrk")
    .arg("-f")
    .arg(&format!("1"))
    .arg("-l")
    .arg(&format!("1"))
    .arg(&format!("{}", temp_pdf_path.display()))
    .arg(&format!("{}", temp_dir.clone().join(format!("pdftotext-01.txt")).display()))
    .status();

    let text_pdftotext = fs::read_to_string(pdftotxt_output_path.clone())
    .map_err(|e| Fehler::Io(format!("{}", pdftotxt_output_path.display()), e))?;

    // Remove artifacts
    let _ = fs::remove_file(pdftotxt_output_path.clone());
    let _ = fs::remove_file(temp_pdf_path.clone());

    let mut zeilen_erste_seite = text_pdftotext
        .lines()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    zeilen_erste_seite.retain(|l| !({
        l.contains("zur Fortführung auf EDV") ||
        l.contains("dabei an die Stelle des bisherigen") ||
        l.contains("Blatt enthaltene Rötungen") ||
        l.contains("Freigegeben am") ||
        l.contains("Geändert am ") ||
        l.trim().is_empty()
    }));

    let titelblatt = zeilen_erste_seite.join(" ");

    let mut titelblatt_iter = titelblatt.split_whitespace();

    let amtsgericht = titelblatt_iter.next().ok_or(TitelblattFehler::KeinAmtsgericht)?;
    let grundbuch_von = titelblatt_iter.next().ok_or(TitelblattFehler::KeinGbBezirk)?;
    let blatt = titelblatt_iter.next().and_then(|p| p.parse::<usize>().ok()).ok_or(TitelblattFehler::KeinGbBlatt)?;

    Ok(Titelblatt {
        amtsgericht: amtsgericht.to_string(),
        grundbuch_von: grundbuch_von.to_string(),
        blatt,
    })
    */
}

pub fn konvertiere_pdf_seite_zu_png_prioritaet(
    webview: &WebView,
    pdf_bytes: &[u8],
    seite: u32,
    titelblatt: &Titelblatt,
    geroetet: bool,
) -> Result<(), Fehler> {
    let temp_ordner = std::env::temp_dir()
        .join(&titelblatt.grundbuch_von)
        .join(&titelblatt.blatt.to_string());

    let _ = fs::create_dir_all(temp_ordner.clone())
        .map_err(|e| Fehler::Io(format!("{}", temp_ordner.clone().display()), e))?;

    let mut pdftoppm_output_path = format!("page-{}.png", seite);

    if !geroetet {
        pdftoppm_output_path = format!("page-clean-{}.png", seite);
    }

    let mut pdf_bytes = pdf_bytes.to_vec();
    if !geroetet {
        pdf_bytes = clean_pdf_bytes(&pdf_bytes)?;
    }

    if temp_ordner.join(&pdftoppm_output_path).exists() {
        return Ok(());
    }

    let pdf_base64 = base64::encode(pdf_bytes);
    let pdf_amtsgericht = &titelblatt.amtsgericht;
    let pdf_grundbuch_von = &titelblatt.grundbuch_von;
    let pdf_blatt = titelblatt.blatt;

    let _ = webview.evaluate_script(&format!("renderPdfPage(`{pdf_base64}`, `{pdf_amtsgericht}`, `{pdf_grundbuch_von}`, `{pdf_blatt}`, {seite}, {geroetet:?}, `{pdftoppm_output_path}`)"));

    Ok(())
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SeitenTyp {
    #[serde(rename = "bv-horz")]
    BestandsverzeichnisHorz,
    #[serde(rename = "bv-horz-zu-und-abschreibungen")]
    BestandsverzeichnisHorzZuUndAbschreibungen,
    #[serde(rename = "bv-vert")]
    BestandsverzeichnisVert,
    #[serde(rename = "bv-vert-typ2")]
    BestandsverzeichnisVertTyp2,
    #[serde(rename = "bv-vert-zu-und-abschreibungen")]
    BestandsverzeichnisVertZuUndAbschreibungen,
    #[serde(rename = "bv-vert-zu-und-abschreibungen-alt")]
    BestandsverzeichnisVertZuUndAbschreibungenAlt,

    #[serde(rename = "abt1-horz")]
    Abt1Horz,
    #[serde(rename = "abt1-vert")]
    Abt1Vert,
    #[serde(rename = "abt1-vert-typ2")]
    Abt1VertTyp2,

    #[serde(rename = "abt2-horz-veraenderungen")]
    Abt2HorzVeraenderungen,
    #[serde(rename = "abt2-horz")]
    Abt2Horz,
    #[serde(rename = "abt2-vert-veraenderungen")]
    Abt2VertVeraenderungen,
    #[serde(rename = "abt2-vert")]
    Abt2Vert,
    #[serde(rename = "abt2-vert-typ2")]
    Abt2VertTyp2,

    #[serde(rename = "abt3-horz-veraenderungen-loeschungen")]
    Abt3HorzVeraenderungenLoeschungen,
    #[serde(rename = "abt3-vert-veraenderungen-loeschungen")]
    Abt3VertVeraenderungenLoeschungen,
    #[serde(rename = "abt3-horz")]
    Abt3Horz,
    #[serde(rename = "abt3-vert-veraenderungen")]
    Abt3VertVeraenderungen,
    #[serde(rename = "abt3-vert-loeschungen")]
    Abt3VertLoeschungen,
    #[serde(rename = "abt3-vert")]
    Abt3Vert,
}

pub fn insert_zeilen_automatisch(file: &mut PdfFile) {
    for aps in file.anpassungen_seite.values_mut() {
        aps.zeilen_auto.clear();
    }

    for (seiten_id, seite) in file.hocr.seiten.iter() {
        let seite_height_mm = seite.hoehe_mm.ceil() as usize;
        let seite_breite_mm = seite.breite_mm.ceil() as usize;

        let seitentyp = match file.get_seiten_typ(seiten_id) {
            Some(s) => s,
            None => continue,
        };

        let columns = seitentyp.get_columns(file.anpassungen_seite.get(seiten_id));

        let min_y_mm = columns
            .iter()
            .map(|col| col.min_y.floor() as usize)
            .min()
            .unwrap_or(0);
        let max_y_mm = columns
            .iter()
            .map(|col| col.max_y.ceil() as usize)
            .max()
            .unwrap_or(seite_height_mm);
        let min_x_mm = columns
            .iter()
            .map(|col| col.min_x.floor() as usize)
            .min()
            .unwrap_or(0);
        let max_x_mm = columns
            .iter()
            .map(|col| col.max_x.ceil() as usize)
            .max()
            .unwrap_or(seite_breite_mm);

        let mut has_hit_element = false;
        let mut min_y_mm = min_y_mm;

        let step = 1;

        while min_y_mm < max_y_mm {
            let rect = Rect {
                min_y: min_y_mm as f32,
                max_y: (min_y_mm + step) as f32,
                min_x: min_x_mm as f32,
                max_x: max_x_mm as f32,
            };

            let rect2 = Rect {
                min_y: (min_y_mm + step) as f32,
                max_y: (min_y_mm + step + step) as f32,
                min_x: min_x_mm as f32,
                max_x: max_x_mm as f32,
            };

            let no_words_in_rect =
                !seite.overlaps_any_word(&rect) && !seite.overlaps_any_word(&rect2);

            if !has_hit_element && !no_words_in_rect {
                has_hit_element = true;
            } else if has_hit_element && no_words_in_rect {
                let mut entry = file
                    .anpassungen_seite
                    .entry(seiten_id.clone())
                    .or_insert_with(|| AnpassungSeite::default());

                entry
                    .zeilen_auto
                    .insert(rand::random(), (min_y_mm + step) as f32);
                has_hit_element = false;
            }

            min_y_mm += step;
        }
    }
}

// Bestimmt den Seitentyp anhand des OCR-Textes der gesamten Seite
pub fn klassifiziere_seitentyp(hocr: &HocrSeite, querformat: bool) -> Result<SeitenTyp, Fehler> {
    let ocr_text = hocr.parsed.get_zeilen().join("\r\n");
    let rect = Rect {
        min_x: (50.0 / 1194.0 * 210.0),
        max_x: (350.0 / 1194.0 * 210.0),
        min_y: (200.0 / 1689.0 * 297.0),
        max_y: (300.0 / 1689.0 * 297.0),
    };
    if ocr_text.contains("Dritte Abteilung")
        || ocr_text.contains("Dritte Abteilu ng")
        || ocr_text.contains("Abteilung 3")
        || ocr_text.contains("Hypothek")
        || ocr_text.contains("Grundschuld")
        || ocr_text.contains("Rentenschuld")
        || ocr_text.contains("Abteilung ||I   ")
        || ocr_text.contains("Abteilung Ill   ")
        || ocr_text.contains("Abteilung IIl   ")
        || ocr_text.contains("Abteilung III   ")
    {
        if querformat {
            if ocr_text.contains("Veränderungen") || ocr_text.contains("Löschungen") {
                Ok(SeitenTyp::Abt3HorzVeraenderungenLoeschungen)
            } else {
                Ok(SeitenTyp::Abt3Horz)
            }
        } else {
            if ocr_text.contains("Veränderungen") {
                Ok(SeitenTyp::Abt3VertVeraenderungen)
            } else if ocr_text.contains("Löschungen") {
                Ok(SeitenTyp::Abt3VertLoeschungen)
            } else if ocr_text.contains("Spalte 1") {
                if hocr
                    .get_words_within_bounds(&rect)
                    .join(" ")
                    .trim()
                    .is_empty()
                {
                    Ok(SeitenTyp::Abt3VertLoeschungen)
                } else {
                    Ok(SeitenTyp::Abt3VertVeraenderungen)
                }
            } else {
                Ok(SeitenTyp::Abt3Vert)
            }
        }
    } else if ocr_text.contains("Zweite Abteilung")
        || ocr_text.contains("Abteilung ||")
        || ocr_text.contains("Abteilung Il")
        || ocr_text.contains("Abteilung II")
        || ocr_text.contains("Abteilung 2")
    {
        if querformat {
            if ocr_text.contains("Veränderungen")
                || ocr_text.contains("Löschungen")
                || ocr_text.contains("Spalte 1")
            {
                Ok(SeitenTyp::Abt2HorzVeraenderungen)
            } else {
                Ok(SeitenTyp::Abt2Horz)
            }
        } else {
            if ocr_text.contains("Veränderungen")
                || ocr_text.contains("Löschungen")
                || ocr_text.contains("Spalte 1")
            {
                Ok(SeitenTyp::Abt2VertVeraenderungen)
            } else {
                let rect = Rect {
                    min_x: 11.48188,
                    min_y: 1.8667322,
                    max_x: 67.83141,
                    max_y: 7.681686,
                };
                if hocr.overlaps_any_word(&rect) {
                    Ok(SeitenTyp::Abt2VertTyp2)
                } else {
                    Ok(SeitenTyp::Abt2Vert)
                }
            }
        }
    } else if ocr_text.contains("Erste Abteilung")
        || ocr_text.contains("Abteilung |   ")
        || ocr_text.contains("Abteilung I   ")
        || ocr_text.contains("Abteilung 1")
        || (ocr_text.contains("Eigentümer") && ocr_text.contains("Grundlage der Eintragung"))
    {
        if querformat {
            Ok(SeitenTyp::Abt1Horz)
        } else {
            let rect = Rect {
                min_x: 11.305236,
                min_y: 2.3953643,
                max_x: 68.89128,
                max_y: 7.329264,
            };
            if hocr.overlaps_any_word(&rect) {
                Ok(SeitenTyp::Abt1VertTyp2)
            } else {
                Ok(SeitenTyp::Abt1Vert)
            }
        }
    } else if ocr_text.contains("Bestandsverzeichnis")
        || ocr_text.contains("Besiandsverzeichnis")
        || ocr_text
            .contains("Bezeichnung der Grundstücke und der mit dem Eigentum verbundenen Rechte")
        || ocr_text.contains("Wirtschaftsart und Lage")
        || ocr_text.contains("Zuschreibunge")
    {
        if querformat {
            if ocr_text.contains("Abschreibungen") {
                Ok(SeitenTyp::BestandsverzeichnisHorzZuUndAbschreibungen)
            } else {
                Ok(SeitenTyp::BestandsverzeichnisHorz)
            }
        } else {
            if ocr_text.contains("Abschreibungen")
                || ocr_text.contains("Zuschreibunge")
                || (ocr_text.contains("Nr. der") && ocr_text.contains("Grund-"))
            {
                let rect = Rect {
                    min_x: 18.547653,
                    min_y: 2.2191536,
                    max_x: 75.42712,
                    max_y: 7.681686,
                };
                if hocr.overlaps_any_word(&rect) {
                    Ok(SeitenTyp::BestandsverzeichnisVertZuUndAbschreibungenAlt)
                } else {
                    Ok(SeitenTyp::BestandsverzeichnisVertZuUndAbschreibungen)
                }
            } else if ocr_text.contains("Gemarkung *") {
                Ok(SeitenTyp::BestandsverzeichnisVert)
            } else {
                Ok(SeitenTyp::BestandsverzeichnisVertTyp2)
            }
        }
    } else {
        Err(Fehler::UnbekannterSeitentyp)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Column {
    pub id: &'static str,
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
    pub is_number_column: bool,
    pub line_break_after_px: f32,
}

impl SeitenTyp {
    pub fn get_columns(&self, anpassungen_seite: Option<&AnpassungSeite>) -> Vec<Column> {
        let scale_factor = 2.85;
        match self {
            SeitenTyp::BestandsverzeichnisHorz => vec![
                // "lfd. Nr. der Grundstücke"
                Column {
                    id: "bv_horz-lfd_nr",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-lfd_nr"))
                        .map(|m| m.min_x)
                        .unwrap_or(60.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-lfd_nr"))
                        .map(|m| m.max_x)
                        .unwrap_or(95.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-lfd_nr"))
                        .map(|m| m.min_y)
                        .unwrap_or(130.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-lfd_nr"))
                        .map(|m| m.max_y)
                        .unwrap_or(565.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Bisherige lfd. Nr."
                Column {
                    id: "bv_horz-bisherige_lfd_nr",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-bisherige_lfd_nr"))
                        .map(|m| m.min_x)
                        .unwrap_or(100.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-bisherige_lfd_nr"))
                        .map(|m| m.max_x)
                        .unwrap_or(140.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-bisherige_lfd_nr"))
                        .map(|m| m.min_y)
                        .unwrap_or(130.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-bisherige_lfd_nr"))
                        .map(|m| m.max_y)
                        .unwrap_or(565.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // Gemarkung
                Column {
                    id: "bv_horz-gemarkung",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-gemarkung"))
                        .map(|m| m.min_x)
                        .unwrap_or(150.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-gemarkung"))
                        .map(|m| m.max_x)
                        .unwrap_or(255.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-gemarkung"))
                        .map(|m| m.min_y)
                        .unwrap_or(130.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-gemarkung"))
                        .map(|m| m.max_y)
                        .unwrap_or(565.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                // Flur
                Column {
                    id: "bv_horz-flur",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-flur"))
                        .map(|m| m.min_x)
                        .unwrap_or(265.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-flur"))
                        .map(|m| m.max_x)
                        .unwrap_or(300.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-flur"))
                        .map(|m| m.min_y)
                        .unwrap_or(130.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-flur"))
                        .map(|m| m.max_y)
                        .unwrap_or(565.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // Flurstück
                Column {
                    id: "bv_horz-flurstueck",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-flurstueck"))
                        .map(|m| m.min_x)
                        .unwrap_or(305.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-flurstueck"))
                        .map(|m| m.max_x)
                        .unwrap_or(370.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-flurstueck"))
                        .map(|m| m.min_y)
                        .unwrap_or(130.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-flurstueck"))
                        .map(|m| m.max_y)
                        .unwrap_or(565.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                // Wirtschaftsart und Lage
                Column {
                    id: "bv_horz-lage",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-lage"))
                        .map(|m| m.min_x)
                        .unwrap_or(375.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-lage"))
                        .map(|m| m.max_x)
                        .unwrap_or(670.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-lage"))
                        .map(|m| m.min_y)
                        .unwrap_or(130.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-lage"))
                        .map(|m| m.max_y)
                        .unwrap_or(565.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 40.0, // 10.0,
                },
                // Größe (ha)
                Column {
                    id: "bv_horz-groesse_ha",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-groesse_ha"))
                        .map(|m| m.min_x)
                        .unwrap_or(675.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-groesse_ha"))
                        .map(|m| m.max_x)
                        .unwrap_or(710.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-groesse_ha"))
                        .map(|m| m.min_y)
                        .unwrap_or(130.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-groesse_ha"))
                        .map(|m| m.max_y)
                        .unwrap_or(565.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // Größe (a)
                Column {
                    id: "bv_horz-groesse_a",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-groesse_a"))
                        .map(|m| m.min_x)
                        .unwrap_or(715.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-groesse_a"))
                        .map(|m| m.max_x)
                        .unwrap_or(735.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-groesse_a"))
                        .map(|m| m.min_y)
                        .unwrap_or(130.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-groesse_a"))
                        .map(|m| m.max_y)
                        .unwrap_or(565.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // Größe (m2)
                Column {
                    id: "bv_horz-groesse_m2",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-groesse_m2"))
                        .map(|m| m.min_x)
                        .unwrap_or(740.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-groesse_m2"))
                        .map(|m| m.max_x)
                        .unwrap_or(763.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-groesse_m2"))
                        .map(|m| m.min_y)
                        .unwrap_or(130.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz-groesse_m2"))
                        .map(|m| m.max_y)
                        .unwrap_or(565.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
            ],
            SeitenTyp::BestandsverzeichnisVert => vec![
                // "lfd. Nr. der Grundstücke"
                Column {
                    id: "bv_vert-lfd_nr",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert-lfd_nr"))
                        .map(|m| m.min_x)
                        .unwrap_or(32.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert-lfd_nr"))
                        .map(|m| m.max_x)
                        .unwrap_or(68.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert-lfd_nr"))
                        .map(|m| m.min_y)
                        .unwrap_or(150.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert-lfd_nr"))
                        .map(|m| m.max_y)
                        .unwrap_or(810.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Bisherige lfd. Nr."
                Column {
                    id: "bv_vert-bisherige_lfd_nr",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert-bisherige_lfd_nr"))
                        .map(|m| m.min_x)
                        .unwrap_or(72.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert-bisherige_lfd_nr"))
                        .map(|m| m.max_x)
                        .unwrap_or(108.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert-bisherige_lfd_nr"))
                        .map(|m| m.min_y)
                        .unwrap_or(150.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert-bisherige_lfd_nr"))
                        .map(|m| m.max_y)
                        .unwrap_or(810.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // Flur
                Column {
                    id: "bv_vert-flur",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert-flur"))
                        .map(|m| m.min_x)
                        .unwrap_or(115.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert-flur"))
                        .map(|m| m.max_x)
                        .unwrap_or(153.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert-flur"))
                        .map(|m| m.min_y)
                        .unwrap_or(150.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert-flur"))
                        .map(|m| m.max_y)
                        .unwrap_or(810.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                // Flurstück
                Column {
                    id: "bv_vert-flurstueck",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert-flurstueck"))
                        .map(|m| m.min_x)
                        .unwrap_or(157.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert-flurstueck"))
                        .map(|m| m.max_x)
                        .unwrap_or(219.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert-flurstueck"))
                        .map(|m| m.min_y)
                        .unwrap_or(150.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert-flurstueck"))
                        .map(|m| m.max_y)
                        .unwrap_or(810.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                // Wirtschaftsart und Lage
                Column {
                    id: "bv_vert-lage",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert-lage"))
                        .map(|m| m.min_x)
                        .unwrap_or(221.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert-lage"))
                        .map(|m| m.max_x)
                        .unwrap_or(500.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert-lage"))
                        .map(|m| m.min_y)
                        .unwrap_or(150.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert-lage"))
                        .map(|m| m.max_y)
                        .unwrap_or(810.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                // Größe
                Column {
                    id: "bv_vert-groesse_m2",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert-groesse_m2"))
                        .map(|m| m.min_x)
                        .unwrap_or(508.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert-groesse_m2"))
                        .map(|m| m.max_x)
                        .unwrap_or(572.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert-groesse_m2"))
                        .map(|m| m.min_y)
                        .unwrap_or(150.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert-groesse_m2"))
                        .map(|m| m.max_y)
                        .unwrap_or(810.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
            ],
            SeitenTyp::BestandsverzeichnisVertTyp2 => vec![
                // "lfd. Nr. der Grundstücke"
                Column {
                    id: "bv_vert_typ2-lfd_nr",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_typ2-lfd_nr"))
                        .map(|m| m.min_x)
                        .unwrap_or(35.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_typ2-lfd_nr"))
                        .map(|m| m.max_x)
                        .unwrap_or(72.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_typ2-lfd_nr"))
                        .map(|m| m.min_y)
                        .unwrap_or(128.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_typ2-lfd_nr"))
                        .map(|m| m.max_y)
                        .unwrap_or(785.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Bisherige lfd. Nr."
                Column {
                    id: "bv_vert_typ2-bisherige_lfd_nr",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_typ2-bisherige_lfd_nr"))
                        .map(|m| m.min_x)
                        .unwrap_or(75.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_typ2-bisherige_lfd_nr"))
                        .map(|m| m.max_x)
                        .unwrap_or(110.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_typ2-bisherige_lfd_nr"))
                        .map(|m| m.min_y)
                        .unwrap_or(128.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_typ2-bisherige_lfd_nr"))
                        .map(|m| m.max_y)
                        .unwrap_or(785.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // Gemarkung, Flur, Flurstück
                Column {
                    id: "bv_vert_typ2-gemarkung_flur_flurstueck",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_typ2-gemarkung_flur_flurstueck"))
                        .map(|m| m.min_x)
                        .unwrap_or(115.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_typ2-gemarkung_flur_flurstueck"))
                        .map(|m| m.max_x)
                        .unwrap_or(230.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_typ2-gemarkung_flur_flurstueck"))
                        .map(|m| m.min_y)
                        .unwrap_or(128.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_typ2-gemarkung_flur_flurstueck"))
                        .map(|m| m.max_y)
                        .unwrap_or(785.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                // Wirtschaftsart und Lage
                Column {
                    id: "bv_vert_typ2-lage",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_typ2-lage"))
                        .map(|m| m.min_x)
                        .unwrap_or(235.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_typ2-lage"))
                        .map(|m| m.max_x)
                        .unwrap_or(485.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_typ2-lage"))
                        .map(|m| m.min_y)
                        .unwrap_or(128.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_typ2-lage"))
                        .map(|m| m.max_y)
                        .unwrap_or(785.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                // Größe (m2)
                Column {
                    id: "bv_vert_typ2-groesse_m2",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_typ2-groesse_m2"))
                        .map(|m| m.min_x)
                        .unwrap_or(490.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_typ2-groesse_m2"))
                        .map(|m| m.max_x)
                        .unwrap_or(198.90),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_typ2-groesse_m2"))
                        .map(|m| m.min_y)
                        .unwrap_or(128.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_typ2-groesse_m2"))
                        .map(|m| m.max_y)
                        .unwrap_or(785.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
            ],
            SeitenTyp::BestandsverzeichnisHorzZuUndAbschreibungen => vec![
                // "Zur lfd. Nr. der Grundstücke"
                Column {
                    id: "bv_horz_zu_abschreibung-lfd_nr_zuschreibungen",
                    min_x: anpassungen_seite
                        .and_then(|s| {
                            s.spalten
                                .get("bv_horz_zu_abschreibung-lfd_nr_zuschreibungen")
                        })
                        .map(|m| m.min_x)
                        .unwrap_or(57.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| {
                            s.spalten
                                .get("bv_horz_zu_abschreibung-lfd_nr_zuschreibungen")
                        })
                        .map(|m| m.max_x)
                        .unwrap_or(95.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| {
                            s.spalten
                                .get("bv_horz_zu_abschreibung-lfd_nr_zuschreibungen")
                        })
                        .map(|m| m.min_y)
                        .unwrap_or(125.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| {
                            s.spalten
                                .get("bv_horz_zu_abschreibung-lfd_nr_zuschreibungen")
                        })
                        .map(|m| m.max_y)
                        .unwrap_or(560.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Bestand und Zuschreibungen"
                Column {
                    id: "bv_horz_zu_abschreibung-zuschreibungen",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz_zu_abschreibung-zuschreibungen"))
                        .map(|m| m.min_x)
                        .unwrap_or(105.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz_zu_abschreibung-zuschreibungen"))
                        .map(|m| m.max_x)
                        .unwrap_or(420.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz_zu_abschreibung-zuschreibungen"))
                        .map(|m| m.min_y)
                        .unwrap_or(125.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz_zu_abschreibung-zuschreibungen"))
                        .map(|m| m.max_y)
                        .unwrap_or(560.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Zur lfd. Nr. der Grundstücke"
                Column {
                    id: "bv_horz_zu_abschreibung-lfd_nr_abschreibungen",
                    min_x: anpassungen_seite
                        .and_then(|s| {
                            s.spalten
                                .get("bv_horz_zu_abschreibung-lfd_nr_abschreibungen")
                        })
                        .map(|m| m.min_x)
                        .unwrap_or(425.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| {
                            s.spalten
                                .get("bv_horz_zu_abschreibung-lfd_nr_abschreibungen")
                        })
                        .map(|m| m.max_x)
                        .unwrap_or(470.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| {
                            s.spalten
                                .get("bv_horz_zu_abschreibung-lfd_nr_abschreibungen")
                        })
                        .map(|m| m.min_y)
                        .unwrap_or(125.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| {
                            s.spalten
                                .get("bv_horz_zu_abschreibung-lfd_nr_abschreibungen")
                        })
                        .map(|m| m.max_y)
                        .unwrap_or(560.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Abschreibungen"
                Column {
                    id: "bv_horz_zu_abschreibung-abschreibungen",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz_zu_abschreibung-abschreibungen"))
                        .map(|m| m.min_x)
                        .unwrap_or(480.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz_zu_abschreibung-abschreibungen"))
                        .map(|m| m.max_x)
                        .unwrap_or(763.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz_zu_abschreibung-abschreibungen"))
                        .map(|m| m.min_y)
                        .unwrap_or(125.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_horz_zu_abschreibung-abschreibungen"))
                        .map(|m| m.max_y)
                        .unwrap_or(560.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
            ],
            SeitenTyp::BestandsverzeichnisVertZuUndAbschreibungenAlt => vec![
                // "Zur lfd. Nr. der Grundstücke"
                Column {
                    id: "bv_vert_zu_abschreibung-lfd_nr_zuschreibungen",
                    min_x: anpassungen_seite
                        .and_then(|s| {
                            s.spalten
                                .get("bv_vert_zu_abschreibung-lfd_nr_zuschreibungen")
                        })
                        .map(|m| m.min_x)
                        .unwrap_or(13.248323),
                    max_x: anpassungen_seite
                        .and_then(|s| {
                            s.spalten
                                .get("bv_vert_zu_abschreibung-lfd_nr_zuschreibungen")
                        })
                        .map(|m| m.max_x)
                        .unwrap_or(26.849936),
                    min_y: anpassungen_seite
                        .and_then(|s| {
                            s.spalten
                                .get("bv_vert_zu_abschreibung-lfd_nr_zuschreibungen")
                        })
                        .map(|m| m.min_y)
                        .unwrap_or(45.038353),
                    max_y: anpassungen_seite
                        .and_then(|s| {
                            s.spalten
                                .get("bv_vert_zu_abschreibung-lfd_nr_zuschreibungen")
                        })
                        .map(|m| m.max_y)
                        .unwrap_or(275.8744),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Bestand und Zuschreibungen"
                Column {
                    id: "bv_vert_zu_abschreibung-zuschreibungen",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_zu_abschreibung-zuschreibungen"))
                        .map(|m| m.min_x)
                        .unwrap_or(29.146313),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_zu_abschreibung-zuschreibungen"))
                        .map(|m| m.max_x)
                        .unwrap_or(118.17505),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_zu_abschreibung-zuschreibungen"))
                        .map(|m| m.min_y)
                        .unwrap_or(45.038353),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_zu_abschreibung-zuschreibungen"))
                        .map(|m| m.max_y)
                        .unwrap_or(275.8744),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Zur lfd. Nr. der Grundstücke"
                Column {
                    id: "bv_vert_zu_abschreibung-lfd_nr_abschreibungen",
                    min_x: anpassungen_seite
                        .and_then(|s| {
                            s.spalten
                                .get("bv_vert_zu_abschreibung-lfd_nr_abschreibungen")
                        })
                        .map(|m| m.min_x)
                        .unwrap_or(121.00135),
                    max_x: anpassungen_seite
                        .and_then(|s| {
                            s.spalten
                                .get("bv_vert_zu_abschreibung-lfd_nr_abschreibungen")
                        })
                        .map(|m| m.max_x)
                        .unwrap_or(134.7796),
                    min_y: anpassungen_seite
                        .and_then(|s| {
                            s.spalten
                                .get("bv_vert_zu_abschreibung-lfd_nr_abschreibungen")
                        })
                        .map(|m| m.min_y)
                        .unwrap_or(45.038353),
                    max_y: anpassungen_seite
                        .and_then(|s| {
                            s.spalten
                                .get("bv_vert_zu_abschreibung-lfd_nr_abschreibungen")
                        })
                        .map(|m| m.max_y)
                        .unwrap_or(275.8744),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Abschreibungen"
                Column {
                    id: "bv_vert_zu_abschreibung-abschreibungen",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_zu_abschreibung-abschreibungen"))
                        .map(|m| m.min_x)
                        .unwrap_or(136.89934),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_zu_abschreibung-abschreibungen"))
                        .map(|m| m.max_x)
                        .unwrap_or(198.37157),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_zu_abschreibung-abschreibungen"))
                        .map(|m| m.min_y)
                        .unwrap_or(45.038353),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_zu_abschreibung-abschreibungen"))
                        .map(|m| m.max_y)
                        .unwrap_or(275.8744),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
            ],
            SeitenTyp::BestandsverzeichnisVertZuUndAbschreibungen => vec![
                // "Zur lfd. Nr. der Grundstücke"
                Column {
                    id: "bv_vert_zu_abschreibung-lfd_nr_zuschreibungen",
                    min_x: anpassungen_seite
                        .and_then(|s| {
                            s.spalten
                                .get("bv_vert_zu_abschreibung-lfd_nr_zuschreibungen")
                        })
                        .map(|m| m.min_x)
                        .unwrap_or(35.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| {
                            s.spalten
                                .get("bv_vert_zu_abschreibung-lfd_nr_zuschreibungen")
                        })
                        .map(|m| m.max_x)
                        .unwrap_or(72.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| {
                            s.spalten
                                .get("bv_vert_zu_abschreibung-lfd_nr_zuschreibungen")
                        })
                        .map(|m| m.min_y)
                        .unwrap_or(150.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| {
                            s.spalten
                                .get("bv_vert_zu_abschreibung-lfd_nr_zuschreibungen")
                        })
                        .map(|m| m.max_y)
                        .unwrap_or(810.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Bestand und Zuschreibungen"
                Column {
                    id: "bv_vert_zu_abschreibung-zuschreibungen",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_zu_abschreibung-zuschreibungen"))
                        .map(|m| m.min_x)
                        .unwrap_or(78.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_zu_abschreibung-zuschreibungen"))
                        .map(|m| m.max_x)
                        .unwrap_or(330.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_zu_abschreibung-zuschreibungen"))
                        .map(|m| m.min_y)
                        .unwrap_or(150.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_zu_abschreibung-zuschreibungen"))
                        .map(|m| m.max_y)
                        .unwrap_or(810.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Zur lfd. Nr. der Grundstücke"
                Column {
                    id: "bv_vert_zu_abschreibung-lfd_nr_abschreibungen",
                    min_x: anpassungen_seite
                        .and_then(|s| {
                            s.spalten
                                .get("bv_vert_zu_abschreibung-lfd_nr_abschreibungen")
                        })
                        .map(|m| m.min_x)
                        .unwrap_or(337.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| {
                            s.spalten
                                .get("bv_vert_zu_abschreibung-lfd_nr_abschreibungen")
                        })
                        .map(|m| m.max_x)
                        .unwrap_or(375.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| {
                            s.spalten
                                .get("bv_vert_zu_abschreibung-lfd_nr_abschreibungen")
                        })
                        .map(|m| m.min_y)
                        .unwrap_or(150.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| {
                            s.spalten
                                .get("bv_vert_zu_abschreibung-lfd_nr_abschreibungen")
                        })
                        .map(|m| m.max_y)
                        .unwrap_or(810.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Abschreibungen"
                Column {
                    id: "bv_vert_zu_abschreibung-abschreibungen",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_zu_abschreibung-abschreibungen"))
                        .map(|m| m.min_x)
                        .unwrap_or(382.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_zu_abschreibung-abschreibungen"))
                        .map(|m| m.max_x)
                        .unwrap_or(573.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_zu_abschreibung-abschreibungen"))
                        .map(|m| m.min_y)
                        .unwrap_or(150.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("bv_vert_zu_abschreibung-abschreibungen"))
                        .map(|m| m.max_y)
                        .unwrap_or(810.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
            ],

            SeitenTyp::Abt1Horz => vec![
                // "lfd. Nr. der Eintragungen"
                Column {
                    id: "abt1_horz-lfd_nr",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_horz-lfd_nr"))
                        .map(|m| m.min_x)
                        .unwrap_or(55.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_horz-lfd_nr"))
                        .map(|m| m.max_x)
                        .unwrap_or(95.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_horz-lfd_nr"))
                        .map(|m| m.min_y)
                        .unwrap_or(130.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_horz-lfd_nr"))
                        .map(|m| m.max_y)
                        .unwrap_or(565.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Eigentümer"
                Column {
                    id: "abt1_horz-eigentuemer",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_horz-eigentuemer"))
                        .map(|m| m.min_x)
                        .unwrap_or(100.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_horz-eigentuemer"))
                        .map(|m| m.max_x)
                        .unwrap_or(405.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_horz-eigentuemer"))
                        .map(|m| m.min_y)
                        .unwrap_or(130.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_horz-eigentuemer"))
                        .map(|m| m.max_y)
                        .unwrap_or(565.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "lfd. Nr. der Grundstücke im BV"
                Column {
                    id: "abt1_horz-lfd_nr_bv",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_horz-lfd_nr_bv"))
                        .map(|m| m.min_x)
                        .unwrap_or(413.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_horz-lfd_nr_bv"))
                        .map(|m| m.max_x)
                        .unwrap_or(520.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_horz-lfd_nr_bv"))
                        .map(|m| m.min_y)
                        .unwrap_or(130.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_horz-lfd_nr_bv"))
                        .map(|m| m.max_y)
                        .unwrap_or(565.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Grundlage der Eintragung"
                Column {
                    id: "abt1_horz-grundlage_der_eintragung",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_horz-grundlage_der_eintragung"))
                        .map(|m| m.min_x)
                        .unwrap_or(525.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_horz-grundlage_der_eintragung"))
                        .map(|m| m.max_x)
                        .unwrap_or(762.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_horz-grundlage_der_eintragung"))
                        .map(|m| m.min_y)
                        .unwrap_or(130.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_horz-grundlage_der_eintragung"))
                        .map(|m| m.max_y)
                        .unwrap_or(565.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
            ],
            SeitenTyp::Abt1Vert => vec![
                // "lfd. Nr. der Eintragungen"
                Column {
                    id: "abt1_vert-lfd_nr",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_vert-lfd_nr"))
                        .map(|m| m.min_x)
                        .unwrap_or(32.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_vert-lfd_nr"))
                        .map(|m| m.max_x)
                        .unwrap_or(60.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_vert-lfd_nr"))
                        .map(|m| m.min_y)
                        .unwrap_or(150.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_vert-lfd_nr"))
                        .map(|m| m.max_y)
                        .unwrap_or(810.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Eigentümer"
                Column {
                    id: "abt1_vert-eigentuemer",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_vert-eigentuemer"))
                        .map(|m| m.min_x)
                        .unwrap_or(65.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_vert-eigentuemer"))
                        .map(|m| m.max_x)
                        .unwrap_or(290.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_vert-eigentuemer"))
                        .map(|m| m.min_y)
                        .unwrap_or(150.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_vert-eigentuemer"))
                        .map(|m| m.max_y)
                        .unwrap_or(810.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "lfd. Nr. der Grundstücke im BV"
                Column {
                    id: "abt1_vert-lfd_nr_bv",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_vert-lfd_nr_bv"))
                        .map(|m| m.min_x)
                        .unwrap_or(298.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_vert-lfd_nr_bv"))
                        .map(|m| m.max_x)
                        .unwrap_or(337.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_vert-lfd_nr_bv"))
                        .map(|m| m.min_y)
                        .unwrap_or(150.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_vert-lfd_nr_bv"))
                        .map(|m| m.max_y)
                        .unwrap_or(810.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Grundlage der Eintragung"
                Column {
                    id: "abt1_vert-grundlage_der_eintragung",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_vert-grundlage_der_eintragung"))
                        .map(|m| m.min_x)
                        .unwrap_or(343.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_vert-grundlage_der_eintragung"))
                        .map(|m| m.max_x)
                        .unwrap_or(567.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_vert-grundlage_der_eintragung"))
                        .map(|m| m.min_y)
                        .unwrap_or(150.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_vert-grundlage_der_eintragung"))
                        .map(|m| m.max_y)
                        .unwrap_or(810.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
            ],
            SeitenTyp::Abt1VertTyp2 => vec![
                // "lfd. Nr. der Eintragungen"
                Column {
                    id: "abt1_vert-lfd_nr",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_vert-lfd_nr"))
                        .map(|m| m.min_x)
                        .unwrap_or(12.365102),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_vert-lfd_nr"))
                        .map(|m| m.max_x)
                        .unwrap_or(23.140406),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_vert-lfd_nr"))
                        .map(|m| m.min_y)
                        .unwrap_or(44.685932),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_vert-lfd_nr"))
                        .map(|m| m.max_y)
                        .unwrap_or(276.0506),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Eigentümer"
                Column {
                    id: "abt1_vert-eigentuemer",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_vert-eigentuemer"))
                        .map(|m| m.min_x)
                        .unwrap_or(24.55356),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_vert-eigentuemer"))
                        .map(|m| m.max_x)
                        .unwrap_or(99.627396),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_vert-eigentuemer"))
                        .map(|m| m.min_y)
                        .unwrap_or(44.685932),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_vert-eigentuemer"))
                        .map(|m| m.max_y)
                        .unwrap_or(276.0506),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "lfd. Nr. der Grundstücke im BV"
                Column {
                    id: "abt1_vert-lfd_nr_bv",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_vert-lfd_nr_bv"))
                        .map(|m| m.min_x)
                        .unwrap_or(101.57048),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_vert-lfd_nr_bv"))
                        .map(|m| m.max_x)
                        .unwrap_or(119.41155),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_vert-lfd_nr_bv"))
                        .map(|m| m.min_y)
                        .unwrap_or(44.685932),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_vert-lfd_nr_bv"))
                        .map(|m| m.max_y)
                        .unwrap_or(276.0506),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Grundlage der Eintragung"
                Column {
                    id: "abt1_vert-grundlage_der_eintragung",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_vert-grundlage_der_eintragung"))
                        .map(|m| m.min_x)
                        .unwrap_or(121.70793),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_vert-grundlage_der_eintragung"))
                        .map(|m| m.max_x)
                        .unwrap_or(198.01828),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_vert-grundlage_der_eintragung"))
                        .map(|m| m.min_y)
                        .unwrap_or(44.685932),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt1_vert-grundlage_der_eintragung"))
                        .map(|m| m.max_y)
                        .unwrap_or(276.0506),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
            ],
            SeitenTyp::Abt2Horz => vec![
                // "lfd. Nr. der Eintragungen"
                Column {
                    id: "abt2_horz-lfd_nr",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_horz-lfd_nr"))
                        .map(|m| m.min_x)
                        .unwrap_or(55.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_horz-lfd_nr"))
                        .map(|m| m.max_x)
                        .unwrap_or(95.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_horz-lfd_nr"))
                        .map(|m| m.min_y)
                        .unwrap_or(130.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_horz-lfd_nr"))
                        .map(|m| m.max_y)
                        .unwrap_or(565.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "lfd. Nr. der Grundstücke im BV"
                Column {
                    id: "abt2_horz-lfd_nr_bv",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_horz-lfd_nr_bv"))
                        .map(|m| m.min_x)
                        .unwrap_or(103.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_horz-lfd_nr_bv"))
                        .map(|m| m.max_x)
                        .unwrap_or(192.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_horz-lfd_nr_bv"))
                        .map(|m| m.min_y)
                        .unwrap_or(130.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_horz-lfd_nr_bv"))
                        .map(|m| m.max_y)
                        .unwrap_or(565.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Lasten und Beschränkungen"
                Column {
                    id: "abt2_horz-lasten_und_beschraenkungen",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_horz-lasten_und_beschraenkungen"))
                        .map(|m| m.min_x)
                        .unwrap_or(200.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_horz-lasten_und_beschraenkungen"))
                        .map(|m| m.max_x)
                        .unwrap_or(765.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_horz-lasten_und_beschraenkungen"))
                        .map(|m| m.min_y)
                        .unwrap_or(130.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_horz-lasten_und_beschraenkungen"))
                        .map(|m| m.max_y)
                        .unwrap_or(565.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 25.0, // 10.0,
                },
            ],
            SeitenTyp::Abt2HorzVeraenderungen => vec![
                // "lfd. Nr. der Spalte 1"
                Column {
                    id: "abt2_horz_veraenderungen-lfd_nr",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_horz_veraenderungen-lfd_nr"))
                        .map(|m| m.min_x)
                        .unwrap_or(55.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_horz_veraenderungen-lfd_nr"))
                        .map(|m| m.max_x)
                        .unwrap_or(95.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_horz_veraenderungen-lfd_nr"))
                        .map(|m| m.min_y)
                        .unwrap_or(130.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_horz_veraenderungen-lfd_nr"))
                        .map(|m| m.max_y)
                        .unwrap_or(565.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Veränderungen"
                Column {
                    id: "abt2_horz_veraenderungen-veraenderungen",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_horz_veraenderungen-veraenderungen"))
                        .map(|m| m.min_x)
                        .unwrap_or(103.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_horz_veraenderungen-veraenderungen"))
                        .map(|m| m.max_x)
                        .unwrap_or(505.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_horz_veraenderungen-veraenderungen"))
                        .map(|m| m.min_y)
                        .unwrap_or(130.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_horz_veraenderungen-veraenderungen"))
                        .map(|m| m.max_y)
                        .unwrap_or(565.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "lfd. Nr. der Spalte 2"
                Column {
                    id: "abt2_horz_veraenderungen-lfd_nr_bv",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_horz_veraenderungen-lfd_nr_bv"))
                        .map(|m| m.min_x)
                        .unwrap_or(515.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_horz_veraenderungen-lfd_nr_bv"))
                        .map(|m| m.max_x)
                        .unwrap_or(552.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_horz_veraenderungen-lfd_nr_bv"))
                        .map(|m| m.min_y)
                        .unwrap_or(130.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_horz_veraenderungen-lfd_nr_bv"))
                        .map(|m| m.max_y)
                        .unwrap_or(565.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Löschungen"
                Column {
                    id: "abt2_horz_veraenderungen-loeschungen",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_horz_veraenderungen-loeschungen"))
                        .map(|m| m.min_x)
                        .unwrap_or(560.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_horz_veraenderungen-loeschungen"))
                        .map(|m| m.max_x)
                        .unwrap_or(770.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_horz_veraenderungen-loeschungen"))
                        .map(|m| m.min_y)
                        .unwrap_or(130.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_horz_veraenderungen-loeschungen"))
                        .map(|m| m.max_y)
                        .unwrap_or(565.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
            ],
            SeitenTyp::Abt2Vert => vec![
                // "lfd. Nr. der Eintragungen"
                Column {
                    id: "abt2_vert-lfd_nr",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert-lfd_nr"))
                        .map(|m| m.min_x)
                        .unwrap_or(32.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert-lfd_nr"))
                        .map(|m| m.max_x)
                        .unwrap_or(60.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert-lfd_nr"))
                        .map(|m| m.min_y)
                        .unwrap_or(150.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert-lfd_nr"))
                        .map(|m| m.max_y)
                        .unwrap_or(810.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "lfd. Nr der betroffenen Grundstücke"
                Column {
                    id: "abt2_vert-lfd_nr_bv",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert-lfd_nr_bv"))
                        .map(|m| m.min_x)
                        .unwrap_or(65.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert-lfd_nr_bv"))
                        .map(|m| m.max_x)
                        .unwrap_or(105.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert-lfd_nr_bv"))
                        .map(|m| m.min_y)
                        .unwrap_or(150.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert-lfd_nr_bv"))
                        .map(|m| m.max_y)
                        .unwrap_or(810.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Lasten und Beschränkungen"
                Column {
                    id: "abt2_vert-lasten_und_beschraenkungen",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert-lasten_und_beschraenkungen"))
                        .map(|m| m.min_x)
                        .unwrap_or(112.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert-lasten_und_beschraenkungen"))
                        .map(|m| m.max_x)
                        .unwrap_or(567.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert-lasten_und_beschraenkungen"))
                        .map(|m| m.min_y)
                        .unwrap_or(150.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert-lasten_und_beschraenkungen"))
                        .map(|m| m.max_y)
                        .unwrap_or(810.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
            ],
            SeitenTyp::Abt2VertTyp2 => vec![
                // "lfd. Nr. der Eintragungen"
                Column {
                    id: "abt2_vert-lfd_nr",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert-lfd_nr"))
                        .map(|m| m.min_x)
                        .unwrap_or(12.71839),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert-lfd_nr"))
                        .map(|m| m.max_x)
                        .unwrap_or(22.96376),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert-lfd_nr"))
                        .map(|m| m.min_y)
                        .unwrap_or(44.685932),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert-lfd_nr"))
                        .map(|m| m.max_y)
                        .unwrap_or(276.2268),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "lfd. Nr der betroffenen Grundstücke"
                Column {
                    id: "abt2_vert-lfd_nr_bv",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert-lfd_nr_bv"))
                        .map(|m| m.min_x)
                        .unwrap_or(24.906849),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert-lfd_nr_bv"))
                        .map(|m| m.max_x)
                        .unwrap_or(46.457455),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert-lfd_nr_bv"))
                        .map(|m| m.min_y)
                        .unwrap_or(44.685932),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert-lfd_nr_bv"))
                        .map(|m| m.max_y)
                        .unwrap_or(276.2268),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Lasten und Beschränkungen"
                Column {
                    id: "abt2_vert-lasten_und_beschraenkungen",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert-lasten_und_beschraenkungen"))
                        .map(|m| m.min_x)
                        .unwrap_or(49.10712),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert-lasten_und_beschraenkungen"))
                        .map(|m| m.max_x)
                        .unwrap_or(198.19492),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert-lasten_und_beschraenkungen"))
                        .map(|m| m.min_y)
                        .unwrap_or(44.685932),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert-lasten_und_beschraenkungen"))
                        .map(|m| m.max_y)
                        .unwrap_or(276.2268),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
            ],
            SeitenTyp::Abt2VertVeraenderungen => vec![
                // "lfd. Nr. der Spalte 1"
                Column {
                    id: "abt2_vert_veraenderungen-lfd_nr",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert_veraenderungen-lfd_nr"))
                        .map(|m| m.min_x)
                        .unwrap_or(32.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert_veraenderungen-lfd_nr"))
                        .map(|m| m.max_x)
                        .unwrap_or(65.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert_veraenderungen-lfd_nr"))
                        .map(|m| m.min_y)
                        .unwrap_or(150.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert_veraenderungen-lfd_nr"))
                        .map(|m| m.max_y)
                        .unwrap_or(810.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Veränderungen"
                Column {
                    id: "abt2_vert_veraenderungen-veraenderungen",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert_veraenderungen-veraenderungen"))
                        .map(|m| m.min_x)
                        .unwrap_or(72.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert_veraenderungen-veraenderungen"))
                        .map(|m| m.max_x)
                        .unwrap_or(362.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert_veraenderungen-veraenderungen"))
                        .map(|m| m.min_y)
                        .unwrap_or(150.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert_veraenderungen-veraenderungen"))
                        .map(|m| m.max_y)
                        .unwrap_or(810.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "lfd. Nr. der Spalte 1"
                Column {
                    id: "abt2_vert_veraenderungen-lfd_nr_bv",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert_veraenderungen-lfd_nr_bv"))
                        .map(|m| m.min_x)
                        .unwrap_or(370.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert_veraenderungen-lfd_nr_bv"))
                        .map(|m| m.max_x)
                        .unwrap_or(400.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert_veraenderungen-lfd_nr_bv"))
                        .map(|m| m.min_y)
                        .unwrap_or(150.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert_veraenderungen-lfd_nr_bv"))
                        .map(|m| m.max_y)
                        .unwrap_or(810.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Löschungen"
                Column {
                    id: "abt2_vert_veraenderungen-loeschungen",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert_veraenderungen-loeschungen"))
                        .map(|m| m.min_x)
                        .unwrap_or(406.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert_veraenderungen-loeschungen"))
                        .map(|m| m.max_x)
                        .unwrap_or(565.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert_veraenderungen-loeschungen"))
                        .map(|m| m.min_y)
                        .unwrap_or(150.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt2_vert_veraenderungen-loeschungen"))
                        .map(|m| m.max_y)
                        .unwrap_or(810.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
            ],

            SeitenTyp::Abt3Horz => vec![
                // "lfd. Nr. der Eintragungen"
                Column {
                    id: "abt3_horz-lfd_nr",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz-lfd_nr"))
                        .map(|m| m.min_x)
                        .unwrap_or(55.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz-lfd_nr"))
                        .map(|m| m.max_x)
                        .unwrap_or(95.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz-lfd_nr"))
                        .map(|m| m.min_y)
                        .unwrap_or(130.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz-lfd_nr"))
                        .map(|m| m.max_y)
                        .unwrap_or(565.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "lfd. Nr. der Grundstücke im BV"
                Column {
                    id: "abt3_horz-lfd_nr_bv",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz-lfd_nr_bv"))
                        .map(|m| m.min_x)
                        .unwrap_or(103.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz-lfd_nr_bv"))
                        .map(|m| m.max_x)
                        .unwrap_or(170.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz-lfd_nr_bv"))
                        .map(|m| m.min_y)
                        .unwrap_or(130.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz-lfd_nr_bv"))
                        .map(|m| m.max_y)
                        .unwrap_or(565.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Betrag"
                Column {
                    id: "abt3_horz-betrag",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz-betrag"))
                        .map(|m| m.min_x)
                        .unwrap_or(180.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz-betrag"))
                        .map(|m| m.max_x)
                        .unwrap_or(275.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz-betrag"))
                        .map(|m| m.min_y)
                        .unwrap_or(130.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz-betrag"))
                        .map(|m| m.max_y)
                        .unwrap_or(565.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Hypotheken, Grundschulden, Rentenschulden"
                Column {
                    id: "abt3_horz-text",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz-text"))
                        .map(|m| m.min_x)
                        .unwrap_or(285.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz-text"))
                        .map(|m| m.max_x)
                        .unwrap_or(760.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz-text"))
                        .map(|m| m.min_y)
                        .unwrap_or(130.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz-text"))
                        .map(|m| m.max_y)
                        .unwrap_or(565.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 25.0, // 10.0,
                },
            ],
            SeitenTyp::Abt3Vert => vec![
                // "lfd. Nr. der Eintragungen"
                Column {
                    id: "abt3_vert-lfd_nr",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert-lfd_nr"))
                        .map(|m| m.min_x)
                        .unwrap_or(32.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert-lfd_nr"))
                        .map(|m| m.max_x)
                        .unwrap_or(60.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert-lfd_nr"))
                        .map(|m| m.min_y)
                        .unwrap_or(150.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert-lfd_nr"))
                        .map(|m| m.max_y)
                        .unwrap_or(785.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "lfd. Nr der belastete Grundstücke im BV"
                Column {
                    id: "abt3_vert-lfd_nr_bv",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert-lfd_nr_bv"))
                        .map(|m| m.min_x)
                        .unwrap_or(65.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert-lfd_nr_bv"))
                        .map(|m| m.max_x)
                        .unwrap_or(100.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert-lfd_nr_bv"))
                        .map(|m| m.min_y)
                        .unwrap_or(150.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert-lfd_nr_bv"))
                        .map(|m| m.max_y)
                        .unwrap_or(785.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Betrag"
                Column {
                    id: "abt3_vert-betrag",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert-betrag"))
                        .map(|m| m.min_x)
                        .unwrap_or(105.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert-betrag"))
                        .map(|m| m.max_x)
                        .unwrap_or(193.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert-betrag"))
                        .map(|m| m.min_y)
                        .unwrap_or(150.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert-betrag"))
                        .map(|m| m.max_y)
                        .unwrap_or(785.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Hypotheken, Grundschulden, Rentenschulden"
                Column {
                    id: "abt3_vert-text",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert-text"))
                        .map(|m| m.min_x)
                        .unwrap_or(195.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert-text"))
                        .map(|m| m.max_x)
                        .unwrap_or(567.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert-text"))
                        .map(|m| m.min_y)
                        .unwrap_or(150.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert-text"))
                        .map(|m| m.max_y)
                        .unwrap_or(785.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 25.0, // 10.0,
                },
            ],
            SeitenTyp::Abt3HorzVeraenderungenLoeschungen => vec![
                // "lfd. Nr. der Spalte 1"
                Column {
                    id: "abt3_horz_veraenderungen_loeschungen-lfd_nr",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-lfd_nr"))
                        .map(|m| m.min_x)
                        .unwrap_or(55.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-lfd_nr"))
                        .map(|m| m.max_x)
                        .unwrap_or(95.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-lfd_nr"))
                        .map(|m| m.min_y)
                        .unwrap_or(127.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-lfd_nr"))
                        .map(|m| m.max_y)
                        .unwrap_or(565.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Betrag"
                Column {
                    id: "abt3_horz_veraenderungen_loeschungen-betrag",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-betrag"))
                        .map(|m| m.min_x)
                        .unwrap_or(105.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-betrag"))
                        .map(|m| m.max_x)
                        .unwrap_or(200.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-betrag"))
                        .map(|m| m.min_y)
                        .unwrap_or(127.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-betrag"))
                        .map(|m| m.max_y)
                        .unwrap_or(565.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Veränderungen"
                Column {
                    id: "abt3_horz_veraenderungen_loeschungen-text",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-text"))
                        .map(|m| m.min_x)
                        .unwrap_or(202.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-text"))
                        .map(|m| m.max_x)
                        .unwrap_or(490.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-text"))
                        .map(|m| m.min_y)
                        .unwrap_or(127.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-text"))
                        .map(|m| m.max_y)
                        .unwrap_or(565.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "lfd. Nr. der Spalte 1"
                Column {
                    id: "abt3_horz_veraenderungen_loeschungen-lfd_nr",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-lfd_nr"))
                        .map(|m| m.min_x)
                        .unwrap_or(495.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-lfd_nr"))
                        .map(|m| m.max_x)
                        .unwrap_or(535.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-lfd_nr"))
                        .map(|m| m.min_y)
                        .unwrap_or(127.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-lfd_nr"))
                        .map(|m| m.max_y)
                        .unwrap_or(565.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Betrag"
                Column {
                    id: "abt3_horz_veraenderungen_loeschungen-betrag",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-betrag"))
                        .map(|m| m.min_x)
                        .unwrap_or(542.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-betrag"))
                        .map(|m| m.max_x)
                        .unwrap_or(640.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-betrag"))
                        .map(|m| m.min_y)
                        .unwrap_or(127.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-betrag"))
                        .map(|m| m.max_y)
                        .unwrap_or(565.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Löschungen"
                Column {
                    id: "abt3_horz_veraenderungen_loeschungen-text",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-text"))
                        .map(|m| m.min_x)
                        .unwrap_or(645.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-text"))
                        .map(|m| m.max_x)
                        .unwrap_or(765.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-text"))
                        .map(|m| m.min_y)
                        .unwrap_or(127.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-text"))
                        .map(|m| m.max_y)
                        .unwrap_or(565.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
            ],
            SeitenTyp::Abt3VertVeraenderungenLoeschungen => vec![
                // "lfd. Nr. der Spalte 1"
                Column {
                    id: "abt3_vert_veraenderungen_loeschungen-lfd_nr",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-lfd_nr"))
                        .map(|m| m.min_x)
                        .unwrap_or(37.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-lfd_nr"))
                        .map(|m| m.max_x)
                        .unwrap_or(75.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-lfd_nr"))
                        .map(|m| m.min_y)
                        .unwrap_or(127.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-lfd_nr"))
                        .map(|m| m.max_y)
                        .unwrap_or(783.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Betrag"
                Column {
                    id: "abt3_vert_veraenderungen_loeschungen-betrag",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-betrag"))
                        .map(|m| m.min_x)
                        .unwrap_or(80.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-betrag"))
                        .map(|m| m.max_x)
                        .unwrap_or(142.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-betrag"))
                        .map(|m| m.min_y)
                        .unwrap_or(127.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-betrag"))
                        .map(|m| m.max_y)
                        .unwrap_or(783.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Veränderungen"
                Column {
                    id: "abt3_vert_veraenderungen_loeschungen-text",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-text"))
                        .map(|m| m.min_x)
                        .unwrap_or(147.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-text"))
                        .map(|m| m.max_x)
                        .unwrap_or(388.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-text"))
                        .map(|m| m.min_y)
                        .unwrap_or(127.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-text"))
                        .map(|m| m.max_y)
                        .unwrap_or(783.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "lfd. Nr. der Spalte 1"
                Column {
                    id: "abt3_vert_veraenderungen_loeschungen-lfd_nr",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-lfd_nr"))
                        .map(|m| m.min_x)
                        .unwrap_or(390.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-lfd_nr"))
                        .map(|m| m.max_x)
                        .unwrap_or(415.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-lfd_nr"))
                        .map(|m| m.min_y)
                        .unwrap_or(127.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-lfd_nr"))
                        .map(|m| m.max_y)
                        .unwrap_or(783.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Betrag"
                Column {
                    id: "abt3_vert_veraenderungen_loeschungen-betrag",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-betrag"))
                        .map(|m| m.min_x)
                        .unwrap_or(420.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-betrag"))
                        .map(|m| m.max_x)
                        .unwrap_or(485.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-betrag"))
                        .map(|m| m.min_y)
                        .unwrap_or(127.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-betrag"))
                        .map(|m| m.max_y)
                        .unwrap_or(783.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Löschungen"
                Column {
                    id: "abt3_vert_veraenderungen_loeschungen-text",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-text"))
                        .map(|m| m.min_x)
                        .unwrap_or(492.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-text"))
                        .map(|m| m.max_x)
                        .unwrap_or(565.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-text"))
                        .map(|m| m.min_y)
                        .unwrap_or(127.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-text"))
                        .map(|m| m.max_y)
                        .unwrap_or(783.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
            ],

            SeitenTyp::Abt3VertVeraenderungen => vec![
                // "lfd. Nr. der Spalte 1"
                Column {
                    id: "abt3_vert_veraenderungen-lfd_nr",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen-lfd_nr"))
                        .map(|m| m.min_x)
                        .unwrap_or(32.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen-lfd_nr"))
                        .map(|m| m.max_x)
                        .unwrap_or(60.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen-lfd_nr"))
                        .map(|m| m.min_y)
                        .unwrap_or(150.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen-lfd_nr"))
                        .map(|m| m.max_y)
                        .unwrap_or(810.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Betrag"
                Column {
                    id: "abt3_vert_veraenderungen-betrag",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen-betrag"))
                        .map(|m| m.min_x)
                        .unwrap_or(70.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen-betrag"))
                        .map(|m| m.max_x)
                        .unwrap_or(160.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen-betrag"))
                        .map(|m| m.min_y)
                        .unwrap_or(150.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen-betrag"))
                        .map(|m| m.max_y)
                        .unwrap_or(810.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Veränderungen"
                Column {
                    id: "abt3_vert_veraenderungen-text",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen-text"))
                        .map(|m| m.min_x)
                        .unwrap_or(165.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen-text"))
                        .map(|m| m.max_x)
                        .unwrap_or(565.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen-text"))
                        .map(|m| m.min_y)
                        .unwrap_or(150.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_veraenderungen-text"))
                        .map(|m| m.max_y)
                        .unwrap_or(810.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
            ],
            SeitenTyp::Abt3VertLoeschungen => vec![
                // "lfd. Nr. der Spalte 1"
                Column {
                    id: "abt3_vert_loeschungen-lfd_nr",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_loeschungen-lfd_nr"))
                        .map(|m| m.min_x)
                        .unwrap_or(175.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_loeschungen-lfd_nr"))
                        .map(|m| m.max_x)
                        .unwrap_or(205.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_loeschungen-lfd_nr"))
                        .map(|m| m.min_y)
                        .unwrap_or(150.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_loeschungen-lfd_nr"))
                        .map(|m| m.max_y)
                        .unwrap_or(810.0 / scale_factor),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Betrag"
                Column {
                    id: "abt3_vert_loeschungen-betrag",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_loeschungen-betrag"))
                        .map(|m| m.min_x)
                        .unwrap_or(215.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_loeschungen-betrag"))
                        .map(|m| m.max_x)
                        .unwrap_or(305.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_loeschungen-betrag"))
                        .map(|m| m.min_y)
                        .unwrap_or(150.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_loeschungen-betrag"))
                        .map(|m| m.max_y)
                        .unwrap_or(810.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                // "Löschungen"
                Column {
                    id: "abt3_vert_loeschungen-text",
                    min_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_loeschungen-text"))
                        .map(|m| m.min_x)
                        .unwrap_or(310.0 / scale_factor),
                    max_x: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_loeschungen-text"))
                        .map(|m| m.max_x)
                        .unwrap_or(570.0 / scale_factor),
                    min_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_loeschungen-text"))
                        .map(|m| m.min_y)
                        .unwrap_or(150.0 / scale_factor),
                    max_y: anpassungen_seite
                        .and_then(|s| s.spalten.get("abt3_vert_loeschungen-text"))
                        .map(|m| m.max_y)
                        .unwrap_or(810.0 / scale_factor),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Textblock {
    pub text: String,
    pub start_y: f32,
    pub end_y: f32,
    pub start_x: f32,
    pub end_x: f32,
}

impl Textblock {
    pub fn ist_leer(&self) -> bool {
        self.text.trim().is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ParsedHocr {
    pub bounds: Rect,
    pub careas: Vec<HocrArea>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HocrArea {
    pub bounds: Rect,
    pub paragraphs: Vec<HocrParagraph>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HocrParagraph {
    pub bounds: Rect,
    pub lines: Vec<HocrLine>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HocrLine {
    pub bounds: Rect,
    pub words: Vec<HocrWord>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]

pub struct HocrWord {
    pub bounds: Rect,
    pub confidence: f32,
    pub text: String,
}

#[test]
fn test_ocr_parsing() {
    let string = r#"
    <div class='ocr_page' id='page_2' title='image "unknown"; bbox 0 0 640 480; ppageno 1; scan_res 70 70'>
    <div class='ocr_carea' id='block_2_1' title="bbox 36 92 618 361">
     <p class='ocr_par' id='par_2_1' lang='eng' title="bbox 36 92 618 184">
      <span class='ocr_line' id='line_2_1' title="bbox 36 92 580 122; baseline 0 -6; x_size 30; x_descenders 6; x_ascenders 6">
       <span class='ocrx_word' id='word_2_1' title='bbox 36 92 96 116; x_wconf 94'>This</span>
       <span class='ocrx_word' id='word_2_2' title='bbox 109 92 129 116; x_wconf 94'>is</span>
       <span class='ocrx_word' id='word_2_3' title='bbox 141 98 156 116; x_wconf 94'>a</span>
       <span class='ocrx_word' id='word_2_4' title='bbox 169 92 201 116; x_wconf 94'>lot</span>
       <span class='ocrx_word' id='word_2_5' title='bbox 212 92 240 116; x_wconf 96'>of</span>
       <span class='ocrx_word' id='word_2_6' title='bbox 251 92 282 116; x_wconf 96'>12</span>
       <span class='ocrx_word' id='word_2_7' title='bbox 296 92 364 122; x_wconf 96'>point</span>
       <span class='ocrx_word' id='word_2_8' title='bbox 374 93 427 116; x_wconf 96'>text</span>
       <span class='ocrx_word' id='word_2_9' title='bbox 437 93 463 116; x_wconf 96'>to</span>
       <span class='ocrx_word' id='word_2_10' title='bbox 474 93 526 116; x_wconf 96'>test</span>
       <span class='ocrx_word' id='word_2_11' title='bbox 536 92 580 116; x_wconf 96'>the</span>
      </span>
      <span class='ocr_line' id='line_2_2' title="bbox 36 126 618 157; baseline 0 -7; x_size 31; x_descenders 7; x_ascenders 6">
       <span class='ocrx_word' id='word_2_12' title='bbox 36 132 81 150; x_wconf 95'>ocr</span>
       <span class='ocrx_word' id='word_2_13' title='bbox 91 126 160 150; x_wconf 96'>code</span>
       <span class='ocrx_word' id='word_2_14' title='bbox 172 126 223 150; x_wconf 95'>and</span>
       <span class='ocrx_word' id='word_2_15' title='bbox 236 132 286 150; x_wconf 95'>see</span>
       <span class='ocrx_word' id='word_2_16' title='bbox 299 126 314 150; x_wconf 92'>if</span>
       <span class='ocrx_word' id='word_2_17' title='bbox 325 126 339 150; x_wconf 96'>it</span>
       <span class='ocrx_word' id='word_2_18' title='bbox 348 126 433 150; x_wconf 96'>works</span>
       <span class='ocrx_word' id='word_2_19' title='bbox 445 132 478 150; x_wconf 75'>on</span>
       <span class='ocrx_word' id='word_2_20' title='bbox 500 126 529 150; x_wconf 75'>all</span>
       <span class='ocrx_word' id='word_2_21' title='bbox 541 127 618 157; x_wconf 96'>types</span>
      </span>
      <span class='ocr_line' id='line_2_3' title="bbox 36 160 223 184; baseline 0 0; x_size 31.214842; x_descenders 7.2148418; x_ascenders 6">
       <span class='ocrx_word' id='word_2_22' title='bbox 36 160 64 184; x_wconf 96'>of</span>
       <span class='ocrx_word' id='word_2_23' title='bbox 72 160 113 184; x_wconf 96'>file</span>
       <span class='ocrx_word' id='word_2_24' title='bbox 123 160 223 184; x_wconf 96'>format.</span>
      </span>
     </p>
 
     <p class='ocr_par' id='par_2_2' lang='eng' title="bbox 36 194 597 361">
      <span class='ocr_line' id='line_2_4' title="bbox 36 194 585 225; baseline 0 -7; x_size 31; x_descenders 7; x_ascenders 6">
       <span class='ocrx_word' id='word_2_25' title='bbox 36 194 91 218; x_wconf 95'>The</span>
       <span class='ocrx_word' id='word_2_26' title='bbox 102 194 177 224; x_wconf 95'>quick</span>
       <span class='ocrx_word' id='word_2_27' title='bbox 189 194 274 218; x_wconf 95'>brown</span>
       <span class='ocrx_word' id='word_2_28' title='bbox 287 194 339 225; x_wconf 96'>dog</span>
       <span class='ocrx_word' id='word_2_29' title='bbox 348 194 456 225; x_wconf 96'>jumped</span>
       <span class='ocrx_word' id='word_2_30' title='bbox 468 200 531 218; x_wconf 96'>over</span>
       <span class='ocrx_word' id='word_2_31' title='bbox 540 194 585 218; x_wconf 96'>the</span>
      </span>
      <span class='ocr_line' id='line_2_5' title="bbox 37 228 585 259; baseline 0 -7; x_size 31; x_descenders 7; x_ascenders 6">
       <span class='ocrx_word' id='word_2_32' title='bbox 37 228 92 259; x_wconf 96'>lazy</span>
       <span class='ocrx_word' id='word_2_33' title='bbox 103 228 153 252; x_wconf 96'>fox.</span>
       <span class='ocrx_word' id='word_2_34' title='bbox 165 228 220 252; x_wconf 96'>The</span>
       <span class='ocrx_word' id='word_2_35' title='bbox 232 228 307 258; x_wconf 95'>quick</span>
       <span class='ocrx_word' id='word_2_36' title='bbox 319 228 404 252; x_wconf 95'>brown</span>
       <span class='ocrx_word' id='word_2_37' title='bbox 417 228 468 259; x_wconf 95'>dog</span>
       <span class='ocrx_word' id='word_2_38' title='bbox 478 228 585 259; x_wconf 95'>jumped</span>
      </span>
      <span class='ocr_line' id='line_2_6' title="bbox 36 262 597 293; baseline 0 -7; x_size 31; x_descenders 7; x_ascenders 6">
       <span class='ocrx_word' id='word_2_39' title='bbox 36 268 99 286; x_wconf 96'>over</span>
       <span class='ocrx_word' id='word_2_40' title='bbox 109 262 153 286; x_wconf 96'>the</span>
       <span class='ocrx_word' id='word_2_41' title='bbox 165 262 221 293; x_wconf 96'>lazy</span>
       <span class='ocrx_word' id='word_2_42' title='bbox 231 262 281 286; x_wconf 96'>fox.</span>
       <span class='ocrx_word' id='word_2_43' title='bbox 294 262 349 286; x_wconf 96'>The</span>
       <span class='ocrx_word' id='word_2_44' title='bbox 360 262 435 292; x_wconf 96'>quick</span>
       <span class='ocrx_word' id='word_2_45' title='bbox 447 262 532 286; x_wconf 95'>brown</span>
       <span class='ocrx_word' id='word_2_46' title='bbox 545 262 597 293; x_wconf 95'>dog</span>
      </span>
      <span class='ocr_line' id='line_2_7' title="bbox 43 296 561 327; baseline 0 -7; x_size 31; x_descenders 7; x_ascenders 6">
       <span class='ocrx_word' id='word_2_47' title='bbox 43 296 150 327; x_wconf 96'>jumped</span>
       <span class='ocrx_word' id='word_2_48' title='bbox 162 302 226 320; x_wconf 96'>over</span>
       <span class='ocrx_word' id='word_2_49' title='bbox 235 296 279 320; x_wconf 96'>the</span>
       <span class='ocrx_word' id='word_2_50' title='bbox 292 296 347 327; x_wconf 96'>lazy</span>
       <span class='ocrx_word' id='word_2_51' title='bbox 357 296 407 320; x_wconf 96'>fox.</span>
       <span class='ocrx_word' id='word_2_52' title='bbox 420 296 475 320; x_wconf 96'>The</span>
       <span class='ocrx_word' id='word_2_53' title='bbox 486 296 561 326; x_wconf 96'>quick</span>
      </span>
      <span class='ocr_line' id='line_2_8' title="bbox 37 330 561 361; baseline 0 -7; x_size 31; x_descenders 7; x_ascenders 6">
       <span class='ocrx_word' id='word_2_54' title='bbox 37 330 122 354; x_wconf 96'>brown</span>
       <span class='ocrx_word' id='word_2_55' title='bbox 135 330 187 361; x_wconf 96'>dog</span>
       <span class='ocrx_word' id='word_2_56' title='bbox 196 330 304 361; x_wconf 96'>jumped</span>
       <span class='ocrx_word' id='word_2_57' title='bbox 316 336 379 354; x_wconf 95'>over</span>
       <span class='ocrx_word' id='word_2_58' title='bbox 388 330 433 354; x_wconf 96'>the</span>
       <span class='ocrx_word' id='word_2_59' title='bbox 445 330 500 361; x_wconf 96'>lazy</span>
       <span class='ocrx_word' id='word_2_60' title='bbox 511 330 561 354; x_wconf 96'>fox.</span>
      </span>
     </p>
    </div>
   </div>
    "#;

    let parsed = ParsedHocr::new(&string).unwrap();

    assert_eq!(
        parsed.get_zeilen(),
        vec![
            "This is a lot of 12 point text to test the".to_string(),
            "ocr code and see if it works on all types".to_string(),
            "of file format.".to_string(),
            "".to_string(),
            "The quick brown dog jumped over the".to_string(),
            "lazy fox. The quick brown dog jumped".to_string(),
            "over the lazy fox. The quick brown dog".to_string(),
            "jumped over the lazy fox. The quick".to_string(),
            "brown dog jumped over the lazy fox.".to_string(),
        ]
    );
}

impl ParsedHocr {
    pub fn new(hocr_tesseract: &str) -> Result<Self, Fehler> {
        use kuchiki::traits::TendrilSink;

        let document = kuchiki::parse_html().one(hocr_tesseract);

        let page_node = document
            .select(".ocr_page")
            .map_err(|_| {
                Fehler::HocrUngueltig(hocr_tesseract.to_string(), "Kein .ocr_page vorhanden")
            })?
            .next()
            .ok_or_else(|| {
                Fehler::HocrUngueltig(hocr_tesseract.to_string(), "Kein .ocr_page vorhanden")
            })?;

        let infos = page_node
            .as_node()
            .0
            .as_element()
            .ok_or_else(|| {
                Fehler::HocrUngueltig(
                    hocr_tesseract.to_string(),
                    ".ocr_page ist nicht vom Typ ElementNode",
                )
            })?
            .attributes
            .borrow()
            .get("title")
            .ok_or_else(|| {
                Fehler::HocrUngueltig(hocr_tesseract.to_string(), ".ocr_page hat kein <title>")
            })?
            .to_string();

        let page_bounds = get_bbox(&infos).ok_or_else(|| {
            Fehler::HocrUngueltig(
                hocr_tesseract.to_string(),
                ".ocr_page hat ungültige <bounds>",
            )
        })?;

        let careas =
            page_node
                .as_node()
                .select(".ocr_carea")
                .and_then(|carea_node| {
                    carea_node
                        .map(|carea_node| {
                            let infos = carea_node
                                .as_node()
                                .0
                                .as_element()
                                .ok_or_else(|| ())?
                                .attributes
                                .borrow()
                                .get("title")
                                .ok_or_else(|| ())?
                                .to_string();

                            let carea_bounds = get_bbox(&infos).ok_or_else(|| ())?;

                            let paragraphs =
                                page_node
                                    .as_node()
                                    .select(".ocr_par")
                                    .and_then(|ocr_par_node| {
                                        ocr_par_node
                                            .map(|ocr_par_node| {
                                                let infos = ocr_par_node
                                                    .as_node()
                                                    .0
                                                    .as_element()
                                                    .ok_or_else(|| ())?
                                                    .attributes
                                                    .borrow()
                                                    .get("title")
                                                    .ok_or_else(|| ())?
                                                    .to_string();

                                                let paragraph_bounds =
                                                    get_bbox(&infos).ok_or_else(|| ())?;

                                                let lines =
                                                    ocr_par_node
                                                        .as_node()
                                                        .select(".ocr_line")
                                                        .and_then(|ocr_line_node| {
                                                            ocr_line_node
                            .map(|ocr_line_node| {
                                let infos = ocr_line_node.as_node().0.as_element()
                                .ok_or_else(|| ())?
                                .attributes
                                .borrow()
                                .get("title")
                                .ok_or_else(|| ())?
                                .to_string();
                        
                                let line_bounds = get_bbox(&infos)
                                .ok_or_else(|| ())?;
            
                                let words = ocr_line_node.as_node()
                                .select(".ocrx_word")
                                .and_then(|ocr_word_node| {
                                    ocr_word_node.map(|ocr_word_node| {
                                        
                                        let infos = ocr_word_node.as_node().0.as_element()
                                        .ok_or_else(|| ())?
                                        .attributes
                                        .borrow()
                                        .get("title")
                                        .ok_or_else(|| ())?
                                        .to_string();
    
                                        let word_bounds = get_bbox(&infos)
                                        .ok_or_else(|| ())?;
                    
                                        let text = ocr_word_node.as_node()
                                        .text_contents()
                                        .trim()
                                        .to_string();
            
                                        Ok(HocrWord {
                                            bounds: word_bounds,
                                            text,
                                            confidence: 100.0,
                                        })
                                    })
                                    .collect::<Result<Vec<_>, _>>()
                                }).unwrap_or_default();
            
                                Ok(HocrLine {
                                    bounds: line_bounds,
                                    words,
                                })
                            })
                            .collect::<Result<Vec<_>, _>>()
                                                        })
                                                        .unwrap_or_default();

                                                Ok(HocrParagraph {
                                                    bounds: paragraph_bounds,
                                                    lines,
                                                })
                                            })
                                            .collect::<Result<Vec<_>, _>>()
                                    })
                                    .unwrap_or_default();

                            Ok(HocrArea {
                                bounds: carea_bounds,
                                paragraphs,
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()
                })
                .unwrap_or_default();

        Ok(Self {
            bounds: page_bounds,
            careas,
        })
    }

    pub fn get_zeilen(&self) -> Vec<String> {
        let mut zeilen = Vec::new();

        for ca in self.careas.iter() {
            for pa in ca.paragraphs.iter() {
                for li in pa.lines.iter() {
                    zeilen.push(
                        li.words
                            .iter()
                            .map(|w| w.text.clone())
                            .collect::<Vec<_>>()
                            .join(" "),
                    );
                }
                zeilen.push(String::new());
            }
        }

        if zeilen.last().cloned() == Some(String::new()) {
            zeilen.pop();
        }

        zeilen
    }

    pub fn get_text(&self) -> String {
        self.get_zeilen().join("")
    }
}

// "image "unknown"; bbox 0 0 640 480; ppageno 1; scan_res 70 70"
// parse_info("bbox", s) == Some("0 0 640 480")
fn parse_info<'a>(key: &str, input: &'a str) -> Option<&'a str> {
    input
        .split(';')
        .find_map(|s| {
            if !s.trim().starts_with(key) {
                None
            } else {
                s.split(key).nth(1)
            }
        })
        .map(|r| r.trim())
}

fn get_bbox(s: &str) -> Option<Rect> {
    let bounds_string = parse_info("bbox", s.trim())?;
    let numbers = bounds_string
        .split_whitespace()
        .filter_map(|s| s.parse::<f32>().ok())
        .collect::<Vec<_>>();
    if !numbers.len() == 4 {
        return None;
    }
    Some(Rect {
        min_x: numbers[0],
        min_y: numbers[1],
        max_x: numbers[2],
        max_y: numbers[3],
    })
}

/// Stroke path
pub const OP_PATH_PAINT_STROKE: &str = "S";
/// Close and stroke path
pub const OP_PATH_PAINT_STROKE_CLOSE: &str = "s";
/// Fill path using nonzero winding number rule
pub const OP_PATH_PAINT_FILL_NZ: &str = "f";
/// Fill path using nonzero winding number rule (obsolete)
pub const OP_PATH_PAINT_FILL_NZ_OLD: &str = "F";
/// Fill path using even-odd rule
pub const OP_PATH_PAINT_FILL_EO: &str = "f*";
/// Fill and stroke path using nonzero winding number rule
pub const OP_PATH_PAINT_FILL_STROKE_NZ: &str = "B";
/// Close, fill and stroke path using nonzero winding number rule
pub const OP_PATH_PAINT_FILL_STROKE_CLOSE_NZ: &str = "b";
/// Fill and stroke path using even-odd rule
pub const OP_PATH_PAINT_FILL_STROKE_EO: &str = "B*";
/// Close, fill and stroke path using even odd rule
pub const OP_PATH_PAINT_FILL_STROKE_CLOSE_EO: &str = "b*";
/// End path without filling or stroking
pub const OP_PATH_PAINT_END: &str = "n";

const OPERATIONS_TO_CLEAN: &[&str; 10] = &[
    OP_PATH_PAINT_STROKE,
    OP_PATH_PAINT_STROKE_CLOSE,
    OP_PATH_PAINT_FILL_NZ,
    OP_PATH_PAINT_FILL_NZ_OLD,
    OP_PATH_PAINT_FILL_EO,
    OP_PATH_PAINT_FILL_STROKE_NZ,
    OP_PATH_PAINT_FILL_STROKE_CLOSE_NZ,
    OP_PATH_PAINT_FILL_STROKE_EO,
    OP_PATH_PAINT_FILL_STROKE_CLOSE_EO,
    OP_PATH_PAINT_END,
];

pub fn clean_pdf_bytes(pdf_bytes: &[u8]) -> Result<Vec<u8>, Fehler> {
    use lopdf::Object;

    let bad_operators = OPERATIONS_TO_CLEAN
        .iter()
        .map(|s| s.to_string())
        .collect::<BTreeSet<_>>();

    let mut pdf = lopdf::Document::load_mem(&pdf_bytes)?;

    let mut stream_ids = Vec::new();

    let resources_dict_ids = pdf
        .objects
        .iter()
        .filter_map(|(_, object)| {
            let dict = object.as_dict().ok()?;
            let contents_dict = dict.get(b"Contents").ok()?.as_reference().ok()?;
            Some(contents_dict)
        })
        .collect::<Vec<_>>();
    stream_ids.extend(resources_dict_ids);

    for sid in stream_ids.into_iter() {
        if let Some(Object::Stream(s)) = pdf.objects.get_mut(&sid) {
            s.decompress();

            let mut stream_decoded = match s.decode_content().ok() {
                Some(s) => s,
                None => {
                    continue;
                }
            };

            stream_decoded
                .operations
                .retain(|op| !bad_operators.contains(&op.operator));

            s.set_plain_content(stream_decoded.encode()?);
            s.start_position = None;
        }
    }

    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes)
        .map_err(|e| Fehler::Io(String::new(), e))?;

    Ok(bytes)
}

fn column_contains_point(col: &Column, start_x: f32, start_y: f32) -> bool {
    start_x <= col.max_x && start_x >= col.min_x && start_y <= col.max_y && start_y >= col.min_y
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grundbuch {
    pub titelblatt: Titelblatt,
    #[serde(default)]
    #[serde(skip_serializing_if = "Bestandsverzeichnis::is_empty")]
    pub bestandsverzeichnis: Bestandsverzeichnis,
    #[serde(default)]
    #[serde(skip_serializing_if = "Abteilung1::is_empty")]
    pub abt1: Abteilung1,
    #[serde(default)]
    #[serde(skip_serializing_if = "Abteilung2::is_empty")]
    pub abt2: Abteilung2,
    #[serde(default)]
    #[serde(skip_serializing_if = "Abteilung3::is_empty")]
    pub abt3: Abteilung3,
}

impl Grundbuch {
    pub fn new(titelblatt: Titelblatt) -> Self {
        Self {
            titelblatt,
            bestandsverzeichnis: Bestandsverzeichnis::default(),
            abt1: Abteilung1::default(),
            abt2: Abteilung2::default(),
            abt3: Abteilung3::default(),
        }
    }
}

#[derive(Debug, Default, Clone, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct PositionInPdf {
    pub seite: u32,
    #[serde(default)]
    pub rect: OptRect,
}

#[derive(Debug, Default, Clone, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct OptRect {
    pub min_x: Option<f32>,
    pub max_x: Option<f32>,
    pub min_y: Option<f32>,
    pub max_y: Option<f32>,
}

impl OptRect {
    pub fn zero() -> Self {
        Self::default()
    }
}

impl PositionInPdf {
    pub fn expand(&mut self, t: &Textblock) {
        self.rect.min_x = Some(self.rect.min_x.get_or_insert(t.start_x).min(t.start_x));
        self.rect.max_x = Some(self.rect.max_x.get_or_insert(t.end_x).max(t.end_x));
        self.rect.min_y = Some(self.rect.min_y.get_or_insert(t.start_y).min(t.start_y));
        self.rect.max_y = Some(self.rect.max_y.get_or_insert(t.end_y).max(t.end_y));
    }

    pub fn get_rect(&self) -> Rect {
        Rect {
            min_x: self.rect.min_x.unwrap_or(0.0),
            max_x: self.rect.max_x.unwrap_or(0.0),
            min_y: self.rect.min_y.unwrap_or(0.0),
            max_y: self.rect.max_y.unwrap_or(0.0),
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Bestandsverzeichnis {
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub eintraege: Vec<BvEintrag>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub zuschreibungen: Vec<BvZuschreibung>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub abschreibungen: Vec<BvAbschreibung>,
}

impl Bestandsverzeichnis {
    pub fn is_empty(&self) -> bool {
        self.eintraege.is_empty()
            && self.zuschreibungen.is_empty()
            && self.abschreibungen.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BvEintrag {
    Flurstueck(BvEintragFlurstueck),
    Recht(BvEintragRecht),
}

impl fmt::Display for BvEintrag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BvEintrag::Flurstueck(BvEintragFlurstueck {
                lfd_nr,
                bisherige_lfd_nr,
                gemarkung,
                flur,
                flurstueck,
                ..
            }) => {
                write!(f, "{lfd_nr}: Gemarkung {gemarkung:?} Flur {flur} Flurstück {flurstueck} (bisher lfd. Nr. {bisherige_lfd_nr:?})")
            }
            BvEintrag::Recht(BvEintragRecht {
                lfd_nr,
                zu_nr,
                bisherige_lfd_nr,
                ..
            }) => {
                let zu_nr = zu_nr.text();
                write!(f, "Grundstücksgleiches Recht {lfd_nr} (zu Nr. {zu_nr}, bisher {bisherige_lfd_nr:?})")
            }
        }
    }
}

// Eintrag für ein grundstücksgleiches Recht
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct BvEintragRecht {
    pub lfd_nr: usize,
    #[serde(default)]
    #[serde(skip_serializing_if = "StringOrLines::is_empty")]
    pub zu_nr: StringOrLines,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bisherige_lfd_nr: Option<usize>,
    #[serde(default)]
    #[serde(skip_serializing_if = "StringOrLines::is_empty")]
    pub text: StringOrLines,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automatisch_geroetet: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manuell_geroetet: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_in_pdf: Option<PositionInPdf>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct BvEintragFlurstueck {
    pub lfd_nr: usize,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bisherige_lfd_nr: Option<usize>,
    pub flur: usize,
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    pub flurstueck: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gemarkung: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bezeichnung: Option<StringOrLines>,
    #[serde(default)]
    #[serde(skip_serializing_if = "FlurstueckGroesse::ist_leer")]
    pub groesse: FlurstueckGroesse,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automatisch_geroetet: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manuell_geroetet: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_in_pdf: Option<PositionInPdf>,
}

impl BvEintragFlurstueck {
    pub fn neu(lfd_nr: usize) -> Self {
        BvEintragFlurstueck {
            lfd_nr,
            bisherige_lfd_nr: None,
            flur: 0,
            flurstueck: String::new(),
            gemarkung: None,
            bezeichnung: None,
            groesse: FlurstueckGroesse::default(),
            automatisch_geroetet: None,
            manuell_geroetet: None,
            position_in_pdf: None,
        }
    }
}

impl BvEintragRecht {
    pub fn neu(lfd_nr: usize) -> Self {
        BvEintragRecht {
            lfd_nr,
            zu_nr: String::new().into(),
            bisherige_lfd_nr: None,
            text: String::new().into(),
            automatisch_geroetet: None,
            manuell_geroetet: None,
            position_in_pdf: None,
        }
    }
}

impl BvEintrag {
    pub fn neu(lfd_nr: usize) -> Self {
        BvEintrag::Flurstueck(BvEintragFlurstueck::neu(lfd_nr))
    }

    pub fn get_position_in_pdf(&self) -> Option<PositionInPdf> {
        match self {
            BvEintrag::Flurstueck(flst) => flst.position_in_pdf.clone(),
            BvEintrag::Recht(recht) => recht.position_in_pdf.clone(),
        }
    }

    pub fn get_flur(&self) -> usize {
        match self {
            BvEintrag::Flurstueck(flst) => flst.flur,
            BvEintrag::Recht(recht) => 0,
        }
    }

    pub fn get_flurstueck(&self) -> String {
        match self {
            BvEintrag::Flurstueck(flst) => flst.flurstueck.clone(),
            BvEintrag::Recht(recht) => String::new(),
        }
    }

    pub fn get_gemarkung(&self) -> Option<String> {
        match self {
            BvEintrag::Flurstueck(flst) => flst.gemarkung.clone(),
            BvEintrag::Recht(_) => None,
        }
    }

    pub fn ist_leer(&self) -> bool {
        match self {
            BvEintrag::Flurstueck(flst) => {
                flst.lfd_nr == 0
                    && flst.bisherige_lfd_nr == None
                    && flst.flur == 0
                    && flst.flurstueck == String::new()
                    && flst.gemarkung == None
                    && flst.bezeichnung == None
                    && flst.groesse.ist_leer()
            }
            BvEintrag::Recht(recht) => {
                recht.lfd_nr == 0 && recht.bisherige_lfd_nr == None && recht.text.is_empty()
            }
        }
    }

    pub fn ist_geroetet(&self) -> bool {
        match self {
            BvEintrag::Flurstueck(flst) => flst
                .manuell_geroetet
                .unwrap_or(flst.automatisch_geroetet.unwrap_or(false)),
            BvEintrag::Recht(recht) => recht
                .manuell_geroetet
                .unwrap_or(recht.automatisch_geroetet.unwrap_or(false)),
        }
    }

    pub fn set_bezeichnung(&mut self, val: String) {
        match self {
            BvEintrag::Flurstueck(flst) => {
                flst.bezeichnung = if val.is_empty() {
                    None
                } else {
                    Some(val.into())
                };
            }
            BvEintrag::Recht(_) => {}
        }
    }

    pub fn get_bezeichnung(&self) -> Option<String> {
        match self {
            BvEintrag::Flurstueck(flst) => flst.bezeichnung.clone().map(|s| s.text()),
            BvEintrag::Recht(recht) => None,
        }
    }

    pub fn get_groesse(&self) -> Option<FlurstueckGroesse> {
        match self {
            BvEintrag::Flurstueck(flst) => Some(flst.groesse.clone()),
            BvEintrag::Recht(recht) => None,
        }
    }

    pub fn get_lfd_nr(&self) -> usize {
        match self {
            BvEintrag::Flurstueck(flst) => flst.lfd_nr,
            BvEintrag::Recht(recht) => recht.lfd_nr,
        }
    }

    pub fn set_lfd_nr(&mut self, nr: usize) {
        match self {
            BvEintrag::Flurstueck(flst) => flst.lfd_nr = nr,
            BvEintrag::Recht(recht) => recht.lfd_nr = nr,
        }
    }

    pub fn get_bisherige_lfd_nr(&self) -> Option<usize> {
        match self {
            BvEintrag::Flurstueck(flst) => flst.bisherige_lfd_nr,
            BvEintrag::Recht(recht) => recht.bisherige_lfd_nr,
        }
    }

    pub fn set_bisherige_lfd_nr(&mut self, nr: Option<usize>) {
        match self {
            BvEintrag::Flurstueck(flst) => flst.bisherige_lfd_nr = nr,
            BvEintrag::Recht(recht) => recht.bisherige_lfd_nr = nr,
        }
    }

    pub fn set_zu_nr(&mut self, val: String) {
        match self {
            BvEintrag::Flurstueck(_) => {}
            BvEintrag::Recht(recht) => {
                recht.zu_nr = val.into();
            }
        }
    }

    pub fn set_recht_text(&mut self, val: String) {
        match self {
            BvEintrag::Flurstueck(_) => {}
            BvEintrag::Recht(recht) => {
                recht.text = val.into();
            }
        }
    }

    pub fn set_gemarkung(&mut self, val: Option<String>) {
        match self {
            BvEintrag::Flurstueck(flst) => {
                flst.gemarkung = val;
            }
            BvEintrag::Recht(_) => {}
        }
    }

    pub fn set_flur(&mut self, val: usize) {
        match self {
            BvEintrag::Flurstueck(flst) => {
                flst.flur = val;
            }
            BvEintrag::Recht(_) => {}
        }
    }

    pub fn set_flurstueck(&mut self, val: String) {
        match self {
            BvEintrag::Flurstueck(flst) => {
                flst.flurstueck = val;
            }
            BvEintrag::Recht(_) => {}
        }
    }

    pub fn set_groesse(&mut self, val: FlurstueckGroesse) {
        match self {
            BvEintrag::Flurstueck(flst) => {
                flst.groesse = val;
            }
            BvEintrag::Recht(_) => {}
        }
    }

    pub fn unset_automatisch_geroetet(&mut self) {
        match self {
            BvEintrag::Flurstueck(flst) => {
                flst.automatisch_geroetet = None;
            }
            BvEintrag::Recht(recht) => {
                recht.automatisch_geroetet = None;
            }
        }
    }

    pub fn get_automatisch_geroetet(&self) -> Option<bool> {
        match self {
            BvEintrag::Flurstueck(flst) => flst.automatisch_geroetet,
            BvEintrag::Recht(recht) => recht.automatisch_geroetet,
        }
    }

    pub fn set_automatisch_geroetet(&mut self, val: bool) {
        match self {
            BvEintrag::Flurstueck(flst) => {
                flst.automatisch_geroetet = Some(val);
            }
            BvEintrag::Recht(recht) => {
                recht.automatisch_geroetet = Some(val);
            }
        }
    }

    pub fn get_manuell_geroetet(&self) -> Option<bool> {
        match self {
            BvEintrag::Flurstueck(flst) => flst.manuell_geroetet,
            BvEintrag::Recht(recht) => recht.manuell_geroetet,
        }
    }

    pub fn set_manuell_geroetet(&mut self, val: Option<bool>) {
        match self {
            BvEintrag::Flurstueck(flst) => {
                flst.manuell_geroetet = val;
            }
            BvEintrag::Recht(recht) => {
                recht.manuell_geroetet = val;
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StringOrLines {
    SingleLine(String),
    MultiLine(Vec<String>),
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub enum TextInputType {
    Text,
    Number,
}

pub enum FocusType {
    Focus,
    NoFocus,
}

impl StringOrLines {
    // id: bv_{zeile_nr}_bezeichnung
    // width:
    // bv_geroetet
    // input_id: bv:{zeile_nr}:bezeichnung
    pub fn get_html_editable_textfield(
        &self,
        width: usize,
        geroetet: bool,
        id: String,
        input_id: String,
        input_type: TextInputType,
        focus_type: FocusType,
    ) -> String {
        let lines = self.lines().iter()
            .map(|l| l.replace(" ", "\u{00a0}"))
            .map(|l| l.replace("\\", "&bsol;"))
            .map(|l| if l.is_empty() { 
                format!("<div style='display:block;font-family:monospace;font-size:16px;word-wrap:break-word;max-width:500px;'>&nbsp;</div>") 
            } else {
                format!("<div style='display:block;font-family:monospace;font-size:16px;word-wrap:break-word;max-width:500px;'>{}</div>", l) 
            })
            .collect::<Vec<String>>()
        .join("\r\n");

        let bv_geroetet = if geroetet {
            "background:rgb(255,195,195);"
        } else {
            "background:white;"
        };

        let width = if width == 0 {
            format!("display:flex;flex-grow:1;max-width:500px;")
        } else {
            format!("width: {width}px;min-width:{width}px;max-width:500px;")
        };

        let input_type = match input_type {
            TextInputType::Text => "text",
            TextInputType::Number => "number",
        };

        let insert_tab_at_caret = match focus_type {
            FocusType::NoFocus => "onkeydown='insertTabAtCaret(event);'",
            FocusType::Focus => "",
        };

        let select_on_click = match focus_type {
            FocusType::NoFocus => "",
            FocusType::Focus => "onfocus='selectAllOnFocusIn(event);'",
        };

        format!("
            <div class='stringorlines-textfield' id='{id}'  data-textInputType='{input_type}'  focusable='true' 
                style='font-size:16px;user-select: initial;-webkit-user-select: initial;flex-direction:column;{width}{bv_geroetet}' 
                {insert_tab_at_caret}
                {select_on_click}
                oninput='editStringOrLines(event, \"{input_id}\");' 
                contenteditable='true'
                focusable='true'
                tabindex='1'
            >{lines}</div>
        ")
    }

    pub fn push_str(&mut self, s: &str) {
        let mut self_str = self.lines().join("\r\n");
        self_str.push_str(s);
        *self = self_str.into();
    }

    pub fn contains(&self, s: &str) -> bool {
        match self {
            StringOrLines::SingleLine(s) => s.contains(s),
            StringOrLines::MultiLine(ml) => ml.iter().any(|q| q.contains(s)),
        }
    }

    pub fn text(&self) -> String {
        self.lines().join("\r\n")
    }

    pub fn lines(&self) -> Vec<String> {
        match self {
            StringOrLines::SingleLine(s) => s.lines().map(|s| s.to_string()).collect(),
            StringOrLines::MultiLine(ml) => ml.clone(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            StringOrLines::SingleLine(s) => s.is_empty(),
            StringOrLines::MultiLine(ml) => ml.is_empty(),
        }
    }
}

impl Default for StringOrLines {
    fn default() -> Self {
        String::new().into()
    }
}

impl From<String> for StringOrLines {
    fn from(s: String) -> StringOrLines {
        StringOrLines::MultiLine(s.lines().map(|s| s.to_string()).collect())
    }
}

impl From<StringOrLines> for String {
    fn from(s: StringOrLines) -> String {
        match s {
            StringOrLines::SingleLine(s) => s,
            StringOrLines::MultiLine(ml) => ml.join("\r\n"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Serialize, Deserialize)]
#[serde(tag = "typ", content = "wert")]
pub enum FlurstueckGroesse {
    #[serde(rename = "m")]
    Metrisch { m2: Option<u64> },
    #[serde(rename = "ha")]
    Hektar {
        ha: Option<u64>,
        a: Option<u64>,
        m2: Option<u64>,
    },
}

impl Default for FlurstueckGroesse {
    fn default() -> Self {
        FlurstueckGroesse::Metrisch { m2: None }
    }
}

impl FlurstueckGroesse {
    pub fn ist_leer(&self) -> bool {
        match self {
            FlurstueckGroesse::Metrisch { m2 } => m2.is_none(),
            FlurstueckGroesse::Hektar { ha, a, m2 } => m2.is_none() && ha.is_none() && a.is_none(),
        }
    }

    pub fn get_m2(&self) -> u64 {
        match self {
            FlurstueckGroesse::Metrisch { m2 } => m2.unwrap_or(0),
            FlurstueckGroesse::Hektar { ha, a, m2 } => {
                ha.unwrap_or(0) * 100_000 + a.unwrap_or(0) * 100 + m2.unwrap_or(0)
            }
        }
    }

    pub fn get_ha_string(&self) -> String {
        let m2_string = format!("{}", self.get_m2());
        let mut m2_string_chars: Vec<char> = m2_string.chars().collect();
        for _ in 0..4 {
            m2_string_chars.pop();
        }
        m2_string_chars.iter().collect()
    }

    pub fn get_a_string(&self) -> String {
        let m2_string = format!("{}", self.get_m2());
        let mut m2_string_chars: Vec<char> = m2_string.chars().collect();
        m2_string_chars.reverse();
        for _ in 0..(m2_string_chars.len().saturating_sub(4)) {
            m2_string_chars.pop();
        }
        m2_string_chars.reverse();
        for _ in 0..2 {
            m2_string_chars.pop();
        }
        m2_string_chars.iter().collect()
    }

    pub fn get_m2_string(&self) -> String {
        let m2_string = format!("{}", self.get_m2());
        let mut m2_string_chars: Vec<char> = m2_string.chars().collect();
        m2_string_chars.reverse();
        for _ in 0..(m2_string_chars.len().saturating_sub(2)) {
            m2_string_chars.pop();
        }
        m2_string_chars.reverse();
        let fi: String = m2_string_chars.iter().collect();
        if fi.is_empty() {
            format!("0")
        } else {
            fi
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct BvZuschreibung {
    #[serde(default)]
    #[serde(skip_serializing_if = "StringOrLines::is_empty")]
    pub bv_nr: StringOrLines,
    #[serde(default)]
    #[serde(skip_serializing_if = "StringOrLines::is_empty")]
    pub text: StringOrLines,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automatisch_geroetet: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manuell_geroetet: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_in_pdf: Option<PositionInPdf>,
}

impl BvZuschreibung {
    pub fn ist_geroetet(&self) -> bool {
        self.manuell_geroetet
            .or(self.automatisch_geroetet.clone())
            .unwrap_or(false)
    }
    pub fn ist_leer(&self) -> bool {
        self.bv_nr.is_empty() && self.text.is_empty()
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct BvAbschreibung {
    #[serde(default)]
    #[serde(skip_serializing_if = "StringOrLines::is_empty")]
    pub bv_nr: StringOrLines,
    #[serde(default)]
    #[serde(skip_serializing_if = "StringOrLines::is_empty")]
    pub text: StringOrLines,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automatisch_geroetet: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manuell_geroetet: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_in_pdf: Option<PositionInPdf>,
}

impl BvAbschreibung {
    pub fn ist_geroetet(&self) -> bool {
        self.manuell_geroetet
            .or(self.automatisch_geroetet.clone())
            .unwrap_or(false)
    }

    pub fn ist_leer(&self) -> bool {
        self.bv_nr.is_empty() && self.text.is_empty()
    }
}

pub fn analysiere_bv(
    vm: PyVm,
    titelblatt: &Titelblatt,
    seiten: &BTreeMap<String, SeiteParsed>,
    anpassungen_seite: &BTreeMap<String, AnpassungSeite>,
    konfiguration: &Konfiguration,
) -> Result<Bestandsverzeichnis, Fehler> {
    let seitenzahlen = seiten
        .keys()
        .cloned()
        .filter_map(|s| s.parse::<usize>().ok())
        .collect::<Vec<_>>();

    let max_seitenzahl = seitenzahlen.iter().copied().max().unwrap_or(0);

    let default_texte = Vec::new();
    let mut last_lfd_nr = 1;

    let mut bv_eintraege = seiten
        .iter()
        .filter(|(num, s)| {
            s.typ == SeitenTyp::BestandsverzeichnisHorz
                || s.typ == SeitenTyp::BestandsverzeichnisVert
                || s.typ == SeitenTyp::BestandsverzeichnisVertTyp2
        })
        .filter_map(|(num, s)| Some((num.parse::<u32>().ok()?, s)))
        .flat_map(|(seitenzahl, s)| {
            let zeilen_auf_seite = anpassungen_seite
                .get(&format!("{}", seitenzahl))
                .map(|aps| aps.get_zeilen())
                .unwrap_or_default();

            if s.typ == SeitenTyp::BestandsverzeichnisHorz {
                if !zeilen_auf_seite.is_empty() {
                    (0..(zeilen_auf_seite.len() + 1))
                        .map(|i| {
                            let mut position_in_pdf = PositionInPdf {
                                seite: seitenzahl,
                                rect: OptRect::zero(),
                            };

                            let lfd_nr = s
                                .texte
                                .get(0)
                                .and_then(|zeilen| zeilen.get(i))
                                .and_then(|t| {
                                    position_in_pdf.expand(&t);
                                    let numeric_chars = String::from_iter(
                                        t.text.chars().filter(|c| c.is_numeric()),
                                    );
                                    numeric_chars.parse::<usize>().ok()
                                })
                                .unwrap_or(0);

                            let bisherige_lfd_nr = s
                                .texte
                                .get(1)
                                .and_then(|zeilen| zeilen.get(i))
                                .and_then(|t| {
                                    position_in_pdf.expand(&t);
                                    let numeric_chars = String::from_iter(
                                        t.text.chars().filter(|c| c.is_numeric()),
                                    );
                                    numeric_chars.parse::<usize>().ok()
                                });

                            let gemarkung = s
                                .texte
                                .get(2)
                                .and_then(|zeilen| zeilen.get(i))
                                .map(|t| {
                                    position_in_pdf.expand(&t);
                                    t.text.trim().to_string()
                                })
                                .unwrap_or_default();

                            let gemarkung = if gemarkung.is_empty() {
                                None
                            } else {
                                Some(gemarkung)
                            };

                            let flur = s
                                .texte
                                .get(3)
                                .and_then(|zeilen| zeilen.get(i))
                                .and_then(|t| {
                                    position_in_pdf.expand(&t);
                                    let numeric_chars = String::from_iter(
                                        t.text.chars().filter(|c| c.is_numeric()),
                                    );
                                    numeric_chars.parse::<usize>().ok()
                                })
                                .unwrap_or_default();

                            let flurstueck = s
                                .texte
                                .get(4)
                                .and_then(|zeilen| zeilen.get(i))
                                .and_then(|t| {
                                    position_in_pdf.expand(&t);
                                    let numeric_chars = String::from_iter(
                                        t.text.chars().filter(|c| c.is_numeric() || *c == '/'),
                                    );
                                    Some(numeric_chars)
                                })
                                .unwrap_or_default();

                            let bezeichnung = s
                                .texte
                                .get(5)
                                .and_then(|zeilen| zeilen.get(i))
                                .map(|t| {
                                    position_in_pdf.expand(&t);
                                    t.text.trim().to_string()
                                })
                                .unwrap_or_default();

                            let bezeichnung = if bezeichnung.is_empty() {
                                None
                            } else {
                                crate::python::text_saubern(
                                    vm.clone(),
                                    bezeichnung.trim(),
                                    konfiguration,
                                )
                                .ok()
                                .map(|o| o.into())
                            };

                            let ha =
                                s.texte
                                    .get(6)
                                    .and_then(|zeilen| zeilen.get(i))
                                    .and_then(|t| {
                                        position_in_pdf.expand(&t);
                                        let numeric_chars = String::from_iter(
                                            t.text.chars().filter(|c| c.is_numeric()),
                                        );
                                        numeric_chars.parse::<u64>().ok()
                                    });

                            let a = s
                                .texte
                                .get(7)
                                .and_then(|zeilen| zeilen.get(i))
                                .and_then(|t| {
                                    position_in_pdf.expand(&t);
                                    let numeric_chars = String::from_iter(
                                        t.text.chars().filter(|c| c.is_numeric()),
                                    );
                                    numeric_chars.parse::<u64>().ok()
                                });

                            let m2 =
                                s.texte
                                    .get(8)
                                    .and_then(|zeilen| zeilen.get(i))
                                    .and_then(|t| {
                                        position_in_pdf.expand(&t);
                                        let numeric_chars = String::from_iter(
                                            t.text.chars().filter(|c| c.is_numeric()),
                                        );
                                        numeric_chars.parse::<u64>().ok()
                                    });

                            let groesse = FlurstueckGroesse::Hektar { ha, a, m2 };

                            BvEintrag::Flurstueck(BvEintragFlurstueck {
                                lfd_nr,
                                bisherige_lfd_nr,
                                flur,
                                flurstueck,
                                gemarkung,
                                bezeichnung,
                                groesse,
                                automatisch_geroetet: None,
                                manuell_geroetet: None,
                                position_in_pdf: Some(position_in_pdf),
                            })
                        })
                        .collect::<Vec<_>>()
                } else {
                    s.texte
                        .get(4)
                        .unwrap_or(&default_texte)
                        .iter()
                        .enumerate()
                        .filter_map(|(lfd_num, flurstueck_text)| {
                            let mut position_in_pdf = PositionInPdf {
                                seite: seitenzahl,
                                rect: OptRect::zero(),
                            };

                            position_in_pdf.expand(&flurstueck_text);

                            // TODO: auch texte "1-3"
                            let flurstueck = flurstueck_text.text.trim().to_string();
                            let flurstueck_start_y = flurstueck_text.start_y;
                            let flurstueck_end_y = flurstueck_text.end_y;

                            let lfd_nr = match get_erster_text_bei_ca(
                                &s.texte.get(0).unwrap_or(&default_texte),
                                lfd_num,
                                flurstueck_start_y,
                                flurstueck_end_y,
                            )
                            .and_then(|t| {
                                position_in_pdf.expand(&t);
                                t.text.parse::<usize>().ok()
                            }) {
                                Some(s) => s,
                                None => last_lfd_nr,
                            };

                            last_lfd_nr = lfd_nr;

                            let bisherige_lfd_nr = get_erster_text_bei_ca(
                                &s.texte.get(1).unwrap_or(&default_texte),
                                lfd_num,
                                flurstueck_start_y,
                                flurstueck_end_y,
                            )
                            .and_then(|t| {
                                position_in_pdf.expand(&t);
                                t.text.parse::<usize>().ok()
                            });

                            let mut gemarkung = if s.typ == SeitenTyp::BestandsverzeichnisHorz {
                                get_erster_text_bei_ca(
                                    &s.texte.get(2).unwrap_or(&default_texte),
                                    lfd_num,
                                    flurstueck_start_y,
                                    flurstueck_end_y,
                                )
                                .map(|t| {
                                    position_in_pdf.expand(&t);
                                    t.text.trim().to_string()
                                })
                            } else {
                                None
                            };

                            let flur = {
                                if s.typ == SeitenTyp::BestandsverzeichnisHorz {
                                    get_erster_text_bei_ca(
                                        &s.texte.get(3).unwrap_or(&default_texte),
                                        lfd_num,
                                        flurstueck_start_y,
                                        flurstueck_end_y,
                                    )
                                    .and_then(|t| {
                                        position_in_pdf.expand(&t);
                                        let numeric_chars = String::from_iter(
                                            t.text.chars().filter(|c| c.is_numeric()),
                                        );
                                        numeric_chars.parse::<usize>().ok()
                                    })?
                                } else {
                                    get_erster_text_bei_ca(
                                        &s.texte.get(2).unwrap_or(&default_texte),
                                        lfd_num,
                                        flurstueck_start_y,
                                        flurstueck_end_y,
                                    )
                                    .and_then(|t| {
                                        position_in_pdf.expand(&t);
                                        // ignoriere Zusatzbemerkungen zu Gemarkung
                                        let numeric_chars = String::from_iter(
                                            t.text.chars().filter(|c| c.is_numeric()),
                                        );
                                        let non_numeric_chars = String::from_iter(
                                            t.text.chars().filter(|c| c.is_alphabetic()),
                                        );

                                        if !non_numeric_chars.is_empty() {
                                            gemarkung = Some(non_numeric_chars.trim().to_string());
                                        }

                                        numeric_chars.parse::<usize>().ok()
                                    })?
                                }
                            };

                            let bezeichnung = if s.typ == SeitenTyp::BestandsverzeichnisHorz {
                                get_erster_text_bei_ca(
                                    &s.texte.get(5).unwrap_or(&default_texte),
                                    lfd_num,
                                    flurstueck_start_y,
                                    flurstueck_end_y,
                                )
                                .map(|t| {
                                    position_in_pdf.expand(&t);
                                    t.text.trim().to_string().into()
                                })
                            } else {
                                get_erster_text_bei_ca(
                                    &s.texte.get(4).unwrap_or(&default_texte),
                                    lfd_num,
                                    flurstueck_start_y,
                                    flurstueck_end_y,
                                )
                                .map(|t| {
                                    position_in_pdf.expand(&t);
                                    t.text.trim().to_string().into()
                                })
                            };

                            let groesse = if s.typ == SeitenTyp::BestandsverzeichnisHorz {
                                let ha = get_erster_text_bei_ca(
                                    &s.texte.get(6).unwrap_or(&default_texte),
                                    lfd_num,
                                    flurstueck_start_y,
                                    flurstueck_end_y,
                                )
                                .and_then(|t| {
                                    position_in_pdf.expand(&t);
                                    t.text.parse::<u64>().ok()
                                });
                                let a = get_erster_text_bei_ca(
                                    &s.texte.get(7).unwrap_or(&default_texte),
                                    lfd_num,
                                    flurstueck_start_y,
                                    flurstueck_end_y,
                                )
                                .and_then(|t| {
                                    position_in_pdf.expand(&t);
                                    t.text.parse::<u64>().ok()
                                });
                                let m2 = get_erster_text_bei_ca(
                                    &s.texte.get(8).unwrap_or(&default_texte),
                                    lfd_num,
                                    flurstueck_start_y,
                                    flurstueck_end_y,
                                )
                                .and_then(|t| {
                                    position_in_pdf.expand(&t);
                                    t.text.parse::<u64>().ok()
                                });

                                FlurstueckGroesse::Hektar { ha, a, m2 }
                            } else {
                                let m2 = get_erster_text_bei_ca(
                                    &s.texte.get(5).unwrap_or(&default_texte),
                                    lfd_num,
                                    flurstueck_start_y,
                                    flurstueck_end_y,
                                )
                                .and_then(|t| {
                                    position_in_pdf.expand(&t);
                                    t.text.parse::<u64>().ok()
                                });
                                FlurstueckGroesse::Metrisch { m2 }
                            };

                            Some(BvEintrag::Flurstueck(BvEintragFlurstueck {
                                lfd_nr,
                                bisherige_lfd_nr,
                                flur,
                                flurstueck,
                                gemarkung,
                                bezeichnung,
                                groesse,
                                automatisch_geroetet: None,
                                manuell_geroetet: None,
                                position_in_pdf: Some(position_in_pdf),
                            }))
                        })
                        .collect::<Vec<_>>()
                }
            } else if s.typ == SeitenTyp::BestandsverzeichnisVert {
                if !zeilen_auf_seite.is_empty() {
                    (0..(zeilen_auf_seite.len() + 1))
                        .map(|i| {
                            let mut position_in_pdf = PositionInPdf {
                                seite: seitenzahl,
                                rect: OptRect::zero(),
                            };

                            let lfd_nr = s
                                .texte
                                .get(0)
                                .and_then(|zeilen| zeilen.get(i))
                                .and_then(|t| {
                                    position_in_pdf.expand(&t);
                                    let numeric_chars = String::from_iter(
                                        t.text.chars().filter(|c| c.is_numeric()),
                                    );
                                    numeric_chars.parse::<usize>().ok()
                                })
                                .unwrap_or(0);

                            let bisherige_lfd_nr = s
                                .texte
                                .get(1)
                                .and_then(|zeilen| zeilen.get(i))
                                .and_then(|t| {
                                    position_in_pdf.expand(&t);
                                    let numeric_chars = String::from_iter(
                                        t.text.chars().filter(|c| c.is_numeric()),
                                    );
                                    numeric_chars.parse::<usize>().ok()
                                });

                            let mut gemarkung = None;

                            let flur = s
                                .texte
                                .get(2)
                                .and_then(|zeilen| zeilen.get(i))
                                .and_then(|t| {
                                    position_in_pdf.expand(&t);
                                    // ignoriere Zusatzbemerkungen zu Gemarkung
                                    let numeric_chars = String::from_iter(
                                        t.text.chars().filter(|c| c.is_numeric()),
                                    );
                                    let non_numeric_chars = String::from_iter(
                                        t.text.chars().filter(|c| c.is_alphabetic()),
                                    );

                                    if !non_numeric_chars.is_empty() {
                                        let gemarkung_str = non_numeric_chars.trim().to_string();
                                        gemarkung = if gemarkung_str.is_empty() {
                                            None
                                        } else {
                                            Some(gemarkung_str)
                                        };
                                    }

                                    numeric_chars.parse::<usize>().ok()
                                })
                                .unwrap_or_default();

                            let flurstueck = s
                                .texte
                                .get(3)
                                .and_then(|zeilen| zeilen.get(i))
                                .and_then(|t| {
                                    position_in_pdf.expand(&t);
                                    let numeric_chars = String::from_iter(
                                        t.text.chars().filter(|c| c.is_numeric() || *c == '/'),
                                    );
                                    Some(numeric_chars)
                                })
                                .unwrap_or_default();

                            let bezeichnung = s
                                .texte
                                .get(4)
                                .and_then(|zeilen| zeilen.get(i))
                                .map(|t| {
                                    position_in_pdf.expand(&t);
                                    t.text.trim().to_string()
                                })
                                .unwrap_or_default();

                            let bezeichnung = if bezeichnung.is_empty() {
                                None
                            } else {
                                Some(bezeichnung.into())
                            };

                            let m2 =
                                s.texte
                                    .get(5)
                                    .and_then(|zeilen| zeilen.get(i))
                                    .and_then(|t| {
                                        position_in_pdf.expand(&t);
                                        let numeric_chars = String::from_iter(
                                            t.text.chars().filter(|c| c.is_numeric()),
                                        );
                                        numeric_chars.parse::<u64>().ok()
                                    });

                            let groesse = FlurstueckGroesse::Metrisch { m2 };

                            BvEintrag::Flurstueck(BvEintragFlurstueck {
                                lfd_nr,
                                bisherige_lfd_nr,
                                flur,
                                flurstueck,
                                gemarkung,
                                bezeichnung,
                                groesse,
                                automatisch_geroetet: None,
                                manuell_geroetet: None,
                                position_in_pdf: Some(position_in_pdf),
                            })
                        })
                        .collect::<Vec<_>>()
                } else {
                    s.texte
                        .get(0)
                        .unwrap_or(&default_texte)
                        .iter()
                        .enumerate()
                        .filter_map(|(lfd_num, ldf_nr_text)| {
                            let mut position_in_pdf = PositionInPdf {
                                seite: seitenzahl,
                                rect: OptRect::zero(),
                            };

                            position_in_pdf.expand(&ldf_nr_text);

                            // TODO: auch texte "1-3"
                            let lfd_nr = ldf_nr_text.text.parse::<usize>().ok()?;

                            let lfd_nr_start_y = ldf_nr_text.start_y;
                            let lfd_nr_end_y = ldf_nr_text.end_y;

                            last_lfd_nr = lfd_nr;

                            let bisherige_lfd_nr = get_erster_text_bei_ca(
                                &s.texte.get(1).unwrap_or(&default_texte),
                                lfd_num,
                                lfd_nr_start_y,
                                lfd_nr_end_y,
                            )
                            .and_then(|t| {
                                position_in_pdf.expand(&t);
                                t.text.parse::<usize>().ok()
                            });

                            let mut gemarkung = None;

                            let flur = get_erster_text_bei_ca(
                                &s.texte.get(2).unwrap_or(&default_texte),
                                lfd_num,
                                lfd_nr_start_y,
                                lfd_nr_end_y,
                            )
                            .and_then(|t| {
                                position_in_pdf.expand(&t);

                                // ignoriere Zusatzbemerkungen zu Gemarkung
                                let numeric_chars =
                                    String::from_iter(t.text.chars().filter(|c| c.is_numeric()));
                                let non_numeric_chars =
                                    String::from_iter(t.text.chars().filter(|c| c.is_alphabetic()));

                                if !non_numeric_chars.is_empty() {
                                    gemarkung = Some(non_numeric_chars.trim().to_string());
                                }

                                numeric_chars.parse::<usize>().ok()
                            })?;

                            let flurstueck = get_erster_text_bei_ca(
                                &s.texte.get(3).unwrap_or(&default_texte),
                                lfd_num,
                                lfd_nr_start_y,
                                lfd_nr_end_y,
                            )
                            .map(|t| {
                                position_in_pdf.expand(&t);
                                t.text.trim().to_string()
                            })?;

                            let bezeichnung = get_erster_text_bei_ca(
                                &s.texte.get(4).unwrap_or(&default_texte),
                                lfd_num,
                                lfd_nr_start_y,
                                lfd_nr_end_y,
                            )
                            .map(|t| {
                                position_in_pdf.expand(&t);
                                t.text.trim().to_string().into()
                            });

                            let groesse = {
                                let m2 = get_erster_text_bei_ca(
                                    &s.texte.get(5).unwrap_or(&default_texte),
                                    lfd_num,
                                    lfd_nr_start_y,
                                    lfd_nr_end_y,
                                )
                                .and_then(|t| {
                                    position_in_pdf.expand(&t);
                                    t.text.parse::<u64>().ok()
                                });
                                FlurstueckGroesse::Metrisch { m2 }
                            };

                            Some(BvEintrag::Flurstueck(BvEintragFlurstueck {
                                lfd_nr,
                                bisherige_lfd_nr,
                                flur,
                                flurstueck,
                                gemarkung,
                                bezeichnung,
                                groesse,
                                automatisch_geroetet: None,
                                manuell_geroetet: None,
                                position_in_pdf: Some(position_in_pdf),
                            }))
                        })
                        .collect::<Vec<_>>()
                }
            } else if s.typ == SeitenTyp::BestandsverzeichnisVertTyp2 {
                if !zeilen_auf_seite.is_empty() {
                    (0..(zeilen_auf_seite.len() + 1))
                        .map(|i| {
                            let mut position_in_pdf = PositionInPdf {
                                seite: seitenzahl,
                                rect: OptRect::zero(),
                            };

                            let lfd_nr = s
                                .texte
                                .get(0)
                                .and_then(|zeilen| zeilen.get(i))
                                .and_then(|t| {
                                    position_in_pdf.expand(&t);
                                    let numeric_chars = String::from_iter(
                                        t.text.chars().filter(|c| c.is_numeric()),
                                    );
                                    numeric_chars.parse::<usize>().ok()
                                })
                                .unwrap_or(0);

                            let bisherige_lfd_nr = s
                                .texte
                                .get(1)
                                .and_then(|zeilen| zeilen.get(i))
                                .and_then(|t| {
                                    position_in_pdf.expand(&t);
                                    let numeric_chars = String::from_iter(
                                        t.text.chars().filter(|c| c.is_numeric()),
                                    );
                                    numeric_chars.parse::<usize>().ok()
                                });

                            let mut gemarkung = None;
                            let mut flurstueck = String::new();
                            let mut flur = 0;

                            if let Some(s) = s.texte.get(2).and_then(|zeilen| zeilen.get(i)) {
                                let mut split_whitespace = s.text.trim().split_whitespace().rev();
                                position_in_pdf.expand(&s);
                                flurstueck = split_whitespace
                                    .next()
                                    .map(|s| {
                                        String::from_iter(
                                            s.chars().filter(|c| c.is_numeric() || *c == '/'),
                                        )
                                    })
                                    .unwrap_or_default();
                                flur = split_whitespace
                                    .next()
                                    .and_then(|s| {
                                        let numeric_chars =
                                            String::from_iter(s.chars().filter(|c| c.is_numeric()));
                                        numeric_chars.parse::<usize>().ok()
                                    })
                                    .unwrap_or_default();
                                let gemarkung_str = split_whitespace
                                    .into_iter()
                                    .map(|s| s.to_string())
                                    .collect::<Vec<_>>()
                                    .join(" ");
                                gemarkung = if gemarkung_str.is_empty() {
                                    None
                                } else {
                                    Some(gemarkung_str)
                                };
                            }

                            let bezeichnung = s
                                .texte
                                .get(3)
                                .and_then(|zeilen| zeilen.get(i))
                                .map(|t| {
                                    position_in_pdf.expand(&t);
                                    t.text.trim().to_string()
                                })
                                .unwrap_or_default();

                            let bezeichnung = if bezeichnung.is_empty() {
                                None
                            } else {
                                Some(bezeichnung.into())
                            };

                            let m2 =
                                s.texte
                                    .get(4)
                                    .and_then(|zeilen| zeilen.get(i))
                                    .and_then(|t| {
                                        position_in_pdf.expand(&t);
                                        let numeric_chars = String::from_iter(
                                            t.text.chars().filter(|c| c.is_numeric()),
                                        );
                                        numeric_chars.parse::<u64>().ok()
                                    });

                            let groesse = FlurstueckGroesse::Metrisch { m2 };

                            BvEintrag::Flurstueck(BvEintragFlurstueck {
                                lfd_nr,
                                bisherige_lfd_nr,
                                flur,
                                flurstueck,
                                gemarkung,
                                bezeichnung,
                                groesse,
                                automatisch_geroetet: None,
                                manuell_geroetet: None,
                                position_in_pdf: Some(position_in_pdf),
                            })
                        })
                        .collect::<Vec<_>>()
                } else {
                    s.texte
                        .get(0)
                        .unwrap_or(&default_texte)
                        .iter()
                        .enumerate()
                        .filter_map(|(lfd_num, ldf_nr_text)| {
                            let mut position_in_pdf = PositionInPdf {
                                seite: seitenzahl,
                                rect: OptRect::zero(),
                            };

                            position_in_pdf.expand(&ldf_nr_text);

                            // TODO: auch texte "1-3"
                            let lfd_nr = ldf_nr_text.text.parse::<usize>().ok()?;

                            let lfd_nr_start_y = ldf_nr_text.start_y;
                            let lfd_nr_end_y = ldf_nr_text.end_y;

                            last_lfd_nr = lfd_nr;

                            let bisherige_lfd_nr = get_erster_text_bei_ca(
                                &s.texte.get(1).unwrap_or(&default_texte),
                                lfd_num,
                                lfd_nr_start_y,
                                lfd_nr_end_y,
                            )
                            .and_then(|t| t.text.parse::<usize>().ok());

                            let mut gemarkung = None;
                            let mut flurstueck = String::new();
                            let mut flur = 0;

                            if let Some(s) = get_erster_text_bei_ca(
                                &s.texte.get(2).unwrap_or(&default_texte),
                                lfd_num,
                                lfd_nr_start_y,
                                lfd_nr_end_y,
                            ) {
                                let mut split_whitespace = s.text.trim().split_whitespace().rev();

                                flurstueck = split_whitespace
                                    .next()
                                    .map(|s| {
                                        String::from_iter(
                                            s.chars().filter(|c| c.is_numeric() || *c == '/'),
                                        )
                                    })
                                    .unwrap_or_default();
                                flur = split_whitespace
                                    .next()
                                    .and_then(|s| {
                                        let numeric_chars =
                                            String::from_iter(s.chars().filter(|c| c.is_numeric()));
                                        numeric_chars.parse::<usize>().ok()
                                    })
                                    .unwrap_or_default();
                                let gemarkung_str = split_whitespace
                                    .into_iter()
                                    .map(|s| s.to_string())
                                    .collect::<Vec<_>>()
                                    .join(" ");
                                gemarkung = if gemarkung_str.is_empty() {
                                    None
                                } else {
                                    Some(gemarkung_str)
                                };
                            }

                            let bezeichnung = get_erster_text_bei_ca(
                                &s.texte.get(3).unwrap_or(&default_texte),
                                lfd_num,
                                lfd_nr_start_y,
                                lfd_nr_end_y,
                            )
                            .map(|t| t.text.trim().to_string().into());

                            let groesse = {
                                let m2 = get_erster_text_bei_ca(
                                    &s.texte.get(4).unwrap_or(&default_texte),
                                    lfd_num,
                                    lfd_nr_start_y,
                                    lfd_nr_end_y,
                                )
                                .and_then(|t| t.text.parse::<u64>().ok());
                                FlurstueckGroesse::Metrisch { m2 }
                            };

                            Some(BvEintrag::Flurstueck(BvEintragFlurstueck {
                                lfd_nr,
                                bisherige_lfd_nr,
                                flur,
                                flurstueck,
                                gemarkung,
                                bezeichnung,
                                groesse,
                                automatisch_geroetet: None,
                                manuell_geroetet: None,
                                position_in_pdf: Some(position_in_pdf),
                            }))
                        })
                        .collect::<Vec<_>>()
                }
            } else {
                Vec::new()
            }
        })
        .filter(|bv| !bv.ist_leer())
        .collect::<Vec<_>>();

    // lfd. Nrn. korrigieren
    let bv_mit_0 = bv_eintraege
        .iter()
        .enumerate()
        .filter_map(|(i, bv)| if bv.get_lfd_nr() == 0 { Some(i) } else { None })
        .collect::<Vec<_>>();

    for bv_idx in bv_mit_0 {
        let bv_clone = bv_eintraege[bv_idx].clone();
        if bv_idx == 0 {
            continue;
        }

        let bv_idx_minus_eins = bv_idx - 1;

        let bv_minus_eins_clone = bv_eintraege[bv_idx_minus_eins].clone();

        if bv_minus_eins_clone.get_lfd_nr() == 0 {
            continue;
        }

        let mut remove = false;
        let (bv_clone_neu, bv_minus_eins_clone_neu) = match (bv_clone, bv_minus_eins_clone) {
            (
                BvEintrag::Flurstueck(mut bv_clone),
                BvEintrag::Flurstueck(mut bv_minus_eins_clone),
            ) => {
                if bv_clone.bisherige_lfd_nr.is_some()
                    && bv_minus_eins_clone.bisherige_lfd_nr.is_none()
                {
                    bv_minus_eins_clone.bisherige_lfd_nr = bv_clone.bisherige_lfd_nr;
                    remove = true;
                }

                if bv_clone.gemarkung.is_some() && bv_minus_eins_clone.gemarkung.is_none() {
                    bv_minus_eins_clone.gemarkung = bv_clone.gemarkung.clone();
                    remove = true;
                }

                if bv_clone.flur == 0 && bv_minus_eins_clone.flur != 0 {
                    bv_minus_eins_clone.flur = bv_clone.flur.clone();
                    remove = true;
                }

                if bv_clone.flurstueck.is_empty() && !bv_minus_eins_clone.flurstueck.is_empty() {
                    bv_minus_eins_clone.flurstueck = bv_clone.flurstueck.clone();
                    remove = true;
                }

                if bv_clone.bezeichnung.is_none() && !bv_minus_eins_clone.bezeichnung.is_none() {
                    bv_minus_eins_clone.bezeichnung = bv_clone.bezeichnung.clone();
                    remove = true;
                }

                if bv_clone.groesse.ist_leer() && !bv_minus_eins_clone.groesse.ist_leer() {
                    bv_minus_eins_clone.groesse = bv_clone.groesse.clone();
                    remove = true;
                }

                if remove {
                    bv_clone = BvEintragFlurstueck::neu(0);
                }

                (
                    BvEintrag::Flurstueck(bv_clone),
                    BvEintrag::Flurstueck(bv_minus_eins_clone),
                )
            }
            (BvEintrag::Recht(mut bv_clone), BvEintrag::Recht(mut bv_minus_eins_clone)) => {
                if bv_clone.bisherige_lfd_nr.is_some()
                    && bv_minus_eins_clone.bisherige_lfd_nr.is_none()
                {
                    bv_minus_eins_clone.bisherige_lfd_nr = bv_clone.bisherige_lfd_nr;
                    remove = true;
                }

                if bv_clone.text.is_empty() && !bv_minus_eins_clone.text.is_empty() {
                    bv_minus_eins_clone.text = bv_clone.text.clone();
                    remove = true;
                }

                if remove {
                    bv_clone = BvEintragRecht::neu(0);
                }

                (
                    BvEintrag::Recht(bv_clone),
                    BvEintrag::Recht(bv_minus_eins_clone),
                )
            }
            (a, b) => (a, b),
        };

        bv_eintraege[bv_idx] = bv_clone_neu;
        bv_eintraege[bv_idx - 1] = bv_minus_eins_clone_neu;
    }

    let mut bv_eintraege = bv_eintraege
        .into_iter()
        .filter(|bv| !bv.ist_leer())
        .collect::<Vec<BvEintrag>>();

    let bv_mit_irregulaerer_lfd_nr = bv_eintraege
        .iter()
        .enumerate()
        .filter_map(|(i, bv)| {
            if i == 0 {
                return None;
            }
            if bv_eintraege[i - 1].get_lfd_nr() > bv.get_lfd_nr() {
                Some(i)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let bv_irr_korrigieren = bv_mit_irregulaerer_lfd_nr
        .into_iter()
        .filter_map(|bv_irr| {
            let vorherige_lfd = bv_eintraege.get(bv_irr - 1)?.get_lfd_nr();
            let naechste_lfd = bv_eintraege.get(bv_irr + 1)?.get_lfd_nr();
            match naechste_lfd.checked_sub(vorherige_lfd) {
                Some(2) => Some((bv_irr, vorherige_lfd + 1)),
                Some(1) => {
                    if bv_eintraege[bv_irr].get_bisherige_lfd_nr() == Some(vorherige_lfd) {
                        Some((bv_irr, naechste_lfd))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        })
        .collect::<Vec<(usize, usize)>>();

    for (idx, lfd_neu) in bv_irr_korrigieren {
        if let Some(bv) = bv_eintraege.get_mut(idx) {
            bv.set_lfd_nr(lfd_neu);
        }
    }

    let bv_bestand_und_zuschreibungen = seiten
        .iter()
        .filter(|(num, s)| {
            s.typ == SeitenTyp::BestandsverzeichnisHorzZuUndAbschreibungen
                || s.typ == SeitenTyp::BestandsverzeichnisVertZuUndAbschreibungen
        })
        .filter_map(|(num, s)| Some((num.parse::<u32>().ok()?, s)))
        .flat_map(|(seitenzahl, s)| {
            let zeilen_auf_seite = anpassungen_seite
                .get(&format!("{}", seitenzahl))
                .map(|aps| aps.get_zeilen())
                .unwrap_or_default();

            if !zeilen_auf_seite.is_empty() {
                (0..(zeilen_auf_seite.len() + 1))
                    .map(|i| {
                        let zur_lfd_nr = s
                            .texte
                            .get(0)
                            .and_then(|zeilen| zeilen.get(i))
                            .map(|t| t.text.trim().to_string())
                            .unwrap_or_default();

                        let bestand_und_zuschreibungen = s
                            .texte
                            .get(1)
                            .and_then(|zeilen| zeilen.get(i))
                            .map(|t| t.text.trim().to_string())
                            .unwrap_or_default();

                        BvZuschreibung {
                            bv_nr: zur_lfd_nr.into(),
                            text: bestand_und_zuschreibungen.into(),
                            automatisch_geroetet: None,
                            manuell_geroetet: None,
                            position_in_pdf: None,
                        }
                    })
                    .collect::<Vec<_>>()
            } else {
                s.texte
                    .get(0)
                    .unwrap_or(&default_texte)
                    .iter()
                    .enumerate()
                    .filter_map(|(lfd_num, lfd_nr_text)| {
                        // TODO: auch texte "1-3"
                        let zur_lfd_nr = lfd_nr_text.text.trim().to_string();

                        let lfd_nr_text_start_y = lfd_nr_text.start_y;
                        let lfd_nr_text_end_y = lfd_nr_text.start_y;

                        let bestand_und_zuschreibungen = get_erster_text_bei_ca(
                            &s.texte.get(1).unwrap_or(&default_texte),
                            lfd_num,
                            lfd_nr_text_start_y,
                            lfd_nr_text_end_y,
                        )
                        .map(|t| t.text.trim().to_string())?;

                        Some(BvZuschreibung {
                            bv_nr: zur_lfd_nr.into(),
                            text: bestand_und_zuschreibungen.into(),
                            automatisch_geroetet: None,
                            manuell_geroetet: None,
                            position_in_pdf: None,
                        })
                    })
                    .collect::<Vec<_>>()
            }
            .into_iter()
        })
        .filter(|bvz| !bvz.ist_leer())
        .collect();

    let bv_abschreibungen = seiten
        .iter()
        .filter(|(num, s)| {
            s.typ == SeitenTyp::BestandsverzeichnisHorzZuUndAbschreibungen
                || s.typ == SeitenTyp::BestandsverzeichnisVertZuUndAbschreibungen
        })
        .filter_map(|(num, s)| Some((num.parse::<u32>().ok()?, s)))
        .flat_map(|(seitenzahl, s)| {
            let zeilen_auf_seite = anpassungen_seite
                .get(&format!("{}", seitenzahl))
                .map(|aps| aps.get_zeilen())
                .unwrap_or_default();

            if !zeilen_auf_seite.is_empty() {
                (0..(zeilen_auf_seite.len() + 1))
                    .map(|i| {
                        let zur_lfd_nr = s
                            .texte
                            .get(2)
                            .and_then(|zeilen| zeilen.get(i))
                            .map(|t| t.text.trim().to_string())
                            .unwrap_or_default();

                        let abschreibungen = s
                            .texte
                            .get(3)
                            .and_then(|zeilen| zeilen.get(i))
                            .map(|t| t.text.trim().to_string())
                            .unwrap_or_default();

                        BvAbschreibung {
                            bv_nr: zur_lfd_nr.into(),
                            text: abschreibungen.into(),
                            automatisch_geroetet: None,
                            manuell_geroetet: None,
                            position_in_pdf: None,
                        }
                    })
                    .collect::<Vec<_>>()
            } else {
                s.texte
                    .get(2)
                    .unwrap_or(&default_texte)
                    .iter()
                    .enumerate()
                    .filter_map(|(lfd_num, lfd_nr_text)| {
                        // TODO: auch texte "1-3"
                        let zur_lfd_nr = lfd_nr_text.text.trim().to_string();

                        let lfd_nr_text_start_y = lfd_nr_text.start_y;
                        let lfd_nr_text_end_y = lfd_nr_text.end_y;

                        let abschreibungen = get_erster_text_bei_ca(
                            &s.texte.get(3).unwrap_or(&default_texte),
                            lfd_num,
                            lfd_nr_text_start_y,
                            lfd_nr_text_end_y,
                        )
                        .map(|t| t.text.trim().to_string())?;

                        Some(BvAbschreibung {
                            bv_nr: zur_lfd_nr.into(),
                            text: abschreibungen.into(),
                            automatisch_geroetet: None,
                            manuell_geroetet: None,
                            position_in_pdf: None,
                        })
                    })
                    .collect::<Vec<_>>()
            }
            .into_iter()
        })
        .filter(|bva| !bva.ist_leer())
        .collect();

    Ok(Bestandsverzeichnis {
        eintraege: bv_eintraege,
        zuschreibungen: bv_bestand_und_zuschreibungen,
        abschreibungen: bv_abschreibungen,
    })
}

pub fn bv_eintraege_roetungen_loeschen(bv_eintraege: &mut [BvEintrag]) {
    for bv in bv_eintraege.iter_mut() {
        bv.unset_automatisch_geroetet();
    }
}

pub fn bv_eintraege_roeten_2(
    bv_eintraege: &mut [BvEintrag],
    titelblatt: &Titelblatt,
    max_seitenzahl: u32,
    hocr_layout: &HocrLayout,
) {
}
/*
pub fn bv_eintraege_roeten(
    bv_eintraege: &mut [BvEintrag],
    titelblatt: &Titelblatt,
    max_seitenzahl: u32,
) {
    // Automatisch BV Einträge röten
    bv_eintraege.par_iter_mut().for_each(|bv| {
        // Cache nutzen !!!
        if bv.get_automatisch_geroetet().is_some() {
            return;
        }

        let ist_geroetet = {
            if let Some(position_in_pdf) = bv.get_position_in_pdf() {
                use image::GenericImageView;
                use image::Pixel;

                let bv_rect = position_in_pdf.get_rect();

                let temp_ordner = std::env::temp_dir().join(&format!(
                    "{gemarkung}/{blatt}",
                    gemarkung = titelblatt.grundbuch_von,
                    blatt = titelblatt.blatt
                ));

                let temp_pdf_pfad = temp_ordner.clone().join("temp.pdf");
                let pdftoppm_output_path = temp_ordner.clone().join(format!(
                    "page-{}.png",
                    crate::digital::formatiere_seitenzahl(position_in_pdf.seite, max_seitenzahl)
                ));

                match image::open(&pdftoppm_output_path).ok().and_then(|o| {
                    let (im_width, im_height) = o.dimensions();
                    let (page_width, page_height) = pdftotext_layout
                        .seiten
                        .get(&format!("{}", position_in_pdf.seite))
                        .map(|o| (o.breite_mm, o.hoehe_mm))?;

                    let im_width = im_width as f32;
                    let im_height = im_height as f32;

                    Some(
                        o.crop_imm(
                            (bv_rect.min_x / page_width * im_width).round() as u32,
                            (bv_rect.min_y / page_height * im_height).round() as u32,
                            ((bv_rect.max_x - bv_rect.min_x).abs() / page_width * im_width).round()
                                as u32,
                            ((bv_rect.max_y - bv_rect.min_y).abs() / page_height * im_height)
                                .round() as u32,
                        )
                        .to_rgb8(),
                    )
                }) {
                    Some(cropped) => cropped.pixels().any(|px| {
                        px.channels().get(0).copied().unwrap_or(0) > 200
                            && px.channels().get(1).copied().unwrap_or(0) < 120
                            && px.channels().get(2).copied().unwrap_or(0) < 120
                    }),
                    None => false,
                }
            } else {
                false
            }
        };

        bv.set_automatisch_geroetet(ist_geroetet);
    });
}
*/

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Abteilung1 {
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub eintraege: Vec<Abt1Eintrag>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub grundlagen_eintragungen: Vec<Abt1GrundEintragung>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub veraenderungen: Vec<Abt1Veraenderung>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub loeschungen: Vec<Abt1Loeschung>,
}

impl Abteilung1 {
    pub fn is_empty(&self) -> bool {
        self.eintraege.is_empty()
            && self.grundlagen_eintragungen.is_empty()
            && self.veraenderungen.is_empty()
            && self.loeschungen.is_empty()
    }
}

impl Abteilung1 {
    pub fn migriere_v2(&mut self) {
        let mut grundlage_eintragungen_neu = Vec::new();

        for e in self.eintraege.iter_mut() {
            let neu = match e.clone() {
                Abt1Eintrag::V1(v1) => {
                    let eintragung_neu = Abt1GrundEintragung {
                        bv_nr: v1.bv_nr,
                        text: v1.grundlage_der_eintragung,
                        automatisch_geroetet: None,
                        manuell_geroetet: None,
                        position_in_pdf: v1.position_in_pdf.clone(),
                    };

                    grundlage_eintragungen_neu.push(eintragung_neu);
                    Abt1Eintrag::V2(Abt1EintragV2 {
                        lfd_nr: v1.lfd_nr,
                        eigentuemer: v1.eigentuemer,
                        version: 2,
                        automatisch_geroetet: v1.automatisch_geroetet,
                        manuell_geroetet: v1.manuell_geroetet,
                        position_in_pdf: v1.position_in_pdf,
                    })
                }
                Abt1Eintrag::V2(v2) => Abt1Eintrag::V2(v2),
            };

            *e = neu;
        }

        self.grundlagen_eintragungen
            .extend(grundlage_eintragungen_neu.into_iter());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
#[repr(C)]
pub enum Abt1Eintrag {
    V1(Abt1EintragV1),
    V2(Abt1EintragV2),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Abt1EintragV2 {
    // lfd. Nr. der Eintragung
    pub lfd_nr: usize,
    // Rechtstext
    #[serde(default)]
    #[serde(skip_serializing_if = "StringOrLines::is_empty")]
    pub eigentuemer: StringOrLines,
    // Used to distinguish from Abt1EintragV1
    pub version: usize,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automatisch_geroetet: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manuell_geroetet: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_in_pdf: Option<PositionInPdf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Abt1EintragV1 {
    // lfd. Nr. der Eintragung
    pub lfd_nr: usize,
    // Rechtstext
    #[serde(default)]
    #[serde(skip_serializing_if = "StringOrLines::is_empty")]
    pub eigentuemer: StringOrLines,
    // lfd. Nr der betroffenen Grundstücke im Bestandsverzeichnis
    #[serde(default)]
    #[serde(skip_serializing_if = "StringOrLines::is_empty")]
    pub bv_nr: StringOrLines,
    // Vec<BvNr>,
    #[serde(default)]
    #[serde(skip_serializing_if = "StringOrLines::is_empty")]
    pub grundlage_der_eintragung: StringOrLines,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automatisch_geroetet: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manuell_geroetet: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_in_pdf: Option<PositionInPdf>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Abt1GrundEintragung {
    // lfd. Nr. der Eintragung
    #[serde(default)]
    #[serde(skip_serializing_if = "StringOrLines::is_empty")]
    pub bv_nr: StringOrLines,
    // Grundlage der Eintragung
    #[serde(default)]
    #[serde(skip_serializing_if = "StringOrLines::is_empty")]
    pub text: StringOrLines,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub automatisch_geroetet: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manuell_geroetet: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_in_pdf: Option<PositionInPdf>,
}

impl Abt1GrundEintragung {
    pub fn new() -> Self {
        Abt1GrundEintragung {
            bv_nr: String::new().into(),
            text: String::new().into(),

            automatisch_geroetet: None,
            manuell_geroetet: None,
            position_in_pdf: None,
        }
    }

    pub fn ist_geroetet(&self) -> bool {
        self.manuell_geroetet
            .or(self.automatisch_geroetet.clone())
            .unwrap_or(false)
    }
}

impl Abt1EintragV1 {
    pub fn ist_geroetet(&self) -> bool {
        self.manuell_geroetet
            .or(self.automatisch_geroetet.clone())
            .unwrap_or(false)
    }
}

impl Abt1EintragV2 {
    pub fn ist_geroetet(&self) -> bool {
        self.manuell_geroetet
            .or(self.automatisch_geroetet.clone())
            .unwrap_or(false)
    }
}

impl Abt1Eintrag {
    pub fn new(lfd_nr: usize) -> Self {
        Abt1Eintrag::V2(Abt1EintragV2 {
            lfd_nr,
            eigentuemer: String::new().into(),
            version: 2,
            automatisch_geroetet: None,
            manuell_geroetet: None,
            position_in_pdf: None,
        })
    }

    pub fn set_lfd_nr(&mut self, lfd_nr: usize) {
        match self {
            Abt1Eintrag::V1(v1) => {
                v1.lfd_nr = lfd_nr;
            }
            Abt1Eintrag::V2(v2) => {
                v2.lfd_nr = lfd_nr;
            }
        }
    }

    pub fn get_lfd_nr(&self) -> usize {
        match self {
            Abt1Eintrag::V1(v1) => v1.lfd_nr,
            Abt1Eintrag::V2(v2) => v2.lfd_nr,
        }
    }

    pub fn set_manuell_geroetet(&mut self, m: Option<bool>) {
        match self {
            Abt1Eintrag::V1(v1) => {
                v1.manuell_geroetet = m;
            }
            Abt1Eintrag::V2(v2) => {
                v2.manuell_geroetet = m;
            }
        }
    }

    pub fn get_manuell_geroetet(&self) -> Option<bool> {
        match self {
            Abt1Eintrag::V1(v1) => v1.manuell_geroetet,
            Abt1Eintrag::V2(v2) => v2.manuell_geroetet,
        }
    }

    pub fn get_automatisch_geroetet(&self) -> bool {
        match self {
            Abt1Eintrag::V1(v1) => v1.automatisch_geroetet.unwrap_or(false),
            Abt1Eintrag::V2(v2) => v2.automatisch_geroetet.unwrap_or(false),
        }
    }

    pub fn get_eigentuemer(&self) -> String {
        match self {
            Abt1Eintrag::V1(v1) => v1.eigentuemer.text(),
            Abt1Eintrag::V2(v2) => v2.eigentuemer.text(),
        }
    }

    pub fn set_eigentuemer(&mut self, eigentuemer: String) {
        match self {
            Abt1Eintrag::V1(v1) => {
                v1.eigentuemer = eigentuemer.into();
            }
            Abt1Eintrag::V2(v2) => {
                v2.eigentuemer = eigentuemer.into();
            }
        }
    }

    pub fn ist_geroetet(&self) -> bool {
        match self {
            Abt1Eintrag::V1(v1) => v1.ist_geroetet(),
            Abt1Eintrag::V2(v2) => v2.ist_geroetet(),
        }
    }
}

pub fn analysiere_abt1(
    vm: PyVm,
    seiten: &BTreeMap<String, SeiteParsed>,
    anpassungen_seite: &BTreeMap<String, AnpassungSeite>,
    bestandsverzeichnis: &Bestandsverzeichnis,
    konfiguration: &Konfiguration,
) -> Result<Abteilung1, Fehler> {
    let default_texte = Vec::new();

    let abt1_eintraege = seiten
        .iter()
        .filter(|(num, s)| {
            s.typ == SeitenTyp::Abt1Vert
                || s.typ == SeitenTyp::Abt1VertTyp2
                || s.typ == SeitenTyp::Abt1Horz
        })
        .flat_map(|(seitenzahl, s)| {
            let zeilen_auf_seite = anpassungen_seite
                .get(&format!("{}", seitenzahl))
                .map(|aps| aps.get_zeilen())
                .unwrap_or_default();

            if !zeilen_auf_seite.is_empty() {
                (0..(zeilen_auf_seite.len() + 1))
                    .filter_map(|i| {
                        let lfd_nr = s
                            .texte
                            .get(0)
                            .and_then(|zeilen| zeilen.get(i))
                            .and_then(|t| {
                                let numeric_chars =
                                    String::from_iter(t.text.chars().filter(|c| c.is_numeric()));
                                numeric_chars.parse::<usize>().ok()
                            })
                            .unwrap_or(0);

                        let eigentuemer = s
                            .texte
                            .get(1)
                            .and_then(|zeilen| zeilen.get(i))
                            .map(|t| t.text.trim().to_string())
                            .unwrap_or_default();

                        Some(Abt1Eintrag::V2(Abt1EintragV2 {
                            lfd_nr,
                            eigentuemer: crate::python::text_saubern(
                                vm.clone(),
                                eigentuemer.trim(),
                                konfiguration,
                            )
                            .ok()?
                            .into(),
                            version: 2,
                            automatisch_geroetet: None,
                            manuell_geroetet: None,
                            position_in_pdf: None,
                        }))
                    })
                    .collect::<Vec<_>>()
            } else {
                let mut texte = s.texte.clone();
                if let Some(t) = texte.get_mut(1) {
                    t.retain(|t| t.text.trim().len() > 12 && t.text.trim().contains(" "));
                }

                texte
                    .get(1)
                    .unwrap_or(&default_texte)
                    .iter()
                    .enumerate()
                    .filter_map(|(text_num, text)| {
                        let text_start_y = text.start_y;
                        let text_end_y = text.end_y;

                        // TODO: bv-nr korrigieren!

                        // TODO: auch texte "1-3"
                        let lfd_nr = get_erster_text_bei_ca(
                            &texte.get(0).unwrap_or(&default_texte),
                            text_num,
                            text_start_y,
                            text_end_y,
                        )
                        .and_then(|s| s.text.trim().parse::<usize>().ok())
                        .unwrap_or(0);

                        let eigentuemer = get_erster_text_bei_ca(
                            &texte.get(1).unwrap_or(&default_texte),
                            text_num,
                            text_start_y,
                            text_end_y,
                        )
                        .map(|s| s.text.trim().to_string())
                        .unwrap_or_default();

                        // versehentlich Fußzeile erwischt
                        if eigentuemer.contains("JVA Branden") {
                            return None;
                        }

                        Some(Abt1Eintrag::V2(Abt1EintragV2 {
                            lfd_nr,
                            eigentuemer: crate::python::text_saubern(
                                vm.clone(),
                                eigentuemer.trim(),
                                konfiguration,
                            )
                            .ok()?
                            .into(),
                            version: 2,
                            automatisch_geroetet: None,
                            manuell_geroetet: None,
                            position_in_pdf: None,
                        }))
                    })
                    .collect::<Vec<_>>()
            }
            .into_iter()
        })
        .collect();

    let abt1_grundlagen_eintragungen = seiten
        .iter()
        .filter(|(num, s)| {
            s.typ == SeitenTyp::Abt1Vert
                || s.typ == SeitenTyp::Abt1VertTyp2
                || s.typ == SeitenTyp::Abt1Horz
        })
        .flat_map(|(seitenzahl, s)| {
            let zeilen_auf_seite = anpassungen_seite
                .get(&format!("{}", seitenzahl))
                .map(|aps| aps.get_zeilen())
                .unwrap_or_default();

            if !zeilen_auf_seite.is_empty() {
                (0..(zeilen_auf_seite.len() + 1))
                    .filter_map(|i| {
                        let bv_nr = s
                            .texte
                            .get(2)
                            .and_then(|zeilen| zeilen.get(i))
                            .map(|t| t.text.trim().to_string())
                            .unwrap_or_default();

                        let grundlage_der_eintragung = s
                            .texte
                            .get(3)
                            .and_then(|zeilen| zeilen.get(i))
                            .map(|t| t.text.trim().to_string())
                            .unwrap_or_default();

                        Some(Abt1GrundEintragung {
                            bv_nr: bv_nr.into(),
                            text: crate::python::text_saubern(
                                vm.clone(),
                                grundlage_der_eintragung.trim(),
                                konfiguration,
                            )
                            .ok()?
                            .into(),
                            automatisch_geroetet: None,
                            manuell_geroetet: None,
                            position_in_pdf: None,
                        })
                    })
                    .collect::<Vec<_>>()
            } else {
                let mut texte = s.texte.clone();
                if let Some(t) = texte.get_mut(3) {
                    t.retain(|t| t.text.trim().len() > 12 && t.text.trim().contains(" "));
                }

                texte
                    .get(3)
                    .unwrap_or(&default_texte)
                    .iter()
                    .enumerate()
                    .filter_map(|(text_num, text)| {
                        let text_start_y = text.start_y;
                        let text_end_y = text.end_y;

                        let bv_nr = get_erster_text_bei_ca(
                            &texte.get(2).unwrap_or(&default_texte),
                            text_num,
                            text_start_y,
                            text_end_y,
                        )
                        .map(|t| t.text.trim().to_string())?;

                        let grundlage_der_eintragung = get_erster_text_bei_ca(
                            &texte.get(3).unwrap_or(&default_texte),
                            text_num,
                            text_start_y,
                            text_end_y,
                        )
                        .map(|t| t.text.trim().to_string())?;

                        Some(Abt1GrundEintragung {
                            bv_nr: bv_nr.into(),
                            text: crate::python::text_saubern(
                                vm.clone(),
                                grundlage_der_eintragung.trim(),
                                konfiguration,
                            )
                            .ok()?
                            .into(),
                            automatisch_geroetet: None,
                            manuell_geroetet: None,
                            position_in_pdf: None,
                        })
                    })
                    .collect::<Vec<_>>()
            }
            .into_iter()
        })
        .collect();

    let mut abt1 = Abteilung1 {
        eintraege: abt1_eintraege,
        grundlagen_eintragungen: abt1_grundlagen_eintragungen,
        veraenderungen: Vec::new(),
        loeschungen: Vec::new(),
    };

    abt1.migriere_v2();

    Ok(abt1)
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Abteilung2 {
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub eintraege: Vec<Abt2Eintrag>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub veraenderungen: Vec<Abt2Veraenderung>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub loeschungen: Vec<Abt2Loeschung>,
}

impl Abteilung2 {
    pub fn is_empty(&self) -> bool {
        self.eintraege.is_empty() && self.veraenderungen.is_empty() && self.loeschungen.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Abt2Eintrag {
    // lfd. Nr. der Eintragung
    pub lfd_nr: usize,
    // lfd. Nr der betroffenen Grundstücke im Bestandsverzeichnis
    #[serde(default)]
    #[serde(skip_serializing_if = "StringOrLines::is_empty")]
    pub bv_nr: StringOrLines,
    // Rechtstext
    #[serde(default)]
    #[serde(skip_serializing_if = "StringOrLines::is_empty")]
    pub text: StringOrLines,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automatisch_geroetet: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manuell_geroetet: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_in_pdf: Option<PositionInPdf>,
}

impl Abt2Eintrag {
    pub fn new(lfd_nr: usize) -> Self {
        Abt2Eintrag {
            lfd_nr,
            bv_nr: String::new().into(),
            text: String::new().into(),
            automatisch_geroetet: None,
            manuell_geroetet: None,
            position_in_pdf: None,
        }
    }

    pub fn ist_geroetet(&self) -> bool {
        self.manuell_geroetet
            .or(self.automatisch_geroetet.clone())
            .unwrap_or(false)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Nebenbeteiligter {
    // ONr., falls bereits vergeben
    pub ordnungsnummer: Option<usize>,
    // Typ des NB, wichtig für ONr.
    pub typ: Option<NebenbeteiligterTyp>,
    // Name des NB
    pub name: String,
    #[serde(default)]
    pub extra: NebenbeteiligterExtra,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NebenbeteiligterExport {
    // ONr., falls bereits vergeben
    pub ordnungsnummer: Option<usize>,
    // Typ des NB, wichtig für ONr.
    pub typ: Option<NebenbeteiligterTyp>,
    // Recht, in dem der NB zum ersten Mal vorkommt
    pub recht: String,
    // Name des NB
    pub name: String,
    #[serde(default)]
    pub extra: NebenbeteiligterExtra,
}

// Extra Informationen, wird 1:1 in LEFIS übernommen
#[derive(Default, Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct NebenbeteiligterExtra {
    pub anrede: Option<Anrede>,
    pub titel: Option<String>,
    pub vorname: Option<String>,
    pub nachname_oder_firma: Option<String>,
    pub geburtsname: Option<String>,
    pub geburtsdatum: Option<DateTime<Utc>>,
    pub wohnort: Option<String>,
}

impl NebenbeteiligterExtra {
    pub fn geburtsdatum_to_str(d: &DateTime<Utc>) -> String {
        d.format("%d.%m.%Y").to_string()
    }

    pub fn geburtsdatum_from_str(d: &str) -> Option<DateTime<Utc>> {
        use chrono::NaiveDateTime;
        let utc_self = Utc::now();
        let naive_date = NaiveDateTime::parse_from_str(d, "%d.%m.%Y").ok()?;
        Some(DateTime::from_utc(naive_date, utc_self.offset().clone()))
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum Anrede {
    Herr,
    Frau,
    Firma,
}

impl Anrede {
    pub fn to_string(&self) -> &'static str {
        use self::Anrede::*;
        match self {
            Herr => "HERR",
            Frau => "FRAU",
            Firma => "FIRMA",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        use self::Anrede::*;
        match s {
            "HERR" => Some(Herr),
            "FRAU" => Some(Frau),
            "FIRMA" => Some(Firma),
            _ => None,
        }
    }
}

impl Nebenbeteiligter {
    pub fn ordnungsnummern_automatisch_vergeben(v: &mut Vec<Self>) {
        use self::NebenbeteiligterTyp::*;

        let mut hoechste_onr_oeffentlich = v
            .iter()
            .filter_map(|v| {
                if v.typ == Some(Oeffentlich) {
                    v.ordnungsnummer
                } else {
                    None
                }
            })
            .max()
            .unwrap_or(810000);

        let mut hoechste_onr_bank = v
            .iter()
            .filter_map(|v| {
                if v.typ == Some(Bank) {
                    v.ordnungsnummer
                } else {
                    None
                }
            })
            .max()
            .map(|s| s + 1)
            .unwrap_or(812000);

        let mut hoechste_onr_agrar = v
            .iter()
            .filter_map(|v| {
                if v.typ == Some(AgrarGenossenschaft) {
                    v.ordnungsnummer
                } else {
                    None
                }
            })
            .max()
            .map(|s| s + 1)
            .unwrap_or(813000);

        let mut hoechste_onr_privat = v
            .iter()
            .filter_map(|v| {
                if v.typ == Some(PrivateigentuemerHerr)
                    || v.typ == Some(PrivateigentuemerFrau)
                    || v.typ == Some(PrivateigentuemerMehrere)
                {
                    v.ordnungsnummer
                } else {
                    None
                }
            })
            .max()
            .map(|s| s + 1)
            .unwrap_or(814000);

        let mut hoechste_onr_jew = v
            .iter()
            .filter_map(|v| {
                if v.typ == Some(JewEigentuemerDesFlurstuecks) {
                    v.ordnungsnummer
                } else {
                    None
                }
            })
            .max()
            .map(|s| s + 1)
            .unwrap_or(815000);

        let mut hoechste_onr_leitung = v
            .iter()
            .filter_map(|v| {
                if v.typ == Some(Leitungsbetreiber) {
                    v.ordnungsnummer
                } else {
                    None
                }
            })
            .max()
            .map(|s| s + 1)
            .unwrap_or(817000);

        let mut hoechste_onr_gmbh = v
            .iter()
            .filter_map(|v| {
                if v.typ == Some(GmbH) {
                    v.ordnungsnummer
                } else {
                    None
                }
            })
            .max()
            .map(|s| s + 1)
            .unwrap_or(819000);

        for e in v.iter_mut() {
            if e.ordnungsnummer.is_some() {
                continue;
            }
            let typ = match e.typ {
                Some(s) => s,
                None => continue,
            };
            match typ {
                Oeffentlich => {
                    e.ordnungsnummer = Some(hoechste_onr_oeffentlich);
                    hoechste_onr_oeffentlich += 1;
                }
                Bank => {
                    e.ordnungsnummer = Some(hoechste_onr_bank);
                    hoechste_onr_bank += 1;
                }
                AgrarGenossenschaft => {
                    e.ordnungsnummer = Some(hoechste_onr_agrar);
                    hoechste_onr_agrar += 1;
                }
                PrivateigentuemerHerr | PrivateigentuemerFrau | PrivateigentuemerMehrere => {
                    e.ordnungsnummer = Some(hoechste_onr_privat);
                    hoechste_onr_privat += 1;
                }
                JewEigentuemerDesFlurstuecks => {
                    e.ordnungsnummer = Some(hoechste_onr_jew);
                    hoechste_onr_jew += 1;
                }
                Leitungsbetreiber => {
                    e.ordnungsnummer = Some(hoechste_onr_leitung);
                    hoechste_onr_leitung += 1;
                }
                GmbH => {
                    e.ordnungsnummer = Some(hoechste_onr_gmbh);
                    hoechste_onr_gmbh += 1;
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Copy, PartialOrd, Serialize, Deserialize)]
pub enum NebenbeteiligterTyp {
    #[serde(rename = "OEFFENTLICH")]
    Oeffentlich,
    #[serde(rename = "BANK")]
    Bank,
    #[serde(rename = "AGRAR")]
    AgrarGenossenschaft,
    #[serde(rename = "PRIVAT")]
    PrivateigentuemerMehrere,
    #[serde(rename = "PRIVAT-M")]
    PrivateigentuemerHerr,
    #[serde(rename = "PRIVAT-F")]
    PrivateigentuemerFrau,
    #[serde(rename = "JEW-EIGENT")]
    JewEigentuemerDesFlurstuecks,
    #[serde(rename = "LEITUNG")]
    Leitungsbetreiber,
    #[serde(rename = "GMBH")]
    GmbH,
}

impl NebenbeteiligterTyp {
    pub fn get_str(&self) -> &'static str {
        use self::NebenbeteiligterTyp::*;
        match self {
            Oeffentlich => "OEFFENTLICH",
            Bank => "BANK",
            AgrarGenossenschaft => "AGRAR",
            PrivateigentuemerMehrere => "PRIVAT",
            PrivateigentuemerHerr => "PRIVAT-M",
            PrivateigentuemerFrau => "PRIVAT-F",
            JewEigentuemerDesFlurstuecks => "JEW-EIGENT",
            Leitungsbetreiber => "LEITUNG",
            GmbH => "GMBH",
        }
    }

    pub fn from_type_str(s: &str) -> Option<Self> {
        use self::NebenbeteiligterTyp::*;
        match s {
            "OEFFENTLICH" => Some(Oeffentlich),
            "BANK" => Some(Bank),
            "AGRAR" => Some(AgrarGenossenschaft),
            "PRIVAT-M" => Some(PrivateigentuemerHerr),
            "PRIVAT-F" => Some(PrivateigentuemerFrau),
            "PRIVAT" => Some(PrivateigentuemerMehrere),
            "JEW-EIGENT" => Some(JewEigentuemerDesFlurstuecks),
            "LEITUNG" => Some(Leitungsbetreiber),
            "GMBH" => Some(GmbH),
            _ => None,
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        let lower = s.to_lowercase();
        if lower.contains("jeweiligen eigentümer") {
            Some(NebenbeteiligterTyp::JewEigentuemerDesFlurstuecks)
        } else if lower.contains("kreis") || 
           lower.contains("gemeinde") || 
           lower.contains("amt ") || // nicht "Amtsgericht"!
           lower.contains("verwaltung")
        {
            Some(NebenbeteiligterTyp::Oeffentlich)
        } else if lower.contains("bank") || lower.contains("sparkasse") {
            Some(NebenbeteiligterTyp::Bank)
        } else if lower.contains("agrar") {
            Some(NebenbeteiligterTyp::AgrarGenossenschaft)
        } else if lower.contains("gas")
            || lower.contains("e.dis")
            || lower.contains("pck")
            || lower.contains("netz")
            || lower.contains("wind")
        {
            Some(NebenbeteiligterTyp::Leitungsbetreiber)
        } else if lower.contains("mbh") {
            Some(NebenbeteiligterTyp::GmbH)
        } else if lower.contains("geb") || lower.trim().split_whitespace().count() == 2 {
            Some(NebenbeteiligterTyp::PrivateigentuemerMehrere)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BvNr {
    Vollstaendig { nr: usize },
    Teilweise { nr: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GemarkungFlurFlurstueck {
    pub gemarkung: Option<String>,
    pub flur: usize,
    pub flurstueck: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Abt1Veraenderung {
    #[serde(default)]
    #[serde(skip_serializing_if = "StringOrLines::is_empty")]
    pub lfd_nr: StringOrLines,
    #[serde(default)]
    #[serde(skip_serializing_if = "StringOrLines::is_empty")]
    pub text: StringOrLines,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automatisch_geroetet: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manuell_geroetet: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_in_pdf: Option<PositionInPdf>,
}

impl Abt1Veraenderung {
    pub fn ist_geroetet(&self) -> bool {
        self.manuell_geroetet
            .or(self.automatisch_geroetet.clone())
            .unwrap_or(false)
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Abt1Loeschung {
    #[serde(default)]
    #[serde(skip_serializing_if = "StringOrLines::is_empty")]
    pub lfd_nr: StringOrLines,
    #[serde(default)]
    #[serde(skip_serializing_if = "StringOrLines::is_empty")]
    pub text: StringOrLines,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automatisch_geroetet: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manuell_geroetet: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_in_pdf: Option<PositionInPdf>,
}

impl Abt1Loeschung {
    pub fn ist_geroetet(&self) -> bool {
        self.manuell_geroetet
            .or(self.automatisch_geroetet.clone())
            .unwrap_or(false)
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Abt2Veraenderung {
    #[serde(default)]
    #[serde(skip_serializing_if = "StringOrLines::is_empty")]
    pub lfd_nr: StringOrLines,
    #[serde(default)]
    #[serde(skip_serializing_if = "StringOrLines::is_empty")]
    pub text: StringOrLines,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automatisch_geroetet: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manuell_geroetet: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_in_pdf: Option<PositionInPdf>,
}

impl Abt2Veraenderung {
    pub fn ist_geroetet(&self) -> bool {
        self.manuell_geroetet
            .or(self.automatisch_geroetet.clone())
            .unwrap_or(false)
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Abt2Loeschung {
    #[serde(default)]
    #[serde(skip_serializing_if = "StringOrLines::is_empty")]
    pub lfd_nr: StringOrLines,
    #[serde(default)]
    #[serde(skip_serializing_if = "StringOrLines::is_empty")]
    pub text: StringOrLines,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automatisch_geroetet: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manuell_geroetet: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_in_pdf: Option<PositionInPdf>,
}

impl Abt2Loeschung {
    pub fn ist_geroetet(&self) -> bool {
        self.manuell_geroetet
            .or(self.automatisch_geroetet.clone())
            .unwrap_or(false)
    }
}

pub fn analysiere_abt2(
    vm: PyVm,
    seiten: &BTreeMap<String, SeiteParsed>,
    anpassungen_seite: &BTreeMap<String, AnpassungSeite>,
    bestandsverzeichnis: &Bestandsverzeichnis,
    konfiguration: &Konfiguration,
) -> Result<Abteilung2, Fehler> {
    let default_texte = Vec::new();

    let abt2_eintraege = seiten
        .iter()
        .filter(|(num, s)| {
            s.typ == SeitenTyp::Abt2Vert
                || s.typ == SeitenTyp::Abt2VertTyp2
                || s.typ == SeitenTyp::Abt2Vert
        })
        .flat_map(|(seitenzahl, s)| {
            let zeilen_auf_seite = anpassungen_seite
                .get(&format!("{}", seitenzahl))
                .map(|aps| aps.get_zeilen())
                .unwrap_or_default();

            if !zeilen_auf_seite.is_empty() {
                (0..(zeilen_auf_seite.len() + 1))
                    .filter_map(|i| {
                        let lfd_nr = s
                            .texte
                            .get(0)
                            .and_then(|zeilen| zeilen.get(i))
                            .and_then(|t| {
                                let numeric_chars =
                                    String::from_iter(t.text.chars().filter(|c| c.is_numeric()));
                                numeric_chars.parse::<usize>().ok()
                            })
                            .unwrap_or(0);

                        let bv_nr = s
                            .texte
                            .get(1)
                            .and_then(|zeilen| zeilen.get(i))
                            .map(|t| t.text.trim().to_string())
                            .unwrap_or_default();

                        let text = s
                            .texte
                            .get(2)
                            .and_then(|zeilen| zeilen.get(i))
                            .map(|t| t.text.trim().to_string())
                            .unwrap_or_default();

                        Some(Abt2Eintrag {
                            lfd_nr,
                            bv_nr: bv_nr.into(),
                            text: crate::python::text_saubern(
                                vm.clone(),
                                text.trim(),
                                konfiguration,
                            )
                            .ok()?
                            .into(),
                            automatisch_geroetet: None,
                            manuell_geroetet: None,
                            position_in_pdf: None,
                        })
                    })
                    .collect::<Vec<_>>()
            } else {
                let mut texte = s.texte.clone();
                if let Some(t) = texte.get_mut(2) {
                    t.retain(|t| t.text.trim().len() > 12 && t.text.trim().contains(" "));
                }

                texte
                    .get(2)
                    .unwrap_or(&default_texte)
                    .iter()
                    .enumerate()
                    .filter_map(|(text_num, text)| {
                        let text_start_y = text.start_y;
                        let text_end_y = text.end_y;

                        // TODO: bv-nr korrigieren!

                        // TODO: auch texte "1-3"
                        let lfd_nr = get_erster_text_bei_ca(
                            &texte.get(0).unwrap_or(&default_texte),
                            text_num,
                            text_start_y,
                            text_end_y,
                        )
                        .and_then(|s| s.text.trim().parse::<usize>().ok())
                        .unwrap_or(0);

                        let bv_nr = get_erster_text_bei_ca(
                            &texte.get(1).unwrap_or(&default_texte),
                            text_num,
                            text_start_y,
                            text_end_y,
                        )
                        .map(|t| t.text.trim().to_string())?;

                        // versehentlich Fußzeile erwischt
                        if bv_nr.contains("JVA Branden") {
                            return None;
                        }

                        Some(Abt2Eintrag {
                            lfd_nr,
                            bv_nr: bv_nr.to_string().into(),
                            text: crate::python::text_saubern(
                                vm.clone(),
                                text.text.trim(),
                                konfiguration,
                            )
                            .ok()?
                            .into(),
                            automatisch_geroetet: None,
                            manuell_geroetet: None,
                            position_in_pdf: None,
                        })
                    })
                    .collect::<Vec<_>>()
            }
            .into_iter()
        })
        .collect();

    let abt2_veraenderungen = seiten
        .iter()
        .filter(|(num, s)| {
            s.typ == SeitenTyp::Abt2VertVeraenderungen || s.typ == SeitenTyp::Abt2HorzVeraenderungen
        })
        .flat_map(|(seitenzahl, s)| {
            let zeilen_auf_seite = anpassungen_seite
                .get(&format!("{}", seitenzahl))
                .map(|aps| aps.get_zeilen())
                .unwrap_or_default();

            if !zeilen_auf_seite.is_empty() {
                (0..(zeilen_auf_seite.len() + 1))
                    .filter_map(|i| {
                        let lfd_nr = s
                            .texte
                            .get(0)
                            .and_then(|zeilen| zeilen.get(i))
                            .map(|t| t.text.trim().to_string())
                            .unwrap_or_default();

                        let text = s
                            .texte
                            .get(1)
                            .and_then(|zeilen| zeilen.get(i))
                            .map(|t| t.text.trim().to_string())
                            .unwrap_or_default();

                        Some(Abt2Veraenderung {
                            lfd_nr: lfd_nr.into(),
                            text: crate::python::text_saubern(
                                vm.clone(),
                                text.trim(),
                                konfiguration,
                            )
                            .ok()?
                            .into(),
                            automatisch_geroetet: None,
                            manuell_geroetet: None,
                            position_in_pdf: None,
                        })
                    })
                    .collect::<Vec<_>>()
            } else {
                let mut texte = s.texte.clone();
                if let Some(t) = texte.get_mut(1) {
                    t.retain(|t| t.text.trim().len() > 12 && t.text.trim().contains(" "));
                }

                texte
                    .get(1)
                    .unwrap_or(&default_texte)
                    .iter()
                    .enumerate()
                    .filter_map(|(text_num, text)| {
                        let text_start_y = text.start_y;
                        let text_end_y = text.end_y;

                        // TODO: bv-nr korrigieren!

                        // TODO: auch texte "1-3"
                        let lfd_nr = get_erster_text_bei_ca(
                            &texte.get(0).unwrap_or(&default_texte),
                            text_num,
                            text_start_y,
                            text_end_y,
                        )
                        .map(|s| s.text.trim().to_string())?;

                        // TODO: recht analysieren!

                        Some(Abt2Veraenderung {
                            lfd_nr: lfd_nr.into(),
                            text: crate::python::text_saubern(
                                vm.clone(),
                                text.text.trim(),
                                konfiguration,
                            )
                            .ok()?
                            .into(),
                            automatisch_geroetet: None,
                            manuell_geroetet: None,
                            position_in_pdf: None,
                        })
                    })
                    .collect::<Vec<_>>()
            }
            .into_iter()
        })
        .collect();

    Ok(Abteilung2 {
        eintraege: abt2_eintraege,
        veraenderungen: abt2_veraenderungen,
        loeschungen: Vec::new(),
    })
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Abteilung3 {
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub eintraege: Vec<Abt3Eintrag>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub veraenderungen: Vec<Abt3Veraenderung>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub loeschungen: Vec<Abt3Loeschung>,
}

impl Abteilung3 {
    pub fn is_empty(&self) -> bool {
        self.eintraege.is_empty() && self.veraenderungen.is_empty() && self.loeschungen.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Abt3Eintrag {
    // lfd. Nr. der Eintragung
    pub lfd_nr: usize,
    // lfd. Nr der betroffenen Grundstücke im Bestandsverzeichnis
    #[serde(default)]
    #[serde(skip_serializing_if = "StringOrLines::is_empty")]
    pub bv_nr: StringOrLines,
    // Betrag (EUR / DM)
    #[serde(default)]
    #[serde(skip_serializing_if = "StringOrLines::is_empty")]
    pub betrag: StringOrLines,
    /// Rechtstext
    #[serde(default)]
    #[serde(skip_serializing_if = "StringOrLines::is_empty")]
    pub text: StringOrLines,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automatisch_geroetet: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manuell_geroetet: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_in_pdf: Option<PositionInPdf>,
}

impl Abt3Eintrag {
    pub fn new(lfd_nr: usize) -> Self {
        Abt3Eintrag {
            lfd_nr,
            bv_nr: String::new().into(),
            text: String::new().into(),
            betrag: String::new().into(),
            automatisch_geroetet: None,
            manuell_geroetet: None,
            position_in_pdf: None,
        }
    }

    pub fn ist_geroetet(&self) -> bool {
        self.manuell_geroetet
            .or(self.automatisch_geroetet.clone())
            .unwrap_or(false)
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Abt3Veraenderung {
    #[serde(default)]
    #[serde(skip_serializing_if = "StringOrLines::is_empty")]
    pub lfd_nr: StringOrLines,
    #[serde(default)]
    #[serde(skip_serializing_if = "StringOrLines::is_empty")]
    pub betrag: StringOrLines,
    #[serde(default)]
    #[serde(skip_serializing_if = "StringOrLines::is_empty")]
    pub text: StringOrLines,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automatisch_geroetet: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manuell_geroetet: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_in_pdf: Option<PositionInPdf>,
}

impl Abt3Veraenderung {
    pub fn ist_geroetet(&self) -> bool {
        self.manuell_geroetet
            .or(self.automatisch_geroetet.clone())
            .unwrap_or(false)
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Abt3Loeschung {
    #[serde(default)]
    #[serde(skip_serializing_if = "StringOrLines::is_empty")]
    pub lfd_nr: StringOrLines,
    #[serde(default)]
    #[serde(skip_serializing_if = "StringOrLines::is_empty")]
    pub betrag: StringOrLines,
    #[serde(default)]
    #[serde(skip_serializing_if = "StringOrLines::is_empty")]
    pub text: StringOrLines,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automatisch_geroetet: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manuell_geroetet: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_in_pdf: Option<PositionInPdf>,
}

impl Abt3Loeschung {
    pub fn ist_geroetet(&self) -> bool {
        self.manuell_geroetet
            .or(self.automatisch_geroetet.clone())
            .unwrap_or(false)
    }
}

pub fn analysiere_abt3(
    vm: PyVm,
    seiten: &BTreeMap<String, SeiteParsed>,
    anpassungen_seite: &BTreeMap<String, AnpassungSeite>,
    bestandsverzeichnis: &Bestandsverzeichnis,
    konfiguration: &Konfiguration,
) -> Result<Abteilung3, Fehler> {
    use crate::SeitenTyp::Abt3HorzVeraenderungenLoeschungen;
    use crate::SeitenTyp::Abt3VertVeraenderungenLoeschungen;

    let mut last_lfd_nr = 1;

    let default_texte = Vec::new();

    let abt3_eintraege = seiten
        .iter()
        .filter(|(num, s)| s.typ == SeitenTyp::Abt3Horz || s.typ == SeitenTyp::Abt3Vert)
        .flat_map(|(seitenzahl, s)| {
            let zeilen_auf_seite = anpassungen_seite
                .get(&format!("{}", seitenzahl))
                .map(|aps| aps.get_zeilen())
                .unwrap_or_default();

            if !zeilen_auf_seite.is_empty() {
                (0..(zeilen_auf_seite.len() + 1))
                    .filter_map(|i| {
                        let lfd_nr = s
                            .texte
                            .get(0)
                            .and_then(|zeilen| zeilen.get(i))
                            .and_then(|t| {
                                let numeric_chars =
                                    String::from_iter(t.text.chars().filter(|c| c.is_numeric()));
                                numeric_chars.parse::<usize>().ok()
                            })
                            .unwrap_or(0);

                        let bv_nr = s
                            .texte
                            .get(1)
                            .and_then(|zeilen| zeilen.get(i))
                            .map(|t| t.text.trim().to_string())
                            .unwrap_or_default();

                        let betrag = s
                            .texte
                            .get(2)
                            .and_then(|zeilen| zeilen.get(i))
                            .map(|t| t.text.trim().to_string())
                            .unwrap_or_default();

                        let text = s
                            .texte
                            .get(3)
                            .and_then(|zeilen| zeilen.get(i))
                            .map(|t| t.text.trim().to_string())
                            .unwrap_or_default();

                        Some(Abt3Eintrag {
                            lfd_nr: lfd_nr.into(),
                            bv_nr: bv_nr.to_string().into(),
                            betrag: betrag.trim().to_string().into(),
                            text: crate::python::text_saubern(
                                vm.clone(),
                                text.trim(),
                                konfiguration,
                            )
                            .ok()?
                            .into(),
                            automatisch_geroetet: None,
                            manuell_geroetet: None,
                            position_in_pdf: None,
                        })
                    })
                    .collect::<Vec<_>>()
            } else {
                let mut texte = s.texte.clone();
                if let Some(t) = texte.get_mut(2) {
                    t.retain(|t| t.text.trim().len() > 12 && t.text.trim().contains(" "));
                }

                texte
                    .get(3)
                    .unwrap_or(&default_texte)
                    .iter()
                    .enumerate()
                    .filter_map(|(text_num, text)| {
                        let text_start_y = text.start_y;
                        let text_end_y = text.end_y;

                        // TODO: bv-nr korrigieren!

                        // TODO: auch texte "1-3"
                        let lfd_nr = match get_erster_text_bei_ca(
                            &texte.get(0).unwrap_or(&default_texte),
                            text_num,
                            text_start_y,
                            text_end_y,
                        )
                        .and_then(|t| t.text.parse::<usize>().ok())
                        {
                            Some(s) => s,
                            None => last_lfd_nr,
                        };

                        last_lfd_nr = lfd_nr + 1;

                        let bv_nr = get_erster_text_bei_ca(
                            &texte.get(1).unwrap_or(&default_texte),
                            text_num,
                            text_start_y,
                            text_end_y,
                        )
                        .map(|t| t.text.trim().to_string())?;

                        let betrag = get_erster_text_bei_ca(
                            &texte.get(2).unwrap_or(&default_texte),
                            text_num,
                            text_start_y,
                            text_end_y,
                        )
                        .map(|t| t.text.trim().to_string())?;

                        // TODO: recht analysieren!

                        // versehentlich Fußzeile erwischt
                        if bv_nr.contains("JVA Branden") {
                            return None;
                        }

                        Some(Abt3Eintrag {
                            lfd_nr: lfd_nr.into(),
                            bv_nr: bv_nr.to_string().into(),
                            betrag: betrag.trim().to_string().into(),
                            text: crate::python::text_saubern(
                                vm.clone(),
                                text.text.trim(),
                                konfiguration,
                            )
                            .ok()?
                            .into(),
                            automatisch_geroetet: None,
                            manuell_geroetet: None,
                            position_in_pdf: None,
                        })
                    })
                    .collect::<Vec<_>>()
            }
            .into_iter()
        })
        .collect();

    let abt3_veraenderungen = seiten
        .iter()
        .filter(|(num, s)| {
            s.typ == SeitenTyp::Abt3HorzVeraenderungenLoeschungen
                || s.typ == SeitenTyp::Abt3VertVeraenderungenLoeschungen
                || s.typ == SeitenTyp::Abt3VertVeraenderungen
        })
        .flat_map(|(seitenzahl, s)| {
            if s.typ == SeitenTyp::Abt3VertVeraenderungen {
                let zeilen_auf_seite = anpassungen_seite
                    .get(&format!("{}", seitenzahl))
                    .map(|aps| aps.get_zeilen())
                    .unwrap_or_default();

                if !zeilen_auf_seite.is_empty() {
                    (0..(zeilen_auf_seite.len() + 1))
                        .filter_map(|i| {
                            let lfd_nr = s
                                .texte
                                .get(0)
                                .and_then(|zeilen| zeilen.get(i))
                                .map(|t| t.text.trim().to_string())
                                .unwrap_or_default();

                            let betrag = s
                                .texte
                                .get(1)
                                .and_then(|zeilen| zeilen.get(i))
                                .map(|t| t.text.trim().to_string())
                                .unwrap_or_default();

                            let text = s
                                .texte
                                .get(2)
                                .and_then(|zeilen| zeilen.get(i))
                                .map(|t| t.text.trim().to_string())
                                .unwrap_or_default();

                            Some(Abt3Veraenderung {
                                lfd_nr: lfd_nr.into(),
                                betrag: betrag.into(),
                                text: crate::python::text_saubern(
                                    vm.clone(),
                                    text.trim(),
                                    konfiguration,
                                )
                                .ok()?
                                .into(),
                                automatisch_geroetet: None,
                                manuell_geroetet: None,
                                position_in_pdf: None,
                            })
                        })
                        .collect::<Vec<_>>()
                } else {
                    let mut texte = s.texte.clone();
                    if let Some(t) = texte.get_mut(2) {
                        t.retain(|t| t.text.trim().len() > 12 && t.text.trim().contains(" "));
                    }

                    texte
                        .get(2)
                        .unwrap_or(&default_texte)
                        .iter()
                        .enumerate()
                        .filter_map(|(text_num, text)| {
                            let text_start_y = text.start_y;
                            let text_end_y = text.end_y;

                            // TODO: auch texte "1-3"
                            let lfd_nr = get_erster_text_bei_ca(
                                &texte.get(0).unwrap_or(&default_texte),
                                text_num,
                                text_start_y,
                                text_end_y,
                            )
                            .map(|s| s.text.trim().to_string())
                            .unwrap_or_default();

                            let betrag = get_erster_text_bei_ca(
                                &texte.get(1).unwrap_or(&default_texte),
                                text_num,
                                text_start_y,
                                text_end_y,
                            )
                            .map(|s| s.text.trim().to_string())
                            .unwrap_or_default();

                            Some(Abt3Veraenderung {
                                lfd_nr: lfd_nr.into(),
                                betrag: betrag.into(),
                                text: crate::python::text_saubern(
                                    vm.clone(),
                                    &text.text.trim(),
                                    konfiguration,
                                )
                                .ok()?
                                .into(),
                                automatisch_geroetet: None,
                                manuell_geroetet: None,
                                position_in_pdf: None,
                            })
                        })
                        .collect::<Vec<_>>()
                }
            } else {
                Vec::new()
            }
            .into_iter()
        })
        .collect();

    let abt3_loeschungen = seiten
        .iter()
        .filter(|(num, s)| {
            s.typ == SeitenTyp::Abt3HorzVeraenderungenLoeschungen
                || s.typ == SeitenTyp::Abt3VertVeraenderungenLoeschungen
                || s.typ == SeitenTyp::Abt3VertLoeschungen
        })
        .flat_map(|(seitenzahl, s)| {
            if s.typ == SeitenTyp::Abt3VertLoeschungen {
                let column_shift = match s.typ {
                    Abt3HorzVeraenderungenLoeschungen | Abt3VertVeraenderungenLoeschungen => 3,
                    _ => 0,
                };

                let zeilen_auf_seite = anpassungen_seite
                    .get(&format!("{}", seitenzahl))
                    .map(|aps| aps.get_zeilen())
                    .unwrap_or_default();

                if !zeilen_auf_seite.is_empty() {
                    (0..(zeilen_auf_seite.len() + 1))
                        .filter_map(|i| {
                            let lfd_nr = s
                                .texte
                                .get(0 + column_shift)
                                .and_then(|zeilen| zeilen.get(i))
                                .map(|t| t.text.trim().to_string())
                                .unwrap_or_default();

                            let betrag = s
                                .texte
                                .get(1 + column_shift)
                                .and_then(|zeilen| zeilen.get(i))
                                .map(|t| t.text.trim().to_string())
                                .unwrap_or_default();

                            let text = s
                                .texte
                                .get(2 + column_shift)
                                .and_then(|zeilen| zeilen.get(i))
                                .map(|t| t.text.trim().to_string())
                                .unwrap_or_default();

                            Some(Abt3Loeschung {
                                lfd_nr: lfd_nr.into(),
                                betrag: betrag.into(),
                                text: crate::python::text_saubern(
                                    vm.clone(),
                                    text.trim(),
                                    konfiguration,
                                )
                                .ok()?
                                .into(),
                                automatisch_geroetet: None,
                                manuell_geroetet: None,
                                position_in_pdf: None,
                            })
                        })
                        .collect::<Vec<_>>()
                } else {
                    let mut texte = s.texte.clone();

                    if let Some(t) = texte.get_mut(2 + column_shift) {
                        t.retain(|t| t.text.trim().len() > 12 && t.text.trim().contains(" "));
                    }

                    texte
                        .get(2 + column_shift)
                        .unwrap_or(&default_texte)
                        .iter()
                        .enumerate()
                        .filter_map(|(text_num, text)| {
                            let text_start_y = text.start_y;
                            let text_end_y = text.end_y;

                            // TODO: auch texte "1-3"
                            let lfd_nr = get_erster_text_bei_ca(
                                &texte.get(0 + column_shift).unwrap_or(&default_texte),
                                text_num,
                                text_start_y,
                                text_end_y,
                            )
                            .map(|s| s.text.trim().to_string())
                            .unwrap_or_default();

                            let betrag = get_erster_text_bei_ca(
                                &texte.get(1 + column_shift).unwrap_or(&default_texte),
                                text_num,
                                text_start_y,
                                text_end_y,
                            )
                            .map(|s| s.text.trim().to_string())
                            .unwrap_or_default();

                            Some(Abt3Loeschung {
                                lfd_nr: lfd_nr.into(),
                                betrag: betrag.into(),
                                text: crate::python::text_saubern(
                                    vm.clone(),
                                    text.text.trim(),
                                    konfiguration,
                                )
                                .ok()?
                                .into(),
                                automatisch_geroetet: None,
                                manuell_geroetet: None,
                                position_in_pdf: None,
                            })
                        })
                        .collect::<Vec<_>>()
                }
            } else {
                Vec::new()
            }
            .into_iter()
        })
        .collect();

    Ok(Abteilung3 {
        eintraege: abt3_eintraege,
        veraenderungen: abt3_veraenderungen,
        loeschungen: abt3_loeschungen,
    })
}

fn get_erster_text_bei_ca(
    texte: &[Textblock],
    skip: usize,
    start: f32,
    ziel: f32,
) -> Option<&Textblock> {
    for t in texte.iter().skip(skip.saturating_sub(1)) {
        let start = start - 20.0;
        // let ziel = ziel + 20.0;
        if t.start_y > start || !(t.end_y < start) {
            return Some(t);
        }
    }

    None
}
