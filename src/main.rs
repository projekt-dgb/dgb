// Linux: apt install libwebkit2gtk-4.0-dev, tesseract-ocr, pdftotext

use std::collections::BTreeMap;
use std::path::Path;
use std::fs;
use std::sync::Mutex;
use std::collections::BTreeSet;
use std::path::PathBuf;

use urlencoding::encode;
use web_view::*;
use serde_derive::{Serialize, Deserialize};
use crate::digitalisiere::{
    SeiteParsed, Nebenbeteiligter, NebenbeteiligterExport,
    NebenbeteiligterExtra, NebenbeteiligterTyp, 
    Titelblatt, SeitenTyp, Grundbuch, Fehler,
    Anrede, PdfToTextLayout, Abt1GrundEintragung,
    BvEintrag, BvZuschreibung, BvAbschreibung, 
    Abt1Eintrag, Abt1Veraenderung, Abt1Loeschung,
    Abt2Eintrag, Abt2Veraenderung, Abt2Loeschung,
    Abt3Eintrag, Abt3Veraenderung, Abt3Loeschung,
};
use crate::analysiere::GrundbuchAnalysiert;
use crate::kurztext::{PyBetrag, SchuldenArtPyWrapper, RechteArtPyWrapper};
use pyo3::{Python, PyClass, PyAny, pyclass, pymethods, IntoPy, ToPyObject};

const APP_TITLE: &str = "Digitales Grundbuch";
const GTK_OVERLAY_SCROLLING: &str = "GTK_OVERLAY_SCROLLING";

type FileName = String;

pub mod ui;
pub mod digitalisiere;
pub mod analysiere;
pub mod kurztext;
pub mod pdf;

#[derive(Debug, Clone)]
pub struct RpcData {
    // UI
    pub active_tab: usize,
    pub popover_state: Option<PopoverState>,
    pub open_page: Option<(FileName, u32)>,
    
    pub loaded_files: BTreeMap<FileName, PdfFile>,
    pub back_forward: BTreeMap<FileName, BackForwardBuf>,
    pub loaded_nb: Vec<Nebenbeteiligter>,
    pub loaded_nb_paths: Vec<String>,
    
    pub konfiguration: Konfiguration,
}

impl RpcData {

    pub fn get_passwort(&self) -> Option<String> {
        let pw_file_path = std::env::temp_dir().join("dgb").join("passwort.txt");
        match std::fs::read_to_string(pw_file_path) {
            Ok(o) => return Some(o.trim().to_string()),
            Err(_) => {
            
                let email = &self.konfiguration.server_email;
                let pw = tinyfiledialogs::password_box(
                    &format!("Passwort für {email} eingeben"), 
                    &format!("Bitte geben Sie das Passwort für {email} ein:")
                )?;
                
                if self.konfiguration.passwort_speichern {
                    let _ = std::fs::create_dir_all(std::env::temp_dir().join("dgb"));
                    let _ = std::fs::write(std::env::temp_dir().join("dgb").join("passwort.txt"), pw.clone().as_bytes());
                }
                
                Some(pw)
            }
        }
    }
    
    pub fn is_context_menu_open(&self) -> bool {
        match self.popover_state {
            Some(PopoverState::ContextMenu(_)) => true,
            _ => false,
        }
    }
    
    pub fn loaded_file_has_no_pdf(&self) -> bool {
        let open_file = match self.open_page.clone().and_then(|(file, _)| self.loaded_files.get(&file)) { 
            Some(s) => s,
            None => return true,
        };
        
        open_file.datei.is_none()
    }
}

#[derive(Debug, Copy, PartialEq, PartialOrd, Clone)]
pub enum PopoverState {
    ContextMenu(ContextMenuData),
    Info,
    ExportPdf,
    CreateNewGrundbuch,
    GrundbuchSuchenDialog,
    GrundbuchUploadDialog,
    Configuration(ConfigurationView),
    Help,
}

#[derive(Debug, Copy, PartialEq, PartialOrd, Clone)]
pub enum ConfigurationView {
    Allgemein,
    RegEx,
    TextSaubern,
    Abkuerzungen,
    FlstAuslesen,
    KlassifizierungRechteArt,
    RechtsinhaberAuslesenAbt2,
    RangvermerkAuslesenAbt2,
    TextKuerzenAbt2,
    BetragAuslesenAbt3,
    KlassifizierungSchuldenArtAbt3,
    RechtsinhaberAuslesenAbt3,
    TextKuerzenAbt3,
}

#[derive(Debug, Copy, PartialEq, PartialOrd, Clone)]
pub struct ContextMenuData {
    pub x: f32,
    pub y: f32,
    pub seite_ausgewaehlt: usize,
}

impl RpcData {
    
    pub fn save_state(&mut self, file: &FileName) {
        let state_clone = match self.loaded_files.get(file) {
            Some(s) => s.clone(),
            None => return,
        };
        let mut back_forward = match self.back_forward.get_mut(file) {
            Some(s) => s,
            None => return,
        };
        
        back_forward.last_states.push(state_clone);
        
        if back_forward.last_states.len() > back_forward.max_states {
            back_forward.last_states.rotate_left(1);
            back_forward.last_states.pop();
        }
        
        back_forward.current_index = back_forward.last_states.len().wrapping_sub(1);
    }
    
    pub fn create_diff_save_point(&self, file_name: &FileName, file: PdfFile) {
        let json = match serde_json::to_string_pretty(&file) { Ok(o) => o, Err(_) => return, };
        let _ = std::fs::create_dir_all(&format!("{}/backup/", Konfiguration::backup_dir()));
        let target_path = format!("{}/backup/{}.gbx", Konfiguration::backup_dir(), file_name);
        if !Path::new(&target_path).exists() {
            let _ = std::fs::write(&target_path, json.as_bytes());
        }
    }
    
    pub fn get_changed_files(&self) -> Vec<(String, PdfFile)>  {
        self.loaded_files.iter()
        .filter(|(file_name, lf)| {
            let json = match serde_json::to_string_pretty(&lf) { Ok(o) => o, Err(_) => return true, };
            let _ = std::fs::create_dir_all(&format!("{}/backup/", Konfiguration::backup_dir()));
            let target_path = format!("{}/backup/{}.gbx", Konfiguration::backup_dir(), file_name);
            if let Ok(exist) = std::fs::read_to_string(&target_path) {
                if exist == json { false } else { true }
            } else {
                true
            }
        })
        .map(|(file_name, lf)| (file_name.clone(), lf.clone()))
        .collect()
    }
    
    pub fn undo(&mut self, file_name: &FileName) {
    
    }
    
    pub fn redo(&mut self, file_name: &FileName) {
    
    }
}

#[derive(Debug, Clone)]
pub struct BackForwardBuf {
    max_states: usize,
    last_states: Vec<PdfFile>,
    current_index: usize,
}

impl Default for RpcData {
    fn default() -> Self {
        Self {
            active_tab: 0,
            open_page: None,
            popover_state: None,
            loaded_files: BTreeMap::new(),
            back_forward: BTreeMap::new(),
            loaded_nb: Vec::new(),
            loaded_nb_paths: Vec::new(),
            konfiguration: Konfiguration::neu_laden().unwrap_or(Konfiguration {
                zeilenumbrueche_in_ocr_text: false,
                lefis_analyse_einblenden: false,
                spalten_ausblenden: false,
                vorschau_ohne_geroetet: false,
                
                server_url: default_server_url(),
                server_benutzer: default_server_benutzer(),
                server_email: default_server_email(),
                server_privater_schluessel_base64: None,
                passwort_speichern: true,
                
                regex: BTreeMap::new(),
                flurstuecke_auslesen_script: Vec::new(),
                abkuerzungen_script: Vec::new(),
                text_saubern_script: Vec::new(),
                text_kuerzen_abt2_script: Vec::new(),
                text_kuerzen_abt3_script: Vec::new(),
                rechtsinhaber_auslesen_abt2_script: Vec::new(),
                rechtsinhaber_auslesen_abt3_script: Vec::new(),
                rangvermerk_auslesen_abt2_script: Vec::new(),
                betrag_auslesen_script: Vec::new(),
                klassifiziere_rechteart: Vec::new(),
                klassifiziere_schuldenart: Vec::new(),
            }),
        }
    }
}

fn default_server_url() -> String { format!("https://127.0.0.1") }
fn default_server_email() -> String { format!("max@mustermann.de") }
fn default_server_benutzer() -> String { format!("Max Mustermann") }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfFile {
    // Pfad der zugehörigen .pdf-Datei
    datei: Option<String>,
    // Some(pfad) wenn Datei digital angelegt wurde
    #[serde(default)]
    gbx_datei_pfad: Option<String>,
    #[serde(default)]
    land: Option<String>,
    titelblatt: Titelblatt,
    seitenzahlen: Vec<u32>,
    geladen: BTreeMap<u32, SeiteParsed>,
    analysiert: Grundbuch,
    pdftotext_layout: PdfToTextLayout,
    #[serde(skip, default)]
    icon: Option<PdfFileIcon>,
    
    /// Seitennummern von Seiten, die versucht wurden, geladen zu werden
    #[serde(default)]
    seiten_versucht_geladen: BTreeSet<u32>,
    #[serde(default)]
    seiten_ocr_text: BTreeMap<u32, String>,
    #[serde(default)]
    anpassungen_seite: BTreeMap<usize, AnpassungSeite>,
    #[serde(default)]
    klassifikation_neu: BTreeMap<usize, SeitenTyp>,
    #[serde(default)]
    nebenbeteiligte_dateipfade: Vec<String>,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum PdfFileIcon {
    // Gelbes Warn-Icon
    HatFehler,
    // Halb-grünes Icon
    KeineOrdnungsnummernZugewiesen,
    // Voll-grünes Icon
    AllesOkay,
}

static WARNING_CHECK_PNG: &[u8] = include_bytes!("../src/img/icons8-warning-48.png");    
static HALF_CHECK_PNG: &[u8] = include_bytes!("../src/img/icons8-in-progress-48.png");    
static FULL_CHECK_PNG: &[u8] = include_bytes!("../src/img/icons8-ok-48.png");

impl PdfFileIcon {
    pub fn get_base64(&self) -> String {
        match self {
            PdfFileIcon::HatFehler => format!("data:image/png;base64,{}", base64::encode(&WARNING_CHECK_PNG)),
            PdfFileIcon::KeineOrdnungsnummernZugewiesen => format!("data:image/png;base64,{}", base64::encode(&HALF_CHECK_PNG)),
            PdfFileIcon::AllesOkay => format!("data:image/png;base64,{}", base64::encode(&FULL_CHECK_PNG)),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnpassungSeite {
    pub spalten: BTreeMap<String, Rect>,    
    pub zeilen: Vec<f32>,
}

#[derive(Debug, Clone, Default, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Rect {
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
}

impl Rect {
    pub fn zero() -> Self { Self::default() }
}

impl PdfFile {
    pub fn get_gbx_datei_parent(&self) -> PathBuf {
        let default_parent = Path::new("/");
        match (self.datei.as_ref(), self.gbx_datei_pfad.as_ref()) {
            (Some(pdf), None) | (Some(pdf), Some(_)) => {
                Path::new(&pdf).clone().parent().unwrap_or(&default_parent).to_path_buf()
            },
            (None, Some(gbx)) => {
                Path::new(&gbx).to_path_buf()
            },
            (None, None) => {
                default_parent.to_path_buf()
            }
        }
    }
    
    pub fn get_gbx_datei_pfad(&self) -> PathBuf {
        let file_name = format!("{}_{}", self.titelblatt.grundbuch_von, self.titelblatt.blatt);
        self.get_gbx_datei_parent()
        .join(&format!("{}.gbx", file_name))
    }

    pub fn speichern(&self) {
        let target_output_path = self.get_gbx_datei_pfad();
        let json = match serde_json::to_string_pretty(&self) { Ok(o) => o, Err(_) => return, };
        let _ = std::fs::write(&target_output_path, json.as_bytes());
    }
    
    pub fn get_icon(&self, nb: &[Nebenbeteiligter], konfiguration: &Konfiguration) -> Option<PdfFileIcon> {
                
        if !self.ist_geladen() {
            return None;
        }
        
        if !self.hat_keine_fehler(nb, konfiguration) {
            return Some(PdfFileIcon::HatFehler);
        }
        
        if !self.alle_ordnungsnummern_zugewiesen(nb, konfiguration) {
            return Some(PdfFileIcon::KeineOrdnungsnummernZugewiesen);
        }
        
        Some(PdfFileIcon::AllesOkay)
    }
    
    pub fn ist_geladen(&self) -> bool {
        self.seitenzahlen
        .iter()
        .all(|sz| self.geladen.contains_key(sz) || self.seiten_versucht_geladen.contains(sz))
    }
    
    pub fn hat_keine_fehler(&self, nb: &[Nebenbeteiligter], konfiguration: &Konfiguration) -> bool {
        let analysiert = crate::analysiere::analysiere_grundbuch(&self.analysiert, nb, konfiguration);
        
        self.ist_geladen()
        && analysiert.abt2.iter().all(|e| e.fehler.is_empty())
        && analysiert.abt3.iter().all(|e| e.fehler.is_empty())
    }
    
    pub fn alle_ordnungsnummern_zugewiesen(&self, nb: &[Nebenbeteiligter], konfiguration: &Konfiguration) -> bool {
        let analysiert = crate::analysiere::analysiere_grundbuch(&self.analysiert, nb, konfiguration);

        let any_abt2 = analysiert.abt2.iter()
            .any(|e| e.warnungen.iter().any(|w| w == "Konnte keine Ordnungsnummer finden"));
        
        let any_abt3 = analysiert.abt3.iter()
            .any(|e| e.warnungen.iter().any(|w| w == "Konnte keine Ordnungsnummer finden"));

        self.ist_geladen() && !any_abt2 && !any_abt3
    }

    pub fn get_nebenbeteiligte(&self, konfiguration: &Konfiguration) -> Vec<NebenbeteiligterExport> {
        let mut v = Vec::new();
        
        let analysiert = crate::analysiere::analysiere_grundbuch(&self.analysiert, &[], konfiguration);
        
        for abt2 in &analysiert.abt2 {
            if !abt2.rechtsinhaber.is_empty() {
                v.push(NebenbeteiligterExport {
                    ordnungsnummer: None,
                    recht: format!("{} Blatt {}, Abt. 2/{}", self.titelblatt.grundbuch_von, self.titelblatt.blatt, abt2.lfd_nr),
                    typ: NebenbeteiligterTyp::from_str(&abt2.rechtsinhaber),
                    name: abt2.rechtsinhaber.clone(),
                    extra: NebenbeteiligterExtra::default(),
                });
            }
        }
        
        for abt3 in &analysiert.abt3 {
            if !abt3.rechtsinhaber.is_empty() {
                v.push(NebenbeteiligterExport {
                    ordnungsnummer: None,
                    recht: format!("{} Blatt {}, Abt. 3/{}", self.titelblatt.grundbuch_von, self.titelblatt.blatt, abt3.lfd_nr),
                    typ: NebenbeteiligterTyp::from_str(&abt3.rechtsinhaber),
                    name: abt3.rechtsinhaber.clone(),
                    extra: NebenbeteiligterExtra::default(),
                });
            }
        }
        
        v
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Konfiguration {
    #[serde(default)]
    pub spalten_ausblenden: bool,
    #[serde(default)]
    pub lefis_analyse_einblenden: bool,
    #[serde(default)]
    pub zeilenumbrueche_in_ocr_text: bool,
    #[serde(default)]
    pub vorschau_ohne_geroetet: bool,
    #[serde(default = "default_server_url")]
    pub server_url: String,
    #[serde(default = "default_server_benutzer")]
    pub server_benutzer: String,
    #[serde(default = "default_server_email")]
    pub server_email: String,
    #[serde(default)]
    pub server_privater_schluessel_base64: Option<String>,
    #[serde(default = "default_passwort_speichern")]
    pub passwort_speichern: bool,                
    #[serde(default)]
    pub regex: BTreeMap<String, String>,
    #[serde(default)]
    pub abkuerzungen_script: Vec<String>,
    #[serde(default)]
    pub text_saubern_script: Vec<String>,
    #[serde(default)]
    pub flurstuecke_auslesen_script: Vec<String>,
    #[serde(default)]
    pub text_kuerzen_abt2_script: Vec<String>,
    #[serde(default)]
    pub text_kuerzen_abt3_script: Vec<String>,
    #[serde(default)]
    pub betrag_auslesen_script: Vec<String>,
    #[serde(default)]
    pub rechtsinhaber_auslesen_abt3_script: Vec<String>,
    #[serde(default)]
    pub rechtsinhaber_auslesen_abt2_script: Vec<String>,
    #[serde(default)]
    pub rangvermerk_auslesen_abt2_script: Vec<String>,
    pub klassifiziere_rechteart: Vec<String>,
    pub klassifiziere_schuldenart: Vec<String>,
}

fn default_passwort_speichern() -> bool { 
    true 
}

impl Konfiguration {

    const DEFAULT: &'static str = include_str!("../Konfiguration.json");
    const FILE_NAME: &'static str = "Konfiguration.json";
    
    pub fn backup_dir() -> String {
        dirs::config_dir()
        .and_then(|p| Some(p.join("dgb").to_str()?.to_string()))
        .or(
            std::env::current_exe().ok()
            .and_then(|p| Some(p.parent()?.to_path_buf().join("dgb").to_str()?.to_string()))
        ).unwrap_or(format!("./dgb/"))
    }
    
    pub fn konfiguration_pfad() -> String {
        dirs::config_dir()
        .and_then(|p| Some(p.join("dgb").join(Self::FILE_NAME).to_str()?.to_string()))
        .or(
            std::env::current_exe().ok()
            .and_then(|p| Some(p.parent()?.to_path_buf().join("dgb").join(Self::FILE_NAME).to_str()?.to_string()))
        )
        .unwrap_or(format!("./dgb/{}", Self::FILE_NAME))
    }
    
    pub fn speichern(&self) {
        let _ = serde_json::to_string_pretty(self).ok().and_then(|s| {
            let s = s.replace("\n", "\r\n");
            std::fs::write(&Self::konfiguration_pfad(), &s.as_bytes()).ok()
        });
    }

    pub fn neu_laden() -> Result<Self, String> {

        if !Path::new(&Self::konfiguration_pfad()).exists() {
            let _ = std::fs::write(&Self::konfiguration_pfad(), &Self::DEFAULT.as_bytes()).ok();
        }

        let konfig = match std::fs::read_to_string(&Self::konfiguration_pfad()) {
            Ok(o) => match serde_json::from_str(&o) {
                Ok(o) => o,
                Err(e) => return Err(format!("Fehler in Konfiguration {}: {}", Self::konfiguration_pfad(), e)),
            },
            Err(e) => return Err(format!("Fehler beim Lesen von Konfiguration in {}: {}", Self::konfiguration_pfad(), e)),
        };

        Ok(konfig)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "cmd")]
pub enum Cmd {
    #[serde(rename = "init")]
    Init,
    // Open file dialog for file(s) to load
    #[serde(rename = "load_pdf")]
    LoadPdf,
    #[serde(rename = "create_new_grundbuch")]
    CreateNewGrundbuch,
    #[serde(rename = "grundbuch_anlegen")]
    GrundbuchAnlegen { 
        land: String,
        grundbuch_von: String, 
        amtsgericht: String, 
        blatt: usize, 
    },
    #[serde(rename = "undo")]
    Undo,
    #[serde(rename = "redo")]
    Redo,
    #[serde(rename = "export_nb")]
    ExportNebenbeteiligte,
    #[serde(rename = "import_nb")]
    ImportNebenbeteiligte,
    #[serde(rename = "delete_nb")]
    DeleteNebenbeteiligte,
    #[serde(rename = "export_lefis")]
    ExportLefis,
    #[serde(rename = "export_alle_rechte")]
    ExportAlleRechte,
    #[serde(rename = "export_alle_fehler")]
    ExportAlleFehler,   
    #[serde(rename = "export_alle_abt1")]
    ExportAlleAbt1,
    #[serde(rename = "export_alle_teilbelastungen")]
    ExportAlleTeilbelastungen,    
    #[serde(rename = "open_grundbuch_suchen_dialog")]
    OpenGrundbuchSuchenDialog,
    #[serde(rename = "open_grundbuch_upload_dialog")]
    OpenGrundbuchUploadDialog,  
    #[serde(rename = "grundbuch_exportieren")]
    GrundbuchExportieren {
        was_exportieren: String, 
        exportiere_bv: bool,
        exportiere_abt_1: bool,
        exportiere_abt_2: bool,
        exportiere_abt_3: bool,
        exportiere_geroetete_eintraege: bool,
        exportiere_pdf_leere_seite: bool,
        exportiere_in_eine_einzelne_datei: bool,
    },
    #[serde(rename = "open_configuration")]
    OpenConfiguration,
    #[serde(rename = "set_configuration_view")]
    SetConfigurationView { section_id: String },
    #[serde(rename = "open_info")]
    OpenInfo,
    #[serde(rename = "open_help")]
    OpenHelp,
    #[serde(rename = "open_export_pdf")]
    OpenExportPdf,
    #[serde(rename = "close_file")]
    CloseFile { file_name: String },
    #[serde(rename = "klassifiziere_seite_neu")]
    KlassifiziereSeiteNeu { seite: usize, klassifikation_neu: String },
    #[serde(rename = "check_pdf_image_sichtbar")]
    CheckPdfImageSichtbar,
    #[serde(rename = "toggle_lefis_analyse")]
    ToggleLefisAnalyse,
    #[serde(rename = "check_pdf_for_errors")]
    CheckPdfForErrors,
    #[serde(rename = "search")]
    Search { search_text: String },
    #[serde(rename = "download_gbx")]
    DownloadGbx { 
        download_id: String, 
        target_folder_path: String,
    },
    #[serde(rename = "upload_gbx")]
    UploadGbx {
        commit_title: String,
        commit_msg: String,
    },
    #[serde(rename = "edit_abkuerzungen_script")]
    EditAbkuerzungenScript { script: String },
    #[serde(rename = "edit_text_saubern_script")]
    EditTextSaubernScript { script: String },
    #[serde(rename = "edit_flurstuecke_auslesen_script")]
    EditFlurstueckeAuslesenScript { script: String },
    
    #[serde(rename = "flurstueck_auslesen_script_testen")]
    FlurstueckAuslesenScriptTesten { text: String, bv_nr: String },
    #[serde(rename = "edit_text_kuerzen_abt2_script")]
    EditTextKuerzenAbt2Script { script: String },
    #[serde(rename = "kurztext_abt2_script_testen")]
    KurzTextAbt2ScriptTesten { text: String },
    #[serde(rename = "edit_rechteart_script")]
    EditRechteArtScript { neu: String },
    #[serde(rename = "rechteart_script_testen")]
    RechteArtScriptTesten { text: String },
    #[serde(rename = "edit_rechtsinhaber_auslesen_abt2_script")]
    EditRechtsinhaberAuslesenAbt2Script { neu: String },
    #[serde(rename = "rechtsinhaber_auslesen_abt2_script_testen")]
    RechtsinhaberAuslesenAbt2ScriptTesten { text: String },
    #[serde(rename = "edit_rangvermerk_auslesen_abt2_script")]
    EditRangvermerkAuslesenAbt2Script { neu: String },
    #[serde(rename = "rangvermerk_auslesen_abt2_script_testen")]
    RangvermerkAuslesenAbt2ScriptTesten { text: String },
        
    #[serde(rename = "edit_text_kuerzen_abt3_script")]
    EditTextKuerzenAbt3Script { script: String },
    #[serde(rename = "kurztext_abt3_script_testen")]
    KurzTextAbt3ScriptTesten { text: String },
    #[serde(rename = "edit_betrag_auslesen_script")]
    EditBetragAuslesenScript { neu: String },
    #[serde(rename = "betrag_auslesen_script_testen")]
    BetragAuslesenScriptTesten { text: String },
    #[serde(rename = "edit_schuldenart_script")]
    EditSchuldenArtScript { neu: String },
    #[serde(rename = "schuldenart_script_testen")]
    SchuldenArtScriptTesten { text: String },
    #[serde(rename = "edit_rechtsinhaber_auslesen_abt3_script")]
    EditRechtsinhaberAuslesenAbt3Script { neu: String },
    #[serde(rename = "rechtsinhaber_auslesen_abt3_script_testen")]
    RechtsinhaberAuslesenAbt3ScriptTesten { text: String },
    
    #[serde(rename = "teste_regex")]
    TesteRegex { regex_id: String, text: String },
    #[serde(rename = "edit_regex_key")]
    EditRegexKey { old_key: String, new_key: String },
    #[serde(rename = "edit_regex_value")]
    EditRegexValue { key: String, value: String },
    #[serde(rename = "insert_regex")]
    InsertRegex { regex_key: String },
    #[serde(rename = "regex_loeschen")]
    RegexLoeschen { regex_key: String },

    // Check whether a "{file_name}".json with analyzed texts exists
    #[serde(rename = "check_for_pdf_loaded")]
    CheckForPdfLoaded { file_path: String, file_name: String },
    // Check whether a "{file_name}".json with analyzed texts exists
    #[serde(rename = "check_for_image_loaded")]
    CheckForImageLoaded { file_path: String, file_name: String },

    #[serde(rename = "edit_text")]
    EditText { path: String, new_value: String },
    #[serde(rename = "eintrag_neu")]
    EintragNeu { path: String },
    #[serde(rename = "eintrag_roeten")]
    EintragRoeten { path: String },
    #[serde(rename = "eintrag_loeschen")]
    EintragLoeschen { path: String },
    #[serde(rename = "open_context_menu")]
    OpenContextMenu { x: f32, y: f32, seite: usize },
    #[serde(rename = "close_pop_over")]
    ClosePopOver,

    #[serde(rename = "reset_ocr_selection")]
    ResetOcrSelection,
    #[serde(rename = "select_ocr")]
    SelectOcr {
        file_name: String,
        page: usize,
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
        page_width: f32,
        page_height: f32,
    },
    #[serde(rename = "resize_column")]
    ResizeColumn {
        direction: String,
        column_id: String,
        x: f32,
        y: f32,
    },
    #[serde(rename = "toggle_checkbox")]
    ToggleCheckBox { checkbox_id: String },
    #[serde(rename = "reload_grundbuch")]
    ReloadGrundbuch,
    #[serde(rename = "zeile_neu")]
    ZeileNeu { file: String, page: usize, y: f32 },
    #[serde(rename = "zeile_loeschen")]
    ZeileLoeschen { file: String, page: usize, zeilen_id: usize },
    #[serde(rename = "bv_eintrag_typ_aendern")]
    BvEintragTypAendern { path: String, value: String },
    
    #[serde(rename = "copy_text_to_clipboard")]
    CopyTextToClipboard { text: String },
    
    // UI stuff
    #[serde(rename = "set_active_ribbon_tab")]
    SetActiveRibbonTab { new_tab: usize },
    #[serde(rename = "set_open_file")]
    SetOpenFile { new_file: String },
    #[serde(rename = "set_open_page")]
    SetOpenPage { active_page: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum GrundbuchSucheResponse {
    #[serde(rename = "ok")]
    StatusOk(GrundbuchSucheOk),
    #[serde(rename = "error")]
    StatusErr(GrundbuchSucheError)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrundbuchSucheOk {
    pub ergebnisse: Vec<GrundbuchSucheErgebnis>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrundbuchSucheErgebnis {
    pub titelblatt: Titelblatt,
    pub ergebnis_text: String,
    pub gefunden_text: String,
    pub download_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrundbuchSucheError {
    pub code: usize,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LefisDateiExport {
    pub titelblatt: Titelblatt,
    pub rechte: GrundbuchAnalysiert,
}
            
fn webview_cb<'a>(webview: &mut WebView<'a, RpcData>, arg: &str, data: &mut RpcData) {
        
    let arg = match serde_json::from_str::<Cmd>(arg) {
        Ok(arg) => arg,
        Err(e) => { 
            return; 
        },
    };
        
    match &arg {
        Cmd::Init => { 
            webview.eval(&format!("replaceEntireScreen(`{}`)", ui::render_entire_screen(data))); 
        },
        Cmd::LoadPdf => {
                       
            let file_dialog_result = tinyfiledialogs::open_file_dialog_multi(
                "Grundbuchblatt-PDF Datei(en) auswählen", 
                "~/", 
                Some((&["*.pdf", "*.gbx"], "Grundbuchblatt")),
            ); 
            
            let dateien = match file_dialog_result {
                Some(f) => f,
                None => return,
            };
            
            // Nur PDF-oder GBX-Dateien laden
            let dateien = dateien
            .iter()
            .filter_map(|dateipfad| {
                let dateiendung = Path::new(dateipfad).extension()?;
                if dateiendung == "pdf" || dateiendung == "gbx" {
                    Some(dateipfad)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
    
            let mut pdf_zu_laden = Vec::new();
            
            for d in dateien.iter() {
                                
                let datei_bytes = match std::fs::read(d) {
                    Ok(o) => o,
                    Err(e) => {
                        continue;
                    }
                };
                
                if let Some(mut grundbuch_json_parsed) = String::from_utf8(datei_bytes.clone()).ok().and_then(|s| serde_json::from_str::<PdfFile>(&s).ok()) {
                    
                    let file_name = format!("{}_{}", grundbuch_json_parsed.titelblatt.grundbuch_von, grundbuch_json_parsed.titelblatt.blatt);

                    for nb_datei in grundbuch_json_parsed.nebenbeteiligte_dateipfade.iter() {
                        if let Some(mut nb) = std::fs::read_to_string(&nb_datei).ok().map(|fs| parse_nb(&fs)) {
                            data.loaded_nb.append(&mut nb);
                            data.loaded_nb.sort_by(|a, b| a.name.cmp(&b.name));
                            data.loaded_nb.dedup();
                            data.loaded_nb_paths.push(nb_datei.clone());
                            data.loaded_nb_paths.sort();
                            data.loaded_nb_paths.dedup();
                        }
                    }
                                    
                    data.loaded_files.insert(file_name.clone(), grundbuch_json_parsed.clone());
                    data.create_diff_save_point(&file_name.clone(), grundbuch_json_parsed.clone());
                    pdf_zu_laden.push(grundbuch_json_parsed);  
                    if data.open_page.is_none() {
                        data.open_page = Some((file_name.clone(), 2));
                    }
                } else {
                
                    let mut seitenzahlen = match digitalisiere::lese_seitenzahlen(&datei_bytes) {
                        Ok(o) => o,
                        Err(e) => {
                            continue;
                        },
                    };
                    
                    let max_sz = seitenzahlen.iter().max().cloned().unwrap_or(0);

                    let titelblatt = match digitalisiere::lese_titelblatt(&datei_bytes) {
                        Ok(o) => o,
                        Err(_) => {
                            continue;
                        },
                    };

                    let default_parent = Path::new("/");
                    let output_parent = Path::new(&d).parent().unwrap_or(&default_parent).to_path_buf();
                    let file_name = format!("{}_{}", titelblatt.grundbuch_von, titelblatt.blatt);
                    let cache_output_path = output_parent.clone().join(&format!("{}.cache.gbx", file_name));
                    let target_output_path = output_parent.clone().join(&format!("{}.gbx", file_name));
                    
                    // Lösche Titelblattseite von Seiten, die gerendert werden müssen
                    seitenzahlen.remove(0);
                    
                    let datei_bytes_clone = datei_bytes.clone();
                    let titelblatt_clone = titelblatt.clone();
                    let seitenzahlen_clone = seitenzahlen.clone();
                    
                    let mut pdf_parsed = PdfFile {
                        datei: Some(d.to_string()),
                        gbx_datei_pfad: None,
                        titelblatt,
                        icon: None,
                        land: None,
                        seiten_ocr_text: BTreeMap::new(),
                        seitenzahlen: seitenzahlen.clone(),
                        klassifikation_neu: BTreeMap::new(),
                        pdftotext_layout: PdfToTextLayout::default(),
                        geladen: BTreeMap::new(),
                        analysiert: Grundbuch::default(),
                        nebenbeteiligte_dateipfade: Vec::new(),
                        anpassungen_seite: BTreeMap::new(),
                        seiten_versucht_geladen: BTreeSet::new(),
                    };
                                    
                    if let Some(cached_pdf) = std::fs::read_to_string(&cache_output_path).ok().and_then(|s| serde_json::from_str(&s).ok()) {
                        pdf_parsed = cached_pdf;
                    }
                    
                    if let Some(target_pdf) = std::fs::read_to_string(&target_output_path).ok().and_then(|s| serde_json::from_str::<PdfFile>(&s).ok()) {
                        pdf_parsed = target_pdf.clone();
                        data.create_diff_save_point(&file_name, target_pdf.clone());
                    }
                
                    for nb_datei in pdf_parsed.nebenbeteiligte_dateipfade.iter() {
                        if let Some(mut nb) = std::fs::read_to_string(&nb_datei).ok().map(|fs| parse_nb(&fs)) {
                            data.loaded_nb.append(&mut nb);
                            data.loaded_nb.sort_by(|a, b| a.name.cmp(&b.name));
                            data.loaded_nb.dedup();
                            data.loaded_nb_paths.push(nb_datei.clone());
                            data.loaded_nb_paths.sort();
                            data.loaded_nb_paths.dedup();
                        }
                    }
                                    
                    let json = match serde_json::to_string_pretty(&pdf_parsed) { Ok(o) => o, Err(_) => continue, };
                    let _ = std::fs::write(&cache_output_path, json.as_bytes());
                    data.loaded_files.insert(file_name.clone(), pdf_parsed.clone());
                    pdf_zu_laden.push(pdf_parsed);  
                    if data.open_page.is_none() {
                        data.open_page = Some((file_name.clone(), 2));
                    }
                }
            }
            
            let html_inner = ui::render_entire_screen(data);
            webview.eval(&format!("replaceEntireScreen(`{}`)", html_inner));
            webview.eval("startCheckingForPdfErrors()");

            for pdf_parsed in &pdf_zu_laden {
                let output_parent = pdf_parsed.get_gbx_datei_parent();
                let file_name = format!("{}_{}", pdf_parsed.titelblatt.grundbuch_von, pdf_parsed.titelblatt.blatt);
                let cache_output_path = output_parent.clone().join(&format!("{}.cache.gbx", file_name));
                webview.eval(&format!("startCheckingForPageLoaded(`{}`, `{}`)", cache_output_path.display(), file_name));
            }
                        
            digitalisiere_dateien(pdf_zu_laden);
        },
        Cmd::CreateNewGrundbuch => {
            data.popover_state = Some(PopoverState::CreateNewGrundbuch);
            webview.eval(&format!("replacePopOver(`{}`)", ui::render_popover_content(data)));
        },
        Cmd::OpenGrundbuchSuchenDialog => {
            data.popover_state = Some(PopoverState::GrundbuchSuchenDialog);
            webview.eval(&format!("replacePopOver(`{}`)", ui::render_popover_content(data)));
        },
        Cmd::OpenGrundbuchUploadDialog => {
            if data.loaded_files.is_empty() {
                return;
            }
            data.popover_state = Some(PopoverState::GrundbuchUploadDialog);
            webview.eval(&format!("replacePopOver(`{}`)", ui::render_popover_content(data)));
        },
        Cmd::GrundbuchAnlegen { land, grundbuch_von, amtsgericht, blatt } => {

            let file_dialog_result = tinyfiledialogs::select_folder_dialog(
                ".gbx-Datei speichern unter...", 
                "~/",
            );
            
            let gbx_folder = match file_dialog_result {
                Some(f) => f,
                None => return,
            };
            
            let file_name = format!("{}_{}", grundbuch_von, blatt);

            let mut pdf_parsed = PdfFile {
                datei: None,
                gbx_datei_pfad: Some(gbx_folder),
                titelblatt: Titelblatt {
                    amtsgericht: amtsgericht.trim().to_string().clone(),
                    grundbuch_von: grundbuch_von.trim().to_string().clone(),
                    blatt: blatt.clone(),
                },
                icon: None,
                land: Some(land.trim().to_string()),
                seiten_ocr_text: BTreeMap::new(),
                seitenzahlen: Vec::new(),
                klassifikation_neu: BTreeMap::new(),
                pdftotext_layout: PdfToTextLayout::default(),
                geladen: BTreeMap::new(),
                analysiert: Grundbuch::default(),
                nebenbeteiligte_dateipfade: Vec::new(),
                anpassungen_seite: BTreeMap::new(),
                seiten_versucht_geladen: BTreeSet::new(),
            };
            pdf_parsed.speichern();
            data.loaded_files.insert(file_name.clone(), pdf_parsed.clone());
            if data.open_page.is_none() {
                data.open_page = Some((file_name.clone(), 2));
            }
            data.popover_state = None;
            webview.eval(&format!("replaceEntireScreen(`{}`)",  ui::render_entire_screen(data)));
            webview.eval("startCheckingForPdfErrors()");
        },
        Cmd::Search { search_text } => {
        
            let passwort = match data.get_passwort() {
                Some(s) => s,
                None => return,
            };
            
            let server_url = &data.konfiguration.server_url;
            let server_benutzer = urlencoding::encode(&data.konfiguration.server_benutzer);
            let server_email = urlencoding::encode(&data.konfiguration.server_email);
            let pkey = data.konfiguration.server_privater_schluessel_base64
                .as_ref().map(|b| format!("&pkey={b}")).unwrap_or_default();
            let search_text = urlencoding::encode(&search_text);
            let url = format!("{server_url}/suche/{search_text}&benutzer={server_benutzer}&email={server_email}&passwort={passwort}{pkey}");
                        
            let resp = match reqwest::blocking::get(&url) {
                Ok(s) => s,
                Err(e) => {
                    webview.eval(&format!("replaceSuchergebnisse(`{}`)", ui::render_suchergebnisse_liste(&GrundbuchSucheResponse::StatusErr(GrundbuchSucheError {
                        code: 0,
                        text: format!("HTTP GET {url}: {}", e),
                    }))));
                    return;
                },
            };
            
            let json = match resp.json::<GrundbuchSucheResponse>() {
                Ok(s) => s,
                Err(e) => {
                    webview.eval(&format!("replaceSuchergebnisse(`{}`)", ui::render_suchergebnisse_liste(&GrundbuchSucheResponse::StatusErr(GrundbuchSucheError {
                        code: 0,
                        text: format!("HTTP GET {url}: {}", e),
                    }))));
                    return;
                },
            };
        
            webview.eval(&format!("replaceSuchergebnisse(`{}`)", ui::render_suchergebnisse_liste(&json)));
        },
        Cmd::DownloadGbx { download_id, target_folder_path } => {
            
            let passwort = match data.get_passwort() {
                Some(s) => s,
                None => return,
            };
            
            let server_url = &data.konfiguration.server_url;
            let server_benutzer = urlencoding::encode(&data.konfiguration.server_benutzer);
            let server_email = urlencoding::encode(&data.konfiguration.server_email);
            let pkey = data.konfiguration.server_privater_schluessel_base64
                .as_ref().map(|b| format!("&pkey={b}")).unwrap_or_default();
            let download_id = urlencoding::encode(&download_id);
            let url = format!("{server_url}/download/{download_id}&benutzer={server_benutzer}&email={server_email}&passwort={passwort}{pkey}");
            
            println!("HTTP GET {url}");

            let resp = match reqwest::blocking::get(&url) {
                Ok(s) => s,
                _ => return,
            };
            
            let json = match resp.json::<PdfFile>() {
                Ok(s) => s,
                _ => return,
            };

            let file_name = format!("{}_{}", json.titelblatt.grundbuch_von, json.titelblatt.blatt);
            let _ = std::fs::write(&format!("{target_folder_path}/{file_name}.gbx"), serde_json::to_string_pretty(&json).unwrap_or_default());
            data.loaded_files.insert(file_name.clone(), json.clone());
            data.create_diff_save_point(&file_name, json.clone());
            data.popover_state = None;
            webview.eval(&format!("replaceEntireScreen(`{}`)",  ui::render_entire_screen(data)));
            webview.eval("startCheckingForPdfErrors()");
        },
        Cmd::UploadGbx { commit_title, commit_msg } => {
            // TODO
            // data.reset_diff_backup_files();
            data.popover_state = None;
            webview.eval(&format!("replaceEntireScreen(`{}`)",  ui::render_entire_screen(data)));
        },    
        Cmd::CheckForImageLoaded { file_path, file_name } => {
            // TODO
            webview.eval(&format!("stopCheckingForImageLoaded(`{}`)", file_name));
        },
        Cmd::CheckPdfImageSichtbar => {
            
            match data.open_page.clone() {
                Some(_) => { },
                None => return,
            }
            
            let open_file = match data.open_page.clone() {
                Some(s) => s,
                None => { return; },
            };
            
            let file = match data.loaded_files.get(&open_file.0) {
                Some(s) => s,
                None => { return; },
            };
            
            let max_seitenzahl = file.seitenzahlen.iter().copied().max().unwrap_or(0);
            
            let temp_ordner = std::env::temp_dir()
            .join(&format!("{gemarkung}/{blatt}", gemarkung = file.titelblatt.grundbuch_von, blatt = file.titelblatt.blatt));
            
            let temp_pdf_pfad = temp_ordner.clone().join("temp.pdf");
            let pdftoppm_output_path = if data.konfiguration.vorschau_ohne_geroetet {
                temp_ordner.clone().join(format!("page-clean-{}.png", crate::digitalisiere::formatiere_seitenzahl(open_file.1, max_seitenzahl)))
            } else {
                temp_ordner.clone().join(format!("page-{}.png", crate::digitalisiere::formatiere_seitenzahl(open_file.1, max_seitenzahl)))
            };
            
            if !pdftoppm_output_path.exists() {
                if let Some(pdf) = file.datei.as_ref() {
                    if let Ok(o) = std::fs::read(&pdf) {
                        let _ = crate::digitalisiere::konvertiere_pdf_seite_zu_png_prioritaet(
                            &o, 
                            &[open_file.1], 
                            &file.titelblatt, 
                            !data.konfiguration.vorschau_ohne_geroetet
                        );
                    }
                }
            }
        
            webview.eval(&format!("replacePdfImage(`{}`)", ui::render_pdf_image(data)));
        },
        Cmd::CheckForPdfLoaded { file_path, file_name } => {
                        
            let default_parent = Path::new("/");
            let output_parent = Path::new(&file_path).clone().parent().unwrap_or(&default_parent).to_path_buf();
            let cache_output_path = output_parent.clone().join(&format!("{}.cache.gbx", file_name));
            let target_output_path = output_parent.clone().join(&format!("{}.gbx", file_name));
            
            let mut pdf_parsed: PdfFile = match std::fs::read_to_string(&cache_output_path).ok().and_then(|s| serde_json::from_str(&s).ok()) {
                Some(s) => s,
                None => { return; },
            };
            
            data.loaded_files.insert(file_name.clone(), pdf_parsed.clone());
            
            webview.eval(&format!("replacePageList(`{}`);", ui::render_page_list(&data)));
            
            if pdf_parsed.ist_geladen() {
                let _ = std::fs::remove_file(&cache_output_path);
                if data.open_page.is_none() {
                    data.open_page = Some((file_name.clone(), 2));
                    webview.eval(&format!("replaceEntireScreen(`{}`)", ui::render_entire_screen(data))); 
                } else if data.open_page.as_ref().map(|s| s.0.clone()).unwrap_or_default() == *file_name {
                    webview.eval(&format!("replaceEntireScreen(`{}`)", ui::render_entire_screen(data))); 
                }
                webview.eval(&format!("stopCheckingForPageLoaded(`{}`)", file_name));
            }
        },
        Cmd::EditText { path, new_value } => {
                        
            fn get_mut_or_insert_last<'a, T>(vec: &'a mut Vec<T>, index: usize, default_value: T) -> &'a mut T {
                let vec_len = vec.len();
                if index + 1 > vec_len {
                    vec.push(default_value);
                    let vec_len = vec.len();
                    &mut vec[vec_len - 1]
                } else {
                    &mut vec[index]
                }
            }
            
            use crate::digitalisiere::FlurstueckGroesse;
            
            let new_value = new_value
                .lines()
                .map(|l| l.replace("\u{00a0}", " "))
                .collect::<Vec<_>>()
                .join("\r\n");

            let split = path.split(":").collect::<Vec<_>>();
            
            let section = match split.get(0) {
                Some(s) => s,
                None => return,
            };
            
            let row = match split.get(1).and_then(|s| s.parse::<usize>().ok()) {
                Some(s) => s,
                None => return,
            };
            
            let cell = match split.get(2) {
                Some(s) => s,
                None => return,
            };
            
            let open_file = match data.open_page.clone().and_then(|(file, _)| data.loaded_files.get_mut(&file)) { 
                Some(s) => s,
                None => return,
            };
                        
            match (*section, *cell) {
                ("bv", "lfd-nr") => {
                    let new_value = match new_value.parse::<usize>().ok() {
                        Some(s) => s,
                        None => return,
                    };
                    let mut bv_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.bestandsverzeichnis.eintraege, 
                        row, 
                        BvEintrag::neu(row + 1)
                    );
                    bv_eintrag.set_lfd_nr(new_value.clone().into());
                },
                ("bv", "bisherige-lfd-nr") => {
                    let new_value = match new_value.parse::<usize>().ok() {
                        Some(s) => Some(s),
                        None => None,
                    };
                    let mut bv_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.bestandsverzeichnis.eintraege, 
                        row, 
                        BvEintrag::neu(row + 1)
                    );
                    bv_eintrag.set_bisherige_lfd_nr(new_value.clone().into());
                },
                ("bv", "zu-nr") => {
                    let mut bv_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.bestandsverzeichnis.eintraege, 
                        row, 
                        BvEintrag::neu(row + 1)
                    );
                    bv_eintrag.set_zu_nr(new_value.clone().into());
                },
                ("bv", "recht-text") => {
                    let mut bv_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.bestandsverzeichnis.eintraege, 
                        row, 
                        BvEintrag::neu(row + 1)
                    );
                    bv_eintrag.set_recht_text(new_value.clone().into());
                },
                ("bv", "gemarkung") => {
                    let mut bv_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.bestandsverzeichnis.eintraege, 
                        row, 
                        BvEintrag::neu(row + 1)
                    );
                    bv_eintrag.set_gemarkung(if new_value.trim().is_empty() { 
                        None 
                    } else { 
                        Some(new_value.clone().into()) 
                    });
                },
                ("bv", "flur") => {
                    let new_value = match new_value.parse::<usize>().ok() {
                        Some(s) => s,
                        None => return,
                    };
                    let mut bv_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.bestandsverzeichnis.eintraege, 
                        row, 
                        BvEintrag::neu(row + 1)
                    );
                    bv_eintrag.set_flur(new_value.clone().into());
                },
                ("bv", "flurstueck") => {
                    let mut bv_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.bestandsverzeichnis.eintraege, 
                        row, 
                        BvEintrag::neu(row + 1)
                    );
                    bv_eintrag.set_flurstueck(new_value.clone().into());
                },
                ("bv", "bezeichnung") => {
                    let mut bv_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.bestandsverzeichnis.eintraege, 
                        row, 
                        BvEintrag::neu(row + 1)
                    );
                    bv_eintrag.set_bezeichnung(new_value.clone().into());
                },
                ("bv", "groesse") => {
                    let new_value = match new_value.parse::<u64>().ok() {
                        Some(s) => Some(s),
                        None => None,
                    };
                    let mut bv_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.bestandsverzeichnis.eintraege, 
                        row, 
                        BvEintrag::neu(row + 1)
                    );
                    bv_eintrag.set_groesse(FlurstueckGroesse::Metrisch { m2: new_value });
                },
                ("bv-zuschreibung", "bv-nr") => {
                    let mut bv_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.bestandsverzeichnis.zuschreibungen, 
                        row, 
                        BvZuschreibung::default()
                    );
                    
                    bv_eintrag.bv_nr = new_value.clone().into();
                },
                ("bv-zuschreibung", "text") => {
                    let mut bv_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.bestandsverzeichnis.zuschreibungen, 
                        row, 
                        BvZuschreibung::default()
                    );
                    bv_eintrag.text = new_value.clone().into();
                },
                ("bv-abschreibung", "bv-nr") => {
                    let mut bv_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.bestandsverzeichnis.abschreibungen, 
                        row, 
                        BvAbschreibung::default()
                    );
                    bv_eintrag.bv_nr = new_value.clone().into();
                },
                ("bv-abschreibung", "text") => {
                    let mut bv_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.bestandsverzeichnis.abschreibungen, 
                        row, 
                        BvAbschreibung::default()
                    );
                    bv_eintrag.text = new_value.clone().into();
                },
                
                ("abt1", "lfd-nr") => {
                    let new_value = match new_value.parse::<usize>().ok() {
                        Some(s) => s,
                        None => return,
                    };
                    let mut abt1_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt1.eintraege, 
                        row, 
                        Abt1Eintrag::new(row + 1)
                    );
                    abt1_eintrag.set_lfd_nr(new_value.clone().into());
                },
                ("abt1", "eigentuemer") => {
                    let mut abt1_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt1.eintraege, 
                        row, 
                        Abt1Eintrag::new(row + 1)
                    );
                    abt1_eintrag.set_eigentuemer(new_value.clone().into());
                },
                ("abt1-grundlage-eintragung", "bv-nr") => {
                    let mut abt1_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt1.grundlagen_eintragungen, 
                        row, 
                        Abt1GrundEintragung::new()
                    );
                    abt1_eintrag.bv_nr = new_value.clone().into();
                },
                ("abt1-grundlage-eintragung", "text") => {
                    let mut abt1_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt1.grundlagen_eintragungen, 
                        row, 
                        Abt1GrundEintragung::new()
                    );
                    abt1_eintrag.text = new_value.clone().into();
                },
                ("abt1-veraenderung", "lfd-nr") => {
                    let mut abt1_veraenderung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt1.veraenderungen, 
                        row, 
                        Abt1Veraenderung::default()
                    );
                    abt1_veraenderung.lfd_nr = new_value.clone().into();
                },
                ("abt1-veraenderung", "text") => {
                    let mut abt1_veraenderung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt1.veraenderungen, 
                        row, 
                        Abt1Veraenderung::default()
                    );
                    abt1_veraenderung.text = new_value.clone().into();
                },
                ("abt1-loeschung", "lfd-nr") => {
                    let mut abt1_loeschung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt1.loeschungen, 
                        row, 
                        Abt1Loeschung::default()
                    );
                    abt1_loeschung.lfd_nr = new_value.clone().into();
                },
                ("abt1-loeschung", "text") => {
                    let mut abt1_loeschung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt1.loeschungen, 
                        row, 
                        Abt1Loeschung::default()
                    );
                    abt1_loeschung.text = new_value.clone().into();
                },
                
                ("abt2", "lfd-nr") => {
                    let new_value = match new_value.parse::<usize>().ok() {
                        Some(s) => s,
                        None => return,
                    };
                    let mut abt2_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt2.eintraege, 
                        row, 
                        Abt2Eintrag::new(row + 1)
                    );
                    abt2_eintrag.lfd_nr = new_value.clone().into();
                },
                ("abt2", "bv-nr") => {
                    let mut abt2_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt2.eintraege, 
                        row, 
                        Abt2Eintrag::new(row + 1)
                    );
                    abt2_eintrag.bv_nr = new_value.clone().into();
                },
                ("abt2", "text") => {
                    let mut abt2_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt2.eintraege, 
                        row, 
                        Abt2Eintrag::new(row + 1)
                    );
                    abt2_eintrag.text = new_value.clone().into();
                },
                ("abt2-veraenderung", "lfd-nr") => {
                    let mut abt2_veraenderung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt2.veraenderungen, 
                        row, 
                        Abt2Veraenderung::default()
                    );
                    abt2_veraenderung.lfd_nr = new_value.clone().into();
                },
                ("abt2-veraenderung", "text") => {
                    let mut abt2_veraenderung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt2.veraenderungen, 
                        row, 
                        Abt2Veraenderung::default()
                    );
                    abt2_veraenderung.text = new_value.clone().into();
                },
                ("abt2-loeschung", "lfd-nr") => {
                    let mut abt2_loeschung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt2.loeschungen, 
                        row, 
                        Abt2Loeschung::default()
                    );
                    abt2_loeschung.lfd_nr = new_value.clone().into();
                },
                ("abt2-loeschung", "text") => {
                    let mut abt2_loeschung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt2.loeschungen, 
                        row, 
                        Abt2Loeschung::default()
                    );
                    abt2_loeschung.text = new_value.clone().into();
                },
                
                ("abt3", "lfd-nr") => {
                    let new_value = match new_value.parse::<usize>().ok() {
                        Some(s) => s,
                        None => return,
                    };
                    let mut abt3_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt3.eintraege, 
                        row, 
                        Abt3Eintrag::new(row + 1)
                    );
                    abt3_eintrag.lfd_nr = new_value.clone();
                },
                ("abt3", "bv-nr") => {
                    let mut abt3_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt3.eintraege, 
                        row, 
                        Abt3Eintrag::new(row + 1)
                    );
                    abt3_eintrag.bv_nr = new_value.clone().into();
                },
                ("abt3", "betrag") => {
                    let mut abt3_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt3.eintraege, 
                        row, 
                        Abt3Eintrag::new(row + 1)
                    );
                    abt3_eintrag.betrag = new_value.clone().into();
                },
                ("abt3", "text") => {
                    let mut abt3_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt3.eintraege, 
                        row, 
                        Abt3Eintrag::new(row + 1)
                    );
                    abt3_eintrag.text = new_value.clone().into();
                },
                ("abt3-veraenderung", "lfd-nr") => {
                    let mut abt3_veraenderung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt3.veraenderungen, 
                        row, 
                        Abt3Veraenderung::default()
                    );
                    abt3_veraenderung.lfd_nr = new_value.clone().into();
                },
                ("abt3-veraenderung", "betrag") => {
                    let mut abt3_veraenderung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt3.veraenderungen, 
                        row, 
                        Abt3Veraenderung::default()
                    );
                    abt3_veraenderung.betrag = new_value.clone().into();
                },
                ("abt3-veraenderung", "text") => {
                    let mut abt3_veraenderung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt3.veraenderungen, 
                        row, 
                        Abt3Veraenderung::default()
                    );
                    abt3_veraenderung.text = new_value.clone().into();
                },
                ("abt3-loeschung", "lfd-nr") => {
                    let mut abt3_loeschung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt3.loeschungen, 
                        row, 
                        Abt3Loeschung::default()
                    );
                    abt3_loeschung.lfd_nr = new_value.clone().into();
                },             
                ("abt3-loeschung", "betrag") => {
                    let mut abt3_loeschung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt3.loeschungen, 
                        row, 
                        Abt3Loeschung::default()
                    );
                    abt3_loeschung.betrag = new_value.clone().into();
                },
                ("abt3-loeschung", "text") => {
                    let mut abt3_loeschung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt3.loeschungen, 
                        row, 
                        Abt3Loeschung::default()
                    );
                    abt3_loeschung.text = new_value.clone().into();
                },
                
                _ => { return; }
            }
            
            open_file.speichern();
            open_file.icon = None;
            if data.konfiguration.lefis_analyse_einblenden {
                webview.eval(&format!("replaceAnalyseGrundbuch(`{}`);", ui::render_analyse_grundbuch(&open_file, &data.loaded_nb, &data.konfiguration, false, false)));
            }
        },
        Cmd::BvEintragTypAendern { path, value } => {
        
            use crate::digitalisiere::{BvEintragFlurstueck, BvEintragRecht};

            let open_file = match data.open_page.clone().and_then(|(file, _)| data.loaded_files.get_mut(&file)) { 
                Some(s) => s,
                None => return,
            };
            
            let split = path.split(":").collect::<Vec<_>>();
            
            match split.get(0) {
                Some(s) => if *s != "bv" { return; } else { },
                None => return,
            };
            
            let row = match split.get(1).and_then(|s| s.parse::<usize>().ok()) {
                Some(s) => s,
                None => return,
            };
            
            match value.as_str() {
                "flst" => {
                    if let Some(BvEintrag::Recht(BvEintragRecht { lfd_nr, bisherige_lfd_nr, .. })) = open_file.analysiert.bestandsverzeichnis.eintraege.get(row).cloned() {
                        open_file.analysiert.bestandsverzeichnis.eintraege[row] = BvEintrag::Flurstueck(BvEintragFlurstueck {
                            lfd_nr,
                            bisherige_lfd_nr,
                            .. BvEintragFlurstueck::neu(0)
                        });
                    }
                },
                "recht" => {
                    if let Some(BvEintrag::Flurstueck(BvEintragFlurstueck { lfd_nr, bisherige_lfd_nr, .. })) = open_file.analysiert.bestandsverzeichnis.eintraege.get(row).cloned() {
                        open_file.analysiert.bestandsverzeichnis.eintraege[row] = BvEintrag::Recht(BvEintragRecht {
                            lfd_nr,
                            bisherige_lfd_nr,
                            .. BvEintragRecht::neu(0)
                        });
                    }
                },
                _ => { return; }
            }
            
            // speichern
            open_file.speichern();
            
            // webview.eval(&format!("replaceMainContainer(`{}`);", ui::render_main_container(data)));
            webview.eval(&format!("replaceBestandsverzeichnis(`{}`);", ui::render_bestandsverzeichnis(open_file, &data.konfiguration)));
            webview.eval(&format!("replaceBestandsverzeichnisZuschreibungen(`{}`);", ui::render_bestandsverzeichnis_zuschreibungen(open_file)));
            webview.eval(&format!("replaceBestandsverzeichnisAbschreibungen(`{}`);", ui::render_bestandsverzeichnis_abschreibungen(open_file)));
            webview.eval(&format!("replaceAbt1(`{}`);", ui::render_abt_1(open_file)));
            webview.eval(&format!("replaceAbt1GrundlagenEintragungen(`{}`);", ui::render_abt_1_grundlagen_eintragungen(open_file)));
            webview.eval(&format!("replaceAbt1Veraenderungen(`{}`);", ui::render_abt_1_veraenderungen(open_file)));
            webview.eval(&format!("replaceAbt1Loeschungen(`{}`);", ui::render_abt_1_loeschungen(open_file)));
            webview.eval(&format!("replaceAbt2(`{}`);", ui::render_abt_2(open_file)));
            webview.eval(&format!("replaceAbt2Veraenderungen(`{}`);", ui::render_abt_2_veraenderungen(open_file)));
            webview.eval(&format!("replaceAbt2Loeschungen(`{}`);", ui::render_abt_2_loeschungen(open_file)));
            webview.eval(&format!("replaceAbt3(`{}`);", ui::render_abt_3(open_file, data.konfiguration.lefis_analyse_einblenden)));
            webview.eval(&format!("replaceAbt3Veraenderungen(`{}`);", ui::render_abt_3_veraenderungen(open_file)));
            webview.eval(&format!("replaceAbt3Loeschungen(`{}`);", ui::render_abt_3_loeschungen(open_file)));
            webview.eval(&format!("replaceAnalyseGrundbuch(`{}`);", ui::render_analyse_grundbuch(&open_file, &data.loaded_nb, &data.konfiguration, false, false))); 
            webview.eval(&format!("replacePageList(`{}`);", ui::render_page_list(data)));
        },
        Cmd::EintragNeu { path } => {
            
            let split = path.split(":").collect::<Vec<_>>();
            
            let section = match split.get(0) {
                Some(s) => s,
                None => return,
            };
            
            let row = match split.get(1).and_then(|s| s.parse::<usize>().ok()) {
                Some(s) => s,
                None => return,
            };
            
            let open_file = match data.open_page.clone().and_then(|(file, _)| data.loaded_files.get_mut(&file)) { 
                Some(s) => s,
                None => return,
            };
            
            fn insert_after<T: Clone>(vec: &mut Vec<T>, index: usize, new: T) {
                if vec.is_empty() {
                    vec.push(new.clone());
                }
                if index + 1 >= vec.len() || vec.is_empty() { 
                    vec.push(new); 
                } else {
                    vec.splice((index + 1)..(index + 1), [new]);
                }
            }

            match *section {
                "bv" => insert_after(&mut open_file.analysiert.bestandsverzeichnis.eintraege, row, BvEintrag::neu(row + 2)),
                "bv-zuschreibung" => insert_after(&mut open_file.analysiert.bestandsverzeichnis.zuschreibungen, row, BvZuschreibung::default()),
                "bv-abschreibung" => insert_after(&mut open_file.analysiert.bestandsverzeichnis.abschreibungen, row, BvAbschreibung::default()),
                
                "abt1" => insert_after(&mut open_file.analysiert.abt1.eintraege, row, Abt1Eintrag::new(row + 2)),
                "abt1-grundlage-eintragung" => insert_after(&mut open_file.analysiert.abt1.grundlagen_eintragungen, row, Abt1GrundEintragung::default()),
                "abt1-veraenderung" => insert_after(&mut open_file.analysiert.abt1.veraenderungen, row, Abt1Veraenderung::default()),
                "abt1-loeschung" => insert_after(&mut open_file.analysiert.abt1.loeschungen, row, Abt1Loeschung::default()),
                
                "abt2" => insert_after(&mut open_file.analysiert.abt2.eintraege, row, Abt2Eintrag::new(row + 2)),
                "abt2-veraenderung" => insert_after(&mut open_file.analysiert.abt2.veraenderungen, row, Abt2Veraenderung::default()),
                "abt2-loeschung" => insert_after(&mut open_file.analysiert.abt2.loeschungen, row, Abt2Loeschung::default()),
                
                "abt3" => insert_after(&mut open_file.analysiert.abt3.eintraege, row, Abt3Eintrag::new(row + 2)),
                "abt3-veraenderung" => insert_after(&mut open_file.analysiert.abt3.veraenderungen, row, Abt3Veraenderung::default()),
                "abt3-loeschung" => insert_after(&mut open_file.analysiert.abt3.loeschungen, row, Abt3Loeschung::default()),
                _ => return,
            }
            
            let next_focus = match *section {
                "bv" => format!("bv_{}_lfd-nr", row + 1),
                "bv-zuschreibung" => format!("bv-zuschreibung_{}_bv-nr", row + 1),
                "bv-abschreibung" => format!("bv-abschreibung_{}_bv-nr", row + 1),
                
                "abt1" => format!("abt1_{}_lfd-nr", row + 1),
                "abt1-grundlage-eintragung" => format!("abt1-grundlage-eintragung_{}_bv-nr", row + 1),
                "abt1-veraenderung" => format!("abt1-veraenderung_{}_lfd-nr", row + 1),
                "abt1-loeschung" => format!("abt1-loeschung_{}_lfd-nr", row + 1),
                
                "abt2" => format!("abt2_{}_lfd-nr", row + 1),
                "abt2-veraenderung" => format!("abt2-veraenderung_{}_lfd-nr", row + 1),
                "abt2-loeschung" => format!("abt2-loeschung_{}_lfd-nr", row + 1),
                
                "abt3" => format!("abt3_{}_lfd-nr", row + 1),
                "abt3-veraenderung" => format!("abt3-veraenderung_{}_lfd-nr", row + 1),
                "abt3-loeschung" => format!("abt3-loeschung_{}_lfd-nr", row + 1),
                _ => return,
            };
            
            // speichern
            open_file.speichern();
                       
            // webview.eval(&format!("replaceMainContainer(`{}`);", ui::render_main_container(data)));
            webview.eval(&format!("replaceBestandsverzeichnis(`{}`);", ui::render_bestandsverzeichnis(open_file, &data.konfiguration)));
            webview.eval(&format!("replaceBestandsverzeichnisZuschreibungen(`{}`);", ui::render_bestandsverzeichnis_zuschreibungen(open_file)));
            webview.eval(&format!("replaceBestandsverzeichnisAbschreibungen(`{}`);", ui::render_bestandsverzeichnis_abschreibungen(open_file)));
            webview.eval(&format!("replaceAbt1(`{}`);", ui::render_abt_1(open_file)));
            webview.eval(&format!("replaceAbt1GrundlagenEintragungen(`{}`);", ui::render_abt_1_grundlagen_eintragungen(open_file)));
            webview.eval(&format!("replaceAbt1Veraenderungen(`{}`);", ui::render_abt_1_veraenderungen(open_file)));
            webview.eval(&format!("replaceAbt1Loeschungen(`{}`);", ui::render_abt_1_loeschungen(open_file)));
            webview.eval(&format!("replaceAbt2(`{}`);", ui::render_abt_2(open_file)));
            webview.eval(&format!("replaceAbt2Veraenderungen(`{}`);", ui::render_abt_2_veraenderungen(open_file)));
            webview.eval(&format!("replaceAbt2Loeschungen(`{}`);", ui::render_abt_2_loeschungen(open_file)));
            webview.eval(&format!("replaceAbt3(`{}`);", ui::render_abt_3(open_file, data.konfiguration.lefis_analyse_einblenden)));
            webview.eval(&format!("replaceAbt3Veraenderungen(`{}`);", ui::render_abt_3_veraenderungen(open_file)));
            webview.eval(&format!("replaceAbt3Loeschungen(`{}`);", ui::render_abt_3_loeschungen(open_file)));
            webview.eval(&format!("replaceAnalyseGrundbuch(`{}`);", ui::render_analyse_grundbuch(&open_file, &data.loaded_nb, &data.konfiguration, false, false))); 
            webview.eval(&format!("replacePageList(`{}`);", ui::render_page_list(data)));

            webview.eval(&format!("document.getElementById(`{}`).focus();", next_focus));
        },
        Cmd::EintragLoeschen { path } | 
        Cmd::EintragRoeten { path } => {
        
            let eintrag_roeten = match arg {
                Cmd::EintragLoeschen { .. } => false,
                _ => true,
            };
            
            let split = path.split(":").collect::<Vec<_>>();
            
            let section = match split.get(0) {
                Some(s) => s,
                None => return,
            };
            
            let row = match split.get(1).and_then(|s| s.parse::<usize>().ok()) {
                Some(s) => s,
                None => return,
            };
            
            let open_file = match data.open_page.clone().and_then(|(file, _)| data.loaded_files.get_mut(&file)) { 
                Some(s) => s,
                None => return,
            };
            
            match (*section, eintrag_roeten) {
                ("bv", false) => { 
                    if !open_file.analysiert.bestandsverzeichnis.eintraege.is_empty() {
                        open_file.analysiert.bestandsverzeichnis.eintraege.remove(row);
                    }
                },
                ("bv", true) => { 
                    open_file.analysiert.bestandsverzeichnis.eintraege
                    .get_mut(row)
                    .map(|e| {
                        let cur = *e.get_manuell_geroetet().get_or_insert_with(|| e.get_automatisch_geroetet().unwrap_or(false));
                        e.set_manuell_geroetet(Some(!cur));
                    });
                },
                
                ("bv-zuschreibung", false) => { 
                    if !open_file.analysiert.bestandsverzeichnis.zuschreibungen.is_empty() {
                        open_file.analysiert.bestandsverzeichnis.zuschreibungen.remove(row);
                    }
                },
                ("bv-zuschreibung", true) => { 
                    open_file.analysiert.bestandsverzeichnis.zuschreibungen
                    .get_mut(row)
                    .map(|e| {
                        let cur = *e.manuell_geroetet.get_or_insert_with(|| e.automatisch_geroetet);
                        e.manuell_geroetet = Some(!cur);
                    });
                },

                ("bv-abschreibung", false) => { 
                    if !open_file.analysiert.bestandsverzeichnis.abschreibungen.is_empty() {
                        open_file.analysiert.bestandsverzeichnis.abschreibungen.remove(row);
                    }
                },
                ("bv-abschreibung", true) => { 
                    open_file.analysiert.bestandsverzeichnis.abschreibungen
                    .get_mut(row)
                    .map(|e| {
                        let cur = *e.manuell_geroetet.get_or_insert_with(|| e.automatisch_geroetet);
                        e.manuell_geroetet = Some(!cur);
                    });
                },

                ("abt1", false) => { 
                    if !open_file.analysiert.abt1.eintraege.is_empty() {
                        open_file.analysiert.abt1.eintraege.remove(row);
                    }
                },
                ("abt1", true) => { 
                    open_file.analysiert.abt1.eintraege
                    .get_mut(row)
                    .map(|e| {
                        let cur = *e.get_manuell_geroetet().get_or_insert_with(|| e.get_automatisch_geroetet());
                        e.set_manuell_geroetet(Some(!cur));
                    });
                },
                
                ("abt1-grundlage-eintragung", false) => { 
                    if !open_file.analysiert.abt1.grundlagen_eintragungen.is_empty() {
                        open_file.analysiert.abt1.grundlagen_eintragungen.remove(row);
                    }
                },
                ("abt1-grundlage-eintragung", true) => { 
                    open_file.analysiert.abt1.grundlagen_eintragungen
                    .get_mut(row)
                    .map(|e| {
                        let cur = *e.manuell_geroetet.get_or_insert_with(|| e.automatisch_geroetet);
                        e.manuell_geroetet = Some(!cur);
                    });
                },
                
                ("abt1-veraenderung", false) => { 
                    if !open_file.analysiert.abt1.veraenderungen.is_empty() {
                        open_file.analysiert.abt1.veraenderungen.remove(row);
                    }
                },
                ("abt1-veraenderung", true) => { 
                    open_file.analysiert.abt1.veraenderungen
                    .get_mut(row)
                    .map(|e| {
                        let cur = *e.manuell_geroetet.get_or_insert_with(|| e.automatisch_geroetet);
                        e.manuell_geroetet = Some(!cur);
                    });
                },
                
                ("abt1-loeschung", false) => { 
                    if !open_file.analysiert.abt1.loeschungen.is_empty() {
                        open_file.analysiert.abt1.loeschungen.remove(row);
                    }
                },
                ("abt1-loeschung", true) => { 
                    open_file.analysiert.abt1.loeschungen
                    .get_mut(row)
                    .map(|e| {
                        let cur = *e.manuell_geroetet.get_or_insert_with(|| e.automatisch_geroetet);
                        e.manuell_geroetet = Some(!cur);
                    });
                },
                
                ("abt2", false) => {
                    if !open_file.analysiert.abt2.eintraege.is_empty() {
                        open_file.analysiert.abt2.eintraege.remove(row); 
                    }
                },
                ("abt2", true) => { 
                    open_file.analysiert.abt2.eintraege
                    .get_mut(row)
                    .map(|e| {
                        let cur = *e.manuell_geroetet.get_or_insert_with(|| e.automatisch_geroetet);
                        e.manuell_geroetet = Some(!cur);
                    });
                },
                
                ("abt2-veraenderung", false) => { 
                    if !open_file.analysiert.abt2.veraenderungen.is_empty() {
                        open_file.analysiert.abt2.veraenderungen.remove(row); 
                    }
                },
                ("abt2-veraenderung", true) => { 
                    open_file.analysiert.abt2.veraenderungen
                    .get_mut(row)
                    .map(|e| {
                        let cur = *e.manuell_geroetet.get_or_insert_with(|| e.automatisch_geroetet);
                        e.manuell_geroetet = Some(!cur);
                    });
                },
                
                ("abt2-loeschung", false) => { 
                    if !open_file.analysiert.abt2.loeschungen.is_empty() {
                        open_file.analysiert.abt2.loeschungen.remove(row); 
                    }
                },
                ("abt2-loeschung", true) => { 
                    open_file.analysiert.abt2.loeschungen
                    .get_mut(row)
                    .map(|e| {
                        let cur = *e.manuell_geroetet.get_or_insert_with(|| e.automatisch_geroetet);
                        e.manuell_geroetet = Some(!cur);
                    });
                },
                
                ("abt3", false) => { 
                    if !open_file.analysiert.abt3.eintraege.is_empty() {
                        open_file.analysiert.abt3.eintraege.remove(row); 
                    }
                },
                ("abt3", true) => { 
                    open_file.analysiert.abt3.eintraege
                    .get_mut(row)
                    .map(|e| {
                        let cur = *e.manuell_geroetet.get_or_insert_with(|| e.automatisch_geroetet);
                        e.manuell_geroetet = Some(!cur);
                    });
                },
                
                ("abt3-veraenderung", false) => { 
                    if !open_file.analysiert.abt3.veraenderungen.is_empty() {
                        open_file.analysiert.abt3.veraenderungen.remove(row); 
                    }
                },
                ("abt3-veraenderung", true) => { 
                    open_file.analysiert.abt3.veraenderungen
                    .get_mut(row)
                    .map(|e| {
                        let cur = *e.manuell_geroetet.get_or_insert_with(|| e.automatisch_geroetet);
                        e.manuell_geroetet = Some(!cur);
                    });
                },
                
                ("abt3-loeschung", false) => { 
                    if !open_file.analysiert.abt3.loeschungen.is_empty() {
                        open_file.analysiert.abt3.loeschungen.remove(row); 
                    }
                },
                ("abt3-loeschung", true) => { 
                    open_file.analysiert.abt3.loeschungen
                    .get_mut(row)
                    .map(|e| {
                        let cur = *e.manuell_geroetet.get_or_insert_with(|| e.automatisch_geroetet);
                        e.manuell_geroetet = Some(!cur);
                    });
                },
                
                _ => return,
            }
            
            let next_focus = match *section {
                "bv" => format!("bv_{}_lfd-nr", if eintrag_roeten { row + 1 } else { row.saturating_sub(1) }),
                "bv-zuschreibung" => format!("bv-zuschreibung_{}_bv-nr", if eintrag_roeten { row + 1 } else { row.saturating_sub(1) }),
                "bv-abschreibung" => format!("bv-abschreibung_{}_bv-nr", if eintrag_roeten { row + 1 } else { row.saturating_sub(1) }),
                
                "abt1" => format!("abt1_{}_lfd-nr", if eintrag_roeten { row + 1 } else { row.saturating_sub(1) }),
                "abt1-grundlage-eintragung" => format!("abt1-grundlage-eintragung_{}_bv-nr", if eintrag_roeten { row + 1 } else { row.saturating_sub(1) }),
                "abt1-veraenderung" => format!("abt1-veraenderung_{}_lfd-nr", if eintrag_roeten { row + 1 } else { row.saturating_sub(1) }),
                "abt1-loeschung" => format!("abt1-loeschung_{}_lfd-nr", if eintrag_roeten { row + 1 } else { row.saturating_sub(1) }),

                "abt2" => format!("abt2_{}_lfd-nr", if eintrag_roeten { row + 1 } else { row.saturating_sub(1) }),
                "abt2-veraenderung" => format!("abt2-veraenderung_{}_lfd-nr", if eintrag_roeten { row + 1 } else { row.saturating_sub(1) }),
                "abt2-loeschung" => format!("abt2-loeschung_{}_lfd-nr", if eintrag_roeten { row + 1 } else { row.saturating_sub(1) }),

                "abt3" => format!("abt3_{}_lfd-nr", if eintrag_roeten { row + 1 } else { row.saturating_sub(1) }),
                "abt3-veraenderung" => format!("abt3-veraenderung_{}_lfd-nr", if eintrag_roeten { row + 1 } else { row.saturating_sub(1) }),
                "abt3-loeschung" => format!("abt3-loeschung_{}_lfd-nr", if eintrag_roeten { row + 1 } else { row.saturating_sub(1) }),

                _ => return,
            };

            // speichern
            open_file.speichern();
    
            // webview.eval(&format!("replaceMainContainer(`{}`);", ui::render_main_container(data)));
            webview.eval(&format!("replaceBestandsverzeichnis(`{}`);", ui::render_bestandsverzeichnis(open_file, &data.konfiguration)));
            webview.eval(&format!("replaceBestandsverzeichnisZuschreibungen(`{}`);", ui::render_bestandsverzeichnis_zuschreibungen(open_file)));
            webview.eval(&format!("replaceBestandsverzeichnisAbschreibungen(`{}`);", ui::render_bestandsverzeichnis_abschreibungen(open_file)));
            webview.eval(&format!("replaceAbt1(`{}`);", ui::render_abt_1(open_file)));
            webview.eval(&format!("replaceAbt1GrundlagenEintragungen(`{}`);", ui::render_abt_1_grundlagen_eintragungen(open_file)));
            webview.eval(&format!("replaceAbt1Veraenderungen(`{}`);", ui::render_abt_1_veraenderungen(open_file)));
            webview.eval(&format!("replaceAbt1Loeschungen(`{}`);", ui::render_abt_1_loeschungen(open_file)));
            webview.eval(&format!("replaceAbt2(`{}`);", ui::render_abt_2(open_file)));
            webview.eval(&format!("replaceAbt2Veraenderungen(`{}`);", ui::render_abt_2_veraenderungen(open_file)));
            webview.eval(&format!("replaceAbt2Loeschungen(`{}`);", ui::render_abt_2_loeschungen(open_file)));
            webview.eval(&format!("replaceAbt3(`{}`);", ui::render_abt_3(open_file, data.konfiguration.lefis_analyse_einblenden)));
            webview.eval(&format!("replaceAbt3Veraenderungen(`{}`);", ui::render_abt_3_veraenderungen(open_file)));
            webview.eval(&format!("replaceAbt3Loeschungen(`{}`);", ui::render_abt_3_loeschungen(open_file)));
            webview.eval(&format!("replaceAnalyseGrundbuch(`{}`);", ui::render_analyse_grundbuch(&open_file, &data.loaded_nb, &data.konfiguration, false, false))); 
            webview.eval(&format!("replacePageList(`{}`);", ui::render_page_list(data)));

            webview.eval(&format!("(function() {{ 
                let element = document.getElementById(`{}`); 
                if (element) {{ element.focus(); }};
            }})();", next_focus));
        },
        Cmd::OpenContextMenu { x, y, seite } => {
            data.popover_state = Some(PopoverState::ContextMenu(ContextMenuData {
                x: *x,
                y: *y,
                seite_ausgewaehlt: *seite,
            }));
            webview.eval(&format!("replacePopOver(`{}`)", ui::render_popover_content(data)));
        },
        Cmd::OpenConfiguration => {
            data.popover_state = Some(PopoverState::Configuration(ConfigurationView::Allgemein));
            webview.eval(&format!("replacePopOver(`{}`)", ui::render_popover_content(data)));
        },
        Cmd::SetConfigurationView { section_id } => {
            data.popover_state = Some(PopoverState::Configuration(match section_id.as_str() {
                "allgemein" => ConfigurationView::Allgemein,
                "regex" => ConfigurationView::RegEx,
                "text-saubern" => ConfigurationView::TextSaubern,
                "abkuerzungen" => ConfigurationView::Abkuerzungen,
                "flst-auslesen" => ConfigurationView::FlstAuslesen,
                "klassifizierung-rechteart-abt2" => ConfigurationView::KlassifizierungRechteArt,
                "rechtsinhaber-auslesen-abt2" => ConfigurationView::RechtsinhaberAuslesenAbt2,
                "rangvermerk-auslesen-abt2" => ConfigurationView::RangvermerkAuslesenAbt2,
                "text-kuerzen-abt2" => ConfigurationView::TextKuerzenAbt2,
                "betrag-auslesen-abt3" => ConfigurationView::BetragAuslesenAbt3,
                "klassifizierung-schuldenart-abt3" => ConfigurationView::KlassifizierungSchuldenArtAbt3,
                "rechtsinhaber-auslesen-abt3" => ConfigurationView::RechtsinhaberAuslesenAbt3,
                "text-kuerzen-abt3" => ConfigurationView::TextKuerzenAbt3,
                _ => { return; }
            }));
            webview.eval(&format!("replacePopOver(`{}`)", ui::render_popover_content(data)));
        },
        Cmd::OpenInfo => {
            data.popover_state = Some(PopoverState::Info);
            webview.eval(&format!("replacePopOver(`{}`)", ui::render_popover_content(data)));
        },
        Cmd::OpenHelp => {
            data.popover_state = Some(PopoverState::Help);
            webview.eval(&format!("replacePopOver(`{}`)", ui::render_popover_content(data)));
        },
        Cmd::OpenExportPdf => {
            if data.loaded_files.is_empty() { return; }
            data.popover_state = Some(PopoverState::ExportPdf);
            webview.eval(&format!("replacePopOver(`{}`)", ui::render_popover_content(data)));
        },
        Cmd::CloseFile { file_name } => {
            let _ = data.loaded_files.remove(file_name);
            data.popover_state = None;
            webview.eval(&format!("stopCheckingForPageLoaded(`{}`)", file_name));
            webview.eval(&format!("replaceEntireScreen(`{}`)", ui::render_entire_screen(data)));
        },
        Cmd::CheckPdfForErrors => {
            let mut new_icons = BTreeMap::new();
            let mut icon_count = 0;
            
            for (k, v) in data.loaded_files.iter() {
                if icon_count >= 1 {
                    break;
                }
                
                if v.ist_geladen() && v.icon.is_none() {
                    let icon = match v.get_icon(&data.loaded_nb, &data.konfiguration) {
                        Some(s) => s,
                        None => { return; },
                    };
                    new_icons.insert(k.clone(), icon);
                    icon_count += 1;
                }
            }

            for (k, v) in new_icons {
                if let Some(s) = data.loaded_files.get_mut(&k.clone()) {
                    s.icon = Some(v);
                    webview.eval(&format!("replaceIcon(`{}`, `{}`)", k, v.get_base64()));
                }
            }
        },
        Cmd::ToggleLefisAnalyse => {
            data.konfiguration.lefis_analyse_einblenden = !data.konfiguration.lefis_analyse_einblenden;
            webview.eval(&format!("replaceMainContainer(`{}`);", ui::render_main_container(data)));
        },
        Cmd::EditTextKuerzenAbt2Script { script } => {
            data.konfiguration.text_kuerzen_abt2_script = script.lines().map(|l| l.replace("\u{00a0}", " ")).collect();
            data.konfiguration.speichern();
        },
        Cmd::EditTextKuerzenAbt3Script { script } => {
            data.konfiguration.text_kuerzen_abt3_script = script.lines().map(|l| l.replace("\u{00a0}", " ")).collect();
            data.konfiguration.speichern();
        },
        Cmd::EditAbkuerzungenScript { script } => {
            data.konfiguration.abkuerzungen_script = script.lines().map(|l| l.replace("\u{00a0}", " ")).collect();
            data.konfiguration.speichern();
        },
        Cmd::EditTextSaubernScript { script } => {
            data.konfiguration.text_saubern_script = script.lines().map(|l| l.replace("\u{00a0}", " ")).collect();
            data.konfiguration.speichern();
        },
        Cmd::EditFlurstueckeAuslesenScript { script } => {
            data.konfiguration.flurstuecke_auslesen_script = script.lines().map(|l| l.replace("\u{00a0}", " ")).collect();
            data.konfiguration.speichern();
        },
        Cmd::EditRechteArtScript { neu } => {
            data.konfiguration.klassifiziere_rechteart = neu.lines().map(|l| l.replace("\u{00a0}", " ")).collect();
            data.konfiguration.speichern();
        },
        Cmd::EditRangvermerkAuslesenAbt2Script { neu } => {
            data.konfiguration.rangvermerk_auslesen_abt2_script = neu.lines().map(|l| l.replace("\u{00a0}", " ")).collect();
            data.konfiguration.speichern();
        },       
        Cmd::EditRechtsinhaberAuslesenAbt2Script { neu } => {
            data.konfiguration.rechtsinhaber_auslesen_abt2_script = neu.lines().map(|l| l.replace("\u{00a0}", " ")).collect();
            data.konfiguration.speichern();
        },
        Cmd::EditRechtsinhaberAuslesenAbt3Script { neu } => {
            data.konfiguration.rechtsinhaber_auslesen_abt3_script = neu.lines().map(|l| l.replace("\u{00a0}", " ")).collect();
            data.konfiguration.speichern();
        },
        Cmd::EditSchuldenArtScript { neu } => {
            data.konfiguration.klassifiziere_schuldenart = neu.lines().map(|l| l.replace("\u{00a0}", " ")).collect();
            data.konfiguration.speichern();
        },
        Cmd::EditBetragAuslesenScript { neu } => {
            data.konfiguration.betrag_auslesen_script = neu.lines().map(|l| l.replace("\u{00a0}", " ")).collect();
            data.konfiguration.speichern();
        },   
        Cmd::FlurstueckAuslesenScriptTesten { text, bv_nr } => {
            
            let start = std::time::Instant::now();
            let mut debug_log = String::new();
            let result: Result<String, String> = Python::with_gil(|py| {
                let (text_sauber, saetze_clean) = crate::kurztext::text_saubern(&*text, &data.konfiguration)?;

                let mut fehler = Vec::new();
                let mut warnungen = Vec::new();
                let mut spalte1_eintraege = Vec::new();
                
                let default_bv = Vec::new();
                let open_file = data.open_page.clone().and_then(|(file, _)| data.loaded_files.get_mut(&file));
            
                let bv_eintraege = crate::analysiere::get_belastete_flurstuecke(
                    py,
                    bv_nr,
                    &text_sauber,
                    &Titelblatt {
                        amtsgericht: "Amtsgericht".to_string(),
                        grundbuch_von: "GrundbuchVon".to_string(),
                        blatt: 0,
                    },
                    open_file
                    .map(|of| &of.analysiert.bestandsverzeichnis.eintraege)
                    .unwrap_or(&default_bv),
                    &data.konfiguration,
                    &mut debug_log,
                    &mut spalte1_eintraege,
                    &mut warnungen,
                    &mut fehler,
                )?;
                
                Ok(spalte1_eintraege.iter().map(|e| format!("{e:#?}")).collect::<Vec<_>>().join("\r\n"))
            });
            
            let time = std::time::Instant::now() - start;
            let result: String = match result {
                Ok(o) => { format!("{}\r\nLOG:\r\n{}\r\nAusgabe berechnet in {:?}", o, debug_log, time) },
                Err(e) => { format!("{}", e) },
            };
            webview.eval(&format!("replaceFlurstueckAuslesenTestOutput(`{}`);", result));
        },
        Cmd::RangvermerkAuslesenAbt2ScriptTesten { text } => {
            let start = std::time::Instant::now();
            let result: Result<String, String> = Python::with_gil(|py| {
                let (text_sauber, saetze_clean) = crate::kurztext::text_saubern(&*text, &data.konfiguration)?;
                python_exec_kurztext_string(
                    py, "RangvermerkAuslesenAbt2ScriptTest", 
                    &text_sauber, &saetze_clean, 
                    &data.konfiguration.rangvermerk_auslesen_abt2_script, 
                    &data.konfiguration
                )
            });
            let time = std::time::Instant::now() - start;
            let result: String = match result {
                Ok(o) => { format!("{}\r\nAusgabe berechnet in {:?}", o, time) },
                Err(e) => { format!("{}", e) },
            };
            webview.eval(&format!("replaceRangvermerkAuslesenAbt2TestOutput(`{}`);", result));
        }, 
        Cmd::RechtsinhaberAuslesenAbt2ScriptTesten { text } => {
            let start = std::time::Instant::now();
            let result: Result<String, String> = Python::with_gil(|py| {
                let (text_sauber, saetze_clean) = crate::kurztext::text_saubern(&*text, &data.konfiguration)?;
                python_exec_kurztext_string(
                    py, "RechtsinhaberAuslesenAbt2ScriptTest", 
                    &text_sauber, &saetze_clean, 
                    &data.konfiguration.rechtsinhaber_auslesen_abt2_script, 
                    &data.konfiguration
                )
            });
            let time = std::time::Instant::now() - start;
            let result: String = match result {
                Ok(o) => { format!("{}\r\nAusgabe berechnet in {:?}", o, time) },
                Err(e) => { format!("{}", e) },
            };
            webview.eval(&format!("replaceRechtsinhaberAbt2TestOutput(`{}`);", result));
        },
        Cmd::RechtsinhaberAuslesenAbt3ScriptTesten { text } => {
            let start = std::time::Instant::now();
            let result: Result<String, String> = Python::with_gil(|py| {
                let (text_sauber, saetze_clean) = crate::kurztext::text_saubern(&*text, &data.konfiguration)?;
                python_exec_kurztext_string(py, 
                    "RechtsinhaberAuslesenAbt3ScriptTest", 
                    &text_sauber, &saetze_clean,  
                    &data.konfiguration.rechtsinhaber_auslesen_abt3_script, 
                    &data.konfiguration
                )
            });
            let time = std::time::Instant::now() - start;
            let result = match result {
                Ok(o) => { format!("{}\r\nAusgabe berechnet in {:?}", o, time) },
                Err(e) => { format!("{}", e) },
            };
            webview.eval(&format!("replaceRechtsinhaberAbt3TestOutput(`{}`);", result));
        },
        Cmd::BetragAuslesenScriptTesten { text } => {
            let start = std::time::Instant::now();
            let result: Result<PyBetrag, String> = Python::with_gil(|py| {
                let (text_sauber, saetze_clean) = crate::kurztext::text_saubern(&*text, &data.konfiguration)?;
                python_exec_kurztext(py, 
                    "BetragAuslesenScriptTest", 
                    &text_sauber, &saetze_clean, &data.konfiguration.betrag_auslesen_script, 
                    &data.konfiguration
                )
            });
            let time = std::time::Instant::now() - start;
            let result = match result {
                Ok(o) => { format!("{:#?}\r\nAusgabe berechnet in {:?}", o.inner, time) },
                Err(e) => { format!("{}", e) },
            };
            webview.eval(&format!("replaceBetragAuslesenTestOutput(`{}`);", result));
        },
        Cmd::KurzTextAbt2ScriptTesten { text } => {
            let start = std::time::Instant::now();
            let result: Result<String, String> = Python::with_gil(|py| {
                let (text_sauber, saetze_clean) = crate::kurztext::text_saubern(&*text, &data.konfiguration)?;

                let rechteart: Result<RechteArtPyWrapper, String> = crate::python_exec_kurztext(
                    py,
                    "KurzTextAbt2ScriptTest",
                    &text_sauber, 
                    &saetze_clean, 
                    &data.konfiguration.klassifiziere_schuldenart, 
                    &data.konfiguration
                );
                let rechteart = rechteart?.inner;
                
                python_exec_kurztext_string(
                    py, "KurzTextAbt2ScriptTest",
                    &text_sauber, 
                    &saetze_clean, 
                    &data.konfiguration.text_kuerzen_abt2_script, 
                    &data.konfiguration
                )
            });
            let time = std::time::Instant::now() - start;
            let result = match result {
                Ok(o) => { format!("{}\r\nAusgabe berechnet in {:?}", o, time) },
                Err(e) => { format!("{}", e) },
            };
            webview.eval(&format!("replaceTextKuerzenAbt2TestOutput(`{}`);", result));
        },
        Cmd::KurzTextAbt3ScriptTesten { text } => {
            let start = std::time::Instant::now();
            let result: Result<String, String> = Python::with_gil(|py| {
                
                let (text_sauber, saetze_clean) = crate::kurztext::text_saubern(&*text, &data.konfiguration)?;
                
                let schuldenart: Result<SchuldenArtPyWrapper, String> = crate::python_exec_kurztext(
                    py,
                    "KurzTextAbt3ScriptTest",
                    &text_sauber, 
                    &saetze_clean, 
                    &data.konfiguration.klassifiziere_schuldenart, 
                    &data.konfiguration
                );
                let schuldenart = schuldenart?.inner;
                
                let betrag: Result<PyBetrag, String> = crate::python_exec_kurztext(
                    py,
                    "KurzTextAbt3ScriptTest",
                    &format!("100.000,00 EUR"), 
                    &[format!("100.000,00 EUR")], 
                    &data.konfiguration.betrag_auslesen_script, 
                    &data.konfiguration
                );
                let betrag = betrag?.inner;
                
                let rechtsinhaber: Result<String, String> = crate::python_exec_kurztext_string(
                    py,
                    "KurzTextAbt3ScriptTest",
                    &text_sauber, 
                    &saetze_clean, 
                    &data.konfiguration.betrag_auslesen_script, 
                    &data.konfiguration
                );
                let rechtsinhaber = rechtsinhaber?;

                python_exec_kuerze_text_abt3(
                    py,
                    "KurzTextAbt3ScriptTest",
                    &text_sauber,
                    Some(format!("{} {}", crate::kurztext::formatiere_betrag(&betrag), betrag.waehrung.to_string())),
                    Some(format!("{}", schuldenart.to_string())),
                    Some(rechtsinhaber),
                    &saetze_clean, 
                    &data.konfiguration.text_kuerzen_abt3_script, 
                    &data.konfiguration
                )
            });
            let time = std::time::Instant::now() - start;
            let result = match result {
                Ok(o) => { format!("{}\r\nAusgabe berechnet in {:?}", o, time) },
                Err(e) => { format!("{}", e) },
            };
            webview.eval(&format!("replaceTextKuerzenAbt3TestOutput(`{}`);", result));
        },
        
        Cmd::RechteArtScriptTesten { text } => {
            let start = std::time::Instant::now();
            let result: Result<RechteArtPyWrapper, String> = Python::with_gil(|py| {
                let (text_sauber, saetze_clean) = crate::kurztext::text_saubern(&*text, &data.konfiguration)?;
                python_exec_kurztext(py, "RechteArtScriptTest", &text_sauber, &saetze_clean, &data.konfiguration.klassifiziere_rechteart, &data.konfiguration)
            });
            let time = std::time::Instant::now() - start;
            let result = match result {
                Ok(o) => { format!("{:?}\r\nAusgabe berechnet in {:?}", o.inner, time) },
                Err(e) => { format!("{}", e) },
            };
            
            webview.eval(&format!("replaceRechteArtTestOutput(`{}`);", result));
        },
        Cmd::SchuldenArtScriptTesten { text } => {
            let start = std::time::Instant::now();
            let result: Result<SchuldenArtPyWrapper, String> = Python::with_gil(|py| {
                let (text_sauber, saetze_clean) = crate::kurztext::text_saubern(&*text, &data.konfiguration)?;
                python_exec_kurztext(py, "SchuldenArtScriptTest", &text_sauber, &saetze_clean, &data.konfiguration.klassifiziere_schuldenart, &data.konfiguration)
            });
            let time = std::time::Instant::now() - start;
            let result = match result {
                Ok(o) => { format!("{:?}\r\nAusgabe berechnet in {:?}", o.inner, time) },
                Err(e) => { format!("{}", e) },
            };
            webview.eval(&format!("replaceSchuldenArtTestOutput(`{}`);", result));
        },
        Cmd::DeleteNebenbeteiligte => {
            use tinyfiledialogs::{YesNo, MessageBoxIcon};
            
            if data.loaded_files.is_empty() {
                return;
            }
            
            if tinyfiledialogs::message_box_yes_no(
                "Wirklich löschen?",
                &format!("Alle Ordnungsnummern werden aus den Dateien gelöscht. Fortfahren?"),
                MessageBoxIcon::Warning,
                YesNo::No,
            ) == YesNo::No {
                return;
            }
                
            data.loaded_nb.clear();
            data.loaded_nb_paths.clear();
            for pdf_file in data.loaded_files.values_mut() {
                pdf_file.nebenbeteiligte_dateipfade.clear();
                pdf_file.speichern();
            }
            
            webview.eval(&format!("replaceEntireScreen(`{}`)", ui::render_entire_screen(data)));
        },
        Cmd::KlassifiziereSeiteNeu { 
            seite, 
            klassifikation_neu 
        } => {
            use crate::digitalisiere::SeitenTyp::*;
                        
            let open_file = match data.open_page.clone().and_then(|(file, _)| data.loaded_files.get_mut(&file)) { 
                Some(s) => s,
                None => return,
            };
            
            let seiten_typ_neu = match klassifikation_neu.as_str() {
                "bv-horz" => BestandsverzeichnisHorz,
                "bv-horz-zu-und-abschreibungen" => BestandsverzeichnisHorzZuUndAbschreibungen,
                "bv-vert" => BestandsverzeichnisVert,
                "bv-vert-typ2" => BestandsverzeichnisVertTyp2,
                "bv-vert-zu-und-abschreibungen" => BestandsverzeichnisVertZuUndAbschreibungen,
                "abt1-horz" => Abt1Horz,
                "abt1-vert" => Abt1Vert,
                "abt2-horz-veraenderungen" => Abt2HorzVeraenderungen,
                "abt2-horz" => Abt2Horz,
                "abt2-vert-veraenderungen" => Abt2VertVeraenderungen,
                "abt2-vert" => Abt2Vert,
                "abt3-horz-veraenderungen-loeschungen" => Abt3HorzVeraenderungenLoeschungen,
                "abt3-vert-veraenderungen-loeschungen" => Abt3VertVeraenderungenLoeschungen,
                "abt3-horz" => Abt3Horz,
                "abt3-vert-veraenderungen" => Abt3VertVeraenderungen,
                "abt3-vert-loeschungen" => Abt3VertLoeschungen,
                "abt3-vert" => Abt3Vert,
                _ => { return; },
            };
                        
            open_file.klassifikation_neu.insert(*seite, seiten_typ_neu);
            data.popover_state = None;            
                        
            let open_file = match data.open_page.clone().and_then(|(file, _)| data.loaded_files.get_mut(&file)) { 
                Some(s) => s,
                None => return,
            };
            
            // speichern
            open_file.speichern();
                        
            webview.eval(&format!("replaceEntireScreen(`{}`);", ui::render_entire_screen(data)));

        },
        Cmd::ClosePopOver { } => {
            if let Some(PopoverState::Configuration(_)) = data.popover_state {
                for (k, v) in data.loaded_files.iter_mut() {
                    v.icon = None;
                }
                webview.eval(&format!("replaceFileList(`{}`)", ui::render_file_list(data)));
            }
            data.popover_state = None;
            webview.eval(&format!("replacePopOver(`{}`)", ui::render_popover_content(data)));
        },
        Cmd::Undo => {
            println!("undo");
        },
        Cmd::Redo => {
            println!("redo");
        },
        Cmd::ResetOcrSelection => {
            webview.eval(&format!("resetOcrSelection()"));
        },
        Cmd::SelectOcr {
            file_name,
            page,
            min_x,
            min_y,
            max_x,
            max_y,
            page_width,
            page_height,
        } => {
            
            use std::fs::File;
            use std::process::Command;
            use image::ImageOutputFormat;
            use crate::digitalisiere::{formatiere_seitenzahl, zeilen_aus_tesseract_hocr};
            
            let file = match data.loaded_files.get_mut(file_name.as_str()) {
                Some(s) => s,
                None => {
                    webview.eval(&format!("resetOcrSelection()"));
                    return;
                }
            };
                        
            if file.datei.as_ref().is_none() {
                return;
            }
            
            let temp_ordner = std::env::temp_dir()
            .join(&format!("{gemarkung}/{blatt}", gemarkung = file.titelblatt.grundbuch_von, blatt = file.titelblatt.blatt));
            
            let max_seitenzahl = file.seitenzahlen.iter().copied().max().unwrap_or(0);

            let pdftoppm_output_path = temp_ordner.clone().join(format!("page-clean-{}.png", crate::digitalisiere::formatiere_seitenzahl(*page as u32, max_seitenzahl)));
            
            if !Path::new(&pdftoppm_output_path).exists() {
                if let Some(pdf) = file.datei.as_ref() {
                    if let Ok(o) = std::fs::read(&pdf) {
                        let _ = crate::digitalisiere::konvertiere_pdf_seiten_zu_png(&o, &[*page as u32], max_seitenzahl, &file.titelblatt);
                    }
                }
            }
            
            let pdf_to_ppm_bytes = match std::fs::read(&pdftoppm_output_path) {
                Ok(o) => o,
                Err(_) => {
                    webview.eval(&format!("resetOcrSelection()"));
                    return;
                },
            };
    
            let (im_width, im_height) = match image::image_dimensions(&pdftoppm_output_path)
            .map_err(|e| Fehler::Bild(format!("{}", pdftoppm_output_path.display()), e)){
                Ok(o) => o,
                Err(_) => {
                    webview.eval(&format!("resetOcrSelection()"));
                    return;
                }
            };

            let im_width = im_width as f32;
            let im_height = im_height as f32;
        
            let x = min_x.min(*max_x) / page_width * im_width as f32;
            let y = min_y.min(*max_y) / page_height * im_height as f32;
            let width = (max_x - min_x).abs() / page_width * im_width as f32;
            let height = (max_y - min_y).abs() / page_width * im_width as f32;
            
            let x = x.round().max(0.0) as u32;
            let y = y.round().max(0.0) as u32;
            let width = width.round().max(0.0) as u32;
            let height = height.round().max(0.0) as u32;
            
            let im = match image::open(&pdftoppm_output_path.clone())
            .map_err(|e| Fehler::Bild(format!("{}", pdftoppm_output_path.display()), e)) {
                Ok(o) => o,
                Err(_) => {
                    webview.eval(&format!("resetOcrSelection()"));
                    return;
                },
            };

            let cropped = im.crop_imm(x, y, width, height);
            
            let cropped_output_path = temp_ordner.clone().join(format!("crop-{}-{}-{}.png", formatiere_seitenzahl(*page as u32, max_seitenzahl), width, height));
            if let Ok(mut output_file) = File::create(cropped_output_path.clone()) {
                let _ = cropped.write_to(&mut output_file, ImageOutputFormat::Png);
            }
                        
            let tesseract_output_path = temp_ordner.clone().join(format!("ocr-selection-{:02}-{:02}-{:02}-{:02}-{:02}.txt.hocr", page, x, y, width, height));
        
            let _ = Command::new("tesseract")
            .arg(&format!("{}", cropped_output_path.display()))
            .arg(&format!("{}", temp_ordner.clone().join(format!("ocr-selection-{:02}-{:02}-{:02}-{:02}-{:02}.txt", page, x, y, width, height)).display()))     
            .arg("--dpi")
            .arg("600")
            .arg("--psm")
            .arg("6")
            .arg("-l")
            .arg("deu")
            .arg("-c")
            .arg("tessedit_char_whitelist=abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZüÜäÄöÖß,.-/%§()€0123456789 ")
            .arg("-c")
            .arg("tessedit_create_hocr=1")
            .arg("-c")
            .arg("debug_file=/dev/null") // TODO: funktioniert nur auf Linux!
            .status();
            
            let zeilen = zeilen_aus_tesseract_hocr(tesseract_output_path.display().to_string()).unwrap_or_default();
            let text = zeilen.join("\r\n");
            let text = if data.konfiguration.zeilenumbrueche_in_ocr_text {
                text
            } else {
                let result: Result<String, String> = Python::with_gil(|py| {
                    let (text_sauber, saetze_clean) = crate::kurztext::text_saubern(&text, &data.konfiguration)?;
                    Ok(text_sauber)
                });
                match result {
                    Ok(o) => o,
                    Err(e) => e,
                }
            };
    
            webview.eval(&format!("copyTextToClipboard(`{}`)", text));
            webview.eval(&format!("resetOcrSelection()"));
        },
        Cmd::CopyTextToClipboard { text } => {
            webview.eval(&format!("copyTextToClipboard(`{}`)", text));
        },
        Cmd::ReloadGrundbuch => {
            
            use tinyfiledialogs::{YesNo, MessageBoxIcon};
            
            if data.loaded_files.is_empty() {
                return;
            }
            
            let (file_id, page) = match data.open_page.clone() {
                Some((file, page)) => (file.clone(), page as usize),
                None => return,
            };
            
            {                
                let open_file = match data.loaded_files.get_mut(&file_id) { 
                    Some(s) => s,
                    None => return,
                };
                
                if tinyfiledialogs::message_box_yes_no(
                    "Grundbuch neu laden?",
                    &format!("Wenn das Grundbuch neu analysiert wird, werden alle manuell eingegebenen Daten überschrieben.\r\nFortfahren?"),
                    MessageBoxIcon::Warning,
                    YesNo::No,
                ) == YesNo::No {
                    return;
                }
                
                open_file.geladen.clear();
                open_file.analysiert = Grundbuch::default();
            }
            
            webview.eval(&format!("replaceEntireScreen(`{}`)", ui::render_entire_screen(data)));
            
            let open_file = match data.loaded_files.get(&file_id) { 
                Some(s) => s,
                None => return,
            };
                
            let file_name = format!("{}_{}", open_file.titelblatt.grundbuch_von, open_file.titelblatt.blatt);
            let output_parent = open_file.get_gbx_datei_parent();
            let cache_output_path = output_parent.clone().join(&format!("{}.cache.gbx", file_name));
            let _ = reload_grundbuch(open_file.clone());
            
            webview.eval(&format!("startCheckingForPageLoaded(`{}`, `{}`)", cache_output_path.display(), file_name));
        },
        Cmd::ZeileNeu { file, page, y } => {
            
            if data.loaded_files.is_empty() {
                return;
            }
            
            let open_file = match data.loaded_files.get_mut(&file.clone()) { 
                Some(s) => s,
                None => return,
            };
            
            let mut ap = open_file.anpassungen_seite
                .entry(*page)
                .or_insert_with(|| AnpassungSeite::default());

            let (im_width, im_height, page_width, page_height) = match open_file.pdftotext_layout.seiten.get(&(*page as u32)) {
                Some(o) => (o.breite_mm as f32 / 25.4 * 600.0, o.hoehe_mm as f32 / 25.4 * 600.0, o.breite_mm, o.hoehe_mm),
                None => return,
            };
            
            let img_ui_width = 1200.0; // px
            let aspect_ratio = im_height / im_width;
            let img_ui_height = img_ui_width * aspect_ratio;
            
            if *y > img_ui_height || *y < 0.0 {
                return;
            }
                        
            ap.zeilen.push(y / img_ui_height * page_height);
            ap.zeilen.sort_by(|a, b| ((a * 1000.0) as usize).cmp(&((b * 1000.0) as usize)));
            ap.zeilen.dedup();

            webview.eval(&format!("replacePdfImageZeilen(`{}`)", crate::ui::render_pdf_image_zeilen(&ap.zeilen, page_height, img_ui_height)));            
            
            // speichern
            open_file.speichern();
        },
        Cmd::ZeileLoeschen { file, page, zeilen_id } => {
        
            if data.loaded_files.is_empty() {
                return;
            }
            
            let open_file = match data.loaded_files.get_mut(&file.clone()) { 
                Some(s) => s,
                None => return,
            };
            
            let (im_width, im_height, page_width, page_height) = match open_file.pdftotext_layout.seiten.get(&(*page as u32)) {
                Some(o) => (o.breite_mm as f32 / 25.4 * 600.0, o.hoehe_mm as f32 / 25.4 * 600.0, o.breite_mm, o.hoehe_mm),
                None => return,
            };
    
            let img_ui_width = 1200.0; // px
            let aspect_ratio = im_height / im_width;
            let img_ui_height = img_ui_width * aspect_ratio;
            
            if let Some(ap) = open_file.anpassungen_seite.get_mut(page) {
                if *zeilen_id < ap.zeilen.len() {
                    let _ = ap.zeilen.remove(*zeilen_id);                    
                    webview.eval(&format!("replacePdfImageZeilen(`{}`)", crate::ui::render_pdf_image_zeilen(&ap.zeilen, page_height, img_ui_height)));            
                }
            }
            
            // speichern
            open_file.speichern();
        },
        Cmd::ResizeColumn {
            direction,
            column_id,
            x, y,
        } => {
                        
            if data.loaded_files.is_empty() {
                return;
            }
            
            let (file_id, page) = match data.open_page.clone() {
                Some((file, page)) => (file.clone(), page as usize),
                None => return,
            };
            
            let open_file = match data.loaded_files.get_mut(&file_id) { 
                Some(s) => s,
                None => return,
            };
            
            let open_page = match open_file.geladen.get_mut(&(page as u32)) {
                Some(s) => s,
                None => return,
            };
            
            let seitentyp = match open_file.klassifikation_neu.get(&page) {
                Some(s) => *s,
                None => open_page.typ,
            };
            
            let current_column = match seitentyp.get_columns(open_file.anpassungen_seite.get(&page)).iter().find(|col| col.id == column_id) {
                Some(s) => s.clone(),
                None => return,
            };
            
            let (im_width, im_height, page_width, page_height) = match open_file.pdftotext_layout.seiten.get(&(page as u32)) {
                Some(o) => (o.breite_mm as f32 / 25.4 * 600.0, o.hoehe_mm as f32 / 25.4 * 600.0, o.breite_mm, o.hoehe_mm),
                None => return,
            };
        
            let img_ui_width = 1200.0; // px
            let aspect_ratio = im_height / im_width;
            let img_ui_height = img_ui_width * aspect_ratio;
    
            {
                let rect_to_modify = open_file.anpassungen_seite
                .entry(page)
                .or_insert_with(|| AnpassungSeite::default())
                .spalten
                .entry(column_id.clone())
                .or_insert_with(|| Rect {
                    min_x: current_column.min_x,
                    max_x: current_column.max_x,
                    min_y: current_column.min_y,
                    max_y: current_column.max_y,
                });
                
                match direction.as_str() {
                    "nw" => { 
                        rect_to_modify.min_y = y / img_ui_height * page_height; 
                        rect_to_modify.min_x = x / img_ui_width * page_width; 
                    },
                    "ne" => { 
                        rect_to_modify.min_y = y / img_ui_height * page_height; 
                        rect_to_modify.max_x = x / img_ui_width * page_width; 
                    },
                    "se" => { 
                        rect_to_modify.max_y = y / img_ui_height * page_height; 
                        rect_to_modify.max_x = x / img_ui_width * page_width; 
                    },
                    "sw" => { 
                        rect_to_modify.max_y = y / img_ui_height * page_height; 
                        rect_to_modify.min_x = x / img_ui_width * page_width; 
                    },
                    _ => return,
                };
            }
            
            let new_column = match seitentyp.get_columns(open_file.anpassungen_seite.get(&page)).iter().find(|col| col.id == column_id) {
                Some(s) => s.clone(),
                None => return,
            };
            
            let new_width = (new_column.max_x - new_column.min_x).abs() / page_width * img_ui_width;
            let new_height = (new_column.max_y - new_column.min_y).abs() / page_height * img_ui_height;
            let new_x = new_column.min_x.min(new_column.max_x) / page_width * img_ui_width;
            let new_y = new_column.min_y.min(new_column.max_y) / page_height * img_ui_height;

            // speichern
            open_file.speichern();
        
            webview.eval(&format!("adjustColumn(`{}`,`{}`,`{}`,`{}`,`{}`)", column_id, new_width, new_height, new_x, new_y));
        },
        Cmd::ToggleCheckBox { checkbox_id } => {
            match checkbox_id.as_str() {
                "konfiguration-zeilenumbrueche-in-ocr-text" => {
                    data.konfiguration.zeilenumbrueche_in_ocr_text = !data.konfiguration.zeilenumbrueche_in_ocr_text;
                },
                "konfiguration-spalten-ausblenden" => {
                    data.konfiguration.spalten_ausblenden = !data.konfiguration.spalten_ausblenden;
                },
                "konfiguration-keine-roten-linien" => {
                    data.konfiguration.vorschau_ohne_geroetet = !data.konfiguration.vorschau_ohne_geroetet;
                },
                _ => return,
            }
            
            data.konfiguration.speichern();
        },
        Cmd::ImportNebenbeteiligte => {
            
            if data.loaded_files.is_empty() {
                return;
            }
            
            let file_dialog_result = tinyfiledialogs::open_file_dialog(
                "Nebenbeteiligte Ordnungsnummern auswählen", 
                "~/", 
                Some((&["*.tsv"], "Nebenbeteiligte")),
            );
            
            let f_name = match file_dialog_result {
                Some(s) => s,
                None => return,
            };
            
            let fs = match fs::read_to_string(&f_name).ok() {
                Some(s) => s,
                None => return,
            };
            
            let mut nb = parse_nb(&fs);
            
            // Vergebe Ordnungsnummern, wenn nicht bereits erledigt
            let n_ohne_onr = nb.iter().filter(|n| n.ordnungsnummer.is_none()).count();
            
            if n_ohne_onr > 0 {
                use tinyfiledialogs::{YesNo, MessageBoxIcon};
                
                if tinyfiledialogs::message_box_yes_no(
                    "Ordnungsnummern automatisch vergeben?",
                    &format!("In der Datei {} wurden {} Einträge ohne Ordnungsnummern gefunden.\r\n\r\nSollen die Ordnungsnummern automatisch vergeben werden?", 
                        Path::new(&f_name).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or(f_name.clone()),
                        n_ohne_onr,
                    ),
                    MessageBoxIcon::Question,
                    YesNo::Yes,
                ) == YesNo::Yes {
                    Nebenbeteiligter::ordnungsnummern_automatisch_vergeben(&mut nb);            
                }
            }
            
            data.loaded_nb = nb;
            data.loaded_nb_paths.push(f_name.clone());
            
            for open_file in data.loaded_files.values_mut() {
                open_file.nebenbeteiligte_dateipfade.push(f_name.clone());
                open_file.nebenbeteiligte_dateipfade.sort();
                open_file.nebenbeteiligte_dateipfade.dedup();
                open_file.speichern();
            }
            
            // Nochmal speichern, nachdem Ordnungsnummern neu vergeben wurden
            let tsv = get_nebenbeteiligte_tsv(&data);
            let _ = fs::write(f_name, tsv.as_bytes());
                    
            let open_file = match data.open_page.clone().and_then(|(file, _)| data.loaded_files.get(&file)) { 
                Some(s) => s,
                None => return,
            };
            
            webview.eval(&format!("replaceAnalyseGrundbuch(`{}`);", ui::render_analyse_grundbuch(&open_file, &data.loaded_nb, &data.konfiguration, false, false)));
        },
        Cmd::ExportNebenbeteiligte => {
        
            if data.loaded_files.is_empty() {
                return;
            }
            
            let file_dialog_result = tinyfiledialogs::save_file_dialog(
                "Nebenbeteiligte .TSV speichern unter", 
                "~/", 
            );
            
            let f = match file_dialog_result {
                Some(f) => {
                    if f.ends_with(".tsv") {
                        f
                    } else {
                        format!("{}.tsv", f)
                    }
                },
                None => return,
            };
            
            let tsv = get_nebenbeteiligte_tsv(&data);

            let _ = std::fs::write(&f, tsv.as_bytes());
            
        },
        Cmd::GrundbuchExportieren {
            was_exportieren, 
            exportiere_bv,
            exportiere_abt_1,
            exportiere_abt_2,
            exportiere_abt_3,
            exportiere_pdf_leere_seite,
            exportiere_geroetete_eintraege,
            exportiere_in_eine_einzelne_datei,
        } => {
            use crate::pdf::{GrundbuchExportConfig, PdfExportTyp, GenerateGrundbuchConfig};
            use tinyfiledialogs::MessageBoxIcon;
            
            if data.loaded_files.is_empty() {
                return;
            }
            
            let target = match exportiere_in_eine_einzelne_datei {
                true => {
                    let file_dialog_result = tinyfiledialogs::save_file_dialog(
                        "PDF Datei speichern unter", 
                        "~/",
                    );
                    
                    let f = match file_dialog_result {
                        Some(f) => f,
                        None => return,
                    };
                    
                    GenerateGrundbuchConfig::EinzelneDatei {
                        datei: f,
                        exportiere_bv: *exportiere_bv,
                        exportiere_abt1: *exportiere_abt_1,
                        exportiere_abt2: *exportiere_abt_2,
                        exportiere_abt3: *exportiere_abt_3,
                        leere_seite_nach_titelblatt: *exportiere_pdf_leere_seite,
                        mit_geroeteten_eintraegen: *exportiere_geroetete_eintraege,
                    }
                },
                false => {
                    let file_dialog_result = tinyfiledialogs::select_folder_dialog(
                        "PDF Dateien speichern unter", 
                        "~/", 
                    );
                    
                    let f = match file_dialog_result {
                        Some(f) => f,
                        None => return,
                    };
                    
                    GenerateGrundbuchConfig::MehrereDateien {
                        ordner: f,
                        exportiere_bv: *exportiere_bv,
                        exportiere_abt1: *exportiere_abt_1,
                        exportiere_abt2: *exportiere_abt_2,
                        exportiere_abt3: *exportiere_abt_3,
                        leere_seite_nach_titelblatt: *exportiere_pdf_leere_seite,
                        mit_geroeteten_eintraegen: *exportiere_geroetete_eintraege,
                    }
                },
            };
            
            let source = match was_exportieren.as_str() {
                "offen" => {

                    let (file_id, page) = match data.open_page.clone() {
                        Some((file, page)) => (file.clone(), page as usize),
                        None => return,
                    };
                    
                    let mut open_file = match data.loaded_files.get(&file_id) { 
                        Some(s) => s.clone(),
                        None => return,
                    };
                    
                    open_file.analysiert.titelblatt = open_file.titelblatt.clone();
                    PdfExportTyp::OffenesGrundbuch(open_file.analysiert.clone())
                },
                "alle-offen-digitalisiert" => {
                
                    let files = data.loaded_files.values()
                    .filter_map(|f| {
                        if f.datei.is_none() { return None; }
                        Some(f.clone())
                    })
                    .map(|mut f| {
                        f.analysiert.titelblatt = f.titelblatt.clone();
                        f.analysiert
                    })
                    .collect::<Vec<_>>();
                    
                    PdfExportTyp::AlleOffenDigitalisiert(files)
                },
                "alle-offen" => {
                    let files = data.loaded_files.values().map(|f| {
                        f.clone()
                    })
                    .map(|mut f| {
                        f.analysiert.titelblatt = f.titelblatt.clone();
                        f.analysiert
                    }).collect::<Vec<_>>();
                    
                    PdfExportTyp::AlleOffen(files)
                },
                "alle-original" => {
                    
                    let files = data.loaded_files.values()
                    .filter_map(|f| f.datei.clone())
                    .collect::<Vec<_>>();
                    
                    PdfExportTyp::AlleOriginalPdf(files)
                },
                _ => { return; },
            };
            
            let result =  pdf::export_grundbuch(GrundbuchExportConfig {
                exportiere: source,
                optionen: target,
            });
            
            if let Err(r) = result {
                let file_dialog_result = tinyfiledialogs::message_box_ok(
                    "Fehler beim Exportieren des PDFs", 
                    &r, 
                    MessageBoxIcon::Error,
                ); 
            }
        },
        Cmd::ExportAlleRechte => {
        
            if data.loaded_files.is_empty() {
                return;
            }
            
            let file_dialog_result = tinyfiledialogs::save_file_dialog(
                "Rechte .HTML speichern unter", 
                "~/", 
            );
            
            let f = match file_dialog_result {
                Some(f) => {
                    if f.ends_with(".html") {
                        f
                    } else {
                        format!("{}.html", f)
                    }
                },
                None => return,
            };
            
            
            let html = format!("<html><head><style>* {{ margin:0px;padding:0px; }}</style></head><body>{}</body>", 
                get_alle_rechte_html(&data)
            );
            
            let _ = std::fs::write(&f, html.as_bytes());
        },
        Cmd::ExportAlleFehler => {
        
            if data.loaded_files.is_empty() {
                return;
            }

            let file_dialog_result = tinyfiledialogs::save_file_dialog(
                "Rechte .HTML speichern unter", 
                "~/", 
            );
            
            let f = match file_dialog_result {
                Some(f) => {
                    if f.ends_with(".html") {
                        f
                    } else {
                        format!("{}.html", f)
                    }
                },
                None => return,
            };
            
            let html = format!("<html><head><style>* {{ margin:0px;padding:0px; }}</style></head><body>{}</body>", 
                get_alle_fehler_html(&data)
            );
            
            let _ = std::fs::write(&f, html.as_bytes());
        },
        Cmd::ExportAlleAbt1 => {
        
            if data.loaded_files.is_empty() {
                return;
            }

            let file_dialog_result = tinyfiledialogs::save_file_dialog(
                "Rechte .HTML speichern unter", 
                "~/", 
            );
            
            let f = match file_dialog_result {
                Some(f) => {
                    if f.ends_with(".html") {
                        f
                    } else {
                        format!("{}.html", f)
                    }
                },
                None => return,
            };
            
            let html = format!("<html><head><style>* {{ margin:0px;padding:0px; }}</style></head><body>{}</body>", 
                get_alle_abt1_html(&data)
            );
            
            let _ = std::fs::write(&f, html.as_bytes());
        },
        Cmd::ExportAlleTeilbelastungen => {
        
            if data.loaded_files.is_empty() {
                return;
            }

            let file_dialog_result = tinyfiledialogs::save_file_dialog(
                "Teilbelastungen .HTML speichern unter", 
                "~/", 
            );
            
            let f = match file_dialog_result {
                Some(f) => {
                    if f.ends_with(".html") {
                        f
                    } else {
                        format!("{}.html", f)
                    }
                },
                None => return,
            };
            
            let html = format!("<html><head><style>* {{ margin:0px;padding:0px; }}</style></head><body>{}</body>", 
                get_alle_teilbelastungen_html(&data)
            );
            
            let _ = std::fs::write(&f, html.as_bytes());
        },
        Cmd::ExportLefis => {

            if data.loaded_files.is_empty() {
                return;
            }
            
            let analysiert = data.loaded_files.values().map(|file| {
                LefisDateiExport {
                    rechte: crate::analysiere::analysiere_grundbuch(&file.analysiert, &data.loaded_nb, &data.konfiguration),
                    titelblatt: file.analysiert.titelblatt.clone(),
                }
            }).collect::<Vec<_>>();
            
            let json = match serde_json::to_string_pretty(&analysiert) {
                Ok(o) => o,
                Err(_) => return,
            };
            
            let json = json.lines().collect::<Vec<_>>().join("\r\n");
            
            // Benutzer warnen, falls Datei noch Fehler enthält
            let mut fehler = analysiert.iter().flat_map(|l| {
                l.rechte.abt2.iter().filter_map(|f| {
                    if f.fehler.is_empty() { None } else { Some(format!("{} Blatt {}, Abt 2 lfd. Nr. {}", l.titelblatt.grundbuch_von, l.titelblatt.blatt, f.lfd_nr)) }
                }).collect::<Vec<_>>()
            }).collect::<Vec<_>>();
            
            fehler.extend(analysiert.iter().flat_map(|l| {
                l.rechte.abt3.iter().filter_map(|f| {
                    if f.fehler.is_empty() { None } else { Some(format!("{} Blatt {}, Abt 3 lfd. Nr. {}", l.titelblatt.grundbuch_von, l.titelblatt.blatt, f.lfd_nr)) }
                }).collect::<Vec<_>>()
            }));

            if !fehler.is_empty() {
                use tinyfiledialogs::{YesNo, MessageBoxIcon};
                
                if tinyfiledialogs::message_box_yes_no(
                    "Mit Fehlern exportieren?",
                    &format!("Die folgenden Einträge enthalten Fehler:\r\n\r\n{}\r\n\r\nTrotzdem .lefis-Datei exportieren?", fehler.join("\r\n")),
                    MessageBoxIcon::Warning,
                    YesNo::No,
                ) == YesNo::No {
                    return;
                }
            }
            
            let file_dialog_result = tinyfiledialogs::save_file_dialog(
                ".lefis-Datei speichern unter", 
                "~/", 
            );
            
            let f = match file_dialog_result {
                Some(f) => {
                    if f.ends_with(".lefis") {
                        f
                    } else {
                        format!("{}.lefis", f)
                    }
                },
                None => return,
            };
            
            let _ = std::fs::write(&f, json.as_bytes());
            
        },
        Cmd::EditRegexKey { old_key, new_key } => {
            let old_key: String = old_key.chars().filter(|c| !c.is_whitespace()).collect();
            let new_key: String = new_key.chars().filter(|c| !c.is_whitespace()).collect();
            if data.konfiguration.regex.get(&new_key).is_some() {
                return;
            }
            let cur_value = data.konfiguration.regex.get(&old_key).cloned().unwrap_or_default();
            data.konfiguration.regex.remove(&old_key);
            data.konfiguration.regex.insert(new_key, cur_value);
            data.konfiguration.speichern();
        },
        Cmd::EditRegexValue { key, value } => {
            let key: String = key.chars().filter(|c| !c.is_whitespace()).collect();
            let value: String = value.chars().filter(|c| *c != '\n').collect();
            data.konfiguration.regex.insert(key, value);
            data.konfiguration.speichern();
        },
        Cmd::InsertRegex { regex_key } => {
            data.konfiguration.regex.insert(format!("{}_1", regex_key), "(.*)".to_string());
            data.konfiguration.speichern();
            webview.eval(&format!("replaceEntireScreen(`{}`)", ui::render_entire_screen(data)));
        },
        Cmd::RegexLoeschen { regex_key } => {
            data.konfiguration.regex.remove(regex_key);
            if data.konfiguration.regex.is_empty() {
                data.konfiguration.regex.insert("REGEX_ID".to_string(), "(.*)".to_string());
            }       
            data.konfiguration.speichern();
            webview.eval(&format!("replaceEntireScreen(`{}`)", ui::render_entire_screen(data)));
        },
        Cmd::TesteRegex { regex_id, text } => {
            let result = teste_regex(&regex_id, text.trim(), &data.konfiguration);
            let result = match result {
                Ok(o) => {
                    if o.is_empty() {
                        format!("[]")
                    } else {
                        o.into_iter()
                        .enumerate()
                        .map(|(col, v)| format!("[{}]: \"{}\"", col, v))
                        .collect::<Vec<_>>()
                        .join("\r\n")
                    }
                },
                Err(e) => { format!("{}", e) },
            };
            webview.eval(&format!("replaceRegexTestOutput(`{}`);", result));
        },
        Cmd::SetActiveRibbonTab { new_tab } => {
            data.active_tab = *new_tab;
            webview.eval(&format!("replaceRibbon(`{}`);", ui::render_ribbon(&data)));
        },
        Cmd::SetOpenFile { new_file } => {
            data.open_page = Some((new_file.clone(), 2));
            
            let open_file = match data.open_page.clone().and_then(|(file, _)| data.loaded_files.get(&file)) { 
                Some(s) => s,
                None => return,
            };
            
            let titelblatt = open_file.titelblatt.clone();
            let seitenzahlen = open_file.pdftotext_layout.seiten.keys().copied().collect::<Vec<_>>();
            if let Some(pdf) = open_file.datei.as_ref() {
    
                let pdf_bytes = match std::fs::read(&pdf) {
                    Ok(o) => o,
                    Err(_) => return,
                };

                rayon::spawn(move || {

                    use crate::digitalisiere::konvertiere_pdf_seiten_zu_png;
        
                    let max_sz = seitenzahlen.iter().max().cloned().unwrap_or(0);

                    let _ = konvertiere_pdf_seiten_zu_png(
                        &pdf_bytes, 
                        &seitenzahlen, 
                        max_sz,
                        &titelblatt,
                    );
                });
            }
            
            webview.eval(&format!("replacePageList(`{}`);", ui::render_page_list(&data)));
            webview.eval(&format!("replaceMainNoFiles(`{}`);", ui::render_application_main_no_files(data)));
        },
        Cmd::SetOpenPage { active_page } => {
            
            if let Some(p) = data.open_page.as_mut() { 
                p.1 = *active_page;
            }
            
            
            let open_file = match data.open_page.clone().and_then(|(file, _)| data.loaded_files.get(&file)) { 
                Some(s) => s,
                None => return,
            };
            
            // webview.eval(&format!("replaceMainContainer(`{}`);", ui::render_main_container(data)));
            webview.eval(&format!("replaceBestandsverzeichnis(`{}`);", ui::render_bestandsverzeichnis(open_file, &data.konfiguration)));
            webview.eval(&format!("replaceBestandsverzeichnisZuschreibungen(`{}`);", ui::render_bestandsverzeichnis_zuschreibungen(open_file)));
            webview.eval(&format!("replaceBestandsverzeichnisAbschreibungen(`{}`);", ui::render_bestandsverzeichnis_abschreibungen(open_file)));
            webview.eval(&format!("replaceAbt1(`{}`);", ui::render_abt_1(open_file)));
            webview.eval(&format!("replaceAbt1GrundlagenEintragungen(`{}`);", ui::render_abt_1_grundlagen_eintragungen(open_file)));
            webview.eval(&format!("replaceAbt1Veraenderungen(`{}`);", ui::render_abt_1_veraenderungen(open_file)));
            webview.eval(&format!("replaceAbt1Loeschungen(`{}`);", ui::render_abt_1_loeschungen(open_file)));
            webview.eval(&format!("replaceAbt2(`{}`);", ui::render_abt_2(open_file)));
            webview.eval(&format!("replaceAbt2Veraenderungen(`{}`);", ui::render_abt_2_veraenderungen(open_file)));
            webview.eval(&format!("replaceAbt2Loeschungen(`{}`);", ui::render_abt_2_loeschungen(open_file)));
            webview.eval(&format!("replaceAbt3(`{}`);", ui::render_abt_3(open_file, data.konfiguration.lefis_analyse_einblenden)));
            webview.eval(&format!("replaceAbt3Veraenderungen(`{}`);", ui::render_abt_3_veraenderungen(open_file)));
            webview.eval(&format!("replaceAbt3Loeschungen(`{}`);", ui::render_abt_3_loeschungen(open_file)));
            webview.eval(&format!("replaceAnalyseGrundbuch(`{}`);", ui::render_analyse_grundbuch(&open_file, &data.loaded_nb, &data.konfiguration, false, false))); 
            webview.eval(&format!("replaceFileList(`{}`);", ui::render_file_list(&data)));
            webview.eval(&format!("replacePageList(`{}`);", ui::render_page_list(&data)));
            webview.eval(&format!("replacePageImage(`{}`);", ui::render_pdf_image(&data)));
        },
    }
}

fn parse_nb(fs: &str) -> Vec<Nebenbeteiligter> {

    let mut nb = Vec::new();

    for line in fs.lines() {
        if line.starts_with("ORDNUNGSNUMMER") { continue; }
        let values = line.split("\t").collect::<Vec<_>>();
        
        let mut b = Nebenbeteiligter {
            ordnungsnummer: None,
            name: String::new(),
            typ: None,
            extra: NebenbeteiligterExtra::default(),
        };
        
        for v in 0..10 {
            let s = match values.get(v) {
                Some(s) => s,
                None => continue,
            };
            if s.trim().is_empty() {
                continue;
            }
            
            match v {
                0 => {
                    if let Some(s) = s.parse::<usize>().ok() {
                        b.ordnungsnummer = Some(s);
                    }
                },
                1 => {
                    if let Some(typ) = NebenbeteiligterTyp::from_type_str(s.trim()) {
                        b.typ = Some(typ);
                    }
                },
                2 => { },
                3 => {
                    b.name = s.trim().to_string();
                },
                4 => {
                    if let Some(anrede) = Anrede::from_str(s.trim()) {
                        b.extra.anrede = Some(anrede);
                    }
                },
                5 => {
                    if !s.trim().is_empty() {
                        b.extra.titel = Some(s.trim().to_string());                            
                    }
                },
                6 => {
                    if !s.trim().is_empty() {
                        b.extra.vorname = Some(s.trim().to_string());                            
                    }
                },
                7 => {
                    if !s.trim().is_empty() {
                        b.extra.nachname_oder_firma = Some(s.trim().to_string());                            
                    }
                },
                8 => {
                    if !s.trim().is_empty() {
                        b.extra.geburtsname = Some(s.trim().to_string());                            
                    }
                },
                9 => {
                    if let Some(datum) = NebenbeteiligterExtra::geburtsdatum_from_str(s.trim()) {
                        b.extra.geburtsdatum = Some(datum);                            
                    }
                },
                10 => {
                    if !s.trim().is_empty() {
                        b.extra.wohnort = Some(s.trim().to_string());                            
                    }
                },
                _ => { },
            }
        }
        
        nb.push(b);
    }
    
    nb
}

fn get_alle_rechte_html(data: &RpcData) -> String {

    let mut entries = Vec::new();
    
    for (f_name, f) in data.loaded_files.iter() {
        entries.push(crate::ui::render_analyse_grundbuch(f, &data.loaded_nb, &data.konfiguration, true, false));
    }
    
    entries.join("\r\n")
}


fn get_alle_teilbelastungen_html(data: &RpcData) -> String {
    
    let mut entries = String::new();
    
    for (f_name, f) in data.loaded_files.iter() {
        
        let gb_analysiert = crate::analysiere::analysiere_grundbuch(
            &f.analysiert, 
            &data.loaded_nb, 
            &data.konfiguration
        );
    
        let mut abt2_entries = String::new();
        
        for abt2 in gb_analysiert.abt2.iter() {
            
            let has_nur_lastend_an = abt2.lastend_an
                .iter()
                .any(|s1| !s1.nur_lastend_an.is_empty());
            
            if has_nur_lastend_an {
                let blatt = &f.titelblatt.grundbuch_von;
                let nr = &f.titelblatt.blatt;
                let lfd_nr = &abt2.lfd_nr;
                let text = &abt2.text_original;
                abt2_entries.push_str(&format!("<div style='display:flex;flex-direction:column;margin:10px;padding:10px;border:1px solid #efefef;page-break-inside:avoid;'><strong>{blatt} Nr. {nr}, A2 / {lfd_nr}</strong><div style='display:flex;flex-direction:row;'><div style='margin-right:10px;'>{text}</div><div>"));
            }
            
            for spalte_1 in abt2.lastend_an.iter() {
                if !spalte_1.nur_lastend_an.is_empty() {
                    abt2_entries.push_str(&format!("<div style='min-width: 350px;'>"));

                    for e in spalte_1.nur_lastend_an.iter() {
                        let flur = &e.flur;
                        let flurstueck = &e.flurstueck;
                        let gemarkung = &e.gemarkung.as_ref().unwrap_or(&f.titelblatt.grundbuch_von);
                        abt2_entries.push_str(&format!("<p>Gemarkung {gemarkung}, Flur {flur}, Flurstück {flurstueck}</p>"));    
                    }
                    
                    abt2_entries.push_str(&format!("</div>"));
                }
            }
            
            if has_nur_lastend_an {
                abt2_entries.push_str(&format!("</div></div></div>"));
            }
        }
        entries.push_str(&abt2_entries);

        let mut abt3_entries = String::new();

        for abt3 in gb_analysiert.abt3.iter() {
        
            let has_nur_lastend_an = abt3.lastend_an
                .iter()
                .any(|s1| !s1.nur_lastend_an.is_empty());
            
            if has_nur_lastend_an {
                let blatt = &f.titelblatt.grundbuch_von;
                let nr = &f.titelblatt.blatt;
                let lfd_nr = &abt3.lfd_nr;
                let text = &abt3.text_original;
                abt3_entries.push_str(&format!("<div style='display:flex;flex-direction:column;margin:10px;padding:10px;border:1px solid #efefef;page-break-inside:avoid;'><strong>{blatt} Nr. {nr}, A3 / {lfd_nr}</strong><div style='display:flex;flex-direction:row;'><div style='margin-right:10px;'>{text}</div><div>"));
            }
            
            for spalte_1 in abt3.lastend_an.iter() {
                if !spalte_1.nur_lastend_an.is_empty() {
                    abt3_entries.push_str(&format!("<div style='min-width: 350px;'>"));

                    for e in spalte_1.nur_lastend_an.iter() {
                        let flur = &e.flur;
                        let flurstueck = &e.flurstueck;
                        let gemarkung = &e.gemarkung.as_ref().unwrap_or(&f.titelblatt.grundbuch_von);
                        abt3_entries.push_str(&format!("<p>Gemarkung {gemarkung}, Flur {flur}, Flurstück {flurstueck}</p>"));
                        
                    }
                    
                    abt3_entries.push_str(&format!("</div>"));

                }
            }
            
            if has_nur_lastend_an {
                abt3_entries.push_str(&format!("</div></div></div>"));
            }
        }
        
        entries.push_str(&abt3_entries);
    }
    
    entries
}

fn get_alle_abt1_html(data: &RpcData) -> String {
        
    let mut entries = String::new();
    
    for (f_name, f) in data.loaded_files.iter() {
        
        let blatt = &f.titelblatt.grundbuch_von;
        let nr = &f.titelblatt.blatt;                        
        entries.push_str(&format!("<div><p>{blatt} Nr. {nr}</p>", ));
                    
        for abt1 in f.analysiert.abt1.eintraege.iter().filter_map(|a1| match a1 { Abt1Eintrag::V2(v2) => Some(v2), _ => None, }) {
            if abt1.ist_geroetet() {
                continue;
            }
            let lfd_nr = &abt1.lfd_nr;
            let text = &abt1.eigentuemer.text();
            entries.push_str(&format!("<div><p>{lfd_nr}</p><p>{text}</p></div>"));
        }
        
        entries.push_str(&format!("</div>"));
    }
    
    entries
}

fn get_alle_fehler_html(data: &RpcData) -> String {

    let mut entries = Vec::new();
    
    for (f_name, f) in data.loaded_files.iter() {
        entries.push(crate::ui::render_analyse_grundbuch(f, &data.loaded_nb, &data.konfiguration, true, true));
    }
    
    entries.join("\r\n")
}

fn get_rangvermerke_tsv(data: &RpcData) -> String {

    let mut entries = Vec::new();
    
    for (f_name, f) in data.loaded_files.iter() {
        let analysiert = crate::analysiere::analysiere_grundbuch(&f.analysiert, &[], &data.konfiguration);
        
        for a2 in analysiert.abt2 {
            if let Some(s) = a2.rangvermerk {
                entries.push(format!("{} A2/{}\t{}\t{}", f_name, a2.lfd_nr, s, a2.text_original));
            }
        }
    }
    
    format!("RECHT\tRVM\tTEXT\r\n{}", entries.join("\r\n"))
}

fn get_nebenbeteiligte_tsv(data: &RpcData) -> String {

    let mut nb = data.loaded_files.values()
    .flat_map(|file| file.get_nebenbeteiligte(&data.konfiguration))
    .collect::<Vec<_>>();
    
    for n in nb.iter_mut() {
        if n.ordnungsnummer.is_none() {
            if let Some(exist) = data.loaded_nb.iter().find(|q| q.name == n.name) {
                n.ordnungsnummer = exist.ordnungsnummer;
            }
        }

        if n.typ.is_none() {
            if let Some(exist) = data.loaded_nb.iter().find(|q| q.name == n.name) {
                n.typ = exist.typ;
            }
        }
    }

    let mut nb_keyed = BTreeMap::new();
    let mut rechte = BTreeMap::new();
    for n in nb {
        rechte.entry(n.name.clone()).or_insert_with(|| Vec::new()).push(n.recht.clone());
        nb_keyed.insert(n.name.clone(), n);
    }
    
    let mut nb = nb_keyed
        .into_iter()
        .map(|(k, v)| v)
        .collect::<Vec<_>>();
    nb.sort_by(|a, b| a.name.cmp(&b.name));
    nb.dedup();
    
    let tsv = nb.iter()
        .map(|nb| format!("{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", 
            nb.ordnungsnummer.map(|s| s.to_string()).unwrap_or_default(), 
            nb.typ.map(|s| s.get_str()).unwrap_or_default(), 
            rechte.get(&nb.name).cloned().unwrap_or_default().join("; "),
            nb.name,
            nb.extra.anrede.map(|s| s.to_string()).unwrap_or_default(),
            nb.extra.titel.clone().unwrap_or_default(),
            nb.extra.vorname.clone().unwrap_or_default(),
            nb.extra.nachname_oder_firma.clone().unwrap_or_default(),
            nb.extra.geburtsname.clone().unwrap_or_default(),
            nb.extra.geburtsdatum.as_ref().map(|gb| NebenbeteiligterExtra::geburtsdatum_to_str(gb)).unwrap_or_default(),
            nb.extra.wohnort.clone().unwrap_or_default(),
        ))
        .collect::<Vec<_>>()
        .join("\r\n");
    let tsv = format!("ORDNUNGSNUMMER\tTYP\tRECHT\tNAME (GRUNDBUCH)\tANREDE\tTITEL\tVORNAME\tNACHNAME_ODER_FIRMA\tGEBURTSNAME\tGEBURTSDATUM\tWOHNORT\r\n{}", tsv);
    tsv
}

fn digitalisiere_dateien(pdfs: Vec<PdfFile>) {
    
    std::thread::spawn(move || {
        for mut pdf in pdfs {
        
            let output_parent = pdf.get_gbx_datei_parent();
            let file_name = format!("{}_{}", pdf.titelblatt.grundbuch_von, pdf.titelblatt.blatt);
            let cache_output_path = output_parent.clone().join(&format!("{}.cache.gbx", file_name));
            let target_output_path = output_parent.clone().join(&format!("{}.gbx", file_name));
            
            let pdf_datei_pfad = match pdf.datei.clone() {
                None => {
                    pdf.analysiert.abt1.migriere_v2();
                    let json = match serde_json::to_string_pretty(&pdf) { Ok(o) => o, Err(_) => return, };
                    let _ = std::fs::write(&target_output_path, json.as_bytes());
                    continue;
                },
                Some(pfad) => pfad,
            };
    
            rayon::spawn(move || {
            
                let mut pdf = pdf;
                
                let datei_bytes = match fs::read(&pdf_datei_pfad).ok() {
                    Some(s) => s,
                    None => return,
                };
                
                let max_sz = pdf.seitenzahlen.iter().max().cloned().unwrap_or(0);
                

                if let Some(mut cached_pdf) = std::fs::read_to_string(&cache_output_path).ok().and_then(|s| serde_json::from_str::<PdfFile>(&s).ok()) {
                    cached_pdf.analysiert.abt1.migriere_v2();
                    pdf = cached_pdf;
                }
                if let Some(mut target_pdf) = std::fs::read_to_string(&target_output_path).ok().and_then(|s| serde_json::from_str::<PdfFile>(&s).ok()) {
                    target_pdf.analysiert.abt1.migriere_v2();
                    pdf = target_pdf;
                }
                
                let seitenzahlen_zu_laden = pdf.seitenzahlen
                    .iter()
                    .filter(|sz| !pdf.geladen.contains_key(sz))
                    .copied()
                    .collect::<Vec<_>>();
                            
                let json = match serde_json::to_string_pretty(&pdf) { 
                    Ok(o) => o, 
                    Err(_) => return, 
                };
                            
                let _ = std::fs::write(&cache_output_path, json.as_bytes());
                
                let _ = digitalisiere::konvertiere_pdf_seiten_zu_png(&datei_bytes, &seitenzahlen_zu_laden, max_sz, &pdf.titelblatt);
                        
                for sz in seitenzahlen_zu_laden {
                    
                    pdf.seiten_versucht_geladen.insert(sz);
                    
                    let pdftotext_layout = match digitalisiere::get_pdftotext_layout(&pdf.titelblatt, &[sz]) { 
                        Ok(o) => o, 
                        Err(_) => continue, 
                    };
                    
                    for (k, v) in pdftotext_layout.seiten.iter() {
                        pdf.pdftotext_layout.seiten.insert(k.clone(), v.clone());
                    }
                    
                    let mut ocr_text_cached = pdf.seiten_ocr_text.get(&sz).cloned();
                    let mut ocr_text_final = None;
                    
                    if ocr_text_cached.is_none() {
                        match digitalisiere::ocr_seite(&pdf.titelblatt, sz, max_sz) {
                            Ok(o) => { 
                                pdf.seiten_ocr_text.insert(sz, o.clone());
                                ocr_text_final = Some(o); 
                            },
                            Err(_) => continue,
                        }
                    } else {
                        ocr_text_final = ocr_text_cached;
                    }
                    
                    let seitentyp = match pdf.klassifikation_neu.get(&(sz as usize)) {
                        Some(s) => *s,
                        None => {
                            match digitalisiere::klassifiziere_seitentyp(&pdf.titelblatt, sz, max_sz, ocr_text_final.as_ref()) { 
                                Ok(o) => o, 
                                Err(_) => continue, 
                            }
                        }
                    };
                    
                    let spalten = match digitalisiere::formularspalten_ausschneiden(
                        &pdf.titelblatt, 
                        sz, 
                        max_sz, 
                        seitentyp, 
                        &pdftotext_layout, 
                        pdf.anpassungen_seite.get(&(sz as usize))
                    ) { 
                        Ok(o) => o, 
                        Err(_) => continue, 
                    };
                    
                    if digitalisiere::ocr_spalten(&pdf.titelblatt, sz, max_sz, &spalten).is_err() { continue; }
                    
                    let textbloecke = match digitalisiere::textbloecke_aus_spalten(
                        &pdf.titelblatt, 
                        sz, 
                        max_sz,
                        &spalten, 
                        &pdftotext_layout,
                        pdf.anpassungen_seite.get(&(sz as usize))
                    ) { 
                        Ok(o) => o, 
                        Err(_) => continue, 
                    };
                    
                    pdf.geladen.insert(sz, SeiteParsed {
                        typ: seitentyp,
                        texte: textbloecke,
                    });

                    pdf.analysiert = match analysiere_grundbuch(&pdf) { 
                        Some(o) => o, 
                        None => continue, 
                    };
                    
                    let json = match serde_json::to_string_pretty(&pdf) { 
                        Ok(o) => o, 
                        Err(_) => continue, 
                    };

                    let _ = std::fs::write(&cache_output_path, json.as_bytes());
                }
                
                crate::digitalisiere::bv_eintraege_roeten(
                    &mut pdf.analysiert.bestandsverzeichnis.eintraege, 
                    &pdf.titelblatt, 
                    max_sz, 
                    &pdf.pdftotext_layout,
                );

                pdf.analysiert.abt1.migriere_v2();
                let json = match serde_json::to_string_pretty(&pdf) { Ok(o) => o, Err(_) => return, };
                let _ = std::fs::write(&target_output_path, json.as_bytes());
            });
        }
    });
}

fn analysiere_grundbuch(pdf: &PdfFile) -> Option<Grundbuch> {

    let bestandsverzeichnis = digitalisiere::analysiere_bv(&pdf.titelblatt, &pdf.pdftotext_layout, &pdf.geladen, &pdf.anpassungen_seite).ok()?;
    let mut abt1 = digitalisiere::analysiere_abt1(&pdf.geladen, &pdf.anpassungen_seite, &bestandsverzeichnis).ok()?;
    let abt2 = digitalisiere::analysiere_abt2(&pdf.geladen, &pdf.anpassungen_seite, &bestandsverzeichnis).ok()?;
    let abt3 = digitalisiere::analysiere_abt3(&pdf.geladen, &pdf.anpassungen_seite, &bestandsverzeichnis).ok()?;
    
    abt1.migriere_v2();
    
    let gb = Grundbuch {
        titelblatt: pdf.titelblatt.clone(),
        bestandsverzeichnis,
        abt1,
        abt2,
        abt3,
    };
        
    Some(gb)
}

fn reload_grundbuch(pdf: PdfFile) {

    use tinyfiledialogs::MessageBoxIcon;
            
    std::thread::spawn(move || {
        if let Err(e) = reload_grundbuch_inner(pdf) {
            tinyfiledialogs::message_box_ok(
                "Fehler",
                &format!("Fehler beim Laden des Grundbuchs: {:?}", e),
                MessageBoxIcon::Error,
            );
        }    
    });
}

fn reload_grundbuch_inner(mut pdf: PdfFile) -> Result<(), Fehler> {
    
    let pdf_datei = match pdf.datei.clone() {
        Some(s) => s,
        None => { return Ok(()); }
    };
    
    let datei_bytes = match fs::read(&pdf_datei) {
        Ok(s) => s,
        Err(e) => return Err(Fehler::Io(pdf_datei.clone(), e)),
    };
        
    let seitenzahlen_zu_laden = pdf.seitenzahlen.clone();
    let max_sz = pdf.seitenzahlen.iter().max().cloned().unwrap_or(0);
    
    let ist_geladen = pdf.ist_geladen();
    pdf.geladen.clear();
    pdf.analysiert = Grundbuch::default();
    
    let output_parent = pdf.get_gbx_datei_parent();
    let file_name = format!("{}_{}", pdf.titelblatt.grundbuch_von, pdf.titelblatt.blatt);
    let cache_output_path = output_parent.clone().join(&format!("{}.cache.gbx", file_name));
    let target_output_path = output_parent.clone().join(&format!("{}.gbx", file_name));
    
    crate::digitalisiere::bv_eintraege_roetungen_loeschen(
        &mut pdf.analysiert.bestandsverzeichnis.eintraege
    );
    
    for sz in seitenzahlen_zu_laden {
        
        pdf.seiten_versucht_geladen.insert(sz);

        let _ = digitalisiere::konvertiere_pdf_seiten_zu_png(&datei_bytes, &[sz], max_sz, &pdf.titelblatt)?;
                   
        let ocr_text_cached = pdf.seiten_ocr_text.get(&sz).cloned();
        let mut ocr_text_final = None;
        
        if ocr_text_cached.is_none() {
            match digitalisiere::ocr_seite(&pdf.titelblatt, sz, max_sz) {
                Ok(o) => { 
                    pdf.seiten_ocr_text.insert(sz, o.clone());
                    ocr_text_final = Some(o); 
                },
                Err(e) => {
                    println!("error: {:?}", e);
                    continue
                },
            }
        } else {
            ocr_text_final = ocr_text_cached;
        }
        
        let seitentyp = match pdf.klassifikation_neu.get(&(sz as usize)).cloned() {
            Some(s) => s,
            None => {
                match digitalisiere::klassifiziere_seitentyp(&pdf.titelblatt, sz, max_sz, ocr_text_final.as_ref()) { 
                    Ok(o) => o, 
                    Err(_) => continue, 
                }
            }
        };
        
        pdf.klassifikation_neu.insert(sz as usize, seitentyp);
                        
        let spalten = match digitalisiere::formularspalten_ausschneiden(
            &pdf.titelblatt, 
            sz, 
            max_sz, 
            seitentyp, 
            &pdf.pdftotext_layout, 
            pdf.anpassungen_seite.get(&(sz as usize)),
        ) { 
            Ok(o) => o, 
            Err(e) => continue, 
        };
                
        let _ = digitalisiere::ocr_spalten(&pdf.titelblatt, sz, max_sz, &spalten)?;

        let textbloecke = digitalisiere::textbloecke_aus_spalten(
            &pdf.titelblatt, 
            sz, 
            max_sz,
            &spalten, 
            &pdf.pdftotext_layout,
            pdf.anpassungen_seite.get(&(sz as usize)),
        )?;
      
        pdf.geladen.insert(sz, SeiteParsed {
            typ: seitentyp,
            texte: textbloecke.clone(),
        });

        pdf.analysiert = match analysiere_grundbuch(&pdf) { 
            Some(o) => o, 
            None => continue, 
        };
        
        crate::digitalisiere::bv_eintraege_roeten(
            &mut pdf.analysiert.bestandsverzeichnis.eintraege, 
            &pdf.titelblatt, 
            max_sz, 
            &pdf.pdftotext_layout,
        );
                        
        let json = match serde_json::to_string_pretty(&pdf) { 
            Ok(o) => o, 
            Err(_) => continue, 
        };

        let _ = std::fs::write(&cache_output_path, json.as_bytes());
    }
        
    pdf.analysiert = match analysiere_grundbuch(&pdf) { 
        Some(o) => o, 
        None => return Ok(()), 
    };
    
    crate::digitalisiere::bv_eintraege_roeten(
        &mut pdf.analysiert.bestandsverzeichnis.eintraege, 
        &pdf.titelblatt, 
        max_sz, 
        &pdf.pdftotext_layout,
    );
    
    let json = match serde_json::to_string_pretty(&pdf) { 
        Ok(o) => o, 
        Err(_) => return Ok(()), 
    };

    let _ = std::fs::write(&cache_output_path, json.as_bytes());
    let _ = std::fs::write(&target_output_path, json.as_bytes());

    Ok(())
}

pub fn python_exec_kuerze_text_abt3<'py>(
    py: Python<'py>,
    recht_id: &str,
    text_sauber: &str, 
    betrag: Option<String>,
    schuldenart: Option<String>,
    rechteinhaber: Option<String>,
    saetze_clean: &[String],
    py_code_lines: &[String], 
    konfiguration: &Konfiguration,
) -> Result<String, String> {

    use pyo3::prelude::*;
    use pyo3::types::{PyDict, PyList, PyTuple};

    use crate::kurztext::PyWaehrung;
        
    let script = py_code_lines
        .iter()
        .map(|l| format!("    {}", l))
        .collect::<Vec<_>>()
        .join("\r\n");
        
    let script = script.replace("\t", "    ");
    let script = script.replace("\u{00a0}", " ");
    let py_code = format!("import inspect\r\n\r\ndef run_script(*args, **kwargs):\r\n    saetze, betrag, schuldenart, rechtsinhaber, re, recht_id = args\r\n{}", script);
    let regex_values = konfiguration.regex.values().cloned().collect::<Vec<_>>();
    
    let saetze = PyList::new(py, saetze_clean.into_iter());

    let module = PyModule::from_code(py, &py_code, "", "main").map_err(|e| format!("{}", e))?;
    module.add_class::<RechteArtPyWrapper>().map_err(|e| format!("{}", e))?;
    module.add_class::<SchuldenArtPyWrapper>().map_err(|e| format!("{}", e))?;
    module.add_class::<CompiledRegex>().map_err(|e| format!("{}", e))?;
    module.add_class::<PyBetrag>().map_err(|e| format!("{}", e))?;
    module.add_class::<PyWaehrung>().map_err(|e| format!("{}", e))?;

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
        saetze.to_object(py), 
        betrag.unwrap_or_default().to_object(py), 
        schuldenart.unwrap_or_default().to_object(py), 
        rechteinhaber.unwrap_or_default().to_object(py), 
        regex_list.to_object(py),
        recht_id.to_string().to_object(py),
    ]);
    let result = fun.call1(py, tuple).map_err(|e| format!("{}", e))?;
    let extract = result.as_ref(py).extract::<String>().map_err(|e| format!("{}", e))?;
    
    Ok(extract)
}


pub fn python_exec_kuerze_text_abt2<'py>(
    py: Python<'py>,
    recht_id: &str,
    text_sauber: &str, 
    rechteinhaber: Option<String>,
    rangvermerk: Option<String>,
    saetze_clean: &[String],
    py_code_lines: &[String], 
    konfiguration: &Konfiguration,
) -> Result<String, String> {

    use pyo3::prelude::*;
    use pyo3::types::{PyDict, PyList, PyTuple};

    use crate::kurztext::PyWaehrung;
        
    let script = py_code_lines
        .iter()
        .map(|l| format!("    {}", l))
        .collect::<Vec<_>>()
        .join("\r\n");
        
    let script = script.replace("\t", "    ");
    let script = script.replace("\u{00a0}", " ");
    let py_code = format!("import inspect\r\n\r\ndef run_script(*args, **kwargs):\r\n    saetze, rechtsinhaber, rangvermerk, re, recht_id = args\r\n{}", script);
    let regex_values = konfiguration.regex.values().cloned().collect::<Vec<_>>();
    
    let saetze = PyList::new(py, saetze_clean.into_iter());

    let module = PyModule::from_code(py, &py_code, "", "main").map_err(|e| format!("{}", e))?;
    module.add_class::<RechteArtPyWrapper>().map_err(|e| format!("{}", e))?;
    module.add_class::<SchuldenArtPyWrapper>().map_err(|e| format!("{}", e))?;
    module.add_class::<CompiledRegex>().map_err(|e| format!("{}", e))?;
    module.add_class::<PyBetrag>().map_err(|e| format!("{}", e))?;
    module.add_class::<PyWaehrung>().map_err(|e| format!("{}", e))?;

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
        saetze.to_object(py), 
        rechteinhaber.unwrap_or_default().to_object(py), 
        rangvermerk.unwrap_or_default().to_object(py), 
        regex_list.to_object(py),
        recht_id.to_string().to_object(py),
    ]);
    let result = fun.call1(py, tuple).map_err(|e| format!("{}", e))?;
    let extract = result.as_ref(py).extract::<String>().map_err(|e| format!("{}", e))?;
    
    Ok(extract)
}

pub fn python_exec_kurztext_string<'py>(
    py: Python<'py>,
    recht_id: &str,
    text_sauber: &str,
    saetze_clean: &[String],
    py_code_lines: &[String], 
    konfiguration: &Konfiguration,
) -> Result<String, String> {
    python_exec_kurztext_inner(
        py,
        recht_id,
        text_sauber,
        saetze_clean,
        py_code_lines,
        konfiguration,
        |py: &PyAny| py.extract::<String>().map_err(|e| format!("{}", e))
    )
}

pub fn python_exec_kurztext<'py, T: PyClass + Clone>(
    py: Python<'py>,
    recht_id: &str,
    text_sauber: &str, 
    saetze_clean: &[String],
    py_code_lines: &[String], 
    konfiguration: &Konfiguration,
) -> Result<T, String> {
    python_exec_kurztext_inner(
        py,
        recht_id,
        text_sauber,
        saetze_clean,
        py_code_lines,
        konfiguration,
        |py: &PyAny| py.extract::<T>().map_err(|e| format!("{}", e))
    )
}

fn python_exec_kurztext_inner<'py, T>(
    py: Python<'py>,
    recht_id: &str,
    text_sauber: &str, 
    saetze_clean: &[String],
    py_code_lines: &[String], 
    konfiguration: &Konfiguration,
    extract: fn(&PyAny) -> Result<T, String>,
) -> Result<T, String> {
    
    use pyo3::prelude::*;
    use pyo3::types::{PyDict, PyList, PyTuple};

    use crate::kurztext::PyWaehrung;
        
    let script = py_code_lines
        .iter()
        .map(|l| format!("    {}", l))
        .collect::<Vec<_>>()
        .join("\r\n");
        
    let script = script.replace("\t", "    ");
    let script = script.replace("\u{00a0}", " ");
    let py_code = format!("import inspect\r\n\r\ndef run_script(*args, **kwargs):\r\n    saetze, re, recht_id = args\r\n{}", script);
    let regex_values = konfiguration.regex.values().cloned().collect::<Vec<_>>();
    
    let saetze = PyList::new(py, saetze_clean.into_iter());

    let module = PyModule::from_code(py, &py_code, "", "main").map_err(|e| format!("{}", e))?;
    module.add_class::<RechteArtPyWrapper>().map_err(|e| format!("{}", e))?;
    module.add_class::<SchuldenArtPyWrapper>().map_err(|e| format!("{}", e))?;
    module.add_class::<CompiledRegex>().map_err(|e| format!("{}", e))?;
    module.add_class::<PyBetrag>().map_err(|e| format!("{}", e))?;
    module.add_class::<PyWaehrung>().map_err(|e| format!("{}", e))?;

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
    let tuple = PyTuple::new(py, &[saetze.to_object(py), regex_list.to_object(py), recht_id.to_string().to_object(py)]);
    let result = fun.call1(py, tuple).map_err(|e| format!("{}", e))?;
    let extract = (extract)(result.as_ref(py))?;
    Ok(extract)
}

lazy_static::lazy_static! {
    static ref REGEX_CACHE: Mutex<BTreeMap<String, CompiledRegex>> = Mutex::new(BTreeMap::new());
}

pub fn get_or_insert_regex(
    all_regex: &[String], 
    regex: &str
) -> Result<CompiledRegex, String> {
    
    let mut lock = match REGEX_CACHE.try_lock() {
        Ok(o) => o,
        Err(e) => return Err(format!("{}", e)),
    };
    
    let compiled_regex = match lock.get(&regex.to_string()).cloned() {
        Some(s) => s,
        None => {
            let cr = CompiledRegex::new(&regex).map_err(|e| format!("Fehler in Regex: {}", e))?;
            lock.insert(regex.to_string(), cr.clone());
            cr
        }
    };
    
    let regex_to_remove = lock
        .keys()
        .filter(|k| !all_regex.contains(k))
        .cloned()
        .collect::<Vec<_>>();
    
    for r in regex_to_remove {
        let _ = lock.remove(&r);
    }
    
    Ok(compiled_regex)
}

#[derive(Debug, Clone)]
#[repr(C)]
#[pyclass(name = "Regex")]
pub struct CompiledRegex {
    re: regex::Regex,
}

impl CompiledRegex {

    pub fn new(s: &str) -> Result<Self, regex::Error> {
        Ok(Self {
            re: regex::RegexBuilder::new(s)
                .multi_line(true)
                .case_insensitive(false)
                //.size_limit(1000)
                .build()?
        })
    }
    
    pub fn find_all_matches(&self, text: &str) -> Vec<String> {
        self.re.find_iter(text).map(|m| m.as_str().to_string()).collect()
    }
    
    pub fn get_captures(&self, text: &str) -> Vec<String> {
        let cap = match self.re.captures_iter(text).next() {
            Some(c) => c,
            None => return Vec::new(),
        };
        
        cap.iter().skip(1).filter_map(|group| {
            Some(group?.as_str().to_string())
        }).collect()
    }
}

impl ToPyObject for CompiledRegex {
    fn to_object(&self, py: pyo3::Python) -> pyo3::PyObject {
        self.clone().into_py(py)
    }
}

#[allow(non_snake_case)]
#[pymethods]
impl CompiledRegex {
    #[pyo3(text_signature = "(text, /)")]
    pub fn matches(&self, text: &str) -> bool {
        !self.get_captures(text).is_empty()
    }
    #[pyo3(text_signature = "(text, index, /)")]
    pub fn find_in(&self, text: &str, index: usize) -> Option<String> {
        self.get_captures(text).get(index).cloned()
    }
    #[pyo3(text_signature = "(text, /)")]
    pub fn find_all(&self, text: &str) -> Vec<String> {
        self.find_all_matches(text)
    }
    #[pyo3(text_signature = "(text, text_neu, /)")]
    pub fn replace_all(&self, text: &str, text_neu: &str) -> String {
        self.re.replace_all(text, text_neu).to_string()
    }
}

fn teste_regex(regex_id: &str, text: &str, konfig: &Konfiguration) -> Result<Vec<String>, String> {
    
    let regex = match konfig.regex.get(regex_id) {
        Some(regex) => regex.clone(),
        None => return Err(format!("Regex-ID \"{}\" nicht gefunden.", regex_id)),
    };
    
    let compiled_regex = get_or_insert_regex(
        &konfig.regex.values().cloned().collect::<Vec<_>>(), 
        &regex,
    )?;
    
    Ok(compiled_regex.get_captures(text))
}

fn main() {

    use std::env;
    
    let num = num_cpus::get();
    let max_threads = (num as f32 / 2.0).ceil().max(2.0) as usize;
    let max_threads = match num {
        0 | 1 | 2 | 3 => 1,
        4 | 5 => 2,
        6 | 7 | 8 => 3,
        12 => 5,
        16 => 7,
        24 => 11,
        48 => 23,
        64 => 31,
        128 => 62,
        256 => 125,
        _ => 3,
    };
    
    let _ = env::set_var("RAYON_NUM_THREADS", format!("{}", max_threads));

    println!("setting RAYON_NUM_THREADS = {}", max_threads);
    
    let _ = rayon::ThreadPoolBuilder::new()
        .num_threads(max_threads)
        .build_global();

    let original_value = env::var(GTK_OVERLAY_SCROLLING);
    env::set_var(GTK_OVERLAY_SCROLLING, "0"); // disable overlaid scrollbars
    
    let _ = Konfiguration::neu_laden();
    
    let mut userdata = RpcData::default();
    
    let initial_screen = ui::render_entire_screen(&mut userdata);
    
    let app_html = include_str!("dist/app.html").to_string().replace("<!-- REPLACED_ON_STARTUP -->", &initial_screen);
    let url = "data:text/html,".to_string() + &encode(&app_html);
    let resizable = true;
    let debug = true;
    
    let (_, launched_successful) = run(
        APP_TITLE, 
        &url, 
        Some((9999, 9999)), // = maximized
        resizable, 
        debug, 
        |webview| { webview.dispatch(|webview, _| { 
            webview.eval(&format!("replaceEntireScreen(`{}`)", initial_screen)); }); 
        }, 
        |webview, arg, data: &mut RpcData| { webview_cb(webview, arg, data); }, 
        userdata
    );

    if !launched_successful {
        println!("failed to launch {}", env!("CARGO_PKG_NAME"));
        return;
    }
    
    let _ = std::fs::remove_file(std::env::temp_dir().join("dgb").join("passwort.txt"));
    
    if let Ok(original_value) = original_value {
        env::set_var(GTK_OVERLAY_SCROLLING, original_value);
    }
}
