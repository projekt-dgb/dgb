use crate::{Grundbuch, Konfiguration};
use crate::digitalisiere::{Nebenbeteiligter, NebenbeteiligterExtra, BvEintrag, Bestandsverzeichnis};
use crate::kurztext::{self, SchuldenArt, RechteArt};
use serde_derive::{Serialize, Deserialize};
use std::collections::{BTreeMap, BTreeSet};
use pyo3::{Python, pyclass, pymethods};
use crate::get_or_insert_regex;
use std::fmt;

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
            Waehrung::MarkDDR => "M",
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[pyclass(name = "Spalte1Eintrag")]
#[repr(C)]
pub struct Spalte1Eintrag {
    // Nummer im BV
    pub lfd_nr: usize,
    // "Teil von", "Teil v.", "X tlw."
    pub voll_belastet: bool,    
    // Leer = gesamte lfd. Nr. ist belastet
    pub nur_lastend_an: Vec<FlurFlurstueck>,
}

#[allow(non_snake_case)]
#[pymethods]
impl Spalte1Eintrag {
    #[new]
    #[args(voll_belastet = "true", nur_lastend_an = "Vec::new()")]
    fn new(lfd_nr: usize, voll_belastet: bool, nur_lastend_an: Vec<FlurFlurstueck>) -> Self {
        Self {
            lfd_nr,
            voll_belastet,
            nur_lastend_an,
        }
    }
    
    fn get_lfd_nr(&self) -> usize{
        self.lfd_nr
    }
    
    fn append_nur_lastend_an(&mut self, mut nur_lastend_an: Vec<FlurFlurstueck>) {
        self.voll_belastet = false;
        self.nur_lastend_an.append(&mut nur_lastend_an);
    }
    
    fn __str__(&self) -> String {
        format!("{:#?}", self)
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[pyclass(name = "FlurFlurstueck")]
#[repr(C)]
pub struct FlurFlurstueck {
    pub flur: usize,
    pub flurstueck: String,
    pub gemarkung: Option<String>,
    pub teilflaeche_qm: Option<usize>,
}

impl fmt::Display for FlurFlurstueck {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(s) = self.gemarkung.as_ref() {
            write!(f, "Gemarkung {}, ", s)?;
        }
        write!(f, "Flur {} Flst. {}", self.flur, self.flurstueck)
    }
}

#[allow(non_snake_case)]
#[pymethods]
impl FlurFlurstueck {
    #[new]
    #[args(gemarkung = "None", teilflaeche_qm = "None")]
    fn new(flur: usize, flurstueck: String, gemarkung: Option<String>, teilflaeche_qm: Option<usize>) -> Self {
        Self {
            gemarkung,
            flur,
            flurstueck,
            teilflaeche_qm,
        }
    }
}

pub fn analysiere_grundbuch<'py>(
    grundbuch: &Grundbuch, 
    nb: &[Nebenbeteiligter], 
    konfiguration: &Konfiguration
) -> GrundbuchAnalysiert {
    
    let mut abt2_analysiert = Vec::<Abt2Analysiert>::new();
    
    for eintrag in grundbuch.abt2.eintraege.iter() {
        
        if eintrag.ist_geroetet() { continue; }
        
        let mut warnungen = Vec::new();
        let mut fehler = Vec::new();
                    
        let mut eintrag_veraenderungen = Vec::new();
        let mut eintrag = eintrag.clone();
        
        for v in grundbuch.abt2.veraenderungen.iter() {
            
            let spalte_1_nummern = match parse_spalte_1_veraenderung(&v.lfd_nr) {
                Ok(s) => s,
                Err(e) => {
                    fehler.push(format!("Konnte Abt. 2 Veränderung nicht lesen: {}: {}", v.lfd_nr, e));
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
        
        let belastete_flurstuecke = match Python::with_gil(|py| {
            get_belastete_flurstuecke(
                py,
                &eintrag.bv_nr, 
                &kt.text_sauber, 
                &grundbuch,
                konfiguration,
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
                | RechteArt::VerausserungsBelastungsverbot
                | RechteArt::Auflassungsvormerkung
                => String::new(),
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
                warnungen.push(format!("Konnte keine Ordnungsnummer finden"));
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
            
            let spalte_1_nummern = match parse_spalte_1_veraenderung(&v.lfd_nr) {
                Ok(s) => s,
                Err(e) => {
                    fehler.push(format!("Konnte Abt. 3 Veränderung nicht lesen: {}: {}", v.lfd_nr, e));
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

        let belastete_flurstuecke = match Python::with_gil(|py| {
            get_belastete_flurstuecke(
                py,
                &eintrag.bv_nr, 
                &kt.text_sauber, 
                &grundbuch,
                konfiguration,
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

fn get_belastete_flurstuecke<'py>(
    py: Python<'py>,
	bv_nr: &str, 
	text_sauber: &str, 
	grundbuch: &Grundbuch,
	konfiguration: &Konfiguration,
	fehler: &mut Vec<String>,
) -> Result<Vec<BvEintrag>, String> {

    let spalte1_eintraege = get_belastete_flurstuecke_python(
        py,
        bv_nr,
        text_sauber,
        grundbuch,
        konfiguration,
        fehler,
    )?;
    
    let grundbuch_von = grundbuch.titelblatt.grundbuch_von.clone();
    let blatt = grundbuch.titelblatt.blatt.clone();

    let mut log = Vec::new();
    
    log.push(format!("<strong>Recht:</strong>"));
    log.push(format!("<p>{text_sauber}</p>"));
    log.push(format!("<p>Spalte 1: {bv_nr}</p>"));
    
    log.push(format!("<strong>Ausgewertet:</strong>"));
    let s1_ohne_teilbelastung = spalte1_eintraege.iter()
        .filter_map(|s| if s.nur_lastend_an.is_empty() { 
            Some(format!("{}", s.lfd_nr)) 
        } else { None })
        .collect::<Vec<_>>();
        
    if !s1_ohne_teilbelastung.is_empty() {
        log.push(format!("<p>&nbsp;&nbsp;{}</p>", s1_ohne_teilbelastung.join(", ")));
    }
    
    let s1_mit_teilbelastung = spalte1_eintraege.iter()
    .filter_map(|s| if s.nur_lastend_an.is_empty() { None } else {
        Some(format!("<p>&nbsp;&nbsp;{}: nur lastend an {}</p>", s.lfd_nr, s.nur_lastend_an.iter().map(|nl| format!("{}", nl)).collect::<Vec<_>>().join(",<br/>")))
    }).collect::<Vec<_>>();
        
    for s1 in s1_mit_teilbelastung {
        log.push(s1);
    }
    
    let mut belastet_bv = Vec::<BvEintrag>::new();
    let mut global_filter = Vec::new();
    
    let mut bv_keep = BTreeMap::new();

    // Spalte 1 Einträge => Bestandsverzeichnis Einträge
    for s1 in spalte1_eintraege.iter() {
        
        // 0 = keine Einschränkung nach BV-Nr., später filtern
        if s1.lfd_nr == 0 {
            global_filter.push(s1.clone());
            continue; 
        }
        
        let mut alle_bv_eintraege = grundbuch.bestandsverzeichnis.eintraege.iter()
            .filter(|bv| bv.get_lfd_nr() == s1.lfd_nr)
            .cloned()
            .collect::<Vec<BvEintrag>>();
                
        for nl in s1.nur_lastend_an.iter() {
        
            let gemarkung_filter = match nl.gemarkung.clone() {
                Some(s) if s == grundbuch_von => None,
                o => o,
            };
                        
            // Filter nach Gemarkung / Flur / Flurstück
            for (i, bv)  in belastet_bv.iter().enumerate() {
                
                // Flur = 0 = kein Filter nach Flur vorhanden
                if nl.flur != 0 {
                    if nl.flur != bv.get_flur() {
                        continue; 
                    }
                }
                
                if bv.get_flurstueck() != nl.flurstueck {
                    continue;
                }
                
                let should_remove = match (bv.get_gemarkung(), gemarkung_filter.clone()) {
                    (None, None) => false,
                    (Some(s), None) => s != grundbuch_von,
                    (None, Some(s)) => s != grundbuch_von,
                    (Some(s1), Some(s2)) => s1 != s2,
                    _ => true,
                };
                
                if !should_remove  {
                    bv_keep
                    .entry(s1.lfd_nr)
                    .or_insert_with(|| Vec::new())
                    .push(i);
                }
            }
        }
                
        alle_bv_eintraege.retain(|bv| *bv != BvEintrag::neu(0));        
        belastet_bv.extend(alle_bv_eintraege.into_iter());
    }
    
    for (i, bv) in belastet_bv.iter_mut().enumerate() {
        
        let should_keep = match bv_keep.get(&bv.get_lfd_nr()) {
            Some(s) => s.contains(&i),
            None => true,
        };
        
        if !should_keep {
            *bv = BvEintrag::neu(0);
        }
    }
    
    belastet_bv.retain(|bv| *bv != BvEintrag::neu(0));  
    
    log.push(format!("<strong>BV-Einträge (ungefiltert):</strong>"));
    for bv in belastet_bv.iter() {
        log.push(format!("<p>[{}]: {} Fl. {} Flst. {}</p>", bv.get_lfd_nr(), bv.get_gemarkung().unwrap_or(grundbuch_von.clone()), bv.get_flur(), bv.get_flurstueck()));
    }
    
    let mut bv_keep = BTreeMap::new();
    
    log.push(format!("<strong>&nbsp;&nbsp;Global Filter</strong>"));

    // Nur lastend an Flur X, Flurstück Y (-> keine BV-Nr.!)
    for s1 in global_filter {
        
        for nl in s1.nur_lastend_an.iter() {
        
            let gemarkung_filter = match nl.gemarkung.clone() {
                Some(s) if s == grundbuch_von => None,
                o => o,
            };
            let flur = nl.flur;
            let flurstueck = &nl.flurstueck;

            // Filter nach Gemarkung / Flur / Flurstück
            for (i, bv)  in belastet_bv.iter().enumerate() {
                
                // Flur = 0 = kein Filter nach Flur vorhanden
                if nl.flur != 0 {
                    if nl.flur != bv.get_flur() {
                        continue; 
                    }
                }
                
                
                if bv.get_flurstueck() != nl.flurstueck {
                    continue;
                }
                
                let should_remove = match (bv.get_gemarkung(), gemarkung_filter.clone()) {
                    (None, None) => false,
                    (Some(s), None) => s != grundbuch_von,
                    (None, Some(s)) => s != grundbuch_von,
                    (Some(s1), Some(s2)) => s1 != s2,
                    _ => true,
                };
                
                if !should_remove  {
                    bv_keep
                    .entry(s1.lfd_nr)
                    .or_insert_with(|| Vec::new())
                    .push(i);
                }
            }
        }
    }
        
    for (i, bv) in belastet_bv.iter_mut().enumerate() {
        
        let should_keep = match bv_keep.get(&0) {
            Some(s) => s.contains(&i),
            None => true,
        };
        
        if !should_keep {
            *bv = BvEintrag::neu(0);
        }
    }
    
    belastet_bv.retain(|bv| *bv != BvEintrag::neu(0));        
    
    let regex_values = konfiguration.regex.values().cloned().collect::<Vec<_>>();
    
    if belastet_bv.is_empty() {
        let regex_matches = konfiguration.regex.iter().filter_map(|(k, v)| {
            if get_or_insert_regex(&regex_values, v).ok()?.matches(text_sauber) { Some(k.clone()) } else { None } 
        })
        .collect::<Vec<_>>();
        fehler.push(format!("Konnte keine Flurstücke zuordnen!"));
        log.push(format!("<strong>Regex:</strong>"));
        log.push(format!("<p>{}</p>", regex_matches.join(", ")));
        fehler.push(format!("<div style='flex-direction:row;max-width:600px;'>{}</div>", log.join("\r\n")));
    }
        
    Ok(belastet_bv)
}

fn get_belastete_flurstuecke_python<'py>(
    py: Python<'py>,
	bv_nr: &str, 
	text_sauber: &str, 
	grundbuch: &Grundbuch,
	konfiguration: &Konfiguration,
	fehler: &mut Vec<String>
) -> Result<Vec<Spalte1Eintrag>, String> {
    
    use pyo3::prelude::*;
    use pyo3::types::{PyDict, PyList, PyTuple};
        
    let script = konfiguration
        .flurstuecke_auslesen_script
        .iter()
        .map(|l| format!("    {}", l))
        .collect::<Vec<_>>()
        .join("\r\n");
        
    let script = script.replace("\t", "    ");
    let script = script.replace("\u{00a0}", " ");
    let py_code = format!("import inspect\r\n\r\ndef run_script(*args, **kwargs):\r\n    spalte_1, text, re = args\r\n{}", script);
    let regex_values = konfiguration.regex.values().cloned().collect::<Vec<_>>();
    
    let module = PyModule::from_code(py, &py_code, "", "main").map_err(|e| format!("{}", e))?;
    module.add_class::<Spalte1Eintrag>().map_err(|e| format!("{}", e))?;
    module.add_class::<FlurFlurstueck>().map_err(|e| format!("{}", e))?;

    let fun: Py<PyAny> = module.getattr("run_script").unwrap().into();
    let regex_list = {
        let dict = PyDict::new(py);
        for (k, v) in konfiguration.regex.iter() {
            if let Ok(v) = get_or_insert_regex(&regex_values, v) {
                let _ = dict.set_item(k.clone(), v);
            }
        }
        dict
    };
    
    let tuple = PyTuple::new(py, &[
        bv_nr.to_string().to_object(py),
        text_sauber.to_string().to_object(py), 
        regex_list.to_object(py)
    ]);
    let result = fun.call1(py, tuple).map_err(|e| format!("{}", e))?;
    let extract = result.as_ref(py).extract::<Vec<Spalte1Eintrag>>().map_err(|e| format!("{}", e))?;
    
    Ok(extract)
    
    
}

fn parse_spalte_1_veraenderung(spalte_1: &str) -> Result<Vec<usize>, String> {
    Ok(Vec::new()) // TODO
}
