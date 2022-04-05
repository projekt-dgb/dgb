use crate::{Grundbuch, Abt2Eintrag, Abt3Eintrag};
use crate::digitalisiere::BvEintrag;
use printpdf::{
    BuiltinFont, PdfDocument, Mm,
    Line, Point, Color, Cmyk
};

pub fn generate_grundbuch_pdf(grundbuch: &Grundbuch) -> Vec<u8> {
    
    let grundbuch_von = grundbuch.titelblatt.grundbuch_von.clone();
    let blatt =  grundbuch.titelblatt.blatt;
    
    let titel = format!("{grundbuch_von} Blatt {blatt} (Amtsgericht {})", grundbuch.titelblatt.amtsgericht);
    let (doc, page1, layer1) = PdfDocument::new(&titel, Mm(210.0), Mm(297.0), "Titelblatt");

    let gb = format!("Grundbuch von {grundbuch_von}");
    let blatt_nr = format!("Blatt {blatt}");
    let amtsgericht = format!("Amtsgericht {}", grundbuch.titelblatt.amtsgericht);

    let times_bold = doc.add_builtin_font(BuiltinFont::TimesBoldItalic).unwrap();
    let times = doc.add_builtin_font(BuiltinFont::TimesItalic).unwrap();
    let courier_bold = doc.add_builtin_font(BuiltinFont::CourierBold).unwrap();
    let helvetica = doc.add_builtin_font(BuiltinFont::HelveticaBold).unwrap();
    
    // text, font size, x from left edge, y from bottom edge, font
    let current_layer = doc.get_page(page1).get_layer(layer1);
    let start = Mm(297.0 / 2.0);
    let rand_x = Mm(25.0);
    current_layer.use_text(&gb, 22.0, Mm(25.0), start, &times_bold);
    
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
        
    current_layer.use_text(&blatt_nr, 16.0, Mm(25.0), start - Mm(12.0), &times);
    current_layer.use_text(&amtsgericht, 16.0, Mm(25.0), start - Mm(18.0), &times);
    
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
