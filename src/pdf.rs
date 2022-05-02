use crate::{Titelblatt, Grundbuch, Abt2Eintrag, Abt3Eintrag};
use crate::digitalisiere::BvEintrag;
use printpdf::{
    BuiltinFont, PdfDocument, Mm, IndirectFontRef,
    PdfDocumentReference, PdfLayerReference,
    Line, Point, Color, Cmyk, Pt,
};
use std::path::Path;
use std::collections::BTreeMap;

pub struct GrundbuchExportConfig {
    pub exportiere: PdfExportTyp,
    pub optionen: GenerateGrundbuchConfig,
}

pub enum PdfExportTyp {
    OffenesGrundbuch(Grundbuch),
    AlleOffenDigitalisiert(Vec<Grundbuch>),
    AlleOffen(Vec<Grundbuch>),
    AlleOriginalPdf(Vec<String>),
}

pub enum GenerateGrundbuchConfig {
    EinzelneDatei {
        datei: String,
        exportiere_bv: bool,
        exportiere_abt1: bool,
        exportiere_abt2: bool,
        exportiere_abt3: bool,
        leere_seite_nach_titelblatt: bool,
        mit_geroeteten_eintraegen: bool,
    },
    MehrereDateien {
        ordner: String,
        exportiere_bv: bool,
        exportiere_abt1: bool,
        exportiere_abt2: bool,
        exportiere_abt3: bool,
        leere_seite_nach_titelblatt: bool,
        mit_geroeteten_eintraegen: bool,
    }
}

impl GenerateGrundbuchConfig {
    pub fn get_options(&self) -> PdfGrundbuchOptions {
        match self {
            GenerateGrundbuchConfig::EinzelneDatei {
                exportiere_bv,
                exportiere_abt1,
                exportiere_abt2,
                exportiere_abt3,
                leere_seite_nach_titelblatt,
                mit_geroeteten_eintraegen,
                ..
            } | GenerateGrundbuchConfig::MehrereDateien {
                exportiere_bv,
                exportiere_abt1,
                exportiere_abt2,
                exportiere_abt3,
                leere_seite_nach_titelblatt,
                mit_geroeteten_eintraegen,
                ..
            } => {
                PdfGrundbuchOptions {
                    exportiere_bv: *exportiere_bv,
                    exportiere_abt1: *exportiere_abt1,
                    exportiere_abt2: *exportiere_abt2,
                    exportiere_abt3: *exportiere_abt3,
                    leere_seite_nach_titelblatt: *leere_seite_nach_titelblatt,
                    mit_geroeteten_eintraegen: *mit_geroeteten_eintraegen,
                }
            }
        }
    }
}

pub struct PdfGrundbuchOptions {
    exportiere_bv: bool,
    exportiere_abt1: bool,
    exportiere_abt2: bool,
    exportiere_abt3: bool,
    leere_seite_nach_titelblatt: bool,
    mit_geroeteten_eintraegen: bool,
}

struct PdfFonts {
    times: IndirectFontRef,
    times_bold: IndirectFontRef,
    courier_bold: IndirectFontRef,
    helvetica: IndirectFontRef,
}

impl PdfFonts {
    fn new(doc: &mut PdfDocumentReference) -> Self {
        Self {
            times_bold: doc.add_builtin_font(BuiltinFont::TimesBoldItalic).unwrap(),
            times: doc.add_builtin_font(BuiltinFont::TimesItalic).unwrap(),
            courier_bold: doc.add_builtin_font(BuiltinFont::CourierBold).unwrap(),
            helvetica: doc.add_builtin_font(BuiltinFont::HelveticaBold).unwrap(),
        }
    }
}

pub fn export_grundbuch(config: GrundbuchExportConfig) -> Result<(), String> {
    match config.optionen {
        GenerateGrundbuchConfig::EinzelneDatei { ref datei, ..  } => {
            export_grundbuch_single_file(config.optionen.get_options(), &config.exportiere, datei.clone())
        },
        GenerateGrundbuchConfig::MehrereDateien { ref ordner, ..  } => {
            export_grundbuch_multi_files(config.optionen.get_options(), &config.exportiere, ordner.clone())
        },
    }
}

fn export_grundbuch_single_file(options: PdfGrundbuchOptions, source: &PdfExportTyp, datei: String) -> Result<(), String> {
    match source {
        PdfExportTyp::AlleOriginalPdf(gb) => {
            let mut files = Vec::new();
            
            for d in gb {
                let bytes = std::fs::read(&d).map_err(|e| format!("Fehler: {}: {}", d, e))?;
                let document = lopdf::Document::load_mem(&bytes).map_err(|e| format!("Fehler: {}: {}", d, e))?;
                files.push(document);
            }
            
            let merged = merge_pdf_files(files).map_err(|e| format!("Fehler: {}: {}", datei, e))?;
            
            let _ = std::fs::write(Path::new(&datei), &merged)
            .map_err(|e| format!("Fehler: {}: {}", datei, e))?;
        },
        PdfExportTyp::OffenesGrundbuch(gb) => {
            
            let grundbuch_von = gb.titelblatt.grundbuch_von.clone();
            let blatt = gb.titelblatt.blatt;
            let amtsgericht = gb.titelblatt.amtsgericht.clone();
    
            let titel = format!("{grundbuch_von} Blatt {blatt} (Amtsgericht {amtsgericht})");
            let (mut doc, page1, layer1) = PdfDocument::new(&titel, Mm(210.0), Mm(297.0), "Titelblatt");
            let titelblatt = format!("{}_{}", gb.titelblatt.grundbuch_von, gb.titelblatt.blatt);
            let fonts = PdfFonts::new(&mut doc);
            
            write_titelblatt(&mut doc.get_page(page1).get_layer(layer1), &fonts, &gb.titelblatt);
            if options.leere_seite_nach_titelblatt {
                // Leere Seite 2
                let (_, _) = doc.add_page(Mm(210.0), Mm(297.0), "Formular");
            }
            write_grundbuch(&mut doc, &gb, &fonts, &options);
            
            let bytes = doc.save_to_bytes().unwrap_or_default();
            let _ = std::fs::write(Path::new(&datei), &bytes) 
            .map_err(|e| format!("Fehler: {}: {}", titelblatt, e))?;
        },
        PdfExportTyp::AlleOffenDigitalisiert(gb) | PdfExportTyp::AlleOffen(gb) => {
           
            let titel = Path::new(&datei).file_name().map(|f| format!("{}", f.to_str().unwrap_or(""))).unwrap_or_default();
            let (mut doc, page1, layer1) = PdfDocument::new(&titel, Mm(210.0), Mm(297.0), "Titelblatt");
            let fonts = PdfFonts::new(&mut doc);
            
            for f in gb {
                write_titelblatt(&mut doc.get_page(page1).get_layer(layer1), &fonts, &f.titelblatt);
                if options.leere_seite_nach_titelblatt {
                    // Leere Seite 2
                    let (_, _) = doc.add_page(Mm(210.0), Mm(297.0), "Formular");
                }
                write_grundbuch(&mut doc, &f, &fonts, &options);
            }
            
            let bytes = doc.save_to_bytes().unwrap_or_default();
            let _ = std::fs::write(Path::new(&datei), &bytes) 
            .map_err(|e| format!("Fehler: {}: {}", titel, e))?;
        },
    }
    
    Ok(())
}

fn export_grundbuch_multi_files(
    options: PdfGrundbuchOptions, 
    source: &PdfExportTyp, 
    ordner: String
) -> Result<(), String> {
    
    match source {
        PdfExportTyp::AlleOriginalPdf(gb) => {
            for datei in gb {
                let titelblatt = Path::new(&datei).file_name().map(|f| format!("{}", f.to_str().unwrap_or(""))).unwrap_or_default();
                let target_path = Path::new(&ordner).join(&format!("{titelblatt}.pdf"));
                let _ = std::fs::copy(Path::new(&datei), target_path) 
                .map_err(|e| format!("Fehler: {}: {}", titelblatt, e))?;
            }
        },
        PdfExportTyp::OffenesGrundbuch(gb) => {
        
            let grundbuch_von = gb.titelblatt.grundbuch_von.clone();
            let blatt = gb.titelblatt.blatt;
            let amtsgericht = gb.titelblatt.amtsgericht.clone();
            
            let titel = format!("{grundbuch_von} Blatt {blatt} (Amtsgericht {amtsgericht})");
            let (mut doc, page1, layer1) = PdfDocument::new(&titel, Mm(210.0), Mm(297.0), "Titelblatt");
            let titelblatt = format!("{}_{}", gb.titelblatt.grundbuch_von, gb.titelblatt.blatt);
            let target_path = Path::new(&ordner).join(&format!("{titelblatt}.pdf"));
            let fonts = PdfFonts::new(&mut doc);
            
            write_titelblatt(&mut doc.get_page(page1).get_layer(layer1), &fonts, &gb.titelblatt);
            if options.leere_seite_nach_titelblatt {
                // Leere Seite 2
                let (_, _) = doc.add_page(Mm(210.0), Mm(297.0), "Formular");
            }
            write_grundbuch(&mut doc, &gb, &fonts, &options);
            
            let bytes = doc.save_to_bytes().unwrap_or_default();
            let _ = std::fs::write(target_path, &bytes) 
            .map_err(|e| format!("Fehler: {}: {}", titelblatt, e))?;
        },
        PdfExportTyp::AlleOffenDigitalisiert(gb) | PdfExportTyp::AlleOffen(gb) => {
            for f in gb {
                let grundbuch_von = f.titelblatt.grundbuch_von.clone();
                let blatt = f.titelblatt.blatt;
                let amtsgericht = f.titelblatt.amtsgericht.clone();
                let titel = format!("{grundbuch_von} Blatt {blatt} (Amtsgericht {amtsgericht})");
            
                let titelblatt = format!("{}_{}", f.titelblatt.grundbuch_von, f.titelblatt.blatt);
                let target_path = Path::new(&ordner).join(&format!("{titelblatt}.pdf"));
                let (mut doc, page1, layer1) = PdfDocument::new(&titel, Mm(210.0), Mm(297.0), "Titelblatt");
                let target_path = Path::new(&ordner).join(&format!("{titelblatt}.pdf"));
                let fonts = PdfFonts::new(&mut doc);
            
                write_titelblatt(&mut doc.get_page(page1).get_layer(layer1), &fonts, &f.titelblatt);          
                if options.leere_seite_nach_titelblatt {
                    // Leere Seite 2
                    let (_, _) = doc.add_page(Mm(210.0), Mm(297.0), "Formular");
                }
                write_grundbuch(&mut doc, &f, &fonts, &options);
                
                let bytes = doc.save_to_bytes().unwrap_or_default();
                let _ = std::fs::write(target_path, &bytes) 
                .map_err(|e| format!("Fehler: {}: {}", titelblatt, e))?;
            }
        },
    }
    
    Ok(())
}


fn write_titelblatt(
    current_layer: &mut PdfLayerReference, 
    fonts: &PdfFonts,
    titelblatt: &Titelblatt,
) {
    let grundbuch_von = titelblatt.grundbuch_von.clone();
    let blatt =  titelblatt.blatt;
    let amtsgericht = titelblatt.amtsgericht.clone();
    
    let gb = format!("Grundbuch von {grundbuch_von}");
    let blatt_nr = format!("Blatt {blatt}");
    let amtsgericht = format!("Amtsgericht {amtsgericht}");
    
    // text, font size, x from left edge, y from bottom edge, font
    let start = Mm(297.0 / 2.0);
    let rand_x = Mm(25.0);
    current_layer.use_text(&gb, 22.0, Mm(25.0), start, &fonts.times_bold);
    current_layer.add_shape(Line {
        points: vec![
            (Point::new(rand_x, start - Mm(4.5)), false),
            (Point::new(rand_x + Mm(25.0), start - Mm(4.5)), false)
        ],
        is_closed: false,
        has_fill: false,
        has_stroke: true,
        is_clipping_path: false,
    });
        
    current_layer.use_text(&blatt_nr, 16.0, Mm(25.0), start - Mm(12.0), &fonts.times);
    current_layer.use_text(&amtsgericht, 16.0, Mm(25.0), start - Mm(18.0), &fonts.times);
}

fn write_grundbuch(
    doc: &mut PdfDocumentReference, 
    grundbuch: &Grundbuch, 
    fonts: &PdfFonts,
    options: &PdfGrundbuchOptions
) {
    let grundbuch_von = grundbuch.titelblatt.grundbuch_von.clone();
    let blatt =  grundbuch.titelblatt.blatt;
    let amtsgericht = grundbuch.titelblatt.amtsgericht.clone();
    
    let gb = format!("Grundbuch von {grundbuch_von}");
    let blatt_nr = format!("Blatt {blatt}");
    let amtsgericht = format!("Amtsgericht {amtsgericht}");

    let text_rows = get_text_rows(grundbuch, options);
    render_text_rows(doc, fonts, &text_rows);
}

#[derive(Debug, Clone, PartialEq)]
pub struct PdfTextRow {
    pub texts: Vec<String>,
    pub header: PdfHeader,
    pub geroetet: bool,
    pub teil_geroetet: BTreeMap<usize, String>,
}

const EXTENT_PER_LINE: f32 = 3.43;
const RED: Cmyk = Cmyk { c: 0.0, m: 0.7, y: 0.4, k: 0.0, icc_profile: None };
const BLACK: Cmyk = Cmyk { c: 0.0, m: 0.0, y: 0.0, k: 1.0, icc_profile: None };

impl PdfTextRow {
    
    pub fn get_height_mm(&self) -> f32 {
        self.texts
        .iter()
        .enumerate()
        .map(|(col_id, text)| {
            let max_col_width_for_column = self.header.get_max_col_width(col_id);
            let text_broken_lines = wordbreak_text(&text, max_col_width_for_column);
            text_broken_lines.lines().count() as f32 * EXTENT_PER_LINE
        })
        .map(|s| (s * 1000.0).round() as usize)
        .max()
        .unwrap_or(0) as f32 / 1000.0_f32
    }
    
    fn add_to_page(&self, layer: &mut PdfLayerReference, fonts: &PdfFonts, y_start: f32) {
        
        if self.geroetet {
            layer.set_fill_color(Color::Cmyk(RED));
        } else {
            layer.set_fill_color(Color::Cmyk(BLACK));
        }
        
        layer.set_font(&fonts.courier_bold, 10.0);
        layer.set_line_height(10.0);

        let max_width_mm = self.header.get_max_width_mm();
        
        let x_start_mm = self.header.get_starting_x_spalte_mm(0);
        
        for (col_id, text) in self.texts.iter().enumerate() {
            let max_col_width_for_column = self.header.get_max_col_width(col_id);
            let text_broken_lines = wordbreak_text(&text, max_col_width_for_column);
            
            layer.begin_text_section();
            layer.set_text_cursor(
                Mm((self.header.get_starting_x_spalte_mm(col_id) + 1.0) as f64),
                Mm(y_start as f64),
            );
            
            for line in text_broken_lines.lines() {
                layer.write_text(line.clone(), &fonts.courier_bold);
                layer.add_line_break();
            }
            
            layer.end_text_section();
        }
        
        if self.geroetet {
            
            let self_height = self.get_height_mm();
            
            layer.add_shape(Line {
                points: vec![
                    (Point::new(Mm(x_start_mm as f64), Mm(y_start as f64) - Mm(1.0)), false),
                    (Point::new(Mm((x_start_mm + max_width_mm) as f64), Mm(y_start as f64) - Mm(1.0)), false),
                    (Point::new(Mm(x_start_mm as f64), Mm((y_start + self_height) as f64) + Mm(1.0)), false),
                    (Point::new(Mm((x_start_mm + max_width_mm) as f64), Mm((y_start + self_height) as f64) + Mm(1.0)), false),
                ],
                is_closed: false,
                has_fill: false,
                has_stroke: true,
                is_clipping_path: false,
            });

            layer.set_fill_color(Color::Cmyk(BLACK));
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PdfHeader {
    Bestandsverzeichnis,
    Abteilung1,
    Abteilung2,
    Abteilung3,
}

fn pt_to_mm(pt: Pt) -> Mm { pt.into() }

impl PdfHeader {

    pub fn get_max_col_width(&self, col_id: usize) -> usize {
        use self::PdfHeader::*;
        let spalten_lines = self.get_spalten_lines();
        ((if col_id == spalten_lines.len() - 1 {
            self.get_starting_x_spalte_mm(0) + 
            self.get_max_width_mm() - 
            self.get_starting_x_spalte_mm(col_id)
        } else {
            self.get_starting_x_spalte_mm(col_id + 1) -
            self.get_starting_x_spalte_mm(col_id)
        } / EXTENT_PER_LINE).floor() as usize).max(3)
    }
    
    fn add_to_page(&self, layer: &mut PdfLayerReference, fonts: &PdfFonts) {
        match self {
            PdfHeader::Bestandsverzeichnis => {
            
                layer.use_text(
                    "Bestandsverzeichnis", 
                    16.0, 
                    Mm(10.0), 
                    Mm(297.0 - 16.0), 
                    &fonts.times_bold
                );
        
                let text_1 = &[
                    ("Laufende",    13.0_f64, 297.0_f64 - 21.0), 
                    ("Nummer",      13.5,     297.0 - 23.5),
                    ("der",         15.5,     297.0 - 26.0), 
                    ("Grund-",      14.0,     297.0 - 28.5), 
                    ("stücke",      14.0,     297.0 - 31.0),
                ];
                
                let text_1_header = Line {
                    points: vec![
                        (Point::new(Mm(10.0), Mm(297.0 - 18.5)), false),
                        (Point::new(Mm(10.0), Mm(297.0 - 32.0)), false),
                        (Point::new(Mm(25.0), Mm(297.0 - 32.0)), false),
                        (Point::new(Mm(25.0), Mm(297.0 - 18.5)), false)
                    ],
                    is_closed: true,
                    has_fill: false,
                    has_stroke: true,
                    is_clipping_path: false,
                };
                
                layer.add_shape(text_1_header);
                
                for (t, x, y) in text_1.iter() {
                    layer.use_text(*t, 6.0, Mm(*x), Mm(*y), &fonts.helvetica);        
                }
                
                // ...
                
            },
            PdfHeader::Abteilung1 => {
            
            },
            PdfHeader::Abteilung2 => {
            
            },
            PdfHeader::Abteilung3 => {
            
            },
        }
    }
    
    pub fn get_starting_x_spalte_mm(&self, spalte_idx: usize) -> f32 {
        self.get_spalten_lines()
        .get(spalte_idx)
        .and_then(|line| Some(line.points.get(0)?.0.x.0 as f32))
        .unwrap_or(0.0)
    }
    
    pub fn get_max_width_mm(&self) -> f32 {
        let start_erste_spalte = self.get_starting_x_spalte_mm(0);
        
        let letzte_spalte_x = self.get_spalten_lines()
        .last()
        .map(|last| {
            last.points.iter()
            .map(|(p, _)| { (pt_to_mm(p.x).0 * 1000.0) as usize })
            .max()
            .unwrap_or((start_erste_spalte * 1000.0) as usize) as f32 / 1000.0
        }).unwrap_or(start_erste_spalte);
        
        letzte_spalte_x - start_erste_spalte
    }
    
    pub fn get_start_y(&self) -> f32 {
        self.get_spalten_lines()
        .iter()
        .map(|line| {
            line.points.iter()
            .map(|(p, _)| { (pt_to_mm(p.y).0 * 1000.0) as usize })
            .max()
            .unwrap_or(0)
        })
        .max()
        .unwrap_or(0) as f32 / 1000.0
    }
    
    pub fn get_end_y(&self) -> f32 {
        self.get_spalten_lines()
        .iter()
        .map(|line| {
            line.points.iter()
            .map(|(p, _)| { (pt_to_mm(p.y).0 * 1000.0) as usize })
            .min()
            .unwrap_or(0)
        })
        .min()
        .unwrap_or(0) as f32 / 1000.0
    }
    
    fn get_spalten_lines(&self) -> Vec<Line> {
        
        let mut spalten_lines = Vec::new();
        
        match self {
            PdfHeader::Bestandsverzeichnis => {
                
                let lfd_nr_spalte = Line {
                    points: vec![
                        (Point::new(Mm(10.0), Mm(297.0 - 36.0)), false),
                        (Point::new(Mm(10.0), Mm(10.0)), false),
                        (Point::new(Mm(25.0), Mm(10.0)), false),
                        (Point::new(Mm(25.0), Mm(297.0 - 36.0)), false)
                    ],
                    is_closed: true,
                    has_fill: false,
                    has_stroke: true,
                    is_clipping_path: false,
                };
                
                spalten_lines.push(lfd_nr_spalte);
                
                let bisherige_lfd_nr_spalte = Line {
                    points: vec![
                        (Point::new(Mm(25.0), Mm(297.0 - 36.0)), false),
                        (Point::new(Mm(25.0), Mm(10.0)), false),
                        (Point::new(Mm(40.0), Mm(10.0)), false),
                        (Point::new(Mm(40.0), Mm(297.0 - 36.0)), false)
                    ],
                    is_closed: true,
                    has_fill: false,
                    has_stroke: true,
                    is_clipping_path: false,
                };
                
                spalten_lines.push(bisherige_lfd_nr_spalte);
                
                let gemarkung_spalte = Line {
                    points: vec![
                        (Point::new(Mm(40.0), Mm(297.0 - 36.0)), false),
                        (Point::new(Mm(40.0), Mm(10.0)), false),
                        (Point::new(Mm(80.0), Mm(10.0)), false),
                        (Point::new(Mm(80.0), Mm(297.0 - 36.0)), false)
                    ],
                    is_closed: true,
                    has_fill: false,
                    has_stroke: true,
                    is_clipping_path: false,
                };
                
                spalten_lines.push(gemarkung_spalte);

                let flur_spalte = Line {
                    points: vec![
                        (Point::new(Mm(80.0), Mm(297.0 - 36.0)), false),
                        (Point::new(Mm(80.0), Mm(10.0)), false),
                        (Point::new(Mm(95.0), Mm(10.0)), false),
                        (Point::new(Mm(95.0), Mm(297.0 - 36.0)), false)
                    ],
                    is_closed: true,
                    has_fill: false,
                    has_stroke: true,
                    is_clipping_path: false,
                };
                
                spalten_lines.push(flur_spalte);

                let flurstueck_spalte = Line {
                    points: vec![
                        (Point::new(Mm(95.0), Mm(297.0 - 36.0)), false),
                        (Point::new(Mm(95.0), Mm(10.0)), false),
                        (Point::new(Mm(115.0), Mm(10.0)), false),
                        (Point::new(Mm(115.0), Mm(297.0 - 36.0)), false)
                    ],
                    is_closed: true,
                    has_fill: false,
                    has_stroke: true,
                    is_clipping_path: false,
                };
                
                spalten_lines.push(flurstueck_spalte);

                let wirtschaftsart_lage_spalte = Line {
                    points: vec![
                        (Point::new(Mm(115.0), Mm(297.0 - 36.0)), false),
                        (Point::new(Mm(115.0), Mm(10.0)), false),
                        (Point::new(Mm(210.0 - 45.0), Mm(10.0)), false),
                        (Point::new(Mm(210.0 - 45.0), Mm(297.0 - 36.0)), false)
                    ],
                    is_closed: true,
                    has_fill: false,
                    has_stroke: true,
                    is_clipping_path: false,
                };
                
                spalten_lines.push(wirtschaftsart_lage_spalte);
                        
                let ha_spalte = Line {
                    points: vec![
                        (Point::new(Mm(210.0 - 45.0), Mm(297.0 - 36.0)), false),
                        (Point::new(Mm(210.0 - 45.0), Mm(10.0)), false),
                        (Point::new(Mm(210.0 - 30.0), Mm(10.0)), false),
                        (Point::new(Mm(210.0 - 30.0), Mm(297.0 - 36.0)), false)
                    ],
                    is_closed: true,
                    has_fill: false,
                    has_stroke: true,
                    is_clipping_path: false,
                };
                
                spalten_lines.push(ha_spalte);

                let a_spalte = Line {
                    points: vec![
                        (Point::new(Mm(210.0 - 30.0), Mm(297.0 - 36.0)), false),
                        (Point::new(Mm(210.0 - 30.0), Mm(10.0)), false),
                        (Point::new(Mm(210.0 - 20.0), Mm(10.0)), false),
                        (Point::new(Mm(210.0 - 20.0), Mm(297.0 - 36.0)), false)
                    ],
                    is_closed: true,
                    has_fill: false,
                    has_stroke: true,
                    is_clipping_path: false,
                };
                
                spalten_lines.push(a_spalte);
                        
                let m2_spalte = Line {
                    points: vec![
                        (Point::new(Mm(210.0 - 20.0), Mm(297.0 - 36.0)), false),
                        (Point::new(Mm(210.0 - 20.0), Mm(10.0)), false),
                        (Point::new(Mm(210.0 - 10.0), Mm(10.0)), false),
                        (Point::new(Mm(210.0 - 10.0), Mm(297.0 - 36.0)), false)
                    ],
                    is_closed: true,
                    has_fill: false,
                    has_stroke: true,
                    is_clipping_path: false,
                };
                
                spalten_lines.push(m2_spalte);
            },
            _ => { } // TODO
        }
        
        spalten_lines
    }
    
    pub fn add_columns_to_page(&self, layer: &mut PdfLayerReference) {
        for l in self.get_spalten_lines() {
            layer.add_shape(l);
        }
    }
}

fn get_text_rows(grundbuch: &Grundbuch, options: &PdfGrundbuchOptions) -> Vec<PdfTextRow> {
    
    let mut rows = Vec::new();
    let mit_geroeteten_eintraegen = options.mit_geroeteten_eintraegen;
    let grundbuch_von = grundbuch.titelblatt.grundbuch_von.clone();

    if options.exportiere_bv {
        for bv in grundbuch.bestandsverzeichnis.eintraege.iter() {
            let ist_geroetet = bv.ist_geroetet();
            if !mit_geroeteten_eintraegen && ist_geroetet { continue; }
            match bv {
                BvEintrag::Flurstueck(flst) => {
                    rows.push(PdfTextRow {
                        texts: vec![
                            format!("{}", flst.lfd_nr),
                            flst.bisherige_lfd_nr.clone().map(|b| format!("{}", b)).unwrap_or_default(),
                            flst.gemarkung.clone().map(|g| if g == grundbuch_von { String::new() } else { g }).unwrap_or_default(),
                            format!("{}", flst.flur),
                            format!("{}", flst.flurstueck),
                            flst.bezeichnung.clone().unwrap_or_default().text(),
                            flst.groesse.get_ha_string(),
                            flst.groesse.get_a_string(),
                            flst.groesse.get_m2_string(),
                        ],
                        header: PdfHeader::Bestandsverzeichnis,
                        geroetet: ist_geroetet,
                        teil_geroetet: BTreeMap::new(),
                    });
                },
                BvEintrag::Recht(hvm) => {
                }
            }
        }
    }
    
    if options.exportiere_abt1 {
        for bv in grundbuch.abt1.eintraege.iter() {
        
        }
    }
    
    if options.exportiere_abt2 {
        for bv in grundbuch.abt2.eintraege.iter() {
        
        }
    }
    
    if options.exportiere_abt3 {
        for bv in grundbuch.abt3.eintraege.iter() {
        
        }
    }
    
    rows
}

fn render_text_rows(doc: &mut PdfDocumentReference, fonts: &PdfFonts, blocks: &[PdfTextRow]) {
    
    if blocks.is_empty() { 
        return; 
    }    
    
    let (mut page, mut layer) = doc.add_page(Mm(210.0), Mm(297.0), "Formular");
    let  current_block = &blocks[0];
    current_block.header.add_to_page(&mut doc.get_page(page).get_layer(layer), fonts);
    current_block.header.add_columns_to_page(&mut doc.get_page(page).get_layer(layer));
    
    let mut current_y = current_block.header.get_start_y();
    println!("current header lines: {:#?}", current_block.header.get_spalten_lines());
    println!("current y: {}", current_y);
    current_block.add_to_page(&mut doc.get_page(page).get_layer(layer), fonts, current_y);
    current_y -= current_block.get_height_mm();
    println!("current y 2: {}", current_y);
    let mut current_header = current_block.header;
    
    for b in blocks.iter().skip(1) {
        
        if b.header != current_header || current_y - b.get_height_mm() < current_header.get_end_y() {
            current_header = b.header;
            current_y = b.header.get_start_y();
            let (new_page, new_layer) = doc.add_page(Mm(210.0), Mm(297.0), "Formular");
            b.header.add_to_page(&mut doc.get_page(new_page).get_layer(new_layer), fonts);
            b.header.add_columns_to_page(&mut doc.get_page(new_page).get_layer(new_layer));
            page = new_page;
            layer = new_layer;
        }
        
        b.add_to_page(&mut doc.get_page(page).get_layer(layer), fonts, current_y);
        current_y -= b.get_height_mm();
        println!("current y loop: {}", current_y);
    }
}

// https://www.dariocancelliere.it/blog/2020/09/29/pdf-manipulation-with-rust-and-considerations
fn merge_pdf_files(documents: Vec<lopdf::Document>) -> Result<Vec<u8>, String> {
    
    use lopdf::{Document, Object, ObjectId};
    use std::io::BufWriter;
    
    // Define a starting max_id (will be used as start index for object_ids)
    let mut max_id = 1;

    // Collect all Documents Objects grouped by a map
    let mut documents_pages = BTreeMap::new();
    let mut documents_objects = BTreeMap::new();

    for mut document in documents {
        document.renumber_objects_with(max_id);

        max_id = document.max_id + 1;

        documents_pages.extend(
            document
                    .get_pages()
                    .into_iter()
                    .map(|(_, object_id)| {
                        (
                            object_id,
                            document.get_object(object_id).unwrap().to_owned(),
                        )
                    })
                    .collect::<BTreeMap<ObjectId, Object>>(),
        );
        documents_objects.extend(document.objects);
    }

    // Initialize a new empty document
    let mut document = Document::with_version("1.5");

    // Catalog and Pages are mandatory
    let mut catalog_object: Option<(ObjectId, Object)> = None;
    let mut pages_object: Option<(ObjectId, Object)> = None;

    // Process all objects except "Page" type
    for (object_id, object) in documents_objects.iter() {
        // We have to ignore "Page" (as are processed later), "Outlines" and "Outline" objects
        // All other objects should be collected and inserted into the main Document
        match object.type_name().unwrap_or("") {
            "Catalog" => {
                // Collect a first "Catalog" object and use it for the future "Pages"
                catalog_object = Some((
                    if let Some((id, _)) = catalog_object {
                        id
                    } else {
                        *object_id
                    },
                    object.clone(),
                ));
            }
            "Pages" => {
                // Collect and update a first "Pages" object and use it for the future "Catalog"
                // We have also to merge all dictionaries of the old and the new "Pages" object
                if let Ok(dictionary) = object.as_dict() {
                    let mut dictionary = dictionary.clone();
                    if let Some((_, ref object)) = pages_object {
                        if let Ok(old_dictionary) = object.as_dict() {
                            dictionary.extend(old_dictionary);
                        }
                    }

                    pages_object = Some((
                        if let Some((id, _)) = pages_object {
                            id
                        } else {
                            *object_id
                        },
                        Object::Dictionary(dictionary),
                    ));
                }
            }
            "Page" => {}     // Ignored, processed later and separately
            "Outlines" => {} // Ignored, not supported yet
            "Outline" => {}  // Ignored, not supported yet
            _ => {
                document.objects.insert(*object_id, object.clone());
            }
        }
    }

    // If no "Pages" found abort
    if pages_object.is_none() {
        return Err(format!("Pages root not found."));
    }

    // Iter over all "Page" and collect with the parent "Pages" created before
    for (object_id, object) in documents_pages.iter() {
        if let Ok(dictionary) = object.as_dict() {
            let mut dictionary = dictionary.clone();
            dictionary.set("Parent", pages_object.as_ref().unwrap().0);

            document
                    .objects
                    .insert(*object_id, Object::Dictionary(dictionary));
        }
    }

    // If no "Catalog" found abort
    if catalog_object.is_none() {
        return Err(format!("Catalog root not found."));
    }

    let catalog_object = catalog_object.unwrap();
    let pages_object = pages_object.unwrap();

    // Build a new "Pages" with updated fields
    if let Ok(dictionary) = pages_object.1.as_dict() {
        let mut dictionary = dictionary.clone();

        // Set new pages count
        dictionary.set("Count", documents_pages.len() as u32);

        // Set new "Kids" list (collected from documents pages) for "Pages"
        dictionary.set(
            "Kids",
            documents_pages
                    .into_iter()
                    .map(|(object_id, _)| Object::Reference(object_id))
                    .collect::<Vec<_>>(),
        );

        document
                .objects
                .insert(pages_object.0, Object::Dictionary(dictionary));
    }

    // Build a new "Catalog" with updated fields
    if let Ok(dictionary) = catalog_object.1.as_dict() {
        let mut dictionary = dictionary.clone();
        dictionary.set("Pages", pages_object.0);
        dictionary.remove(b"Outlines"); // Outlines not supported in merged PDFs

        document
                .objects
                .insert(catalog_object.0, Object::Dictionary(dictionary));
    }

    document.trailer.set("Root", catalog_object.0);

    // Update the max internal ID as wasn't updated before due to direct objects insertion
    document.max_id = document.objects.len() as u32;

    // Reorder all new Document objects
    document.renumber_objects();
    document.compress();

    // Save the merged PDF
    let mut bytes = Vec::new();
    let mut writer = BufWriter::new(&mut bytes);
    document.save_to(&mut writer)
    .map_err(|e| format!("{}", e))?;
    std::mem::drop(writer);
    Ok(bytes)
}

fn clean_bv(s: &str) -> String {
    let s = s.split(",").collect::<Vec<_>>().join(" ");
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    s
}

// Format a string so that it fits into N characters per line
fn wordbreak_text(s: &str, max_cols: usize) -> String {
    
    let mut lines = s.lines()
    .map(|l| l.split_whitespace().map(|s| s.to_string()).collect::<Vec<_>>())
    .collect::<Vec<_>>();
    
    let mut output = String::new();
    
    for words in lines {
        let mut line_len = 0;

        for w in words {
            
            let word_len = w.chars().count() + 1;
            let (before, after) = split_hyphenate(&w, max_cols.saturating_sub(line_len).saturating_sub(1));
            
            if !before.is_empty() {               
               if !after.is_empty() {
                    output.push_str(&before);
                    output.push_str("-\r\n");
                    output.push_str(&after);
                    output.push_str(" ");
                    line_len = after.chars().count() + 1;
                } else {
                    output.push_str(&before);
                    output.push_str(" ");
                    line_len += before.chars().count() + 1;
                }
            } else if !after.is_empty() {
                output.push_str(&after);
                output.push_str(" ");
                line_len += after.chars().count() + 1;
            }
        }
        
        output.push_str("\r\n");
    }
        
    output.trim().to_string()
}

fn split_hyphenate(word: &str, remaining: usize) -> (String, String) {
    
    if remaining == 0 {
        return (String::new(), word.to_string());
    }
    
    let mut before = String::new();
    let mut after = String::new();
    let mut counter = 0;
    
    for syllable in get_syllables(word) {
        let syllable_len = syllable.chars().count();
        if counter + syllable_len > remaining {
            after.push_str(&syllable);
        } else {
            before.push_str(&syllable);
        }
        counter += syllable_len;
    }
    
    (before, after)
}

fn get_syllables(s: &str) -> Vec<String> {
    let vocals = ['a', 'e', 'i', 'o', 'u', 'ö', 'ä', 'ü', 'y'];
    let vocals2 = ['a', 'e', 'i', 'o', 'u', 'y'];

    let mut results = Vec::new();
    let chars = s.chars().collect::<Vec<_>>();
    let mut current_position = chars.len() - 1;
    let mut last_split = 0;
    for i in 0..current_position {
        if i != 0 && 
            vocals.contains(&chars[i]) && 
            !vocals2.contains(&chars[i - 1]) && 
            i - last_split > 1 {
            let a = &chars[last_split..i];
            let b = &chars[i..];
            last_split = i;
            results.push(a.iter().collect::<String>());
        }
    }
    
    results.push((&chars[last_split..]).iter().collect::<String>());
    
    results
}
