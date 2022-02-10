// Linux: apt install libwebkit2gtk-4.0-dev, tesseract-ocr, pdftotext

use std::collections::BTreeMap;
use std::path::Path;
use std::{fs, thread};
use std::sync::Mutex;

use urlencoding::encode;
use web_view::*;
use serde_derive::{Serialize, Deserialize};
use crate::digitalisiere::{
    SeiteParsed, Nebenbeteiligter, NebenbeteiligterExtra,
    NebenbeteiligterTyp, Titelblatt, SeitenTyp,
    Grundbuch, Fehler,
    Anrede, PdfToTextLayout,
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

#[derive(Debug, Clone)]
pub struct RpcData {
    // UI
    pub active_tab: usize,
    pub configuration_active: bool,
    pub info_active: bool,
    pub context_menu_active: Option<ContextMenuData>,
    pub open_page: Option<(FileName, u32)>,
    
    pub loaded_files: BTreeMap<FileName, PdfFile>,
    pub back_forward: BTreeMap<FileName, BackForwardBuf>,
    pub loaded_nb: Vec<Nebenbeteiligter>,
    pub loaded_nb_paths: Vec<String>,
    
    pub konfiguration: Konfiguration,
}

#[derive(Debug, Clone)]
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
            configuration_active: false,
            info_active: false,
            context_menu_active: None,
            loaded_files: BTreeMap::new(),
            back_forward: BTreeMap::new(),
            loaded_nb: Vec::new(),
            loaded_nb_paths: Vec::new(),
            konfiguration: Konfiguration::neu_laden().unwrap_or(Konfiguration {
                zeilenumbrueche_in_ocr_text: false,
                spalten_ausblenden: false,
                vorschau_ohne_geroetet: false,
                regex: BTreeMap::new(),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfFile {
    datei: String,
    titelblatt: Titelblatt,
    seitenzahlen: Vec<u32>,
    geladen: BTreeMap<u32, SeiteParsed>,
    analysiert: Grundbuch,
    pdftotext_layout: PdfToTextLayout,
    #[serde(default)]
    anpassungen_seite: BTreeMap<usize, AnpassungSeite>,
    #[serde(default)]
    klassifikation_neu: BTreeMap<usize, SeitenTyp>,
    #[serde(default)]
    nebenbeteiligte_dateipfade: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnpassungSeite {
    pub spalten: BTreeMap<String, Rect>,    
    pub zeilen: Vec<f32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Rect {
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
}

impl PdfFile {
    pub fn speichern(&self) {
        let file_name = format!("{}_{}", self.titelblatt.grundbuch_von, self.titelblatt.blatt);
        let default_parent = Path::new("/");
        let output_parent = Path::new(&self.datei).clone().parent().unwrap_or(&default_parent).to_path_buf();
        let target_output_path = output_parent.clone().join(&format!("{}.gbx", file_name));
        let json = match serde_json::to_string_pretty(&self) { Ok(o) => o, Err(_) => return, };
        let _ = std::fs::write(&target_output_path, json.as_bytes());
    }
    
    pub fn ist_geladen(&self) -> bool {
        self.seitenzahlen
        .iter()
        .all(|sz| self.geladen.contains_key(sz))
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

    pub fn get_nebenbeteiligte(&self, konfiguration: &Konfiguration) -> Vec<Nebenbeteiligter> {
        let mut v = Vec::new();
        
        let analysiert = crate::analysiere::analysiere_grundbuch(&self.analysiert, &[], konfiguration);
        
        for abt2 in &analysiert.abt2 {
            if !abt2.rechtsinhaber.is_empty() {
                v.push(Nebenbeteiligter {
                    ordnungsnummer: None,
                    typ: NebenbeteiligterTyp::from_str(&abt2.rechtsinhaber),
                    name: abt2.rechtsinhaber.clone(),
                    extra: NebenbeteiligterExtra::default(),
                })
            }
        }
        
        for abt3 in &analysiert.abt3 {
            if !abt3.rechtsinhaber.is_empty() {
                v.push(Nebenbeteiligter {
                    ordnungsnummer: None,
                    typ: NebenbeteiligterTyp::from_str(&abt3.rechtsinhaber),
                    name: abt3.rechtsinhaber.clone(),
                    extra: NebenbeteiligterExtra::default(),
                })
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
    pub zeilenumbrueche_in_ocr_text: bool,
    #[serde(default)]
    pub vorschau_ohne_geroetet: bool,
    #[serde(default)]
    pub regex: BTreeMap<String, String>,
    #[serde(default)]
    pub abkuerzungen_script: Vec<String>,
    #[serde(default)]
    pub text_saubern_script: Vec<String>,
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

impl Konfiguration {

    const DEFAULT: &'static str = include_str!("../Konfiguration.json");
    const FILE_NAME: &'static str = "Konfiguration.json";
    
    pub fn konfiguration_pfad() -> String {
        dirs::config_dir()
        .and_then(|p| Some(p.join(Self::FILE_NAME).to_str()?.to_string()))
        .or(
            std::env::current_exe().ok()
            .and_then(|p| Some(p.parent()?.to_path_buf().join(Self::FILE_NAME).to_str()?.to_string()))
        )
        .unwrap_or(format!("./{}", Self::FILE_NAME))
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
    #[serde(rename = "open_configuration")]
    OpenConfiguration,
    #[serde(rename = "open_info")]
    OpenInfo,
    #[serde(rename = "close_file")]
    CloseFile { file_name: String },
    #[serde(rename = "klassifiziere_seite_neu")]
    KlassifiziereSeiteNeu { seite: usize, klassifikation_neu: String },

    #[serde(rename = "edit_abkuerzungen_script")]
    EditAbkuerzungenScript { script: String },
    #[serde(rename = "edit_text_saubern_script")]
    EditTextSaubernScript { script: String },
    
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
        number: f32,
    },
    #[serde(rename = "toggle_checkbox")]
    ToggleCheckBox { checkbox_id: String },
    #[serde(rename = "reload_grundbuch")]
    ReloadGrundbuch,
    #[serde(rename = "zeile_neu")]
    ZeileNeu { file: String, page: usize, y: f32 },
    #[serde(rename = "zeile_loeschen")]
    ZeileLoeschen { file: String, page: usize, zeilen_id: usize },
    
    // UI stuff
    #[serde(rename = "set_active_ribbon_tab")]
    SetActiveRibbonTab { new_tab: usize },
    #[serde(rename = "set_open_file")]
    SetOpenFile { new_file: String },
    #[serde(rename = "set_open_page")]
    SetOpenPage { active_page: u32 },
}

#[derive(Debug, Serialize, Deserialize)]
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
                Some((&["*.pdf"], "Grundbuchblatt")),
            ); 
            
            let dateien = match file_dialog_result {
                Some(f) => f,
                None => return,
            };
            
            // Nur PDF-Dateien laden
            let dateien = dateien
            .iter()
            .filter_map(|dateipfad| {
                let dateiendung = Path::new(dateipfad).extension()?;
                if dateiendung == "pdf" {
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
                
                let mut pdf_parsed = PdfFile {
		            datei: d.to_string(),
		            titelblatt,
		            seitenzahlen: seitenzahlen.clone(),
                    klassifikation_neu: BTreeMap::new(),
                    pdftotext_layout: PdfToTextLayout::default(),
		            geladen: BTreeMap::new(),
		            analysiert: Grundbuch::default(),
		            nebenbeteiligte_dateipfade: Vec::new(),
		            anpassungen_seite: BTreeMap::new(),
		        };
		                        
                if let Some(cached_pdf) = std::fs::read_to_string(&cache_output_path).ok().and_then(|s| serde_json::from_str(&s).ok()) {
                    pdf_parsed = cached_pdf;
                }
                if let Some(target_pdf) = std::fs::read_to_string(&target_output_path).ok().and_then(|s| serde_json::from_str(&s).ok()) {
                    pdf_parsed = target_pdf;
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
                
                crate::analysiere::roete_bestandsverzeichnis_automatisch(&mut pdf_parsed.analysiert.bestandsverzeichnis);
                
                let json = match serde_json::to_string_pretty(&pdf_parsed) { Ok(o) => o, Err(_) => continue, };
                let _ = std::fs::write(&cache_output_path, json.as_bytes());
                data.loaded_files.insert(file_name.clone(), pdf_parsed.clone());
                pdf_zu_laden.push(pdf_parsed);  
                if data.open_page.is_none() {
                    data.open_page = Some((file_name.clone(), 2));
                }
            }
            
            webview.eval(&format!("replaceEntireScreen(`{}`)", ui::render_entire_screen(data)));
                        
            for pdf_parsed in &pdf_zu_laden {
                let default_parent = Path::new("/");
                let output_parent = Path::new(&pdf_parsed.datei).parent().unwrap_or(&default_parent).to_path_buf();
                let file_name = format!("{}_{}", pdf_parsed.titelblatt.grundbuch_von, pdf_parsed.titelblatt.blatt);
                let cache_output_path = output_parent.clone().join(&format!("{}.cache.gbx", file_name));
                webview.eval(&format!("startCheckingForPageLoaded(`{}`, `{}`)", cache_output_path.display(), file_name));
            }
            
            digitalisiere_dateien(pdf_zu_laden);
        },
        Cmd::CheckForImageLoaded { file_path, file_name } => {
            // TODO
            webview.eval(&format!("stopCheckingForImageLoaded(`{}`)", file_name));
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
            
            crate::analysiere::roete_bestandsverzeichnis_automatisch(&mut pdf_parsed.analysiert.bestandsverzeichnis);

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
                        BvEintrag::new(row + 1)
                    );
                    bv_eintrag.lfd_nr = new_value.clone();
                },
                ("bv", "bisherige-lfd-nr") => {
                    let new_value = match new_value.parse::<usize>().ok() {
                        Some(s) => Some(s),
                        None => None,
                    };
                    let mut bv_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.bestandsverzeichnis.eintraege, 
                        row, 
                        BvEintrag::new(row + 1)
                    );
                    bv_eintrag.bisherige_lfd_nr = new_value.clone();
                },
                ("bv", "gemarkung") => {
                    let mut bv_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.bestandsverzeichnis.eintraege, 
                        row, 
                        BvEintrag::new(row + 1)
                    );
                    bv_eintrag.gemarkung = if new_value.trim().is_empty() { 
                        None 
                    } else { 
                        Some(new_value.clone()) 
                    };
                },
                ("bv", "flur") => {
                    let new_value = match new_value.parse::<usize>().ok() {
                        Some(s) => s,
                        None => return,
                    };
                    let mut bv_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.bestandsverzeichnis.eintraege, 
                        row, 
                        BvEintrag::new(row + 1)
                    );
                    bv_eintrag.flur = new_value.clone();
                },
                ("bv", "flurstueck") => {
                    let mut bv_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.bestandsverzeichnis.eintraege, 
                        row, 
                        BvEintrag::new(row + 1)
                    );
                    bv_eintrag.flurstueck = new_value.clone();
                },
                ("bv", "groesse") => {
                    let new_value = match new_value.parse::<usize>().ok() {
                        Some(s) => Some(s),
                        None => None,
                    };
                    let mut bv_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.bestandsverzeichnis.eintraege, 
                        row, 
                        BvEintrag::new(row + 1)
                    );
                    bv_eintrag.groesse = FlurstueckGroesse::Metrisch { m2: new_value };
                },
                ("bv-zuschreibung", "bv-nr") => {
                    let mut bv_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.bestandsverzeichnis.zuschreibungen, 
                        row, 
                        BvZuschreibung::default()
                    );
                    
                    bv_eintrag.bv_nr = new_value.clone();
                },
                ("bv-zuschreibung", "text") => {
                    let mut bv_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.bestandsverzeichnis.zuschreibungen, 
                        row, 
                        BvZuschreibung::default()
                    );
                    bv_eintrag.text = new_value.clone();
                },
                ("bv-abschreibung", "bv-nr") => {
                    let mut bv_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.bestandsverzeichnis.abschreibungen, 
                        row, 
                        BvAbschreibung::default()
                    );
                    bv_eintrag.bv_nr = new_value.clone();
                },
                ("bv-abschreibung", "text") => {
                    let mut bv_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.bestandsverzeichnis.abschreibungen, 
                        row, 
                        BvAbschreibung::default()
                    );
                    bv_eintrag.text = new_value.clone();
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
                    abt1_eintrag.lfd_nr = new_value.clone();
                },
                ("abt1", "eigentuemer") => {
                    let mut abt1_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt1.eintraege, 
                        row, 
                        Abt1Eintrag::new(row + 1)
                    );
                    abt1_eintrag.eigentuemer = new_value.clone();
                },
                ("abt1", "bv-nr") => {
                    let mut abt1_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt1.eintraege, 
                        row, 
                        Abt1Eintrag::new(row + 1)
                    );
                    abt1_eintrag.bv_nr = new_value.clone();
                },
                ("abt1", "grundlage-der-eintragung") => {
                    let mut abt1_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt1.eintraege, 
                        row, 
                        Abt1Eintrag::new(row + 1)
                    );
                    abt1_eintrag.grundlage_der_eintragung = new_value.clone();
                },
                ("abt1-veraenderung", "lfd-nr") => {
                    let mut abt1_veraenderung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt1.veraenderungen, 
                        row, 
                        Abt1Veraenderung::default()
                    );
                    abt1_veraenderung.lfd_nr = new_value.clone();
                },
                ("abt1-veraenderung", "text") => {
                    let mut abt1_veraenderung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt1.veraenderungen, 
                        row, 
                        Abt1Veraenderung::default()
                    );
                    abt1_veraenderung.text = new_value.clone();
                },
                ("abt1-loeschung", "lfd-nr") => {
                    let mut abt1_loeschung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt1.loeschungen, 
                        row, 
                        Abt1Loeschung::default()
                    );
                    abt1_loeschung.lfd_nr = new_value.clone();
                },
                ("abt1-loeschung", "text") => {
                    let mut abt1_loeschung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt1.loeschungen, 
                        row, 
                        Abt1Loeschung::default()
                    );
                    abt1_loeschung.text = new_value.clone();
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
                    abt2_eintrag.lfd_nr = new_value.clone();
                },
                ("abt2", "bv-nr") => {
                    let mut abt2_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt2.eintraege, 
                        row, 
                        Abt2Eintrag::new(row + 1)
                    );
                    abt2_eintrag.bv_nr = new_value.clone();
                },
                ("abt2", "text") => {
                    let mut abt2_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt2.eintraege, 
                        row, 
                        Abt2Eintrag::new(row + 1)
                    );
                    abt2_eintrag.text = new_value.clone();
                },
                ("abt2-veraenderung", "lfd-nr") => {
                    let mut abt2_veraenderung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt2.veraenderungen, 
                        row, 
                        Abt2Veraenderung::default()
                    );
                    abt2_veraenderung.lfd_nr = new_value.clone();
                },
                ("abt2-veraenderung", "text") => {
                    let mut abt2_veraenderung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt2.veraenderungen, 
                        row, 
                        Abt2Veraenderung::default()
                    );
                    abt2_veraenderung.text = new_value.clone();
                },
                ("abt2-loeschung", "lfd-nr") => {
                    let mut abt2_loeschung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt2.loeschungen, 
                        row, 
                        Abt2Loeschung::default()
                    );
                    abt2_loeschung.lfd_nr = new_value.clone();
                },
                ("abt2-loeschung", "text") => {
                    let mut abt2_loeschung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt2.loeschungen, 
                        row, 
                        Abt2Loeschung::default()
                    );
                    abt2_loeschung.text = new_value.clone();
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
                    abt3_eintrag.bv_nr = new_value.clone();
                },
                ("abt3", "betrag") => {
                    let mut abt3_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt3.eintraege, 
                        row, 
                        Abt3Eintrag::new(row + 1)
                    );
                    abt3_eintrag.betrag = new_value.clone();
                },
                ("abt3", "text") => {
                    let mut abt3_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt3.eintraege, 
                        row, 
                        Abt3Eintrag::new(row + 1)
                    );
                    abt3_eintrag.text = new_value.clone();
                },
                ("abt3-veraenderung", "lfd-nr") => {
                    let mut abt3_veraenderung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt3.veraenderungen, 
                        row, 
                        Abt3Veraenderung::default()
                    );
                    abt3_veraenderung.lfd_nr = new_value.clone();
                },
                ("abt3-veraenderung", "text") => {
                    let mut abt3_veraenderung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt3.veraenderungen, 
                        row, 
                        Abt3Veraenderung::default()
                    );
                    abt3_veraenderung.text = new_value.clone();
                },
                ("abt3-loeschung", "lfd-nr") => {
                    let mut abt3_loeschung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt3.loeschungen, 
                        row, 
                        Abt3Loeschung::default()
                    );
                    abt3_loeschung.lfd_nr = new_value.clone();
                },
                ("abt3-loeschung", "text") => {
                    let mut abt3_loeschung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt3.loeschungen, 
                        row, 
                        Abt3Loeschung::default()
                    );
                    abt3_loeschung.text = new_value.clone();
                },
                
                _ => { return; }
            }
                        
            crate::analysiere::roete_bestandsverzeichnis_automatisch(&mut open_file.analysiert.bestandsverzeichnis);
            
            let default_parent = Path::new("/");
            let output_parent = Path::new(&open_file.datei).parent().unwrap_or(&default_parent).to_path_buf();
            let file_name = format!("{}_{}", open_file.titelblatt.grundbuch_von, open_file.titelblatt.blatt);
            let target_output_path = output_parent.clone().join(&format!("{}.gbx", file_name));
            if let Ok(json) = serde_json::to_string_pretty(&open_file) {
                let _ = std::fs::write(&target_output_path, json.as_bytes());
            }
            webview.eval(&format!("replaceAnalyseGrundbuch(`{}`);", ui::render_analyse_grundbuch(&open_file, &data.loaded_nb, &data.konfiguration)));
        },
        Cmd::EintragNeu { path } => {
            
            use crate::digitalisiere::{BvEintrag, Abt2Eintrag, Abt3Eintrag};

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
                "bv" => insert_after(&mut open_file.analysiert.bestandsverzeichnis.eintraege, row, BvEintrag::new(row + 2)),
                "bv-zuschreibung" => insert_after(&mut open_file.analysiert.bestandsverzeichnis.zuschreibungen, row, BvZuschreibung::default()),
                "bv-abschreibung" => insert_after(&mut open_file.analysiert.bestandsverzeichnis.abschreibungen, row, BvAbschreibung::default()),
                
                "abt1" => insert_after(&mut open_file.analysiert.abt1.eintraege, row, Abt1Eintrag::new(row + 2)),
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
            
            crate::analysiere::roete_bestandsverzeichnis_automatisch(&mut open_file.analysiert.bestandsverzeichnis);

            let next_focus = match *section {
                "bv" => format!("bv_{}_lfd-nr", row + 1),
                "bv-zuschreibung" => format!("bv-zuschreibung_{}_bv-nr", row + 1),
                "bv-abschreibung" => format!("bv-abschreibung_{}_bv-nr", row + 1),
                
                "abt1" => format!("abt1_{}_lfd-nr", row + 1),
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
            
            let default_parent = Path::new("/");
            let output_parent = Path::new(&open_file.datei).parent().unwrap_or(&default_parent).to_path_buf();
            let file_name = format!("{}_{}", open_file.titelblatt.grundbuch_von, open_file.titelblatt.blatt);
            let target_output_path = output_parent.clone().join(&format!("{}.gbx", file_name));
            if let Ok(json) = serde_json::to_string_pretty(&open_file) {
                let _ = std::fs::write(&target_output_path, json.as_bytes());
            }
            
            let analyse_neu = ui::render_analyse_grundbuch(&open_file, &data.loaded_nb, &data.konfiguration);
            webview.eval(&format!("replaceMainContainer(`{}`);", ui::render_main_container(data)));
            webview.eval(&format!("replaceAnalyseGrundbuch(`{}`);", analyse_neu));            
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
                        let cur = *e.manuell_geroetet.get_or_insert_with(|| e.automatisch_geroetet);
                        e.manuell_geroetet = Some(!cur);
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
                    if !open_file.analysiert.abt1.veraenderungen.is_empty() {
                        open_file.analysiert.abt1.veraenderungen.remove(row);
                    }
                },
                ("abt1", true) => { 
                    open_file.analysiert.abt1.eintraege
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
            
            crate::analysiere::roete_bestandsverzeichnis_automatisch(&mut open_file.analysiert.bestandsverzeichnis);

            let default_parent = Path::new("/");
            let output_parent = Path::new(&open_file.datei).parent().unwrap_or(&default_parent).to_path_buf();
            let file_name = format!("{}_{}", open_file.titelblatt.grundbuch_von, open_file.titelblatt.blatt);
            let target_output_path = output_parent.clone().join(&format!("{}.gbx", file_name));
            if let Ok(json) = serde_json::to_string_pretty(&open_file) {
                let _ = std::fs::write(&target_output_path, json.as_bytes());
            }
            
            let analyse_neu = ui::render_analyse_grundbuch(&open_file, &data.loaded_nb, &data.konfiguration);

            webview.eval(&format!("replaceMainContainer(`{}`);", ui::render_main_container(data)));
            webview.eval(&format!("replaceAnalyseGrundbuch(`{}`);", analyse_neu));            
            webview.eval(&format!("(function() {{ 
                let element = document.getElementById(`{}`); 
                if (element) {{ element.focus(); }};
            }})();", next_focus));
        },
        Cmd::OpenContextMenu { x, y, seite } => {
            data.context_menu_active = Some(ContextMenuData {
                x: *x,
                y: *y,
                seite_ausgewaehlt: *seite,
            });
            data.info_active = false;
            data.configuration_active = false;
            webview.eval(&format!("replacePopOver(`{}`)", ui::render_popover_content(data)));
        },
        Cmd::OpenConfiguration => {
            data.context_menu_active = None;
            data.configuration_active = true;
            data.info_active = false;
            webview.eval(&format!("replaceEntireScreen(`{}`)", ui::render_entire_screen(data)));
        },
        Cmd::OpenInfo => {
            data.context_menu_active = None;
            data.configuration_active = false;
            data.info_active = true;
            webview.eval(&format!("replaceEntireScreen(`{}`)", ui::render_entire_screen(data)));
        },
        Cmd::CloseFile { file_name } => {
            let _ = data.loaded_files.remove(file_name);
            data.info_active = false;
            data.configuration_active = false;
            data.context_menu_active = None;
            webview.eval(&format!("stopCheckingForPageLoaded(`{}`)", file_name));
            webview.eval(&format!("replaceEntireScreen(`{}`)", ui::render_entire_screen(data)));
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
        Cmd::RangvermerkAuslesenAbt2ScriptTesten { text } => {
            let start = std::time::Instant::now();
            let result: Result<String, String> = Python::with_gil(|py| {
                let (text_sauber, saetze_clean) = crate::kurztext::text_saubern(&*text, &data.konfiguration)?;
                python_exec_kurztext_string(py, &text_sauber, &saetze_clean, &data.konfiguration.rangvermerk_auslesen_abt2_script, &data.konfiguration)
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
                python_exec_kurztext_string(py, &text_sauber, &saetze_clean, &data.konfiguration.rechtsinhaber_auslesen_abt2_script, &data.konfiguration)
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
                python_exec_kurztext_string(py, &text_sauber, &saetze_clean,  &data.konfiguration.rechtsinhaber_auslesen_abt3_script, &data.konfiguration)
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
                python_exec_kurztext(py, &text_sauber, &saetze_clean, &data.konfiguration.betrag_auslesen_script, &data.konfiguration)
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
                    &text_sauber, 
                    &saetze_clean, 
                    &data.konfiguration.klassifiziere_schuldenart, 
                    &data.konfiguration
                );
                let rechteart = rechteart?.inner;
                
                python_exec_kurztext_string(py, &text_sauber, &saetze_clean, &data.konfiguration.text_kuerzen_abt2_script, &data.konfiguration)
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
                    &text_sauber, 
                    &saetze_clean, 
                    &data.konfiguration.klassifiziere_schuldenart, 
                    &data.konfiguration
                );
                let schuldenart = schuldenart?.inner;
                
                let betrag: Result<PyBetrag, String> = crate::python_exec_kurztext(
                    py,
                    &format!("100.000,00 EUR"), 
                    &[format!("100.000,00 EUR")], 
                    &data.konfiguration.betrag_auslesen_script, 
                    &data.konfiguration
                );
                let betrag = betrag?.inner;
                
                let rechtsinhaber: Result<String, String> = crate::python_exec_kurztext_string(
                    py,
                    &text_sauber, 
                    &saetze_clean, 
                    &data.konfiguration.betrag_auslesen_script, 
                    &data.konfiguration
                );
                let rechtsinhaber = rechtsinhaber?;

                python_exec_kuerze_text_abt3(
                    py,
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
                python_exec_kurztext(py, &text_sauber, &saetze_clean, &data.konfiguration.klassifiziere_rechteart, &data.konfiguration)
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
                python_exec_kurztext(py, &text_sauber, &saetze_clean, &data.konfiguration.klassifiziere_schuldenart, &data.konfiguration)
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
                "abt3-horz-veraenderungen" => Abt3HorzVeraenderungen,
                "abt3-horz-loeschungen" => Abt3HorzLoeschungen,
                "abt3-horz" => Abt3Horz,
                "abt3-vert-veraenderungen" => Abt3VertVeraenderungen,
                "abt3-vert-loeschungen" => Abt3VertLoeschungen,
                "abt3-vert" => Abt3Vert,
                _ => { return; },
            };
                        
            open_file.klassifikation_neu.insert(*seite, seiten_typ_neu);
            data.context_menu_active = None;            
            webview.eval(&format!("replaceEntireScreen(`{}`)", ui::render_entire_screen(data)));
                
            let open_file = match data.open_page.clone().and_then(|(file, _)| data.loaded_files.get_mut(&file)) { 
                Some(s) => s,
                None => return,
            };
            
            klassifiziere_pdf_seiten_neu(open_file, &[*seite as u32]);
            webview.eval(&format!("replaceEntireScreen(`{}`)", ui::render_entire_screen(data)));
        },
        Cmd::ClosePopOver { } => {
            data.context_menu_active = None;
            data.configuration_active = false;
            data.info_active = false;
            webview.eval(&format!("replacePopOver(`{}`)", String::new()));
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
            
            use std::env::temp_dir;
            use crate::digitalisiere::Fehler;
            use std::fs::File;
            use crate::digitalisiere::formatiere_seitenzahl;
            use std::process::Command;
            use image::ImageOutputFormat;
            use crate::digitalisiere::zeilen_aus_tesseract_hocr;
            
            let file = match data.loaded_files.get_mut(file_name.as_str()) {
                Some(s) => s,
                None => {
                    webview.eval(&format!("resetOcrSelection()"));
                    return;
                }
            };
            
            let temp_ordner = std::env::temp_dir()
            .join(&format!("{gemarkung}/{blatt}", gemarkung = file.titelblatt.grundbuch_von, blatt = file.titelblatt.blatt));
            
            let max_seitenzahl = file.seitenzahlen.iter().copied().max().unwrap_or(0);

            let pdftoppm_output_path = temp_ordner.clone().join(format!("page-clean-{}.png", crate::digitalisiere::formatiere_seitenzahl(*page as u32, max_seitenzahl)));
            
            if !Path::new(&pdftoppm_output_path).exists() {
                if let Ok(o) = std::fs::read(&file.datei) {
                    let _ = crate::digitalisiere::konvertiere_pdf_seiten_zu_png(&o, &[*page as u32], &file.titelblatt);
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
                
            let default_parent = Path::new("/");
            let output_parent = Path::new(&open_file.datei).parent().unwrap_or(&default_parent).to_path_buf();
            let file_name = format!("{}_{}", open_file.titelblatt.grundbuch_von, open_file.titelblatt.blatt);
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
            let default_parent = Path::new("/");
            let output_parent = Path::new(&open_file.datei).parent().unwrap_or(&default_parent).to_path_buf();
            let file_name = format!("{}_{}", open_file.titelblatt.grundbuch_von, open_file.titelblatt.blatt);
            let target_output_path = output_parent.clone().join(&format!("{}.gbx", file_name));
            if let Ok(json) = serde_json::to_string_pretty(&open_file) {
                let _ = std::fs::write(&target_output_path, json.as_bytes());
            }
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
            let default_parent = Path::new("/");
            let output_parent = Path::new(&open_file.datei).parent().unwrap_or(&default_parent).to_path_buf();
            let file_name = format!("{}_{}", open_file.titelblatt.grundbuch_von, open_file.titelblatt.blatt);
            let target_output_path = output_parent.clone().join(&format!("{}.gbx", file_name));
            if let Ok(json) = serde_json::to_string_pretty(&open_file) {
                let _ = std::fs::write(&target_output_path, json.as_bytes());
            }
        },
        Cmd::ResizeColumn {
            direction,
            column_id,
            number,
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
            
            let current_column = match open_page.typ.get_columns(open_file.anpassungen_seite.get(&page)).iter().find(|col| col.id == column_id) {
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
                    "n" => { rect_to_modify.min_y = number / img_ui_height * page_height; },
                    "s" => { rect_to_modify.max_y = number / img_ui_height * page_height; },
                    "e" => { rect_to_modify.min_x = number / img_ui_width * page_width; },
                    "w" => { rect_to_modify.max_x = number / img_ui_width * page_width; },
                    _ => return,
                };
            }
            
            let new_column = match open_page.typ.get_columns(open_file.anpassungen_seite.get(&page)).iter().find(|col| col.id == column_id) {
                Some(s) => s.clone(),
                None => return,
            };
            
            let new_width = (new_column.max_x - new_column.min_x).abs() / page_width * img_ui_width;
            let new_height = (new_column.max_y - new_column.min_y).abs() / page_height * img_ui_height;
            let new_x = new_column.min_x.min(new_column.max_x) / page_width * img_ui_width;
            let new_y = new_column.min_y.min(new_column.max_y) / page_height * img_ui_height;

            // speichern
            let default_parent = Path::new("/");
            let output_parent = Path::new(&open_file.datei).parent().unwrap_or(&default_parent).to_path_buf();
            let file_name = format!("{}_{}", open_file.titelblatt.grundbuch_von, open_file.titelblatt.blatt);
            let target_output_path = output_parent.clone().join(&format!("{}.gbx", file_name));
            if let Ok(json) = serde_json::to_string_pretty(&open_file) {
                let _ = std::fs::write(&target_output_path, json.as_bytes());
            }
        
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
                
                let default_parent = Path::new("/");
                let output_parent = Path::new(&open_file.datei).parent().unwrap_or(&default_parent).to_path_buf();
                let file_name = format!("{}_{}", open_file.titelblatt.grundbuch_von, open_file.titelblatt.blatt);
                let target_output_path = output_parent.clone().join(&format!("{}.gbx", file_name));
                if let Ok(json) = serde_json::to_string_pretty(&open_file) {
                    let _ = std::fs::write(&target_output_path, json.as_bytes());
                }
            }
            
            // Nochmal speichern, nachdem Ordnungsnummern neu vergeben wurden
            let tsv = get_nebenbeteiligte_tsv(&data);
            let _ = fs::write(f_name, tsv.as_bytes());
                    
            let open_file = match data.open_page.clone().and_then(|(file, _)| data.loaded_files.get(&file)) { 
                Some(s) => s,
                None => return,
            };
            
            webview.eval(&format!("replaceAnalyseGrundbuch(`{}`);", ui::render_analyse_grundbuch(&open_file, &data.loaded_nb, &data.konfiguration)));
        },
        Cmd::ExportNebenbeteiligte => {
        
            if data.loaded_files.is_empty() {
                return;
            }
            
            let tsv = get_nebenbeteiligte_tsv(&data);

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
            
            let _ = std::fs::write(&f, tsv.as_bytes());
            
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
            
            match data.open_page.clone().and_then(|(file, _)| data.loaded_files.get_mut(&file)) { 
                Some(open_file) => {
                    crate::analysiere::roete_bestandsverzeichnis_automatisch(&mut open_file.analysiert.bestandsverzeichnis);
                },
                None => { },
            };
                        
            webview.eval(&format!("replaceEntireScreen(`{}`)", ui::render_entire_screen(data)));
        },
        Cmd::SetOpenPage { active_page } => {
            
            if let Some(p) = data.open_page.as_mut() { 
                p.1 = *active_page;
            }
            
            match data.open_page.clone().and_then(|(file, _)| data.loaded_files.get_mut(&file)) { 
                Some(open_file) => {
                    crate::analysiere::roete_bestandsverzeichnis_automatisch(&mut open_file.analysiert.bestandsverzeichnis);
                },
                None => { },
            };
                        
            webview.eval(&format!("replaceEntireScreen(`{}`);", ui::render_entire_screen(data)));
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
                2 => {
                    b.name = s.trim().to_string();
                },
                3 => {
                    if let Some(anrede) = Anrede::from_str(s.trim()) {
                        b.extra.anrede = Some(anrede);
                    }
                },
                4 => {
                    if !s.trim().is_empty() {
                        b.extra.titel = Some(s.trim().to_string());                            
                    }
                },
                5 => {
                    if !s.trim().is_empty() {
                        b.extra.vorname = Some(s.trim().to_string());                            
                    }
                },
                6 => {
                    if !s.trim().is_empty() {
                        b.extra.nachname_oder_firma = Some(s.trim().to_string());                            
                    }
                },
                7 => {
                    if !s.trim().is_empty() {
                        b.extra.geburtsname = Some(s.trim().to_string());                            
                    }
                },
                8 => {
                    if let Some(datum) = NebenbeteiligterExtra::geburtsdatum_from_str(s.trim()) {
                        b.extra.geburtsdatum = Some(datum);                            
                    }
                },
                9 => {
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

    nb.sort_by(|a, b| a.name.cmp(&b.name));
    nb.dedup();
    
    let tsv = nb.iter()
        .map(|nb| format!("{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", 
            nb.ordnungsnummer.map(|s| s.to_string()).unwrap_or_default(), 
            nb.typ.map(|s| s.get_str()).unwrap_or_default(), 
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
    let tsv = format!("ORDNUNGSNUMMER\tTYP\tNAME (GRUNDBUCH)\tANREDE\tTITEL\tVORNAME\tNACHNAME_ODER_FIRMA\tGEBURTSNAME\tGEBURTSDATUM\tWOHNORT\r\n{}", tsv);
    tsv
}

fn klassifiziere_pdf_seiten_neu(pdf: &mut PdfFile, seiten_neu: &[u32]) {
        
    let max_sz = pdf.seitenzahlen.iter().max().cloned().unwrap_or(0);
    
    let default_parent = Path::new("/");
    let output_parent = Path::new(&pdf.datei).clone().parent().unwrap_or(&default_parent).to_path_buf();
    let file_name = format!("{}_{}", pdf.titelblatt.grundbuch_von, pdf.titelblatt.blatt);
    let cache_output_path = output_parent.clone().join(&format!("{}.cache.gbx", file_name));
    let target_output_path = output_parent.clone().join(&format!("{}.gbx", file_name));
        
    let datei_bytes = match fs::read(&pdf.datei) {
        Ok(s) => s,
        Err(e) => return,
    };
    
    let _ = digitalisiere::konvertiere_pdf_seiten_zu_png(&datei_bytes, seiten_neu.as_ref(), &pdf.titelblatt);

    for sz in seiten_neu {

        if digitalisiere::ocr_seite(&pdf.titelblatt, *sz, max_sz).is_err() { continue; }

        let seitentyp = match pdf.klassifikation_neu.get(&(*sz as usize)) {
            Some(s) => *s,
            None => {
                match digitalisiere::klassifiziere_seitentyp(&pdf.titelblatt, *sz, max_sz) { 
                    Ok(o) => o, 
                    Err(_) => continue, 
                }
            }
        };
                        
        let spalten = match digitalisiere::formularspalten_ausschneiden(
            &pdf.titelblatt, 
            *sz, 
            max_sz, 
            seitentyp, 
            &pdf.pdftotext_layout, 
            pdf.anpassungen_seite.get(&(*sz as usize))
        ) { 
            Ok(o) => o, 
            Err(e) => {
                println!("FEHLER: {:?}", e);
                continue;
            }, 
        };
                        
        if digitalisiere::ocr_spalten(&pdf.titelblatt, *sz, max_sz, &spalten).is_err() { continue; }
        
        let textbloecke = match digitalisiere::textbloecke_aus_spalten(
            &pdf.titelblatt, 
            *sz, 
            max_sz,
            &spalten, 
            &pdf.pdftotext_layout,
            pdf.anpassungen_seite.get(&(*sz as usize))
        ) { 
            Ok(o) => o, 
            Err(e) => {
                println!("FEHLER: {:?}", e);
                continue;
            }, 
        };
  
        pdf.geladen.insert(*sz, SeiteParsed {
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
   
    crate::analysiere::roete_bestandsverzeichnis_automatisch(&mut pdf.analysiert.bestandsverzeichnis);

    let json = match serde_json::to_string_pretty(&pdf) { Ok(o) => o, Err(_) => return, };
    let _ = std::fs::write(&target_output_path, json.as_bytes());
}

fn digitalisiere_dateien(pdfs: Vec<PdfFile>) {
    std::thread::spawn(move || {
        for pdf in pdfs {
        
            rayon::spawn(move || {
            
                let mut pdf = pdf;
                
                let datei_bytes = match fs::read(&pdf.datei).ok() {
                    Some(s) => s,
                    None => return,
                };
                
                let max_sz = pdf.seitenzahlen.iter().max().cloned().unwrap_or(0);
                
                let default_parent = Path::new("/");
                let output_parent = Path::new(&pdf.datei).clone().parent().unwrap_or(&default_parent).to_path_buf();
                let file_name = format!("{}_{}", pdf.titelblatt.grundbuch_von, pdf.titelblatt.blatt);
                let cache_output_path = output_parent.clone().join(&format!("{}.cache.gbx", file_name));
                let target_output_path = output_parent.clone().join(&format!("{}.gbx", file_name));

                if let Some(cached_pdf) = std::fs::read_to_string(&cache_output_path).ok().and_then(|s| serde_json::from_str(&s).ok()) {
                    pdf = cached_pdf;
                }
                if let Some(target_pdf) = std::fs::read_to_string(&target_output_path).ok().and_then(|s| serde_json::from_str(&s).ok()) {
                    pdf = target_pdf;
                }
                
                let seitenzahlen_zu_laden = pdf.seitenzahlen
                    .iter()
                    .filter(|sz| !pdf.geladen.contains_key(sz))
                    .copied()
                    .collect::<Vec<_>>();

                crate::analysiere::roete_bestandsverzeichnis_automatisch(&mut pdf.analysiert.bestandsverzeichnis);
                            
                let json = match serde_json::to_string_pretty(&pdf) { Ok(o) => o, Err(_) => return, };
                let _ = std::fs::write(&cache_output_path, json.as_bytes());
                
                let _ = digitalisiere::konvertiere_pdf_seiten_zu_png(&datei_bytes, &seitenzahlen_zu_laden, &pdf.titelblatt);
                                
                for sz in seitenzahlen_zu_laden {
                
                    let pdftotext_layout = match digitalisiere::get_pdftotext_layout(&pdf.titelblatt, &[sz]) { Ok(o) => o, Err(_) => continue, };
                    for (k, v) in pdftotext_layout.seiten.iter() {
                        pdf.pdftotext_layout.seiten.insert(k.clone(), v.clone());
                    }
                    if digitalisiere::ocr_seite(&pdf.titelblatt, sz, max_sz).is_err() { continue; }
                    let seitentyp = match pdf.klassifikation_neu.get(&(sz as usize)) {
                        Some(s) => *s,
                        None => {
                            match digitalisiere::klassifiziere_seitentyp(&pdf.titelblatt, sz, max_sz) { 
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
                
                crate::analysiere::roete_bestandsverzeichnis_automatisch(&mut pdf.analysiert.bestandsverzeichnis);

                let json = match serde_json::to_string_pretty(&pdf) { Ok(o) => o, Err(_) => return, };
                let _ = std::fs::write(&target_output_path, json.as_bytes());
            });
        }
    });
}

fn analysiere_grundbuch(pdf: &PdfFile) -> Option<Grundbuch> {

    let bestandsverzeichnis = digitalisiere::analysiere_bv(&pdf.geladen, &pdf.anpassungen_seite).ok()?;
    let abt1 = digitalisiere::analysiere_abt1(&pdf.geladen, &pdf.anpassungen_seite, &bestandsverzeichnis).ok()?;
    let abt2 = digitalisiere::analysiere_abt2(&pdf.geladen, &pdf.anpassungen_seite, &bestandsverzeichnis).ok()?;
    let abt3 = digitalisiere::analysiere_abt3(&pdf.geladen, &pdf.anpassungen_seite, &bestandsverzeichnis).ok()?;
    
    let mut gb = Grundbuch {
        titelblatt: pdf.titelblatt.clone(),
        bestandsverzeichnis,
        abt1,
        abt2,
        abt3,
    };
    
    crate::analysiere::roete_bestandsverzeichnis_automatisch(&mut gb.bestandsverzeichnis);
    
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
    
    use rayon::prelude::*;
    
    let datei_bytes = match fs::read(&pdf.datei) {
        Ok(s) => s,
        Err(e) => return Err(Fehler::Io(pdf.datei.clone(), e)),
    };
        
    let seitenzahlen_zu_laden = pdf.seitenzahlen.clone();
    let max_sz = pdf.seitenzahlen.iter().max().cloned().unwrap_or(0);
    let _ = digitalisiere::konvertiere_pdf_seiten_zu_png(&datei_bytes, seitenzahlen_zu_laden.as_ref(), &pdf.titelblatt)?;
    
    let ist_geladen = pdf.ist_geladen();
    pdf.geladen.clear();
    pdf.analysiert = Grundbuch::default();
    
    let default_parent = Path::new("/");
    let output_parent = Path::new(&pdf.datei).clone().parent().unwrap_or(&default_parent).to_path_buf();
    let file_name = format!("{}_{}", pdf.titelblatt.grundbuch_von, pdf.titelblatt.blatt);
    let cache_output_path = output_parent.clone().join(&format!("{}.cache.gbx", file_name));
    let target_output_path = output_parent.clone().join(&format!("{}.gbx", file_name));
            
    for sz in seitenzahlen_zu_laden {
            
        if digitalisiere::ocr_seite(&pdf.titelblatt, sz, max_sz).is_err() { 
            continue; 
        }
                            
        let seitentyp = match pdf.klassifikation_neu.get(&(sz as usize)) {
            Some(s) => *s,
            None => {
                match digitalisiere::klassifiziere_seitentyp(&pdf.titelblatt, sz, max_sz) { 
                    Ok(o) => o, 
                    Err(e) => {
                        println!("Fehler beim klassifizieren: {:?}", e);
                        continue;
                    }, 
                }
            }
        };
                
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
    let py_code = format!("import inspect\r\n\r\ndef run_script(*args, **kwargs):\r\n    saetze, betrag, schuldenart, rechtsinhaber, re = args\r\n{}", script);
    let regex_values = konfiguration.regex.values().cloned().collect::<Vec<_>>();
    
    let saetze = PyList::new(py, saetze_clean.into_iter());

    let mut module = PyModule::from_code(py, &py_code, "", "main").map_err(|e| format!("{}", e))?;
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
        regex_list.to_object(py)
    ]);
    let result = fun.call1(py, tuple).map_err(|e| format!("{}", e))?;
    let extract = result.as_ref(py).extract::<String>().map_err(|e| format!("{}", e))?;
    
    Ok(extract)
}


pub fn python_exec_kuerze_text_abt2<'py>(
    py: Python<'py>,
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
    let py_code = format!("import inspect\r\n\r\ndef run_script(*args, **kwargs):\r\n    saetze, rechtsinhaber, rangvermerk, re = args\r\n{}", script);
    let regex_values = konfiguration.regex.values().cloned().collect::<Vec<_>>();
    
    let saetze = PyList::new(py, saetze_clean.into_iter());

    let mut module = PyModule::from_code(py, &py_code, "", "main").map_err(|e| format!("{}", e))?;
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
        regex_list.to_object(py)
    ]);
    let result = fun.call1(py, tuple).map_err(|e| format!("{}", e))?;
    let extract = result.as_ref(py).extract::<String>().map_err(|e| format!("{}", e))?;
    
    Ok(extract)
}

pub fn python_exec_kurztext_string<'py>(
     py: Python<'py>,
    text_sauber: &str, 
    saetze_clean: &[String],
    py_code_lines: &[String], 
    konfiguration: &Konfiguration,
) -> Result<String, String> {
    python_exec_kurztext_inner(
        py,
        text_sauber,
        saetze_clean,
        py_code_lines,
        konfiguration,
        |py: &PyAny| py.extract::<String>().map_err(|e| format!("{}", e))
    )
}

pub fn python_exec_kurztext<'py, T: PyClass + Clone>(
    py: Python<'py>,
    text_sauber: &str, 
    saetze_clean: &[String],
    py_code_lines: &[String], 
    konfiguration: &Konfiguration,
) -> Result<T, String> {
    python_exec_kurztext_inner(
        py,
        text_sauber,
        saetze_clean,
        py_code_lines,
        konfiguration,
        |py: &PyAny| py.extract::<T>().map_err(|e| format!("{}", e))
    )
}

fn python_exec_kurztext_inner<'py, T>(
    py: Python<'py>,
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
    let py_code = format!("import inspect\r\n\r\ndef run_script(*args, **kwargs):\r\n    saetze, re = args\r\n{}", script);
    let regex_values = konfiguration.regex.values().cloned().collect::<Vec<_>>();
    
    let saetze = PyList::new(py, saetze_clean.into_iter());

    let mut module = PyModule::from_code(py, &py_code, "", "main").map_err(|e| format!("{}", e))?;
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
    let tuple = PyTuple::new(py, &[saetze.to_object(py), regex_list.to_object(py)]);
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
    let max_threads = if num > 1 {
         max_threads.min(num.saturating_sub(1)).saturating_sub(1)
    } else {
        1
    };
    
    let _ = env::set_var("RAYON_NUM_THREADS", format!("{}", max_threads));

    println!("setting RAYON_NUM_THREADS = {}", max_threads);
    
    let _ = rayon::ThreadPoolBuilder::new()
        .num_threads(max_threads)
        .build_global();

    let original_value = env::var(GTK_OVERLAY_SCROLLING);
    env::set_var(GTK_OVERLAY_SCROLLING, "0"); // disable overlaid scrollbars
    
    let _ = Konfiguration::neu_laden();
    
    let app_html = include_str!("dist/app.html");
    let url = "data:text/html,".to_string() + &encode(&app_html);
    let size = (1300, 900);
    let resizable = true;
    let debug = true;

    let init_cb = |_webview| { };

    let userdata = RpcData::default();

    let (_, launched_successful) = run(APP_TITLE, &url, Some(size), resizable, debug, init_cb, |webview, arg, data: &mut RpcData| {
        webview_cb(webview, arg, data);
    }, userdata);

    if !launched_successful {
        println!("failed to launch {}", env!("CARGO_PKG_NAME"));
    }

    if let Ok(original_value) = original_value {
        env::set_var(GTK_OVERLAY_SCROLLING, original_value);
    }
}
