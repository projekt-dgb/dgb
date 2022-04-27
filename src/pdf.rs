use crate::{Titelblatt, Grundbuch, Abt2Eintrag, Abt3Eintrag};
use crate::digitalisiere::BvEintrag;
use printpdf::{
    BuiltinFont, PdfDocument, Mm, IndirectFontRef,
    PdfDocumentReference, PdfLayerReference,
    Line, Point, Color, Cmyk
};
use std::path::Path;

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

}

/*
fn generate_grundbuch_pdf(config: GrundbuchExport) {
    
    let (doc, page1, layer1) = PdfDocument::new(&titel, Mm(210.0), Mm(297.0), "Titelblatt");
    let fonts = PdfFonts::new(&mut doc);
    
    let gb = format!("Grundbuch von {grundbuch_von}");
    let blatt_nr = format!("Blatt {blatt}");
    let amtsgericht = format!("Amtsgericht {amtsgericht}");
    
    // Leere Seite 2
    let (_, _) = doc.add_page(Mm(210.0), Mm(297.0), "Formular");

    // Bestandsverzeichnis
    for bv in grundbuch.bestandsverzeichnis.eintraege.chunks(59) {
            
        // Bestandsverzeichnis Einträge
        let (current_page, formular_layer) = doc.add_page(Mm(210.0), Mm(297.0), "Formular");
        let text_layer = doc.get_page(current_page).add_layer("Text");
        let rot_layer = doc.get_page(current_page).add_layer("Roetungen");

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
        
        let text_1 = &[
            ("Laufende", 13.0_f64, 297.0_f64 - 21.0), 
            ("Nummer", 13.5, 297.0 - 23.5),
            ("der", 15.5, 297.0 - 26.0), 
            ("Grund-", 14.0, 297.0 - 28.5), 
            ("stücke", 14.0, 297.0 - 31.0),
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
        
        doc.get_page(current_page).get_layer(formular_layer)
        .add_shape(text_1_header);
        
        let text_2 = &["Bisherige", "laufende", "Nummer", "der Grund-", "stücke"];
        let text_3 = &["Gemarkung", "(nur bei Abweichung vom", "Grundbuchbezirk angeben)"];
        let text_4 = &["Karte"];
        let text_5 = &["Flur"];
        let text_6 = &["flurstück"];
        let text_7 = &["Wirtschaftsart und Lage"];
        let text_8 = &["Größe"];
        let text_9 = &["ha"];
        let text_10 = &["a"];
        let text_11 = &["m²"];
        let text_12 = &["Bezeichnung der Grundstücke und der mit dem Eigentum verbundene Rechte"];

        for (t, x, y) in text_1.iter() {
            doc.get_page(current_page).get_layer(formular_layer)
            .use_text(*t, 6.0, Mm(*x), Mm(*y), &helvetica);        
        }
        
        doc.get_page(current_page).get_layer(formular_layer)
        .use_text(&format!("Grundbuch von {grundbuch_von}  -  Blatt {blatt}"), 12.0, Mm(10.0), Mm(297.0 - 10.0), &times);        
        
        doc.get_page(current_page).get_layer(formular_layer)
        .use_text("Bestandsverzeichnis", 16.0, Mm(10.0), Mm(297.0 - 16.0), &times_bold);
        
        doc.get_page(current_page).get_layer(formular_layer)
        .add_shape(lfd_nr_spalte);
        
        doc.get_page(current_page).get_layer(formular_layer)
        .add_shape(bisherige_lfd_nr_spalte);
        
        doc.get_page(current_page).get_layer(formular_layer)
        .add_shape(gemarkung_spalte);
        
        doc.get_page(current_page).get_layer(formular_layer)
        .add_shape(flur_spalte);
        
        doc.get_page(current_page).get_layer(formular_layer)
        .add_shape(flurstueck_spalte);
        
        doc.get_page(current_page).get_layer(formular_layer)
        .add_shape(wirtschaftsart_lage_spalte);
        
        doc.get_page(current_page).get_layer(formular_layer)
        .add_shape(ha_spalte);
        
        doc.get_page(current_page).get_layer(formular_layer)
        .add_shape(a_spalte);
        
        doc.get_page(current_page).get_layer(formular_layer)
        .add_shape(m2_spalte);
        
        let mut start = Mm(297.0 - 41.0);
        for b in bv {
            if b.ist_geroetet() {
                text_layer.set_fill_color(Color::Cmyk(Cmyk {
                    c: 0.0,
                    m: 0.7,
                    y: 0.4,
                    k: 0.0,
                    icc_profile: None,
                }));
                rot_layer.set_outline_color(Color::Cmyk(Cmyk {
                    c: 0.0,
                    m: 0.7,
                    y: 0.4,
                    k: 0.0,
                    icc_profile: None,
                }));
            }
                    
            match b {
                BvEintrag::Flurstueck(flst) => {
                    
                    text_layer
                    .use_text(&format!("{}", flst.lfd_nr), 10.0, Mm(12.0), start, &courier_bold);
                    
                    if let Some(bisherig) = flst.bisherige_lfd_nr.as_ref() {
                        text_layer
                        .use_text(&format!("{}", bisherig), 10.0, Mm(27.0), start, &courier_bold);                
                    }

                    let gemarkung = b.get_gemarkung().clone().unwrap_or_default();
                    let gemarkung = if gemarkung == grundbuch_von { String::new() } else { gemarkung };
            
                    if let Some(gemarkung) = flst.gemarkung.as_ref() {
                        if *gemarkung != grundbuch_von {
                            text_layer.use_text(gemarkung, 10.0, Mm(42.0), start, &courier_bold);  
                        }
                    }
                    
                    text_layer
                    .use_text(&format!("{}", flst.flur), 10.0, Mm(82.0), start, &courier_bold);
                    
                    text_layer
                    .use_text(&format!("{}", flst.flurstueck), 10.0, Mm(97.0), start, &courier_bold);
                    
                    let m2 = flst.groesse.get_m2();

                    let mut m2_chars = format!("{}", m2).chars().collect::<Vec<_>>();
                    
                    let mut m2_string = Vec::new();
                    if let Some(l) = m2_chars.pop() { m2_string.push(l); }
                    if let Some(l) = m2_chars.pop() { m2_string.push(l); }
                    m2_string.reverse();
                    let m2_string: String = m2_string.into_iter().collect();
                    
                    let mut a_string = Vec::new();
                    if let Some(l) = m2_chars.pop() { a_string.push(l); }
                    if let Some(l) = m2_chars.pop() { a_string.push(l); }
                    a_string.reverse();
                    let a_string: String = a_string.into_iter().collect();

                    let ha_string: String = m2_chars.into_iter().collect();

                    text_layer
                    .use_text(&ha_string, 10.0, Mm(210.0 - 43.0), start, &courier_bold);
                    
                    text_layer
                    .use_text(&a_string, 10.0, Mm(210.0 - 28.0), start, &courier_bold);
                    
                    text_layer
                    .use_text(&m2_string, 10.0, Mm(210.0 - 18.0), start, &courier_bold);
                    
                    if b.ist_geroetet() {    
                        rot_layer
                        .add_shape(Line {
                            points: vec![
                                (Point::new(Mm(12.0), start - Mm(1.0)), false),
                                (Point::new(Mm(210.0 - 12.0), start - Mm(1.0)), false)
                            ],
                            is_closed: false,
                            has_fill: false,
                            has_stroke: true,
                            is_clipping_path: false,
                        });
                    }
                    
                    start = start - Mm(4.1);   
                },
                BvEintrag::Recht(hvm) => {
                
                }
            }
            
            if b.ist_geroetet() {
                text_layer.set_fill_color(Color::Cmyk(Cmyk {
                    c: 0.0,
                    m: 0.0,
                    y: 0.0,
                    k: 1.0,
                    icc_profile: None,
                }));
                rot_layer.set_outline_color(Color::Cmyk(Cmyk {
                    c: 0.0,
                    m: 0.0,
                    y: 0.0,
                    k: 1.0,
                    icc_profile: None,
                }));
            }
        }
    }
    
    let mut abt2_seiten: Vec<Vec<Abt2Eintrag>> = Vec::new();
    let mut cursor = 297.0 - 29.0;
    
    for abt2_eintrag in grundbuch.abt2.eintraege.iter() {
    
        let extent_y_lfd_nr = 3.43;
        let extent_y_text = wordbreak_text(&abt2_eintrag.text.text(), 50).lines().count() as f64 * 3.43;
        let extent_y_bv_nr = wordbreak_text(&clean_bv(&abt2_eintrag.bv_nr.text()), 12).lines().count() as f64 * 3.43;
        let extent_y = extent_y_bv_nr
            .max(extent_y_text)
            .max(extent_y_lfd_nr);
        
        if cursor - extent_y < 12.0 || abt2_seiten.is_empty() {
            abt2_seiten.push(Vec::new());
            cursor = 297.0 - 29.0;
        }
        
        abt2_seiten.last_mut().unwrap().push(abt2_eintrag.clone());
        cursor -= extent_y + 5.0;
    }
    
    for abt2_seite in abt2_seiten {
        
        let (current_page, formular_layer) = doc.add_page(Mm(210.0), Mm(297.0), "Formular");
        let text_layer = doc.get_page(current_page).add_layer("Text");
        let rot_layer = doc.get_page(current_page).add_layer("Roetungen");
        let mut start = Mm(297.0 - 29.0);

        for abt2_eintrag in abt2_seite {
        
            if abt2_eintrag.ist_geroetet() {
                text_layer.set_fill_color(Color::Cmyk(Cmyk {
                    c: 0.0,
                    m: 0.7,
                    y: 0.4,
                    k: 0.0,
                    icc_profile: None,
                }));
                rot_layer.set_outline_color(Color::Cmyk(Cmyk {
                    c: 0.0,
                    m: 0.7,
                    y: 0.4,
                    k: 0.0,
                    icc_profile: None,
                }));
            }
            
            let lfd_nr_spalte = Line {
                points: vec![
                    (Point::new(Mm(10.0), Mm(297.0 - 24.0)), false),
                    (Point::new(Mm(10.0), Mm(10.0)), false),
                    (Point::new(Mm(25.0), Mm(10.0)), false),
                    (Point::new(Mm(25.0), Mm(297.0 - 24.0)), false)
                ],
                is_closed: true,
                has_fill: false,
                has_stroke: true,
                is_clipping_path: false,
            };
            
            let bv_spalte = Line {
                points: vec![
                    (Point::new(Mm(25.0), Mm(297.0 - 24.0)), false),
                    (Point::new(Mm(25.0), Mm(10.0)), false),
                    (Point::new(Mm(60.0), Mm(10.0)), false),
                    (Point::new(Mm(60.0), Mm(297.0 - 24.0)), false)
                ],
                is_closed: true,
                has_fill: false,
                has_stroke: true,
                is_clipping_path: false,
            };
            
            let text_spalte = Line {
                points: vec![
                    (Point::new(Mm(60.0), Mm(297.0 - 24.0)), false),
                    (Point::new(Mm(60.0), Mm(10.0)), false),
                    (Point::new(Mm(210.0 - 10.0), Mm(10.0)), false),
                    (Point::new(Mm(210.0 - 10.0), Mm(297.0 - 24.0)), false)
                ],
                is_closed: true,
                has_fill: false,
                has_stroke: true,
                is_clipping_path: false,
            };
            
            let bv_break = wordbreak_text(&clean_bv(&abt2_eintrag.bv_nr.text()), 12);
            let text_break = wordbreak_text(&abt2_eintrag.text.text(), 50);
            
            doc.get_page(current_page).get_layer(formular_layer)
            .use_text(&format!("Grundbuch von {grundbuch_von}  -  Blatt {blatt}"), 12.0, Mm(10.0), Mm(297.0 - 10.0), &times);        
            
            doc.get_page(current_page).get_layer(formular_layer)
            .use_text("Abteilung 2 (Lasten und Beschränkungen)", 16.0, Mm(10.0), Mm(297.0 - 16.0), &times_bold);
            
            doc.get_page(current_page).get_layer(formular_layer)
            .add_shape(lfd_nr_spalte);
            
            doc.get_page(current_page).get_layer(formular_layer)
            .add_shape(bv_spalte);
            
            doc.get_page(current_page).get_layer(formular_layer)
            .add_shape(text_spalte);
            
            text_layer
            .use_text(&format!("{}", abt2_eintrag.lfd_nr), 10.0, Mm(12.0), start, &courier_bold);
        
            // write BV
            text_layer.begin_text_section();
            text_layer.set_font(&courier_bold, 10.0);
            text_layer.set_text_cursor(Mm(27.0), start);
            text_layer.set_line_height(10.0);
            for line in bv_break.lines() {
                text_layer.write_text(line.clone(), &courier_bold);
                text_layer.add_line_break();
            }
            text_layer.end_text_section();
    
            // write text
            text_layer.begin_text_section();
            text_layer.set_font(&courier_bold, 10.0);
            text_layer.set_text_cursor(Mm(62.0),start);
            text_layer.set_line_height(10.0);
            for line in text_break.lines() {
                text_layer.write_text(line.clone(), &courier_bold);
                text_layer.add_line_break();
            }
            text_layer.end_text_section();
            
            let extent_y_lfd_nr = 3.43;
            let extent_y_bv_nr = wordbreak_text(&clean_bv(&abt2_eintrag.bv_nr.text()), 12).lines().count() as f64 * 3.43;
            let extent_y_text = wordbreak_text(&abt2_eintrag.text.text(), 50).lines().count() as f64 * 3.43;
            let extent_y = extent_y_bv_nr
                .max(extent_y_text)
                .max(extent_y_lfd_nr);
                    
            if abt2_eintrag.ist_geroetet() {
                rot_layer
                .add_shape(Line {
                    points: vec![
                        (Point::new(Mm(12.0), start + Mm(3.43)), false),
                        (Point::new(Mm(210.0 - 12.0), start + Mm(3.43)), false),
                        (Point::new(Mm(12.0), start - Mm(extent_y)), false),
                        (Point::new(Mm(210.0) - Mm(12.0), start - Mm(extent_y)), false),
                    ],
                    is_closed: false,
                    has_fill: false,
                    has_stroke: true,
                    is_clipping_path: false,
                });
            }
            
            start -= Mm(extent_y + 5.0);

            if abt2_eintrag.ist_geroetet() {
                text_layer.set_fill_color(Color::Cmyk(Cmyk {
                    c: 0.0,
                    m: 0.0,
                    y: 0.0,
                    k: 1.0,
                    icc_profile: None,
                }));
                rot_layer.set_outline_color(Color::Cmyk(Cmyk {
                    c: 0.0,
                    m: 0.0,
                    y: 0.0,
                    k: 1.0,
                    icc_profile: None,
                }));
            }
        }
    }
    
    let mut abt3_seiten: Vec<Vec<Abt3Eintrag>> = Vec::new();
    let mut cursor = 297.0 - 29.0;
    
    for abt3_eintrag in grundbuch.abt3.eintraege.iter() {
    
        let extent_y_lfd_nr = 3.43;
        let extent_y_text = wordbreak_text(&abt3_eintrag.text.text(), 50).lines().count() as f64 * 3.43;
        let extent_y_bv_nr = wordbreak_text(&clean_bv(&abt3_eintrag.bv_nr.text()), 7).lines().count() as f64 * 3.43;
        let extent_y = extent_y_bv_nr
            .max(extent_y_text)
            .max(extent_y_lfd_nr);
        
        if cursor - extent_y < 12.0 || abt3_seiten.is_empty() {
            abt3_seiten.push(Vec::new());
            cursor = 297.0 - 29.0;
        }
        
        abt3_seiten.last_mut().unwrap().push(abt3_eintrag.clone());
        cursor -= extent_y + 5.0;
    }
    
    for abt3_seite in abt3_seiten {
        
        let (current_page, formular_layer) = doc.add_page(Mm(210.0), Mm(297.0), "Formular");
        let text_layer = doc.get_page(current_page).add_layer("Text");
        let rot_layer = doc.get_page(current_page).add_layer("Roetungen");
        let mut start = Mm(297.0 - 29.0);

        for abt3_eintrag in abt3_seite {
        
            if abt3_eintrag.ist_geroetet() {
                text_layer.set_fill_color(Color::Cmyk(Cmyk {
                    c: 0.0,
                    m: 0.7,
                    y: 0.4,
                    k: 0.0,
                    icc_profile: None,
                }));
                rot_layer.set_outline_color(Color::Cmyk(Cmyk {
                    c: 0.0,
                    m: 0.7,
                    y: 0.4,
                    k: 0.0,
                    icc_profile: None,
                }));
            }
            
            let lfd_nr_spalte = Line {
                points: vec![
                    (Point::new(Mm(10.0), Mm(297.0 - 24.0)), false),
                    (Point::new(Mm(10.0), Mm(10.0)), false),
                    (Point::new(Mm(25.0), Mm(10.0)), false),
                    (Point::new(Mm(25.0), Mm(297.0 - 24.0)), false)
                ],
                is_closed: true,
                has_fill: false,
                has_stroke: true,
                is_clipping_path: false,
            };
            
            let bv_spalte = Line {
                points: vec![
                    (Point::new(Mm(25.0), Mm(297.0 - 24.0)), false),
                    (Point::new(Mm(25.0), Mm(10.0)), false),
                    (Point::new(Mm(45.0), Mm(10.0)), false),
                    (Point::new(Mm(45.0), Mm(297.0 - 24.0)), false)
                ],
                is_closed: true,
                has_fill: false,
                has_stroke: true,
                is_clipping_path: false,
            };
            
            let betrag_spalte = Line {
                points: vec![
                    (Point::new(Mm(45.0), Mm(297.0 - 24.0)), false),
                    (Point::new(Mm(45.0), Mm(10.0)), false),
                    (Point::new(Mm(85.0), Mm(10.0)), false),
                    (Point::new(Mm(85.0), Mm(297.0 - 24.0)), false)
                ],
                is_closed: true,
                has_fill: false,
                has_stroke: true,
                is_clipping_path: false,
            };
            
            let text_spalte = Line {
                points: vec![
                    (Point::new(Mm(85.0), Mm(297.0 - 24.0)), false),
                    (Point::new(Mm(85.0), Mm(10.0)), false),
                    (Point::new(Mm(210.0 - 10.0), Mm(10.0)), false),
                    (Point::new(Mm(210.0 - 10.0), Mm(297.0 - 24.0)), false)
                ],
                is_closed: true,
                has_fill: false,
                has_stroke: true,
                is_clipping_path: false,
            };
            
            let bv_break = wordbreak_text(&clean_bv(&abt3_eintrag.bv_nr.text()), 7);
            let text_break = wordbreak_text(&abt3_eintrag.text.text(), 50);
            
            doc.get_page(current_page).get_layer(formular_layer)
            .use_text(&format!("Grundbuch von {grundbuch_von}  -  Blatt {blatt}"), 12.0, Mm(10.0), Mm(297.0 - 10.0), &times);        
            
            doc.get_page(current_page).get_layer(formular_layer)
            .use_text("Abteilung 3 (Schulden)", 16.0, Mm(10.0), Mm(297.0 - 16.0), &times_bold);
            
            doc.get_page(current_page).get_layer(formular_layer)
            .add_shape(lfd_nr_spalte);
            
            doc.get_page(current_page).get_layer(formular_layer)
            .add_shape(bv_spalte);
            
            doc.get_page(current_page).get_layer(formular_layer)
            .add_shape(betrag_spalte);
            
            doc.get_page(current_page).get_layer(formular_layer)
            .add_shape(text_spalte);
            
            text_layer
            .use_text(&format!("{}", abt3_eintrag.lfd_nr), 10.0, Mm(12.0), start, &courier_bold);
        
            // write BV
            text_layer.begin_text_section();
            text_layer.set_font(&courier_bold, 10.0);
            text_layer.set_text_cursor(Mm(27.0), start);
            text_layer.set_line_height(10.0);
            for line in bv_break.lines() {
                text_layer.write_text(line.clone(), &courier_bold);
                text_layer.add_line_break();
            }
            text_layer.end_text_section();
    
            text_layer
            .use_text(
                &abt3_eintrag.betrag.text(), 
                10.0, Mm(47.0), start, &courier_bold
            );
        
            // write text
            text_layer.begin_text_section();
            text_layer.set_font(&courier_bold, 10.0);
            text_layer.set_text_cursor(Mm(87.0),start);
            text_layer.set_line_height(10.0);
            for line in text_break.lines() {
                text_layer.write_text(line.clone(), &courier_bold);
                text_layer.add_line_break();
            }
            text_layer.end_text_section();
            
            let extent_y_lfd_nr = 3.43;
            let extent_y_bv_nr = wordbreak_text(&clean_bv(&abt3_eintrag.bv_nr.text()), 7).lines().count() as f64 * 3.43;
            let extent_y_text = wordbreak_text(&abt3_eintrag.text.text(), 50).lines().count() as f64 * 3.43;
            let extent_y = extent_y_bv_nr
                .max(extent_y_text)
                .max(extent_y_lfd_nr);
                    
            if abt3_eintrag.ist_geroetet() {
                rot_layer
                .add_shape(Line {
                    points: vec![
                        (Point::new(Mm(12.0), start + Mm(3.43)), false),
                        (Point::new(Mm(210.0 - 12.0), start + Mm(3.43)), false),
                        (Point::new(Mm(12.0), start - Mm(extent_y)), false),
                        (Point::new(Mm(210.0) - Mm(12.0), start - Mm(extent_y)), false),
                    ],
                    is_closed: false,
                    has_fill: false,
                    has_stroke: true,
                    is_clipping_path: false,
                });
            }
            
            start -= Mm(extent_y + 5.0);

            if abt3_eintrag.ist_geroetet() {
                text_layer.set_fill_color(Color::Cmyk(Cmyk {
                    c: 0.0,
                    m: 0.0,
                    y: 0.0,
                    k: 1.0,
                    icc_profile: None,
                }));
                rot_layer.set_outline_color(Color::Cmyk(Cmyk {
                    c: 0.0,
                    m: 0.0,
                    y: 0.0,
                    k: 1.0,
                    icc_profile: None,
                }));
            }
        }
    }
    
    doc.save_to_bytes().unwrap_or_default()
}
*/

// https://www.dariocancelliere.it/blog/2020/09/29/pdf-manipulation-with-rust-and-considerations
fn merge_pdf_files(documents: Vec<lopdf::Document>) -> Result<Vec<u8>, String> {
    
    use std::collections::BTreeMap;
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
    let mut words = s.split_whitespace().map(|s| s.to_string()).collect::<Vec<_>>();
    let mut output = String::new();
    let mut line_len = 0;
    for w in words {
        
        let word_len = w.chars().count() + 1;
        
        if line_len + word_len > max_cols {
            output.push_str("\r\n");
            line_len = 0;
        }
        
        if line_len == 0 {
            output.push_str(&format!("{w}"));
            line_len += word_len - 1;
        } else {
            output.push_str(&format!(" {w}"));
            line_len += word_len;
        }
    }
    
    output
}
