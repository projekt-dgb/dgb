use lazy_static::lazy_static;
use regex::Regex;
use crate::{Konfiguration, python::{PyVm, Betrag, RechteArt, SchuldenArt}};

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
    vm: PyVm,
    recht_id: &str, 
    input: &str, 
    fehler: &mut Vec<String>, 
    konfiguration: &Konfiguration
) -> KurzTextAbt2 {
    
    let (text_sauber, saetze_clean) = match text_saubern(vm.clone(), input, konfiguration) {
        Ok(o) => o,
        Err(e) => {
            fehler.push(e);
            (String::new(), Vec::new())
        }
    };
    
    let rechtsinhaber = match crate::python::get_rechtsinhaber_abt2(
        vm.clone(), 
        recht_id, 
        &text_sauber, 
        &saetze_clean, 
        konfiguration
    ) {
        Ok(o) => Some(o),
        Err(e) => {
            fehler.push(e);
            None
        }
    };
    
    let rechteart = match crate::python::get_rechte_art_abt2(
        vm.clone(),
        recht_id,
        &text_sauber, 
        &saetze_clean, 
        konfiguration,
    ) {
        Ok(o) => Some(o),
        Err(e) => {
            fehler.push(e);
            None
        }
    };
    
    let rangvermerk = match crate::python::get_rangvermerk_abt2(
        vm.clone(),
        recht_id,
        &text_sauber, 
        &saetze_clean, 
        konfiguration,
    ) {
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

    let gekuerzt = match crate::python::get_kurztext_abt2(
        vm.clone(),
        recht_id,
        &text_sauber, 
        rechtsinhaber.clone(),
        rangvermerk.clone(),
        &saetze_clean, 
        konfiguration,
    ) {
        Ok(o) => o.trim().to_string(),
        Err(e) => {
            fehler.push(e);
            String::new()
        }
    };

    let eingetragen_am = get_eingetragen_am(&saetze_clean);
    let rechtsinhaber = rechtsinhaber.clone().unwrap_or_default();
    let rechtsinhaber = {
        if rechtsinhaber.trim().is_empty() { 
            None 
        } else { 
            Some(rechtsinhaber.trim().to_string()) 
        }
    };

    KurzTextAbt2 {
        text_sauber,
        gekuerzt,
        rechtsinhaber,
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
    vm: PyVm,
    recht_id: &str, 
    betrag: &str, 
    input: &str, 
    fehler: &mut Vec<String>, 
    konfiguration: &Konfiguration
) -> KurzTextAbt3 {

    let (text_sauber, saetze_clean) = match text_saubern(vm.clone(), input, konfiguration) {
        Ok(o) => o,
        Err(e) => {
            fehler.push(e);
            (String::new(), Vec::new())
        }
    };
    
    let rechtsinhaber = match crate::python::get_rechtsinhaber_abt3(
        vm.clone(), 
        recht_id, 
        &text_sauber, 
        &saetze_clean, 
        konfiguration
    ) {
        Ok(o) => Some(o),
        Err(e) => {
            fehler.push(e);
            None
        }
    };

    let schuldenart = match crate::python::get_schulden_art_abt3(
        vm.clone(),
        recht_id,
        &text_sauber, 
        &saetze_clean, 
        konfiguration,
    ) {
        Ok(o) => Some(o),
        Err(e) => {
            fehler.push(e);
            None
        }
    };

    let betrag = match crate::python::get_betrag_abt3(
        vm.clone(),
        recht_id,
        betrag,
        &[betrag.to_string()], 
        konfiguration,
    ) {
        Ok(o) => Some(o),
        Err(e) => {
            fehler.push(e);
            None
        }
    };

    let gekuerzt = match crate::python::get_kurztext_abt3(
        vm.clone(),
        recht_id,
        &text_sauber, 
        betrag.map(|b| format!("{} {}", formatiere_betrag(&b), b.waehrung.to_string())),
        schuldenart.map(|s| format!("{}", s.to_string())),
        rechtsinhaber.clone(),
        &saetze_clean, 
        konfiguration,
    ) {
        Ok(o) => o.trim().to_string(),
        Err(e) => {
            fehler.push(e);
            String::new()
        }
    };

    let eingetragen_am = get_eingetragen_am(&saetze_clean);
    let rechtsinhaber = rechtsinhaber.unwrap_or_default();
    let rechtsinhaber = {
        if rechtsinhaber.trim().is_empty() { 
            None 
        } else { 
            Some(rechtsinhaber.trim().to_string()) 
        }
    };

    KurzTextAbt3 {
        text_sauber,
        gekuerzt,
        rechtsinhaber,
        schuldenart,
        saetze: saetze_clean,
        betrag: betrag.unwrap_or_default(),
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

/// Säubert den Text und zerlegt den Text in Sätze
pub fn text_saubern(
    vm: PyVm, 
    input: &str, 
    konfiguration: &Konfiguration,
) -> Result<(String, Vec<String>), String> {

    let text_sauber = crate::python::text_saubern(
        vm.clone(),
        input,
        konfiguration,
    )?;
    
    let abkuerzungen = crate::python::get_abkuerzungen(
        vm.clone(),
        konfiguration,
    )?;

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
