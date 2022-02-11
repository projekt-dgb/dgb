use crate::{Grundbuch, Konfiguration};
use crate::digitalisiere::{Nebenbeteiligter, NebenbeteiligterExtra, BvEintrag, Bestandsverzeichnis};
use crate::kurztext::{self, SchuldenArt, RechteArt};
use serde_derive::{Serialize, Deserialize};
use std::collections::{BTreeMap, BTreeSet};

pub fn analysiere_grundbuch(grundbuch: &Grundbuch, nb: &[Nebenbeteiligter], konfiguration: &Konfiguration) -> GrundbuchAnalysiert {
    
    let mut abt2_analysiert = Vec::new();
    
    for eintrag in grundbuch.abt2.eintraege.iter() {
        
        if eintrag.ist_geroetet() { continue; }
        
        let mut warnungen = Vec::new();
        let mut fehler = Vec::new();
                    
        let mut eintrag_veraenderungen = Vec::new();
        let mut eintrag = eintrag.clone();
        
        for v in grundbuch.abt2.veraenderungen.iter() {
            
            let spalte_1_nummern = match parse_spalte_1(&v.lfd_nr) {
                Some(s) => s,
                None => {
                    fehler.push(format!("Konnte Abt. 2 Veränderung nicht lesen: {}", v.lfd_nr));
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
                warnungen.push(format!("Veränderungsmitteilung beachten:<br/>{}", v));
                if eintrag.text.contains("Rang") || eintrag.text.contains("Gleichrang") {
                    eintrag.text.push_str(" ");
                    eintrag.text.push_str(v);
                    eintrag.text.push_str("\r\n");
                }
            }
        }
        
        let kt = kurztext::text_kuerzen_abt2(&eintrag.text, &mut fehler, konfiguration);
        
        let belastete_flurstuecke = match get_belastete_flurstuecke(
            &eintrag.bv_nr, 
            &kt.text_sauber, 
            &grundbuch, 
            &mut warnungen
        ) {
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
        
        let rechteart = match kt.rechteart.clone() {
            Some(s) => s,
            None => {
                fehler.push(format!("Konnte Rechteart nicht auslesen"));
                RechteArt::SonstigeDabagrechteart
            }
        };

        let rangvermerk = kt.rangvermerk.clone();
        
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
        
        abt2_analysiert.push(Abt2Analysiert {
            lfd_nr: eintrag.lfd_nr,
            text_kurz: kt.gekuerzt,
            rechteart,
            rechtsinhaber,
            rangvermerk,
            spalte_2: eintrag.bv_nr.clone(),
            belastete_flurstuecke,
            text_original: kt.text_sauber,
            nebenbeteiligter,
            warnungen,
            fehler,
        })
    }
    
    let mut abt3_analysiert = Vec::new();
    for eintrag in grundbuch.abt3.eintraege.iter() {
    
        if eintrag.ist_geroetet() { continue; }

        let mut warnungen = Vec::new();
        let mut fehler = Vec::new();
                    
        let mut eintrag_veraenderungen = Vec::new();
        let mut eintrag = eintrag.clone();
        
        for v in grundbuch.abt3.veraenderungen.iter() {
            
            let spalte_1_nummern = match parse_spalte_1(&v.lfd_nr) {
                Some(s) => s,
                None => {
                    fehler.push(format!("Konnte Abt. 3 Veränderung nicht lesen: {}", v.lfd_nr));
                    Vec::new()
                },
            };
            
            if spalte_1_nummern.contains(&eintrag.lfd_nr) {
                eintrag_veraenderungen.push(v.text.clone());
            }
        }
                    
        // Veränderungen Abt. 2 einfügen (speziell Rangvermerke)
        if !eintrag_veraenderungen.is_empty() {
            warnungen.push(format!("Veränderungsmittelungen Abt.3 beachten!: {}", eintrag_veraenderungen.join("\r\n")));
            for v in eintrag_veraenderungen.iter() {
                if eintrag.text.contains("Rang") || 
                    eintrag.text.contains("Gleichrang") || 
                    eintrag.text.contains("Mithaft") || 
                    eintrag.text.contains("Gesamthaft") {
                    
                    eintrag.text.push_str(" ");
                    eintrag.text.push_str(v);
                    eintrag.text.push_str("\r\n");
                }
            }
        }

        let kt = kurztext::text_kuerzen_abt3(&eintrag.betrag, &eintrag.text, &mut fehler, konfiguration);

        let belastete_flurstuecke = match get_belastete_flurstuecke(
            &eintrag.bv_nr, 
            &kt.text_sauber, 
            &grundbuch, 
            &mut warnungen
        ) {
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
            spalte_2: eintrag.bv_nr.clone(),
            belastete_flurstuecke,
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
    pub text_original: String,
    pub nebenbeteiligter: Nebenbeteiligter,
    pub warnungen: Vec<String>,
    pub fehler: Vec<String>,
}

// Setzt das Feld "BvEintrag.automatisch_geroetet"
pub fn roete_bestandsverzeichnis_automatisch(bv: &mut Bestandsverzeichnis) {

    return; 
    /*
    let zu_ab = analysiere_bestandsverzeichnis_zu_ab(&bv);
    let analyse_bv = analysiere_bestandsverzeichnis(&bv, &zu_ab);

    let mut bv_eintraege_zu_roeten = zu_ab.flurstuecke_zu_roeten.clone();
    bv_eintraege_zu_roeten.append(&mut analyse_bv.flurstuecke_zu_roeten.clone());

    bv_eintraege_zu_roeten.sort();
    bv_eintraege_zu_roeten.dedup();
    
    for bv in bv.eintraege.iter_mut() {
        bv.automatisch_geroetet = false;
    }
    
    for bve_rot in bv_eintraege_zu_roeten {
        if let Some(b) = bv.eintraege.iter_mut().find(|bve| **bve == bve_rot) {
            b.automatisch_geroetet = true;
        }
    }*/
}

fn get_belastete_flurstuecke(
	bv_nr: &str, 
	text_sauber: &str, 
	grundbuch: &Grundbuch, 
	warnung: &mut Vec<String>
) -> Result<Vec<BvEintrag>, String> {
    
    let mut bv_split = bv_nr
        .split(" ")
        .flat_map(|s| s.split(",").map(|s| s.to_string()))
        .collect::<Vec<String>>();
    
    // "Teil von" löschen
    let mut teil_von_flst = get_teilbelastete_flst(&mut bv_split);

    if !teil_von_flst.is_empty() && 
       !text_sauber.contains("lastend an") && 
       !text_sauber.contains("lastend auf") {
        return Err(format!("Hat \"Teil von\" in der laufenden Nummer, aber kein \"nur lastend an\" im Text, bitte manuell korrigieren!"));
    }
    
    // Andere (voll belastete?) Ordnungsnummern auslesen
    //
    // NOTIZ: Manche Bearbeiter tragen Teilbelastungen nur im Text ein, 
    // andere tragen Belastungen 
    let bv_nr_voll_belastet = get_vollbelastete_flst(&bv_split, &bv_nr, &grundbuch)?;

    // Lese "nur lastend an ... " aus
    let mut nur_lastend_an_str = Vec::new();
    if text_sauber.contains("lastend an") || text_sauber.contains("lastend auf") {
        for c in NUR_LASTEND_AN_REGEX_1.captures_iter(&text_sauber) {
            if let Some(flurstueck) = c.get(1).map(|s| s.as_str().trim().to_string()) {
                nur_lastend_an_str.push(flurstueck);
            }
        }
        
        for c in NUR_LASTEND_AN_REGEX_2.captures_iter(&text_sauber) {
            if let Some(flurstueck) = c.get(1).map(|s| s.as_str().trim().to_string()) {
                nur_lastend_an_str.push(flurstueck);
            }
        }
        
        for c in NUR_LASTEND_AN_REGEX_3.captures_iter(&text_sauber) {
            if let Some(flurstueck) = c.get(1).map(|s| s.as_str().trim().to_string()) {
                nur_lastend_an_str.push(flurstueck);
            }
        }
                        
        for c in NUR_LASTEND_AN_REGEX_4.captures_iter(&text_sauber) {
            if let Some(flurstueck) = c.get(1).map(|s| s.as_str().trim().to_string()) {
                nur_lastend_an_str.push(flurstueck);
            }
        }
    }
                
    // Für jedes "nur lastend an ...", lese flurstücke aus
    let teilweise_lastend_an = parse_teilweise_lastend_an(&nur_lastend_an_str, warnung)?;
    
    // Überprüfe, ob die Teil-BV-Nr. im BV zu finden sind, und ob die BV-Nr. stimmt
    let mut bv_teilweise = Vec::new();
    for (flur, flurstueck) in teilweise_lastend_an.iter() {
        
        let bv_eintrag_gefunden = grundbuch.bestandsverzeichnis.eintraege
        .iter()
        .rev()
        .find(|bv_eintrag| { 
            bv_eintrag.get_flur() == *flur && 
            bv_eintrag.get_flurstueck() == *flurstueck 
        });
        
        let bv_eintrag = match bv_eintrag_gefunden {
            Some(s) => s.clone(),
            None => {
                return Err(format!("Flur {:?} Flurstück {:?} ist im Text referenziert, aber existiert nicht im Bestandsverzeichnis!", 
                    flur,
                    flurstueck,
                ));
            },
        };
        
        teil_von_flst.push(bv_eintrag.get_lfd_nr());        
        bv_teilweise.push(bv_eintrag);
    }
    
    // Jetzt die BV-Einträge für die voll belasteten BV-Einträge finden
    let mut bv_voll = Vec::new();
    for bv in bv_nr_voll_belastet {

        // Nicht die gesamte BV-Nr. belasten, wenn nur ein Flurstück teilweise belastet ist
        if teil_von_flst.contains(&bv) {
            continue;
        }
        
        let mut bv_eintrag_gefunden = grundbuch.bestandsverzeichnis.eintraege
        .iter()
        .rev()
        .filter(|bv_eintrag| bv_eintrag.get_lfd_nr() == bv || bv_eintrag.get_bisherige_lfd_nr() == Some(bv))
        .cloned()
        .collect::<Vec<_>>();
        
        if bv_eintrag_gefunden.is_empty() {
            return Err(format!("Voll belastete BV-Nr. {} existiert nicht im Bestandsverzeichnis: (gefunden in Spalte 2 = {:?})", 
                bv,
                bv_nr,
            ));
        } else {
            bv_voll.append(&mut bv_eintrag_gefunden);
        }
    }
    
    // Teilweise und ganz belastete ONr. zusammenführen
    bv_voll.append(&mut bv_teilweise);
    let mut bv_belastet = bv_voll;
    
    if bv_belastet.is_empty() {
        return Err(format!("Keine BV-Einträge gefunden (Spalte 2 = {:?})", bv_nr));
    }
    
    // prüfen, ob BV-Nr. nicht fortgeführt wurde
    let mut alle_fortgefuehrt = false;
    while !alle_fortgefuehrt {
        alle_fortgefuehrt = true;

        let mut bv_zerlegt_belastet = Vec::new();
        
        for (bv_idx, bv) in bv_belastet.clone().iter().enumerate() {
            
            let mut fortgeführt_als = grundbuch.bestandsverzeichnis.eintraege
                .iter()
                .filter(|b| {
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
                warnung.push(format!("Flur {} Flst. {} wird automatisch fortgeführt von BV-Nr. {} auf BV-Nr. {}", 
                    fortgeführt_als[0].get_flur(),
                    fortgeführt_als[0].get_flurstueck(),
                    bv.get_lfd_nr(),
                    fortgeführt_als[0].get_lfd_nr(),
                ));
                alle_fortgefuehrt = false; // nochmal prüfen
                bv_belastet[bv_idx] = fortgeführt_als[0].clone(); // Fortführung ausführen
            } else if fortgeführt_als.len() == 2 {
                // Zerlegung?
                
                // if bv_analyse.contains(geteilt == )

            } else {
                return Err(format!("BV-Nr. {} wurde fortgeführt, kann aber nicht eindeutig zugeordnet werden (Zerlegung?): Fortgeführt als eins von {:?}", 
                    bv.get_lfd_nr(),
                    fortgeführt_als.iter().map(|l| l.get_lfd_nr()).collect::<Vec<_>>(),
                ));
            }
        }
        
        bv_belastet.append(&mut bv_zerlegt_belastet);
    }
    
    // deduplicate found entries
    bv_belastet.retain(|bv| !bv.ist_geroetet());
    bv_belastet.sort_by(|a, b| a.get_lfd_nr().cmp(&b.get_lfd_nr()));
    bv_belastet.dedup();
    
    Ok(bv_belastet)
}

pub struct BestandsverzeichnisAnalyse {
    pub warnungen: Vec<String>,
    pub fehler: Vec<String>,
    // lfd. Nr., Flur, Flurstück
    pub flurstuecke_zu_roeten: Vec<BvEintrag>
}

fn analysiere_bestandsverzeichnis(bv: &Bestandsverzeichnis, bv_zu_ab: &BestandsverzeichnisZuAbAnalyse) -> BestandsverzeichnisAnalyse {
    
    let mut warnungen = Vec::new();
    let mut fehler = Vec::new();
    let mut flurstuecke_zu_roeten = Vec::new();

    // Prüfe, dass alle lfd. Nr. von 0 - MAX eingetragen sind
    let max_nr_bv = bv.eintraege.iter().map(|f| f.get_lfd_nr()).max().unwrap_or(1);
    let mut alle_eintraege = (1..max_nr_bv.max(1)).collect::<BTreeSet<_>>();
    for e in bv.eintraege.iter() {
        alle_eintraege.remove(&e.get_lfd_nr());
    }
    if !alle_eintraege.is_empty() {
        warnungen.push(format!("BV-Nummer(n) {:?} scheinen nicht zu existieren", alle_eintraege));
    }
    
    // Prüfe auf doppelt eingetragene Flurstücke - automatisch röten
    let mut flur_flurstuecke_reverse = BTreeMap::new();
    for e in bv.eintraege.iter() {
        if !e.ist_geroetet() {
            flur_flurstuecke_reverse
                .entry((e.get_flur(), e.get_flurstueck().clone()))
                .or_insert_with(|| Vec::new())
                .push(e.get_lfd_nr());
        }
    }
    
    for ((flur, flurstueck), lfd_nrn) in flur_flurstuecke_reverse.iter() {
        if lfd_nrn.len() > 1 {
            // Automatisch Flurstück mit kleinerer lfd. Nr. röten
            let letzte_lfd_nr = lfd_nrn.iter().max().unwrap();
            flurstuecke_zu_roeten.extend(bv.eintraege.iter().filter(|bve| {
                bve.get_flur() == *flur &&
                bve.get_flurstueck() == *flurstueck &&
                bve.get_lfd_nr() < *letzte_lfd_nr
            }).cloned());
        }
    }
    
    // Prüfe auf Erwähnungen von Flurstücken oder BV-Nummern
    // in BV-Änderungen, die im BV nicht existieren
    /*
    for (bv_nr, a) in bv_zu_ab.aenderungen() {
        match a {
            Irrelevant => { },
            Teilung { von_nr, nach_nr } => {
                
            },
            NeuEingetragen { von_nr, nach_nr } => {
            
            },
            Verselbstständigt { aus } => {
            
            },
            ÜbertragenNach { was } => {
                if let Some(flur_flurstueck) = 
            },
        }
    }*/

    BestandsverzeichnisAnalyse {
        warnungen,
        fehler,
        flurstuecke_zu_roeten,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BestandsverzeichnisZuAbAnalyse {
    pub warnungen: Vec<String>,
    pub fehler: Vec<String>,
    pub flurstuecke_zu_roeten: Vec<BvEintrag>,
    // Änderungen / Löschungen
    // indexiert nach BV-Nr.
    pub aenderungen: BTreeMap<usize, Vec<BvAenderung>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BvAenderung {
    // "Hier eingetragen am ..."
    Irrelevant,
    // "Flurstück {von_flst} zerlegt in Flurstücke {nach_1} und {nach_2}"
    Zerlegt { 
        von_flst: String, 
        nach_1_flst: String,
        nach_2_flst: String,
    },
    // "Nr. {original_bv_nr} neu eingetragen als Nr. {neu_bv_nr}"
    BvNrNeuEingetragen { 
        original_bv_nr: usize, 
        neu_bv_nr: usize 
    },
    // "X Flurstück(e) verselbstständigt aus Nr. {von} und eingetragen unter Nr. {nach}"
    Verselbstständigt { 
        von: usize, 
        nach: usize,
    },
    // "Nr. {bv_nr} übertragen (= gelöscht) nach Blatt X"
    BvNrÜbertragen { 
        bv_nr: usize,
    },
    // "Flurstück {bv_nr} der Flur {flur} übertragen (= gelöscht) nach Blatt X"
    FlurstückÜbertragen { 
        bv_nr: usize, 
        flur: usize, 
        flurstueck: String 
    },
}

fn parse_spalte_1(spalte_1: &str) -> Option<Vec<usize>>  {
    
    let bv_nr = spalte_1
        .trim()
        .split(" ")
        .flat_map(|s| s.trim().split(",").map(|s| s.to_string()))
        .collect::<Vec<String>>();
        
    let mut bv_nr_voll_belastet = Vec::new();
    
    for bv in bv_nr {
        if bv.trim().is_empty() { continue; }
        if VON_BIS_REGEX.is_match(&bv) {
            let mut split = bv.split("-");
            
            let start = split.next().and_then(|s| s.parse::<usize>().ok())?;
            let end = split.next().and_then(|s| s.parse::<usize>().ok())?;
            
            let min = start.min(end);
            let max = start.max(end);
            
            for i in min..max {
                bv_nr_voll_belastet.push(i);
            }
        
            continue;
        } 
        
        if let Some(i) = bv.trim().parse::<usize>().ok() {
            bv_nr_voll_belastet.push(i);
        } else {
            return None;
        }
    }
    
    if bv_nr_voll_belastet.is_empty() {
        None
    } else {
        Some(bv_nr_voll_belastet)            
    }
}

fn get_teilbelastete_flst(bv_nr: &mut Vec<String>) -> Vec<usize> {
    
    bv_nr.retain(|b| !b.trim().is_empty());
    
    let mut teil_von_flst = Vec::new();

    let mut to_remove = Vec::new();
    let mut i = 0;
    for _ in 0..bv_nr.len() {
        if i >= bv_nr.len() { break; }
        if bv_nr[i] == "Teil" && bv_nr.get(i + 1).map(|s| s.as_str()) == Some("von") {
            if let Some(next) = bv_nr.get(i + 2).and_then(|s| s.parse::<usize>().ok()) {
                teil_von_flst.push(next);
                to_remove.push(next);
                i += 2;
            }
        }
        i += 1;
    }
    
    for q in to_remove {
        bv_nr.retain(|b| b != "Teil" && b != "von" && *b != q.to_string());
    }
    
    teil_von_flst
}

fn get_vollbelastete_flst(bv_split: &Vec<String>, bv_nr: &str, grundbuch: &Grundbuch) -> Result<Vec<usize>, String> {
     
    let mut bv_nr_voll_belastet = Vec::new();
    
    for bv in bv_split.iter() {
        
        let bv = bv.trim();
        if bv.is_empty() {
            continue;
        }
        
        if let Ok(o) = bv.parse::<usize>() {
            bv_nr_voll_belastet.push(o);
            continue;
        }
        
        if VON_BIS_REGEX.is_match(bv) {
            let mut split = bv.split("-");
            let start = match split.next().and_then(|s| s.parse::<usize>().ok()) {
                Some(s) => s,
                None => {
                    return Err(format!("Unlesbare Spalte 1: {:?} ist keine Zahl in \"{}\"", 
                        bv,
                        bv_nr,
                    ));
                },
            };
            
            let end = match split.next().and_then(|s| s.parse::<usize>().ok()) {
                Some(s) => s,
                None => {
                    return Err(format!("Unlesbare Spalte 1: {:?} ist keine Zahl in \"{}\"", 
                        bv,
                        bv_nr,
                    ));
                },
            };
            
            let min = start.min(end);
            let max = start.max(end);
            for i in min..max {
                bv_nr_voll_belastet.push(i);
            }
            continue;
        }
        
        return Err(format!("Unlesbare Spalte 1: {:?} ist keine Zahl in \"{}\"", 
            bv,
            bv_nr,
        ));
    }
    
    Ok(bv_nr_voll_belastet)
}

// parse: "Flur X, Flurstück Y" => (X, Y)
fn parse_teilweise_lastend_an(
    nur_lastend_an_str: &[String], 
    warnung: &mut Vec<String>
) -> Result<Vec<(usize, String)>, String> {
    
    let mut teilweise_lastend_an = Vec::new();
    
    for s in nur_lastend_an_str {
    
        let mut s = s.clone();
        
        let mut captured = false;
        let mut nur_bezogen_auf = None;
        if let Some(bzgl) = FLUR_FLURSTUECK_BZGL_REGEX.captures_iter(&s).nth(0) {
            let bzgl_1 = bzgl.get(1).unwrap().as_str().trim().to_string();
            match bzgl_1.parse::<usize>() {
                Ok(s) => nur_bezogen_auf = Some(s),
                Err(_) => { return Err(format!("\"Nur lastend an (bzgl. ...)\" in Text, konnte Bezug nicht lesen: \"{}\"", bzgl_1)); },
            }
            s = FLUR_FLURSTUECK_BZGL_REGEX.replace_all(&s, "").trim().to_string();
        }
        
        if s.contains("und") {

            let mut captures = FLUR_FLURSTUECK_REGEX_6.captures_iter(&s).collect::<Vec<_>>();                    
            let mut captures_7 = FLUR_FLURSTUECK_REGEX_7.captures_iter(&s).collect::<Vec<_>>();     
            captures.append(&mut captures_7);
            
            if captures.is_empty() {
                return Err(format!("\"nur lastend an\" im Text, aber kein Fl./Flst.: {:?}", s));
            }

            for c in captures {
                                    
                let flur = match c.get(1).map(|p| p.as_str().trim().parse::<usize>()) {
                    Some(Ok(o)) => o,
                    e => {
                        return Err(format!("Konnte Flur nicht aus Text auslesen: {:?} => Flur {:?}", s, e));
                    }
                };
                
                let flurstueck_1 = match c.get(2).map(|p| p.as_str().trim()) {
                    Some(o) => o.to_string(),
                    e => {
                        return Err(format!("Konnte Flurstück nicht aus Text auslesen: {:?} => Flur {:?}", s, e));
                    }
                };
                
                let flurstueck_2 = match c.get(3).map(|p| p.as_str().trim()) {
                    Some(o) => o.to_string(),
                    e => {
                        return Err(format!("Konnte Flurstück nicht aus Text auslesen: {:?} => Flur {:?}", s, e));
                    }
                };
                

                teilweise_lastend_an.push((flur, flurstueck_1));
                teilweise_lastend_an.push((flur, flurstueck_2));
            }
            
            captured = true;
        } 
        
        // Flur und Flurstück verkehrt
        let captures_4 = FLUR_FLURSTUECK_REGEX_4.captures_iter(&s).collect::<Vec<_>>();
        if !captures_4.is_empty() {
            captured = true;
        }
        
        for c in captures_4 {
                                
            let flur = match c.get(2).map(|p| p.as_str().trim().parse::<usize>()) {
                Some(Ok(o)) => o,
                e => {
                    return Err(format!("Konnte Flur nicht aus Text auslesen: {:?} => Flur {:?}", s, e));
                }
            };
            
            let flurstueck = match c.get(1).map(|p| p.as_str().trim()) {
                Some(o) => o.to_string(),
                e => {
                    return Err(format!("Konnte Flurstück nicht aus Text auslesen: {:?} => Flur {:?}", s, e));
                }
            };
                                
            teilweise_lastend_an.push((flur, flurstueck));
        }
        
        let mut captures = FLUR_FLURSTUECK_REGEX_1.captures_iter(&s).collect::<Vec<_>>();
        let mut captures_2 = FLUR_FLURSTUECK_REGEX_2.captures_iter(&s).collect::<Vec<_>>();
        captures.append(&mut captures_2);
        let mut captures_3 = FLUR_FLURSTUECK_REGEX_3.captures_iter(&s).collect::<Vec<_>>();
        captures.append(&mut captures_3);
        let mut captures_5 = FLUR_FLURSTUECK_REGEX_5.captures_iter(&s).collect::<Vec<_>>();
        captures.append(&mut captures_5);    
        
        if !captures.is_empty() {
            captured = true;
        }
        
        for c in captures {
                                
            let flur = match c.get(1).map(|p| p.as_str().trim().parse::<usize>()) {
                Some(Ok(o)) => o,
                e => {
                    return Err(format!("Konnte Flur nicht aus Text auslesen: {:?} => Flur {:?}", s, e));
                }
            };
            
            let flurstueck = match c.get(2).map(|p| p.as_str().trim()) {
                Some(o) => o.to_string(),
                e => {
                    return Err(format!("Konnte Flurstück nicht aus Text auslesen: {:?} => Flur {:?}", s, e));
                }
            };
                                
            teilweise_lastend_an.push((flur, flurstueck));
        }
    
        if LASTEND_AN_ANTEIL.is_match(&s) {
            warnung.push(format!("Miteigentumsrecht: nur lastend an {}", s));
            captured = true;
        }
        
        if !captured {
            return Err(format!("\"nur lastend an\" im Text, passt auf keine Regex: {:?}", s));
        }
    }
    
    Ok(teilweise_lastend_an)
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
pub enum Waehrung { 
    Euro,
    DMark,
    MarkDDR,
    Goldmark,
    Rentenmark,
    Reichsmark,
    GrammFeingold,
}

impl Waehrung {
    pub fn to_string(&self) -> &'static str {
        match self {
            Waehrung::Euro => "€",
            Waehrung::DMark => "DM",
            Waehrung::MarkDDR => "DDR-Mark",
            Waehrung::Goldmark => "Goldmark",
            Waehrung::Reichsmark => "Reichsmark",
            Waehrung::Rentenmark => "Rentenmark",
            Waehrung::GrammFeingold => "Gr. Feingold",
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
pub struct Betrag {
    pub wert: usize,
    pub nachkomma: usize,
    pub waehrung: Waehrung,
}

use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {

    // Nur lastend an Flur 2, Flurstück XX: ...
    static ref NUR_LASTEND_AN_REGEX_1: Regex = Regex::new(r"Nur lastend an (.*):").unwrap();
    static ref NUR_LASTEND_AN_REGEX_2: Regex = Regex::new(r"Nur lastend auf (.*):").unwrap();

    // ...) nur lastend an Flur 2, Flurstück XX für ...
    static ref NUR_LASTEND_AN_REGEX_3: Regex = Regex::new(r"nur lastend an (.*) für").unwrap();
    static ref NUR_LASTEND_AN_REGEX_4: Regex = Regex::new(r"nur lastend auf (.*) für").unwrap();
    
    // lastend an dem Anteil Abt. I Nr. XX - Teilrecht
    static ref LASTEND_AN_ANTEIL: Regex = Regex::new(r"dem Anteil Abt. I Nr. (.*)").unwrap();

    static ref FLUR_FLURSTUECK_BZGL_REGEX: Regex = Regex::new(r"\((.*)BV-Nr.(.*)\)").unwrap();

    // ... Flur X Flurstück Y ...
    static ref FLUR_FLURSTUECK_REGEX_1: Regex = Regex::new(r"Flur (\d*) Flurstück (\S*)").unwrap();
        // ... Flur 1 Flst. 293... 
    static ref FLUR_FLURSTUECK_REGEX_2: Regex = Regex::new(r"Flur (\d*) Flst. (\S*)").unwrap();
    // Flur 1, Flst. 293
    static ref FLUR_FLURSTUECK_REGEX_3: Regex = Regex::new(r"Flur (\d*), Flst. (\S*)").unwrap();
    // Flurstück 140 der Flur 2 
    static ref FLUR_FLURSTUECK_REGEX_4: Regex = Regex::new(r"Flurstück (\S*) der Flur (\d*)").unwrap();
    static ref FLUR_FLURSTUECK_REGEX_5: Regex = Regex::new(r"Flur (\d*), Flurstück (\S*)").unwrap();
    static ref FLUR_FLURSTUECK_REGEX_6: Regex = Regex::new(r"Flur (\d*), Flurstück (\S*) und (\S*)").unwrap();
    static ref FLUR_FLURSTUECK_REGEX_7: Regex = Regex::new(r"Flur (\d*), Flst. (\S*) und (\S*)").unwrap();

    // "10-48"
    static ref VON_BIS_REGEX: Regex = Regex::new(r"(\d*)-(\d*)").unwrap();
}
