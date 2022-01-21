use lazy_static::lazy_static;
use regex::Regex;
use serde_derive::{Serialize, Deserialize};
use crate::analysiere::{Betrag, Waehrung};
use pyo3::pyclass;
use pyo3::prelude::*;

lazy_static! {
    static ref AUSSUEBUNG_REGEX: Regex = Regex::new(r"Die Ausübung (.*) darf Dritten überlassen werden").unwrap();
    static ref VOLLSTRECKBAR_REGEX: Regex = Regex::new(r"(.)ollstreckbar gemäß (.*) ZPO").unwrap();
    // vollstreckbar nach | vollstreckbar gemäß
    static ref VOLLSTRECKBAR_REGEX_2: Regex = Regex::new(r",?(.)(.)ollstreckbar nach (.*)").unwrap();

    static ref VERERBLICH_REGEX: Regex = Regex::new(r"Das Recht ist vererblich").unwrap();
    static ref VON_AMTS_WEGEN_EINGETRAGEN: Regex = Regex::new(r"Von Amts wegen eingetragen").unwrap();
    static ref AUFLOESEND_BEDINGT_REGEX: Regex = Regex::new(r"Das Recht ist auflösend bedingt").unwrap();
    static ref LÖSCHBAR_BEI_TODESNACHWEIS: Regex = Regex::new(r",? löschbar bei Todesnachweis,?").unwrap();
    static ref ALS_GESAMTBERECHTIGTE_REGEX: Regex = Regex::new(r",? als Gesamtberechtigte gemäß § (\d*) BGB -?").unwrap();
    static ref BELASTETEN_GRUNDSTUECKE: Regex = Regex::new(r"jetzt unter(.*)Nr. (.*) gebucht").unwrap();

    static ref HAT_RECHT_VOR_REGEX_1: Regex = Regex::new(r"(.*) hat Recht vor (.*)").unwrap();
    static ref HAT_RECHT_VOR_REGEX_2: Regex = Regex::new(r"(.*) hat Rang vor (.*)").unwrap();

    static ref IM_RANG_VOR_EINGETRAGEN_REGEX_1: Regex = Regex::new(r"(.*) im Rang vor (.*) eingetragen am (.*)").unwrap();
    static ref IM_RANG_VOR_EINGETRAGEN_REGEX_2: Regex = Regex::new(r"(.*) im Rang vor (.*)").unwrap();                           // TODO -----------
    
    static ref GLEICHRANG_REGEX_1: Regex = Regex::new(r"(.*) im gleichen Rang mit (.*) eingetragen am (.*)").unwrap();
    static ref GLEICHRANG_REGEX_2: Regex = Regex::new(r"(.*) im Gleichrang mit den Rechten (.*) eingetragen am (.*)").unwrap();
    static ref GLEICHRANG_REGEX_3: Regex = Regex::new(r"(.*) im Gleichrang mit (.*) eingetragen am (.*)").unwrap(); // matches
    static ref GLEICHRANG_REGEX_4: Regex = Regex::new(r"ingetragen (.*) im gleichen Rang mit (.*) am (.*)").unwrap();
    static ref GLEICHRANG_REGEX_5: Regex = Regex::new(r"(.*) im Gleichrang mit (.*) am").unwrap();
        
    static ref EINGETRAGEN_AM_REGEX: Regex = Regex::new(r"ingetragen am (\d\d).(\d\d).(\d\d\d\d)").unwrap();
    static ref UEBERTRAGEN_AM_REGEX: Regex = Regex::new(r"hierher übertragen am (\d\d).(\d\d).(\d\d\d\d)").unwrap();
    static ref EINGETRAGEN_AM_REGEX_2: Regex = Regex::new(r"ingetragen (.*) am (\d\d).(\d\d).(\d\d\d\d)").unwrap();
    
    static ref GESAMTHAFT_REGEX: Regex = Regex::new(r"Gesamthaft besteht in (.*)").unwrap();
    static ref GRUNDSCHULD_REGEX_1: Regex = Regex::new(r"Grundschuld (.*) über (.*) für (.*)").unwrap();
    // XXX Euro Grundschuld ohne Brief für ...
    static ref GRUNDSCHULD_REGEX_2: Regex = Regex::new(r"(.*) Grundschuld (.*) für (.*)").unwrap();

    // Nur lastend an Flur 2, Flurstück XX: ...
    static ref NUR_LASTEND_AN_REGEX_1: Regex = Regex::new(r"Nur lastend an (.*):").unwrap();
    static ref NUR_LASTEND_AN_REGEX_2: Regex = Regex::new(r"Nur lastend auf (.*):").unwrap();
    // ...) nur lastend an Flur 2, Flurstück XX für ...
    static ref NUR_LASTEND_AN_REGEX_3: Regex = Regex::new(r"nur lastend an (.*) für").unwrap();
    static ref NUR_LASTEND_AN_REGEX_4: Regex = Regex::new(r"nur lastend auf (.*) für").unwrap();
    static ref WIDERSPRUCH_REGEX: Regex = Regex::new(r"Widerspruch (.*) zugunsten (.*) für (.*) gegen").unwrap();

    // ...mit dem Inhalt des Rechts Abteilung II Nr. XX eingetragen am...
    static ref MIT_DEM_INHALT_DES_RECHTS: Regex = Regex::new(r"mit dem Inhalt des Rechts Abteilung II Nr. (\d*)").unwrap();
    static ref BINDESTRICH_REGEX: Regex = Regex::new(r"- ([[:lower:]])").unwrap();
    static ref BINDESTRICH_REGEX_2: Regex = Regex::new(r"-\nund").unwrap();
}

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

pub fn text_kuerzen_abt2(input: &str, warnung: &mut Vec<String>) -> KurzTextAbt2 {

    let (text_sauber, saetze_clean) = text_saubern(input);
    
    let rechteart = saetze_clean
        .get(0)
        .and_then(|s| s.split("für").nth(0).map(|s| s.trim().to_string()))
        .and_then(|s| klassifiziere_rechteart_abt2(&s));
        
    let eingetragen_am = get_eingetragen_am(&saetze_clean);
    
    // filter: "Mit Bewilligung ... eingetragen am ..."
    // wenn kein "Recht" vorhanden
    let mut saetze = saetze_clean
    .iter()
    .filter(|satz| {
        
        let satz_contains_rvm = 
            satz.contains("Gleichrang") || 
            satz.contains("Rang");
        
        if satz_contains_rvm {
            return true;
        }
        
        let satz_nicht_relevant = 
            satz.starts_with("Gemäß") || 
            satz.starts_with("Unter Bezugnahme auf") || 
            satz.starts_with("Eingetragen") ||
            satz.contains("eingetragen am");

        return !satz_nicht_relevant;
    })
    .map(|s| s.trim().to_string())
    .filter(|s| !AUSSUEBUNG_REGEX.is_match(s.as_str()))
    .filter(|s| !VOLLSTRECKBAR_REGEX.is_match(s.as_str()))
    .filter(|s| !VERERBLICH_REGEX.is_match(s.as_str()))
    .filter(|s| !VON_AMTS_WEGEN_EINGETRAGEN.is_match(s.as_str()))
    .filter(|s| !BELASTETEN_GRUNDSTUECKE.is_match(s.as_str()))
    .filter(|s| s.trim().split_whitespace().count() > 2)
    .filter(|s| if AUFLOESEND_BEDINGT_REGEX.is_match(s.as_str()) { rechteart == Some(RechteArt::Auflassungsvormerkung) } else { true })
    .collect::<Vec<_>>();
    
    let mut gleichrang = Vec::new();
    let mut im_rang_vor = Vec::new();
    let mut rechtsvorrang = Vec::new();
    
    for s in saetze.iter_mut() {
        if HAT_RECHT_VOR_REGEX_1.is_match(s.as_str()) ||
           HAT_RECHT_VOR_REGEX_2.is_match(s.as_str()) {
            // "Abt. X hat Recht vor ..."
            rechtsvorrang.push(s.clone());
            s.clear(); // satz löschen
        } 
        
        if GLEICHRANG_REGEX_1.is_match(s.as_str()) || 
           GLEICHRANG_REGEX_2.is_match(s.as_str()) ||
           GLEICHRANG_REGEX_3.is_match(s.as_str()) ||
           GLEICHRANG_REGEX_4.is_match(s.as_str()) || 
           GLEICHRANG_REGEX_5.is_match(s.as_str()) {
           
            let mut satz_löschen = false;
            
            // ... im gleichen Rang mit ... eingetragen am ...
            for c in GLEICHRANG_REGEX_1.captures_iter(s.as_str()) {
                if let Some(g) = c.get(2) {
                    gleichrang.push(format!("im gleichen Rang mit {}", g.as_str()).trim().to_string());
                    satz_löschen = true;
                }
            }
            
            // ... im Gleichrang mit ... eingetragen am ...
            for c in GLEICHRANG_REGEX_2.captures_iter(s.as_str()) {
                if let Some(g) = c.get(2) {
                    gleichrang.push(format!("im Gleichrang mit {}", g.as_str()).trim().to_string());
                    satz_löschen = true;
                }
            }
            
            if !GLEICHRANG_REGEX_2.is_match(s.as_str()) {
                for c in GLEICHRANG_REGEX_3.captures_iter(s.as_str()) {
                    if let Some(g) = c.get(2) {
                        gleichrang.push(format!("im Gleichrang mit {}", g.as_str()).trim().to_string());
                        satz_löschen = true;
                    }
                }
            }
            
            for c in GLEICHRANG_REGEX_4.captures_iter(s.as_str()) {
                if let Some(g) = c.get(2) {
                    gleichrang.push(format!("im Gleichrang mit {}", g.as_str()).trim().to_string());
                    satz_löschen = true;
                }
            }
            
            if !GLEICHRANG_REGEX_2.is_match(s.as_str()) && !GLEICHRANG_REGEX_3.is_match(s.as_str()) {
                for c in GLEICHRANG_REGEX_5.captures_iter(s.as_str()) {
                    if let Some(g) = c.get(2) {
                        gleichrang.push(format!("im Gleichrang mit {}", g.as_str()).trim().to_string());
                        satz_löschen = true;
                    }
                }
            }
            
            if satz_löschen { 
                s.clear(); 
            }
        }
	    
        if IM_RANG_VOR_EINGETRAGEN_REGEX_1.is_match(s.as_str()) || 
           IM_RANG_VOR_EINGETRAGEN_REGEX_2.is_match(s.as_str()) { 
            
            let mut satz_löschen = false;
            
            // ... im Rang vor ... eingetragen am ...
            for c in IM_RANG_VOR_EINGETRAGEN_REGEX_1.captures_iter(s.as_str()) {
                if let Some(g) = c.get(2) {
                    im_rang_vor.push(format!("im Rang vor {}", g.as_str()).trim().to_string());
                    satz_löschen = true;
                }
            }
            
            if !IM_RANG_VOR_EINGETRAGEN_REGEX_1.is_match(s.as_str()) {
                for c in IM_RANG_VOR_EINGETRAGEN_REGEX_2.captures_iter(s.as_str()) {
		            if let Some(g) = c.get(2) {
		                im_rang_vor.push(format!("im Rang vor {}", g.as_str()).trim().to_string());
		                satz_löschen = true;
		            }
		        }
            
            }
            
            if satz_löschen { 
                s.clear(); 
            }
        } 
        
    }
    
    for g in gleichrang.iter_mut() {
        *g = g.split("am").nth(0).map(|s| s.trim().to_string()).unwrap_or(g.clone());
        *g = g.replace("eingetragen", "");
        *g = g.trim().to_string();
    }
	
    gleichrang.sort();
    gleichrang.dedup();
    
    // Mehr als ein Rangvermerk pro Recht deutet üblicherweise auf einen Fehler
    // beim mergen von Rechten hin
    if gleichrang.len() > 1 {
        warnung
        .push(format!("Unüblich: Mehr als ein Rangvermerk: {:?}", gleichrang));
    }
    
    if im_rang_vor.len() > 1 {
        warnung
        .push(format!("Unüblich: Mehr als ein Rangvermerk: {:?}", im_rang_vor));
    }
    
    saetze.retain(|s| !s.is_empty());
    
    let mut gekuerzt = saetze
    .into_iter()
    .map(|s| s.trim().to_string())
    .collect::<Vec<_>>()
    .join(". ")
    .trim()
    .to_string();
    
    let mut kurztext_rangvermerk = String::new();
    
    if !im_rang_vor.is_empty() {
        if !gekuerzt.ends_with(" ") { gekuerzt.push_str(" ") };
        gekuerzt.push_str(&im_rang_vor.join(", "));
        kurztext_rangvermerk.push_str(&im_rang_vor.join(", "));
    }
    
    if !gleichrang.is_empty() {
        if gekuerzt.ends_with(" ") { 
            gekuerzt = gekuerzt[..gekuerzt.len() - 1].to_string(); 
        }
        gekuerzt.push_str(", ");
        gekuerzt.push_str(&gleichrang.join(", "));
        kurztext_rangvermerk.push_str(&gleichrang.join(", "));
    }
    
    if !rechtsvorrang.is_empty() {
        if !gekuerzt.ends_with(" ") { gekuerzt.push_str(". ") };
        gekuerzt.push_str(&rechtsvorrang.join(". "));
        gekuerzt.push('.');
        
        if !kurztext_rangvermerk.ends_with(" ") { kurztext_rangvermerk.push_str(" ") };
        kurztext_rangvermerk.push_str(&rechtsvorrang.join(". "));
        kurztext_rangvermerk.push('.');
    }

    let mut nur_lastend_an = Vec::<String>::new();
    if NUR_LASTEND_AN_REGEX_1.is_match(&gekuerzt) || NUR_LASTEND_AN_REGEX_2.is_match(&gekuerzt) {
        
        for c in NUR_LASTEND_AN_REGEX_1.captures_iter(&gekuerzt) {
            if let Some(flurstueck) = c.get(1).map(|s| s.as_str().trim().to_string()) {
                nur_lastend_an.push(flurstueck);
            }
        }
        
        for c in NUR_LASTEND_AN_REGEX_2.captures_iter(&gekuerzt) {
            if let Some(flurstueck) = c.get(1).map(|s| s.as_str().trim().to_string()) {
                nur_lastend_an.push(flurstueck);
            }
        }
        
        gekuerzt = NUR_LASTEND_AN_REGEX_1.replace_all(&gekuerzt, "").to_string();
        gekuerzt = NUR_LASTEND_AN_REGEX_2.replace_all(&gekuerzt, "").to_string();
    }
    
    if NUR_LASTEND_AN_REGEX_3.is_match(&gekuerzt) || 
       NUR_LASTEND_AN_REGEX_4.is_match(&gekuerzt) {
        
        for c in NUR_LASTEND_AN_REGEX_3.captures_iter(&gekuerzt) {
            if let Some(flurstueck) = c.get(1).map(|s| s.as_str().trim().to_string()) {
                nur_lastend_an.push(flurstueck);
            }
        }
        
        for c in NUR_LASTEND_AN_REGEX_4.captures_iter(&gekuerzt) {
            if let Some(flurstueck) = c.get(1).map(|s| s.as_str().trim().to_string()) {
                nur_lastend_an.push(flurstueck);
            }
        }
        
        gekuerzt = NUR_LASTEND_AN_REGEX_3.replace_all(&gekuerzt, "für").to_string();
        gekuerzt = NUR_LASTEND_AN_REGEX_4.replace_all(&gekuerzt, "für").to_string();
    }
    
    gekuerzt = gekuerzt.trim().to_string();
    if gekuerzt.starts_with("Beschränkte persönliche Dienstbarkeit") {
        gekuerzt = gekuerzt.replace("Beschränkte persönliche Dienstbarkeit", "BpD");
    }
    gekuerzt = LÖSCHBAR_BEI_TODESNACHWEIS.replace_all(&gekuerzt, "").to_string();
    if !gekuerzt.ends_with(".") {
        gekuerzt.push('.');
    }
    gekuerzt = ALS_GESAMTBERECHTIGTE_REGEX.replace_all(&gekuerzt, "").to_string();

    let rechtsinhaber = saetze_clean
        .get(0)
        .and_then(|s| analysiere_rechteinhaber(s, &rechteart));
    
    KurzTextAbt2 {
        text_sauber: text_sauber,
        saetze: saetze_clean,
        gekuerzt,
        rechtsinhaber,
        rechteart,
        rangvermerk: if kurztext_rangvermerk.is_empty() { None } else { Some(kurztext_rangvermerk) },
        eingetragen_am: if eingetragen_am.is_empty() { None } else { Some(eingetragen_am) },
    }
}


fn get_eingetragen_am(saetze_clean: &Vec<String>) -> String {
    
    let mut eingetragen_am = String::new();
    
    for s in saetze_clean.iter() {
        for c in EINGETRAGEN_AM_REGEX.captures_iter(s.as_str()) {
            if let (Some(d), Some(m), Some(y)) = (c.get(1), c.get(2), c.get(3)) {
                eingetragen_am = format!("{}.{}.{}", 
                    d.as_str().trim(),
                    m.as_str().trim(),
                    y.as_str().trim(),
                )
            }
        }
        
        for c in EINGETRAGEN_AM_REGEX_2.captures_iter(s.as_str()) {
            if let (Some(d), Some(m), Some(y)) = (c.get(2), c.get(3), c.get(4)) {
                eingetragen_am = format!("{}.{}.{}", 
                    d.as_str().trim(),
                    m.as_str().trim(),
                    y.as_str().trim(),
                )
            }
        }
    }
    
    if eingetragen_am.is_empty() {
        for s in saetze_clean.iter() {
           for c in UEBERTRAGEN_AM_REGEX.captures_iter(s.as_str()) {
                if let (Some(d), Some(m), Some(y)) = (c.get(1), c.get(2), c.get(3)) {
                    eingetragen_am = format!("{}.{}.{}", 
                        d.as_str().trim(),
                        m.as_str().trim(),
                        y.as_str().trim(),
                    )
                }
            }
        }
    }
    
    eingetragen_am
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
    pub gesamthaft: Option<String>,
    pub grundschuld_fuer: Option<String>,
}

pub fn text_kuerzen_abt3(betrag: &str, input: &str, warnung: &mut Vec<String>, fehler: &mut Vec<String>) -> KurzTextAbt3 {
    
    let (text_sauber, saetze_clean) = text_saubern(input);
    
    let betrag = match analysiere_betrag(&betrag, &text_sauber, warnung) {
        Some(s) => s,
        None => {
            fehler.push(format!("Konnte Betrag nicht lesen: {:?}", betrag));
            Betrag { wert: 0, nachkomma: 0, waehrung: Waehrung::Euro }
        }
    };
        
    let eingetragen_am = get_eingetragen_am(&saetze_clean);
    let mut gesamthaft = String::new();
    let mut grundschuld_fuer = String::new();
    
    let mut saetze = saetze_clean
    .iter()
    .filter(|satz| {
        
        let satz_contains_rvm = 
            satz.contains("Gleichrang") || 
            satz.contains("Rang");
        
        if satz_contains_rvm {
            return true;
        }
        
        let satz_nicht_relevant = 
            satz.starts_with("Gemäß") || 
            satz.starts_with("Unter Bezugnahme auf") || 
            satz.starts_with("Eingetragen") ||
            satz.contains("eingetragen am");

        return !satz_nicht_relevant;
    })
    .map(|s| s.trim().to_string())
    .map(|s| VOLLSTRECKBAR_REGEX_2.replace_all(&s, "").trim().to_string())
    .filter(|s| !AUSSUEBUNG_REGEX.is_match(s.as_str()))
    .filter(|s| !VOLLSTRECKBAR_REGEX.is_match(s.as_str()))
    .filter(|s| !VERERBLICH_REGEX.is_match(s.as_str()))
    .filter(|s| s.split_whitespace().count() > 2)
    .filter(|s| !VON_AMTS_WEGEN_EINGETRAGEN.is_match(s.as_str()))
    .collect::<Vec<_>>();
    
    for s in saetze.iter_mut() {
    
        if GESAMTHAFT_REGEX.is_match(s.as_str()) {
            let mut satz_löschen = false;
    
            for c in GESAMTHAFT_REGEX.captures_iter(s.as_str()) {
                if let Some(g) = c.get(1) {
                    gesamthaft = g.as_str().to_string();
                    satz_löschen = true;
                }
            }
            
            if satz_löschen { 
                s.clear(); 
            }
        } 
        
        if GRUNDSCHULD_REGEX_1.is_match(s.as_str()) {
            let mut satz_löschen = false;
    
            for c in GRUNDSCHULD_REGEX_1.captures_iter(s.as_str()) {
                if let Some(g) = c.get(3) {
                    grundschuld_fuer = g.as_str().trim().to_string();
                    satz_löschen = true;
                }
            }
            
            if satz_löschen { 
                s.clear(); 
            }
        } else if GRUNDSCHULD_REGEX_2.is_match(s.as_str()) {
            let mut satz_löschen = false;
    
            for c in GRUNDSCHULD_REGEX_2.captures_iter(s.as_str()) {
                if let Some(g) = c.get(3) {
                    grundschuld_fuer = g.as_str().trim().to_string();
                    satz_löschen = true;
                }
            }
            
            if satz_löschen { 
                s.clear(); 
            }
        }
    }
    
    let mut gekuerzt = saetze
    .into_iter()
    .map(|s| s.trim().to_string())
    .collect::<Vec<_>>()
    .join(". ")
    .trim()
    .to_string();
    
    let waehrung = match betrag.waehrung {
        Waehrung::Euro => "EUR",
        Waehrung::DMark => "DM",
    };
    
    if grundschuld_fuer.is_empty() {
        warnung.push(format!("Grundschuld in Text nicht erkannt - {}", gekuerzt));
    } else {
        let gekuerzt_sub = if gekuerzt.is_empty() { String::new() } else { format!(" {}", gekuerzt) };
        gekuerzt = format!("{} {} Grundschuld für {}{}", formatiere_betrag(&betrag), waehrung, grundschuld_fuer.trim(), gekuerzt_sub.trim());
    }
    
    if !gesamthaft.is_empty() {
        if !gekuerzt.ends_with(" ") { gekuerzt.push_str(" ") };
        gekuerzt.push_str("Gesamthaft besteht in ");
        gekuerzt.push_str(&gesamthaft);
        gekuerzt.push('.');
    }
    
    let schuldenart = klassifiziere_rechteart_abt3(&text_sauber);
    let rechtsinhaber = saetze_clean
        .get(0)
        .and_then(|s| analysiere_rechteinhaber(s, &None));
        
    KurzTextAbt3 {
        text_sauber: text_sauber,
        saetze: saetze_clean,
        gekuerzt,
        betrag,
        gesamthaft: if gesamthaft.is_empty() { None } else { Some(gesamthaft) },
        grundschuld_fuer: if grundschuld_fuer.is_empty() { None } else { Some(grundschuld_fuer) },
        eingetragen_am: if eingetragen_am.is_empty() { None } else { Some(eingetragen_am) },
        rechtsinhaber,
        schuldenart,
    }
}

// 100000 => "100.000,00"
// 1500000 => "1.500.000,00"
pub fn formatiere_betrag(b: &Betrag) -> String {
    
    let letzte_drei_stellen = b.wert % 1000;
    let mut hunderttausender = b.wert / 1000;
    
    let million_prefix = if b.wert > 1_000_000 {
        let millionen = hunderttausender / 1000;
        hunderttausender = hunderttausender % 1000;
        Some(millionen)
    } else {
        None
    };
    
    match million_prefix {
        Some(s) => format!("{}.{:03}.{:03},{:02}", s, hunderttausender, letzte_drei_stellen, b.nachkomma),
        None => {
            if b.wert >= 100_000 {
                format!("{:03}.{:03},{:02}", hunderttausender, letzte_drei_stellen, b.nachkomma)
            } else {
                format!("{}.{:03},{:02}", hunderttausender, letzte_drei_stellen, b.nachkomma)
            }
        },
    }
}

/// Säubert den Text und zerlegt den Text in Sätze
pub fn text_saubern(input: &str) -> (String, Vec<String>) {

    let input = input.replace("G r u n d s c h u l d", "Grundschuld");
    let input = input.replace("o h n e", "ohne");
    let input = input.replace("B r i e f", "Brief");
    let input = input.replace("ü b e r", "über");
    let input = input.replace("E u r o", "Euro");
    let input = input.replace("f ü r", "für");

    let mut zeilen = input.lines().map(|s| s.to_string()).collect::<Vec<_>>();
    
    for zeile in zeilen.iter_mut() {
        if BINDESTRICH_REGEX.is_match(zeile) {
            *zeile = BINDESTRICH_REGEX.replace_all(zeile, "-\n$1").to_string();
            *zeile = BINDESTRICH_REGEX_2.replace_all(zeile, "- und").to_string();
        }
    }
    
    let zeilen = zeilen.iter().flat_map(|z| z.lines().map(|s| s.to_string())).collect::<Vec<_>>();

    let mut text_sauber = String::new();
    for (zeilen_idx, zeile) in zeilen.iter().enumerate() {
        
        let zeile = zeile.trim();
        if zeile.is_empty() {
            continue;
        }
        
        // "-\r\n" ersetzen
        let mut zeile_sauber = if zeile.ends_with("-") && !zeilen.get(zeilen_idx + 1).map(|s| s.starts_with("und")).unwrap_or(false) {
            zeile[..zeile.len() - 1].to_string()
        } else {
            format!("{} ", zeile)
        };

        zeile_sauber = zeile_sauber.replace("Aus- übung", "Ausübung");
        text_sauber.push_str(&zeile_sauber);
    }

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
    
    // Manche Abkürzungen werden versehentlich als Satzendungen erkannt ("Dr.", "v.", etc.)
    let abk = [
        " Dr", // Dr.
        " Prof",
        " Co", // Co. KG
        " v", // "v."
        " URNr", // URNr.
        " Abt", // "Abt."
        " Co", // Co.
        "bzlg", //  bzlg.
        " geb", // geb. 
        " lfd", // lfd.
        " Nr", // Nr.
    ];
    
    let mut saetze_clean = Vec::new();
    let mut letzter_satz = String::new();
    for (s_idx, s) in saetze.iter().enumerate() {

        let endet_mit_abkuerzung = 
            // Satz endet mit Abkürzung oder Zahl: vereinen
            abk.iter().any(|a| s.ends_with(a)) || 
            s.chars().last().map(|c| c.is_numeric()).unwrap_or(false) && 
            // nächster Satz fängt mit Großbuchstaben an: trennen
            !saetze.get(s_idx + 1).and_then(|s| s.trim().chars().nth(0)).map(|fc| fc.is_uppercase()).unwrap_or(false);

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
    
    (text_sauber, saetze_clean)
}

fn analysiere_rechteinhaber(s: &str, rechteart: &Option<RechteArt>) -> Option<String> {

    let mut inhaber = if *rechteart == Some(RechteArt::Widerspruch) {
        WIDERSPRUCH_REGEX
        .captures_iter(s)
        .nth(0)?
        .get(3)
        .map(|s| s.as_str().trim().to_string())
    } else {
        // TODO: "für das Landesamt für"
        let s = if s.contains("amt für") || s.contains("Amt für") {
            
            let mut target = String::new();
            let mut taken = false;
            
            for sp in s.split("für") {
                
                let mut sp = sp.trim().to_string();
                
                if sp.trim().ends_with("amt") { 
                    taken = true;
                    sp.push_str(" für ");
                }
                
                if taken {
                    target.push_str(&sp);
                }
            }
            
            target
        } else {
            s.split("für").last().map(|s| s.trim().to_string())?
        };

        if s.contains("zugunsten") {
            s.split("zugunsten").last().map(|s| s.trim().to_string())
        } else {
            Some(s)
        }
    }?;
    
    // lösche "als Gesamtberechtigte nach ... BGB"
    inhaber = ALS_GESAMTBERECHTIGTE_REGEX.replace_all(&inhaber, "").to_string();
    inhaber = LÖSCHBAR_BEI_TODESNACHWEIS.replace_all(&inhaber, "").to_string();
    inhaber = VOLLSTRECKBAR_REGEX_2.replace_all(&inhaber, "").to_string();
    inhaber = inhaber.trim().to_string();
    
    inhaber = inhaber.split("unter Bezugnahme auf").nth(0).map(|s| s.trim().to_string())?;
    
    if inhaber.starts_with("die") {
        inhaber = inhaber.split("die").nth(1).map(|s| s.trim().to_string())?;
    } else if inhaber.starts_with("der") {
        inhaber = inhaber.split("der").nth(1).map(|s| s.trim().to_string())?;
    } else if inhaber.starts_with("das") {
        inhaber = inhaber.split("das").nth(1).map(|s| s.trim().to_string())?;
    } else if inhaber.starts_with("den") {
        inhaber = inhaber.split("den").nth(1).map(|s| s.trim().to_string())?;
    }
    
    // zugunsten des Berechtigten ...
    if inhaber.starts_with("des Berechtigten") {
        inhaber = inhaber.split("des Berechtigten").nth(1).map(|s| s.trim().to_string())?;
    }
    
    // TODO: jew. Eigentümer: (derzeit Dauer Blatt 398 laufende Nummer 2) - ersetzen???
    // des Berechtigten: rauskürzen?
    // TODO: Schenkenberg Blatt 211, Abt. 2, lfd. Nr. 10
    // TODO: Schenkenberg Blatt 99, Abt. 2, lfd. Nr. 2
    // TODO: Schenkenberg Blatt 319, Abt. 2, lfd. Nr. 5
    // TODO: Schenkenberg Blatt 319, Abt. 2, lfd. Nr. 8
    
    Some(inhaber)
}

lazy_static! {
    static ref KOMMA_REGEX: Regex = Regex::new(r",(.*)").unwrap();
}

fn analysiere_betrag(betrag: &str, text_clean: &str, warnung: &mut Vec<String>) -> Option<Betrag> {
    
    let waehrung = if betrag.contains("EUR") || 
       betrag.contains("Euro") || 
       text_clean.contains("Euro") ||
       text_clean.contains("EUR") {
        Waehrung::Euro
    } else if betrag.contains("DM") || 
       betrag.contains("Mark") || 
       text_clean.contains("Mark") ||
       text_clean.contains("DM") {
        Waehrung::DMark
    } else {
        warnung.push(format!("Konnte Währung nicht bestimmen: {:?}", betrag));
        return None;
    };
    
    let betrag = betrag.replace("Euro", "");
    let betrag = betrag.replace("EUR", "");
    let betrag = betrag.replace("DM", "");
    let betrag = betrag.replace("Mark", "");

    let betrag = betrag.trim();
    
    // Nur "," "." und "0-9" lassen
    for c in betrag.chars() {
        let char_ok = c.is_numeric() || c == '.' || c == ',';
        if !char_ok {
            warnung.push(format!("Unbekanntes Zeichen in Betrag: '{}' in {:?}", c, betrag));
            return None;
        }
    }
    
    let mut betrag = betrag.chars()
        .filter(|c| c.is_numeric() || *c == '.' || *c == ',')
        .collect::<String>();
    
    if betrag.contains(",00") {
        betrag = betrag.replace(",00", "");
    }

    let mut nachkomma = 0;
    
    // Wenn immernoch ein "," im Text ist, ist der Cent-Betrag nicht 0
    if let Some(nk) = KOMMA_REGEX.captures_iter(&betrag).nth(0) {
        warnung.push(format!("Cent-Betrag nicht \",00\" ??? - {:?}", betrag));
        nachkomma = nk.get(1)?.as_str().trim().parse::<usize>().ok()?;
        betrag = KOMMA_REGEX.replace_all(&betrag, "").to_string();
    }
    
    let betrag = betrag.chars()
    .filter(|c| c.is_numeric())
    .collect::<String>();
    
    let wert = betrag.parse::<usize>().ok()?;
    
    Some(Betrag { wert, nachkomma, waehrung })
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

// klassifiziere_rechteart_abt3
fn klassifiziere_rechteart_abt3(input: &str) -> Option<SchuldenArt> {
    if input.contains("Grundschuld") {
        Some(SchuldenArt::Grundschuld)
    } else if input.contains("Hypothek") {
        Some(SchuldenArt::Hypothek)
    } else {
        None
    }
}


fn klassifiziere_rechteart_abt2(input: &str) -> Option<RechteArt> {
    if input.contains("Leitungsrecht") || 
       input.contains("Leitungs-") || 
       input.contains("Leitung") ||
       input.contains("Trinkwasserleitung") ||
       input.contains("leitungsrecht") || 
       input.contains("Ferngasleitungs") ||
       input.contains("Kabelanlagenrecht") {
        if input.contains("Gasleitungsrecht") {
            Some(RechteArt::GasleitungGasreglerstationFerngasltg)
        } else if input.contains("Hochspannungsfreileitung") {
            Some(RechteArt::Hochspannungsleitungsrecht)
        } else {
            Some(RechteArt::LeitungsOderAnlagenrecht)
        }
    
    } else if 
        input.contains("Kabelrecht") || 
        input.contains("Kabeltrassenrecht") {
        Some(RechteArt::Kabelrecht)
    } else if input.contains("Wegerecht") {
        Some(RechteArt::GehWegeFahrOderLeitungsrecht)
    } else if input.contains("Vormerkung zur Sicherung des Anspruchs auf Rückauflassung") {
        Some(RechteArt::Rueckauflassungsvormerkung)
    } else if 
        input.contains("Auflassungsvormerkung") {
        Some(RechteArt::Auflassungsvormerkung)
    } else if 
       input.contains("Vorkaufsrecht") || 
       input.contains("Eigentumsübertragungsvormerkung") { // TODO: richtig?
        Some(RechteArt::Vorkaufsrecht)
    } else if 
        input.contains("Nichteinhaltung der Abstandsflächen") || 
        input.contains("Abstandsflächenrecht") ||
        input.contains("Bepflanzung") ||
        input.contains("Vormerkung zur Sicherung des Ankaufsrechtes") ||
        input.contains("Zugangsrecht") ||
        input.contains("Grundwassermessstellenrecht") {
        Some(RechteArt::SonstigeDabagrechteart)
    } else if MIT_DEM_INHALT_DES_RECHTS.is_match(input) {
        let captures = MIT_DEM_INHALT_DES_RECHTS.captures_iter(input).collect::<Vec<_>>();
        let rechteverweis = captures.get(0).and_then(|cg| cg.get(1)).and_then(|d| d.as_str().parse::<usize>().ok())?;
        Some(RechteArt::SpeziellVormerkung { rechteverweis })
    } else if input.contains("Nießbrauch") {
        Some(RechteArt::Niessbrauchrecht)
    } else if input.contains("Mitbenutzungsrecht") {
            Some(RechteArt::Mitbenutzungsrecht)
    } else if
        input.contains("Benutzungsrecht") || 
        input.contains("Benutzungs-") ||
        input.contains("Benutzung-") {
        Some(RechteArt::Benutzungsrecht)
    } else if input.contains("Widerspruch") {
        Some(RechteArt::Widerspruch)
    } else {
        None
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
    SpeziellVormerkung { rechteverweis: usize },                          // Vormerkung zur Sicherung
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
    DuldungVonFernmeldeanlagen,                                            //     Duldung von Femmeldeanlagen
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
    Zaunerrichtungsverbot,                                                //     Zaunemichtungsverbot
    Zaunrecht,                                                            //     Zaunrecht
    Zustimmungsvorbehalt,                                                 //     Zustimmungsvorbehalt
    Zwangsversteigerungsvermerk,                                          //     Zwangsversteigerungsvermerk
    Zwangsverwaltungsvermerk,                                             //     Zwangsverwaltungsvermerk
}

