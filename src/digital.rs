use std::{fs, fmt, process::Command};
use std::io::Error as IoError;
use std::collections::BTreeMap;
use chrono::{DateTime, Utc};

use lopdf::Error as LoPdfError;
use image::ImageError;
use serde_derive::{Serialize, Deserialize};
use rayon::prelude::*;
use crate::{
    Rect, AnpassungSeite, Konfiguration,
    get_tesseract_command, get_pdftoppm_command, get_pdftotext_command
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeiteParsed {
    pub typ: SeitenTyp,
    pub texte: Vec<Vec<Textblock>>,
}

/// Alle Fehler, die im Programm passieren können
#[derive(Debug)]
pub enum Fehler {
    // Seite X kann mit pdftotext nicht gelesen werden
    FalscheSeitenZahl(u32),
    // Kann Seite X nicht klassifizieren
    UnbekannterSeitentyp(u32),
    // Fehler beim Auslesen des Titelblatts
    Titelblatt(TitelblattFehler),
    // Datei ist kein PDF
    Pdf(LoPdfError),
    // Fehler bei Bildbearbeitung
    Bild(String, ImageError),
    // Fehler bei Lese- / Schreibvorgang
    Io(String, IoError), // String = FilePath
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
            Fehler::FalscheSeitenZahl(seite) => write!(f, "Seite {} kann mit pdftotext nicht gelesen werden", seite),
            Fehler::UnbekannterSeitentyp(seite) => write!(f, "Kann Seite {} nicht klassifizieren", seite),
            Fehler::Titelblatt(e) => write!(f, "Fehler beim Auslesen des Titelblatts: {}", e),
            Fehler::Pdf(e) => write!(f, "Fehler im PDF: {}", e),
            Fehler::Bild(pfad, e) => write!(f, "Bildbearbeitungsfehler: \"{}\": {}", pfad, e),
            Fehler::Io(pfad, e) => write!(f, "Fehler beim Lesen / Schreiben vom Pfad \"{}\": {}", pfad, e),
        }
    }
}

// Funktion, die prüft, ob die Eingabedatei ein PDF ist und die Seitenzahlen zurückgibt
pub fn lese_seitenzahlen(pdf_bytes: &[u8]) -> Result<Vec<u32>, Fehler> {
    let pdf = lopdf::Document::load_mem(pdf_bytes)?;
    Ok(pdf.get_pages().keys().copied().collect())
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

// Layout mit PdfToText (kein OCR! - schnell, aber nicht alle Rechte vorhanden)
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PdfToTextLayout {
    #[serde(default)]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub seiten: BTreeMap<String, PdfToTextSeite>,
}

impl PdfToTextLayout {
    pub fn is_empty(&self) -> bool { self.seiten.is_empty() }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfToTextSeite {
    pub breite_mm: f32,
    pub hoehe_mm: f32,
    pub texte: Vec<Textblock>,
}

// pdftotext -bbox-layout ./temp.pdf
pub fn get_pdftotext_layout(titelblatt: &Titelblatt, seitenzahlen: &[u32]) -> Result<PdfToTextLayout, Fehler> {

    use kuchiki::traits::TendrilSink;

    let temp_ordner = std::env::temp_dir()
    .join(&format!("{gemarkung}/{blatt}", gemarkung = titelblatt.grundbuch_von, blatt = titelblatt.blatt));
    
    let temp_pdf_path = temp_ordner.clone().join("temp.pdf");
    let temp_html_path = temp_ordner.clone().join(format!("pdftotext.html"));

    // pdftotext -bbox-layout /tmp/temp.pdf -o temp.html
    // to get the layout
    let _ = get_pdftotext_command()
    .arg("-q")
    .arg("-bbox-layout")
    .arg(&format!("{}", temp_pdf_path.display()))     
    .arg(&format!("{}", temp_html_path.display()))     
    .status();
    
    let html_pdftotext = fs::read_to_string(temp_html_path.clone())
        .map_err(|e| Fehler::Io(format!("{}", temp_html_path.display()), e))?;
    
    let _ = fs::remove_file(temp_html_path.clone());
    
    let document = kuchiki::parse_html()
    .one(html_pdftotext.as_str());
    
    let seiten = seitenzahlen
    .iter()
    .filter_map(|sz| {
        let css_selector = format!("page:nth-child(0n+{}) word", sz);
        let flow_nodes = document.select(&css_selector).ok()?;
        
        let texte = flow_nodes.filter_map(|css_match| {
                        
            let as_node = css_match.as_node();
            let as_element = as_node.as_element()?;
            let block_attributes = as_element.attributes.try_borrow().ok()?;
            
            let xmin = (&*block_attributes).get("xmin").and_then(|b| b.parse::<f32>().ok())?;
            let xmax = (&*block_attributes).get("xmax").and_then(|b| b.parse::<f32>().ok())?;
            let ymin = (&*block_attributes).get("ymin").and_then(|b| b.parse::<f32>().ok())?;
            let ymax = (&*block_attributes).get("ymax").and_then(|b| b.parse::<f32>().ok())?;

            let line_text = as_node
                .text_contents()
                .lines()
                .map(|l| l.trim().to_string())
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();

            Some(Textblock {
                text: line_text,
                start_y: ymin,
                end_y: ymax,
                start_x: xmin,
                end_x: xmax,
            })
        }).collect();
        
        let css_selector = format!("page:nth-child(0n+{})", sz);
        let mut seite_node = document.select(&css_selector).ok()?;
        let css_match = seite_node.next()?;
        let as_node = css_match.as_node();
        let as_element = as_node.as_element()?;
        let seite_attributes = as_element.attributes.try_borrow().ok()?;
        let breite_mm = (&*seite_attributes).get("width").and_then(|b| b.parse::<f32>().ok())?;
        let hoehe_mm = (&*seite_attributes).get("height").and_then(|b| b.parse::<f32>().ok())?;

                    
        Some((format!("{}", *sz), PdfToTextSeite { breite_mm, hoehe_mm, texte, }))        
    })
    .collect();
    
    Ok(PdfToTextLayout { seiten })
}

// Funktion, die das Titelblatt ausliest
pub fn lese_titelblatt(pdf_bytes: &[u8]) -> Result<Titelblatt, Fehler> {

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
}

pub fn konvertiere_pdf_seite_zu_png_prioritaet(pdf_bytes: &[u8], seitenzahlen: &[u32], titelblatt: &Titelblatt, geroetet: bool) -> Result<(), Fehler> {
    
    use std::path::Path;
    
    let temp_ordner = std::env::temp_dir()
    .join(&format!("{gemarkung}/{blatt}", gemarkung = titelblatt.grundbuch_von, blatt = titelblatt.blatt));
    
    let max_sz = seitenzahlen.iter().cloned().max().unwrap_or(0);
    
    let _ = fs::create_dir_all(temp_ordner.clone())
        .map_err(|e| Fehler::Io(format!("{}", temp_ordner.clone().display()), e))?;

    for sz in seitenzahlen {
        
        if geroetet {
        
            let temp_pdf_pfad = temp_ordner.clone().join("temp.pdf");
    
            if !Path::new(&temp_pdf_pfad).exists() {
                fs::write(temp_pdf_pfad.clone(), pdf_bytes)
                    .map_err(|e| Fehler::Io(format!("{}", temp_pdf_pfad.display()), e))?;
            }
        
            let pdftoppm_output_path = temp_ordner.clone().join(format!("page-{}.png", formatiere_seitenzahl(*sz, max_sz)));
        
            if !pdftoppm_output_path.exists() {
                // pdftoppm -q -r 600 -png -f 1 -l 1 /tmp/Ludwigsburg/17/temp.pdf /tmp/Ludwigsburg/17/test
                // writes result to /tmp/test-01.png
                let _ = get_pdftoppm_command()
                .arg("-q")
                .arg("-r")
                .arg("600") // 600 DPI
                .arg("-png")
                .arg("-f")
                .arg(&format!("{}", sz))
                .arg("-l")
                .arg(&format!("{}", sz))
                .arg(&format!("{}", temp_pdf_pfad.display()))     
                .arg(&format!("{}", temp_ordner.clone().join(format!("page")).display()))     
                .status();
            }
                
                
        } else {
                
            let temp_clean_pdf_pfad = temp_ordner.clone().join("temp-clean.pdf");
            if !Path::new(&temp_clean_pdf_pfad).exists() {
                let pdf_clean = clean_pdf(pdf_bytes, titelblatt)?;
            
                fs::write(temp_clean_pdf_pfad.clone(), pdf_clean)
                    .map_err(|e| Fehler::Io(format!("{}", temp_clean_pdf_pfad.display()), e))?;
            }
            
            let pdftoppm_clean_output_path = temp_ordner.clone().join(format!("page-clean-{}.png", formatiere_seitenzahl(*sz, max_sz)));
                    
            if !pdftoppm_clean_output_path.exists() {
                    
                // pdftoppm -q -r 600 -png -f 1 -l 1 /tmp/Ludwigsburg/17/temp.pdf /tmp/Ludwigsburg/17/test
                // writes result to /tmp/page-clean-01.png
                let _ = get_pdftoppm_command()
                .arg("-q")
                .arg("-r")
                .arg("600") // 600 DPI
                .arg("-png")
                .arg("-f")
                .arg(&format!("{}", sz))
                .arg("-l")
                .arg(&format!("{}", sz))
                .arg(&format!("{}", temp_clean_pdf_pfad.display()))     
                .arg(&format!("{}", temp_ordner.clone().join(format!("page-clean")).display()))     
                .status();
            }
        }
    }
    
    Ok(())
}

// Konvertiert alle Seiten zu PNG Dateien (für Schrifterkennung)
pub fn konvertiere_pdf_seiten_zu_png(pdf_bytes: &[u8], seitenzahlen: &[u32], max_sz: u32, titelblatt: &Titelblatt) -> Result<(), Fehler> {
    
    use std::path::Path;
    
    let temp_ordner = std::env::temp_dir()
    .join(&format!("{gemarkung}/{blatt}", gemarkung = titelblatt.grundbuch_von, blatt = titelblatt.blatt));
        
    let _ = fs::create_dir_all(temp_ordner.clone())
        .map_err(|e| Fehler::Io(format!("{}", temp_ordner.clone().display()), e))?;

    let temp_pdf_pfad = temp_ordner.clone().join("temp.pdf");
    
    if !Path::new(&temp_pdf_pfad).exists() {
        fs::write(temp_pdf_pfad.clone(), pdf_bytes)
            .map_err(|e| Fehler::Io(format!("{}", temp_pdf_pfad.display()), e))?;
    }
    
    let temp_clean_pdf_pfad = temp_ordner.clone().join("temp-clean.pdf");
    if !Path::new(&temp_clean_pdf_pfad).exists() {
        let pdf_clean = clean_pdf(pdf_bytes, titelblatt)?;
    
        fs::write(temp_clean_pdf_pfad.clone(), pdf_clean)
            .map_err(|e| Fehler::Io(format!("{}", temp_clean_pdf_pfad.display()), e))?;
    }
        
    seitenzahlen
    .par_iter()
    .for_each(|sz| {
    
        let pdftoppm_output_path = temp_ordner.clone().join(format!("page-{}.png", formatiere_seitenzahl(*sz, max_sz)));
        
        if !pdftoppm_output_path.exists() {
            println!("{} existiert NICHT!", pdftoppm_output_path.display());

            // pdftoppm -q -r 600 -png -f 1 -l 1 /tmp/Ludwigsburg/17/temp.pdf /tmp/Ludwigsburg/17/test
            // writes result to /tmp/test-01.png
            let _ = get_pdftoppm_command()
            .arg("-q")
            .arg("-r")
            .arg("600") // 600 DPI
            .arg("-png")
            .arg("-f")
            .arg(&format!("{}", sz))
            .arg("-l")
            .arg(&format!("{}", sz))
            .arg(&format!("{}", temp_pdf_pfad.display()))     
            .arg(&format!("{}", temp_ordner.clone().join(format!("page")).display()))     
            .status();
        } else {
            println!("{} existiert!", pdftoppm_output_path.display());
        }
                
        let pdftoppm_clean_output_path = temp_ordner.clone().join(format!("page-clean-{}.png", formatiere_seitenzahl(*sz, max_sz)));
                
        if !pdftoppm_clean_output_path.exists() {
            
            println!("{} existiert NICHT!", pdftoppm_clean_output_path.display());

            // pdftoppm -q -r 600 -png -f 1 -l 1 /tmp/Ludwigsburg/17/temp.pdf /tmp/Ludwigsburg/17/test
            // writes result to /tmp/page-clean-01.png
            let _ = get_pdftoppm_command()
            .arg("-q")
            .arg("-r")
            .arg("600") // 600 DPI
            .arg("-png")
            .arg("-f")
            .arg(&format!("{}", sz))
            .arg("-l")
            .arg(&format!("{}", sz))
            .arg(&format!("{}", temp_clean_pdf_pfad.display()))     
            .arg(&format!("{}", temp_ordner.clone().join(format!("page-clean")).display()))     
            .status();
        } else {
            println!("{} existiert!", pdftoppm_clean_output_path.display());
        }
    });
    
    Ok(())
}

pub fn ocr_seite(titelblatt: &Titelblatt, seitenzahl: u32, max_seitenzahl: u32) -> Result<String, Fehler> {
        
    let temp_ordner = std::env::temp_dir()
    .join(&format!("{gemarkung}/{blatt}", gemarkung = titelblatt.grundbuch_von, blatt = titelblatt.blatt));
    
    let pdftoppm_output_path = temp_ordner.clone().join(format!("page-clean-{}.png", formatiere_seitenzahl(seitenzahl, max_seitenzahl)));
    let tesseract_output_path = temp_ordner.clone().join(format!("tesseract-{:02}.txt", seitenzahl));

    if !tesseract_output_path.exists() {
        // tesseract ./test-01.png ./tesseract-01 -l deu -c preserve_interword_spaces=1
        // Ausgabe -> /tmp/tesseract-01.txt
        let _ = get_tesseract_command()
        .arg(&format!("{}", pdftoppm_output_path.display()))
        .arg(&format!("{}", temp_ordner.clone().join(format!("tesseract-{:02}", seitenzahl)).display()))     
        .arg("-l")
        .arg("deu")
        .arg("--dpi")
        .arg("600")
        .arg("-c")
        .arg("preserve_interword_spaces=1")
        .arg("-c")
        .arg("debug_file=/dev/null") // TODO: funktioniert nur auf Linux!
        .status();
    }
    
    std::fs::read_to_string(tesseract_output_path.clone())
        .map_err(|e| Fehler::Io(format!("{}", tesseract_output_path.display()), e))
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
	
    #[serde(rename = "abt1-horz")]
	Abt1Horz,
    #[serde(rename = "abt1-vert")]
	Abt1Vert,
	
    #[serde(rename = "abt2-horz-veraenderungen")]
	Abt2HorzVeraenderungen,
    #[serde(rename = "abt2-horz")]
	Abt2Horz,
    #[serde(rename = "abt2-vert-veraenderungen")]
	Abt2VertVeraenderungen,
    #[serde(rename = "abt2-vert")]
	Abt2Vert,

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

// Bestimmt den Seitentyp anhand des OCR-Textes der gesamten Seite
pub fn klassifiziere_seitentyp(
    titelblatt: &Titelblatt, 
    seitenzahl: u32, 
    max_sz: u32, 
    ocr_text: Option<&String>
) -> Result<SeitenTyp, Fehler> {
    
    // Um die Seite zu erkennen, müssen wir erst den Typ der Seite erkennen 
    //
    // Der OCR-Text (wenn auch nicht genau), enthält üblicherweise bereits den Typ der Seite

    let temp_ordner = std::env::temp_dir()
    .join(&format!("{gemarkung}/{blatt}", gemarkung = titelblatt.grundbuch_von, blatt = titelblatt.blatt));
    
    let pdftoppm_output_path = temp_ordner.clone().join(format!("page-clean-{}.png", formatiere_seitenzahl(seitenzahl, max_sz)));
    
    let (w, h) = image::image_dimensions(pdftoppm_output_path.clone())
        .map_err(|e| Fehler::Bild(format!("{}", pdftoppm_output_path.display()), e))?;
    
    let is_landscape_page = w > h;
            
    let ocr_text = match ocr_text {
        Some(s) => s.clone(),
        None => {            
            let tesseract_output_path = temp_ordner.clone().join(format!("tesseract-{:02}.txt", seitenzahl));
            fs::read_to_string(tesseract_output_path.clone())
                .map_err(|e| Fehler::Io(format!("{}", tesseract_output_path.display()), e))?
        }
    };
    
    
    if 
        ocr_text.contains("Dritte Abteilung") || 
        ocr_text.contains("Dritte Abteilu ng") || 
        ocr_text.contains("Abteilung 3") || 
        ocr_text.contains("Hypothek") ||
        ocr_text.contains("Grundschuld") ||
        ocr_text.contains("Rentenschuld") ||
        ocr_text.contains("Abteilung ||I   ") ||
        ocr_text.contains("Abteilung Ill   ") ||
        ocr_text.contains("Abteilung IIl   ") ||
        ocr_text.contains("Abteilung III   ")
    {
        if is_landscape_page {
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
            } else {
                Ok(SeitenTyp::Abt3Vert)
            }
        }
    } else if 
        ocr_text.contains("Zweite Abteilung") || 
        ocr_text.contains("Abteilung ||") || 
        ocr_text.contains("Abteilung Il") ||
        ocr_text.contains("Abteilung II") ||
        ocr_text.contains("Abteilung 2")
    {
        if is_landscape_page {
            if ocr_text.contains("Veränderungen") || ocr_text.contains("Löschungen") {
                Ok(SeitenTyp::Abt2HorzVeraenderungen)
            } else {
                Ok(SeitenTyp::Abt2Horz)
            }
        } else {
            if ocr_text.contains("Veränderungen") || ocr_text.contains("Löschungen") {
                Ok(SeitenTyp::Abt2VertVeraenderungen)
            } else {
                Ok(SeitenTyp::Abt2Vert)
            }
        }  
    } else if     
        ocr_text.contains("Erste Abteilung") || 
        ocr_text.contains("Abteilung |   ") || 
        ocr_text.contains("Abteilung I   ") || 
        ocr_text.contains("Abteilung 1") ||
        (ocr_text.contains("Eigentümer") && ocr_text.contains("Grundlage der Eintragung"))
    {
        if is_landscape_page {
            Ok(SeitenTyp::Abt1Horz)
        } else {
            Ok(SeitenTyp::Abt1Vert)
        }
    } else if 
        ocr_text.contains("Bestandsverzeichnis") ||
        ocr_text.contains("Besiandsverzeichnis") ||
        ocr_text.contains("Bezeichnung der Grundstücke und der mit dem Eigentum verbundenen Rechte") ||
        ocr_text.contains("Wirtschaftsart und Lage") ||
        ocr_text.contains("Bestand und Zuschreibungen")
    {
        if is_landscape_page {
            if ocr_text.contains("Abschreibungen") {
                Ok(SeitenTyp::BestandsverzeichnisHorzZuUndAbschreibungen)
            } else {
                Ok(SeitenTyp::BestandsverzeichnisHorz)
            }
        } else {
            if ocr_text.contains("Abschreibungen") {
                Ok(SeitenTyp::BestandsverzeichnisVertZuUndAbschreibungen)
            } else if ocr_text.contains("a/b/c") || ocr_text.contains("alb/c") {
                Ok(SeitenTyp::BestandsverzeichnisVertTyp2)
            } else {
                Ok(SeitenTyp::BestandsverzeichnisVert)
            }
        }
    } else {
        Err(Fehler::UnbekannterSeitentyp(seitenzahl))
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
        match self {
            
            SeitenTyp::BestandsverzeichnisHorz => vec![
            
                // "lfd. Nr. der Grundstücke"
                Column {
                    id: "bv_horz-lfd_nr",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-lfd_nr")).map(|m| m.min_x).unwrap_or(60.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-lfd_nr")).map(|m| m.max_x).unwrap_or(95.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-lfd_nr")).map(|m| m.min_y).unwrap_or(130.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-lfd_nr")).map(|m| m.max_y).unwrap_or(565.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Bisherige lfd. Nr."
                Column {
                    id: "bv_horz-bisherige_lfd_nr",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-bisherige_lfd_nr")).map(|m| m.min_x).unwrap_or(100.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-bisherige_lfd_nr")).map(|m| m.max_x).unwrap_or(140.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-bisherige_lfd_nr")).map(|m| m.min_y).unwrap_or(130.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-bisherige_lfd_nr")).map(|m| m.max_y).unwrap_or(565.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // Gemarkung
                Column {
                    id: "bv_horz-gemarkung",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-gemarkung")).map(|m| m.min_x).unwrap_or(150.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-gemarkung")).map(|m| m.max_x).unwrap_or(255.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-gemarkung")).map(|m| m.min_y).unwrap_or(130.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-gemarkung")).map(|m| m.max_y).unwrap_or(565.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // Flur
                Column {
                    id: "bv_horz-flur",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-flur")).map(|m| m.min_x).unwrap_or(265.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-flur")).map(|m| m.max_x).unwrap_or(300.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-flur")).map(|m| m.min_y).unwrap_or(130.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-flur")).map(|m| m.max_y).unwrap_or(565.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // Flurstück
                Column {
                    id: "bv_horz-flurstueck",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-flurstueck")).map(|m| m.min_x).unwrap_or(305.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-flurstueck")).map(|m| m.max_x).unwrap_or(370.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-flurstueck")).map(|m| m.min_y).unwrap_or(130.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-flurstueck")).map(|m| m.max_y).unwrap_or(565.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },

                // Wirtschaftsart und Lage
                Column {
                    id: "bv_horz-lage",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-lage")).map(|m| m.min_x).unwrap_or(375.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-lage")).map(|m| m.max_x).unwrap_or(670.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-lage")).map(|m| m.min_y).unwrap_or(130.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-lage")).map(|m| m.max_y).unwrap_or(565.0),
                    is_number_column: false,
                    line_break_after_px: 40.0, // 10.0,
                },
                
                // Größe (ha)
                Column {
                    id: "bv_horz-groesse_ha",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-groesse_ha")).map(|m| m.min_x).unwrap_or(675.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-groesse_ha")).map(|m| m.max_x).unwrap_or(710.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-groesse_ha")).map(|m| m.min_y).unwrap_or(130.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-groesse_ha")).map(|m| m.max_y).unwrap_or(565.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // Größe (a)
                Column {
                    id: "bv_horz-groesse_a",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-groesse_a")).map(|m| m.min_x).unwrap_or(715.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-groesse_a")).map(|m| m.max_x).unwrap_or(735.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-groesse_a")).map(|m| m.min_y).unwrap_or(130.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-groesse_a")).map(|m| m.max_y).unwrap_or(565.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // Größe (m2)
                Column {
                    id: "bv_horz-groesse_m2",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-groesse_m2")).map(|m| m.min_x).unwrap_or(740.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-groesse_m2")).map(|m| m.max_x).unwrap_or(763.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-groesse_m2")).map(|m| m.min_y).unwrap_or(130.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz-groesse_m2")).map(|m| m.max_y).unwrap_or(565.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
            ],
            SeitenTyp::BestandsverzeichnisVert => vec![
                
                // "lfd. Nr. der Grundstücke"
                Column {
                    id: "bv_vert-lfd_nr",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert-lfd_nr")).map(|m| m.min_x).unwrap_or(32.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert-lfd_nr")).map(|m| m.max_x).unwrap_or(68.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert-lfd_nr")).map(|m| m.min_y).unwrap_or(150.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert-lfd_nr")).map(|m| m.max_y).unwrap_or(810.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Bisherige lfd. Nr."
                Column {
                    id: "bv_vert-bisherige_lfd_nr",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert-bisherige_lfd_nr")).map(|m| m.min_x).unwrap_or(72.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert-bisherige_lfd_nr")).map(|m| m.max_x).unwrap_or(108.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert-bisherige_lfd_nr")).map(|m| m.min_y).unwrap_or(150.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert-bisherige_lfd_nr")).map(|m| m.max_y).unwrap_or(810.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // Flur
                Column {
                    id: "bv_vert-flur",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert-flur")).map(|m| m.min_x).unwrap_or(115.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert-flur")).map(|m| m.max_x).unwrap_or(153.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert-flur")).map(|m| m.min_y).unwrap_or(150.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert-flur")).map(|m| m.max_y).unwrap_or(810.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // Flurstück
                Column {
                    id: "bv_vert-flurstueck",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert-flurstueck")).map(|m| m.min_x).unwrap_or(157.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert-flurstueck")).map(|m| m.max_x).unwrap_or(219.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert-flurstueck")).map(|m| m.min_y).unwrap_or(150.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert-flurstueck")).map(|m| m.max_y).unwrap_or(810.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },

                // Wirtschaftsart und Lage
                Column {
                    id: "bv_vert-lage",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert-lage")).map(|m| m.min_x).unwrap_or(221.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert-lage")).map(|m| m.max_x).unwrap_or(500.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert-lage")).map(|m| m.min_y).unwrap_or(150.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert-lage")).map(|m| m.max_y).unwrap_or(810.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // Größe
                Column {
                    id: "bv_vert-groesse_m2",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert-groesse_m2")).map(|m| m.min_x).unwrap_or(508.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert-groesse_m2")).map(|m| m.max_x).unwrap_or(567.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert-groesse_m2")).map(|m| m.min_y).unwrap_or(150.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert-groesse_m2")).map(|m| m.max_y).unwrap_or(810.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
            ],
            SeitenTyp::BestandsverzeichnisVertTyp2 => vec![
                
                // "lfd. Nr. der Grundstücke"
                Column {
                    id: "bv_vert_typ2-lfd_nr",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_typ2-lfd_nr")).map(|m| m.min_x).unwrap_or(35.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_typ2-lfd_nr")).map(|m| m.max_x).unwrap_or(72.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_typ2-lfd_nr")).map(|m| m.min_y).unwrap_or(128.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_typ2-lfd_nr")).map(|m| m.max_y).unwrap_or(785.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Bisherige lfd. Nr."
                Column {
                    id: "bv_vert_typ2-bisherige_lfd_nr",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_typ2-bisherige_lfd_nr")).map(|m| m.min_x).unwrap_or(75.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_typ2-bisherige_lfd_nr")).map(|m| m.max_x).unwrap_or(110.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_typ2-bisherige_lfd_nr")).map(|m| m.min_y).unwrap_or(128.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_typ2-bisherige_lfd_nr")).map(|m| m.max_y).unwrap_or(785.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // Gemarkung, Flur, Flurstück
                Column {
                    id: "bv_vert_typ2-gemarkung_flur_flurstueck",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_typ2-gemarkung_flur_flurstueck")).map(|m| m.min_x).unwrap_or(115.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_typ2-gemarkung_flur_flurstueck")).map(|m| m.max_x).unwrap_or(230.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_typ2-gemarkung_flur_flurstueck")).map(|m| m.min_y).unwrap_or(128.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_typ2-gemarkung_flur_flurstueck")).map(|m| m.max_y).unwrap_or(785.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // Wirtschaftsart und Lage
                Column {
                    id: "bv_vert_typ2-lage",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_typ2-lage")).map(|m| m.min_x).unwrap_or(235.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_typ2-lage")).map(|m| m.max_x).unwrap_or(485.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_typ2-lage")).map(|m| m.min_y).unwrap_or(128.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_typ2-lage")).map(|m| m.max_y).unwrap_or(785.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // Größe (m2)
                Column {
                    id: "bv_vert_typ2-groesse_m2",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_typ2-groesse_m2")).map(|m| m.min_x).unwrap_or(490.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_typ2-groesse_m2")).map(|m| m.max_x).unwrap_or(555.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_typ2-groesse_m2")).map(|m| m.min_y).unwrap_or(128.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_typ2-groesse_m2")).map(|m| m.max_y).unwrap_or(785.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
            ],
            SeitenTyp::BestandsverzeichnisHorzZuUndAbschreibungen => vec![
            
                // "Zur lfd. Nr. der Grundstücke"
                Column {
                    id: "bv_horz_zu_abschreibung-lfd_nr_zuschreibungen",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz_zu_abschreibung-lfd_nr_zuschreibungen")).map(|m| m.min_x).unwrap_or(57.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz_zu_abschreibung-lfd_nr_zuschreibungen")).map(|m| m.max_x).unwrap_or(95.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz_zu_abschreibung-lfd_nr_zuschreibungen")).map(|m| m.min_y).unwrap_or(125.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz_zu_abschreibung-lfd_nr_zuschreibungen")).map(|m| m.max_y).unwrap_or(560.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Bestand und Zuschreibungen"
                Column {
                    id: "bv_horz_zu_abschreibung-zuschreibungen",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz_zu_abschreibung-zuschreibungen")).map(|m| m.min_x).unwrap_or(105.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz_zu_abschreibung-zuschreibungen")).map(|m| m.max_x).unwrap_or(420.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz_zu_abschreibung-zuschreibungen")).map(|m| m.min_y).unwrap_or(125.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz_zu_abschreibung-zuschreibungen")).map(|m| m.max_y).unwrap_or(560.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Zur lfd. Nr. der Grundstücke"
                Column {
                    id: "bv_horz_zu_abschreibung-lfd_nr_abschreibungen",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz_zu_abschreibung-lfd_nr_abschreibungen")).map(|m| m.min_x).unwrap_or(425.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz_zu_abschreibung-lfd_nr_abschreibungen")).map(|m| m.max_x).unwrap_or(470.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz_zu_abschreibung-lfd_nr_abschreibungen")).map(|m| m.min_y).unwrap_or(125.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz_zu_abschreibung-lfd_nr_abschreibungen")).map(|m| m.max_y).unwrap_or(560.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Abschreibungen"
                Column {
                    id: "bv_horz_zu_abschreibung-abschreibungen",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz_zu_abschreibung-abschreibungen")).map(|m| m.min_x).unwrap_or(480.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz_zu_abschreibung-abschreibungen")).map(|m| m.max_x).unwrap_or(763.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz_zu_abschreibung-abschreibungen")).map(|m| m.min_y).unwrap_or(125.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_horz_zu_abschreibung-abschreibungen")).map(|m| m.max_y).unwrap_or(560.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
            ],
            SeitenTyp::BestandsverzeichnisVertZuUndAbschreibungen => vec![
            
                // "Zur lfd. Nr. der Grundstücke"
                Column {
                    id: "bv_vert_zu_abschreibung-lfd_nr_zuschreibungen",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_zu_abschreibung-lfd_nr_zuschreibungen")).map(|m| m.min_x).unwrap_or(35.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_zu_abschreibung-lfd_nr_zuschreibungen")).map(|m| m.max_x).unwrap_or(72.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_zu_abschreibung-lfd_nr_zuschreibungen")).map(|m| m.min_y).unwrap_or(150.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_zu_abschreibung-lfd_nr_zuschreibungen")).map(|m| m.max_y).unwrap_or(810.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Bestand und Zuschreibungen"
                Column {
                    id: "bv_vert_zu_abschreibung-zuschreibungen",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_zu_abschreibung-zuschreibungen")).map(|m| m.min_x).unwrap_or(78.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_zu_abschreibung-zuschreibungen")).map(|m| m.max_x).unwrap_or(330.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_zu_abschreibung-zuschreibungen")).map(|m| m.min_y).unwrap_or(150.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_zu_abschreibung-zuschreibungen")).map(|m| m.max_y).unwrap_or(810.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Zur lfd. Nr. der Grundstücke"
                Column {
                    id: "bv_vert_zu_abschreibung-lfd_nr_abschreibungen",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_zu_abschreibung-lfd_nr_abschreibungen")).map(|m| m.min_x).unwrap_or(337.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_zu_abschreibung-lfd_nr_abschreibungen")).map(|m| m.max_x).unwrap_or(375.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_zu_abschreibung-lfd_nr_abschreibungen")).map(|m| m.min_y).unwrap_or(150.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_zu_abschreibung-lfd_nr_abschreibungen")).map(|m| m.max_y).unwrap_or(810.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Abschreibungen"
                Column {
                    id: "bv_vert_zu_abschreibung-abschreibungen",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_zu_abschreibung-abschreibungen")).map(|m| m.min_x).unwrap_or(382.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_zu_abschreibung-abschreibungen")).map(|m| m.max_x).unwrap_or(520.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_zu_abschreibung-abschreibungen")).map(|m| m.min_y).unwrap_or(150.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("bv_vert_zu_abschreibung-abschreibungen")).map(|m| m.max_y).unwrap_or(810.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
            ],

            
            SeitenTyp::Abt1Horz => vec![
            
                // "lfd. Nr. der Eintragungen"
                Column {
                    id: "abt1_horz-lfd_nr",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt1_horz-lfd_nr")).map(|m| m.min_x).unwrap_or(55.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt1_horz-lfd_nr")).map(|m| m.max_x).unwrap_or(95.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt1_horz-lfd_nr")).map(|m| m.min_y).unwrap_or(130.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt1_horz-lfd_nr")).map(|m| m.max_y).unwrap_or(565.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Eigentümer"
                Column {
                    id: "abt1_horz-eigentuemer",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt1_horz-eigentuemer")).map(|m| m.min_x).unwrap_or(100.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt1_horz-eigentuemer")).map(|m| m.max_x).unwrap_or(405.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt1_horz-eigentuemer")).map(|m| m.min_y).unwrap_or(130.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt1_horz-eigentuemer")).map(|m| m.max_y).unwrap_or(565.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "lfd. Nr. der Grundstücke im BV"
                Column {
                    id: "abt1_horz-lfd_nr_bv",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt1_horz-lfd_nr_bv")).map(|m| m.min_x).unwrap_or(413.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt1_horz-lfd_nr_bv")).map(|m| m.max_x).unwrap_or(520.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt1_horz-lfd_nr_bv")).map(|m| m.min_y).unwrap_or(130.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt1_horz-lfd_nr_bv")).map(|m| m.max_y).unwrap_or(565.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Grundlage der Eintragung"
                Column {
                    id: "abt1_horz-grundlage_der_eintragung",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt1_horz-grundlage_der_eintragung")).map(|m| m.min_x).unwrap_or(525.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt1_horz-grundlage_der_eintragung")).map(|m| m.max_x).unwrap_or(762.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt1_horz-grundlage_der_eintragung")).map(|m| m.min_y).unwrap_or(130.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt1_horz-grundlage_der_eintragung")).map(|m| m.max_y).unwrap_or(565.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
            ],
            SeitenTyp::Abt1Vert => vec![
                
                // "lfd. Nr. der Eintragungen"
                Column {
                    id: "abt1_vert-lfd_nr",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt1_vert-lfd_nr")).map(|m| m.min_x).unwrap_or(32.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt1_vert-lfd_nr")).map(|m| m.max_x).unwrap_or(60.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt1_vert-lfd_nr")).map(|m| m.min_y).unwrap_or(150.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt1_vert-lfd_nr")).map(|m| m.max_y).unwrap_or(810.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Eigentümer"
                Column {
                    id: "abt1_vert-eigentuemer",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt1_vert-eigentuemer")).map(|m| m.min_x).unwrap_or(65.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt1_vert-eigentuemer")).map(|m| m.max_x).unwrap_or(290.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt1_vert-eigentuemer")).map(|m| m.min_y).unwrap_or(150.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt1_vert-eigentuemer")).map(|m| m.max_y).unwrap_or(810.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "lfd. Nr. der Grundstücke im BV"
                Column {
                    id: "abt1_vert-lfd_nr_bv",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt1_vert-lfd_nr_bv")).map(|m| m.min_x).unwrap_or(298.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt1_vert-lfd_nr_bv")).map(|m| m.max_x).unwrap_or(337.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt1_vert-lfd_nr_bv")).map(|m| m.min_y).unwrap_or(150.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt1_vert-lfd_nr_bv")).map(|m| m.max_y).unwrap_or(810.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Grundlage der Eintragung"
                Column {
                    id: "abt1_vert-grundlage_der_eintragung",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt1_vert-grundlage_der_eintragung")).map(|m| m.min_x).unwrap_or(343.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt1_vert-grundlage_der_eintragung")).map(|m| m.max_x).unwrap_or(567.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt1_vert-grundlage_der_eintragung")).map(|m| m.min_y).unwrap_or(150.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt1_vert-grundlage_der_eintragung")).map(|m| m.max_y).unwrap_or(810.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
            ],
            
            SeitenTyp::Abt2Horz => vec![
            
                // "lfd. Nr. der Eintragungen"
                Column {
                    id: "abt2_horz-lfd_nr",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt2_horz-lfd_nr")).map(|m| m.min_x).unwrap_or(55.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt2_horz-lfd_nr")).map(|m| m.max_x).unwrap_or(95.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt2_horz-lfd_nr")).map(|m| m.min_y).unwrap_or(130.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt2_horz-lfd_nr")).map(|m| m.max_y).unwrap_or(565.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "lfd. Nr. der Grundstücke im BV"
                Column {
                    id: "abt2_horz-lfd_nr_bv",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt2_horz-lfd_nr_bv")).map(|m| m.min_x).unwrap_or(103.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt2_horz-lfd_nr_bv")).map(|m| m.max_x).unwrap_or(192.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt2_horz-lfd_nr_bv")).map(|m| m.min_y).unwrap_or(130.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt2_horz-lfd_nr_bv")).map(|m| m.max_y).unwrap_or(565.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Lasten und Beschränkungen"
                Column {
                    id: "abt2_horz-lasten_und_beschraenkungen",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt2_horz-lasten_und_beschraenkungen")).map(|m| m.min_x).unwrap_or(200.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt2_horz-lasten_und_beschraenkungen")).map(|m| m.max_x).unwrap_or(765.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt2_horz-lasten_und_beschraenkungen")).map(|m| m.min_y).unwrap_or(130.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt2_horz-lasten_und_beschraenkungen")).map(|m| m.max_y).unwrap_or(565.0),
                    is_number_column: false,
                    line_break_after_px: 25.0, // 10.0,
                },
            ],
            SeitenTyp::Abt2HorzVeraenderungen => vec![
            
                // "lfd. Nr. der Spalte 1"
                Column {
                    id: "abt2_horz_veraenderungen-lfd_nr",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt2_horz_veraenderungen-lfd_nr")).map(|m| m.min_x).unwrap_or(55.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt2_horz_veraenderungen-lfd_nr")).map(|m| m.max_x).unwrap_or(95.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt2_horz_veraenderungen-lfd_nr")).map(|m| m.min_y).unwrap_or(130.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt2_horz_veraenderungen-lfd_nr")).map(|m| m.max_y).unwrap_or(565.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Veränderungen"
                Column {
                    id: "abt2_horz_veraenderungen-veraenderungen",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt2_horz_veraenderungen-veraenderungen")).map(|m| m.min_x).unwrap_or(103.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt2_horz_veraenderungen-veraenderungen")).map(|m| m.max_x).unwrap_or(505.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt2_horz_veraenderungen-veraenderungen")).map(|m| m.min_y).unwrap_or(130.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt2_horz_veraenderungen-veraenderungen")).map(|m| m.max_y).unwrap_or(565.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "lfd. Nr. der Spalte 2"
                Column {
                    id: "abt2_horz_veraenderungen-lfd_nr_bv",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt2_horz_veraenderungen-lfd_nr_bv")).map(|m| m.min_x).unwrap_or(515.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt2_horz_veraenderungen-lfd_nr_bv")).map(|m| m.max_x).unwrap_or(552.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt2_horz_veraenderungen-lfd_nr_bv")).map(|m| m.min_y).unwrap_or(130.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt2_horz_veraenderungen-lfd_nr_bv")).map(|m| m.max_y).unwrap_or(565.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Löschungen"
                Column {
                    id: "abt2_horz_veraenderungen-loeschungen",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt2_horz_veraenderungen-loeschungen")).map(|m| m.min_x).unwrap_or(560.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt2_horz_veraenderungen-loeschungen")).map(|m| m.max_x).unwrap_or(770.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt2_horz_veraenderungen-loeschungen")).map(|m| m.min_y).unwrap_or(130.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt2_horz_veraenderungen-loeschungen")).map(|m| m.max_y).unwrap_or(565.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
            ],
            SeitenTyp::Abt2Vert => vec![
            
                // "lfd. Nr. der Eintragungen"
                Column {
                    id: "abt2_vert-lfd_nr",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt2_vert-lfd_nr")).map(|m| m.min_x).unwrap_or(32.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt2_vert-lfd_nr")).map(|m| m.max_x).unwrap_or(60.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt2_vert-lfd_nr")).map(|m| m.min_y).unwrap_or(150.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt2_vert-lfd_nr")).map(|m| m.max_y).unwrap_or(810.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "lfd. Nr der betroffenen Grundstücke"
                Column {
                    id: "abt2_vert-lfd_nr_bv",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt2_vert-lfd_nr_bv")).map(|m| m.min_x).unwrap_or(65.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt2_vert-lfd_nr_bv")).map(|m| m.max_x).unwrap_or(105.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt2_vert-lfd_nr_bv")).map(|m| m.min_y).unwrap_or(150.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt2_vert-lfd_nr_bv")).map(|m| m.max_y).unwrap_or(810.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Lasten und Beschränkungen"
                Column {
                    id: "abt2_vert-lasten_und_beschraenkungen",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt2_vert-lasten_und_beschraenkungen")).map(|m| m.min_x).unwrap_or(112.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt2_vert-lasten_und_beschraenkungen")).map(|m| m.max_x).unwrap_or(567.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt2_vert-lasten_und_beschraenkungen")).map(|m| m.min_y).unwrap_or(150.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt2_vert-lasten_und_beschraenkungen")).map(|m| m.max_y).unwrap_or(810.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
            ],
            SeitenTyp::Abt2VertVeraenderungen => vec![
            
                // "lfd. Nr. der Spalte 1"
                Column {
                    id: "abt2_vert_veraenderungen-lfd_nr",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt2_vert_veraenderungen-lfd_nr")).map(|m| m.min_x).unwrap_or(32.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt2_vert_veraenderungen-lfd_nr")).map(|m| m.max_x).unwrap_or(65.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt2_vert_veraenderungen-lfd_nr")).map(|m| m.min_y).unwrap_or(150.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt2_vert_veraenderungen-lfd_nr")).map(|m| m.max_y).unwrap_or(810.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Veränderungen"
                Column {
                    id: "abt2_vert_veraenderungen-veraenderungen",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt2_vert_veraenderungen-veraenderungen")).map(|m| m.min_x).unwrap_or(72.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt2_vert_veraenderungen-veraenderungen")).map(|m| m.max_x).unwrap_or(362.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt2_vert_veraenderungen-veraenderungen")).map(|m| m.min_y).unwrap_or(150.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt2_vert_veraenderungen-veraenderungen")).map(|m| m.max_y).unwrap_or(810.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "lfd. Nr. der Spalte 1"
                Column {
                    id: "abt2_vert_veraenderungen-lfd_nr_bv",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt2_vert_veraenderungen-lfd_nr_bv")).map(|m| m.min_x).unwrap_or(370.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt2_vert_veraenderungen-lfd_nr_bv")).map(|m| m.max_x).unwrap_or(400.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt2_vert_veraenderungen-lfd_nr_bv")).map(|m| m.min_y).unwrap_or(150.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt2_vert_veraenderungen-lfd_nr_bv")).map(|m| m.max_y).unwrap_or(810.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Löschungen"
                Column {
                    id: "abt2_vert_veraenderungen-loeschungen",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt2_vert_veraenderungen-loeschungen")).map(|m| m.min_x).unwrap_or(406.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt2_vert_veraenderungen-loeschungen")).map(|m| m.max_x).unwrap_or(565.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt2_vert_veraenderungen-loeschungen")).map(|m| m.min_y).unwrap_or(150.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt2_vert_veraenderungen-loeschungen")).map(|m| m.max_y).unwrap_or(810.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
            ],
            
            SeitenTyp::Abt3Horz => vec![
            
                // "lfd. Nr. der Eintragungen"
                Column {
                    id: "abt3_horz-lfd_nr",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz-lfd_nr")).map(|m| m.min_x).unwrap_or(55.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz-lfd_nr")).map(|m| m.max_x).unwrap_or(95.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz-lfd_nr")).map(|m| m.min_y).unwrap_or(130.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz-lfd_nr")).map(|m| m.max_y).unwrap_or(565.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "lfd. Nr. der Grundstücke im BV"
                Column {
                    id: "abt3_horz-lfd_nr_bv",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz-lfd_nr_bv")).map(|m| m.min_x).unwrap_or(103.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz-lfd_nr_bv")).map(|m| m.max_x).unwrap_or(170.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz-lfd_nr_bv")).map(|m| m.min_y).unwrap_or(130.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz-lfd_nr_bv")).map(|m| m.max_y).unwrap_or(565.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Betrag"
                Column {
                    id: "abt3_horz-betrag",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz-betrag")).map(|m| m.min_x).unwrap_or(180.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz-betrag")).map(|m| m.max_x).unwrap_or(275.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz-betrag")).map(|m| m.min_y).unwrap_or(130.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz-betrag")).map(|m| m.max_y).unwrap_or(565.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Hypotheken, Grundschulden, Rentenschulden"
                Column {
                    id: "abt3_horz-text",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz-text")).map(|m| m.min_x).unwrap_or(285.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz-text")).map(|m| m.max_x).unwrap_or(760.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz-text")).map(|m| m.min_y).unwrap_or(130.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz-text")).map(|m| m.max_y).unwrap_or(565.0),
                    is_number_column: false,
                    line_break_after_px: 25.0, // 10.0,
                },
            ],
            SeitenTyp::Abt3Vert => vec![
            
                // "lfd. Nr. der Eintragungen"
                Column {
                    id: "abt3_vert-lfd_nr",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert-lfd_nr")).map(|m| m.min_x).unwrap_or(32.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert-lfd_nr")).map(|m| m.max_x).unwrap_or(60.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert-lfd_nr")).map(|m| m.min_y).unwrap_or(150.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert-lfd_nr")).map(|m| m.max_y).unwrap_or(785.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "lfd. Nr der belastete Grundstücke im BV"
                Column {
                    id: "abt3_vert-lfd_nr_bv",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert-lfd_nr_bv")).map(|m| m.min_x).unwrap_or(65.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert-lfd_nr_bv")).map(|m| m.max_x).unwrap_or(100.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert-lfd_nr_bv")).map(|m| m.min_y).unwrap_or(150.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert-lfd_nr_bv")).map(|m| m.max_y).unwrap_or(785.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Betrag"
                Column {
                    id: "abt3_vert-betrag",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert-betrag")).map(|m| m.min_x).unwrap_or(105.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert-betrag")).map(|m| m.max_x).unwrap_or(193.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert-betrag")).map(|m| m.min_y).unwrap_or(150.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert-betrag")).map(|m| m.max_y).unwrap_or(785.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Hypotheken, Grundschulden, Rentenschulden"
                Column {
                    id: "abt3_vert-text",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert-text")).map(|m| m.min_x).unwrap_or(195.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert-text")).map(|m| m.max_x).unwrap_or(567.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert-text")).map(|m| m.min_y).unwrap_or(150.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert-text")).map(|m| m.max_y).unwrap_or(785.0),
                    is_number_column: false,
                    line_break_after_px: 25.0, // 10.0,
                },
            ],  
            SeitenTyp::Abt3HorzVeraenderungenLoeschungen => vec![
                
                // "lfd. Nr. der Spalte 1"
                Column {
                    id: "abt3_horz_veraenderungen_loeschungen-lfd_nr",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-lfd_nr")).map(|m| m.min_x).unwrap_or(55.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-lfd_nr")).map(|m| m.max_x).unwrap_or(95.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-lfd_nr")).map(|m| m.min_y).unwrap_or(127.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-lfd_nr")).map(|m| m.max_y).unwrap_or(565.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Betrag"
                Column {
                    id: "abt3_horz_veraenderungen_loeschungen-betrag",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-betrag")).map(|m| m.min_x).unwrap_or(105.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-betrag")).map(|m| m.max_x).unwrap_or(200.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-betrag")).map(|m| m.min_y).unwrap_or(127.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-betrag")).map(|m| m.max_y).unwrap_or(565.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Veränderungen"
                Column {
                    id: "abt3_horz_veraenderungen_loeschungen-text",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-text")).map(|m| m.min_x).unwrap_or(202.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-text")).map(|m| m.max_x).unwrap_or(490.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-text")).map(|m| m.min_y).unwrap_or(127.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-text")).map(|m| m.max_y).unwrap_or(565.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                
                // "lfd. Nr. der Spalte 1"
                Column {
                    id: "abt3_horz_veraenderungen_loeschungen-lfd_nr",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-lfd_nr")).map(|m| m.min_x).unwrap_or(495.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-lfd_nr")).map(|m| m.max_x).unwrap_or(535.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-lfd_nr")).map(|m| m.min_y).unwrap_or(127.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-lfd_nr")).map(|m| m.max_y).unwrap_or(565.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Betrag"
                Column {
                    id: "abt3_horz_veraenderungen_loeschungen-betrag",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-betrag")).map(|m| m.min_x).unwrap_or(542.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-betrag")).map(|m| m.max_x).unwrap_or(640.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-betrag")).map(|m| m.min_y).unwrap_or(127.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-betrag")).map(|m| m.max_y).unwrap_or(565.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Löschungen"
                Column {
                    id: "abt3_horz_veraenderungen_loeschungen-text",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-text")).map(|m| m.min_x).unwrap_or(645.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-text")).map(|m| m.max_x).unwrap_or(765.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-text")).map(|m| m.min_y).unwrap_or(127.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_horz_veraenderungen_loeschungen-text")).map(|m| m.max_y).unwrap_or(565.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
            ],
            SeitenTyp::Abt3VertVeraenderungenLoeschungen => vec![
                
                // "lfd. Nr. der Spalte 1"
                Column {
                    id: "abt3_vert_veraenderungen_loeschungen-lfd_nr",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-lfd_nr")).map(|m| m.min_x).unwrap_or(37.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-lfd_nr")).map(|m| m.max_x).unwrap_or(75.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-lfd_nr")).map(|m| m.min_y).unwrap_or(127.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-lfd_nr")).map(|m| m.max_y).unwrap_or(783.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Betrag"
                Column {
                    id: "abt3_vert_veraenderungen_loeschungen-betrag",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-betrag")).map(|m| m.min_x).unwrap_or(80.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-betrag")).map(|m| m.max_x).unwrap_or(142.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-betrag")).map(|m| m.min_y).unwrap_or(127.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-betrag")).map(|m| m.max_y).unwrap_or(783.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Veränderungen"
                Column {
                    id: "abt3_vert_veraenderungen_loeschungen-text",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-text")).map(|m| m.min_x).unwrap_or(147.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-text")).map(|m| m.max_x).unwrap_or(388.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-text")).map(|m| m.min_y).unwrap_or(127.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-text")).map(|m| m.max_y).unwrap_or(783.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                
                // "lfd. Nr. der Spalte 1"
                Column {
                    id: "abt3_vert_veraenderungen_loeschungen-lfd_nr",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-lfd_nr")).map(|m| m.min_x).unwrap_or(390.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-lfd_nr")).map(|m| m.max_x).unwrap_or(415.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-lfd_nr")).map(|m| m.min_y).unwrap_or(127.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-lfd_nr")).map(|m| m.max_y).unwrap_or(783.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Betrag"
                Column {
                    id: "abt3_vert_veraenderungen_loeschungen-betrag",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-betrag")).map(|m| m.min_x).unwrap_or(420.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-betrag")).map(|m| m.max_x).unwrap_or(485.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-betrag")).map(|m| m.min_y).unwrap_or(127.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-betrag")).map(|m| m.max_y).unwrap_or(783.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Löschungen"
                Column {
                    id: "abt3_vert_veraenderungen_loeschungen-text",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-text")).map(|m| m.min_x).unwrap_or(492.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-text")).map(|m| m.max_x).unwrap_or(565.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-text")).map(|m| m.min_y).unwrap_or(127.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen_loeschungen-text")).map(|m| m.max_y).unwrap_or(783.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
            ],

            SeitenTyp::Abt3VertVeraenderungen => vec![
            
                // "lfd. Nr. der Spalte 1"
                Column {
                    id: "abt3_vert_veraenderungen-lfd_nr",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen-lfd_nr")).map(|m| m.min_x).unwrap_or(32.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen-lfd_nr")).map(|m| m.max_x).unwrap_or(60.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen-lfd_nr")).map(|m| m.min_y).unwrap_or(150.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen-lfd_nr")).map(|m| m.max_y).unwrap_or(810.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Betrag"
                Column {
                    id: "abt3_vert_veraenderungen-betrag",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen-betrag")).map(|m| m.min_x).unwrap_or(70.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen-betrag")).map(|m| m.max_x).unwrap_or(160.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen-betrag")).map(|m| m.min_y).unwrap_or(150.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen-betrag")).map(|m| m.max_y).unwrap_or(810.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Veränderungen"
                Column {
                    id: "abt3_vert_veraenderungen-text",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen-text")).map(|m| m.min_x).unwrap_or(165.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen-text")).map(|m| m.max_x).unwrap_or(565.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen-text")).map(|m| m.min_y).unwrap_or(150.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_veraenderungen-text")).map(|m| m.max_y).unwrap_or(810.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
            ],
            SeitenTyp::Abt3VertLoeschungen => vec![
            
                // "lfd. Nr. der Spalte 1"
                Column {
                    id: "abt3_vert_loeschungen-lfd_nr",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_loeschungen-lfd_nr")).map(|m| m.min_x).unwrap_or(175.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_loeschungen-lfd_nr")).map(|m| m.max_x).unwrap_or(205.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_loeschungen-lfd_nr")).map(|m| m.min_y).unwrap_or(150.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_loeschungen-lfd_nr")).map(|m| m.max_y).unwrap_or(810.0),
                    is_number_column: true,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Betrag"
                Column {
                    id: "abt3_vert_loeschungen-betrag",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_loeschungen-betrag")).map(|m| m.min_x).unwrap_or(215.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_loeschungen-betrag")).map(|m| m.max_x).unwrap_or(305.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_loeschungen-betrag")).map(|m| m.min_y).unwrap_or(150.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_loeschungen-betrag")).map(|m| m.max_y).unwrap_or(810.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                },
                
                // "Löschungen"
                Column {
                    id: "abt3_vert_loeschungen-text",
                    min_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_loeschungen-text")).map(|m| m.min_x).unwrap_or(310.0),
                    max_x: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_loeschungen-text")).map(|m| m.max_x).unwrap_or(570.0),
                    min_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_loeschungen-text")).map(|m| m.min_y).unwrap_or(150.0),
                    max_y: anpassungen_seite.and_then(|s| s.spalten.get("abt3_vert_loeschungen-text")).map(|m| m.max_y).unwrap_or(810.0),
                    is_number_column: false,
                    line_break_after_px: 10.0, // 10.0,
                }
            ],
        }
    }
}

// Wenn der Seitentyp bekannt ist, schneide die Seiten nach ihrem Seitentyp in Spalten
//
// Gibt die Spalten zurück
pub fn formularspalten_ausschneiden(
    titelblatt: &Titelblatt, 
    seitenzahl: u32, 
    max_seitenzahl: u32, 
    seitentyp: SeitenTyp, 
    pdftotext_layout: &PdfToTextLayout,
    anpassungen_seite: Option<&AnpassungSeite>,
) -> Result<Vec<Column>, Fehler> {

    use image::ImageOutputFormat;
    use std::fs::File;
    use std::path::Path;
    
    let columns = seitentyp.get_columns(anpassungen_seite);
    let temp_dir = std::env::temp_dir().join(&format!("{gemarkung}/{blatt}", gemarkung = titelblatt.grundbuch_von, blatt = titelblatt.blatt));

    let columns_to_recalc = 
    columns
    .clone()
    .par_iter()
    .enumerate()
    .filter_map(|(col_idx, col)| {
        
        let cropped_output_path = temp_dir.clone().join(format!("page-{}-col-{:02}-{:02}-{:02}-{:02}-{:02}.png", 
            formatiere_seitenzahl(seitenzahl, max_seitenzahl), 
            col_idx,
            col.min_x,
            col.min_y,
            col.max_x,
            col.max_y,
        ));
        
        if Path::new(&cropped_output_path).exists() {
            None
        } else {
            Some((col_idx, col.clone(), cropped_output_path))
        }
    }).collect::<Vec<_>>();
    
    if columns_to_recalc.is_empty() { 
        return Ok(columns); 
    }
    
    let seite = pdftotext_layout.seiten
        .get(&format!("{}", seitenzahl))
        .ok_or(Fehler::FalscheSeitenZahl(seitenzahl))?;
    
    let _ = fs::create_dir_all(temp_dir.clone())
        .map_err(|e| Fehler::Io(format!("{}", temp_dir.clone().display()), e))?;
    
    let pdftoppm_output_path = temp_dir.clone().join(format!("page-clean-{}.png", formatiere_seitenzahl(seitenzahl, max_seitenzahl)));
    let (im_width, im_height) = image::image_dimensions(&pdftoppm_output_path)
        .map_err(|e| Fehler::Bild(format!("{}", pdftoppm_output_path.display()), e))?;

    let im_width = im_width as f32;
    let im_height = im_height as f32;
    
    let mut im = image::open(&pdftoppm_output_path.clone())
        .map_err(|e| Fehler::Bild(format!("{}", pdftoppm_output_path.display()), e))?;
    
    let page_width = seite.breite_mm;
    let page_height = seite.hoehe_mm;
    
    let mut rgb_bytes = im.to_rgb8();

    // Textblöcke schwärzen, die bereits in pdftotext vorhanden sind
    for t in seite.texte.iter() {
        
        let t_start_y_px = ((t.start_y * im_height / seite.hoehe_mm).floor() as u32).min(im_height as u32);
        let t_end_y_px = ((t.end_y * im_height / seite.hoehe_mm).floor() as u32).min(im_height as u32);
        let t_start_x_px = ((t.start_x * im_width / seite.breite_mm).floor() as u32).min(im_width as u32);
        let t_end_x_px = ((t.end_x * im_width / seite.breite_mm).floor() as u32).min(im_width as u32);
        
        for y in t_start_y_px..t_end_y_px {
            for x in t_start_x_px..t_end_x_px {
                rgb_bytes.put_pixel(x, y, image::Rgb([255, 255, 255]));
            }
        }
    }
    
    im = image::DynamicImage::ImageRgb8(rgb_bytes);
    
    columns_to_recalc
    .into_par_iter()
    .for_each(|(col_idx, col, col_path)| {
        
        // crop columns of image
        let x = col.min_x / page_width * im_width as f32;
        let y = col.min_y / page_height * im_height as f32;
        let width = (col.max_x - col.min_x) / page_width * im_width as f32;
        let height = (col.max_y - col.min_y) / page_height * im_height as f32;
        
        let cropped = im.crop_imm(
            x.round().max(0.0) as u32, 
            y.round().max(0.0) as u32, 
            width.round().max(0.0) as u32, 
            height.round().max(0.0) as u32, 
        );
        
        if let Ok(mut output_file) = File::create(col_path.clone()) {
            let _ = cropped.write_to(&mut output_file, ImageOutputFormat::Png);
        }
    });
    
    Ok(columns)
}

// Seitenzahlen sind 
pub fn formatiere_seitenzahl(zahl: u32, max_seiten: u32) -> String {
    if max_seiten < 10 {
        format!("{}", zahl)
    } else if max_seiten < 100 {
        format!("{:02}", zahl)
    } else {
        format!("{:03}", zahl)
    }
}

// Lässt die Schrifterkennung über die Spalten laufen, Ausgabe in .hocr Dateien
pub fn ocr_spalten(
    titelblatt: &Titelblatt, 
    seitenzahl: u32, 
    max_seitenzahl: u32, 
    spalten: &[Column]
) -> Result<(), Fehler> {
    
    use std::path::Path;
    
    let temp_dir = std::env::temp_dir()
    .join(&format!("{gemarkung}/{blatt}", gemarkung = titelblatt.grundbuch_von, blatt = titelblatt.blatt));

    for (col_idx, col) in spalten.iter().enumerate() {
        
        let cropped_output_path = temp_dir.clone().join(format!("page-{}-col-{:02}-{:02}-{:02}-{:02}-{:02}.png", 
            formatiere_seitenzahl(seitenzahl, max_seitenzahl), 
            col_idx,
            col.min_x,
            col.min_y,
            col.max_x,
            col.max_y,
        ));
        
        let tesseract_path = format!("tesseract-{:02}-col-{:02}-{:02}-{:02}-{:02}-{:02}", 
            seitenzahl, 
            col_idx,
            col.min_x,
            col.min_y,
            col.max_x,
            col.max_y
        );
            
        let tesseract_output_path = temp_dir.clone().join(format!("{}.hocr", tesseract_path));
        
        if Path::new(&tesseract_output_path).exists() {
            continue;
        }
        
        if col.is_number_column {
            let _ = get_tesseract_command()
            .arg(&format!("{}", cropped_output_path.display()))
            .arg(&format!("{}", temp_dir.clone().join(tesseract_path.clone()).display()))     
            .arg("--dpi")
            .arg("600")
            .arg("--psm")
            .arg("6")
            .arg("-c")
            .arg("tessedit_char_whitelist=,.-/%€0123456789 ")
            .arg("-c")
            .arg("tessedit_create_hocr=1")
            .arg("-c")
            .arg("debug_file=/dev/null") // TODO: funktioniert nur auf Linux!
            .status();
        } else {
            let _ = get_tesseract_command()
            .arg(&format!("{}", cropped_output_path.display()))
            .arg(&format!("{}", temp_dir.clone().join(tesseract_path.clone()).display()))     
            .arg("--dpi")
            .arg("600")
            .arg("--psm")
            .arg("6")
            .arg("-l")
            .arg("deu")
            .arg("-c")
            .arg("tessedit_char_whitelist=abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZüÜäÄöÖß,.-/%§()€0123456789 ")
            .arg("-c")
            .arg("tessedit_create_hocr=1")
            .arg("-c")
            .arg("debug_file=/dev/null") // TODO: funktioniert nur auf Linux!
            .status();
        }
    }
    
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Textblock {
    pub text: String,
    pub start_y: f32,
    pub end_y: f32,
    pub start_x: f32,
    pub end_x: f32,
}

pub fn zeilen_aus_tesseract_hocr(
    tesseract_hocr_path: String
) -> Result<Vec<String>, Fehler> {
    
    use kuchiki::traits::TendrilSink;

    let hocr_tesseract = fs::read_to_string(tesseract_hocr_path.clone())
        .map_err(|e| Fehler::Io(tesseract_hocr_path.clone(), e))?;
    
    let document = kuchiki::parse_html()
        .one(hocr_tesseract.as_str());
    
    let css_selector = ".ocr_line";
    let mut result = Vec::new();
    
    if let Ok(m) = document.select(css_selector) {

        for css_match in m {
        
            let as_node = css_match.as_node();
            let as_element = match as_node.as_element() {
                Some(s) => s,
                None => continue,
            };
            
            let line_text = as_node
                .text_contents()
                .lines()
                .map(|l| l.trim().to_string())
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();
            
            result.push(line_text.clone());
        }
    }
    
    Ok(result)
}

// Liest die Textblöcke aus den Spalten (mit Koordinaten in Pixeln) aus
pub fn textbloecke_aus_spalten(
    titelblatt: &Titelblatt, 
    seitenzahl: u32,
    max_seitenzahl: u32,
    spalten: &[Column], 
    pdftotext: &PdfToTextLayout,
    anpassungen_seite: Option<&AnpassungSeite>,
) -> Result<Vec<Vec<Textblock>>, Fehler> {

    let temp_dir = std::env::temp_dir()
    .join(&format!("{gemarkung}/{blatt}", gemarkung = titelblatt.grundbuch_von, blatt = titelblatt.blatt));
    
    let pdftoppm_output_path = temp_dir.clone().join(format!("page-clean-{}.png", formatiere_seitenzahl(seitenzahl, max_seitenzahl)));
    let (im_width, im_height) = image::image_dimensions(&pdftoppm_output_path)
        .map_err(|e| Fehler::Bild(format!("{}", pdftoppm_output_path.display()), e))?;

    let im_width = im_width as f32;
    let im_height = im_height as f32;
    
    Ok(spalten.par_iter().enumerate().map(|(col_idx, col)| {
    
        use kuchiki::traits::TendrilSink;

        // Textblöcke tesseract
        
        let zeilen_vordefiniert = anpassungen_seite
            .map(|aps| aps.zeilen.clone())
            .unwrap_or_default();
        
        let tesseract_path = format!("tesseract-{:02}-col-{:02}-{:02}-{:02}-{:02}-{:02}", 
            seitenzahl, 
            col_idx,
            col.min_x,
            col.min_y,
            col.max_x,
            col.max_y
        );
            
        let tesseract_hocr_path = temp_dir.clone().join(format!("{}.hocr", tesseract_path));

        // Read /tmp/tesseract-01-col-00.hocr
        let hocr_tesseract = match fs::read_to_string(tesseract_hocr_path.clone()) {
            Ok(o) => o,
            Err(e) => { return Err(Fehler::Io(format!("{}", tesseract_hocr_path.display()), e)); },
        };
        
        let document = kuchiki::parse_html()
            .one(hocr_tesseract.as_str());
        
        let css_selector = ".ocr_line";
        
        let (page_width, page_height) = match pdftotext.seiten.get(&format!("{}", seitenzahl)) {
            Some(o) => (o.breite_mm, o.hoehe_mm),
            None => { return Err(Fehler::FalscheSeitenZahl(seitenzahl)); },
        };
                
        let col_width_px = (col.max_x - col.min_x).abs() / page_width * im_width as f32;
        let col_height_px = (col.max_y - col.min_y).abs() / page_height * im_height as f32;
        let col_width_mm = (col.max_x - col.min_x).abs();
        let col_height_mm = (col.max_y - col.min_y).abs();
            
        if zeilen_vordefiniert.is_empty() {
            
            let mut text_blocks = Vec::new();
            let mut block_start_y = 0.0;
            let mut block_end_y = 0.0;
            let mut block_start_x = 0.0;
            let mut block_end_x = 0.0;
            
            let mut current_text_block = Vec::new();
            
            if let Ok(m) = document.select(css_selector) {

                for css_match in m {
                    
                    let as_node = css_match.as_node();
                    let as_element = match as_node.as_element() {
                        Some(s) => s,
                        None => continue,
                    };
                    
                    let bbox_attribute = as_element.attributes.borrow();
                    let bbox = (&*bbox_attribute)
                    .get("title")
                    .and_then(|b| b.split(";").next());
                    
                    let line_text = as_node
                        .text_contents()
                        .lines()
                        .map(|l| l.trim().to_string())
                        .collect::<Vec<_>>()
                        .join(" ")
                        .trim()
                        .to_string();
                    
                    // "bbox 882 201 1227 254"
                    let bbox_clean = match bbox {
                        Some(s) => s,
                        None => continue,
                    };
                    
                    // startx, starty, endx, endy
                    // 882 201 1227 254
                    let bbox = bbox_clean.replace("bbox", "");
                    let values = bbox
                        .trim()
                        .split_whitespace()
                        .filter_map(|s| s.parse::<f32>().ok())
                        .collect::<Vec<_>>();
                    
                    let current_line_start_x = match values.get(0) {
                        Some(s) => (*s / col_width_px * col_width_mm) + col.min_x,
                        None => continue,
                    };
                    
                    let current_line_start_y = match values.get(1) {
                        Some(s) => (*s / col_height_px * col_height_mm) + col.min_y,
                        None => continue,
                    };
                    
                    let current_line_end_x = match values.get(2) {
                        Some(s) => (*s / col_width_px * col_width_mm) + col.min_x,
                        None => continue,
                    };
                    
                    let current_line_end_y = match values.get(3) {
                        Some(s) => (*s / col_height_px * col_height_mm) + col.min_y,
                        None => continue,
                    };
                    
                    // new text block start
                    if current_line_start_y > block_end_y + col.line_break_after_px && 
                    !current_text_block.is_empty() {
                        text_blocks.push(Textblock {
                            text: current_text_block.join(" "),
                            start_y: block_start_y,
                            end_y: block_end_y,
                            start_x: block_start_x,
                            end_x: block_end_x,
                        });
                        
                        block_start_y = current_line_start_y;
                        block_end_y = current_line_start_y;
                        block_start_x = current_line_start_x;
                        block_end_x = current_line_start_x;
                        current_text_block.clear();
                    }
                    
                    block_end_y = current_line_end_y.max(block_end_y);
                    block_end_x = current_line_end_x.max(block_end_x);
                    current_text_block.push(line_text.clone());
                }
            }
            
            if !current_text_block.is_empty() {
                text_blocks.push(Textblock {
                    text: current_text_block.join(" "),
                    start_y: block_start_y,
                    end_y: block_end_y,
                    start_x: block_start_x,
                    end_x: block_end_x,
                });
            }
            
            // Textblöcke pdftotext
            let texts_on_page = pdftotext.seiten
                .get(&format!("{}", seitenzahl))
                .map(|s| s.texte.clone())
                .unwrap_or_default();
            
            for t in texts_on_page {
                if column_contains_point(col, t.start_x, t.start_y) {
                    let mut merge = false;
                    
                    if let Some(last_y) = text_blocks.last().map(|last_t| last_t.end_y) {
                        if t.start_y - last_y < col.line_break_after_px {
                            merge = true;
                        }
                    }
                    
                    if merge {
                        if let Some(l) = text_blocks.last_mut() {
                            l.text.push_str(&format!(" {}", t.text));
                            l.end_x = l.end_x.max(t.end_x);
                            l.start_x = l.start_x.min(t.start_x);
                            l.start_y = l.start_y.min(t.start_y);
                            l.end_y = l.end_y.max(t.end_y);
                        }
                    } else {
                        text_blocks.push(t.clone());
                    }
                }
            }
            
            Ok(text_blocks)

        } else {
        
            let mut zellen = zeilen_vordefiniert.iter().map(|z| Rect {
                min_x: col.min_x,
                min_y: col.min_y,
                max_x: col.max_x,
                max_y: col.max_y,
            }).collect::<Vec<_>>();
            
            zellen.push(Rect {
                min_x: col.min_x,
                min_y: col.min_y,
                max_x: col.max_x,
                max_y: col.max_y,
            });
            
            for (i, y) in zeilen_vordefiniert.iter().enumerate() {
                zellen[i + 1].min_y = *y;
                zellen[i].max_y = *y;
            }
            
            let mut zeilen = Vec::new();

            if let Ok(m) = document.select(css_selector) {

                for css_match in m {
            
                    let as_node = css_match.as_node();
                    let as_element = match as_node.as_element() {
                        Some(s) => s,
                        None => continue,
                    };
                    
                    let bbox_attribute = as_element.attributes.borrow();
                    let bbox = (&*bbox_attribute)
                    .get("title")
                    .and_then(|b| b.split(";").next());
                    
                    let line_text = as_node
                        .text_contents()
                        .lines()
                        .map(|l| l.trim().to_string())
                        .collect::<Vec<_>>()
                        .join(" ")
                        .trim()
                        .to_string();
                    
                    // "bbox 882 201 1227 254"
                    let bbox_clean = match bbox {
                        Some(s) => s,
                        None => continue,
                    };
                    
                    // startx, starty, endx, endy
                    // 882 201 1227 254
                    let bbox = bbox_clean.replace("bbox", "");
                    let values = bbox
                        .trim()
                        .split_whitespace()
                        .filter_map(|s| s.parse::<f32>().ok())
                        .collect::<Vec<_>>();
                    
                    let current_line_start_x = match values.get(0) {
                        Some(s) => (*s / col_width_px * col_width_mm) + col.min_x,
                        None => continue,
                    };
                    
                    let current_line_start_y = match values.get(1) {
                        Some(s) => (*s / col_height_px * col_height_mm) + col.min_y,
                        None => continue,
                    };
                    
                    let current_line_end_x = match values.get(2) {
                        Some(s) => (*s / col_width_px * col_width_mm) + col.min_x,
                        None => continue,
                    };
                    
                    let current_line_end_y = match values.get(3) {
                        Some(s) => (*s / col_height_px * col_height_mm) + col.min_y,
                        None => continue,
                    };
                    
                    zeilen.push(Textblock {
                        text: line_text,
                        start_y: current_line_start_y,
                        end_y: current_line_end_y,
                        start_x: current_line_start_x,
                        end_x: current_line_end_x,
                    });
                }
            }
            
            let texts_on_page = pdftotext.seiten
                .get(&format!("{}", seitenzahl))
                .map(|s| s.texte.clone())
                .unwrap_or_default();
            
            // Textblöcke pdftotext
            for t in texts_on_page {
                if column_contains_point(col, t.start_x, t.start_y) {
                    zeilen.push(t.clone());
                }
            }
                           
            fn zelle_contains_point(z: &Rect, x: f32, y: f32) -> bool {
                x <= z.max_x &&
                x >= z.min_x &&
                y <= z.max_y &&
                y >= z.min_y
            }

            Ok(zellen.into_iter().map(|z| {
                
                let texte_in_zelle = zeilen.iter().filter(|zeile| {
                    zelle_contains_point(&z, zeile.start_x, zeile.start_y)
                }).collect::<Vec<_>>();
                
                let texte_in_zelle_string = texte_in_zelle
                    .iter()
                    .map(|s| s.text.clone())
                    .collect::<Vec<_>>()
                    .join(" ");
                
                Textblock {
                    text: texte_in_zelle_string,
                    start_y: z.min_y,
                    end_y: z.max_y,
                    start_x: z.min_x,
                    end_x: z.max_x,
                }
            }).collect::<Vec<_>>())
        }
    }).collect::<Result<Vec<Vec<Textblock>>, Fehler>>()?)
}

/// Stroke path
pub const OP_PATH_PAINT_STROKE: &str                         = "S";
/// Close and stroke path
pub const OP_PATH_PAINT_STROKE_CLOSE: &str                   = "s";
/// Fill path using nonzero winding number rule
pub const OP_PATH_PAINT_FILL_NZ: &str                        = "f";
/// Fill path using nonzero winding number rule (obsolete)
pub const OP_PATH_PAINT_FILL_NZ_OLD: &str                    = "F";
/// Fill path using even-odd rule
pub const OP_PATH_PAINT_FILL_EO: &str                        = "f*";
/// Fill and stroke path using nonzero winding number rule
pub const OP_PATH_PAINT_FILL_STROKE_NZ: &str                 = "B";
/// Close, fill and stroke path using nonzero winding number rule
pub const OP_PATH_PAINT_FILL_STROKE_CLOSE_NZ: &str           = "b";
/// Fill and stroke path using even-odd rule
pub const OP_PATH_PAINT_FILL_STROKE_EO: &str                 = "B*";
/// Close, fill and stroke path using even odd rule
pub const OP_PATH_PAINT_FILL_STROKE_CLOSE_EO: &str           = "b*";
/// End path without filling or stroking
pub const OP_PATH_PAINT_END: &str                            = "n";

const OPERATIONS_TO_CLEAN: &[&str;10] = &[
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

// Löscht alle gemalten Linien aus dem PDF heraus
pub fn clean_pdf(
    pdf_bytes: &[u8], 
    titelblatt: &Titelblatt
) -> Result<Vec<u8>, Fehler> {
    
    use lopdf::Object;
    use std::collections::BTreeSet;
    use std::path::Path;
    
    let target = std::env::temp_dir()
    .join(&format!("{gemarkung}/{blatt}/temp-clean.pdf", gemarkung = titelblatt.grundbuch_von, blatt = titelblatt.blatt))
    .display().to_string();
    
    if Path::new(&target).exists() {
        return std::fs::read(target.clone())
        .map_err(|e| Fehler::Io(target.clone(), e));
    }
    
    // Dekomprimierung mit LZW funktioniert nicht, erst 
    // mit podofouncompress alle PDF-Streams dekomprimieren!
    let tmp = std::env::temp_dir()
    .join(&format!("{gemarkung}/{blatt}/decompress.pdf", gemarkung = titelblatt.grundbuch_von, blatt = titelblatt.blatt))
    .display().to_string();
    
    let _ = std::fs::write(tmp.clone(), pdf_bytes);
    let _ = Command::new("podofouncompress")
        .arg(tmp.clone())
        .arg(tmp.clone())
        .status();
    
    let pdf_bytes = std::fs::read(tmp.clone())
    .map_err(|e| Fehler::Io(tmp.clone(), e))?;
    
    let _ = std::fs::remove_file(tmp.clone());
    
    let bad_operators = OPERATIONS_TO_CLEAN.iter().map(|s| s.to_string()).collect::<BTreeSet<_>>();
    
    let mut pdf = lopdf::Document::load_mem(&pdf_bytes)?;
    
    let mut stream_ids = Vec::new();
    
    for (page_num, page_id) in pdf.get_pages().into_iter() {
        if let Some(Object::Dictionary(page_dict)) = pdf.objects.get(&page_id) {
            if let Some(Object::Dictionary(resources_dict)) = page_dict.get(b"Resources").ok() {
                if let Some(Object::Dictionary(xobjects)) = resources_dict.get(b"XObject").ok() {
                    for (_, xo) in xobjects.iter() {
                        if let Object::Reference(xobject_id) = xo {
                            stream_ids.push(xobject_id.clone());
                        }
                    }
                }
            }
        }
    }
        
    for sid in stream_ids.into_iter() {
    
            if let Some(Object::Stream(s)) = pdf.objects.get_mut(&sid) {
                                
                let mut stream_decoded = match s.decode_content().ok()  {
                    Some(s) => s,
                    None => {
                        continue;
                    },
                };
                                
                stream_decoded.operations.retain(|op| {
                    !bad_operators.contains(&op.operator)
                });
                
                s.set_plain_content(stream_decoded.encode()?);
                s.start_position = None;
            }  
    }
    
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes)
    .map_err(|e| Fehler::Io(String::new(), e))?;
    
    let _ = std::fs::write(target, &bytes);

    Ok(bytes)
}

fn column_contains_point(col: &Column, start_x: f32, start_y: f32) -> bool {
    start_x <= col.max_x &&
    start_x >= col.min_x &&
    start_y <= col.max_y &&
    start_y >= col.min_y
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
    pub fn zero() -> Self { Self::default() }
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
        self.eintraege.is_empty() &&
        self.zuschreibungen.is_empty() &&
        self.abschreibungen.is_empty()
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
            BvEintrag::Flurstueck(BvEintragFlurstueck { lfd_nr, bisherige_lfd_nr, gemarkung, flur, flurstueck, .. }) => {
                write!(f, "{lfd_nr}: Gemarkung {gemarkung:?} Flur {flur} Flurstück {flurstueck} (bisher lfd. Nr. {bisherige_lfd_nr:?})")
            },
            BvEintrag::Recht(BvEintragRecht { lfd_nr, zu_nr, bisherige_lfd_nr, .. }) => {
                let zu_nr = zu_nr.text();
                write!(f, "Grundstücksgleiches Recht {lfd_nr} (zu Nr. {zu_nr}, bisher {bisherige_lfd_nr:?})")
            },
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
                flst.lfd_nr == 0 &&
                flst.bisherige_lfd_nr == None &&
                flst.flur == 0 &&
                flst.flurstueck == String::new() &&
                flst.gemarkung == None &&
                flst.bezeichnung == None &&
                flst.groesse.ist_leer()
            },
            BvEintrag::Recht(recht) => {
                recht.lfd_nr == 0 &&
                recht.bisherige_lfd_nr == None &&
                recht.text.is_empty()
            }
        }
    }
    
    pub fn ist_geroetet(&self) -> bool {
        match self {
            BvEintrag::Flurstueck(flst) => {
                flst.manuell_geroetet.unwrap_or(flst.automatisch_geroetet.unwrap_or(false))
            },
            BvEintrag::Recht(recht) => {
                recht.manuell_geroetet.unwrap_or(recht.automatisch_geroetet.unwrap_or(false))
            }
        }
    }
        
    pub fn set_bezeichnung(&mut self, val: String) {
        match self {
            BvEintrag::Flurstueck(flst) => { flst.bezeichnung = if val.is_empty() { None } else { Some(val.into()) }; },
            BvEintrag::Recht(_) => { },
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
            BvEintrag::Flurstueck(_) => { },
            BvEintrag::Recht(recht) => { recht.zu_nr = val.into(); },
        }
    }
    
    pub fn set_recht_text(&mut self, val: String) {
        match self {
            BvEintrag::Flurstueck(_) => { },
            BvEintrag::Recht(recht) => { recht.text = val.into(); },
        }
    }
    
    pub fn set_gemarkung(&mut self, val: Option<String>) {
        match self {
            BvEintrag::Flurstueck(flst) => { flst.gemarkung = val; },
            BvEintrag::Recht(_) => { },
        }
    }
    
    pub fn set_flur(&mut self, val: usize) {
        match self {
            BvEintrag::Flurstueck(flst) => { flst.flur = val; },
            BvEintrag::Recht(_) => { },
        }
    }
    
    pub fn set_flurstueck(&mut self, val: String) {
        match self {
            BvEintrag::Flurstueck(flst) => { flst.flurstueck = val; },
            BvEintrag::Recht(_) => { },
        }
    }
    
    pub fn set_groesse(&mut self, val: FlurstueckGroesse) {
        match self {
            BvEintrag::Flurstueck(flst) => { flst.groesse = val; },
            BvEintrag::Recht(_) => { },
        }
    }
    
    
    pub fn unset_automatisch_geroetet(&mut self) {
        match self {
            BvEintrag::Flurstueck(flst) => { flst.automatisch_geroetet = None; },
            BvEintrag::Recht(recht) => { recht.automatisch_geroetet = None; },
        }
    }
    
    pub fn get_automatisch_geroetet(&self) -> Option<bool> {
        match self {
            BvEintrag::Flurstueck(flst) => { flst.automatisch_geroetet },
            BvEintrag::Recht(recht) => { recht.automatisch_geroetet },
        }
    }
    
    pub fn set_automatisch_geroetet(&mut self, val: bool) {
        match self {
            BvEintrag::Flurstueck(flst) => { flst.automatisch_geroetet = Some(val); },
            BvEintrag::Recht(recht) => { recht.automatisch_geroetet = Some(val); },
        }
    }
    
    pub fn get_manuell_geroetet(&self) -> Option<bool> {
        match self {
            BvEintrag::Flurstueck(flst) => { flst.manuell_geroetet },
            BvEintrag::Recht(recht) => { recht.manuell_geroetet },
        }
    }
    
    pub fn set_manuell_geroetet(&mut self, val: Option<bool>) {
        match self {
            BvEintrag::Flurstueck(flst) => { flst.manuell_geroetet = val; },
            BvEintrag::Recht(recht) => { recht.manuell_geroetet = val; },
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
        focus_type: FocusType
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
        StringOrLines::MultiLine(
            s.lines()
            .map(|s| s.to_string())
            .collect()
        )
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
    Metrisch { 
        m2: Option<u64>
    },
    #[serde(rename = "ha")]
    Hektar { 
        ha: Option<u64>, 
        a: Option<u64>, 
        m2: Option<u64>,
    }
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
            FlurstueckGroesse::Hektar { ha, a, m2 } => ha.unwrap_or(0) * 100_000 + a.unwrap_or(0) * 100 + m2.unwrap_or(0),
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
        if fi.is_empty() { format!("0") } else { fi }
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
        self.manuell_geroetet.or(self.automatisch_geroetet.clone()).unwrap_or(false)
    }
    pub fn ist_leer(&self) -> bool {
        self.bv_nr.is_empty() &&
        self.text.is_empty()
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
        self.manuell_geroetet.or(self.automatisch_geroetet.clone()).unwrap_or(false)
    }
    
    pub fn ist_leer(&self) -> bool {
        self.bv_nr.is_empty() &&
        self.text.is_empty()
    }
}

pub fn analysiere_bv(
    titelblatt: &Titelblatt,
    pdftotext_layout: &PdfToTextLayout,
    seiten: &BTreeMap<String, SeiteParsed>, 
    anpassungen_seite: &BTreeMap<String, AnpassungSeite>,
    konfiguration: &Konfiguration,
) -> Result<Bestandsverzeichnis, Fehler> {

    let seitenzahlen = seiten.keys().cloned()
        .filter_map(|s| s.parse::<usize>().ok())
        .collect::<Vec<_>>();
    
    let max_seitenzahl = seitenzahlen.iter().copied().max().unwrap_or(0);
    
    let default_texte = Vec::new();
    let mut last_lfd_nr = 1;

    let mut bv_eintraege = seiten
    .iter()
    .filter(|(num, s)| {
        s.typ == SeitenTyp::BestandsverzeichnisHorz || 
        s.typ == SeitenTyp::BestandsverzeichnisVert ||
        s.typ == SeitenTyp::BestandsverzeichnisVertTyp2
    })
    .filter_map(|(num, s)| {
        Some((num.parse::<u32>().ok()?, s))
    })
    .flat_map(|(seitenzahl, s)| {
        
        let zeilen_auf_seite = anpassungen_seite
            .get(&format!("{}", seitenzahl))
            .map(|aps| aps.zeilen.clone())
            .unwrap_or_default();
        
        if s.typ == SeitenTyp::BestandsverzeichnisHorz {
            
            if !zeilen_auf_seite.is_empty() {
                (0..(zeilen_auf_seite.len() + 1)).map(|i| {
                
                    let mut position_in_pdf = PositionInPdf {
                        seite: seitenzahl,
                        rect: OptRect::zero(),
                    };
                    
                    let lfd_nr = s.texte
                    .get(0)
                    .and_then(|zeilen| zeilen.get(i))
                    .and_then(|t| {
                        position_in_pdf.expand(&t);
                        let numeric_chars = String::from_iter(t.text.chars().filter(|c| c.is_numeric()));                            
                        numeric_chars.parse::<usize>().ok()
                    }).unwrap_or(0);
                    
                    let bisherige_lfd_nr = s.texte
                    .get(1)
                    .and_then(|zeilen| zeilen.get(i))
                    .and_then(|t| {
                        position_in_pdf.expand(&t);
                        let numeric_chars = String::from_iter(t.text.chars().filter(|c| c.is_numeric()));                            
                        numeric_chars.parse::<usize>().ok()
                    });
                    
                    let gemarkung = s.texte
                    .get(2)
                    .and_then(|zeilen| zeilen.get(i))
                    .map(|t| {
                        position_in_pdf.expand(&t);
                        t.text.trim().to_string()
                    })
                    .unwrap_or_default();
                    
                    let gemarkung = if gemarkung.is_empty() { None } else { Some(gemarkung) };
                    
                    let flur = s.texte
                    .get(3)
                    .and_then(|zeilen| zeilen.get(i))
                    .and_then(|t| {
                        position_in_pdf.expand(&t);
                        let numeric_chars = String::from_iter(t.text.chars().filter(|c| c.is_numeric()));                            
                        numeric_chars.parse::<usize>().ok()
                    })
                    .unwrap_or_default();
                    
                    let flurstueck = s.texte
                    .get(4)
                    .and_then(|zeilen| zeilen.get(i))
                    .and_then(|t| {
                        position_in_pdf.expand(&t);
                        let numeric_chars = String::from_iter(t.text.chars().filter(|c| c.is_numeric() || *c == '/'));                            
                        Some(numeric_chars)
                    })
                    .unwrap_or_default();
                    
                    let bezeichnung = s.texte
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
                        clean_text_python(bezeichnung.trim(), konfiguration).ok().map(|o| o.into())
                    };
                    
                    let ha = s.texte
                    .get(6)
                    .and_then(|zeilen| zeilen.get(i))
                    .and_then(|t| {
                        position_in_pdf.expand(&t);
                        let numeric_chars = String::from_iter(t.text.chars().filter(|c| c.is_numeric()));                            
                        numeric_chars.parse::<u64>().ok()
                    });
                    
                    let a = s.texte
                    .get(7)
                    .and_then(|zeilen| zeilen.get(i))
                    .and_then(|t| {
                        position_in_pdf.expand(&t);
                        let numeric_chars = String::from_iter(t.text.chars().filter(|c| c.is_numeric()));                            
                        numeric_chars.parse::<u64>().ok()
                    });
                
                    let m2 = s.texte
                    .get(8)
                    .and_then(|zeilen| zeilen.get(i))
                    .and_then(|t| {
                        position_in_pdf.expand(&t);
                        let numeric_chars = String::from_iter(t.text.chars().filter(|c| c.is_numeric()));                            
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
                }).collect::<Vec<_>>()
            } else {
                s.texte.get(4)
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
                    ).and_then(|t| { position_in_pdf.expand(&t); t.text.parse::<usize>().ok() }) {
                        Some(s) => s,
                        None => last_lfd_nr,
                    };
                    
                    last_lfd_nr = lfd_nr;
                    
                    let bisherige_lfd_nr = get_erster_text_bei_ca(
                        &s.texte.get(1).unwrap_or(&default_texte),
                        lfd_num,
                        flurstueck_start_y,
                        flurstueck_end_y,
                    ).and_then(|t| { position_in_pdf.expand(&t); t.text.parse::<usize>().ok() });
                    
                    let mut gemarkung = if s.typ == SeitenTyp::BestandsverzeichnisHorz {
                        get_erster_text_bei_ca(&s.texte.get(2).unwrap_or(&default_texte), lfd_num, flurstueck_start_y, flurstueck_end_y)
                        .map(|t| { position_in_pdf.expand(&t); t.text.trim().to_string()})
                    } else { 
                        None 
                    };
                                    
                    let flur = {
                        if s.typ == SeitenTyp::BestandsverzeichnisHorz {
                            get_erster_text_bei_ca(&s.texte.get(3).unwrap_or(&default_texte), lfd_num, flurstueck_start_y, flurstueck_end_y)
                            .and_then(|t| {
                                position_in_pdf.expand(&t); 
                                let numeric_chars = String::from_iter(t.text.chars().filter(|c| c.is_numeric()));                            
                                numeric_chars.parse::<usize>().ok()
                            })?
                        } else {
                            get_erster_text_bei_ca(&s.texte.get(2).unwrap_or(&default_texte), lfd_num, flurstueck_start_y, flurstueck_end_y)
                            .and_then(|t| {
                                position_in_pdf.expand(&t); 
                                // ignoriere Zusatzbemerkungen zu Gemarkung
                                let numeric_chars = String::from_iter(t.text.chars().filter(|c| c.is_numeric()));
                                let non_numeric_chars = String::from_iter(t.text.chars().filter(|c| c.is_alphabetic()));
                                
                                if !non_numeric_chars.is_empty() {
                                    gemarkung = Some(non_numeric_chars.trim().to_string());
                                }
                                
                                numeric_chars.parse::<usize>().ok()
                            })?
                        }
                    };
                    
                    let bezeichnung = if s.typ == SeitenTyp::BestandsverzeichnisHorz {
                        get_erster_text_bei_ca(&s.texte.get(5).unwrap_or(&default_texte), lfd_num, flurstueck_start_y, flurstueck_end_y)
                        .map(|t| { position_in_pdf.expand(&t); t.text.trim().to_string().into() })
                    } else {
                        get_erster_text_bei_ca(&s.texte.get(4).unwrap_or(&default_texte), lfd_num, flurstueck_start_y, flurstueck_end_y)
                        .map(|t| { position_in_pdf.expand(&t); t.text.trim().to_string().into() })
                    };
                    
                    let groesse = if s.typ == SeitenTyp::BestandsverzeichnisHorz {
                        let ha = get_erster_text_bei_ca(&s.texte.get(6).unwrap_or(&default_texte), lfd_num, flurstueck_start_y, flurstueck_end_y)
                        .and_then(|t| { position_in_pdf.expand(&t); t.text.parse::<u64>().ok() });
                        let a = get_erster_text_bei_ca(&s.texte.get(7).unwrap_or(&default_texte), lfd_num, flurstueck_start_y, flurstueck_end_y)
                        .and_then(|t| { position_in_pdf.expand(&t); t.text.parse::<u64>().ok() });
                        let m2 = get_erster_text_bei_ca(&s.texte.get(8).unwrap_or(&default_texte), lfd_num, flurstueck_start_y, flurstueck_end_y)
                        .and_then(|t| { position_in_pdf.expand(&t); t.text.parse::<u64>().ok() });
                        
                        FlurstueckGroesse::Hektar { ha, a, m2 }
                    } else {
                        let m2 = get_erster_text_bei_ca(&s.texte.get(5).unwrap_or(&default_texte), lfd_num, flurstueck_start_y, flurstueck_end_y)
                        .and_then(|t| { position_in_pdf.expand(&t); t.text.parse::<u64>().ok() });
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
        } else if s.typ == SeitenTyp:: BestandsverzeichnisVert {
            if !zeilen_auf_seite.is_empty() {
                (0..(zeilen_auf_seite.len() + 1)).map(|i| {
                    
                    let mut position_in_pdf = PositionInPdf {
                        seite: seitenzahl,
                        rect: OptRect::zero(),
                    };
                    
                    let lfd_nr = s.texte
                    .get(0)
                    .and_then(|zeilen| zeilen.get(i))
                    .and_then(|t| {
                        position_in_pdf.expand(&t); 
                        let numeric_chars = String::from_iter(t.text.chars().filter(|c| c.is_numeric()));                            
                        numeric_chars.parse::<usize>().ok()
                    }).unwrap_or(0);
                    
                    let bisherige_lfd_nr = s.texte
                    .get(1)
                    .and_then(|zeilen| zeilen.get(i))
                    .and_then(|t| {
                        position_in_pdf.expand(&t); 
                        let numeric_chars = String::from_iter(t.text.chars().filter(|c| c.is_numeric()));                            
                        numeric_chars.parse::<usize>().ok()
                    });
                    
                    let mut gemarkung = None;
                    
                    let flur = s.texte
                    .get(2)
                    .and_then(|zeilen| zeilen.get(i))
                    .and_then(|t| {
                        position_in_pdf.expand(&t); 
                        // ignoriere Zusatzbemerkungen zu Gemarkung
                        let numeric_chars = String::from_iter(t.text.chars().filter(|c| c.is_numeric()));
                        let non_numeric_chars = String::from_iter(t.text.chars().filter(|c| c.is_alphabetic()));
                        
                        if !non_numeric_chars.is_empty() {
                            let gemarkung_str = non_numeric_chars.trim().to_string();
                            gemarkung = if gemarkung_str.is_empty() { None } else { Some(gemarkung_str) };
                        }
                        
                        numeric_chars.parse::<usize>().ok()
                    })
                    .unwrap_or_default();
                    
                    let flurstueck = s.texte
                    .get(3)
                    .and_then(|zeilen| zeilen.get(i))
                    .and_then(|t| {
                        position_in_pdf.expand(&t); 
                        let numeric_chars = String::from_iter(t.text.chars().filter(|c| c.is_numeric() || *c == '/'));                            
                        Some(numeric_chars)
                    })
                    .unwrap_or_default();
                    
                    let bezeichnung = s.texte
                    .get(4)
                    .and_then(|zeilen| zeilen.get(i))
                    .map(|t| { 
                        position_in_pdf.expand(&t); 
                        t.text.trim().to_string() 
                    })
                    .unwrap_or_default();
                    
                    let bezeichnung = if bezeichnung.is_empty() { None } else { Some(bezeichnung.into()) };
                    
                    let m2 = s.texte
                    .get(5)
                    .and_then(|zeilen| zeilen.get(i))
                    .and_then(|t| {
                        position_in_pdf.expand(&t); 
                        let numeric_chars = String::from_iter(t.text.chars().filter(|c| c.is_numeric()));                            
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
                }).collect::<Vec<_>>()
            } else {
                s.texte.get(0)
                .unwrap_or(&default_texte)
                .iter().enumerate()
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
                    ).and_then(|t| { position_in_pdf.expand(&t); t.text.parse::<usize>().ok() });
                    
                    let mut gemarkung = None;
                                    
                    let flur = get_erster_text_bei_ca(&s.texte.get(2).unwrap_or(&default_texte), lfd_num, lfd_nr_start_y, lfd_nr_end_y)
                    .and_then(|t| {
                        position_in_pdf.expand(&t); 
                        
                        // ignoriere Zusatzbemerkungen zu Gemarkung
                        let numeric_chars = String::from_iter(t.text.chars().filter(|c| c.is_numeric()));
                        let non_numeric_chars = String::from_iter(t.text.chars().filter(|c| c.is_alphabetic()));
                        
                        if !non_numeric_chars.is_empty() {
                            gemarkung = Some(non_numeric_chars.trim().to_string());
                        }
                        
                        numeric_chars.parse::<usize>().ok()
                    })?;
                    
                    let flurstueck = get_erster_text_bei_ca(&s.texte.get(3).unwrap_or(&default_texte), lfd_num, lfd_nr_start_y, lfd_nr_end_y)
                        .map(|t| { position_in_pdf.expand(&t); t.text.trim().to_string() })?;
                        
                    let bezeichnung = get_erster_text_bei_ca(&s.texte.get(4).unwrap_or(&default_texte), lfd_num, lfd_nr_start_y, lfd_nr_end_y)
                        .map(|t| { position_in_pdf.expand(&t); t.text.trim().to_string().into() });
                    
                    let groesse = {
                        let m2 = get_erster_text_bei_ca(&s.texte.get(5).unwrap_or(&default_texte), lfd_num, lfd_nr_start_y, lfd_nr_end_y)
                        .and_then(|t| { position_in_pdf.expand(&t); t.text.parse::<u64>().ok() });
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
                (0..(zeilen_auf_seite.len() + 1)).map(|i| {
                    
                    let mut position_in_pdf = PositionInPdf {
                        seite: seitenzahl,
                        rect: OptRect::zero(),
                    };
                    
                    let lfd_nr = s.texte
                    .get(0)
                    .and_then(|zeilen| zeilen.get(i))
                    .and_then(|t| {
                        position_in_pdf.expand(&t);
                        let numeric_chars = String::from_iter(t.text.chars().filter(|c| c.is_numeric()));                            
                        numeric_chars.parse::<usize>().ok()
                    }).unwrap_or(0);
                    
                    let bisherige_lfd_nr = s.texte
                    .get(1)
                    .and_then(|zeilen| zeilen.get(i))
                    .and_then(|t| {
                        position_in_pdf.expand(&t); 
                        let numeric_chars = String::from_iter(t.text.chars().filter(|c| c.is_numeric()));                            
                        numeric_chars.parse::<usize>().ok()
                    });
                    
                    let mut gemarkung = None;
                    let mut flurstueck = String::new();
                    let mut flur = 0;
                    
                    if let Some(s) = s.texte.get(2).and_then(|zeilen| zeilen.get(i)) {
                        let mut split_whitespace = s.text.trim().split_whitespace().rev();
                        position_in_pdf.expand(&s); 
                        flurstueck = split_whitespace.next().map(|s| {
                            String::from_iter(s.chars().filter(|c| c.is_numeric() || *c == '/'))
                        }).unwrap_or_default();
                        flur = split_whitespace.next().and_then(|s| {
                            let numeric_chars = String::from_iter(s.chars().filter(|c| c.is_numeric()));
                            numeric_chars.parse::<usize>().ok()
                        }).unwrap_or_default();
                        let gemarkung_str = split_whitespace.into_iter().map(|s| s.to_string()).collect::<Vec<_>>().join(" ");
                        gemarkung = if gemarkung_str.is_empty() { None } else { Some(gemarkung_str) };
                    }
                    
                    let bezeichnung = s.texte
                    .get(3)
                    .and_then(|zeilen| zeilen.get(i))
                    .map(|t| { position_in_pdf.expand(&t); t.text.trim().to_string() })
                    .unwrap_or_default();
                    
                    let bezeichnung = if bezeichnung.is_empty() { None } else { Some(bezeichnung.into()) };
                    
                    let m2 = s.texte
                    .get(4)
                    .and_then(|zeilen| zeilen.get(i))
                    .and_then(|t| {
                        position_in_pdf.expand(&t); 
                        let numeric_chars = String::from_iter(t.text.chars().filter(|c| c.is_numeric()));                            
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
                }).collect::<Vec<_>>()
            } else {
                s.texte.get(0)
                .unwrap_or(&default_texte)
                .iter().enumerate()
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
                    ).and_then(|t| t.text.parse::<usize>().ok());
                    
                    let mut gemarkung = None;
                    let mut flurstueck = String::new();
                    let mut flur = 0;
                    
                    if let Some(s) = get_erster_text_bei_ca(&s.texte.get(2).unwrap_or(&default_texte), lfd_num, lfd_nr_start_y, lfd_nr_end_y) {
                        let mut split_whitespace = s.text.trim().split_whitespace().rev();
                        
                        flurstueck = split_whitespace.next().map(|s| {
                            String::from_iter(s.chars().filter(|c| c.is_numeric() || *c == '/'))
                        }).unwrap_or_default();
                        flur = split_whitespace.next().and_then(|s| {
                            let numeric_chars = String::from_iter(s.chars().filter(|c| c.is_numeric()));
                            numeric_chars.parse::<usize>().ok()
                        }).unwrap_or_default();
                        let gemarkung_str = split_whitespace.into_iter().map(|s| s.to_string()).collect::<Vec<_>>().join(" ");
                        gemarkung = if gemarkung_str.is_empty() { None } else { Some(gemarkung_str) };
                    }
                        
                    let bezeichnung = get_erster_text_bei_ca(&s.texte.get(3).unwrap_or(&default_texte), lfd_num, lfd_nr_start_y, lfd_nr_end_y)
                        .map(|t| t.text.trim().to_string().into());
                    
                    let groesse = {
                        let m2 = get_erster_text_bei_ca(&s.texte.get(4).unwrap_or(&default_texte), lfd_num, lfd_nr_start_y, lfd_nr_end_y)
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
    let bv_mit_0 = bv_eintraege.iter().enumerate().filter_map(|(i, bv)| {
        if bv.get_lfd_nr() == 0 { Some(i) } else { None }
    }).collect::<Vec<_>>();
    
    for bv_idx in bv_mit_0 {
        
        let bv_clone = bv_eintraege[bv_idx].clone();
        if bv_idx == 0 { continue; }
        
        let bv_idx_minus_eins = bv_idx - 1;
        
        let bv_minus_eins_clone = bv_eintraege[bv_idx_minus_eins].clone();
        
        if bv_minus_eins_clone.get_lfd_nr() == 0 {
            continue;
        }
                
        let mut remove = false;
        let (bv_clone_neu, bv_minus_eins_clone_neu) = match (bv_clone, bv_minus_eins_clone) {
            (BvEintrag::Flurstueck(mut bv_clone), BvEintrag::Flurstueck(mut bv_minus_eins_clone)) => {
                
                if  bv_clone.bisherige_lfd_nr.is_some() && 
                    bv_minus_eins_clone.bisherige_lfd_nr.is_none() {
                    
                    bv_minus_eins_clone.bisherige_lfd_nr = bv_clone.bisherige_lfd_nr;
                    remove = true;
                }
                
                if  bv_clone.gemarkung.is_some() && 
                    bv_minus_eins_clone.gemarkung.is_none() {
                    
                    bv_minus_eins_clone.gemarkung = bv_clone.gemarkung.clone();
                    remove = true;
                }
                
                if  bv_clone.flur == 0 && 
                    bv_minus_eins_clone.flur != 0 {
                    
                    bv_minus_eins_clone.flur = bv_clone.flur.clone();
                    remove = true;
                }
                
                if   bv_clone.flurstueck.is_empty() && 
                    !bv_minus_eins_clone.flurstueck.is_empty() {
                    
                    bv_minus_eins_clone.flurstueck = bv_clone.flurstueck.clone();
                    remove = true;
                }
                
                if  bv_clone.bezeichnung.is_none() && 
                    !bv_minus_eins_clone.bezeichnung.is_none() {
                    
                    bv_minus_eins_clone.bezeichnung = bv_clone.bezeichnung.clone();
                    remove = true;
                }
                
                if  bv_clone.groesse.ist_leer() && 
                    !bv_minus_eins_clone.groesse.ist_leer() {
                
                    bv_minus_eins_clone.groesse = bv_clone.groesse.clone();
                    remove = true;
                }
                
                if remove {
                    bv_clone = BvEintragFlurstueck::neu(0);
                }
                
                (BvEintrag::Flurstueck(bv_clone), BvEintrag::Flurstueck(bv_minus_eins_clone))
            },
            (BvEintrag::Recht(mut bv_clone), BvEintrag::Recht(mut bv_minus_eins_clone)) => {
                
                if  bv_clone.bisherige_lfd_nr.is_some() && 
                    bv_minus_eins_clone.bisherige_lfd_nr.is_none() {
                    
                    bv_minus_eins_clone.bisherige_lfd_nr = bv_clone.bisherige_lfd_nr;
                    remove = true;
                }
                
                if  bv_clone.text.is_empty() && 
                    !bv_minus_eins_clone.text.is_empty() {
                    
                    bv_minus_eins_clone.text = bv_clone.text.clone();
                    remove = true;
                }
                
                if remove {
                    bv_clone = BvEintragRecht::neu(0);
                }
                
                (BvEintrag::Recht(bv_clone), BvEintrag::Recht(bv_minus_eins_clone))
            },
            (a, b) => (a, b)
        };
        
        bv_eintraege[bv_idx] = bv_clone_neu;
        bv_eintraege[bv_idx - 1] = bv_minus_eins_clone_neu;
    }
    
    let mut bv_eintraege = bv_eintraege
        .into_iter()
        .filter(|bv| !bv.ist_leer())
        .collect::<Vec<BvEintrag>>();
    
    let bv_mit_irregulaerer_lfd_nr = bv_eintraege.iter().enumerate().filter_map(|(i, bv)| {
        if i == 0 { return None; }
        if bv_eintraege[i - 1].get_lfd_nr() > bv.get_lfd_nr() { Some(i) } else { None }
    }).collect::<Vec<_>>();
    
    let bv_irr_korrigieren = bv_mit_irregulaerer_lfd_nr.into_iter().filter_map(|bv_irr| {
        let vorherige_lfd = bv_eintraege.get(bv_irr - 1)?.get_lfd_nr();
        let naechste_lfd = bv_eintraege.get(bv_irr + 1)?.get_lfd_nr();
        match naechste_lfd - vorherige_lfd {
            2 => Some((bv_irr, vorherige_lfd + 1)),
            1 => if bv_eintraege[bv_irr].get_bisherige_lfd_nr() == Some(vorherige_lfd) { Some((bv_irr, naechste_lfd)) } else { None }
            _ => None,
        }
    }).collect::<Vec<(usize, usize)>>();
    
    for (idx, lfd_neu) in bv_irr_korrigieren {
        if let Some(bv) = bv_eintraege.get_mut(idx) {
            bv.set_lfd_nr(lfd_neu);
        }
    }
    
    let bv_bestand_und_zuschreibungen = seiten
    .iter()
    .filter(|(num, s)| {
        s.typ == SeitenTyp::BestandsverzeichnisHorzZuUndAbschreibungen || 
        s.typ == SeitenTyp::BestandsverzeichnisVertZuUndAbschreibungen
    })
    .filter_map(|(num, s)| {
        Some((num.parse::<u32>().ok()?, s))
    })
    .flat_map(|(seitenzahl, s)| {
    
        let zeilen_auf_seite = anpassungen_seite
            .get(&format!("{}", seitenzahl))
            .map(|aps| aps.zeilen.clone())
            .unwrap_or_default();
    
        if !zeilen_auf_seite.is_empty() {
            (0..(zeilen_auf_seite.len() + 1)).map(|i| {
                
                let zur_lfd_nr = s.texte
                .get(0)
                .and_then(|zeilen| zeilen.get(i))
                .map(|t| t.text.trim().to_string())
                .unwrap_or_default();

                let bestand_und_zuschreibungen = s.texte
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
            }).collect::<Vec<_>>()
        } else {
            s.texte.get(0).unwrap_or(&default_texte).iter().enumerate().filter_map(|(lfd_num, lfd_nr_text)| {
            
                // TODO: auch texte "1-3"
                let zur_lfd_nr = lfd_nr_text.text.trim().to_string();
                                
                let lfd_nr_text_start_y = lfd_nr_text.start_y;
                let lfd_nr_text_end_y = lfd_nr_text.start_y;
                
                let bestand_und_zuschreibungen = get_erster_text_bei_ca(&s.texte.get(1).unwrap_or(&default_texte), lfd_num, lfd_nr_text_start_y, lfd_nr_text_end_y)
                    .map(|t| t.text.trim().to_string())?;            
                
                Some(BvZuschreibung {
                    bv_nr: zur_lfd_nr.into(),
                    text: bestand_und_zuschreibungen.into(),
                    automatisch_geroetet: None,
                    manuell_geroetet: None,
                    position_in_pdf: None,
                })
            }).collect::<Vec<_>>()
        }.into_iter()
        
    })
    .filter(|bvz| !bvz.ist_leer())
    .collect();
    
    let bv_abschreibungen = seiten
    .iter()
    .filter(|(num, s)| {
        s.typ == SeitenTyp::BestandsverzeichnisHorzZuUndAbschreibungen || 
        s.typ == SeitenTyp::BestandsverzeichnisVertZuUndAbschreibungen
    })
    .filter_map(|(num, s)| {
        Some((num.parse::<u32>().ok()?, s))
    })
    .flat_map(|(seitenzahl, s)| {
    
        let zeilen_auf_seite = anpassungen_seite
            .get(&format!("{}", seitenzahl))
            .map(|aps| aps.zeilen.clone())
            .unwrap_or_default();
            
        if !zeilen_auf_seite.is_empty() {
            (0..(zeilen_auf_seite.len() + 1)).map(|i| {
                
                let zur_lfd_nr = s.texte
                .get(2)
                .and_then(|zeilen| zeilen.get(i))
                .map(|t| t.text.trim().to_string())
                .unwrap_or_default();
                
                let abschreibungen = s.texte
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
            }).collect::<Vec<_>>()
        } else {
            s.texte.get(2).unwrap_or(&default_texte).iter().enumerate().filter_map(|(lfd_num, lfd_nr_text)| {
            
                // TODO: auch texte "1-3"
                let zur_lfd_nr = lfd_nr_text.text.trim().to_string();
                                
                let lfd_nr_text_start_y = lfd_nr_text.start_y;
                let lfd_nr_text_end_y = lfd_nr_text.end_y;

                let abschreibungen = get_erster_text_bei_ca(&s.texte.get(3).unwrap_or(&default_texte), lfd_num, lfd_nr_text_start_y, lfd_nr_text_end_y)
                    .map(|t| t.text.trim().to_string())?;            
                
                Some(BvAbschreibung {
                    bv_nr: zur_lfd_nr.into(),
                    text: abschreibungen.into(),
                    automatisch_geroetet: None,
                    manuell_geroetet: None,
                    position_in_pdf: None,
                })
            }).collect::<Vec<_>>()
        
        }.into_iter()
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

pub fn bv_eintraege_roeten(
    bv_eintraege: &mut [BvEintrag], 
    titelblatt: &Titelblatt, 
    max_seitenzahl: u32, 
    pdftotext_layout: &PdfToTextLayout
) {    
    // Automatisch BV Einträge röten
    bv_eintraege
    .par_iter_mut()
    .for_each(|bv| {
        
        // Cache nutzen !!!
        if bv.get_automatisch_geroetet().is_some() {
            return;
        }
                
        let ist_geroetet = {
            if let Some(position_in_pdf) = bv.get_position_in_pdf() {
                
                use image::Pixel;
                use image::GenericImageView;

                let bv_rect = position_in_pdf.get_rect();
                
                let temp_ordner = std::env::temp_dir()
                .join(&format!("{gemarkung}/{blatt}", gemarkung = titelblatt.grundbuch_von, blatt = titelblatt.blatt));
                
                let temp_pdf_pfad = temp_ordner.clone().join("temp.pdf");
                let pdftoppm_output_path = temp_ordner.clone().join(format!("page-{}.png", crate::digital::formatiere_seitenzahl(position_in_pdf.seite, max_seitenzahl)));
                
                match image::open(&pdftoppm_output_path).ok()
                .and_then(|o| {
                    
                    let (im_width, im_height) = o.dimensions();
                    let (page_width, page_height) = pdftotext_layout.seiten
                        .get(&format!("{}", position_in_pdf.seite))
                        .map(|o| (o.breite_mm, o.hoehe_mm))?;
                    
                    let im_width = im_width as f32;
                    let im_height = im_height as f32;
                    
                    Some(o.crop_imm(
                        (bv_rect.min_x / page_width * im_width).round() as u32, 
                        (bv_rect.min_y / page_height * im_height).round() as u32, 
                        (
                            (bv_rect.max_x - bv_rect.min_x).abs() / 
                            page_width * 
                            im_width
                        ).round() as u32, 
                        (
                            (bv_rect.max_y - bv_rect.min_y).abs() / 
                            page_height * 
                            im_height
                        ).round() as u32, 
                    ).to_rgb8())
                }) {
                    Some(cropped) => cropped.pixels().any(|px| {                        
                        px.channels().get(0).copied().unwrap_or(0) > 200 && 
                        px.channels().get(1).copied().unwrap_or(0) < 120 &&
                        px.channels().get(2).copied().unwrap_or(0) < 120
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
        self.eintraege.is_empty() &&
        self.grundlagen_eintragungen.is_empty() &&
        self.veraenderungen.is_empty() &&
        self.loeschungen.is_empty()
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
                },
                Abt1Eintrag::V2(v2) => Abt1Eintrag::V2(v2),
            };
        
            *e = neu;
        }
        
        self.grundlagen_eintragungen.extend(grundlage_eintragungen_neu.into_iter());
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
        self.manuell_geroetet.or(self.automatisch_geroetet.clone()).unwrap_or(false)
    }
}

impl Abt1EintragV1 {
    pub fn ist_geroetet(&self) -> bool {
        self.manuell_geroetet.or(self.automatisch_geroetet.clone()).unwrap_or(false)
    }
}

impl Abt1EintragV2 {
    pub fn ist_geroetet(&self) -> bool {
        self.manuell_geroetet.or(self.automatisch_geroetet.clone()).unwrap_or(false)
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
            Abt1Eintrag::V1(v1) => { v1.lfd_nr = lfd_nr; },
            Abt1Eintrag::V2(v2) => { v2.lfd_nr = lfd_nr; },
        }
    }
    
    pub fn get_lfd_nr(&self) -> usize {
        match self {
            Abt1Eintrag::V1(v1) => { v1.lfd_nr },
            Abt1Eintrag::V2(v2) => { v2.lfd_nr },
        }
    }
    
    pub fn set_manuell_geroetet(&mut self, m: Option<bool>) {
        match self {
            Abt1Eintrag::V1(v1) => { v1.manuell_geroetet = m; },
            Abt1Eintrag::V2(v2) => { v2.manuell_geroetet = m; },
        }
    }
    
    pub fn get_manuell_geroetet(&self) -> Option<bool> {
        match self {
            Abt1Eintrag::V1(v1) => { v1.manuell_geroetet },
            Abt1Eintrag::V2(v2) => { v2.manuell_geroetet },
        }
    }
    
    pub fn get_automatisch_geroetet(&self) -> bool {
        match self {
            Abt1Eintrag::V1(v1) => { v1.automatisch_geroetet.unwrap_or(false) },
            Abt1Eintrag::V2(v2) => { v2.automatisch_geroetet.unwrap_or(false) },
        }
    }
    
    pub fn get_eigentuemer(&self) -> String {
        match self {
            Abt1Eintrag::V1(v1) => { v1.eigentuemer.text() },
            Abt1Eintrag::V2(v2) =>{ v2.eigentuemer.text() },
        }
    }
    
    pub fn set_eigentuemer(&mut self, eigentuemer: String) {
        match self {
            Abt1Eintrag::V1(v1) => { v1.eigentuemer = eigentuemer.into(); },
            Abt1Eintrag::V2(v2) =>{ v2.eigentuemer = eigentuemer.into(); },
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
    seiten: &BTreeMap<String, SeiteParsed>, 
    anpassungen_seite: &BTreeMap<String, AnpassungSeite>,
    bestandsverzeichnis: &Bestandsverzeichnis,
    konfiguration: &Konfiguration,
) -> Result<Abteilung1, Fehler> {
    
    let default_texte = Vec::new();
    
    let abt1_eintraege = seiten
    .iter()
    .filter(|(num, s)| {
        s.typ == SeitenTyp::Abt1Vert || 
        s.typ == SeitenTyp::Abt1Horz
    }).flat_map(|(seitenzahl, s)| {
    
        let zeilen_auf_seite = anpassungen_seite
            .get(&format!("{}", seitenzahl))
            .map(|aps| aps.zeilen.clone())
            .unwrap_or_default();
        
        if !zeilen_auf_seite.is_empty() {
            (0..(zeilen_auf_seite.len() + 1)).filter_map(|i| {
                
                let lfd_nr = s.texte
                    .get(0)
                    .and_then(|zeilen| zeilen.get(i))
                    .and_then(|t| {
                        let numeric_chars = String::from_iter(t.text.chars().filter(|c| c.is_numeric()));                            
                        numeric_chars.parse::<usize>().ok()
                    }).unwrap_or(0);

                let eigentuemer = s.texte
                .get(1)
                .and_then(|zeilen| zeilen.get(i))
                .map(|t| t.text.trim().to_string())
                .unwrap_or_default();
                
                Some(Abt1Eintrag::V2(Abt1EintragV2 {
                    lfd_nr,
                    eigentuemer: clean_text_python(eigentuemer.trim(), konfiguration).ok()?.into(),
                    version: 2,
                    automatisch_geroetet: None,
                    manuell_geroetet: None,
                    position_in_pdf: None,
                }))
            }).collect::<Vec<_>>()
        } else {
            let mut texte = s.texte.clone();
            texte.get_mut(1).unwrap().retain(|t| t.text.trim().len() > 12 && t.text.trim().contains(" "));
            
            texte.get(1).unwrap_or(&default_texte).iter().enumerate().filter_map(|(text_num, text)| {
                
                let text_start_y = text.start_y;
                let text_end_y = text.end_y;

                // TODO: bv-nr korrigieren!

                // TODO: auch texte "1-3"
                let lfd_nr = get_erster_text_bei_ca(
                    &texte.get(0).unwrap_or(&default_texte), 
                    text_num,
                    text_start_y,
                    text_end_y,
                ).and_then(|s| s.text.trim().parse::<usize>().ok()).unwrap_or(0);
                
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
                    eigentuemer: clean_text_python(eigentuemer.trim(), konfiguration).ok()?.into(),
                    version: 2,
                    automatisch_geroetet: None,
                    manuell_geroetet: None,
                    position_in_pdf: None,
                }))
            })
            .collect::<Vec<_>>()
        }.into_iter()
    }).collect();
    
    let abt1_grundlagen_eintragungen = seiten
    .iter()
    .filter(|(num, s)| {
        s.typ == SeitenTyp::Abt1Vert || 
        s.typ == SeitenTyp::Abt1Horz
    }).flat_map(|(seitenzahl, s)| {
    
        let zeilen_auf_seite = anpassungen_seite
            .get(&format!("{}", seitenzahl))
            .map(|aps| aps.zeilen.clone())
            .unwrap_or_default();
        
        if !zeilen_auf_seite.is_empty() {
            (0..(zeilen_auf_seite.len() + 1)).filter_map(|i| {
                
                let bv_nr = s.texte
                .get(2)
                .and_then(|zeilen| zeilen.get(i))
                .map(|t| t.text.trim().to_string())
                .unwrap_or_default();
                
                let grundlage_der_eintragung = s.texte
                .get(3)
                .and_then(|zeilen| zeilen.get(i))
                .map(|t| t.text.trim().to_string())
                .unwrap_or_default();
                
                Some(Abt1GrundEintragung {
                    bv_nr: bv_nr.into(),
                    text: clean_text_python(grundlage_der_eintragung.trim(), konfiguration).ok()?.into(),
                    automatisch_geroetet: None,
                    manuell_geroetet: None,
                    position_in_pdf: None,
                })
            }).collect::<Vec<_>>()
        } else {
            let mut texte = s.texte.clone();
            texte.get_mut(3).unwrap().retain(|t| t.text.trim().len() > 12 && t.text.trim().contains(" "));
            
            texte.get(3).unwrap_or(&default_texte).iter().enumerate().filter_map(|(text_num, text)| {
                
                let text_start_y = text.start_y;
                let text_end_y = text.end_y;

                let bv_nr = get_erster_text_bei_ca(
                    &texte.get(2).unwrap_or(&default_texte), 
                    text_num,
                    text_start_y,
                    text_end_y,
                ).map(|t| t.text.trim().to_string())?;
                
                let grundlage_der_eintragung = get_erster_text_bei_ca(
                    &texte.get(3).unwrap_or(&default_texte), 
                    text_num,
                    text_start_y,
                    text_end_y,
                ).map(|t| t.text.trim().to_string())?;
                
                Some(Abt1GrundEintragung {
                    bv_nr: bv_nr.into(),
                    text: clean_text_python(grundlage_der_eintragung.trim(), konfiguration).ok()?.into(),
                    automatisch_geroetet: None,
                    manuell_geroetet: None,
                    position_in_pdf: None,
                })
            })
            .collect::<Vec<_>>()
        }.into_iter()
    }).collect();
    
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
        self.eintraege.is_empty() &&
        self.veraenderungen.is_empty() &&
        self.loeschungen.is_empty()
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
        self.manuell_geroetet.or(self.automatisch_geroetet.clone()).unwrap_or(false)
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
        
        let mut hoechste_onr_oeffentlich = v.iter()
        .filter_map(|v| if v.typ == Some(Oeffentlich) { v.ordnungsnummer } else { None })
        .max()
        .unwrap_or(810000);
        
        let mut hoechste_onr_bank = v.iter()
        .filter_map(|v| if v.typ == Some(Bank) { v.ordnungsnummer } else { None })
        .max()
        .map(|s| s + 1)
        .unwrap_or(812000);
        
        let mut hoechste_onr_agrar = v.iter()
        .filter_map(|v| if v.typ == Some(AgrarGenossenschaft) { v.ordnungsnummer } else { None })
        .max()
        .map(|s| s + 1)
        .unwrap_or(813000);
        
        let mut hoechste_onr_privat = v.iter()
        .filter_map(|v| if v.typ == Some(PrivateigentuemerHerr) || 
                           v.typ == Some(PrivateigentuemerFrau) || 
                           v.typ == Some(PrivateigentuemerMehrere) { v.ordnungsnummer } else { None })
        .max()
        .map(|s| s + 1)
        .unwrap_or(814000);
        
        let mut hoechste_onr_jew = v.iter()
        .filter_map(|v| if v.typ == Some(JewEigentuemerDesFlurstuecks) { v.ordnungsnummer } else { None })
        .max()
        .map(|s| s + 1)
        .unwrap_or(815000);
        
        let mut hoechste_onr_leitung = v.iter()
        .filter_map(|v| if v.typ == Some(Leitungsbetreiber) { v.ordnungsnummer } else { None })
        .max()
        .map(|s| s + 1)
        .unwrap_or(817000);
        
        let mut hoechste_onr_gmbh = v.iter()
        .filter_map(|v| if v.typ == Some(GmbH) { v.ordnungsnummer } else { None })
        .max()
        .map(|s| s + 1)
        .unwrap_or(819000);

        for e in v.iter_mut() {
            if e.ordnungsnummer.is_some() { continue; }
            let typ = match e.typ {
                Some(s) => s,
                None => continue,
            };
            match typ {
                Oeffentlich => { 
                    e.ordnungsnummer = Some(hoechste_onr_oeffentlich); 
                    hoechste_onr_oeffentlich += 1; 
                },
                Bank => { 
                    e.ordnungsnummer = Some(hoechste_onr_bank); 
                    hoechste_onr_bank += 1; 
                },
                AgrarGenossenschaft => { 
                    e.ordnungsnummer = Some(hoechste_onr_agrar); 
                    hoechste_onr_agrar += 1; 
                },
                PrivateigentuemerHerr | PrivateigentuemerFrau | PrivateigentuemerMehrere => { 
                    e.ordnungsnummer = Some(hoechste_onr_privat); 
                    hoechste_onr_privat += 1; 
                },
                JewEigentuemerDesFlurstuecks => { 
                    e.ordnungsnummer = Some(hoechste_onr_jew); 
                    hoechste_onr_jew += 1; 
                },
                Leitungsbetreiber => { 
                    e.ordnungsnummer = Some(hoechste_onr_leitung); 
                    hoechste_onr_leitung += 1; 
                },
                GmbH => { 
                    e.ordnungsnummer = Some(hoechste_onr_gmbh); 
                    hoechste_onr_gmbh += 1; 
                },
            }
        }
    }
}



#[derive(Debug, Clone, PartialEq, Copy, PartialOrd, Serialize, Deserialize)]
pub enum NebenbeteiligterTyp {
    #[serde(rename="OEFFENTLICH")]
    Oeffentlich,
    #[serde(rename="BANK")]
    Bank,
    #[serde(rename="AGRAR")]
    AgrarGenossenschaft,
    #[serde(rename="PRIVAT")]
    PrivateigentuemerMehrere,
    #[serde(rename="PRIVAT-M")]
    PrivateigentuemerHerr,
    #[serde(rename="PRIVAT-F")]
    PrivateigentuemerFrau,
    #[serde(rename="JEW-EIGENT")]
    JewEigentuemerDesFlurstuecks,
    #[serde(rename="LEITUNG")]
    Leitungsbetreiber,
    #[serde(rename="GMBH")]
    GmbH
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
            "OEFFENTLICH"	=> Some(Oeffentlich),
            "BANK"			=> Some(Bank),
            "AGRAR"			=> Some(AgrarGenossenschaft),
            "PRIVAT-M"		=> Some(PrivateigentuemerHerr),
            "PRIVAT-F"		=> Some(PrivateigentuemerFrau),
            "PRIVAT"		=> Some(PrivateigentuemerMehrere),
            "JEW-EIGENT"	=> Some(JewEigentuemerDesFlurstuecks),
            "LEITUNG"		=> Some(Leitungsbetreiber),
            "GMBH"			=> Some(GmbH),
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
           lower.contains("verwaltung") {
            Some(NebenbeteiligterTyp::Oeffentlich)
        } else if 
            lower.contains("bank") || 
            lower.contains("sparkasse") {
            Some(NebenbeteiligterTyp::Bank)
        } else if 
            lower.contains("agrar") {
            Some(NebenbeteiligterTyp::AgrarGenossenschaft)
        } else if 
            lower.contains("gas") || 
            lower.contains("e.dis") || 
            lower.contains("pck") || 
            lower.contains("netz") ||
            lower.contains("wind") {
            Some(NebenbeteiligterTyp::Leitungsbetreiber)
        } else if lower.contains("mbh") {
            Some(NebenbeteiligterTyp::GmbH)
        } else if lower.contains("geb") || lower.trim().split_whitespace().count() == 2 {
            Some(NebenbeteiligterTyp::PrivateigentuemerMehrere)
        } else  {
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
        self.manuell_geroetet.or(self.automatisch_geroetet.clone()).unwrap_or(false)
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
        self.manuell_geroetet.or(self.automatisch_geroetet.clone()).unwrap_or(false)
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
        self.manuell_geroetet.or(self.automatisch_geroetet.clone()).unwrap_or(false)
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
        self.manuell_geroetet.or(self.automatisch_geroetet.clone()).unwrap_or(false)
    }
}

pub fn analysiere_abt2(
    seiten: &BTreeMap<String, SeiteParsed>, 
    anpassungen_seite: &BTreeMap<String, AnpassungSeite>,
    bestandsverzeichnis: &Bestandsverzeichnis,
    konfiguration: &Konfiguration,
) -> Result<Abteilung2, Fehler> {
        
    let default_texte = Vec::new();
    
    let abt2_eintraege = seiten
    .iter()
    .filter(|(num, s)| {
        s.typ == SeitenTyp::Abt2Vert || 
        s.typ == SeitenTyp::Abt2Vert
    }).flat_map(|(seitenzahl, s)| {
    
        let zeilen_auf_seite = anpassungen_seite
        .get(&format!("{}", seitenzahl))
        .map(|aps| aps.zeilen.clone())
        .unwrap_or_default();
        
        if !zeilen_auf_seite.is_empty() {
            (0..(zeilen_auf_seite.len() + 1)).filter_map(|i| {
                
                let lfd_nr = s.texte
                .get(0)
                .and_then(|zeilen| zeilen.get(i))
                .and_then(|t| {
                    let numeric_chars = String::from_iter(t.text.chars().filter(|c| c.is_numeric()));                            
                    numeric_chars.parse::<usize>().ok()
                }).unwrap_or(0);

                let bv_nr = s.texte
                .get(1)
                .and_then(|zeilen| zeilen.get(i))
                .map(|t| t.text.trim().to_string())
                .unwrap_or_default();
                
                let text = s.texte
                .get(2)
                .and_then(|zeilen| zeilen.get(i))
                .map(|t| t.text.trim().to_string())
                .unwrap_or_default();
                
                Some(Abt2Eintrag {
                    lfd_nr,
                    bv_nr: bv_nr.into(),
                    text: clean_text_python(text.trim(), konfiguration).ok()?.into(),
                    automatisch_geroetet: None,
                    manuell_geroetet: None,
                    position_in_pdf: None,
                })
            }).collect::<Vec<_>>()
        
        } else {
            
            let mut texte = s.texte.clone();
            texte.get_mut(2).unwrap().retain(|t| t.text.trim().len() > 12 && t.text.trim().contains(" "));
            
            texte.get(2).unwrap_or(&default_texte).iter().enumerate().filter_map(|(text_num, text)| {
                
                let text_start_y = text.start_y;
                let text_end_y = text.end_y;

                // TODO: bv-nr korrigieren!

                // TODO: auch texte "1-3"
                let lfd_nr = get_erster_text_bei_ca(
                    &texte.get(0).unwrap_or(&default_texte), 
                    text_num,
                    text_start_y,
                    text_end_y,
                ).and_then(|s| s.text.trim().parse::<usize>().ok()).unwrap_or(0);
                            
                let bv_nr = get_erster_text_bei_ca(
                    &texte.get(1).unwrap_or(&default_texte), 
                    text_num,
                    text_start_y,
                    text_end_y,
                ).map(|t| t.text.trim().to_string())?;
                            
                // versehentlich Fußzeile erwischt
                if bv_nr.contains("JVA Branden") {
                    return None;
                }
                
                Some(Abt2Eintrag {
                    lfd_nr,
                    bv_nr: bv_nr.to_string().into(),
                    text: clean_text_python(text.text.trim(), konfiguration).ok()?.into(),
                    automatisch_geroetet: None,
                    manuell_geroetet: None,
                    position_in_pdf: None,
                })
            })
            .collect::<Vec<_>>()   
        }.into_iter()
        
    }).collect();
    
    let abt2_veraenderungen = seiten
    .iter()
    .filter(|(num, s)| {
        s.typ == SeitenTyp::Abt2VertVeraenderungen || 
        s.typ == SeitenTyp::Abt2HorzVeraenderungen
    }).flat_map(|(seitenzahl, s)| {

        let zeilen_auf_seite = anpassungen_seite
        .get(&format!("{}", seitenzahl))
        .map(|aps| aps.zeilen.clone())
        .unwrap_or_default();
        
        if !zeilen_auf_seite.is_empty() {
            (0..(zeilen_auf_seite.len() + 1)).filter_map(|i| {
                
                let lfd_nr = s.texte
                .get(0)
                .and_then(|zeilen| zeilen.get(i))
                .map(|t| t.text.trim().to_string())
                .unwrap_or_default();
                
                let text = s.texte
                .get(1)
                .and_then(|zeilen| zeilen.get(i))
                .map(|t| t.text.trim().to_string())
                .unwrap_or_default();
                
                Some(Abt2Veraenderung {
                    lfd_nr: lfd_nr.into(),
                    text: clean_text_python(text.trim(), konfiguration).ok()?.into(),
                    automatisch_geroetet: None,
                    manuell_geroetet: None,
                    position_in_pdf: None,
                })
            }).collect::<Vec<_>>()
        } else {
            let mut texte = s.texte.clone();
            texte.get_mut(1).unwrap().retain(|t| t.text.trim().len() > 12 && t.text.trim().contains(" "));

            texte.get(1).unwrap_or(&default_texte).iter().enumerate().filter_map(|(text_num, text)| {
                
                let text_start_y = text.start_y;
                let text_end_y = text.end_y;

                // TODO: bv-nr korrigieren!

                // TODO: auch texte "1-3"
                let lfd_nr = get_erster_text_bei_ca(
                    &texte.get(0).unwrap_or(&default_texte), 
                    text_num,
                    text_start_y,
                    text_end_y,
                ).map(|s| s.text.trim().to_string())?;
                                
                // TODO: recht analysieren!
                
                Some(Abt2Veraenderung {
                    lfd_nr: lfd_nr.into(),
                    text: clean_text_python(text.text.trim(), konfiguration).ok()?.into(),
                    automatisch_geroetet: None,
                    manuell_geroetet: None,
                    position_in_pdf: None,
                })
            })
            .collect::<Vec<_>>()
        }.into_iter()
        
    }).collect();
    
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
        self.eintraege.is_empty() &&
        self.veraenderungen.is_empty() &&
        self.loeschungen.is_empty()
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
        self.manuell_geroetet.or(self.automatisch_geroetet.clone()).unwrap_or(false)
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
        self.manuell_geroetet.or(self.automatisch_geroetet.clone()).unwrap_or(false)
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
        self.manuell_geroetet.or(self.automatisch_geroetet.clone()).unwrap_or(false)
    }
}

pub fn analysiere_abt3(
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
    .filter(|(num, s)| {
        s.typ == SeitenTyp::Abt3Horz || 
        s.typ == SeitenTyp::Abt3Vert
    }).flat_map(|(seitenzahl, s)| {
    
        let zeilen_auf_seite = anpassungen_seite
        .get(&format!("{}", seitenzahl))
        .map(|aps| aps.zeilen.clone())
        .unwrap_or_default();
        
        if !zeilen_auf_seite.is_empty() {
            (0..(zeilen_auf_seite.len() + 1)).filter_map(|i| {
                
                let lfd_nr = s.texte
                .get(0)
                .and_then(|zeilen| zeilen.get(i))
                .and_then(|t| {
                    let numeric_chars = String::from_iter(t.text.chars().filter(|c| c.is_numeric()));                            
                    numeric_chars.parse::<usize>().ok()
                }).unwrap_or(0);
                
                let bv_nr = s.texte
                .get(1)
                .and_then(|zeilen| zeilen.get(i))
                .map(|t| t.text.trim().to_string())
                .unwrap_or_default();
                
                let betrag = s.texte
                .get(2)
                .and_then(|zeilen| zeilen.get(i))
                .map(|t| t.text.trim().to_string())
                .unwrap_or_default();
                
                let text = s.texte
                .get(3)
                .and_then(|zeilen| zeilen.get(i))
                .map(|t| t.text.trim().to_string())
                .unwrap_or_default();
                
                Some(Abt3Eintrag {
                    lfd_nr: lfd_nr.into(),
                    bv_nr: bv_nr.to_string().into(),
                    betrag: betrag.trim().to_string().into(),
                    text: clean_text_python(text.trim(), konfiguration).ok()?.into(),
                    automatisch_geroetet: None,
                    manuell_geroetet: None,
                    position_in_pdf: None,
                })
            }).collect::<Vec<_>>()
        
        } else {
        
            let mut texte = s.texte.clone();
            texte.get_mut(2).unwrap().retain(|t| t.text.trim().len() > 12 && t.text.trim().contains(" "));
            
            texte.get(3).unwrap_or(&default_texte).iter().enumerate().filter_map(|(text_num, text)| {
                
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
                .and_then(|t| t.text.parse::<usize>().ok()) {
                    Some(s) => s,
                    None => last_lfd_nr,
                };
                
                last_lfd_nr = lfd_nr + 1;
                            
                let bv_nr = get_erster_text_bei_ca(
                    &texte.get(1).unwrap_or(&default_texte), 
                    text_num,
                    text_start_y,
                    text_end_y,
                ).map(|t| t.text.trim().to_string())?;
                
                let betrag = get_erster_text_bei_ca(
                    &texte.get(2).unwrap_or(&default_texte), 
                    text_num,
                    text_start_y,
                    text_end_y,
                ).map(|t| t.text.trim().to_string())?;
                
                // TODO: recht analysieren!
                
                // versehentlich Fußzeile erwischt
                if bv_nr.contains("JVA Branden") {
                    return None;
                }
                
                Some(Abt3Eintrag {
                    lfd_nr: lfd_nr.into(),
                    bv_nr: bv_nr.to_string().into(),
                    betrag: betrag.trim().to_string().into(),
                    text: clean_text_python(text.text.trim(), konfiguration).ok()?.into(),
                    automatisch_geroetet: None,
                    manuell_geroetet: None,
                    position_in_pdf: None,
                })
            })
            .collect::<Vec<_>>()
        }.into_iter()
        
    }).collect();
    
    let abt3_veraenderungen = seiten
    .iter()
    .filter(|(num, s)| {
        s.typ == SeitenTyp::Abt3HorzVeraenderungenLoeschungen || 
        s.typ == SeitenTyp::Abt3VertVeraenderungenLoeschungen || 
        s.typ == SeitenTyp::Abt3VertVeraenderungen
    }).flat_map(|(seitenzahl, s)| {
        if s.typ == SeitenTyp::Abt3VertVeraenderungen {
            
            let zeilen_auf_seite = anpassungen_seite
            .get(&format!("{}", seitenzahl))
            .map(|aps| aps.zeilen.clone())
            .unwrap_or_default();
            
            if !zeilen_auf_seite.is_empty() {
                (0..(zeilen_auf_seite.len() + 1)).filter_map(|i| {
                    
                    let lfd_nr = s.texte
                    .get(0)
                    .and_then(|zeilen| zeilen.get(i))
                    .map(|t| t.text.trim().to_string())
                    .unwrap_or_default();
                
                    let betrag = s.texte
                    .get(1)
                    .and_then(|zeilen| zeilen.get(i))
                    .map(|t| t.text.trim().to_string())
                    .unwrap_or_default();
                    
                    let text = s.texte
                    .get(2)
                    .and_then(|zeilen| zeilen.get(i))
                    .map(|t| t.text.trim().to_string())
                    .unwrap_or_default();
                    
                    Some(Abt3Veraenderung {
                        lfd_nr: lfd_nr.into(),
                        betrag: betrag.into(),
                        text: clean_text_python(text.trim(), konfiguration).ok()?.into(),
                        automatisch_geroetet: None,
                        manuell_geroetet: None,
                        position_in_pdf: None,
                    })
                }).collect::<Vec<_>>()
            } else {
                let mut texte = s.texte.clone();
                texte.get_mut(2).unwrap().retain(|t| t.text.trim().len() > 12 && t.text.trim().contains(" "));

                texte.get(2).unwrap_or(&default_texte).iter().enumerate().filter_map(|(text_num, text)| {
                    
                    let text_start_y = text.start_y;
                    let text_end_y = text.end_y;

                    // TODO: auch texte "1-3"
                    let lfd_nr = get_erster_text_bei_ca(
                        &texte.get(0).unwrap_or(&default_texte), 
                        text_num,
                        text_start_y,
                        text_end_y,
                    ).map(|s| s.text.trim().to_string()).unwrap_or_default();
                
                    let betrag = get_erster_text_bei_ca(
                        &texte.get(1).unwrap_or(&default_texte), 
                        text_num,
                        text_start_y,
                        text_end_y,
                    ).map(|s| s.text.trim().to_string()).unwrap_or_default();
                    
                    Some(Abt3Veraenderung {
                        lfd_nr: lfd_nr.into(),
                        betrag: betrag.into(),
                        text: clean_text_python(&text.text.trim(), konfiguration).ok()?.into(),
                        automatisch_geroetet: None,
                        manuell_geroetet: None,
                        position_in_pdf: None,
                    })
                })
                .collect::<Vec<_>>()
            }
        } else { 
            Vec::new() 
        }.into_iter()
    }).collect();
    
    let abt3_loeschungen = seiten
    .iter()
    .filter(|(num, s)| {
        s.typ == SeitenTyp::Abt3HorzVeraenderungenLoeschungen || 
        s.typ == SeitenTyp::Abt3VertVeraenderungenLoeschungen || 
        s.typ == SeitenTyp::Abt3VertLoeschungen
    }).flat_map(|(seitenzahl, s)| {
        if s.typ == SeitenTyp::Abt3VertLoeschungen {
            
            let column_shift = match s.typ {
                Abt3HorzVeraenderungenLoeschungen |
                Abt3VertVeraenderungenLoeschungen => 3,
                _ => 0,
            };
            
            let zeilen_auf_seite = anpassungen_seite
            .get(&format!("{}", seitenzahl))
            .map(|aps| aps.zeilen.clone())
            .unwrap_or_default();
            
            if !zeilen_auf_seite.is_empty() {
                (0..(zeilen_auf_seite.len() + 1)).filter_map(|i| {
                    
                    let lfd_nr = s.texte
                    .get(0 + column_shift)
                    .and_then(|zeilen| zeilen.get(i))
                    .map(|t| t.text.trim().to_string())
                    .unwrap_or_default();
                    
                    let betrag = s.texte
                    .get(1 + column_shift)
                    .and_then(|zeilen| zeilen.get(i))
                    .map(|t| t.text.trim().to_string())
                    .unwrap_or_default();
                    
                    let text = s.texte
                    .get(2 + column_shift)
                    .and_then(|zeilen| zeilen.get(i))
                    .map(|t| t.text.trim().to_string())
                    .unwrap_or_default();
                    
                    Some(Abt3Loeschung {
                        lfd_nr: lfd_nr.into(),
                        betrag: betrag.into(),
                        text: clean_text_python(text.trim(), konfiguration).ok()?.into(),
                        automatisch_geroetet: None,
                        manuell_geroetet: None,
                        position_in_pdf: None,
                    })
                }).collect::<Vec<_>>()
            } else {
                let mut texte = s.texte.clone();
                
                texte
                .get_mut(2 + column_shift)
                .unwrap()
                .retain(|t| t.text.trim().len() > 12 && t.text.trim().contains(" "));

                texte
                .get(2 + column_shift)
                .unwrap_or(&default_texte).iter().enumerate().filter_map(|(text_num, text)| {
                    
                    let text_start_y = text.start_y;
                    let text_end_y = text.end_y;

                    // TODO: auch texte "1-3"
                    let lfd_nr = get_erster_text_bei_ca(
                        &texte.get(0 + column_shift).unwrap_or(&default_texte), 
                        text_num,
                        text_start_y,
                        text_end_y,
                    ).map(|s| s.text.trim().to_string()).unwrap_or_default();
                    
                    let betrag = get_erster_text_bei_ca(
                        &texte.get(1 + column_shift).unwrap_or(&default_texte), 
                        text_num,
                        text_start_y,
                        text_end_y,
                    ).map(|s| s.text.trim().to_string()).unwrap_or_default();
                    
                    Some(Abt3Loeschung {
                        lfd_nr: lfd_nr.into(),
                        betrag: betrag.into(),
                        text: clean_text_python(text.text.trim(), konfiguration).ok()?.into(),
                        automatisch_geroetet: None,
                        manuell_geroetet: None,
                        position_in_pdf: None,
                    })
                })
                .collect::<Vec<_>>()
            }
        } else { 
            Vec::new() 
        }.into_iter()
    }).collect();
    
    Ok(Abteilung3 {
        eintraege: abt3_eintraege,
        veraenderungen: abt3_veraenderungen,
        loeschungen: abt3_loeschungen,
    })
}

fn clean_text_python(text: &str, konfiguration: &Konfiguration) -> Result<String, String> {
    
    use pyo3::Python;
    use crate::kurztext::python_text_saubern;
    
    let text_sauber = Python::with_gil(|py| {
        python_text_saubern(py, text, konfiguration)
        .map_err(|e| format!("In Funktion text_säubern(): {}", e))
    })?;
    Ok(text_sauber)
}

fn get_erster_text_bei_ca(texte: &[Textblock], skip: usize, start: f32, ziel: f32) -> Option<&Textblock> {    
    for t in texte.iter().skip(skip.saturating_sub(1)) {
        let start = start - 20.0;
        // let ziel = ziel + 20.0;
        if t.start_y > start || !(t.end_y < start) {
            return Some(t)
        }
    }
    
    None
}
