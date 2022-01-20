// Linux: apt install libwebkit2gtk-4.0-dev, tesseract-ocr, pdftotext

use std::collections::BTreeMap;
use std::path::Path;
use std::{fs, thread};

use urlencoding::encode;
use web_view::*;
use serde_derive::{Serialize, Deserialize};
use crate::digitalisiere::{
    SeiteParsed, Nebenbeteiligter, NebenbeteiligterExtra,
    NebenbeteiligterTyp, Titelblatt, SeitenTyp,
    Grundbuch, BvZuschreibung, Anrede, PdfToTextLayout,
};
use crate::analysiere::GrundbuchAnalysiert;
use crate::kurztext::RechteArt;

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
            konfiguration: Konfiguration::default(),
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
    klassifikation_neu: BTreeMap<usize, SeitenTyp>,
    #[serde(default)]
    nebenbeteiligte_dateipfade: Vec<String>,
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
        self.seitenzahlen.iter().all(|sz| self.geladen.contains_key(sz))
    }
    
    pub fn get_nebenbeteiligte(&self) -> Vec<Nebenbeteiligter> {
        let mut v = Vec::new();
        
        let analysiert = crate::analysiere::analysiere_grundbuch(&self.analysiert, &[]);
        
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
    pub kurztext_script: String,
}

impl Konfiguration {

    fn konfiguration_pfad() -> String {
        std::env::current_exe().ok()
        .and_then(|p| Some({
            p.parent()?.to_path_buf().join("Konfiguration.toml").to_str()?.to_string()
        }))
        .unwrap_or(format!("./Konfiguration.toml"))
    }
    
    pub fn speichern(&self) {
        let _ = toml::to_string(self).ok().and_then(|s| {
            let s = s.replace("\n", "\r\n");
            std::fs::write(&Self::konfiguration_pfad(), &s.as_bytes()).ok()
        });

    }

    pub fn neu_laden() -> Result<Self, String> {

        if !Path::new(&Self::konfiguration_pfad()).exists() {
            Konfiguration::default().speichern();
        }

        let konfig = match std::fs::read_to_string(&Self::konfiguration_pfad()) {
            Ok(o) => match toml::from_str(&o) {
                Ok(o) => o,
                Err(e) => return Err(format!("Fehler in Konfiguration {}: {}", Self::konfiguration_pfad(), e)),
            },
            Err(e) => return Err(format!("Fehler beim Lesen von Konfiguration in {}: {}", Self::konfiguration_pfad(), e)),
        };

        Ok(konfig)
    }
}

impl Default for Konfiguration {
    fn default() -> Self {
        Konfiguration {
            kurztext_script: format!("return RechteArt.GasleitungGasreglerstationFerngasltg"),
        }
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
    #[serde(rename = "edit_rechteart_script")]
    EditRechteArtScript { neu: String },
    #[serde(rename = "kurztext_testen")]
    KurztextTesten { text: String },
    #[serde(rename = "klassifiziere_seite_neu")]
    KlassifiziereSeiteNeu { seite: usize, klassifikation_neu: String },

    // Check whether a "{file_name}".json with analyzed texts exists
    #[serde(rename = "check_for_pdf_loaded")]
    CheckForPdfLoaded { file_path: String, file_name: String },
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
            webview.eval(&format!("displayError(`{}`);", format!("{:?}", e))); 
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
            
            for d in dateien {
                                
                let datei_bytes = match std::fs::read(d) {
                    Ok(o) => o,
                    Err(e) => {
                        webview.eval(&format!("displayError(`{}`);", format!("Konnte \"{}\" nicht lesen: {}", d, e)));
                        continue;
                    }
                };
                
                webview.eval(&format!("logInfo(`{}`);", format!("Digitalisiere \"{}\"", d)));  
                
                let mut seitenzahlen = match digitalisiere::lese_seitenzahlen(&datei_bytes) {
                    Ok(o) => o,
                    Err(e) => {
                        webview.eval(&format!("displayError(`{}`);", format!("Fehler in Datei \"{}\": {}", d, e)));
                        continue;
                    },
                };
                
                let max_sz = seitenzahlen.iter().max().cloned().unwrap_or(0);
                webview.eval(&format!("logInfo(`{}`);", format!("{} Seiten erkannt.", max_sz)));  

                let titelblatt = match digitalisiere::lese_titelblatt(&datei_bytes) {
                    Ok(o) => o,
                    Err(_) => {
                        webview.eval(&format!("displayError(`{}`);", format!("Kann Titelblatt aus Datei \"{}\" nicht lesen - kein Grundbuchblatt?", d)));
                        continue;
                    },
                };
            
                let default_parent = Path::new("/");
                let output_parent = Path::new(&d).parent().unwrap_or(&default_parent).to_path_buf();
                let file_name = format!("{}_{}", titelblatt.grundbuch_von, titelblatt.blatt);
                let cache_output_path = output_parent.clone().join(&format!("{}.cache.gbx", file_name));
                let target_output_path = output_parent.clone().join(&format!("{}.gbx", file_name));
                webview.eval(&format!("logInfo(`{}`);", format!("Lese Grundbuch von {} Blatt {}, AG {}", titelblatt.grundbuch_von, titelblatt.blatt, titelblatt.amtsgericht)));  
                
                // Lösche Titelblattseite von Seiten, die gerendert werden müssen
                seitenzahlen.remove(0);
                
                let mut pdf_parsed = PdfFile {
		            datei: d.clone(),
		            titelblatt,
		            seitenzahlen: seitenzahlen.clone(),
                    klassifikation_neu: BTreeMap::new(),
                    pdftotext_layout: PdfToTextLayout::default(),
		            geladen: BTreeMap::new(),
		            analysiert: Grundbuch::default(),
		            nebenbeteiligte_dateipfade: Vec::new(),
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
                webview.eval(&format!("replaceMain(`{}`);", ui::render_main(data)));
            }
            
            for pdf_parsed in &pdf_zu_laden {
                let default_parent = Path::new("/");
                let output_parent = Path::new(&pdf_parsed.datei).parent().unwrap_or(&default_parent).to_path_buf();
                let file_name = format!("{}_{}", pdf_parsed.titelblatt.grundbuch_von, pdf_parsed.titelblatt.blatt);
                let cache_output_path = output_parent.clone().join(&format!("{}.cache.gbx", file_name));
                webview.eval(&format!("startCheckingForPageLoaded(`{}`, `{}`)", cache_output_path.display(), file_name));
            }
            
            digitalisiere_dateien(pdf_zu_laden);
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
                    webview.eval(&format!("replacePageList(`{}`);", ui::render_page_list(&data)));
                    webview.eval(&format!("replaceMainContainer(`{}`);", ui::render_main_container(data)));
                    webview.eval(&format!("replacePageImage(`{}`);", ui::render_pdf_image(&data)));
                }
                webview.eval(&format!("stopCheckingForPageLoaded(`{}`)", file_name));
            }
        },
        Cmd::EditText { path, new_value } => {
            
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
                    let mut bv_eintrag = match open_file.analysiert.bestandsverzeichnis.eintraege.get_mut(row) {
                        Some(s) => s,
                        None => return,
                    };
                    bv_eintrag.lfd_nr = new_value.clone();
                },
                ("bv", "bisherige-lfd-nr") => {
                    let new_value = match new_value.parse::<usize>().ok() {
                        Some(s) => Some(s),
                        None => None,
                    };
                    let mut bv_eintrag = match open_file.analysiert.bestandsverzeichnis.eintraege.get_mut(row) {
                        Some(s) => s,
                        None => return,
                    };
                    bv_eintrag.bisherige_lfd_nr = new_value.clone();
                },
                ("bv", "gemarkung") => {
                    let mut bv_eintrag = match open_file.analysiert.bestandsverzeichnis.eintraege.get_mut(row) {
                        Some(s) => s,
                        None => return,
                    };
                    
                    bv_eintrag.gemarkung = if new_value.trim().is_empty() { 
                        Some(new_value.clone()) 
                    } else { 
                        None 
                    };
                },
                ("bv", "flur") => {
                    let new_value = match new_value.parse::<usize>().ok() {
                        Some(s) => s,
                        None => return,
                    };
                    let mut bv_eintrag = match open_file.analysiert.bestandsverzeichnis.eintraege.get_mut(row) {
                        Some(s) => s,
                        None => return,
                    };
                    bv_eintrag.flur = new_value.clone();
                },
                ("bv", "flurstueck") => {
                    let mut bv_eintrag = match open_file.analysiert.bestandsverzeichnis.eintraege.get_mut(row) {
                        Some(s) => s,
                        None => return,
                    };
                    bv_eintrag.flurstueck = new_value.clone();
                },
                ("bv", "groesse") => {
                    let new_value = match new_value.parse::<usize>().ok() {
                        Some(s) => Some(s),
                        None => None,
                    };
                    let mut bv_eintrag = match open_file.analysiert.bestandsverzeichnis.eintraege.get_mut(row) {
                        Some(s) => s,
                        None => return,
                    };
                    bv_eintrag.groesse = FlurstueckGroesse::Metrisch { m2: new_value };
                },
                ("bv-zuschreibung", "bv-nr") => {
                    let mut bv_eintrag = match open_file.analysiert.bestandsverzeichnis.zuschreibungen.get_mut(row) {
                        Some(s) => s,
                        None => return,
                    };
                    bv_eintrag.bv_nr = new_value.clone();
                },
                ("bv-zuschreibung", "text") => {
                    let mut bv_eintrag = match open_file.analysiert.bestandsverzeichnis.zuschreibungen.get_mut(row) {
                        Some(s) => s,
                        None => return,
                    };
                    bv_eintrag.text = new_value.clone();
                },
                ("bv-abschreibung", "bv-nr") => {
                    let mut bv_eintrag = match open_file.analysiert.bestandsverzeichnis.abschreibungen.get_mut(row) {
                        Some(s) => s,
                        None => return,
                    };
                    bv_eintrag.bv_nr = new_value.clone();
                },
                ("bv-abschreibung", "text") => {
                    let mut bv_eintrag = match open_file.analysiert.bestandsverzeichnis.abschreibungen.get_mut(row) {
                        Some(s) => s,
                        None => return,
                    };
                    bv_eintrag.text = new_value.clone();
                },
                ("abt2", "lfd-nr") => {
                    let new_value = match new_value.parse::<usize>().ok() {
                        Some(s) => s,
                        None => return,
                    };
                    let mut abt2_eintrag = match open_file.analysiert.abt2.eintraege.get_mut(row) {
                        Some(s) => s,
                        None => return,
                    };
                    abt2_eintrag.lfd_nr = new_value.clone();
                },
                ("abt2", "bv-nr") => {
                    let mut abt2_eintrag = match open_file.analysiert.abt2.eintraege.get_mut(row) {
                        Some(s) => s,
                        None => return,
                    };
                    abt2_eintrag.bv_nr = new_value.clone();
                },
                ("abt2", "text") => {
                    let mut abt2_eintrag = match open_file.analysiert.abt2.eintraege.get_mut(row) {
                        Some(s) => s,
                        None => return,
                    };
                    abt2_eintrag.text = new_value.clone();
                },
                
                ("abt3", "lfd-nr") => {
                    let new_value = match new_value.parse::<usize>().ok() {
                        Some(s) => s,
                        None => return,
                    };
                    let mut abt3_eintrag = match open_file.analysiert.abt3.eintraege.get_mut(row) {
                        Some(s) => s,
                        None => return,
                    };
                    abt3_eintrag.lfd_nr = new_value.clone();
                },
                ("abt3", "bv-nr") => {
                    let mut abt3_eintrag = match open_file.analysiert.abt3.eintraege.get_mut(row) {
                        Some(s) => s,
                        None => return,
                    };
                    abt3_eintrag.bv_nr = new_value.clone();
                },
                ("abt3", "betrag") => {
                    let mut abt3_eintrag = match open_file.analysiert.abt3.eintraege.get_mut(row) {
                        Some(s) => s,
                        None => return,
                    };
                    abt3_eintrag.betrag = new_value.clone();
                },
                ("abt3", "text") => {
                    let mut abt3_eintrag = match open_file.analysiert.abt3.eintraege.get_mut(row) {
                        Some(s) => s,
                        None => return,
                    };
                    abt3_eintrag.text = new_value.clone();
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
            webview.eval(&format!("replaceAnalyseGrundbuch(`{}`);", ui::render_analyse_grundbuch(&open_file, &data.loaded_nb)));
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
            
            let cell = match split.get(2) {
                Some(s) => s,
                None => return,
            };
            
            let open_file = match data.open_page.clone().and_then(|(file, _)| data.loaded_files.get_mut(&file)) { 
                Some(s) => s,
                None => return,
            };
            
            fn insert_after<T>(vec: &mut Vec<T>, index: usize, new: T) {
                if index + 1 >= vec.len() || vec.is_empty() { 
                    vec.push(new); 
                } else {
                    vec.splice((index + 1)..(index + 1), [new]);
                }
            }
            
            use crate::digitalisiere::{BvEintrag, Abt2Eintrag, Abt3Eintrag};
            
            match *section {
                "bv" => insert_after(&mut open_file.analysiert.bestandsverzeichnis.eintraege, row, BvEintrag::new(row + 2)),
                "bv-zuschreibung" => insert_after(&mut open_file.analysiert.bestandsverzeichnis.zuschreibungen, row, BvZuschreibung::default()),
                "bv-abschreibung" => insert_after(&mut open_file.analysiert.bestandsverzeichnis.zuschreibungen, row, BvZuschreibung::default()),
                "abt2" => insert_after(&mut open_file.analysiert.abt2.eintraege, row, Abt2Eintrag::new(row + 2)),
                "abt3" => insert_after(&mut open_file.analysiert.abt3.eintraege, row, Abt3Eintrag::new(row + 2)),
                _ => return,
            }
            
            crate::analysiere::roete_bestandsverzeichnis_automatisch(&mut open_file.analysiert.bestandsverzeichnis);

            let next_focus = match *section {
                "bv" => format!("bv_{}_lfd-nr", row + 1),
                "bv-zuschreibung" => format!("bv-zuschreibung_{}_bv-nr", row + 1),
                "bv-abschreibung" => format!("bv-abschreibung_{}_bv-nr", row + 1),
                "abt2" => format!("abt2_{}_lfd-nr", row + 1),
                "abt3" => format!("abt3_{}_lfd-nr", row + 1),
                _ => return,
            };
            
            let default_parent = Path::new("/");
            let output_parent = Path::new(&open_file.datei).parent().unwrap_or(&default_parent).to_path_buf();
            let file_name = format!("{}_{}", open_file.titelblatt.grundbuch_von, open_file.titelblatt.blatt);
            let target_output_path = output_parent.clone().join(&format!("{}.gbx", file_name));
            if let Ok(json) = serde_json::to_string_pretty(&open_file) {
                let _ = std::fs::write(&target_output_path, json.as_bytes());
            }
            
            let analyse_neu = ui::render_analyse_grundbuch(&open_file, &data.loaded_nb);
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
            
            let cell = match split.get(2) {
                Some(s) => s,
                None => return,
            };
            
            let open_file = match data.open_page.clone().and_then(|(file, _)| data.loaded_files.get_mut(&file)) { 
                Some(s) => s,
                None => return,
            };
            
            match (*section, eintrag_roeten) {
                ("bv", false) => { open_file.analysiert.bestandsverzeichnis.eintraege.remove(row); },
                ("bv", true) => { 
                    open_file.analysiert.bestandsverzeichnis.eintraege
                    .get_mut(row)
                    .map(|e| {
                        let cur = *e.manuell_geroetet.get_or_insert_with(|| e.automatisch_geroetet);
                        e.manuell_geroetet = Some(!cur);
                    });
                },
                
                ("bv-zuschreibung", false) => { open_file.analysiert.bestandsverzeichnis.zuschreibungen.remove(row); },
                ("bv-abschreibung", false) => { open_file.analysiert.bestandsverzeichnis.abschreibungen.remove(row); },
                
                ("abt2", false) => { open_file.analysiert.abt2.eintraege.remove(row); },
                ("abt2", true) => { 
                    open_file.analysiert.abt2.eintraege
                    .get_mut(row)
                    .map(|e| {
                        let cur = *e.manuell_geroetet.get_or_insert_with(|| e.automatisch_geroetet);
                        e.manuell_geroetet = Some(!cur);
                    });
                },

                ("abt3", false) => { open_file.analysiert.abt3.eintraege.remove(row); },
                ("abt3", true) => { 
                    open_file.analysiert.abt3.eintraege
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
                "abt2" => format!("abt2_{}_lfd-nr", if eintrag_roeten { row + 1 } else { row.saturating_sub(1) }),
                "abt3" => format!("abt3_{}_lfd-nr", if eintrag_roeten { row + 1 } else { row.saturating_sub(1) }),
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
            
            let analyse_neu = ui::render_analyse_grundbuch(&open_file, &data.loaded_nb);

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
            webview.eval(&format!("replaceEntireScreen(`{}`)", ui::render_entire_screen(data)));
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
            println!("closeFile {}", file_name);
        },
        Cmd::EditRechteArtScript { neu } => {
            data.konfiguration.kurztext_script = neu.trim().to_string();
            data.konfiguration.speichern();
        },
        Cmd::DeleteNebenbeteiligte => {
            use tinyfiledialogs::{YesNo, MessageBoxIcon};
            
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
            
            klassifiziere_pdf_seiten_neu(open_file, &[*seite]);
            webview.eval(&format!("replaceEntireScreen(`{}`)", ui::render_entire_screen(data)));
        },
        Cmd::ClosePopOver { } => {
            data.context_menu_active = None;
            data.configuration_active = false;
            data.info_active = false;
            webview.eval(&format!("replaceEntireScreen(`{}`)", ui::render_entire_screen(data)));
        },
        Cmd::Undo => {
            println!("undo");
        },
        Cmd::Redo => {
            println!("redo");
        },
        Cmd::ImportNebenbeteiligte => {
                        
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
            
            webview.eval(&format!("replaceAnalyseGrundbuch(`{}`);", ui::render_analyse_grundbuch(&open_file, &data.loaded_nb)));
        },
        Cmd::ExportNebenbeteiligte => {
        
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

            let analysiert = data.loaded_files.values().map(|file| {
                LefisDateiExport {
                    rechte: crate::analysiere::analysiere_grundbuch(&file.analysiert, &data.loaded_nb),
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
                Some(f) => f,
                None => return,
            };
            
            let _ = std::fs::write(&f, json.as_bytes());
            
        },
        Cmd::KurztextTesten { text } => {
            let result = python_exec_kurztext(&*text, &data.konfiguration);
            match result {
                Ok(o) => { webview.eval(&format!("replaceKurzTextTestString(`{}`);", format!("{:?}", o).to_uppercase())); },
                Err(e) => { webview.eval(&format!("replaceKurzTextTestString(`{}`);", format!("FEHLER: {}", e))); },
            }
            
        }
        Cmd::SetActiveRibbonTab { new_tab } => {
            data.active_tab = *new_tab;
            webview.eval(&format!("replaceRibbon(`{}`);", ui::render_ribbon(&data)));
        },
        Cmd::SetOpenFile { new_file } => {
            data.open_page = Some((new_file.clone(), 2));

            let open_file = match data.open_page.clone().and_then(|(file, _)| data.loaded_files.get_mut(&file)) { 
                Some(s) => s,
                None => return,
            };
            
            crate::analysiere::roete_bestandsverzeichnis_automatisch(&mut open_file.analysiert.bestandsverzeichnis);

            webview.eval(&format!("replacePageList(`{}`);", ui::render_page_list(&data)));
            webview.eval(&format!("replaceMainContainer(`{}`);", ui::render_main_container(data)));
            webview.eval(&format!("replacePageImage(`{}`);", ui::render_pdf_image(&data)));
        },
        Cmd::SetOpenPage { active_page } => {
            
            if let Some(p) = data.open_page.as_mut() { 
                p.1 = *active_page;
            }
            
            let open_file = match data.open_page.clone().and_then(|(file, _)| data.loaded_files.get_mut(&file)) { 
                Some(s) => s,
                None => return,
            };
            
            crate::analysiere::roete_bestandsverzeichnis_automatisch(&mut open_file.analysiert.bestandsverzeichnis);
            
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
    .flat_map(|file| file.get_nebenbeteiligte())
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

fn klassifiziere_pdf_seiten_neu(pdf: &mut PdfFile, seiten_neu: &[usize]) {
    
    println!("klassifiziere PDF seiten neu: {:?}", seiten_neu);
    
    let max_sz = pdf.seitenzahlen.iter().max().cloned().unwrap_or(0);
    
    let default_parent = Path::new("/");
    let output_parent = Path::new(&pdf.datei).clone().parent().unwrap_or(&default_parent).to_path_buf();
    let file_name = format!("{}_{}", pdf.titelblatt.grundbuch_von, pdf.titelblatt.blatt);
    let cache_output_path = output_parent.clone().join(&format!("{}.cache.gbx", file_name));
    let target_output_path = output_parent.clone().join(&format!("{}.gbx", file_name));
            
    for sz in seiten_neu {

        let seitentyp = match pdf.klassifikation_neu.get(sz) {
            Some(s) => *s,
            None => {
                match digitalisiere::klassifiziere_seitentyp(&pdf.titelblatt, *sz as u32, max_sz) { 
                    Ok(o) => o, 
                    Err(_) => continue, 
                }
            }
        };
                
        let spalten = match digitalisiere::formularspalten_ausschneiden(&pdf.titelblatt, *sz as u32, max_sz, seitentyp, &pdf.pdftotext_layout) { 
            Ok(o) => o, 
            Err(e) => {
                println!("Fehler: {}", e);
                continue;
            }, 
        };
        
        if digitalisiere::ocr_spalten(&pdf.titelblatt, *sz as u32, max_sz, &spalten).is_err() { continue; }
        let textbloecke = match digitalisiere::textbloecke_aus_spalten(&pdf.titelblatt, *sz as u32, &spalten, &pdf.pdftotext_layout) { 
            Ok(o) => o, 
            Err(e) => {
                println!("Fehler: {}", e);
                continue;
            }, 
        };
  
        pdf.geladen.insert(*sz as u32, SeiteParsed {
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
    
    for pdf in pdfs {
    
        thread::spawn(move || {
        
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
            
            for sz in seitenzahlen_zu_laden {
            
                println!("{}: konvertiere seite {} zu png", file_name, sz);
                if digitalisiere::konvertiere_pdf_seiten_zu_png(&datei_bytes, &[sz], &pdf.titelblatt).is_err() { continue; };
                println!("{}: schreibe pdftotext {}", file_name, sz);
                let pdftotext_layout = match digitalisiere::get_pdftotext_layout(&pdf.titelblatt, &[sz]) { Ok(o) => o, Err(_) => continue, };
                println!("{}: digitalisiere OCR {}", file_name, sz);
                for (k, v) in pdftotext_layout.seiten.iter() {
                    pdf.pdftotext_layout.seiten.insert(k.clone(), v.clone());
                }
                if digitalisiere::ocr_seite(&pdf.titelblatt, sz, max_sz).is_err() { continue; }
                println!("{}: bestimme seitentyp {}", file_name, sz);
                let seitentyp = match pdf.klassifikation_neu.get(&(sz as usize)) {
                    Some(s) => *s,
                    None => {
                        match digitalisiere::klassifiziere_seitentyp(&pdf.titelblatt, sz, max_sz) { 
                            Ok(o) => o, 
                            Err(_) => continue, 
                        }
                    }
                };
                println!("{}: schneide formularspalten {}", file_name, sz);
                let spalten = match digitalisiere::formularspalten_ausschneiden(&pdf.titelblatt, sz, max_sz, seitentyp, &pdftotext_layout) { Ok(o) => o, Err(_) => continue, };
                println!("{}: digitalisiere formularspalten {}", file_name, sz);
                if digitalisiere::ocr_spalten(&pdf.titelblatt, sz, max_sz, &spalten).is_err() { continue; }
                println!("{}: lese textbloecke {}", file_name, sz);
                let textbloecke = match digitalisiere::textbloecke_aus_spalten(&pdf.titelblatt, sz, &spalten, &pdftotext_layout) { Ok(o) => o, Err(_) => continue, };
                
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
}

fn analysiere_grundbuch(pdf: &PdfFile) -> Option<Grundbuch> {

    let bestandsverzeichnis = digitalisiere::analysiere_bv(&pdf.geladen).ok()?;
    let abt2 = digitalisiere::analysiere_abt2(&pdf.geladen, &bestandsverzeichnis).ok()?;
    let abt3 = digitalisiere::analysiere_abt3(&pdf.geladen, &bestandsverzeichnis).ok()?;
    
    let mut gb = Grundbuch {
        titelblatt: pdf.titelblatt.clone(),
        bestandsverzeichnis,
        abt2,
        abt3,
    };
    
    crate::analysiere::roete_bestandsverzeichnis_automatisch(&mut gb.bestandsverzeichnis);
    
    Some(gb)
}

fn python_exec_kurztext(text: &str, konfig: &Konfiguration) -> Result<RechteArt, String> {
    
    use pyo3::prelude::*;
    use pyo3::types::PyTuple;
    use crate::kurztext::{RechteArtPyWrapper, SchuldenArtPyWrapper};
    
    let script = konfig.kurztext_script
        .lines()
        .map(|l| format!("    {}", l))
        .collect::<Vec<_>>()
        .join("\r\n");
        
    let script = script.replace("\t", "    ");
    let script = script.replace("\u{00a0}", " ");
    let py_code = format!("def klassifiziere_rechte(*args, **kwargs):\r\n    recht = args\r\n{}", script);
    
    println!("py code:\r\n{}", py_code);
    Python::with_gil(|py| {
        let mut module = PyModule::from_code(py, &py_code, "", "main").map_err(|e| format!("{}", e))?;
        module.add_class::<RechteArtPyWrapper>().map_err(|e| format!("{}", e))?;
        module.add_class::<SchuldenArtPyWrapper>().map_err(|e| format!("{}", e))?;
        
        let fun: Py<PyAny> = module.getattr("klassifiziere_rechte").unwrap().into();
        let tuple = PyTuple::new(py, &[text.trim().to_string()]);
        let result = fun.call1(py, tuple).map_err(|e| format!("{}", e))?;
        let extract = result.extract::<RechteArtPyWrapper>(py).map_err(|e| format!("{}", e))?;
        Ok(extract.inner)
    })  
}

fn main() {

    use std::env;
    
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
