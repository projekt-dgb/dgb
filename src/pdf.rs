use printpdf::*;
use std::fs::File;
use std::io::Cursor;
use crate::{Grundbuch, Abt2Eintrag};
use crate::digitalisiere::BvEintrag;
use qrcode::{QrCode, EcLevel};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io;
use std::io::prelude::*;
use std::collections::{BTreeSet, BTreeMap};

pub fn generate_grundbuch_pdf(grundbuch: &Grundbuch) -> Vec<u8> {
    
    let grundbuch_von = grundbuch.titelblatt.grundbuch_von.clone();
    let blatt =  grundbuch.titelblatt.blatt;
    
    let titel = format!("{grundbuch_von} Blatt {blatt} (Amtsgericht {})", grundbuch.titelblatt.amtsgericht);
    let (doc, page1, layer1) = PdfDocument::new(&titel, Mm(297.0), Mm(210.0), "Titelblatt");

    let gb = format!("Grundbuch von {grundbuch_von}");
    let blatt_nr = format!("Blatt {blatt}");
    let amtsgericht = format!("Amtsgericht {}", grundbuch.titelblatt.amtsgericht);

    let times_bold = doc.add_builtin_font(BuiltinFont::TimesBoldItalic).unwrap();
    let times = doc.add_builtin_font(BuiltinFont::TimesItalic).unwrap();
    let courier_bold = doc.add_builtin_font(BuiltinFont::CourierBold).unwrap();

    // text, font size, x from left edge, y from bottom edge, font
    let current_layer = doc.get_page(page1).get_layer(layer1);
    let start = Mm(105.0);
    let rand_x = Mm(25.0);
    current_layer.use_text(&gb, 22.0, Mm(24.0), start, &times_bold);
    
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
        
    current_layer.use_text(&blatt_nr, 16.0, Mm(24.0), start - Mm(12.0), &times);
    current_layer.use_text(&amtsgericht, 16.0, Mm(24.0), start - Mm(18.0), &times);
    
    // Leere Seite 2
    let (_, _) = doc.add_page(Mm(297.0), Mm(210.0), "Formular");

    // Bestandsverzeichnis
    for bv in grundbuch.bestandsverzeichnis.eintraege.chunks(29) {
    
        use printpdf::SvgTransform;
        
        // Bestandsverzeichnis Einträge
        let (current_page, formular_layer) = doc.add_page(Mm(297.0), Mm(210.0), "Formular");
        let text_layer = doc.get_page(current_page).add_layer("Text");
        let rot_layer = doc.get_page(current_page).add_layer("Roetungen");

        let csv = bv.iter().map(|b| {
            let lfd_nr = format!("{}", b.get_lfd_nr());
            let bisherige_lfd_nr = match b.get_bisherige_lfd_nr() {
                Some(s) => format!("{}", s),
                None => String::new(),
            };
            let rot = if b.ist_geroetet() { "R" } else { "" };
            let flur = format!("{}", b.get_flur());
            let flurstueck = format!("{}", b.get_flurstueck());
            let gemarkung = b.get_gemarkung().clone().unwrap_or_default();
            let gemarkung = if gemarkung == grundbuch_von { String::new() } else { gemarkung };
            let groesse = b.get_groesse().map(|g| format!("{}", g.get_m2())).unwrap_or_default();
            format!("{rot}\t{lfd_nr}\t{bisherige_lfd_nr}\t{flur}\t{flurstueck}\t{gemarkung}\t{groesse}")
        }).collect::<Vec<_>>()
        .join("\n");
        
        let images = string_to_qr_codes(&csv);
        for (i, img) in images.into_iter().rev().enumerate() {
            img.add_to_layer(doc.get_page(current_page).get_layer(formular_layer), ImageTransform {
                translate_x: Some(Mm(297.0 - 28.0 - (i as f64 * 25.0))),
                translate_y: Some(Mm(210.0 - 22.0)),
                dpi: Some(105.0),
                .. Default::default()
            });
        }

        let lfd_nr_spalte = Line {
            points: vec![
                (Point::new(Mm(10.0), Mm(210.0 - 24.0)), false),
                (Point::new(Mm(10.0), Mm(10.0)), false),
                (Point::new(Mm(25.0), Mm(10.0)), false),
                (Point::new(Mm(25.0), Mm(210.0 - 24.0)), false)
            ],
            is_closed: true,
            has_fill: false,
            has_stroke: true,
            is_clipping_path: false,
        };
        
        let bisherige_lfd_nr_spalte = Line {
            points: vec![
                (Point::new(Mm(25.0), Mm(210.0 - 24.0)), false),
                (Point::new(Mm(25.0), Mm(10.0)), false),
                (Point::new(Mm(40.0), Mm(10.0)), false),
                (Point::new(Mm(40.0), Mm(210.0 - 24.0)), false)
            ],
            is_closed: true,
            has_fill: false,
            has_stroke: true,
            is_clipping_path: false,
        };
        
        let gemarkung_spalte = Line {
            points: vec![
                (Point::new(Mm(40.0), Mm(210.0 - 24.0)), false),
                (Point::new(Mm(40.0), Mm(10.0)), false),
                (Point::new(Mm(90.0), Mm(10.0)), false),
                (Point::new(Mm(90.0), Mm(210.0 - 24.0)), false)
            ],
            is_closed: true,
            has_fill: false,
            has_stroke: true,
            is_clipping_path: false,
        };
        
        let flur_spalte = Line {
            points: vec![
                (Point::new(Mm(90.0), Mm(210.0 - 24.0)), false),
                (Point::new(Mm(90.0), Mm(10.0)), false),
                (Point::new(Mm(105.0), Mm(10.0)), false),
                (Point::new(Mm(105.0), Mm(210.0 - 24.0)), false)
            ],
            is_closed: true,
            has_fill: false,
            has_stroke: true,
            is_clipping_path: false,
        };
        
        let flurstueck_spalte = Line {
            points: vec![
                (Point::new(Mm(105.0), Mm(210.0 - 24.0)), false),
                (Point::new(Mm(105.0), Mm(10.0)), false),
                (Point::new(Mm(125.0), Mm(10.0)), false),
                (Point::new(Mm(125.0), Mm(210.0 - 24.0)), false)
            ],
            is_closed: true,
            has_fill: false,
            has_stroke: true,
            is_clipping_path: false,
        };
        
        let wirtschaftsart_lage_spalte = Line {
            points: vec![
                (Point::new(Mm(125.0), Mm(210.0 - 24.0)), false),
                (Point::new(Mm(125.0), Mm(10.0)), false),
                (Point::new(Mm(297.0 - 45.0), Mm(10.0)), false),
                (Point::new(Mm(297.0 - 45.0), Mm(210.0 - 24.0)), false)
            ],
            is_closed: true,
            has_fill: false,
            has_stroke: true,
            is_clipping_path: false,
        };
        
        let ha_spalte = Line {
            points: vec![
                (Point::new(Mm(297.0 - 45.0), Mm(210.0 - 24.0)), false),
                (Point::new(Mm(297.0 - 45.0), Mm(10.0)), false),
                (Point::new(Mm(297.0 - 30.0), Mm(10.0)), false),
                (Point::new(Mm(297.0 - 30.0), Mm(210.0 - 24.0)), false)
            ],
            is_closed: true,
            has_fill: false,
            has_stroke: true,
            is_clipping_path: false,
        };
        
        let a_spalte = Line {
            points: vec![
                (Point::new(Mm(297.0 - 30.0), Mm(210.0 - 24.0)), false),
                (Point::new(Mm(297.0 - 30.0), Mm(10.0)), false),
                (Point::new(Mm(297.0 - 20.0), Mm(10.0)), false),
                (Point::new(Mm(297.0 - 20.0), Mm(210.0 - 24.0)), false)
            ],
            is_closed: true,
            has_fill: false,
            has_stroke: true,
            is_clipping_path: false,
        };
        
        let m2_spalte = Line {
            points: vec![
                (Point::new(Mm(297.0 - 20.0), Mm(210.0 - 24.0)), false),
                (Point::new(Mm(297.0 - 20.0), Mm(10.0)), false),
                (Point::new(Mm(297.0 - 10.0), Mm(10.0)), false),
                (Point::new(Mm(297.0 - 10.0), Mm(210.0 - 24.0)), false)
            ],
            is_closed: true,
            has_fill: false,
            has_stroke: true,
            is_clipping_path: false,
        };
        
        let text_1 = &["Laufende", "Nummer", "der", "Grund-", "stücke"];
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

        doc.get_page(current_page).get_layer(formular_layer)
        .use_text(&format!("Grundbuch von {grundbuch_von}  -  Blatt {blatt}"), 12.0, Mm(10.0), Mm(210.0 - 10.0), &times);        
        
        doc.get_page(current_page).get_layer(formular_layer)
        .use_text("Bestandsverzeichnis", 16.0, Mm(10.0), Mm(210.0 - 16.0), &times_bold);
        
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
        
        let mut start = Mm(210.0 - 29.0);
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
                    .use_text(&format!("{}", flst.lfd_nr), 12.0, Mm(12.0), start, &courier_bold);
                    
                    if let Some(bisherig) = flst.bisherige_lfd_nr.as_ref() {
                        text_layer
                        .use_text(&format!("{}", bisherig), 12.0, Mm(27.0), start, &courier_bold);                
                    }

                    let gemarkung = b.get_gemarkung().clone().unwrap_or_default();
                    let gemarkung = if gemarkung == grundbuch_von { String::new() } else { gemarkung };
            
                    if let Some(gemarkung) = flst.gemarkung.as_ref() {
                        if *gemarkung != grundbuch_von {
                            text_layer.use_text(gemarkung, 12.0, Mm(42.0), start, &courier_bold);  
                        }
                    }
                    
                    text_layer
                    .use_text(&format!("{}", flst.flur), 12.0, Mm(92.0), start, &courier_bold);
                    
                    text_layer
                    .use_text(&format!("{}", flst.flurstueck), 12.0, Mm(107.0), start, &courier_bold);
                    
                    let m2 = flst.groesse.get_m2();

                    text_layer
                    .use_text(&format!("{}", m2), 12.0, Mm(297.0 - 43.0), start, &courier_bold);
                    
                    if b.ist_geroetet() {    
                        rot_layer
                        .add_shape(Line {
                            points: vec![
                                (Point::new(Mm(12.0), start - Mm(1.0)), false),
                                (Point::new(Mm(297.0 - 12.0), start - Mm(1.0)), false)
                            ],
                            is_closed: false,
                            has_fill: false,
                            has_stroke: true,
                            is_clipping_path: false,
                        });
                    }
                    
                    start = start - Mm(6.0);   
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
    
        let extent_y_lfd_nr = 4.13;
        let extent_y_text = wordbreak_text(&abt2_eintrag.text, 50).lines().count() as f64 * 4.13;
        let extent_y_bv_nr = wordbreak_text(&clean_bv(&abt2_eintrag.bv_nr), 12).lines().count() as f64 * 4.13;
        let extent_y = extent_y_bv_nr
            .max(extent_y_text)
            .max(extent_y_lfd_nr);
        
        if cursor - extent_y < 12.0 || abt2_seiten.is_empty() {
            abt2_seiten.push(Vec::new());
            cursor = 297.0 - 29.0;
        }
        
        abt2_seiten.last_mut().unwrap().push(abt2_eintrag.clone());
        cursor -= extent_y;
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
            
            let bv_break = wordbreak_text(&clean_bv(&abt2_eintrag.bv_nr), 12);
            let text_break = wordbreak_text(&abt2_eintrag.text, 50);
            
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
            .use_text(&format!("{}", abt2_eintrag.lfd_nr), 12.0, Mm(12.0), start, &courier_bold);
        
            // write BV
            text_layer.begin_text_section();
            text_layer.set_font(&courier_bold, 12.0);
            text_layer.set_text_cursor(Mm(27.0), start);
            text_layer.set_line_height(12.0);
            for line in bv_break.lines() {
                text_layer.write_text(line.clone(), &courier_bold);
                text_layer.add_line_break();
            }
            text_layer.end_text_section();
    
            // write text
            text_layer.begin_text_section();
            text_layer.set_font(&courier_bold, 12.0);
            text_layer.set_text_cursor(Mm(62.0),start);
            text_layer.set_line_height(12.0);
            for line in text_break.lines() {
                text_layer.write_text(line.clone(), &courier_bold);
                text_layer.add_line_break();
            }
            text_layer.end_text_section();
            
            let extent_y_lfd_nr = 4.13;
            let extent_y_bv_nr = wordbreak_text(&clean_bv(&abt2_eintrag.bv_nr), 12).lines().count() as f64 * 4.13;
            let extent_y_text = wordbreak_text(&abt2_eintrag.text, 50).lines().count() as f64 * 4.13;
            let extent_y = extent_y_bv_nr
                .max(extent_y_text)
                .max(extent_y_lfd_nr);
                    
            if abt2_eintrag.ist_geroetet() {
                rot_layer
                .add_shape(Line {
                    points: vec![
                        (Point::new(Mm(12.0), start + Mm(4.13)), false),
                        (Point::new(Mm(210.0 - 12.0), start + Mm(4.13)), false),
                        (Point::new(Mm(12.0), start - Mm(extent_y)), false),
                        (Point::new(Mm(210.0) - Mm(12.0), start - Mm(extent_y)), false),
                    ],
                    is_closed: false,
                    has_fill: false,
                    has_stroke: true,
                    is_clipping_path: false,
                });
            }
            
            start -= Mm(extent_y);

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
    
    for abt3 in grundbuch.abt3.eintraege.chunks(29) {
    
    }
    
    doc.save_to_bytes().unwrap_or_default()
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

fn string_to_qr_codes(s: &str) -> Vec<Image> {

    let chunks = s.lines().collect::<Vec<_>>();
    
    chunks.chunks(5).map(|s| {
        let mut s = s.to_vec().join("\n");
        if s.len() < 180 { s.push_str(&"\n".repeat(180 - s.len())); }
        let string = QrCode::with_error_correction_level(s, EcLevel::H).unwrap()
            .render::<char>()
            .quiet_zone(false)
            .module_dimensions(2, 1)
            .build();
        
        let img: Vec<u8> = string.lines().flat_map(|row| {
            row.chars()
            .enumerate()
            .filter_map(|(i, c)| if i % 2 == 0 { None } else { Some(c) })
            .map(|c| if c == '█' { 0 } else { 255 })
            .collect::<Vec<_>>()
        }).collect();

        let img_height = (img.len() as f32).sqrt().round() as usize;        
                
        Image::from(ImageXObject {
            width: Px(img_height),
            height: Px(img_height),
            color_space: ColorSpace::Greyscale,
            bits_per_component: ColorBits::Bit8,
            interpolate: false,
            image_data: img,
            image_filter: None,
            clipping_bbox: None,
        })        
    }).collect()
}
