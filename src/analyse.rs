use crate::{Grundbuch, Titelblatt, Konfiguration};
use crate::digital::{Nebenbeteiligter, BvEintrag};
use crate::python::{Spalte1Eintrag, SchuldenArt, RechteArt};
use serde_derive::{Serialize, Deserialize};
use std::collections::BTreeMap;
use crate::get_or_insert_regex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrundbuchAnalysiert {
    pub abt2: Vec<Abt2Analysiert>,
    pub abt3: Vec<Abt3Analysiert>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Abt2Analysiert {
    pub lfd_nr: usize,
    pub text_kurz: String,
    pub rechteart: RechteArt,
    pub rechtsinhaber: String,
    pub rangvermerk: Option<String>,
    pub spalte_2: String,
    // Flur, Flurstück
    pub belastete_flurstuecke: Vec<BvEintrag>,
    #[serde(default)]
    pub lastend_an: Vec<Spalte1Eintrag>,
    pub text_original: String,
    pub nebenbeteiligter: Nebenbeteiligter,
    pub warnungen: Vec<String>,
    pub fehler: Vec<String>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Abt3Analysiert {
    pub lfd_nr: usize,
    pub text_kurz: String,
    pub betrag: Betrag,
    pub schuldenart: SchuldenArt,
    pub rechtsinhaber: String,
    pub spalte_2: String,
    // Flur, Flurstück
    pub belastete_flurstuecke: Vec<BvEintrag>,
    #[serde(default)]
    pub lastend_an: Vec<Spalte1Eintrag>,
    pub text_original: String,
    pub nebenbeteiligter: Nebenbeteiligter,
    pub warnungen: Vec<String>,
    pub fehler: Vec<String>,
}

pub fn analysiere_grundbuch(
    grundbuch: &Grundbuch, 
    nb: &[Nebenbeteiligter], 
    konfiguration: &Konfiguration
) -> GrundbuchAnalysiert {
    
    let mut abt2_analysiert = Vec::<Abt2Analysiert>::new();
    let mut abt3_analysiert = Vec::new();

    for eintrag in grundbuch.abt2.eintraege.iter() {
        
        if eintrag.ist_geroetet() { continue; }
        
        let mut warnungen = Vec::new();
        let mut fehler = Vec::new();
                    
        let mut eintrag_veraenderungen = Vec::new();
        let mut eintrag = eintrag.clone();
        
        for v in grundbuch.abt2.veraenderungen.iter() {
            
            let spalte_1_nummern = match parse_spalte_1_veraenderung(&v.lfd_nr.text()) {
                Ok(s) => s,
                Err(e) => {
                    fehler.push(format!("Konnte Abt. 2 Veränderung nicht lesen: {}: {}", v.lfd_nr.text(), e));
                    Vec::new()
                },
            };
            
            if spalte_1_nummern.contains(&eintrag.lfd_nr) {
                eintrag_veraenderungen.push(v.text.clone());
            }
        }
                
        // Veränderungen Abt. 2 einfügen (speziell Rangvermerke)
        if !eintrag_veraenderungen.is_empty() {
            for v in eintrag_veraenderungen.iter() {
                warnungen.push(format!("Veränderungsmitteilung beachten:<br/>{}", v.text()));
                if eintrag.text.contains("Rang") || eintrag.text.contains("Gleichrang") {
                    eintrag.text.push_str(" ");
                    eintrag.text.push_str(&v.text());
                    eintrag.text.push_str("\r\n");
                }
            }
        }
        
        let grundbuch_von = grundbuch.titelblatt.grundbuch_von.clone();
        let blatt = grundbuch.titelblatt.blatt.clone();
        let lfd_nr = eintrag.lfd_nr;
        let recht_id = format!("{grundbuch_von} Blatt {blatt} Abt. 2 lfd. Nr. {lfd_nr}");
        
        let kt = kurztext::text_kuerzen_abt2(&recht_id, &eintrag.text.text(), &mut fehler, konfiguration);
        let mut lastend_an = Vec::new();
        let mut debug_log = String::new();
        let belastete_flurstuecke = match Python::with_gil(|py| {
            get_belastete_flurstuecke(
                py,
                &eintrag.bv_nr.text(), 
                &kt.text_sauber, 
                &grundbuch.titelblatt,
                &grundbuch.bestandsverzeichnis.eintraege,
                konfiguration,
                &mut debug_log,
                &mut lastend_an,
                &mut warnungen,
                &mut fehler
            ) 
        }) {
            Ok(o) => o,
            Err(e) => {
                fehler.push(e);
                Vec::new()
            }
        };
        
        let mut rechteart = match kt.rechteart.clone() {
            Some(s) => s,
            None => {
                fehler.push(format!("Konnte Rechteart nicht auslesen"));
                RechteArt::SonstigeDabagrechteart
            }
        };
        
        let rechtsinhaber = match kt.rechtsinhaber.clone() {
            Some(s) => s,
            None => match rechteart.clone() {
                r if !r.benoetigt_rechteinhaber() => String::new(),
                RechteArt::SpeziellVormerkung { rechteverweis } => {
                    if let Some(recht) = abt2_analysiert.iter().find(|r| r.lfd_nr == rechteverweis).cloned() {
                        rechteart = recht.rechteart.clone();
                        recht.rechtsinhaber.clone()
                    } else {
                        fehler.push(format!("Konnte Rechtsinhaber nicht auslesen"));
                        String::new()
                    }
                },
                
                _ => {
                    fehler.push(format!("Konnte Rechtsinhaber nicht auslesen"));
                    String::new()
                }
            }
        };

        let rangvermerk = kt.rangvermerk.clone();
        
        let nebenbeteiligter = match nb.iter().find(|n| n.name == rechtsinhaber) {
            Some(s) => s.clone(),
            None => {
                if rechteart.benoetigt_rechteinhaber() {
                    warnungen.push(format!("Konnte keine Ordnungsnummer finden"));                
                }
                Nebenbeteiligter {
                    typ: None,
                    ordnungsnummer: None,
                    name: rechtsinhaber.clone(),
                    extra: NebenbeteiligterExtra::default(),
                }
            }
        };
        
        if rangvermerk.is_some() {
            if !(kt.gekuerzt.contains("Rang") || kt.gekuerzt.contains("Gleichrang")) {
                fehler.push(format!("Rangvermerk vorhanden, aber nicht in Kurztext vermerkt"));
            }
        }
        
        abt2_analysiert.push(Abt2Analysiert {
            lfd_nr: eintrag.lfd_nr,
            text_kurz: kt.gekuerzt,
            rechteart,
            rechtsinhaber,
            rangvermerk,
            spalte_2: eintrag.bv_nr.clone().text(),
            belastete_flurstuecke,
            lastend_an,
            text_original: kt.text_sauber,
            nebenbeteiligter,
            warnungen,
            fehler,
        })
    }
    
    for eintrag in grundbuch.abt3.eintraege.iter() {
    
        if eintrag.ist_geroetet() { continue; }

        let mut warnungen = Vec::new();
        let mut fehler = Vec::new();
                    
        let mut eintrag_veraenderungen = Vec::new();
        let mut eintrag = eintrag.clone();
        
        for v in grundbuch.abt3.veraenderungen.iter() {
            
            let spalte_1_nummern = match parse_spalte_1_veraenderung(&v.lfd_nr.text()) {
                Ok(s) => s,
                Err(e) => {
                    fehler.push(format!("Konnte Abt. 3 Veränderung nicht lesen: {}: {}", v.lfd_nr.text(), e));
                    Vec::new()
                },
            };
            
            if spalte_1_nummern.contains(&eintrag.lfd_nr) {
                eintrag_veraenderungen.push(v.text.clone());
            }
        }
                    
        // Veränderungen Abt. 2 einfügen (speziell Rangvermerke)
        if !eintrag_veraenderungen.is_empty() {
            warnungen.push(format!("Veränderungsmittelungen Abt.3 beachten!: {}", eintrag_veraenderungen.iter().map(|q| q.text()).collect::<Vec<_>>().join("\r\n")));
            for v in eintrag_veraenderungen.iter() {
                if eintrag.text.contains("Rang") || 
                    eintrag.text.contains("Gleichrang") || 
                    eintrag.text.contains("Mithaft") || 
                    eintrag.text.contains("Gesamthaft") {
                    
                    eintrag.text.push_str(" ");
                    eintrag.text.push_str(&v.text());
                    eintrag.text.push_str("\r\n");
                }
            }
        }

        let grundbuch_von = grundbuch.titelblatt.grundbuch_von.clone();
        let blatt = grundbuch.titelblatt.blatt.clone();
        let lfd_nr = eintrag.lfd_nr;
        let recht_id = format!("{grundbuch_von} Blatt {blatt} Abt. 3 lfd. Nr. {lfd_nr}");
        
        let kt = kurztext::text_kuerzen_abt3(&recht_id, &eintrag.betrag.text(), &eintrag.text.text(), &mut fehler, konfiguration);
        let mut lastend_an = Vec::new();
        let mut debug_log = String::new();
        let belastete_flurstuecke = match Python::with_gil(|py| {
            get_belastete_flurstuecke(
                py,
                &eintrag.bv_nr.text(), 
                &kt.text_sauber, 
                &grundbuch.titelblatt,
                &grundbuch.bestandsverzeichnis.eintraege,
                konfiguration,
                &mut debug_log,
                &mut lastend_an,
                &mut warnungen,
                &mut fehler
            ) 
        }) {
            Ok(o) => o,
            Err(e) => {
                fehler.push(e);
                Vec::new()
            }
        };
        
        let rechtsinhaber = match kt.rechtsinhaber.clone() {
            Some(s) => s,
            None => {
                fehler.push(format!("Konnte Rechtsinhaber nicht auslesen"));
                String::new()
            }
        };
        
        let schuldenart = match kt.schuldenart.clone() {
            Some(s) => s,
            None => {
                fehler.push(format!("Konnte Schuldenart nicht auslesen"));
                SchuldenArt::Grundschuld
            }
        };
        
        let nebenbeteiligter = match nb.iter().find(|n| n.name == rechtsinhaber) {
            Some(s) => s.clone(),
            None => {
                warnungen.push(format!("Konnte keine Ordnungsnummer finden"));
                Nebenbeteiligter {
                    typ: None,
                    ordnungsnummer: None,
                    name: rechtsinhaber.clone(),
                    extra: NebenbeteiligterExtra::default(),
                }
            }
        };
        
        abt3_analysiert.push(Abt3Analysiert {
            lfd_nr: eintrag.lfd_nr,
            text_kurz: kt.gekuerzt,
            schuldenart,
            rechtsinhaber,
            betrag: kt.betrag,
            spalte_2: eintrag.bv_nr.clone().text(),
            belastete_flurstuecke,
            lastend_an,
            text_original: kt.text_sauber,
            nebenbeteiligter,
            warnungen,
            fehler,
        });            
    }
    
    GrundbuchAnalysiert {
        abt2: abt2_analysiert,
        abt3: abt3_analysiert,
    }
}

pub fn get_belastete_flurstuecke(
    vm: PyVm,
	bv_nr: &str, 
	text_sauber: &str, 
	titelblatt: &Titelblatt,
	bestandsverzeichnis: &[BvEintrag],
	konfiguration: &Konfiguration,
	debug_log: &mut String,
	eintraege: &mut Vec<Spalte1Eintrag>,
	warnungen: &mut Vec<String>,
	fehler: &mut Vec<String>,
) -> Result<Vec<BvEintrag>, String> {

    let spalte1_eintraege = crate::python::get_belastete_flurstuecke(
        py,
        bv_nr,
        text_sauber,
        konfiguration,
        fehler,
    )?;
    
    eintraege.append(&mut spalte1_eintraege.clone());
    
    let grundbuch_von = titelblatt.grundbuch_von.clone();
    let blatt = titelblatt.blatt.clone();

    let mut log = Vec::new();
    
    log.push(format!("<strong>Recht:</strong>"));
    debug_log.push_str(&format!("Recht:\r\n"));
    log.push(format!("<p>{text_sauber}</p>"));
    debug_log.push_str(&format!("    {text_sauber}\r\n"));
    log.push(format!("<p>Spalte 1: {bv_nr}</p>"));
    debug_log.push_str(&format!("Spalte 1: {bv_nr:?}\r\n"));

    log.push(format!("<strong>Ausgewertet:</strong>"));
    debug_log.push_str(&format!("Ausgewertet:\r\n"));
    
    let s1_ohne_teilbelastung = spalte1_eintraege.iter()
        .filter_map(|s| if s.nur_lastend_an.is_empty() { 
            Some(format!("{}", s.lfd_nr)) 
        } else { None })
        .collect::<Vec<_>>();
        
    if !s1_ohne_teilbelastung.is_empty() {
        log.push(format!("<p>&nbsp;&nbsp;{}</p>", s1_ohne_teilbelastung.join(", ")));
        debug_log.push_str(&format!("  {}\r\n", s1_ohne_teilbelastung.join(", ")));
    }
    
    let s1_mit_teilbelastung = spalte1_eintraege.iter()
    .filter_map(|s| if s.nur_lastend_an.is_empty() { 
        None 
    } else {
        Some(
            s.nur_lastend_an
            .iter()
            .map(|nl| format!("{}: nur lastend an {}", s.lfd_nr, nl))
            .collect::<Vec<_>>()
        )
    }).collect::<Vec<_>>();
        
    for s1 in s1_mit_teilbelastung {
        for s in s1 {
            log.push(format!("<p>&nbsp;&nbsp;{}</p>,<br/>", s));
            debug_log.push_str(&format!("  {}\r\n", s));
        }
    }
    
    let mut belastet_bv = Vec::<BvEintrag>::new();
    let mut global_filter = Vec::new();

    let mut nur_lastend = BTreeMap::new();
    for s1 in spalte1_eintraege.iter() {
        for nl in s1.nur_lastend_an.iter() {
            nur_lastend.entry(s1.lfd_nr)
            .or_insert_with(|| Vec::new())
            .push((nl.gemarkung.clone().unwrap_or(grundbuch_von.clone()), nl.flur, nl.flurstueck.clone(), nl.gemarkung.is_some()));
        }
    }
    
    // Spalte 1 Einträge => Bestandsverzeichnis Einträge
    for s1 in spalte1_eintraege.iter() {
        
        // 0 = keine Einschränkung nach BV-Nr., später filtern
        if s1.lfd_nr == 0 {
            global_filter.push(s1.clone());
            continue; 
        }
        
        let mut alle_bv_eintraege = bestandsverzeichnis.iter()
            .filter(|bv| bv.get_lfd_nr() == s1.lfd_nr)
            .cloned()
            .collect::<Vec<BvEintrag>>();
                
        alle_bv_eintraege.retain(|bv| *bv != BvEintrag::neu(0));        
        belastet_bv.extend(alle_bv_eintraege.into_iter());
    }

    for bv in belastet_bv.iter_mut() {
        if let Some(nur_filter) = nur_lastend.get(&bv.get_lfd_nr()) {
            let bv_flur = bv.get_flur();
            let bv_flurstueck = bv.get_flurstueck();
            let bv_gemarkung = bv.get_gemarkung().unwrap_or(grundbuch_von.clone());
            if !(nur_filter.iter().any(|i| if i.3 { 
                    i.0 == bv_gemarkung.clone() && i.1 == bv_flur && i.2 == bv_flurstueck.clone() 
                } else { 
                    i.1 == bv_flur && i.2 == bv_flurstueck.clone() 
                }) || nur_filter.iter().any(|i| 
                if i.3 { i.0 == bv_gemarkung.clone() && i.1 == 0 && i.2 == bv_flurstueck.clone() } else { 
                    i.1 == 0 && i.2 == bv_flurstueck.clone()
                })) {
                *bv = BvEintrag::neu(0); // remove
            }
        }
    }
    
    belastet_bv.retain(|bv| *bv != BvEintrag::neu(0));  
    
    log.push(format!("<strong>BV-Einträge (ungefiltert):</strong>"));
    debug_log.push_str(&format!("BV-Einträge (ungefiltert):\r\n"));
    
    for bv in belastet_bv.iter() {
        log.push(format!("<p>[{}]: {} Fl. {} Flst. {}</p>", bv.get_lfd_nr(), 
        bv.get_gemarkung().unwrap_or(grundbuch_von.clone()), bv.get_flur(), bv.get_flurstueck()));
        debug_log.push_str(
            &format!("[{}]: {} Fl. {} Flst. {}\r\n", 
                bv.get_lfd_nr(), 
                bv.get_gemarkung().unwrap_or(grundbuch_von.clone()), 
                bv.get_flur(), 
                bv.get_flurstueck()
            )
        );
    }
        
    log.push(format!("<strong>&nbsp;&nbsp;Filter:</strong>"));
    debug_log.push_str(&format!("Filter:\r\n"));

    // Nur lastend an Flur X, Flurstück Y (-> keine BV-Nr.!)
    let mut global_nur_lastend = Vec::new();
    for s1 in global_filter.iter() {
        debug_log.push_str(&format!("{:#?}\r\n", s1));
        for nl in s1.nur_lastend_an.iter() {
            global_nur_lastend.push((nl.gemarkung.clone().unwrap_or(grundbuch_von.clone()), nl.flur, nl.flurstueck.clone(), nl.gemarkung.is_some()));
        }
    }
    
    if !global_nur_lastend.is_empty() {
        for bv in belastet_bv.iter_mut() {
            let bv_flur = bv.get_flur();
            let bv_flurstueck = bv.get_flurstueck();
            let bv_gemarkung = bv.get_gemarkung().unwrap_or(grundbuch_von.clone());
            if !(global_nur_lastend.iter().any(|i| if i.3 { 
                    i.0 == bv_gemarkung.clone() && i.1 == bv_flur && i.2 == bv_flurstueck.clone() 
                } else { 
                    i.1 == bv_flur && i.2 == bv_flurstueck.clone() 
                }) || global_nur_lastend.iter().any(|i| 
                if i.3 { i.0 == bv_gemarkung.clone() && i.1 == 0 && i.2 == bv_flurstueck.clone() } else { 
                    i.1 == 0 && i.2 == bv_flurstueck.clone()
                })) {
                *bv = BvEintrag::neu(0); // remove
            }
        }
    }

    belastet_bv.retain(|bv| *bv != BvEintrag::neu(0));        
    
    let regex_values = konfiguration.regex.values().cloned().collect::<Vec<_>>();
    let regex_matches = konfiguration.regex.iter().filter_map(|(k, v)| {
        if get_or_insert_regex(&regex_values, v).ok()?.matches(text_sauber) { Some(k.clone()) } else { None } 
    })
    .collect::<Vec<_>>();
    
    if belastet_bv.is_empty() {
        fehler.push(format!("Konnte keine Flurstücke zuordnen!"));
        log.push(format!("<strong>Regex:</strong>"));
        log.push(format!("<p>{}</p>", regex_matches.join(", ")));
        fehler.push(format!("<div style='flex-direction:row;max-width:600px;'>{}</div>", log.join("\r\n")));
    }
    
    debug_log.push_str(&format!("Regex:\r\n"));
    debug_log.push_str(&format!("{}\r\n", regex_matches.join(",\r\n ")));
        
    let belastet_bv = flurstuecke_fortfuehren(
        &belastet_bv, 
        titelblatt, 
        bestandsverzeichnis, 
        warnungen, 
        fehler
    );
    
    let belastet_bv = belastet_bv
        .into_iter()
        .filter(|bv| !bv.ist_geroetet())
        .collect::<Vec<BvEintrag>>();
    
    // deduplicate
    let mut belastet_bv_map = BTreeMap::<String, BvEintrag>::new();
    for bv in belastet_bv {
        belastet_bv_map.insert(format!("{}", bv), bv);
    }
    
    let belastet_bv = belastet_bv_map
        .into_iter()
        .map(|(k, v)| v)
        .collect::<Vec<_>>();
    
    Ok(belastet_bv)
}

// Flurstücke automatisch so weit wie möglich automatisch fortführen
fn flurstuecke_fortfuehren(
    bv_eintraege: &[BvEintrag],
    titelblatt: &Titelblatt,
    bestandsverzeichnis: &[BvEintrag],
    warnungen: &mut Vec<String>,
    fehler: &mut Vec<String>,
) -> Vec<BvEintrag> {

    let mut bv_belastet = bv_eintraege.to_vec();
    let mut alle_fortgefuehrt = false;
    while !alle_fortgefuehrt {
        
        alle_fortgefuehrt = true;

        let mut bv_zerlegt_belastet = Vec::new();
        
        for (bv_idx, bv) in bv_belastet.clone().iter().enumerate() {
            
            let mut fortgeführt_als = bestandsverzeichnis
                .iter()
                .filter(|b| {
                    !b.ist_geroetet() &&
                    b.get_gemarkung().unwrap_or(titelblatt.grundbuch_von.clone()) == bv.get_gemarkung().unwrap_or(titelblatt.grundbuch_von.clone()) &&
                    b.get_flur() == bv.get_flur() && 
                    b.get_flurstueck() == bv.get_flurstueck() &&
                    b.get_lfd_nr() > bv.get_lfd_nr()
                })
                .cloned()
                .collect::<Vec<_>>();
            
            fortgeführt_als.sort_by(|a, b| a.get_lfd_nr().cmp(&b.get_lfd_nr()));
            fortgeführt_als.dedup();
            
            if fortgeführt_als.is_empty() {
                continue;
            }
            
            // BV-Änderungen benutzen:
            
            // "Flurstück X ist zerlegt in die Flurstücke Y und Z"
            // => bv_zerlegt_belastet.insert();
            
            // "Nummer X als Y eingetragen"
            
            // "Aus lfd. Nr. X ein Flurstück verselbstständigt und als lfd. Nr. Y eingetragen"
            // gucken, ob irgendein Fl. / Flst. doppelt eingetragen ist unter einer höheren lfd. Nr.
            // wenn ja, höhere Nr. nehmen
            
            // Versuche, automatisch nach Flur / Flurstücksnummern zu matchen
            // let mut nicht_fortgeführt = Vec::new();
                
            if fortgeführt_als.len() == 1 {
                warnungen.push(format!("Flur {} Flst. {} wird automatisch fortgeführt von BV-Nr. {} auf BV-Nr. {}", 
                    fortgeführt_als[0].get_flur(),
                    fortgeführt_als[0].get_flurstueck(),
                    bv.get_lfd_nr(),
                    fortgeführt_als[0].get_lfd_nr(),
                ));
                alle_fortgefuehrt = false; // nochmal prüfen
                bv_belastet[bv_idx] = fortgeführt_als[0].clone(); // Fortführung ausführen
            } else if fortgeführt_als.len() == 2 {
                // TODO: Zerlegung?
                // if bv_analyse.contains(geteilt == )
                fehler.push(format!("BV-Nr. {} wurde fortgeführt, kann aber nicht eindeutig zugeordnet werden (Zerlegung?): Fortgeführt als eins von {:?}", 
                    bv.get_lfd_nr(),
                    fortgeführt_als.iter().map(|l| l.get_lfd_nr()).collect::<Vec<_>>(),
                ));
            } else {
                fehler.push(format!("BV-Nr. {} wurde fortgeführt, kann aber nicht eindeutig zugeordnet werden (Zerlegung?): Fortgeführt als eins von {:?}", 
                    bv.get_lfd_nr(),
                    fortgeführt_als.iter().map(|l| l.get_lfd_nr()).collect::<Vec<_>>(),
                ));
            }
        }
        
        bv_belastet.append(&mut bv_zerlegt_belastet);
    }
    
    bv_belastet
}

fn parse_spalte_1_veraenderung(spalte_1: &str) -> Result<Vec<usize>, String> {
    Ok(Vec::new()) // TODO
}
