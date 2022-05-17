use lazy_static::lazy_static;
use regex::Regex;
use serde_derive::{Serialize, Deserialize};
use crate::{Konfiguration, analyse::{Betrag, Waehrung}};
use pyo3::pyclass;
use pyo3::prelude::*;

#[derive(Debug, Clone)]
pub struct KurzTextAbt2 {
    pub text_sauber: String,
    pub gekuerzt: String,
    pub rechtsinhaber: Option<String>,
    pub rechteart: Option<RechteArt>,
    pub rangvermerk: Option<String>,
    pub saetze: Vec<String>,
    pub eingetragen_am: Option<String>,
}

pub fn text_kuerzen_abt2(
    recht_id: &str, 
    input: &str, 
    fehler: &mut Vec<String>, 
    konfiguration: &Konfiguration
) -> KurzTextAbt2 {
    
    let (text_sauber, saetze_clean) = match text_saubern(input, konfiguration) {
        Ok(o) => o,
        Err(e) => {
            fehler.push(e);
            (String::new(), Vec::new())
        }
    };
    
    let rechtsinhaber = match Python::with_gil(|py| {
        crate::python_exec_kurztext_string(
            py,
            recht_id,
            &text_sauber, 
            &saetze_clean, 
            &konfiguration.rechtsinhaber_auslesen_abt2_script, 
            konfiguration
        ) 
    }) {
        Ok(o) => Some(o),
        Err(e) => {
            fehler.push(e);
            None
        }
    };
    
    let rechteart = match Python::with_gil(|py| {
        let rechteart: Result<RechteArtPyWrapper, String> = crate::python_exec_kurztext(
            py,
            recht_id,
            &text_sauber, 
            &saetze_clean, 
            &konfiguration.klassifiziere_rechteart, 
            konfiguration
        );
        Ok(rechteart?.inner)
    }) {
        Ok(o) => Some(o),
        Err(e) => {
            fehler.push(e);
            None
        }
    };
    
    let rangvermerk = match Python::with_gil(|py| {
        crate::python_exec_kurztext_string(
            py,
            recht_id,
            &text_sauber, 
            &saetze_clean, 
            &konfiguration.rangvermerk_auslesen_abt2_script, 
            konfiguration
        )
    }) {
        Ok(o) => {
            if o.trim().is_empty() {
                None
            } else {
                Some(o.trim().to_string())
            }
        },
        Err(e) => {
            fehler.push(e);
            None
        }
    };
    
    let gekuerzt = match Python::with_gil(|py| {
        crate::python_exec_kuerze_text_abt2(
            py,
            recht_id,
            &text_sauber,
            rechtsinhaber.clone(),
            rangvermerk.clone(),
            &saetze_clean, 
            &konfiguration.text_kuerzen_abt2_script, 
            konfiguration
        )
    }) {
        Ok(o) => o.trim().to_string(),
        Err(e) => {
            fehler.push(e);
            String::new()
        }
    };
    
    let eingetragen_am = get_eingetragen_am(&saetze_clean);
    let rechtsinhaber = rechtsinhaber.clone().unwrap_or_default();
    
    KurzTextAbt2 {
        text_sauber,
        gekuerzt,
        rechtsinhaber: {
            if rechtsinhaber.trim().is_empty() { 
                None 
            } else { 
                Some(rechtsinhaber.trim().to_string()) 
            }
        },
        rechteart,
        rangvermerk,
        saetze: saetze_clean,
        eingetragen_am,
    }
}

#[derive(Debug, Clone)]
pub struct KurzTextAbt3 {
    pub text_sauber: String,
    pub gekuerzt: String,
    pub rechtsinhaber: Option<String>,
    pub schuldenart: Option<SchuldenArt>,
    pub saetze: Vec<String>,
    pub betrag: Betrag,
    pub eingetragen_am: Option<String>,
}

pub fn text_kuerzen_abt3(
    recht_id: &str, 
    betrag: &str, 
    input: &str, 
    fehler: &mut Vec<String>, 
    konfiguration: &Konfiguration
) -> KurzTextAbt3 {

    let (text_sauber, saetze_clean) = match text_saubern(input, konfiguration) {
        Ok(o) => o,
        Err(e) => {
            fehler.push(e);
            (String::new(), Vec::new())
        }
    };    
    
    let rechtsinhaber = match Python::with_gil(|py| {
        crate::python_exec_kurztext_string(
            py,
            recht_id,
            &text_sauber, 
            &saetze_clean, 
            &konfiguration.rechtsinhaber_auslesen_abt3_script, 
            konfiguration
        ) 
    }) {
        Ok(o) => Some(o),
        Err(e) => {
            fehler.push(e);
            None
        }
    };

    let schuldenart = match Python::with_gil(|py| {
        let schuldenart: Result<SchuldenArtPyWrapper, String> = crate::python_exec_kurztext(
            py,
            recht_id,
            &text_sauber, 
            &saetze_clean, 
            &konfiguration.klassifiziere_schuldenart, 
            konfiguration
        );
        Ok(schuldenart?.inner)
    }) {
        Ok(o) => Some(o),
        Err(e) => {
            fehler.push(e);
            None
        }
    };
        
    let betrag = match Python::with_gil(|py| {
        let betrag: Result<PyBetrag, String> = crate::python_exec_kurztext(
            py,
            recht_id,
            betrag, 
            &[betrag.to_string()], 
            &konfiguration.betrag_auslesen_script, 
            konfiguration
        );
        Ok(betrag?.inner)
    }) {
        Ok(o) => Some(o),
        Err(e) => {
            fehler.push(e);
            None
        }
    };

    let gekuerzt = match Python::with_gil(|py| {
        crate::python_exec_kuerze_text_abt3(
            py,
            recht_id,
            &text_sauber,
            betrag.map(|b| format!("{} {}", formatiere_betrag(&b), b.waehrung.to_string())),
            schuldenart.map(|s| format!("{}", s.to_string())),
            rechtsinhaber.clone(),
            &saetze_clean, 
            &konfiguration.text_kuerzen_abt3_script, 
            konfiguration
        )
    }) {
        Ok(o) => o,
        Err(e) => {
            fehler.push(e);
            String::new()
        }
    };

    let eingetragen_am = get_eingetragen_am(&saetze_clean);
    let rechtsinhaber = rechtsinhaber.unwrap_or_default();
    
    KurzTextAbt3 {
        text_sauber,
        gekuerzt,
        rechtsinhaber: {
            if rechtsinhaber.trim().is_empty() { 
                None 
            } else { 
                Some(rechtsinhaber.trim().to_string()) 
            }
        },
        schuldenart,
        saetze: saetze_clean,
        betrag: betrag.unwrap_or(Betrag {
            wert: 0,
            nachkomma: 0,
            waehrung: Waehrung::Euro,
        }),
        eingetragen_am,
    }
}

lazy_static! {
    static ref EINGETRAGEN_AM_REGEX: Regex = Regex::new(r"ingetragen am (\d\d).(\d\d).(\d\d\d\d)").unwrap();
    static ref UEBERTRAGEN_AM_REGEX: Regex = Regex::new(r"hierher übertragen am (\d\d).(\d\d).(\d\d\d\d)").unwrap();
    static ref EINGETRAGEN_AM_REGEX_2: Regex = Regex::new(r"ingetragen (.*) am (\d\d).(\d\d).(\d\d\d\d)").unwrap();
}

fn get_eingetragen_am(saetze_clean: &Vec<String>) -> Option<String> {
    
    let mut eingetragen_am = None;
    
    for s in saetze_clean.iter() {
        for c in EINGETRAGEN_AM_REGEX.captures_iter(s.as_str()) {
            if let (Some(d), Some(m), Some(y)) = (c.get(1), c.get(2), c.get(3)) {
                eingetragen_am = Some(format!("{}.{}.{}", 
                    d.as_str().trim(),
                    m.as_str().trim(),
                    y.as_str().trim(),
                ));
            }
        }
        
        for c in EINGETRAGEN_AM_REGEX_2.captures_iter(s.as_str()) {
            if let (Some(d), Some(m), Some(y)) = (c.get(2), c.get(3), c.get(4)) {
                eingetragen_am = Some(format!("{}.{}.{}", 
                    d.as_str().trim(),
                    m.as_str().trim(),
                    y.as_str().trim(),
                ));
            }
        }
    }
    
    if eingetragen_am.is_none() {
        for s in saetze_clean.iter() {
           for c in UEBERTRAGEN_AM_REGEX.captures_iter(s.as_str()) {
                if let (Some(d), Some(m), Some(y)) = (c.get(1), c.get(2), c.get(3)) {
                    eingetragen_am = Some(format!("{}.{}.{}", 
                        d.as_str().trim(),
                        m.as_str().trim(),
                        y.as_str().trim(),
                    ));
                }
            }
        }
    }
    
    eingetragen_am
}


// 100000 => "100.000,00"
// 1500000 => "1.500.000,00"
pub fn formatiere_betrag(b: &Betrag) -> String {
    
    let letzte_drei_stellen = b.wert % 1000;
    let hunderttausender = b.wert / 1000;
    
    if b.wert >= 1_000_000 {
        let millionen = hunderttausender / 1000;
        let hunderttausender = hunderttausender % 1000;
        format!("{}.{:03}.{:03},{:02}", millionen, hunderttausender, letzte_drei_stellen, b.nachkomma)
    } else if b.wert >= 100_000 {
        let hunderttausender = hunderttausender % 1000;
        format!("{:03}.{:03},{:02}", hunderttausender, letzte_drei_stellen, b.nachkomma)
    } else if b.wert >= 1000 {
        let hunderttausender = hunderttausender % 1000;
        format!("{}.{:03},{:02}", hunderttausender, letzte_drei_stellen, b.nachkomma)
    } else {
        format!("{},{:02}", letzte_drei_stellen, b.nachkomma)
    }
}

pub fn python_text_saubern<'py>(
    py: Python<'py>,
    input: &str, 
    konfiguration: &Konfiguration
) -> Result<String, String> {

    use pyo3::prelude::*;
    use pyo3::types::{PyDict, PyTuple};
    use crate::get_or_insert_regex;

    let script = konfiguration.text_saubern_script
        .iter()
        .map(|l| format!("    {}", l))
        .collect::<Vec<_>>()
        .join("\r\n");
        
    let script = script.replace("\t", "    ");
    let script = script.replace("\u{00a0}", " ");
    let py_code = format!("def run_script(*args, **kwargs):\r\n    recht, re = args\r\n{}", script);
    let regex_values = konfiguration.regex.values().cloned().collect::<Vec<_>>();
    
    let mut module = PyModule::from_code(py, &py_code, "", "main").map_err(|e| format!("{}", e))?;

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
    let tuple = PyTuple::new(py, &[input.to_string().to_object(py), regex_list.to_object(py)]);
    let result = fun.call1(py, tuple).map_err(|e| format!("{}", e))?;
    let extract = result.as_ref(py).extract::<String>().map_err(|e| format!("{}", e))?;
    Ok(extract)
}

fn python_get_abkuerzungen<'py>(
    py: Python<'py>,
    konfiguration: &Konfiguration,
) -> Result<Vec<String>, String> {

    use pyo3::prelude::*;
    use pyo3::types::PyTuple;

    let script = konfiguration.abkuerzungen_script
        .iter()
        .map(|l| format!("    {}", l))
        .collect::<Vec<_>>()
        .join("\r\n");
        
    let py_code = format!("def run_script(*args, **kwargs):\r\n{}", script);
    let mut module = PyModule::from_code(py, &py_code, "", "main").map_err(|e| format!("{}", e))?;
    let fun: Py<PyAny> = module.getattr("run_script").unwrap().into();
    let tuple = PyTuple::new(py, &[String::new().to_object(py)]);
    let result = fun.call1(py, tuple).map_err(|e| format!("{}", e))?;
    let extract = result
        .as_ref(py)
        .extract::<Vec<String>>()
        .map_err(|e| format!("{}", e))?;
    Ok(extract)
}

/// Säubert den Text und zerlegt den Text in Sätze
pub fn text_saubern(input: &str, konfiguration: &Konfiguration) -> Result<(String, Vec<String>), String> {

    let text_sauber = Python::with_gil(|py| {
        python_text_saubern(py, input, konfiguration)
        .map_err(|e| format!("In Funktion text_säubern(): {}", e))
    })?;
    let abkuerzungen = Python::with_gil(|py| {
        python_get_abkuerzungen(py, konfiguration)
        .map_err(|e| format!("In Funktion abkuerzungen(): {}", e))
    })?;

    let text_sauber = text_sauber.trim().to_string();
    let kurztext_text_sauber = text_sauber.clone();
    
    let chars = text_sauber.chars().collect::<Vec<_>>();
    let mut saetze = Vec::<String>::new();
    let mut letzter_satz = Vec::new();
    for (ch_idx, c) in chars.iter().enumerate() {
        if *c == '.' && 
           chars.get(ch_idx + 1).copied() == Some(' ') && 
           chars.get(ch_idx + 2).is_some() &&
           chars[ch_idx + 2].is_uppercase() {
            saetze.push(letzter_satz.clone().into_iter().collect());
            letzter_satz.clear();
        } else {
            letzter_satz.push(*c);
        }
    }
    
    if !letzter_satz.is_empty() {
        saetze.push(letzter_satz.clone().into_iter().collect());
        letzter_satz.clear();
    }
    
    static MONATE: &[&'static str;12] = &[
        "Januar", "Februar", "März", 
        "April", "Juni", "Juli", 
        "Mai", "August", "September", 
        "Oktober", "November", "Dezember"
    ];
    
    // Manche Abkürzungen werden versehentlich als Satzendungen erkannt ("Dr.", "v.", etc.)
    let mut saetze_clean = Vec::new();
    let mut letzter_satz = String::new();
    for (s_idx, s) in saetze.iter().enumerate() {

        let naechster_satz_faengt_mit_großbuchstaben_an = saetze.get(s_idx + 1).and_then(|s| s.trim().chars().nth(0)).map(|fc| fc.is_uppercase()).unwrap_or(false);
        let naechster_satz_ist_monat = saetze.get(s_idx + 1).map(|s| {
            MONATE.iter().any(|m| s.trim().starts_with(m))
        }).unwrap_or(false);
        
        let endet_mit_abkuerzung = 
            // Satz endet mit Abkürzung oder Zahl: vereinen
            abkuerzungen.iter().any(|a| s.ends_with(a)) || 
            s.chars().last().map(|c| c.is_numeric()).unwrap_or(false) && 
            // nächster Satz fängt mit Großbuchstaben an: trennen, außer: Satz = Monatsname
            !(naechster_satz_faengt_mit_großbuchstaben_an && !naechster_satz_ist_monat);

        if endet_mit_abkuerzung {
            letzter_satz.push_str(s);
            letzter_satz.push_str(".");
        } else {
            letzter_satz.push_str(s);
            let letzter_satz_kopie = letzter_satz.trim().to_string();
            if letzter_satz_kopie.ends_with(".") {
                saetze_clean.push(letzter_satz_kopie[..letzter_satz_kopie.len() - 1].to_string());
            } else {
                saetze_clean.push(letzter_satz_kopie);
            }
            letzter_satz.clear();
        }
    }
    
    if !letzter_satz.is_empty() {
        saetze_clean.push(letzter_satz.trim().to_string());
        letzter_satz.clear();
    }
    
    Ok((text_sauber, saetze_clean))
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[repr(C)]
pub enum SchuldenArt {
    Grundschuld,
    Hypothek,
    Rentenschuld,
    Aufbauhypothek,
    Sicherungshypothek,
    Widerspruch,
    Arresthypothek,
    SicherungshypothekGem128ZVG,
    Hoechstbetragshypothek,
    Sicherungsgrundschuld,
    Zwangssicherungshypothek,
    NichtDefiniert,
}

impl SchuldenArt {
    pub fn to_string(&self) -> &'static str {
        use self::SchuldenArt::*;
        match self {
            Grundschuld => "Grundschuld",
            Hypothek => "Hypothek",
            Rentenschuld => "Rentenschuld",
            Aufbauhypothek => "Aufbauhypothek",
            Sicherungshypothek => "Sicherungshypothek",
            Widerspruch => "Widerspruch",
            Arresthypothek => "Arresthypothek",
            SicherungshypothekGem128ZVG => "Sicherungshypothek gemäß §128 ZVG",
            Hoechstbetragshypothek => "Höchstbetragshypothek",
            Sicherungsgrundschuld => "Sicherungsgrundschuld",
            Zwangssicherungshypothek => "Zwangssicherungshypothek",
            NichtDefiniert => "",
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[pyclass(name = "RechteArt")]
#[repr(C)]
pub struct RechteArtPyWrapper {
    pub inner: RechteArt
}

#[allow(non_snake_case)]
#[pymethods]
impl RechteArtPyWrapper {
    #[staticmethod] fn SpeziellVormerkung(rechteverweis: usize) -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::SpeziellVormerkung { rechteverweis } }}
    #[classattr] fn Abwasserleitungsrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Abwasserleitungsrecht }}
    #[classattr] fn Auflassungsvormerkung() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Auflassungsvormerkung }}
    #[classattr] fn Ausbeutungsrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Ausbeutungsrecht }}
    #[classattr] fn AusschlussDerAufhebungDerGemeinschaftGem1010BGB() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::AusschlussDerAufhebungDerGemeinschaftGem1010BGB }}
    #[classattr] fn Baubeschraenkung() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Baubeschraenkung }}
    #[classattr] fn Bebauungsverbot() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Bebauungsverbot }}
    #[classattr] fn Benutzungsrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Benutzungsrecht }}
    #[classattr] fn BenutzungsregelungGem1010BGB() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::BenutzungsregelungGem1010BGB }}
    #[classattr] fn Bepflanzungsverbot() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Bepflanzungsverbot }}
    #[classattr] fn Bergschadenverzicht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Bergschadenverzicht }}
    #[classattr] fn Betretungsrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Betretungsrecht }}
    #[classattr] fn Bewässerungsrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Bewässerungsrecht }}
    #[classattr] fn BpD() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::BpD }}
    #[classattr] fn BesitzrechtNachEGBGB() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::BesitzrechtNachEGBGB }}
    #[classattr] fn BohrUndSchuerfrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::BohrUndSchuerfrecht }}
    #[classattr] fn Brunnenrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Brunnenrecht }}
    #[classattr] fn Denkmalschutz() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Denkmalschutz }}
    #[classattr] fn DinglichesNutzungsrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::DinglichesNutzungsrecht }}
    #[classattr] fn DuldungVonEinwirkungenDurchBaumwurf() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::DuldungVonEinwirkungenDurchBaumwurf }}
    #[classattr] fn DuldungVonFernmeldeanlagen() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::DuldungVonFernmeldeanlagen }}
    #[classattr] fn Durchleitungsrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Durchleitungsrecht }}
    #[classattr] fn EinsitzInsitzrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::EinsitzInsitzrecht }}
    #[classattr] fn Entwasserungsrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Entwasserungsrecht }}
    #[classattr] fn Erbbaurecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Erbbaurecht }}
    #[classattr] fn Erwerbsvormerkung() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Erwerbsvormerkung }}
    #[classattr] fn Fensterrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Fensterrecht }}
    #[classattr] fn Fensterverbot() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Fensterverbot }}
    #[classattr] fn Fischereirecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Fischereirecht }}
    #[classattr] fn Garagenrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Garagenrecht }}
    #[classattr] fn Gartenbenutzungsrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Gartenbenutzungsrecht }}
    #[classattr] fn GasleitungGasreglerstationFerngasltg() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::GasleitungGasreglerstationFerngasltg }}
    #[classattr] fn GehWegeFahrOderLeitungsrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::GehWegeFahrOderLeitungsrecht }}
    #[classattr] fn Gewerbebetriebsbeschrankung() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Gewerbebetriebsbeschrankung }}
    #[classattr] fn GewerblichesBenutzungsrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::GewerblichesBenutzungsrecht }}
    #[classattr] fn Grenzbebauungsrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Grenzbebauungsrecht }}
    #[classattr] fn Grunddienstbarkeit() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Grunddienstbarkeit }}
    #[classattr] fn Hochspannungsleitungsrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Hochspannungsleitungsrecht }}
    #[classattr] fn Immissionsduldungsverpflichtung() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Immissionsduldungsverpflichtung }}
    #[classattr] fn Insolvenzvermerk() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Insolvenzvermerk }}
    #[classattr] fn Kabelrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Kabelrecht }}
    #[classattr] fn Kanalrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Kanalrecht }}
    #[classattr] fn Kiesabbauberechtigung() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Kiesabbauberechtigung }}
    #[classattr] fn Kraftfahrzeugabstellrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Kraftfahrzeugabstellrecht }}
    #[classattr] fn LeibgedingAltenteilsrechtAuszugsrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::LeibgedingAltenteilsrechtAuszugsrecht }}
    #[classattr] fn LeitungsOderAnlagenrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::LeitungsOderAnlagenrecht }}
    #[classattr] fn Mauerrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Mauerrecht }}
    #[classattr] fn Mitbenutzungsrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Mitbenutzungsrecht }}
    #[classattr] fn Mobilfunkstationsrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Mobilfunkstationsrecht }}
    #[classattr] fn Muehlenrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Muehlenrecht }}
    #[classattr] fn Mulltonnenabstellrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Mulltonnenabstellrecht }}
    #[classattr] fn Nacherbenvermerk() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Nacherbenvermerk }}
    #[classattr] fn Niessbrauchrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Niessbrauchrecht }}
    #[classattr] fn Nutzungsbeschrankung() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Nutzungsbeschrankung }}
    #[classattr] fn Pfandung() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Pfandung }}
    #[classattr] fn Photovoltaikanlagenrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Photovoltaikanlagenrecht }}
    #[classattr] fn Pumpenrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Pumpenrecht }}
    #[classattr] fn Reallast() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Reallast }}
    #[classattr] fn RegelungUeberDieHöheDerNotwegrenteGemaess912Bgb() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::RegelungUeberDieHöheDerNotwegrenteGemaess912Bgb }}
    #[classattr] fn RegelungUeberDieHöheDerUeberbaurenteGemaess912Bgb() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::RegelungUeberDieHöheDerUeberbaurenteGemaess912Bgb }}
    #[classattr] fn Rueckauflassungsvormerkung() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Rueckauflassungsvormerkung }}
    #[classattr] fn Ruckerwerbsvormerkung() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Ruckerwerbsvormerkung }}
    #[classattr] fn Sanierungsvermerk() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Sanierungsvermerk }}
    #[classattr] fn Schachtrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Schachtrecht }}
    #[classattr] fn SonstigeDabagrechteart() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::SonstigeDabagrechteart }}
    #[classattr] fn SonstigeRechte() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::SonstigeRechte }}
    #[classattr] fn Tankstellenrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Tankstellenrecht }}
    #[classattr] fn Testamentsvollstreckervermerk() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Testamentsvollstreckervermerk }}
    #[classattr] fn Transformatorenrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Transformatorenrecht }}
    #[classattr] fn Ueberbaurecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Ueberbaurecht }}
    #[classattr] fn UebernahmeVonAbstandsflachen() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::UebernahmeVonAbstandsflachen }}
    #[classattr] fn Umlegungsvermerk() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Umlegungsvermerk }}
    #[classattr] fn Umspannanlagenrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Umspannanlagenrecht }}
    #[classattr] fn Untererbbaurecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Untererbbaurecht }}
    #[classattr] fn VerausserungsBelastungsverbot() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::VerausserungsBelastungsverbot }}
    #[classattr] fn Verfuegungsverbot() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Verfuegungsverbot }}
    #[classattr] fn VerwaltungsUndBenutzungsregelung() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::VerwaltungsUndBenutzungsregelung }}
    #[classattr] fn VerwaltungsregelungGem1010Bgb() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::VerwaltungsregelungGem1010Bgb }}
    #[classattr] fn VerzichtAufNotwegerente() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::VerzichtAufNotwegerente }}
    #[classattr] fn VerzichtAufUeberbaurente() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::VerzichtAufUeberbaurente }}
    #[classattr] fn Viehtrankerecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Viehtrankerecht }}
    #[classattr] fn Viehtreibrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Viehtreibrecht }}
    #[classattr] fn Vorkaufsrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Vorkaufsrecht }}
    #[classattr] fn Wasseraufnahmeverpflichtung() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Wasseraufnahmeverpflichtung }}
    #[classattr] fn Wasserentnahmerecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Wasserentnahmerecht }}
    #[classattr] fn Weiderecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Weiderecht }}
    #[classattr] fn Widerspruch() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Widerspruch }}
    #[classattr] fn Windkraftanlagenrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Windkraftanlagenrecht }}
    #[classattr] fn Wohnrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Wohnrecht }}
    #[classattr] fn WohnungsOderMitbenutzungsrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::WohnungsOderMitbenutzungsrecht }}
    #[classattr] fn Wohnungsbelegungsrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Wohnungsbelegungsrecht }}
    #[classattr] fn WohnungsrechtNach1093Bgb() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::WohnungsrechtNach1093Bgb }}
    #[classattr] fn Zaunerrichtungsverbot() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Zaunerrichtungsverbot }}
    #[classattr] fn Zaunrecht() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Zaunrecht }}
    #[classattr] fn Zustimmungsvorbehalt() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Zustimmungsvorbehalt }}
    #[classattr] fn Zwangsversteigerungsvermerk() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Zwangsversteigerungsvermerk }}
    #[classattr] fn Zwangsverwaltungsvermerk() -> RechteArtPyWrapper { RechteArtPyWrapper { inner: RechteArt::Zwangsverwaltungsvermerk }}
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[pyclass(name = "SchuldenArt")]
#[repr(C)]
pub struct SchuldenArtPyWrapper {
    pub inner: SchuldenArt,
}

#[allow(non_snake_case)]
#[pymethods]
impl SchuldenArtPyWrapper {
    #[classattr] fn Grundschuld() -> SchuldenArtPyWrapper { SchuldenArtPyWrapper { inner: SchuldenArt::Grundschuld }}
    #[classattr] fn Hypothek() -> SchuldenArtPyWrapper { SchuldenArtPyWrapper { inner: SchuldenArt::Hypothek }}
    #[classattr] fn Rentenschuld() -> SchuldenArtPyWrapper { SchuldenArtPyWrapper { inner: SchuldenArt::Rentenschuld }}
    #[classattr] fn Aufbauhypothek() -> SchuldenArtPyWrapper { SchuldenArtPyWrapper { inner: SchuldenArt::Aufbauhypothek }}
    #[classattr] fn Sicherungshypothek() -> SchuldenArtPyWrapper { SchuldenArtPyWrapper { inner: SchuldenArt::Sicherungshypothek }}
    #[classattr] fn Widerspruch() -> SchuldenArtPyWrapper { SchuldenArtPyWrapper { inner: SchuldenArt::Widerspruch }}
    #[classattr] fn Arresthypothek() -> SchuldenArtPyWrapper { SchuldenArtPyWrapper { inner: SchuldenArt::Arresthypothek }}
    #[classattr] fn SicherungshypothekGem128ZVG() -> SchuldenArtPyWrapper { SchuldenArtPyWrapper { inner: SchuldenArt::SicherungshypothekGem128ZVG }}
    #[classattr] fn Hoechstbetragshypothek() -> SchuldenArtPyWrapper { SchuldenArtPyWrapper { inner: SchuldenArt::Hoechstbetragshypothek }}
    #[classattr] fn Sicherungsgrundschuld() -> SchuldenArtPyWrapper { SchuldenArtPyWrapper { inner: SchuldenArt::Sicherungsgrundschuld }}
    #[classattr] fn Zwangssicherungshypothek() -> SchuldenArtPyWrapper { SchuldenArtPyWrapper { inner: SchuldenArt::Zwangssicherungshypothek }}
    #[classattr] fn NichtDefiniert() -> SchuldenArtPyWrapper { SchuldenArtPyWrapper { inner: SchuldenArt::NichtDefiniert }}
}


#[derive(Debug, Clone, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
#[pyclass(name = "Waehrung")]
pub struct PyWaehrung {
    inner: Waehrung,
}

#[allow(non_snake_case)]
#[pymethods]
impl PyWaehrung {
    #[classattr] fn Euro() -> PyWaehrung { PyWaehrung { inner: Waehrung::Euro }}
    #[classattr] fn DMark() -> PyWaehrung { PyWaehrung { inner: Waehrung::DMark }}
    #[classattr] fn MarkDDR() -> PyWaehrung { PyWaehrung { inner: Waehrung::MarkDDR }}
    #[classattr] fn Goldmark() -> PyWaehrung { PyWaehrung { inner: Waehrung::Goldmark }}
    #[classattr] fn Rentenmark() -> PyWaehrung { PyWaehrung { inner: Waehrung::Rentenmark }}
    #[classattr] fn Reichsmark() -> PyWaehrung { PyWaehrung { inner: Waehrung::Reichsmark }}
    #[classattr] fn GrammFeingold() -> PyWaehrung { PyWaehrung { inner: Waehrung::GrammFeingold }}
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
#[pyclass(name = "Betrag")]
pub struct PyBetrag {
    pub inner: Betrag,
}

#[allow(non_snake_case)]
#[pymethods]
impl PyBetrag {
    #[new]
    fn new(wert: usize, nachkomma: usize, waehrung: PyWaehrung) -> Self {
        Self { inner: Betrag { wert, nachkomma, waehrung: waehrung.inner } }
    }
}

// TODO: teilw. Flurstücke möglicherweise Komma drin

// Das Recht ist vererblich. => raus
// Das Recht ist auflösend bedingt. => bei bedingter Auflassungsvormerkung drin lassen, ansonsten raus

// , als Gesamtberechtigte gemäß § 428 BGB - => im Text lassen, im Rechteinhaber nicht
// "für den Fall der Übernahme der Rechte und Pflichten des Berechtigten zugunsten der ENERTRAG Netz GmbH, Dauerthal"
// im gleichen Rang mit Abt. II Nr. 5 am 11.05.1995
// Abt 2 Nr. 10 falsch
// Abt 2 Nr. 7 falsch
// Abt 2 Nr. 3
// Abt 2 Nr. 8 => "Glatz"
// Abt 2 Nr. 11 falsch
// Abt 2 Nr. 12 => Gleichrang nicht drin
// "Widerspruch gemäß § 53 GBO" 
// Eingetragen mit Bezug
// Von Amts wegen eingetragen
// Abt 2 Nr. 20 => "Rechteinhaber 
// Widerspruch ... [für / zugunsten] ... gegen ...

#[derive(Debug, PartialEq, Copy, Clone, Serialize, Deserialize)]
#[repr(C)]
pub enum RechteArt {
    SpeziellVormerkung { rechteverweis: usize },                          //     Vormerkung zur Sicherung
    Abwasserleitungsrecht,                                                //     Abwasserleitungsrecht
    Auflassungsvormerkung,                                                //     Auflassungsvormerkung
    Ausbeutungsrecht,                                                     //     Ausbeutungsrecht
    AusschlussDerAufhebungDerGemeinschaftGem1010BGB,                      //     Ausschluss der Aufhebung der Gemeinschaft gem. $ 1010 BGB
    Baubeschraenkung,                                                     //     Baubeschränkung
    Bebauungsverbot,                                                      //     Bebauungsverbot
    Benutzungsrecht,                                                      //     Benutzungsrecht
    BenutzungsregelungGem1010BGB,                                         //     Benutzungsregelung gem. §1010 BGB
    Bepflanzungsverbot,                                                   //     Bepflanzungsverbot
    Bergschadenverzicht,                                                  //     Bergschadenverzicht
    Betretungsrecht,                                                      //     Betretungsrecht
    Bewässerungsrecht,                                                    //     Bewässerungsrecht
    BpD,                                                                  //     beschrankte persönliche Dienstbarkeit
    BesitzrechtNachEGBGB,                                                 //     Besitzrecht nach EGBGB
    BohrUndSchuerfrecht,                                                  //     Bohr- und Schürfrecht
    Brunnenrecht,                                                         //     Brunnenrecht
    Denkmalschutz,                                                        //     Denkmalschutz
    DinglichesNutzungsrecht,                                              //     dingliches Nutzungsrecht
    DuldungVonEinwirkungenDurchBaumwurf,                                  //     Duldung von Einwirkungen durch Baumwurf
    DuldungVonFernmeldeanlagen,                                            //    Duldung von Femmeldeanlagen
    Durchleitungsrecht,                                                   //     Durchleitungsrecht
    EinsitzInsitzrecht,                                                   //     Einsitz-/ Insitzrecht
    Entwasserungsrecht,                                                   //     Entwasserungsrecht
    Erbbaurecht,                                                          //     Erbbaurecht
    Erwerbsvormerkung,                                                    //     Erwerbsvormerkung
    Fensterrecht,                                                         //     Fensterrecht
    Fensterverbot,                                                        //     Fensterverbot
    Fischereirecht,                                                       //     Fischereirecht
    Garagenrecht,                                                         //     Garagenrecht
    Gartenbenutzungsrecht,                                                //     Gartenbenutzungsrecht
    GasleitungGasreglerstationFerngasltg,                                 //     Gasleitung‚ Gasreglerstation, Ferngasltg.
    GehWegeFahrOderLeitungsrecht,                                         //     Geh-, Wege-, Fahr- oder Leitungsrecht
    Gewerbebetriebsbeschrankung,                                          //     Gewerbebetriebsbeschrankung
    GewerblichesBenutzungsrecht,                                          //     gewerbliches Benutzungsrecht
    Grenzbebauungsrecht,                                                  //     Grenzbebauungsrecht
    Grunddienstbarkeit,                                                   //     Grunddienstbarkeit
    Hochspannungsleitungsrecht,                                           //     Hochspannungsleitungsrecht
    Immissionsduldungsverpflichtung,                                      //     Immissionsduldungsverpflichtung
    Insolvenzvermerk,                                                     //     Insolvenzvermerk
    Kabelrecht,                                                           //     Kabelrecht
    Kanalrecht,                                                           //     Kanalrecht
    Kiesabbauberechtigung,                                                //     Kiesabbauberechtigung
    Kraftfahrzeugabstellrecht,                                            //     Kraftfahrzeugabstellrecht
    LeibgedingAltenteilsrechtAuszugsrecht,                                //     LeibgedingAttenteilsrechtAuszugsrecht
    LeitungsOderAnlagenrecht,                                             //     LeitungsOderAnlagenrecht
    Mauerrecht,                                                           //     Mauerrecht
    Mitbenutzungsrecht,                                                   //     Mitbenutzungsrecht
    Mobilfunkstationsrecht,                                               //     Mobilfunkstationsrecht
    Muehlenrecht,                                                         //     Mühlenrecht
    Mulltonnenabstellrecht,                                               //     Mulltonnenabstellrecht
    Nacherbenvermerk,                                                     //     Nacherbenvermerk
    Niessbrauchrecht,                                                     //     Nießbrauchrecht
    Nutzungsbeschrankung,                                                 //     Nutzungsbeschrankung
    Pfandung,                                                             //     Pfandung
    Photovoltaikanlagenrecht,                                             //     Photovoltaikanlagenrecht
    Pumpenrecht,                                                          //     Pumpenrecht
    Reallast,                                                             //     Reallast
    RegelungUeberDieHöheDerNotwegrenteGemaess912Bgb,                      //     Regelung über die Höhe der Notwegrente gemaß 8 912 BGB
    RegelungUeberDieHöheDerUeberbaurenteGemaess912Bgb,                    //     Regelung über die Höhe der Überbaurente gemaß $ 912 BGB
    Rueckauflassungsvormerkung,                                           //     Rueckauflassungsvormerkung
    Ruckerwerbsvormerkung,                                                //     Ruckerwerbsvormerkung
    Sanierungsvermerk,                                                    //     Sanierungsvermerk
    Schachtrecht,                                                         //     Schachtrecht
    SonstigeDabagrechteart,                                               //     sonstige dabag-Rechteart
    SonstigeRechte,                                                       //     Sonstige Rechte
    Tankstellenrecht,                                                     //     Tankstellenrecht
    Testamentsvollstreckervermerk,                                        //     Testamentsvollstreckervermerk
    Transformatorenrecht,                                                 //     Transformatorenrecht
    Ueberbaurecht,                                                        //     Überbaurecht
    UebernahmeVonAbstandsflachen,                                         //     Übernahme von Abstandsflachen
    Umlegungsvermerk,                                                     //     Umlegungsvermerk
    Umspannanlagenrecht,                                                  //     Umspannanlagenrecht
    Untererbbaurecht,                                                     //     Untererbbaurecht
    VerausserungsBelastungsverbot,                                        //     Veraußerungs-/Belastungsverbot
    Verfuegungsverbot,                                                    //     Verfügungsverbot
    VerwaltungsUndBenutzungsregelung,                                     //     Verwaltungs- und Benutzungsregelung
    VerwaltungsregelungGem1010Bgb,                                        //     Verwaltungsregelung gem. & 1010 BGB
    VerzichtAufNotwegerente,                                              //     Verzicht auf Notwegerente
    VerzichtAufUeberbaurente,                                             //     Verzicht auf Überbaurente
    Viehtrankerecht,                                                      //     Viehtrankerecht
    Viehtreibrecht,                                                       //     Viehtreibrecht
    Vorkaufsrecht,                                                        //     Vorkaufsrecht
    Wasseraufnahmeverpflichtung,                                          //     Wasseraufnahmeverpflichtung
    Wasserentnahmerecht,                                                  //     Wasserentnahmerecht
    Weiderecht,                                                           //     Weiderecht
    Widerspruch,                                                          //     Widerspruch
    Windkraftanlagenrecht,                                                //     Windkraftanlagenrecht
    Wohnrecht,                                                            //     Wohnrecht
    WohnungsOderMitbenutzungsrecht,                                       //     Wohnungs- oder Mitbenutzungsrecht
    Wohnungsbelegungsrecht,                                               //     Wohnungsbelegungsrecht
    WohnungsrechtNach1093Bgb,                                             //     Wohnungsrecht nach 81093 BGB
    Zaunerrichtungsverbot,                                                //     Zaunerrichtungsverbot
    Zaunrecht,                                                            //     Zaunrecht
    Zustimmungsvorbehalt,                                                 //     Zustimmungsvorbehalt
    Zwangsversteigerungsvermerk,                                          //     Zwangsversteigerungsvermerk
    Zwangsverwaltungsvermerk,                                             //     Zwangsverwaltungsvermerk
}

impl RechteArt {
    pub fn benoetigt_rechteinhaber(&self) -> bool {
        match self {
            | RechteArt::VerausserungsBelastungsverbot
            | RechteArt::Auflassungsvormerkung => false,
            _ => true,
        }
    }
}
