use crate::{Grundbuch, Konfiguration};
use crate::digitalisiere::{Nebenbeteiligter, NebenbeteiligterExtra, BvEintrag, Bestandsverzeichnis};
use crate::kurztext::{self, SchuldenArt, RechteArt};
use serde_derive::{Serialize, Deserialize};
use std::collections::{BTreeMap, BTreeSet};
use pyo3::{Python, pyclass, pymethods};
use crate::get_or_insert_regex;

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[pyclass(name = "FlurFlurstueck")]
#[repr(C)]
pub struct FlurFlurstueck {
    pub gemarkung: Option<String>,
    pub flur: usize,
    pub flurstueck: String,
}

#[allow(non_snake_case)]
#[pymethods]
impl FlurFlurstueck {
    #[new]
    #[args(gemarkung = "None")]
    fn new(flur: usize, flurstueck: String, gemarkung: Option<String>) -> Self {
        Self {
            gemarkung,
            flur,
            flurstueck,
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
    
    let mut belastet_bv = Vec::new();
    
    // Spalte 1 Einträge => Bestandsverzeichnis Einträge
    for s1 in spalte1_eintraege {
        
        /*
        let bv_nr = ;
        let gemarkung = ;
        let flur = ;
        let flurstueck = ;
        */
        
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
