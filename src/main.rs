#![deny(unreachable_code)]

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::analyse::GrundbuchAnalysiert;
use crate::digital::HocrLayout;
use crate::digital::{
    Abt1Eintrag, Abt1GrundEintragung, Abt1Loeschung, Abt1Veraenderung, Abt2Eintrag, Abt2Loeschung,
    Abt2Veraenderung, Abt3Eintrag, Abt3Loeschung, Abt3Veraenderung, Anrede, BvAbschreibung,
    BvEintrag, BvZuschreibung, Grundbuch, Nebenbeteiligter, NebenbeteiligterExport,
    NebenbeteiligterExtra, NebenbeteiligterTyp, SeitenTyp, Titelblatt,
};
use crate::digital::{Abteilung1, Abteilung2, Abteilung3, Bestandsverzeichnis};
use crate::python::{Betrag, PyVm, RechteArt, SchuldenArt};
use analyse::GrundbuchAnalysiertCache;
use digital::HocrSeite;
use digital::ParsedHocr;
use digital::StringOrLines;
use serde_derive::{Deserialize, Serialize};
use tinyfiledialogs::MessageBoxIcon;
use wry::webview::WebView;

const APP_TITLE: &str = "Digitales Grundbuch";
const GTK_OVERLAY_SCROLLING: &str = "GTK_OVERLAY_SCROLLING";

#[cfg(target_os = "windows")]
static TESSERACT_SOURCE_ZIP: &[u8] = include_bytes!("../bin/Tesseract-OCR.zip");
#[cfg(target_os = "windows")]
static PDFTOOLS_SOURCE_ZIP: &[u8] = include_bytes!("../bin/xpdf-tools-win-4.04.zip");
#[cfg(target_os = "windows")]
static QPDF_SOURCE_ZIP: &[u8] = include_bytes!("../bin/qpdf-10.6.3-bin-mingw32.zip");

type FileName = String;

pub mod analyse;
pub mod cmd;
pub mod digital;
pub mod kurztext;
pub mod pdf;
pub mod python;
pub mod ui;

use crate::cmd::Cmd;

#[derive(Debug, Clone)]
pub struct RpcData {
    // UI
    pub active_tab: usize,
    pub popover_state: Option<PopoverState>,
    pub open_page: Option<(FileName, u32)>,

    pub commit_title: String,
    pub commit_msg: String,

    pub loaded_files: BTreeMap<FileName, PdfFile>,
    pub loaded_nb: Vec<Nebenbeteiligter>,
    pub loaded_nb_paths: Vec<String>,

    pub konfiguration: Konfiguration,
    pub vm: PyVm,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadChangeset {
    pub titel: String,
    pub beschreibung: Vec<String>,
    pub fingerprint: String,
    pub signatur: PgpSignatur,
    pub data: UploadChangesetData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PgpSignatur {
    pub hash: String,
    pub pgp_signatur: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadChangesetData {
    pub neu: Vec<PdfFile>,
    pub geaendert: Vec<GbxAenderung>,
}

pub type DateTime = chrono::DateTime<chrono::Local>;

impl UploadChangesetData {
    pub fn format_patch(&self) -> Result<String, String> {
        Ok(serde_json::to_string_pretty(&self)
            .map_err(|e| format!("{e}"))?
            .lines()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join("\r\n"))
    }
    pub fn clear_personal_info(&mut self) {
        for n in self.neu.iter_mut() {
            n.clear_personal_info();
        }

        for g in self.geaendert.iter_mut() {
            g.alt.clear_personal_info();
            g.neu.clear_personal_info();
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum UploadChangesetResponse {
    #[serde(rename = "ok")]
    StatusOk(UploadChangesetResponseOk),
    #[serde(rename = "error")]
    StatusError(UploadChangesetResponseError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadChangesetResponseOk {
    pub neu: Vec<PdfFile>,
    pub geaendert: Vec<PdfFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadChangesetResponseError {
    pub code: isize,
    pub text: String,
}

impl RpcData {
    pub fn get_aenderungen(&self) -> GbxAenderungen {
        let mut neue_dateien = BTreeMap::new();
        let mut geaenderte_dateien = BTreeMap::new();

        for (file_name, open_file) in self.loaded_files.iter() {
            if !open_file.ist_geladen() {
                continue;
            }

            let mut open_file = open_file.clone();
            open_file.clear_personal_info();

            let json = match serde_json::to_string_pretty(&open_file) {
                Ok(o) => o,
                Err(_) => continue,
            };

            match std::fs::read_to_string(
                Path::new(&Konfiguration::backup_dir())
                    .join("backup")
                    .join(&format!("{file_name}.gbx")),
            ) {
                Ok(o) => {
                    let mut o_parsed: PdfFile = match serde_json::from_str(&o) {
                        Ok(o) => o,
                        Err(_) => {
                            neue_dateien.insert(file_name.clone(), open_file.clone());
                            continue;
                        }
                    };

                    o_parsed.clear_personal_info();

                    let o_json = match serde_json::to_string_pretty(&o_parsed) {
                        Ok(o) => o,
                        Err(_) => {
                            neue_dateien.insert(file_name.clone(), open_file.clone());
                            continue;
                        }
                    };

                    if o_json != json {
                        geaenderte_dateien.insert(
                            file_name.clone(),
                            GbxAenderung {
                                alt: o_parsed,
                                neu: open_file.clone(),
                            },
                        );
                    }
                }
                Err(_) => {
                    neue_dateien.insert(file_name.clone(), open_file.clone());
                }
            }
        }

        GbxAenderungen {
            neue_dateien,
            geaenderte_dateien,
        }
    }

    pub fn reset_diff_backup_files(&self, changed_files: &UploadChangesetResponseOk) {
        let path = Path::new(&Konfiguration::backup_dir()).join("backup");

        for new_state in changed_files.neu.iter() {
            if let Ok(json) = serde_json::to_string_pretty(&new_state) {
                let file_name = format!(
                    "{}_{}.gbx",
                    new_state.analysiert.titelblatt.grundbuch_von,
                    new_state.analysiert.titelblatt.blatt
                );
                let _ = fs::write(
                    path.clone().join(&format!("{file_name}.gbx")),
                    json.as_bytes(),
                );
            }
        }

        for new_state in changed_files.geaendert.iter() {
            if let Ok(json) = serde_json::to_string_pretty(&new_state) {
                let file_name = format!(
                    "{}_{}.gbx",
                    new_state.analysiert.titelblatt.grundbuch_von,
                    new_state.analysiert.titelblatt.blatt
                );
                let _ = fs::write(
                    path.clone().join(&format!("{file_name}.gbx")),
                    json.as_bytes(),
                );
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
        let open_file = match self
            .open_page
            .clone()
            .and_then(|(file, _)| self.loaded_files.get(&file))
        {
            Some(s) => s,
            None => return true,
        };

        open_file.datei.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GbxAenderungen {
    pub neue_dateien: BTreeMap<FileName, PdfFile>,
    pub geaenderte_dateien: BTreeMap<FileName, GbxAenderung>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GbxAenderung {
    pub alt: PdfFile,
    pub neu: PdfFile,
}

impl GbxAenderungen {
    pub fn ist_leer(&self) -> bool {
        self.neue_dateien.is_empty() && self.geaenderte_dateien.is_empty()
    }
}

#[derive(Debug, Copy, PartialEq, PartialOrd, Clone)]
pub enum PopoverState {
    ContextMenu(ContextMenuData),
    Info,
    ExportPdf,
    CreateNewGrundbuch,
    GrundbuchSuchenDialog,
    GrundbuchUploadDialog(usize),
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
    pub fn create_diff_save_point(&self, file_name: &FileName, file: PdfFile) {
        let json = match serde_json::to_string_pretty(&file) {
            Ok(o) => o,
            Err(_) => return,
        };
        let _ = std::fs::create_dir_all(&format!("{}/backup/", Konfiguration::backup_dir()));
        let target_path = format!("{}/backup/{}.gbx", Konfiguration::backup_dir(), file_name);
        if !Path::new(&target_path).exists() {
            let _ = std::fs::write(&target_path, json.as_bytes());
        }
    }

    pub fn get_changed_files(&self) -> Vec<(String, PdfFile)> {
        self.loaded_files
            .iter()
            .filter(|(file_name, lf)| {
                let json = match serde_json::to_string_pretty(&lf) {
                    Ok(o) => o,
                    Err(_) => return true,
                };
                let _ =
                    std::fs::create_dir_all(&format!("{}/backup/", Konfiguration::backup_dir()));
                let target_path =
                    format!("{}/backup/{}.gbx", Konfiguration::backup_dir(), file_name);
                if let Ok(exist) = std::fs::read_to_string(&target_path) {
                    if exist == json {
                        false
                    } else {
                        true
                    }
                } else {
                    true
                }
            })
            .map(|(file_name, lf)| (file_name.clone(), lf.clone()))
            .collect()
    }
}

impl Default for RpcData {
    fn default() -> Self {
        Self {
            active_tab: 0,
            open_page: None,
            popover_state: None,
            loaded_files: BTreeMap::new(),
            commit_title: String::new(),
            commit_msg: String::new(),
            loaded_nb: Vec::new(),
            loaded_nb_paths: Vec::new(),
            konfiguration: Konfiguration::neu_laden()
                .unwrap_or(Konfiguration::parse_from(Konfiguration::DEFAULT).unwrap()),
            vm: PyVm::new().unwrap(),
        }
    }
}

fn default_server_url() -> String {
    format!("https://127.0.0.1")
}
fn default_server_email() -> String {
    format!("max@mustermann.de")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfFile {
    // Pfad der zugehörigen .pdf-Datei
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    datei: Option<String>,
    // Some(pfad) wenn Datei digital angelegt wurde
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    gbx_datei_pfad: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    land: Option<String>,
    #[serde(skip_serializing_if = "HocrLayout::is_empty")]
    #[serde(default)]
    hocr: HocrLayout,
    #[serde(skip, default)]
    icon: Option<PdfFileIcon>,
    #[serde(default)]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    anpassungen_seite: BTreeMap<String, AnpassungSeite>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    nebenbeteiligte_dateipfade: Vec<String>,
    #[serde(skip, default)]
    next_state: Option<Box<PdfFile>>,
    #[serde(skip, default)]
    previous_state: Option<Box<PdfFile>>,
    analysiert: Grundbuch,
    #[serde(skip, default)]
    cache: GrundbuchAnalysiertCache,
}

impl PdfFile {
    pub fn get_seitenzahlen(&self) -> Vec<u32> {
        self.datei
            .clone()
            .and_then(|p| fs::read(p).ok())
            .and_then(|pdf_bytes: Vec<u8>| digital::lese_seitenzahlen(&pdf_bytes).ok())
            .unwrap_or_default()
    }
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
            PdfFileIcon::HatFehler => format!(
                "data:image/png;base64,{}",
                base64::encode(&WARNING_CHECK_PNG)
            ),
            PdfFileIcon::KeineOrdnungsnummernZugewiesen => {
                format!("data:image/png;base64,{}", base64::encode(&HALF_CHECK_PNG))
            }
            PdfFileIcon::AllesOkay => {
                format!("data:image/png;base64,{}", base64::encode(&FULL_CHECK_PNG))
            }
        }
    }
}

pub type ZeilenId = u32;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnpassungSeite {
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub klassifikation_neu: Option<SeitenTyp>,
    #[serde(default)]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub spalten: BTreeMap<String, Rect>,
    #[serde(default)]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    zeilen: BTreeMap<ZeilenId, f32>,
    #[serde(default)]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    zeilen_auto: BTreeMap<ZeilenId, f32>,
}

impl AnpassungSeite {
    pub fn get_zeilen(&self) -> BTreeMap<u32, f32> {
        let mut z = self.zeilen.clone();
        z.append(&mut self.zeilen_auto.clone());
        z
    }

    pub fn insert_zeile_manuell(&mut self, zeile: f32) {
        let random_id = rand::random::<u32>();
        self.zeilen.insert(random_id, zeile);
    }

    pub fn delete_zeile_manuell(&mut self, zeile: ZeilenId) {
        let _ = self.zeilen.remove(&zeile);
        let _ = self.zeilen_auto.remove(&zeile);
    }
}

#[derive(Debug, Clone, Default, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Rect {
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
}

impl Rect {
    pub fn zero() -> Self {
        Self::default()
    }

    pub fn contains_point(&self, x: f32, y: f32) -> bool {
        x <= self.max_x && x >= self.min_x && y <= self.max_y && y >= self.min_y
    }

    pub fn overlaps(&self, other: &Rect) -> bool {
        if self.max_x < other.min_x || self.min_x > other.max_x {
            return false;
        }
        if self.max_y < other.min_y || self.min_y > other.max_y {
            return false;
        }
        true
    }
}

impl PdfFile {
    pub fn get_gbx_datei_parent(&self) -> PathBuf {
        let default_parent = Path::new("/");
        match (self.datei.as_ref(), self.gbx_datei_pfad.as_ref()) {
            (Some(pdf), None) | (Some(pdf), Some(_)) => Path::new(&pdf)
                .clone()
                .parent()
                .unwrap_or(&default_parent)
                .to_path_buf(),
            (None, Some(gbx)) => Path::new(&gbx).to_path_buf(),
            (None, None) => default_parent.to_path_buf(),
        }
    }

    pub fn get_seiten_typ(&self, seite: &str) -> Option<SeitenTyp> {
        if let Some(override_seitentyp) = self
            .anpassungen_seite
            .get(seite)
            .and_then(|s| s.klassifikation_neu.clone())
        {
            return Some(override_seitentyp);
        }

        let hocr_seite = self.hocr.seiten.get(seite)?;
        let querformat = hocr_seite.breite_mm > hocr_seite.hoehe_mm;

        crate::digital::klassifiziere_seitentyp(&hocr_seite, querformat).ok()
    }

    pub fn clear_personal_info(&mut self) {
        self.datei = if self.datei.is_some() {
            Some(String::new())
        } else {
            None
        };
        self.gbx_datei_pfad = if self.gbx_datei_pfad.is_some() {
            Some(String::new())
        } else {
            None
        };
        self.nebenbeteiligte_dateipfade.clear();
    }

    pub fn get_gbx_datei_pfad(&self) -> PathBuf {
        let file_name = format!(
            "{}_{}",
            self.analysiert.titelblatt.grundbuch_von, self.analysiert.titelblatt.blatt
        );
        self.get_gbx_datei_parent()
            .join(&format!("{}.gbx", file_name))
    }

    pub fn speichern(&self) {
        let target_output_path = self.get_gbx_datei_pfad();
        let json = match serde_json::to_string_pretty(&self) {
            Ok(o) => o,
            Err(_) => return,
        };
        let _ = std::fs::write(&target_output_path, json.as_bytes());
    }

    pub fn get_icon(
        &self,
        vm: PyVm,
        nb: &[Nebenbeteiligter],
        konfiguration: &Konfiguration,
    ) -> Option<PdfFileIcon> {
        return None;
        /*
        if !self.ist_geladen() {
            return None;
        }

        if !self.hat_keine_fehler(vm.clone(), nb, konfiguration) {
            return Some(PdfFileIcon::HatFehler);
        }

        if !self.alle_ordnungsnummern_zugewiesen(vm, nb, konfiguration) {
            return Some(PdfFileIcon::KeineOrdnungsnummernZugewiesen);
        }

        Some(PdfFileIcon::AllesOkay)
        */
    }

    pub fn ist_geladen(&self) -> bool {
        let tempdir = std::env::temp_dir()
            .join(&self.analysiert.titelblatt.grundbuch_von)
            .join(self.analysiert.titelblatt.blatt.to_string());

        for s in self.get_seitenzahlen().iter() {
            if !tempdir.join(format!("{s}.hocr.json")).exists() {
                return false;
            }
        }

        true
    }

    pub fn hat_keine_fehler(
        &self,
        vm: PyVm,
        nb: &[Nebenbeteiligter],
        konfiguration: &Konfiguration,
    ) -> bool {
        let analysiert = self
            .cache
            .start_analyzing(&self.analysiert, &vm, nb, konfiguration);

        self.ist_geladen()
            && analysiert.abt2.iter().all(|e| e.fehler.is_empty())
            && analysiert.abt3.iter().all(|e| e.fehler.is_empty())
    }

    pub fn alle_ordnungsnummern_zugewiesen(
        &self,
        vm: PyVm,
        nb: &[Nebenbeteiligter],
        konfiguration: &Konfiguration,
    ) -> bool {
        let analysiert = self
            .cache
            .start_analyzing(&self.analysiert, &vm, nb, konfiguration);

        let any_abt2 = analysiert.abt2.iter().any(|e| {
            e.warnungen
                .iter()
                .any(|w| w == "Konnte keine Ordnungsnummer finden")
        });

        let any_abt3 = analysiert.abt3.iter().any(|e| {
            e.warnungen
                .iter()
                .any(|w| w == "Konnte keine Ordnungsnummer finden")
        });

        self.ist_geladen() && !any_abt2 && !any_abt3
    }

    pub fn get_nebenbeteiligte(
        &self,
        vm: PyVm,
        konfiguration: &Konfiguration,
    ) -> Vec<NebenbeteiligterExport> {
        let mut v = Vec::new();

        let analysiert =
            self.cache
                .start_and_block_until_finished(&self.analysiert, &vm, &[], konfiguration);

        for abt2 in &analysiert.abt2 {
            if !abt2.rechtsinhaber.is_empty() {
                v.push(NebenbeteiligterExport {
                    ordnungsnummer: None,
                    recht: format!(
                        "{} Blatt {}, Abt. 2/{}",
                        self.analysiert.titelblatt.grundbuch_von,
                        self.analysiert.titelblatt.blatt,
                        abt2.lfd_nr
                    ),
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
                    recht: format!(
                        "{} Blatt {}, Abt. 3/{}",
                        self.analysiert.titelblatt.grundbuch_von,
                        self.analysiert.titelblatt.blatt,
                        abt3.lfd_nr
                    ),
                    typ: NebenbeteiligterTyp::from_str(&abt3.rechtsinhaber),
                    name: abt3.rechtsinhaber.clone(),
                    extra: NebenbeteiligterExtra::default(),
                });
            }
        }

        v
    }
}

impl Konfiguration {
    pub fn get_hash(&self) -> String {
        use sha2::Digest;

        let arr = serde_json::to_string(&[
            serde_json::to_string(&self.regex).unwrap_or_default(),
            serde_json::to_string(&self.abkuerzungen_script).unwrap_or_default(),
            serde_json::to_string(&self.text_saubern_script).unwrap_or_default(),
            serde_json::to_string(&self.flurstuecke_auslesen_script).unwrap_or_default(),
            serde_json::to_string(&self.text_kuerzen_abt2_script).unwrap_or_default(),
            serde_json::to_string(&self.text_kuerzen_abt3_script).unwrap_or_default(),
            serde_json::to_string(&self.betrag_auslesen_script).unwrap_or_default(),
            serde_json::to_string(&self.rechtsinhaber_auslesen_abt3_script).unwrap_or_default(),
            serde_json::to_string(&self.rechtsinhaber_auslesen_abt2_script).unwrap_or_default(),
            serde_json::to_string(&self.rangvermerk_auslesen_abt2_script).unwrap_or_default(),
            serde_json::to_string(&self.klassifiziere_rechteart).unwrap_or_default(),
            serde_json::to_string(&self.klassifiziere_schuldenart).unwrap_or_default(),
        ])
        .unwrap_or_default();

        let mut hasher = sha2::Sha256::default();
        hasher.update(arr.as_bytes());
        let hash = hasher.finalize();
        hex::encode(hash)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Konfiguration {
    #[serde(skip, default)]
    pub tab: usize,
    #[serde(skip, default)]
    pub dateiliste_ausblenden: bool,
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
    #[serde(default)]
    pub klassifiziere_rechteart: Vec<String>,
    #[serde(default)]
    pub klassifiziere_schuldenart: Vec<String>,
}

fn default_passwort_speichern() -> bool {
    true
}

pub mod pgp {

    use sequoia_openpgp::parse::Parse;
    use sequoia_openpgp::policy::Policy;
    use sequoia_openpgp::serialize::stream::*;
    use std::io::Write;

    pub fn parse_cert(cert: &[u8]) -> Result<sequoia_openpgp::Cert, String> {
        use sequoia_openpgp::parse::PacketParser;

        let ppr = PacketParser::from_bytes(cert).map_err(|e| format!("{e}"))?;

        sequoia_openpgp::Cert::try_from(ppr).map_err(|e| format!("{e}"))
    }

    pub fn sign(
        p: &dyn Policy,
        sink: &mut (dyn Write + Send + Sync),
        plaintext: &str,
        tsk: &sequoia_openpgp::Cert,
    ) -> sequoia_openpgp::Result<()> {
        // Get the keypair to do the signing from the Cert.
        let keypair = tsk
            .keys()
            .unencrypted_secret()
            .with_policy(p, None)
            .supported()
            .alive()
            .revoked(false)
            .for_signing()
            .next()
            .unwrap()
            .key()
            .clone()
            .into_keypair()?;

        // Start streaming an OpenPGP message.
        let message = Message::new(sink);

        // We want to sign a literal data packet.
        let mut signer = Signer::new(message, keypair)
            .detached()
            .cleartext()
            .build()?;

        // Sign the data.
        signer.write_all(plaintext.as_bytes())?;

        // Finalize the OpenPGP message to make sure that all data is
        // written.
        signer.finalize()?;

        Ok(())
    }
}

impl Konfiguration {
    const DEFAULT: &'static str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/Konfiguration.json"
    ));
    const FILE_NAME: &'static str = "Konfiguration.json";

    pub fn parse_from(s: &str) -> Result<Konfiguration, String> {
        match serde_json::from_str::<Konfiguration>(s) {
            Ok(o) => Ok(o),
            Err(e) => Err(format!(
                "Fehler in Konfiguration {}: {}",
                Self::konfiguration_pfad(),
                e
            )),
        }
    }

    pub fn create_empty_diff_save_point(&self, file_name: &FileName) {
        let _ = std::fs::create_dir_all(&format!("{}/backup/", Konfiguration::backup_dir()));
        let target_path = format!("{}/backup/{}.gbx", Konfiguration::backup_dir(), file_name);
        let _ = std::fs::write(&target_path, "".as_bytes());
    }

    pub fn get_cert(&self) -> Result<sequoia_openpgp::Cert, String> {
        use sequoia_openpgp::policy::StandardPolicy as P;

        let p = &P::new();

        let base64 = self
            .server_privater_schluessel_base64
            .as_ref()
            .ok_or(format!(
                "Kein privater Schlüssel in Konfiguration eingestellt"
            ))?;

        let privater_schluessel_dekodiert = base64::decode(&base64)
            .map_err(|e| format!("Privater Schlüssel ist nicht im richtigen Format: {e}"))?;

        let cert = self::pgp::parse_cert(&privater_schluessel_dekodiert)
            .map_err(|e| format!("Privater Schlüssel ist nicht im richtigen Format: {e}"))?;

        let policy_cert = cert
            .with_policy(p, None)
            .map_err(|e| format!("Privater Schlüssel ist nicht im richtigen Format: {e}"))?;

        if let Err(e) = policy_cert.alive() {
            return Err(format!("Zertifikat ist abgelaufen: {e}"));
        }

        Ok(cert)
    }

    pub fn get_private_key_fingerprint(&self) -> Result<String, String> {
        let cert = self.get_cert()?;
        Ok(cert.fingerprint().to_hex())
    }

    pub fn sign_message(&self, msg: &str) -> Result<(String, Vec<String>), String> {
        use sequoia_openpgp::policy::StandardPolicy as P;

        let p = &P::new();

        let cert = self.get_cert()?;
        let mut signature = Vec::new();

        self::pgp::sign(p, &mut signature, msg, &cert).map_err(|e| format!("{e}"))?;

        let sig_str =
            String::from_utf8(signature).map_err(|e| format!("Ungültige Signatur: {e}"))?;

        let lines = sig_str.lines().map(|s| s.to_string()).collect::<Vec<_>>();

        let hash = lines
            .get(1)
            .map(|s| s.replace("Hash: ", "").trim().to_string())
            .ok_or(format!(
                "Ungültige Hashfunktion in Zeile 2: {:?}",
                lines.get(1)
            ))?;

        let begin_pgp_signature_line = lines
            .iter()
            .position(|l| l.contains("BEGIN PGP SIGNATURE"))
            .ok_or(format!(
                "Ungültige PGP-Signatur: Kein BEGIN PGP SIGNATURE gefunden"
            ))?;

        let end_pgp_signature_line = lines
            .iter()
            .position(|l| l.contains("END PGP SIGNATURE"))
            .ok_or(format!(
                "Ungültige PGP-Signatur: Kein END PGP SIGNATURE gefunden"
            ))?;

        let min = begin_pgp_signature_line.min(end_pgp_signature_line);
        let max = end_pgp_signature_line.max(begin_pgp_signature_line);
        let mut signatur = Vec::new();

        for i in min..max {
            let line = lines.get(i).ok_or(format!("Ungültige PGP-Signatur"))?;
            if line.trim().is_empty() {
                continue;
            }
            if line.contains("BEGIN PGP SIGNATURE") || line.contains("END PGP SIGNATURE") {
                continue;
            }
            signatur.push(line.trim().to_string());
        }

        Ok((hash, signatur))
    }

    pub fn get_passwort(&self) -> Option<String> {
        let pw_file_path = std::env::temp_dir().join("dgb").join("passwort.txt");
        match std::fs::read_to_string(pw_file_path) {
            Ok(o) => return Some(o.trim().to_string()),
            Err(_) => {
                let email = &self.server_email;
                let pw = tinyfiledialogs::password_box(
                    &format!("Passwort für {email} eingeben"),
                    &format!("Bitte geben Sie das Passwort für {email} ein:"),
                )?;

                let _ = std::fs::create_dir_all(std::env::temp_dir().join("dgb"));
                let _ = std::fs::write(
                    std::env::temp_dir().join("dgb").join("passwort.txt"),
                    pw.clone().as_bytes(),
                );

                Some(pw)
            }
        }
    }

    pub fn backup_dir() -> String {
        dirs::config_dir()
            .and_then(|p| Some(p.join("dgb").to_str()?.to_string()))
            .or(std::env::current_exe()
                .ok()
                .and_then(|p| Some(p.parent()?.to_path_buf().join("dgb").to_str()?.to_string())))
            .unwrap_or(format!("./dgb/"))
    }

    pub fn konfiguration_pfad() -> String {
        dirs::config_dir()
            .and_then(|p| Some(p.join("dgb").join(Self::FILE_NAME).to_str()?.to_string()))
            .or(std::env::current_exe().ok().and_then(|p| {
                Some(
                    p.parent()?
                        .to_path_buf()
                        .join("dgb")
                        .join(Self::FILE_NAME)
                        .to_str()?
                        .to_string(),
                )
            }))
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
            Ok(o) => Self::parse_from(&o)?,
            Err(e) => {
                return Err(format!(
                    "Fehler beim Lesen von Konfiguration in {}: {}",
                    Self::konfiguration_pfad(),
                    e
                ))
            }
        };

        Ok(konfig)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AboNeuAnfrage {
    #[serde(rename = "ok")]
    Ok(AboNeuAnfrageOk),
    #[serde(rename = "error")]
    Err(AboNeuAnfrageErr),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AboNeuAnfrageOk {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AboNeuAnfrageErr {
    pub code: usize,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum GrundbuchSucheResponse {
    #[serde(rename = "ok")]
    StatusOk(GrundbuchSucheOk),
    #[serde(rename = "error")]
    StatusErr(GrundbuchSucheError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrundbuchSucheOk {
    pub grundbuecher: Vec<GrundbuchSucheErgebnis>,
    pub aenderungen: Vec<CommitSucheErgebnis>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GrundbuchSucheErgebnis {
    pub titelblatt: Titelblatt,
    pub ergebnis: SuchErgebnisGrundbuch,
    pub abos: Vec<AbonnementInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CommitSucheErgebnis {
    pub aenderung_id: String,
    pub ergebnis: SuchErgebnisAenderung,
    pub titelblaetter: Vec<Titelblatt>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SuchErgebnisAenderung {
    pub aenderungs_id: String,
    pub bearbeiter: String,
    pub datum: String,
    pub titel: String,
    pub beschreibung: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SuchErgebnisGrundbuch {
    pub land: String,
    pub amtsgericht: String,
    pub grundbuch_von: String,
    pub blatt: String,
    pub abteilung: String,
    pub lfd_nr: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AbonnementInfo {
    pub amtsgericht: String,
    pub grundbuchbezirk: String,
    pub blatt: i32,
    pub text: String,
    pub aktenzeichen: String,
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

fn webview_cb(webview: &WebView, arg: &Cmd, data: &mut RpcData) {
    match &arg {
        Cmd::CheckForGrundbuchLoaded => {
            let open_file = match data
                .open_page
                .clone()
                .and_then(|(file, _)| data.loaded_files.get(&file))
            {
                Some(s) => s,
                None => return,
            };

            if !open_file.ist_geladen() {
                return;
            }

            let analyse = open_file.cache.start_analyzing(
                &open_file.analysiert,
                &data.vm,
                &data.loaded_nb,
                &data.konfiguration,
            );

            if data.konfiguration.lefis_analyse_einblenden {
                let _ = webview.evaluate_script(&format!(
                    "replaceAnalyseGrundbuch(`{}`);",
                    ui::render_analyse_grundbuch(&analyse, false, false)
                ));
            }
        }
        Cmd::SignalPdfPageRendered {
            pdf_amtsgericht,
            pdf_grundbuch_von,
            pdf_blatt,
            seite,
            image_data_base64,
        } => {
            let pdf_grundbuch_von = pdf_grundbuch_von.clone();
            let pdf_blatt = pdf_blatt.clone();
            let image_data_base64 = image_data_base64.clone();
            let seite = seite.clone();
            let image_filename = format!("page-clean-{seite}.png");

            std::thread::spawn(move || {
                use image::io::Reader as ImageReader;
                use std::io::Cursor;
                const DATA_START: &str = "data:image/png;base64,";
                if !image_data_base64.starts_with(DATA_START) {
                    return;
                }

                let output_image = match base64::decode(&image_data_base64[DATA_START.len()..]) {
                    Ok(o) => o,
                    Err(_) => {
                        return;
                    }
                };

                let reader = match ImageReader::new(Cursor::new(output_image)).with_guessed_format()
                {
                    Ok(o) => o,
                    Err(_) => {
                        return;
                    }
                };

                let decoded = match reader.decode() {
                    Ok(o) => o,
                    Err(_) => {
                        return;
                    }
                };

                let flipped = decoded.flipv();
                let grayscale = flipped.grayscale();

                let mut bytes: Vec<u8> = Vec::new();
                let _ =
                    grayscale.write_to(&mut Cursor::new(&mut bytes), image::ImageOutputFormat::Png);

                let tempdir = std::env::temp_dir()
                    .join(&pdf_grundbuch_von)
                    .join(&pdf_blatt);
                let _ = std::fs::create_dir_all(&tempdir);
                let _ = std::fs::write(tempdir.join(&image_filename), &bytes);
                let pnm_bytes = match crate::digital::read_png_and_convert_to_bmp(
                    &tempdir.join(&image_filename),
                ) {
                    Some(b) => b,
                    None => return,
                };

                let target_path = tempdir.join(format!("{seite}.hocr.json"));
                if !target_path.exists() {
                    let hocr = match tesseract_get_hocr(&pnm_bytes) {
                        Ok(o) => o,
                        Err(e) => {
                            tinyfiledialogs::message_box_ok(
                                &format!("Fehler beim OCR von {image_filename}"),
                                &format!("{e}"),
                                MessageBoxIcon::Error,
                            );
                            return;
                        }
                    };

                    let _ = std::fs::write(
                        &target_path,
                        serde_json::to_string_pretty(&hocr).unwrap_or_default(),
                    );
                }
            });
        }
        Cmd::Init => {
            let _ = webview.evaluate_script(&format!(
                "replaceEntireScreen(`{}`)",
                ui::render_entire_screen(data)
            ));
        }
        Cmd::LoadPdf => {
            let file_dialog_result = tinyfiledialogs::open_file_dialog_multi(
                "Grundbuchblatt-PDF Datei(en) auswählen",
                "",
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

                if let Some(mut grundbuch_json_parsed) = String::from_utf8(datei_bytes.clone())
                    .ok()
                    .and_then(|s| serde_json::from_str::<PdfFile>(&s).ok())
                {
                    let file_name = format!(
                        "{}_{}",
                        grundbuch_json_parsed.analysiert.titelblatt.grundbuch_von,
                        grundbuch_json_parsed.analysiert.titelblatt.blatt
                    );

                    for nb_datei in grundbuch_json_parsed.nebenbeteiligte_dateipfade.iter() {
                        if let Some(mut nb) = std::fs::read_to_string(&nb_datei)
                            .ok()
                            .map(|fs| parse_nb(&fs))
                        {
                            data.loaded_nb.append(&mut nb);
                            data.loaded_nb.sort_by(|a, b| a.name.cmp(&b.name));
                            data.loaded_nb.dedup();
                            data.loaded_nb_paths.push(nb_datei.clone());
                            data.loaded_nb_paths.sort();
                            data.loaded_nb_paths.dedup();
                        }
                    }

                    data.loaded_files
                        .insert(file_name.clone(), grundbuch_json_parsed.clone());
                    data.create_diff_save_point(&file_name.clone(), grundbuch_json_parsed.clone());
                    pdf_zu_laden.push(grundbuch_json_parsed);
                    if data.open_page.is_none() {
                        data.open_page = Some((file_name.clone(), 2));
                    }
                } else {
                    let seiten_dimensionen = match digital::get_seiten_dimensionen(&datei_bytes) {
                        Ok(o) => o,
                        Err(e) => continue,
                    };

                    let seitenzahlen = crate::digital::lese_seitenzahlen(&datei_bytes)
                        .ok()
                        .unwrap_or_default();

                    let max_sz = seitenzahlen.iter().max().cloned().unwrap_or(0);

                    let titelblatt = match digital::lese_titelblatt(&datei_bytes) {
                        Ok(o) => o,
                        Err(_) => {
                            continue;
                        }
                    };

                    let default_parent = Path::new("/");
                    let output_parent = Path::new(&d)
                        .parent()
                        .unwrap_or(&default_parent)
                        .to_path_buf();
                    let file_name = format!("{}_{}", titelblatt.grundbuch_von, titelblatt.blatt);
                    let cache_output_path = output_parent
                        .clone()
                        .join(&format!("{}.cache.gbx", file_name));
                    let target_output_path =
                        output_parent.clone().join(&format!("{}.gbx", file_name));

                    if !Path::new(&target_output_path).exists() {
                        data.konfiguration.create_empty_diff_save_point(&file_name);
                    }

                    // Lösche Titelblattseite von Seiten, die gerendert werden müssen
                    let mut pdf_parsed = PdfFile {
                        datei: Some(d.to_string()),
                        gbx_datei_pfad: None,
                        icon: None,
                        land: None,
                        hocr: HocrLayout::init_from_dimensionen(&seiten_dimensionen),
                        anpassungen_seite: BTreeMap::new(),
                        analysiert: Grundbuch::new(titelblatt),
                        cache: GrundbuchAnalysiertCache::default(),
                        nebenbeteiligte_dateipfade: Vec::new(),
                        previous_state: None,
                        next_state: None,
                    };

                    if let Some(cached_pdf) = std::fs::read_to_string(&cache_output_path)
                        .ok()
                        .and_then(|s| serde_json::from_str(&s).ok())
                    {
                        pdf_parsed = cached_pdf;
                    }

                    if let Some(mut target_pdf) = std::fs::read_to_string(&target_output_path)
                        .ok()
                        .and_then(|s| serde_json::from_str::<PdfFile>(&s).ok())
                    {
                        let json = match serde_json::to_string_pretty(&target_pdf) {
                            Ok(o) => o,
                            Err(_) => continue,
                        };
                        let _ = std::fs::write(&target_output_path, json.as_bytes());
                        pdf_parsed = target_pdf.clone();
                        data.create_diff_save_point(&file_name, target_pdf.clone());
                    }

                    for nb_datei in pdf_parsed.nebenbeteiligte_dateipfade.iter() {
                        if let Some(mut nb) = std::fs::read_to_string(&nb_datei)
                            .ok()
                            .map(|fs| parse_nb(&fs))
                        {
                            data.loaded_nb.append(&mut nb);
                            data.loaded_nb.sort_by(|a, b| a.name.cmp(&b.name));
                            data.loaded_nb.dedup();
                            data.loaded_nb_paths.push(nb_datei.clone());
                            data.loaded_nb_paths.sort();
                            data.loaded_nb_paths.dedup();
                        }
                    }

                    let json = match serde_json::to_string_pretty(&pdf_parsed) {
                        Ok(o) => o,
                        Err(_) => continue,
                    };
                    let _ = std::fs::write(&cache_output_path, json.as_bytes());
                    data.loaded_files
                        .insert(file_name.clone(), pdf_parsed.clone());
                    pdf_zu_laden.push(pdf_parsed);
                    if data.open_page.is_none() {
                        data.open_page = Some((file_name.clone(), 2));
                    }
                }
            }

            let html_inner = ui::render_entire_screen(data);
            let _ = webview.evaluate_script(&format!("replaceEntireScreen(`{}`)", html_inner));
            let _ = webview.evaluate_script("startCheckingForPdfErrors()");

            for pdf_parsed in &pdf_zu_laden {
                let output_parent = pdf_parsed.get_gbx_datei_parent();
                let file_name = format!(
                    "{}_{}",
                    pdf_parsed.analysiert.titelblatt.grundbuch_von,
                    pdf_parsed.analysiert.titelblatt.blatt
                );
                let cache_output_path = output_parent
                    .clone()
                    .join(&format!("{}.cache.gbx", file_name));
                let _ = webview.evaluate_script(&format!(
                    "startCheckingForPageLoaded(`{}`, `{}`, `{}`)",
                    cache_output_path.display(),
                    file_name,
                    pdf_parsed.datei.clone().unwrap_or_default()
                ));
            }

            render_pdf_seiten(webview, &mut pdf_zu_laden);
        }
        Cmd::CreateNewGrundbuch => {
            data.popover_state = Some(PopoverState::CreateNewGrundbuch);
            let _ = webview.evaluate_script(&format!(
                "replacePopOver(`{}`)",
                ui::render_popover_content(data)
            ));
        }
        Cmd::OpenGrundbuchSuchenDialog => {
            data.popover_state = Some(PopoverState::GrundbuchSuchenDialog);
            let _ = webview.evaluate_script(&format!(
                "replacePopOver(`{}`)",
                ui::render_popover_content(data)
            ));
        }
        Cmd::OpenGrundbuchUploadDialog => {
            if data.loaded_files.is_empty() {
                return;
            }

            let passwort = match data.konfiguration.get_passwort() {
                Some(s) => s,
                None => return,
            };

            let dateien = data.loaded_files.clone();
            let konfiguration = data.konfiguration.clone();

            for (_, d) in dateien {
                if let Err(e) = try_download_file_database(
                    konfiguration.clone(),
                    d.analysiert.titelblatt.clone(),
                ) {
                    if let Some(msg) = e {
                        let msg = msg.replace("\"", "").replace("'", "");
                        let file_name = format!(
                            "{}_{}",
                            d.analysiert.titelblatt.grundbuch_von, d.analysiert.titelblatt.blatt
                        );
                        tinyfiledialogs::message_box_ok(
                            "Fehler beim Synchronisieren mit Datenbank", 
                            &format!("Der aktuelle Stand von {file_name}.gbx konnte nicht aus der Datenbank geladen werden:\r\n{msg}\r\nBitte überprüfen Sie das Passwort oder wenden Sie sich an einen Administrator."), 
                            MessageBoxIcon::Error
                        );
                    }
                    let _ =
                        std::fs::remove_file(std::env::temp_dir().join("dgb").join("passwort.txt"));
                    return;
                }
            }

            let aenderungen = data.get_aenderungen();
            if aenderungen.ist_leer() {
                tinyfiledialogs::message_box_ok(
                    "Keine Änderungen zum Hochladen vorhanden", 
                    "Es sind noch keine Änderungen zum Hochladen vorhanden.\r\nAlle Dateien sind bereits auf dem neuesten Stand.", 
                    MessageBoxIcon::Info
                );
                return;
            }

            data.popover_state = Some(PopoverState::GrundbuchUploadDialog(0));
            let _ = webview.evaluate_script(&format!(
                "replacePopOver(`{}`)",
                ui::render_popover_content(data)
            ));
        }
        Cmd::GrundbuchAnlegen {
            land,
            grundbuch_von,
            amtsgericht,
            blatt,
        } => {
            let file_dialog_result =
                tinyfiledialogs::select_folder_dialog(".gbx-Datei speichern unter...", "");

            let gbx_folder = match file_dialog_result {
                Some(f) => f,
                None => return,
            };

            let file_name = format!("{}_{}", grundbuch_von, blatt);

            let pdf_parsed = PdfFile {
                datei: None,
                gbx_datei_pfad: Some(gbx_folder),
                icon: None,
                land: Some(land.trim().to_string()),
                hocr: HocrLayout::default(),
                analysiert: Grundbuch {
                    titelblatt: Titelblatt {
                        amtsgericht: amtsgericht.trim().to_string().clone(),
                        grundbuch_von: grundbuch_von.trim().to_string().clone(),
                        blatt: blatt.clone(),
                    },
                    bestandsverzeichnis: Bestandsverzeichnis::default(),
                    abt1: Abteilung1::default(),
                    abt2: Abteilung2::default(),
                    abt3: Abteilung3::default(),
                },
                cache: GrundbuchAnalysiertCache::default(),
                nebenbeteiligte_dateipfade: Vec::new(),
                anpassungen_seite: BTreeMap::new(),
                previous_state: None,
                next_state: None,
            };
            pdf_parsed.speichern();
            data.loaded_files
                .insert(file_name.clone(), pdf_parsed.clone());
            if data.open_page.is_none() {
                data.open_page = Some((file_name.clone(), 2));
            }
            data.popover_state = None;
            let _ = std::fs::create_dir_all(Path::new(&Konfiguration::backup_dir()).join("backup"));
            let _ = std::fs::write(
                Path::new(&Konfiguration::backup_dir())
                    .join("backup")
                    .join("{file_name}.gbx"),
                "".as_bytes(),
            );
            let _ = webview.evaluate_script(&format!(
                "replaceEntireScreen(`{}`)",
                ui::render_entire_screen(data)
            ));
            let _ = webview.evaluate_script("startCheckingForPdfErrors()");
        }
        Cmd::Search { search_text } => {
            let passwort = match data.konfiguration.get_passwort() {
                Some(s) => s,
                None => return,
            };

            let server_url = &data.konfiguration.server_url;
            let server_email = urlencoding::encode(&data.konfiguration.server_email);
            let search_text = urlencoding::encode(&search_text);
            let passwort = urlencoding::encode(&passwort);
            let url = format!(
                "{server_url}/suche/{search_text}?email={server_email}&passwort={passwort}"
            );

            let client = reqwest::blocking::Client::new();
            let res = client
                .get(&url)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .send();

            let resp = match res {
                Ok(s) => s,
                Err(e) => {
                    let _ = webview.evaluate_script(&format!(
                        "replaceSuchergebnisse(`{}`)",
                        ui::render_suchergebnisse_liste(&GrundbuchSucheResponse::StatusErr(
                            GrundbuchSucheError {
                                code: 0,
                                text: format!("HTTP GET {url}: {}", e),
                            }
                        ))
                    ));
                    let _ =
                        std::fs::remove_file(std::env::temp_dir().join("dgb").join("passwort.txt"));
                    return;
                }
            };

            let json = match resp.json::<GrundbuchSucheResponse>() {
                Ok(s) => s,
                Err(e) => {
                    let _ = webview.evaluate_script(&format!(
                        "replaceSuchergebnisse(`{}`)",
                        ui::render_suchergebnisse_liste(&GrundbuchSucheResponse::StatusErr(
                            GrundbuchSucheError {
                                code: 0,
                                text: format!("HTTP GET {url}: {}", e),
                            }
                        ))
                    ));
                    let _ =
                        std::fs::remove_file(std::env::temp_dir().join("dgb").join("passwort.txt"));
                    return;
                }
            };

            let _ = webview.evaluate_script(&format!(
                "replaceSuchergebnisse(`{}`)",
                ui::render_suchergebnisse_liste(&json)
            ));
        }
        Cmd::GrundbuchAbonnieren { download_id } => {
            let passwort = match data.konfiguration.get_passwort() {
                Some(s) => s,
                None => return,
            };

            let server_url = &data.konfiguration.server_url;
            let server_email = urlencoding::encode(&data.konfiguration.server_email);
            let download_id = download_id;
            let server_url = &data.konfiguration.server_url;
            let server_email = urlencoding::encode(&data.konfiguration.server_email);
            let passwort = urlencoding::encode(&passwort);

            let tag = tinyfiledialogs::input_box(
                &format!("Aktenzeichen eingeben"),
                &format!("Bitte geben Sie ein (kurzes) Aktenzeichen für Ihr neues Abonnement ein:"),
                "",
            );

            let tag = match tag {
                Some(s) => s.trim().to_string(),
                None => return,
            };

            let tag = urlencoding::encode(&tag);
            let url = format!("{server_url}/abo-neu/email/{download_id}/{tag}?email={server_email}&passwort={passwort}");

            let client = reqwest::blocking::Client::new();
            let res = client
                .get(&url)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .send();

            let resp = match res {
                Ok(s) => s,
                Err(e) => {
                    tinyfiledialogs::message_box_ok(
                        "Fehler beim Abonnieren des Grundbuchs", 
                        &format!("Grundbuch konnte nicht abonniert werden: Anfrage an Server konnte nicht abgesendet werden: {e}"), 
                        MessageBoxIcon::Error
                    );
                    let _ =
                        std::fs::remove_file(std::env::temp_dir().join("dgb").join("passwort.txt"));
                    return;
                }
            };

            let json = match resp.json::<AboNeuAnfrage>() {
                Ok(s) => s,
                Err(e) => {
                    let e = format!("{e}").replace("\"", "").replace("'", "");
                    tinyfiledialogs::message_box_ok(
                        "Fehler beim Abonnieren des Grundbuchs", 
                        &format!("Grundbuch konnte nicht abonniert werden: Antwort von Server ist im falschen Format: {e}"), 
                        MessageBoxIcon::Error
                    );
                    let _ =
                        std::fs::remove_file(std::env::temp_dir().join("dgb").join("passwort.txt"));
                    return;
                }
            };

            match json {
                AboNeuAnfrage::Ok(_) => {
                    tinyfiledialogs::message_box_ok(
                        "Grundbuch wurde erfolgreich abonniert", 
                        &format!("Sie haben das Grundbuch {download_id} mit dem Aktenzeichen {tag} abonniert.\r\nIn Zukunft werden Sie bei Änderungen an diesem Grundbuch per E-Mail benachrichtigt werden."), 
                        MessageBoxIcon::Info
                    );
                }
                AboNeuAnfrage::Err(e) => {
                    let code = e.code;
                    let e = e.text.replace("\"", "").replace("'", "");
                    tinyfiledialogs::message_box_ok(
                        "Fehler beim Abonnieren des Grundbuchs", 
                        &format!("Grundbuch konnte nicht abonniert werden: Interner Serverfehler (E{code}: {e}"), 
                        MessageBoxIcon::Error
                    );
                    let _ =
                        std::fs::remove_file(std::env::temp_dir().join("dgb").join("passwort.txt"));
                    return;
                }
            }
        }
        Cmd::DownloadGbx { download_id } => {
            let file_dialog_result =
                tinyfiledialogs::select_folder_dialog(".gbx-Datei speichern unter...", "");

            let target_folder_path = match file_dialog_result {
                Some(f) => f,
                None => return,
            };

            let passwort = match data.konfiguration.get_passwort() {
                Some(s) => s,
                None => return,
            };

            let server_url = &data.konfiguration.server_url;
            let server_email = urlencoding::encode(&data.konfiguration.server_email);
            let download_id = download_id;
            let server_url = &data.konfiguration.server_url;
            let server_email = urlencoding::encode(&data.konfiguration.server_email);
            let passwort = urlencoding::encode(&passwort);
            let url = format!(
                "{server_url}/download/gbx/{download_id}?email={server_email}&passwort={passwort}"
            );

            let resp = match reqwest::blocking::get(&url) {
                Ok(s) => s,
                Err(e) => {
                    let _ =
                        std::fs::remove_file(std::env::temp_dir().join("dgb").join("passwort.txt"));
                    let fehler = format!("{e}").replace("\"", "").replace("'", "");
                    tinyfiledialogs::message_box_ok(
                        &format!("Fehler beim Herunterladen von {download_id}"), 
                        &format!("Datei {download_id} konnte nicht heruntergeladen werden:\r\nInterner Server-Fehler:\r\nHTTP GET {url}:\r\n{fehler}"), 
                        MessageBoxIcon::Error
                    );
                    return;
                }
            };

            let text = match resp.text() {
                Ok(s) => s,
                Err(e) => {
                    let _ =
                        std::fs::remove_file(std::env::temp_dir().join("dgb").join("passwort.txt"));
                    let fehler = format!("{e}").replace("\"", "").replace("'", "");
                    tinyfiledialogs::message_box_ok(
                        &format!("Fehler beim Herunterladen von {download_id}"), 
                        &format!("Datei {download_id} konnte nicht heruntergeladen werden:\r\nInterner Server-Fehler:\r\nHTTP GET {url}:\r\n{fehler}"), 
                        MessageBoxIcon::Error
                    );
                    return;
                }
            };

            let json = match serde_json::from_str::<PdfFileOrEmpty>(&text) {
                Ok(s) => s,
                Err(e) => {
                    let _ =
                        std::fs::remove_file(std::env::temp_dir().join("dgb").join("passwort.txt"));
                    let fehler = format!("{e}").replace("\"", "").replace("'", "");
                    tinyfiledialogs::message_box_ok(
                        &format!("Fehler beim Herunterladen von {download_id}"), 
                        &format!("Datei {download_id} konnte nicht heruntergeladen werden:\r\nServer-Antwort hat falsches Format:\r\nHTTP GET {url}:\r\n{fehler}"), 
                        MessageBoxIcon::Error
                    );
                    return;
                }
            };

            match json {
                PdfFileOrEmpty::Pdf(mut json) => {
                    let file_name = format!(
                        "{}_{}",
                        json.analysiert.titelblatt.grundbuch_von, json.analysiert.titelblatt.blatt
                    );
                    let backup_1 = Path::new(&Konfiguration::backup_dir()).join("backup");
                    let path = Path::new(&target_folder_path);
                    if json.gbx_datei_pfad.is_some() {
                        json.gbx_datei_pfad = Some(format!("{}", path.display()));
                    }
                    if json.datei.is_some() {
                        json.datei = Some(format!(
                            "{}",
                            path.join(&format!("{file_name}.pdf")).display()
                        ));
                    }
                    let _ = std::fs::write(
                        backup_1.join(&format!("{file_name}.gbx")),
                        serde_json::to_string_pretty(&json).unwrap_or_default(),
                    );
                    let _ = std::fs::write(
                        &format!("{target_folder_path}/{file_name}.gbx"),
                        serde_json::to_string_pretty(&json).unwrap_or_default(),
                    );
                    data.create_diff_save_point(&file_name, json.clone());
                    data.loaded_files.insert(file_name.clone(), json.clone());
                    data.open_page = Some((file_name.clone(), 2));
                    data.popover_state = None;
                    let _ = webview.evaluate_script(&format!(
                        "replaceEntireScreen(`{}`)",
                        ui::render_entire_screen(data)
                    ));
                    let _ = webview.evaluate_script("startCheckingForPdfErrors()");
                }
                PdfFileOrEmpty::NichtVorhanden(err) => {
                    tinyfiledialogs::message_box_ok(
                        &format!("Fehler beim Herunterladen von {download_id}.gbx"),
                        &format!(
                            "{download_id}.gbx konnte nicht heruntergeladen werden:\r\nE{}: {}",
                            err.code, err.text
                        ),
                        MessageBoxIcon::Error,
                    );
                }
            }
        }
        Cmd::UploadGbx => {
            let fingerprint = match data.konfiguration.get_private_key_fingerprint() {
                Ok(o) => o,
                Err(e) => {
                    tinyfiledialogs::message_box_ok(
                        "Kein gültiges Zertifikat", 
                        &format!("Zum Hochladen von Daten ist ein gültiges Schlüsselzertifikat notwendig.\r\nDas momentane Zertifikat ist ungültig oder existiert nicht (siehe Einstellungen / Konfigurations):\r\n{}", e), 
                        MessageBoxIcon::Error
                    );
                    return;
                }
            };

            let aenderungen = data.get_aenderungen();
            if aenderungen.ist_leer() {
                data.popover_state = None;
                let _ = webview.evaluate_script(&format!(
                    "replaceEntireScreen(`{}`)",
                    ui::render_entire_screen(data)
                ));
                return;
            }

            let mut d = UploadChangesetData {
                neu: aenderungen.neue_dateien.values().cloned().collect(),
                geaendert: aenderungen.geaenderte_dateien.values().cloned().collect(),
            };

            d.clear_personal_info();

            let patch = match d.format_patch() {
                Ok(o) => o,
                Err(e) => {
                    let e = e.replace("\"", "").replace("'", "");
                    tinyfiledialogs::message_box_ok(
                        "Konnte Patch nicht erstellen",
                        &format!("Konnte .patch-Datei nicht erstellen:\r\n{e}"),
                        MessageBoxIcon::Error,
                    );
                    return;
                }
            };

            let signatur = match data.konfiguration.sign_message(&patch) {
                Ok(o) => o,
                Err(e) => {
                    let e = e.replace("\"", "").replace("'", "");
                    tinyfiledialogs::message_box_ok(
                        "Konnte Patch nicht unterzeichnen", 
                        &format!("Konnte .patch-Datei nicht mit Schlüsselzertifikat unterschreiben:\r\n{e}"), 
                        MessageBoxIcon::Error
                    );
                    return;
                }
            };

            let passwort = match data.konfiguration.get_passwort() {
                Some(s) => s,
                None => return,
            };

            let server_url = &data.konfiguration.server_url;
            let server_email = urlencoding::encode(&data.konfiguration.server_email);
            let passwort = urlencoding::encode(&passwort);
            let url = format!("{server_url}/upload?email={server_email}&passwort={passwort}");

            let commit_msg = crate::pdf::hyphenate(&crate::pdf::unhyphenate(&data.commit_msg), 80);

            let data_changes = UploadChangeset {
                titel: data.commit_title.clone(),
                beschreibung: commit_msg,
                fingerprint,
                signatur: PgpSignatur {
                    hash: signatur.0,
                    pgp_signatur: signatur.1,
                },
                data: d,
            };

            let client = reqwest::blocking::Client::new();
            let res = match client.post(url.clone()).json(&data_changes).send() {
                Ok(o) => match o.json::<UploadChangesetResponse>() {
                    Ok(UploadChangesetResponse::StatusOk(o)) => {
                        data.reset_diff_backup_files(&o);
                        data.popover_state = None;
                        data.commit_title.clear();
                        data.commit_msg.clear();
                    }
                    Ok(UploadChangesetResponse::StatusError(e)) => {
                        let err = e.text.replace("\"", "").replace("'", "");
                        tinyfiledialogs::message_box_ok(
                            "Fehler beim Hochladen der Dateien",
                            &format!("E{}: {err}", e.code),
                            MessageBoxIcon::Error,
                        );
                        let _ = std::fs::remove_file(
                            std::env::temp_dir().join("dgb").join("passwort.txt"),
                        );
                    }
                    Err(e) => {
                        let e = format!("{e}").replace("\"", "").replace("'", "");
                        tinyfiledialogs::message_box_ok(
                            "Fehler beim Hochladen der Dateien",
                            &format!("Antwort vom Server ist nicht im richtigen Format:\r\n{}", e),
                            MessageBoxIcon::Error,
                        );
                        let _ = std::fs::remove_file(
                            std::env::temp_dir().join("dgb").join("passwort.txt"),
                        );
                    }
                },
                Err(e) => {
                    let e = format!("{e}").replace("\"", "").replace("'", "");
                    tinyfiledialogs::message_box_ok(
                        "Fehler beim Hochladen der Dateien",
                        &format!("HTTP POST {url}:\r\n{}", e),
                        MessageBoxIcon::Error,
                    );
                    let _ =
                        std::fs::remove_file(std::env::temp_dir().join("dgb").join("passwort.txt"));
                }
            };

            let _ = webview.evaluate_script(&format!(
                "replaceEntireScreen(`{}`)",
                ui::render_entire_screen(data)
            ));
        }
        Cmd::CheckPdfImageSichtbar => {
            match data.open_page.clone() {
                Some(_) => {}
                None => return,
            }

            let open_file = match data.open_page.clone() {
                Some(s) => s,
                None => {
                    return;
                }
            };

            let file = match data.loaded_files.get(&open_file.0) {
                Some(s) => s,
                None => {
                    return;
                }
            };

            let _ = webview.evaluate_script(&format!(
                "replacePdfImage(`{}`)",
                ui::render_pdf_image(data)
            ));
        }
        Cmd::CheckForPdfLoaded {
            file_path,
            file_name,
            pdf_path,
        } => {
            let default_parent = Path::new("/");
            let output_parent = Path::new(&file_path)
                .clone()
                .parent()
                .unwrap_or(&default_parent)
                .to_path_buf();
            let cache_output_path = output_parent
                .clone()
                .join(&format!("{}.cache.gbx", file_name));

            let mut pdf_parsed: PdfFile = match std::fs::read_to_string(&cache_output_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
            {
                Some(s) => s,
                None => {
                    let _ = webview
                        .evaluate_script(&format!("stopCheckingForPageLoaded(`{}`)", file_name));
                    return;
                }
            };

            pdf_parsed = reload_hocr_files(&pdf_parsed);
            if !pdf_parsed.ist_geladen() {
                crate::digital::insert_zeilen_automatisch(&mut pdf_parsed);
            }

            let _ = std::fs::write(
                &cache_output_path,
                serde_json::to_string_pretty(&pdf_parsed).unwrap_or_default(),
            );

            data.loaded_files
                .insert(file_name.clone(), pdf_parsed.clone());

            let _ = webview.evaluate_script(&format!(
                "replacePageList(`{}`);",
                ui::render_page_list(&data)
            ));

            if !pdf_parsed.ist_geladen() {
                return;
            }

            let _ = std::fs::remove_file(&cache_output_path);
            if data.open_page.is_none() {
                data.open_page = Some((file_name.clone(), 2));
                let _ = webview.evaluate_script(&format!(
                    "replaceEntireScreen(`{}`)",
                    ui::render_entire_screen(data)
                ));
            } else if data
                .open_page
                .as_ref()
                .map(|s| s.0.clone())
                .unwrap_or_default()
                == *file_name
            {
                let _ = webview.evaluate_script(&format!(
                    "replaceEntireScreen(`{}`)",
                    ui::render_entire_screen(data)
                ));
            }

            let _ = webview.evaluate_script(&format!("stopCheckingForPageLoaded(`{}`)", file_name));
        }
        Cmd::EditText { path, new_value } => {
            fn get_mut_or_insert_last<'a, T>(
                vec: &'a mut Vec<T>,
                index: usize,
                default_value: T,
            ) -> &'a mut T {
                let vec_len = vec.len();
                if index + 1 > vec_len {
                    vec.push(default_value);
                    let vec_len = vec.len();
                    &mut vec[vec_len - 1]
                } else {
                    &mut vec[index]
                }
            }

            use crate::digital::FlurstueckGroesse;

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

            let open_file = match data
                .open_page
                .clone()
                .and_then(|(file, _)| data.loaded_files.get_mut(&file))
            {
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
                        BvEintrag::neu(row + 1),
                    );
                    bv_eintrag.set_lfd_nr(new_value.clone().into());
                }
                ("bv", "bisherige-lfd-nr") => {
                    let new_value = match new_value.parse::<usize>().ok() {
                        Some(s) => Some(s),
                        None => None,
                    };
                    let mut bv_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.bestandsverzeichnis.eintraege,
                        row,
                        BvEintrag::neu(row + 1),
                    );
                    bv_eintrag.set_bisherige_lfd_nr(new_value.clone().into());
                }
                ("bv", "zu-nr") => {
                    let mut bv_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.bestandsverzeichnis.eintraege,
                        row,
                        BvEintrag::neu(row + 1),
                    );
                    bv_eintrag.set_zu_nr(new_value.clone().into());
                }
                ("bv", "recht-text") => {
                    let mut bv_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.bestandsverzeichnis.eintraege,
                        row,
                        BvEintrag::neu(row + 1),
                    );
                    bv_eintrag.set_recht_text(new_value.clone().into());
                }
                ("bv", "gemarkung") => {
                    let mut bv_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.bestandsverzeichnis.eintraege,
                        row,
                        BvEintrag::neu(row + 1),
                    );
                    bv_eintrag.set_gemarkung(if new_value.trim().is_empty() {
                        None
                    } else {
                        Some(new_value.clone().into())
                    });
                }
                ("bv", "flur") => {
                    let new_value = match new_value.parse::<usize>().ok() {
                        Some(s) => s,
                        None => return,
                    };
                    let mut bv_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.bestandsverzeichnis.eintraege,
                        row,
                        BvEintrag::neu(row + 1),
                    );
                    bv_eintrag.set_flur(new_value.clone().into());
                }
                ("bv", "flurstueck") => {
                    let mut bv_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.bestandsverzeichnis.eintraege,
                        row,
                        BvEintrag::neu(row + 1),
                    );
                    bv_eintrag.set_flurstueck(new_value.clone().into());
                }
                ("bv", "bezeichnung") => {
                    let mut bv_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.bestandsverzeichnis.eintraege,
                        row,
                        BvEintrag::neu(row + 1),
                    );
                    bv_eintrag.set_bezeichnung(new_value.clone().into());
                }
                ("bv", "groesse") => {
                    let new_value = match new_value.parse::<u64>().ok() {
                        Some(s) => Some(s),
                        None => None,
                    };
                    let mut bv_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.bestandsverzeichnis.eintraege,
                        row,
                        BvEintrag::neu(row + 1),
                    );
                    bv_eintrag.set_groesse(FlurstueckGroesse::Metrisch { m2: new_value });
                }
                ("bv-zuschreibung", "bv-nr") => {
                    let mut bv_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.bestandsverzeichnis.zuschreibungen,
                        row,
                        BvZuschreibung::default(),
                    );

                    bv_eintrag.bv_nr = new_value.clone().into();
                }
                ("bv-zuschreibung", "text") => {
                    let mut bv_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.bestandsverzeichnis.zuschreibungen,
                        row,
                        BvZuschreibung::default(),
                    );
                    bv_eintrag.text = new_value.clone().into();
                }
                ("bv-abschreibung", "bv-nr") => {
                    let mut bv_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.bestandsverzeichnis.abschreibungen,
                        row,
                        BvAbschreibung::default(),
                    );
                    bv_eintrag.bv_nr = new_value.clone().into();
                }
                ("bv-abschreibung", "text") => {
                    let mut bv_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.bestandsverzeichnis.abschreibungen,
                        row,
                        BvAbschreibung::default(),
                    );
                    bv_eintrag.text = new_value.clone().into();
                }

                ("abt1", "lfd-nr") => {
                    let new_value = match new_value.parse::<usize>().ok() {
                        Some(s) => s,
                        None => return,
                    };
                    let mut abt1_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt1.eintraege,
                        row,
                        Abt1Eintrag::new(row + 1),
                    );
                    abt1_eintrag.set_lfd_nr(new_value.clone().into());
                }
                ("abt1", "eigentuemer") => {
                    let mut abt1_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt1.eintraege,
                        row,
                        Abt1Eintrag::new(row + 1),
                    );
                    abt1_eintrag.set_eigentuemer(new_value.clone().into());
                }
                ("abt1-grundlage-eintragung", "bv-nr") => {
                    let mut abt1_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt1.grundlagen_eintragungen,
                        row,
                        Abt1GrundEintragung::new(),
                    );
                    abt1_eintrag.bv_nr = new_value.clone().into();
                }
                ("abt1-grundlage-eintragung", "text") => {
                    let mut abt1_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt1.grundlagen_eintragungen,
                        row,
                        Abt1GrundEintragung::new(),
                    );
                    abt1_eintrag.text = new_value.clone().into();
                }
                ("abt1-veraenderung", "lfd-nr") => {
                    let mut abt1_veraenderung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt1.veraenderungen,
                        row,
                        Abt1Veraenderung::default(),
                    );
                    abt1_veraenderung.lfd_nr = new_value.clone().into();
                }
                ("abt1-veraenderung", "text") => {
                    let mut abt1_veraenderung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt1.veraenderungen,
                        row,
                        Abt1Veraenderung::default(),
                    );
                    abt1_veraenderung.text = new_value.clone().into();
                }
                ("abt1-loeschung", "lfd-nr") => {
                    let mut abt1_loeschung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt1.loeschungen,
                        row,
                        Abt1Loeschung::default(),
                    );
                    abt1_loeschung.lfd_nr = new_value.clone().into();
                }
                ("abt1-loeschung", "text") => {
                    let mut abt1_loeschung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt1.loeschungen,
                        row,
                        Abt1Loeschung::default(),
                    );
                    abt1_loeschung.text = new_value.clone().into();
                }

                ("abt2", "lfd-nr") => {
                    let new_value = match new_value.parse::<usize>().ok() {
                        Some(s) => s,
                        None => return,
                    };
                    let mut abt2_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt2.eintraege,
                        row,
                        Abt2Eintrag::new(row + 1),
                    );
                    abt2_eintrag.lfd_nr = new_value.clone().into();
                }
                ("abt2", "bv-nr") => {
                    let mut abt2_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt2.eintraege,
                        row,
                        Abt2Eintrag::new(row + 1),
                    );
                    abt2_eintrag.bv_nr = new_value.clone().into();
                }
                ("abt2", "text") => {
                    let mut abt2_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt2.eintraege,
                        row,
                        Abt2Eintrag::new(row + 1),
                    );
                    abt2_eintrag.text = new_value.clone().into();
                }
                ("abt2-veraenderung", "lfd-nr") => {
                    let mut abt2_veraenderung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt2.veraenderungen,
                        row,
                        Abt2Veraenderung::default(),
                    );
                    abt2_veraenderung.lfd_nr = new_value.clone().into();
                }
                ("abt2-veraenderung", "text") => {
                    let mut abt2_veraenderung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt2.veraenderungen,
                        row,
                        Abt2Veraenderung::default(),
                    );
                    abt2_veraenderung.text = new_value.clone().into();
                }
                ("abt2-loeschung", "lfd-nr") => {
                    let mut abt2_loeschung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt2.loeschungen,
                        row,
                        Abt2Loeschung::default(),
                    );
                    abt2_loeschung.lfd_nr = new_value.clone().into();
                }
                ("abt2-loeschung", "text") => {
                    let mut abt2_loeschung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt2.loeschungen,
                        row,
                        Abt2Loeschung::default(),
                    );
                    abt2_loeschung.text = new_value.clone().into();
                }

                ("abt3", "lfd-nr") => {
                    let new_value = match new_value.parse::<usize>().ok() {
                        Some(s) => s,
                        None => return,
                    };
                    let mut abt3_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt3.eintraege,
                        row,
                        Abt3Eintrag::new(row + 1),
                    );
                    abt3_eintrag.lfd_nr = new_value.clone();
                }
                ("abt3", "bv-nr") => {
                    let mut abt3_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt3.eintraege,
                        row,
                        Abt3Eintrag::new(row + 1),
                    );
                    abt3_eintrag.bv_nr = new_value.clone().into();
                }
                ("abt3", "betrag") => {
                    let mut abt3_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt3.eintraege,
                        row,
                        Abt3Eintrag::new(row + 1),
                    );
                    abt3_eintrag.betrag = new_value.clone().into();
                }
                ("abt3", "text") => {
                    let mut abt3_eintrag = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt3.eintraege,
                        row,
                        Abt3Eintrag::new(row + 1),
                    );
                    abt3_eintrag.text = new_value.clone().into();
                }
                ("abt3-veraenderung", "lfd-nr") => {
                    let mut abt3_veraenderung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt3.veraenderungen,
                        row,
                        Abt3Veraenderung::default(),
                    );
                    abt3_veraenderung.lfd_nr = new_value.clone().into();
                }
                ("abt3-veraenderung", "betrag") => {
                    let mut abt3_veraenderung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt3.veraenderungen,
                        row,
                        Abt3Veraenderung::default(),
                    );
                    abt3_veraenderung.betrag = new_value.clone().into();
                }
                ("abt3-veraenderung", "text") => {
                    let mut abt3_veraenderung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt3.veraenderungen,
                        row,
                        Abt3Veraenderung::default(),
                    );
                    abt3_veraenderung.text = new_value.clone().into();
                }
                ("abt3-loeschung", "lfd-nr") => {
                    let mut abt3_loeschung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt3.loeschungen,
                        row,
                        Abt3Loeschung::default(),
                    );
                    abt3_loeschung.lfd_nr = new_value.clone().into();
                }
                ("abt3-loeschung", "betrag") => {
                    let mut abt3_loeschung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt3.loeschungen,
                        row,
                        Abt3Loeschung::default(),
                    );
                    abt3_loeschung.betrag = new_value.clone().into();
                }
                ("abt3-loeschung", "text") => {
                    let mut abt3_loeschung = get_mut_or_insert_last(
                        &mut open_file.analysiert.abt3.loeschungen,
                        row,
                        Abt3Loeschung::default(),
                    );
                    abt3_loeschung.text = new_value.clone().into();
                }

                _ => {
                    return;
                }
            }

            open_file.speichern();
            let _ = webview.evaluate_script("saveState();");
            open_file.icon = None;
            if data.konfiguration.lefis_analyse_einblenden {
                let analyse = open_file.cache.start_analyzing(
                    &open_file.analysiert,
                    &data.vm,
                    &data.loaded_nb,
                    &data.konfiguration,
                );
                let _ = webview.evaluate_script(&format!(
                    "replaceAnalyseGrundbuch(`{}`);",
                    ui::render_analyse_grundbuch(&analyse, false, false)
                ));
            }
        }
        Cmd::BvEintragTypAendern { path, value } => {
            use crate::digital::{BvEintragFlurstueck, BvEintragRecht};

            let open_file = match data
                .open_page
                .clone()
                .and_then(|(file, _)| data.loaded_files.get_mut(&file))
            {
                Some(s) => s,
                None => return,
            };

            let split = path.split(":").collect::<Vec<_>>();

            match split.get(0) {
                Some(s) => {
                    if *s != "bv" {
                        return;
                    } else {
                    }
                }
                None => return,
            };

            let row = match split.get(1).and_then(|s| s.parse::<usize>().ok()) {
                Some(s) => s,
                None => return,
            };

            match value.as_str() {
                "flst" => {
                    if let Some(BvEintrag::Recht(BvEintragRecht {
                        lfd_nr,
                        bisherige_lfd_nr,
                        ..
                    })) = open_file
                        .analysiert
                        .bestandsverzeichnis
                        .eintraege
                        .get(row)
                        .cloned()
                    {
                        open_file.analysiert.bestandsverzeichnis.eintraege[row] =
                            BvEintrag::Flurstueck(BvEintragFlurstueck {
                                lfd_nr,
                                bisherige_lfd_nr,
                                ..BvEintragFlurstueck::neu(0)
                            });
                    }
                }
                "recht" => {
                    if let Some(BvEintrag::Flurstueck(BvEintragFlurstueck {
                        lfd_nr,
                        bisherige_lfd_nr,
                        ..
                    })) = open_file
                        .analysiert
                        .bestandsverzeichnis
                        .eintraege
                        .get(row)
                        .cloned()
                    {
                        open_file.analysiert.bestandsverzeichnis.eintraege[row] =
                            BvEintrag::Recht(BvEintragRecht {
                                lfd_nr,
                                bisherige_lfd_nr,
                                ..BvEintragRecht::neu(0)
                            });
                    }
                }
                _ => {
                    return;
                }
            }

            // speichern
            open_file.speichern();
            let _ = webview.evaluate_script("saveState();");

            // let _ = webview.evaluate_script(&format!("replaceMainContainer(`{}`);", ui::render_main_container(data)));
            let _ = webview.evaluate_script(&format!(
                "replaceBestandsverzeichnis(`{}`);",
                ui::render_bestandsverzeichnis(open_file, &data.konfiguration)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceBestandsverzeichnisZuschreibungen(`{}`);",
                ui::render_bestandsverzeichnis_zuschreibungen(open_file)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceBestandsverzeichnisAbschreibungen(`{}`);",
                ui::render_bestandsverzeichnis_abschreibungen(open_file)
            ));
            let _ = webview
                .evaluate_script(&format!("replaceAbt1(`{}`);", ui::render_abt_1(open_file)));
            let _ = webview.evaluate_script(&format!(
                "replaceAbt1GrundlagenEintragungen(`{}`);",
                ui::render_abt_1_grundlagen_eintragungen(open_file)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceAbt1Veraenderungen(`{}`);",
                ui::render_abt_1_veraenderungen(open_file)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceAbt1Loeschungen(`{}`);",
                ui::render_abt_1_loeschungen(open_file)
            ));
            let _ = webview
                .evaluate_script(&format!("replaceAbt2(`{}`);", ui::render_abt_2(open_file)));
            let _ = webview.evaluate_script(&format!(
                "replaceAbt2Veraenderungen(`{}`);",
                ui::render_abt_2_veraenderungen(open_file)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceAbt2Loeschungen(`{}`);",
                ui::render_abt_2_loeschungen(open_file)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceAbt3(`{}`);",
                ui::render_abt_3(open_file, data.konfiguration.lefis_analyse_einblenden)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceAbt3Veraenderungen(`{}`);",
                ui::render_abt_3_veraenderungen(open_file)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceAbt3Loeschungen(`{}`);",
                ui::render_abt_3_loeschungen(open_file)
            ));
            let analyse = open_file.cache.start_analyzing(
                &open_file.analysiert,
                &data.vm,
                &data.loaded_nb,
                &data.konfiguration,
            );
            let _ = webview.evaluate_script(&format!(
                "replaceAnalyseGrundbuch(`{}`);",
                ui::render_analyse_grundbuch(&analyse, false, false)
            ));
            let _ = webview.evaluate_script(&format!(
                "replacePageList(`{}`);",
                ui::render_page_list(data)
            ));
        }
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

            let open_file = match data
                .open_page
                .clone()
                .and_then(|(file, _)| data.loaded_files.get_mut(&file))
            {
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
                "bv" => insert_after(
                    &mut open_file.analysiert.bestandsverzeichnis.eintraege,
                    row,
                    BvEintrag::neu(row + 2),
                ),
                "bv-zuschreibung" => insert_after(
                    &mut open_file.analysiert.bestandsverzeichnis.zuschreibungen,
                    row,
                    BvZuschreibung::default(),
                ),
                "bv-abschreibung" => insert_after(
                    &mut open_file.analysiert.bestandsverzeichnis.abschreibungen,
                    row,
                    BvAbschreibung::default(),
                ),

                "abt1" => insert_after(
                    &mut open_file.analysiert.abt1.eintraege,
                    row,
                    Abt1Eintrag::new(row + 2),
                ),
                "abt1-grundlage-eintragung" => insert_after(
                    &mut open_file.analysiert.abt1.grundlagen_eintragungen,
                    row,
                    Abt1GrundEintragung::default(),
                ),
                "abt1-veraenderung" => insert_after(
                    &mut open_file.analysiert.abt1.veraenderungen,
                    row,
                    Abt1Veraenderung::default(),
                ),
                "abt1-loeschung" => insert_after(
                    &mut open_file.analysiert.abt1.loeschungen,
                    row,
                    Abt1Loeschung::default(),
                ),

                "abt2" => insert_after(
                    &mut open_file.analysiert.abt2.eintraege,
                    row,
                    Abt2Eintrag::new(row + 2),
                ),
                "abt2-veraenderung" => insert_after(
                    &mut open_file.analysiert.abt2.veraenderungen,
                    row,
                    Abt2Veraenderung::default(),
                ),
                "abt2-loeschung" => insert_after(
                    &mut open_file.analysiert.abt2.loeschungen,
                    row,
                    Abt2Loeschung::default(),
                ),

                "abt3" => insert_after(
                    &mut open_file.analysiert.abt3.eintraege,
                    row,
                    Abt3Eintrag::new(row + 2),
                ),
                "abt3-veraenderung" => insert_after(
                    &mut open_file.analysiert.abt3.veraenderungen,
                    row,
                    Abt3Veraenderung::default(),
                ),
                "abt3-loeschung" => insert_after(
                    &mut open_file.analysiert.abt3.loeschungen,
                    row,
                    Abt3Loeschung::default(),
                ),
                _ => return,
            }

            let next_focus = match *section {
                "bv" => format!("bv_{}_lfd-nr", row + 1),
                "bv-zuschreibung" => format!("bv-zuschreibung_{}_bv-nr", row + 1),
                "bv-abschreibung" => format!("bv-abschreibung_{}_bv-nr", row + 1),

                "abt1" => format!("abt1_{}_lfd-nr", row + 1),
                "abt1-grundlage-eintragung" => {
                    format!("abt1-grundlage-eintragung_{}_bv-nr", row + 1)
                }
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
            let _ = webview.evaluate_script("saveState();");

            // let _ = webview.evaluate_script(&format!("replaceMainContainer(`{}`);", ui::render_main_container(data)));
            let _ = webview.evaluate_script(&format!(
                "replaceBestandsverzeichnis(`{}`);",
                ui::render_bestandsverzeichnis(open_file, &data.konfiguration)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceBestandsverzeichnisZuschreibungen(`{}`);",
                ui::render_bestandsverzeichnis_zuschreibungen(open_file)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceBestandsverzeichnisAbschreibungen(`{}`);",
                ui::render_bestandsverzeichnis_abschreibungen(open_file)
            ));
            let _ = webview
                .evaluate_script(&format!("replaceAbt1(`{}`);", ui::render_abt_1(open_file)));
            let _ = webview.evaluate_script(&format!(
                "replaceAbt1GrundlagenEintragungen(`{}`);",
                ui::render_abt_1_grundlagen_eintragungen(open_file)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceAbt1Veraenderungen(`{}`);",
                ui::render_abt_1_veraenderungen(open_file)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceAbt1Loeschungen(`{}`);",
                ui::render_abt_1_loeschungen(open_file)
            ));
            let _ = webview
                .evaluate_script(&format!("replaceAbt2(`{}`);", ui::render_abt_2(open_file)));
            let _ = webview.evaluate_script(&format!(
                "replaceAbt2Veraenderungen(`{}`);",
                ui::render_abt_2_veraenderungen(open_file)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceAbt2Loeschungen(`{}`);",
                ui::render_abt_2_loeschungen(open_file)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceAbt3(`{}`);",
                ui::render_abt_3(open_file, data.konfiguration.lefis_analyse_einblenden)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceAbt3Veraenderungen(`{}`);",
                ui::render_abt_3_veraenderungen(open_file)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceAbt3Loeschungen(`{}`);",
                ui::render_abt_3_loeschungen(open_file)
            ));
            let analyse = open_file.cache.start_analyzing(
                &open_file.analysiert,
                &data.vm,
                &data.loaded_nb,
                &data.konfiguration,
            );
            let _ = webview.evaluate_script(&format!(
                "replaceAnalyseGrundbuch(`{}`);",
                ui::render_analyse_grundbuch(&analyse, false, false)
            ));
            let _ = webview.evaluate_script(&format!(
                "replacePageList(`{}`);",
                ui::render_page_list(data)
            ));

            let _ = webview.evaluate_script(&format!(
                "document.getElementById(`{}`).focus();",
                next_focus
            ));
        }
        Cmd::EintragLoeschen { path } | Cmd::EintragRoeten { path } => {
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

            let open_file = match data
                .open_page
                .clone()
                .and_then(|(file, _)| data.loaded_files.get_mut(&file))
            {
                Some(s) => s,
                None => return,
            };

            match (*section, eintrag_roeten) {
                ("bv", false) => {
                    if !open_file
                        .analysiert
                        .bestandsverzeichnis
                        .eintraege
                        .is_empty()
                    {
                        open_file
                            .analysiert
                            .bestandsverzeichnis
                            .eintraege
                            .remove(row);
                    }
                }
                ("bv", true) => {
                    open_file
                        .analysiert
                        .bestandsverzeichnis
                        .eintraege
                        .get_mut(row)
                        .map(|e| {
                            let cur = *e.get_manuell_geroetet().get_or_insert_with(|| {
                                e.get_automatisch_geroetet().unwrap_or(false)
                            });
                            e.set_manuell_geroetet(Some(!cur));
                        });
                }

                ("bv-zuschreibung", false) => {
                    if !open_file
                        .analysiert
                        .bestandsverzeichnis
                        .zuschreibungen
                        .is_empty()
                    {
                        open_file
                            .analysiert
                            .bestandsverzeichnis
                            .zuschreibungen
                            .remove(row);
                    }
                }
                ("bv-zuschreibung", true) => {
                    open_file
                        .analysiert
                        .bestandsverzeichnis
                        .zuschreibungen
                        .get_mut(row)
                        .map(|e| {
                            let cur = *e
                                .manuell_geroetet
                                .get_or_insert_with(|| e.automatisch_geroetet.unwrap_or(false));
                            e.manuell_geroetet = Some(!cur);
                        });
                }

                ("bv-abschreibung", false) => {
                    if !open_file
                        .analysiert
                        .bestandsverzeichnis
                        .abschreibungen
                        .is_empty()
                    {
                        open_file
                            .analysiert
                            .bestandsverzeichnis
                            .abschreibungen
                            .remove(row);
                    }
                }
                ("bv-abschreibung", true) => {
                    open_file
                        .analysiert
                        .bestandsverzeichnis
                        .abschreibungen
                        .get_mut(row)
                        .map(|e| {
                            let cur = *e
                                .manuell_geroetet
                                .get_or_insert_with(|| e.automatisch_geroetet.unwrap_or(false));
                            e.manuell_geroetet = Some(!cur);
                        });
                }

                ("abt1", false) => {
                    if !open_file.analysiert.abt1.eintraege.is_empty() {
                        open_file.analysiert.abt1.eintraege.remove(row);
                    }
                }
                ("abt1", true) => {
                    open_file.analysiert.abt1.eintraege.get_mut(row).map(|e| {
                        let cur = *e
                            .get_manuell_geroetet()
                            .get_or_insert_with(|| e.get_automatisch_geroetet());
                        e.set_manuell_geroetet(Some(!cur));
                    });
                }

                ("abt1-grundlage-eintragung", false) => {
                    if !open_file.analysiert.abt1.grundlagen_eintragungen.is_empty() {
                        open_file
                            .analysiert
                            .abt1
                            .grundlagen_eintragungen
                            .remove(row);
                    }
                }
                ("abt1-grundlage-eintragung", true) => {
                    open_file
                        .analysiert
                        .abt1
                        .grundlagen_eintragungen
                        .get_mut(row)
                        .map(|e| {
                            let cur = *e
                                .manuell_geroetet
                                .get_or_insert_with(|| e.automatisch_geroetet.unwrap_or(false));
                            e.manuell_geroetet = Some(!cur);
                        });
                }

                ("abt1-veraenderung", false) => {
                    if !open_file.analysiert.abt1.veraenderungen.is_empty() {
                        open_file.analysiert.abt1.veraenderungen.remove(row);
                    }
                }
                ("abt1-veraenderung", true) => {
                    open_file
                        .analysiert
                        .abt1
                        .veraenderungen
                        .get_mut(row)
                        .map(|e| {
                            let cur = *e
                                .manuell_geroetet
                                .get_or_insert_with(|| e.automatisch_geroetet.unwrap_or(false));
                            e.manuell_geroetet = Some(!cur);
                        });
                }

                ("abt1-loeschung", false) => {
                    if !open_file.analysiert.abt1.loeschungen.is_empty() {
                        open_file.analysiert.abt1.loeschungen.remove(row);
                    }
                }
                ("abt1-loeschung", true) => {
                    open_file.analysiert.abt1.loeschungen.get_mut(row).map(|e| {
                        let cur = *e
                            .manuell_geroetet
                            .get_or_insert_with(|| e.automatisch_geroetet.unwrap_or(false));
                        e.manuell_geroetet = Some(!cur);
                    });
                }

                ("abt2", false) => {
                    if !open_file.analysiert.abt2.eintraege.is_empty() {
                        open_file.analysiert.abt2.eintraege.remove(row);
                    }
                }
                ("abt2", true) => {
                    open_file.analysiert.abt2.eintraege.get_mut(row).map(|e| {
                        let cur = *e
                            .manuell_geroetet
                            .get_or_insert_with(|| e.automatisch_geroetet.unwrap_or(false));
                        e.manuell_geroetet = Some(!cur);
                    });
                }

                ("abt2-veraenderung", false) => {
                    if !open_file.analysiert.abt2.veraenderungen.is_empty() {
                        open_file.analysiert.abt2.veraenderungen.remove(row);
                    }
                }
                ("abt2-veraenderung", true) => {
                    open_file
                        .analysiert
                        .abt2
                        .veraenderungen
                        .get_mut(row)
                        .map(|e| {
                            let cur = *e
                                .manuell_geroetet
                                .get_or_insert_with(|| e.automatisch_geroetet.unwrap_or(false));
                            e.manuell_geroetet = Some(!cur);
                        });
                }

                ("abt2-loeschung", false) => {
                    if !open_file.analysiert.abt2.loeschungen.is_empty() {
                        open_file.analysiert.abt2.loeschungen.remove(row);
                    }
                }
                ("abt2-loeschung", true) => {
                    open_file.analysiert.abt2.loeschungen.get_mut(row).map(|e| {
                        let cur = *e
                            .manuell_geroetet
                            .get_or_insert_with(|| e.automatisch_geroetet.unwrap_or(false));
                        e.manuell_geroetet = Some(!cur);
                    });
                }

                ("abt3", false) => {
                    if !open_file.analysiert.abt3.eintraege.is_empty() {
                        open_file.analysiert.abt3.eintraege.remove(row);
                    }
                }
                ("abt3", true) => {
                    open_file.analysiert.abt3.eintraege.get_mut(row).map(|e| {
                        let cur = *e
                            .manuell_geroetet
                            .get_or_insert_with(|| e.automatisch_geroetet.unwrap_or(false));
                        e.manuell_geroetet = Some(!cur);
                    });
                }

                ("abt3-veraenderung", false) => {
                    if !open_file.analysiert.abt3.veraenderungen.is_empty() {
                        open_file.analysiert.abt3.veraenderungen.remove(row);
                    }
                }
                ("abt3-veraenderung", true) => {
                    open_file
                        .analysiert
                        .abt3
                        .veraenderungen
                        .get_mut(row)
                        .map(|e| {
                            let cur = *e
                                .manuell_geroetet
                                .get_or_insert_with(|| e.automatisch_geroetet.unwrap_or(false));
                            e.manuell_geroetet = Some(!cur);
                        });
                }

                ("abt3-loeschung", false) => {
                    if !open_file.analysiert.abt3.loeschungen.is_empty() {
                        open_file.analysiert.abt3.loeschungen.remove(row);
                    }
                }
                ("abt3-loeschung", true) => {
                    open_file.analysiert.abt3.loeschungen.get_mut(row).map(|e| {
                        let cur = *e
                            .manuell_geroetet
                            .get_or_insert_with(|| e.automatisch_geroetet.unwrap_or(false));
                        e.manuell_geroetet = Some(!cur);
                    });
                }

                _ => return,
            }

            let next_focus = match *section {
                "bv" => format!(
                    "bv_{}_lfd-nr",
                    if eintrag_roeten {
                        row + 1
                    } else {
                        row.saturating_sub(1)
                    }
                ),
                "bv-zuschreibung" => format!(
                    "bv-zuschreibung_{}_bv-nr",
                    if eintrag_roeten {
                        row + 1
                    } else {
                        row.saturating_sub(1)
                    }
                ),
                "bv-abschreibung" => format!(
                    "bv-abschreibung_{}_bv-nr",
                    if eintrag_roeten {
                        row + 1
                    } else {
                        row.saturating_sub(1)
                    }
                ),

                "abt1" => format!(
                    "abt1_{}_lfd-nr",
                    if eintrag_roeten {
                        row + 1
                    } else {
                        row.saturating_sub(1)
                    }
                ),
                "abt1-grundlage-eintragung" => format!(
                    "abt1-grundlage-eintragung_{}_bv-nr",
                    if eintrag_roeten {
                        row + 1
                    } else {
                        row.saturating_sub(1)
                    }
                ),
                "abt1-veraenderung" => format!(
                    "abt1-veraenderung_{}_lfd-nr",
                    if eintrag_roeten {
                        row + 1
                    } else {
                        row.saturating_sub(1)
                    }
                ),
                "abt1-loeschung" => format!(
                    "abt1-loeschung_{}_lfd-nr",
                    if eintrag_roeten {
                        row + 1
                    } else {
                        row.saturating_sub(1)
                    }
                ),

                "abt2" => format!(
                    "abt2_{}_lfd-nr",
                    if eintrag_roeten {
                        row + 1
                    } else {
                        row.saturating_sub(1)
                    }
                ),
                "abt2-veraenderung" => format!(
                    "abt2-veraenderung_{}_lfd-nr",
                    if eintrag_roeten {
                        row + 1
                    } else {
                        row.saturating_sub(1)
                    }
                ),
                "abt2-loeschung" => format!(
                    "abt2-loeschung_{}_lfd-nr",
                    if eintrag_roeten {
                        row + 1
                    } else {
                        row.saturating_sub(1)
                    }
                ),

                "abt3" => format!(
                    "abt3_{}_lfd-nr",
                    if eintrag_roeten {
                        row + 1
                    } else {
                        row.saturating_sub(1)
                    }
                ),
                "abt3-veraenderung" => format!(
                    "abt3-veraenderung_{}_lfd-nr",
                    if eintrag_roeten {
                        row + 1
                    } else {
                        row.saturating_sub(1)
                    }
                ),
                "abt3-loeschung" => format!(
                    "abt3-loeschung_{}_lfd-nr",
                    if eintrag_roeten {
                        row + 1
                    } else {
                        row.saturating_sub(1)
                    }
                ),

                _ => return,
            };

            // speichern
            open_file.speichern();
            let _ = webview.evaluate_script("saveState();");

            // let _ = webview.evaluate_script(&format!("replaceMainContainer(`{}`);", ui::render_main_container(data)));
            let _ = webview.evaluate_script(&format!(
                "replaceBestandsverzeichnis(`{}`);",
                ui::render_bestandsverzeichnis(open_file, &data.konfiguration)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceBestandsverzeichnisZuschreibungen(`{}`);",
                ui::render_bestandsverzeichnis_zuschreibungen(open_file)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceBestandsverzeichnisAbschreibungen(`{}`);",
                ui::render_bestandsverzeichnis_abschreibungen(open_file)
            ));
            let _ = webview
                .evaluate_script(&format!("replaceAbt1(`{}`);", ui::render_abt_1(open_file)));
            let _ = webview.evaluate_script(&format!(
                "replaceAbt1GrundlagenEintragungen(`{}`);",
                ui::render_abt_1_grundlagen_eintragungen(open_file)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceAbt1Veraenderungen(`{}`);",
                ui::render_abt_1_veraenderungen(open_file)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceAbt1Loeschungen(`{}`);",
                ui::render_abt_1_loeschungen(open_file)
            ));
            let _ = webview
                .evaluate_script(&format!("replaceAbt2(`{}`);", ui::render_abt_2(open_file)));
            let _ = webview.evaluate_script(&format!(
                "replaceAbt2Veraenderungen(`{}`);",
                ui::render_abt_2_veraenderungen(open_file)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceAbt2Loeschungen(`{}`);",
                ui::render_abt_2_loeschungen(open_file)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceAbt3(`{}`);",
                ui::render_abt_3(open_file, data.konfiguration.lefis_analyse_einblenden)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceAbt3Veraenderungen(`{}`);",
                ui::render_abt_3_veraenderungen(open_file)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceAbt3Loeschungen(`{}`);",
                ui::render_abt_3_loeschungen(open_file)
            ));
            let analyse = open_file.cache.start_analyzing(
                &open_file.analysiert,
                &data.vm,
                &data.loaded_nb,
                &data.konfiguration,
            );
            let _ = webview.evaluate_script(&format!(
                "replaceAnalyseGrundbuch(`{}`);",
                ui::render_analyse_grundbuch(&analyse, false, false)
            ));
            let _ = webview.evaluate_script(&format!(
                "replacePageList(`{}`);",
                ui::render_page_list(data)
            ));

            let _ = webview.evaluate_script(&format!(
                "(function() {{ 
                let element = document.getElementById(`{}`); 
                if (element) {{ element.focus(); }};
            }})();",
                next_focus
            ));
        }
        Cmd::EditCommitTitle { value } => {
            data.commit_title = value.trim().to_string();
        }
        Cmd::EditCommitDescription { value } => {
            data.commit_msg = value.clone();
        }
        Cmd::EditKonfigurationTextField { id, value } => {
            match id.as_str() {
                "server-url" => {
                    data.konfiguration.server_url = value.trim().to_string();
                }
                "email" => {
                    data.konfiguration.server_email = value.trim().to_string();
                }
                _ => {
                    return;
                }
            }

            data.konfiguration.speichern();
        }
        Cmd::EditKonfigurationSchluesseldatei { base64 } => {
            data.konfiguration.server_privater_schluessel_base64 = Some(base64::encode(base64));
            data.konfiguration.speichern();
        }
        Cmd::SwitchAenderungView { i } => {
            data.popover_state = Some(PopoverState::GrundbuchUploadDialog(*i));
            let aenderungen = data.get_aenderungen();
            let _ = webview.evaluate_script(&format!(
                "replaceAenderungDateien(`{}`)",
                ui::render_aenderungen_dateien(&aenderungen, *i)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceAenderungDiff(`{}`)",
                ui::render_aenderung_diff(&aenderungen, *i)
            ));
        }
        Cmd::OpenContextMenu { x, y, seite } => {
            data.popover_state = Some(PopoverState::ContextMenu(ContextMenuData {
                x: *x,
                y: *y,
                seite_ausgewaehlt: *seite,
            }));
            let _ = webview.evaluate_script(&format!(
                "replacePopOver(`{}`)",
                ui::render_popover_content(data)
            ));
        }
        Cmd::OpenConfiguration => {
            data.popover_state = Some(PopoverState::Configuration(ConfigurationView::Allgemein));
            let _ = webview.evaluate_script(&format!(
                "replacePopOver(`{}`)",
                ui::render_popover_content(data)
            ));
        }
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
                "klassifizierung-schuldenart-abt3" => {
                    ConfigurationView::KlassifizierungSchuldenArtAbt3
                }
                "rechtsinhaber-auslesen-abt3" => ConfigurationView::RechtsinhaberAuslesenAbt3,
                "text-kuerzen-abt3" => ConfigurationView::TextKuerzenAbt3,
                _ => {
                    return;
                }
            }));
            let _ = webview.evaluate_script(&format!(
                "replacePopOver(`{}`)",
                ui::render_popover_content(data)
            ));
        }
        Cmd::OpenInfo => {
            data.popover_state = Some(PopoverState::Info);
            let _ = webview.evaluate_script(&format!(
                "replacePopOver(`{}`)",
                ui::render_popover_content(data)
            ));
        }
        Cmd::OpenHelp => {
            data.popover_state = Some(PopoverState::Help);
            let _ = webview.evaluate_script(&format!(
                "replacePopOver(`{}`)",
                ui::render_popover_content(data)
            ));
        }
        Cmd::OpenExportPdf => {
            if data.loaded_files.is_empty() {
                return;
            }
            data.popover_state = Some(PopoverState::ExportPdf);
            let _ = webview.evaluate_script(&format!(
                "replacePopOver(`{}`)",
                ui::render_popover_content(data)
            ));
        }
        Cmd::CloseFile { file_name } => {
            let _ = data.loaded_files.remove(file_name);
            data.popover_state = None;
            let _ = webview.evaluate_script(&format!("stopCheckingForPageLoaded(`{}`)", file_name));
            let _ = webview.evaluate_script(&format!(
                "replaceEntireScreen(`{}`)",
                ui::render_entire_screen(data)
            ));
        }
        Cmd::CheckPdfForErrors => {
            let mut new_icons = BTreeMap::new();
            let mut icon_count = 0;

            for (k, v) in data.loaded_files.iter() {
                if icon_count >= 1 {
                    break;
                }

                if v.ist_geladen() && v.icon.is_none() {
                    let icon =
                        match v.get_icon(data.vm.clone(), &data.loaded_nb, &data.konfiguration) {
                            Some(s) => s,
                            None => {
                                return;
                            }
                        };
                    new_icons.insert(k.clone(), icon);
                    icon_count += 1;
                }
            }

            for (k, v) in new_icons {
                if let Some(s) = data.loaded_files.get_mut(&k.clone()) {
                    s.icon = Some(v);
                    let konfiguration = data.konfiguration.clone();
                    let titelblatt = s.analysiert.titelblatt.clone();
                    let _ = webview.evaluate_script(&format!(
                        "replaceIcon(`{}`, `{}`)",
                        k,
                        v.get_base64()
                    ));
                }
            }
        }
        Cmd::ToggleLefisAnalyse => {
            data.konfiguration.lefis_analyse_einblenden =
                !data.konfiguration.lefis_analyse_einblenden;
            let _ = webview.evaluate_script(&format!(
                "replaceMainContainer(`{}`);",
                ui::render_main_container(data)
            ));
        }
        Cmd::SelectTab { tab } => {
            data.konfiguration.tab = *tab;
            let _ =
                webview.evaluate_script(&format!("replaceRibbon(`{}`);", ui::render_ribbon(&data)));
        }
        Cmd::ToggleDateiliste { toggle } => {
            data.konfiguration.dateiliste_ausblenden = *toggle;
            let _ = webview.evaluate_script(&format!(
                "replacePageList(`{}`);",
                ui::render_page_list(&data)
            ));
        }
        Cmd::EditTextKuerzenAbt2Script { script } => {
            data.konfiguration.text_kuerzen_abt2_script =
                script.lines().map(|l| l.replace("\u{00a0}", " ")).collect();
            data.konfiguration.speichern();
        }
        Cmd::EditTextKuerzenAbt3Script { script } => {
            data.konfiguration.text_kuerzen_abt3_script =
                script.lines().map(|l| l.replace("\u{00a0}", " ")).collect();
            data.konfiguration.speichern();
        }
        Cmd::EditAbkuerzungenScript { script } => {
            data.konfiguration.abkuerzungen_script =
                script.lines().map(|l| l.replace("\u{00a0}", " ")).collect();
            data.konfiguration.speichern();
        }
        Cmd::EditTextSaubernScript { script } => {
            data.konfiguration.text_saubern_script =
                script.lines().map(|l| l.replace("\u{00a0}", " ")).collect();
            data.konfiguration.speichern();
        }
        Cmd::EditFlurstueckeAuslesenScript { script } => {
            data.konfiguration.flurstuecke_auslesen_script =
                script.lines().map(|l| l.replace("\u{00a0}", " ")).collect();
            data.konfiguration.speichern();
        }
        Cmd::EditRechteArtScript { neu } => {
            data.konfiguration.klassifiziere_rechteart =
                neu.lines().map(|l| l.replace("\u{00a0}", " ")).collect();
            data.konfiguration.speichern();
        }
        Cmd::EditRangvermerkAuslesenAbt2Script { neu } => {
            data.konfiguration.rangvermerk_auslesen_abt2_script =
                neu.lines().map(|l| l.replace("\u{00a0}", " ")).collect();
            data.konfiguration.speichern();
        }
        Cmd::EditRechtsinhaberAuslesenAbt2Script { neu } => {
            data.konfiguration.rechtsinhaber_auslesen_abt2_script =
                neu.lines().map(|l| l.replace("\u{00a0}", " ")).collect();
            data.konfiguration.speichern();
        }
        Cmd::EditRechtsinhaberAuslesenAbt3Script { neu } => {
            data.konfiguration.rechtsinhaber_auslesen_abt3_script =
                neu.lines().map(|l| l.replace("\u{00a0}", " ")).collect();
            data.konfiguration.speichern();
        }
        Cmd::EditSchuldenArtScript { neu } => {
            data.konfiguration.klassifiziere_schuldenart =
                neu.lines().map(|l| l.replace("\u{00a0}", " ")).collect();
            data.konfiguration.speichern();
        }
        Cmd::EditBetragAuslesenScript { neu } => {
            data.konfiguration.betrag_auslesen_script =
                neu.lines().map(|l| l.replace("\u{00a0}", " ")).collect();
            data.konfiguration.speichern();
        }
        Cmd::FlurstueckAuslesenScriptTesten { text, bv_nr } => {
            let start = std::time::Instant::now();
            let mut debug_log = String::new();
            let result: Result<String, String> = Err(String::new()).or_else(|_| {
                let (text_sauber, saetze_clean) =
                    crate::kurztext::text_saubern(data.vm.clone(), &*text, &data.konfiguration)?;

                let mut fehler = Vec::new();
                let mut warnungen = Vec::new();
                let mut spalte1_eintraege = Vec::new();

                let default_bv = Vec::new();
                let open_file = data
                    .open_page
                    .clone()
                    .and_then(|(file, _)| data.loaded_files.get_mut(&file));

                let bv_eintraege = crate::analyse::get_belastete_flurstuecke(
                    data.vm.clone(),
                    bv_nr,
                    &text_sauber,
                    &Titelblatt {
                        amtsgericht: "XXX".to_string(),
                        grundbuch_von: "Unbekannt".to_string(),
                        blatt: "0".to_string(),
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
                Ok(spalte1_eintraege
                    .iter()
                    .map(|e| format!("{e:#?}"))
                    .collect::<Vec<_>>()
                    .join("\r\n"))
            });

            let time = std::time::Instant::now() - start;
            let result: String = match result {
                Ok(o) => {
                    format!(
                        "{}\r\nLOG:\r\n{}\r\nAusgabe berechnet in {:?}",
                        o, debug_log, time
                    )
                }
                Err(e) => {
                    format!("{}", e)
                }
            };
            let _ = webview.evaluate_script(&format!(
                "replaceFlurstueckAuslesenTestOutput(`{}`);",
                result
            ));
        }
        Cmd::RangvermerkAuslesenAbt2ScriptTesten { text } => {
            let start = std::time::Instant::now();
            let result: Result<String, String> = Err(String::new()).or_else(|_| {
                let (text_sauber, saetze_clean) =
                    crate::kurztext::text_saubern(data.vm.clone(), &*text, &data.konfiguration)?;

                crate::python::get_rangvermerk_abt2(
                    data.vm.clone(),
                    "",
                    &text_sauber,
                    &saetze_clean,
                    &data.konfiguration,
                )
            });

            let time = std::time::Instant::now() - start;
            let result: String = match result {
                Ok(o) => {
                    format!("{}\r\nAusgabe berechnet in {:?}", o, time)
                }
                Err(e) => {
                    format!("{}", e)
                }
            };
            let _ = webview.evaluate_script(&format!(
                "replaceRangvermerkAuslesenAbt2TestOutput(`{}`);",
                result
            ));
        }
        Cmd::RechtsinhaberAuslesenAbt2ScriptTesten { text } => {
            let start = std::time::Instant::now();
            let result: Result<String, String> = Err(String::new()).or_else(|_| {
                let (text_sauber, saetze_clean) =
                    crate::kurztext::text_saubern(data.vm.clone(), &*text, &data.konfiguration)?;

                crate::python::get_rechtsinhaber_abt2(
                    data.vm.clone(),
                    "",
                    &text_sauber,
                    &saetze_clean,
                    &data.konfiguration,
                )
            });
            let time = std::time::Instant::now() - start;
            let result: String = match result {
                Ok(o) => {
                    format!("{}\r\nAusgabe berechnet in {:?}", o, time)
                }
                Err(e) => {
                    format!("{}", e)
                }
            };
            let _ = webview.evaluate_script(&format!(
                "replaceRechtsinhaberAbt2TestOutput(`{}`);",
                result
            ));
        }
        Cmd::RechtsinhaberAuslesenAbt3ScriptTesten { text } => {
            let start = std::time::Instant::now();
            let result: Result<String, String> = Err(String::new()).or_else(|_| {
                let (text_sauber, saetze_clean) =
                    crate::kurztext::text_saubern(data.vm.clone(), &*text, &data.konfiguration)?;

                crate::python::get_rechtsinhaber_abt3(
                    data.vm.clone(),
                    "",
                    &text_sauber,
                    &saetze_clean,
                    &data.konfiguration,
                )
            });
            let time = std::time::Instant::now() - start;
            let result = match result {
                Ok(o) => {
                    format!("{}\r\nAusgabe berechnet in {:?}", o, time)
                }
                Err(e) => {
                    format!("{}", e)
                }
            };
            let _ = webview.evaluate_script(&format!(
                "replaceRechtsinhaberAbt3TestOutput(`{}`);",
                result
            ));
        }
        Cmd::BetragAuslesenScriptTesten { text } => {
            let start = std::time::Instant::now();
            let result: Result<Betrag, String> = Err(String::new()).or_else(|_| {
                let (text_sauber, saetze_clean) =
                    crate::kurztext::text_saubern(data.vm.clone(), &*text, &data.konfiguration)?;

                crate::python::get_betrag_abt3(
                    data.vm.clone(),
                    "",
                    &text_sauber,
                    &saetze_clean,
                    &data.konfiguration,
                )
            });
            let time = std::time::Instant::now() - start;
            let result = match result {
                Ok(o) => {
                    format!("{:#?}\r\nAusgabe berechnet in {:?}", o, time)
                }
                Err(e) => {
                    format!("{}", e)
                }
            };
            let _ =
                webview.evaluate_script(&format!("replaceBetragAuslesenTestOutput(`{}`);", result));
        }
        Cmd::KurzTextAbt2ScriptTesten { text } => {
            let start = std::time::Instant::now();
            let result: Result<String, String> = Err(String::new()).or_else(|_| {
                let (text_sauber, saetze_clean) =
                    crate::kurztext::text_saubern(data.vm.clone(), &*text, &data.konfiguration)?;

                let rechtsinhaber = crate::python::get_rechtsinhaber_abt2(
                    data.vm.clone(),
                    "",
                    &text_sauber,
                    &saetze_clean,
                    &data.konfiguration,
                )
                .ok();

                let rangvermerk = crate::python::get_rangvermerk_abt2(
                    data.vm.clone(),
                    "",
                    &text_sauber,
                    &saetze_clean,
                    &data.konfiguration,
                )
                .ok();

                crate::python::get_kurztext_abt2(
                    data.vm.clone(),
                    "",
                    &text_sauber,
                    rechtsinhaber,
                    rangvermerk,
                    &saetze_clean,
                    &data.konfiguration,
                )
            });

            let time = std::time::Instant::now() - start;
            let result = match result {
                Ok(o) => {
                    format!("{}\r\nAusgabe berechnet in {:?}", o, time)
                }
                Err(e) => {
                    format!("{}", e)
                }
            };
            let _ = webview
                .evaluate_script(&format!("replaceTextKuerzenAbt2TestOutput(`{}`);", result));
        }
        Cmd::KurzTextAbt3ScriptTesten { text } => {
            let start = std::time::Instant::now();

            let result: Result<String, String> = Err(String::new()).or_else(|_| {
                let (text_sauber, saetze_clean) =
                    crate::kurztext::text_saubern(data.vm.clone(), &*text, &data.konfiguration)?;

                let schuldenart = crate::python::get_schulden_art_abt3(
                    data.vm.clone(),
                    "",
                    &text_sauber,
                    &saetze_clean,
                    &data.konfiguration,
                )?;

                let betrag = crate::python::get_betrag_abt3(
                    data.vm.clone(),
                    "",
                    &format!("100.000,00 EUR"),
                    &[format!("100.000,00 EUR")],
                    &data.konfiguration,
                )?;

                let rechtsinhaber = crate::python::get_rechtsinhaber_abt3(
                    data.vm.clone(),
                    "",
                    &text_sauber,
                    &saetze_clean,
                    &data.konfiguration,
                )?;

                let betrag = format!(
                    "{} {}",
                    crate::kurztext::formatiere_betrag(&betrag),
                    betrag.waehrung.to_string()
                );
                crate::python::get_kurztext_abt3(
                    data.vm.clone(),
                    "",
                    &text_sauber,
                    Some(betrag),
                    Some(schuldenart.to_string().to_string()),
                    Some(rechtsinhaber),
                    &saetze_clean,
                    &data.konfiguration,
                )
            });

            let time = std::time::Instant::now() - start;
            let result = match result {
                Ok(o) => {
                    format!("{}\r\nAusgabe berechnet in {:?}", o, time)
                }
                Err(e) => {
                    format!("{}", e)
                }
            };
            let _ = webview
                .evaluate_script(&format!("replaceTextKuerzenAbt3TestOutput(`{}`);", result));
        }

        Cmd::RechteArtScriptTesten { text } => {
            let start = std::time::Instant::now();
            let result: Result<RechteArt, String> = Err(String::new()).or_else(|_| {
                let (text_sauber, saetze_clean) =
                    crate::kurztext::text_saubern(data.vm.clone(), &*text, &data.konfiguration)?;

                crate::python::get_rechte_art_abt2(
                    data.vm.clone(),
                    "",
                    &text_sauber,
                    &saetze_clean,
                    &data.konfiguration,
                )
            });
            let time = std::time::Instant::now() - start;
            let result = match result {
                Ok(o) => {
                    format!("{:?}\r\nAusgabe berechnet in {:?}", o, time)
                }
                Err(e) => {
                    format!("{}", e)
                }
            };

            let _ = webview.evaluate_script(&format!("replaceRechteArtTestOutput(`{}`);", result));
        }
        Cmd::SchuldenArtScriptTesten { text } => {
            let start = std::time::Instant::now();
            let result: Result<SchuldenArt, String> = Err(String::new()).or_else(|_| {
                let (text_sauber, saetze_clean) =
                    crate::kurztext::text_saubern(data.vm.clone(), &*text, &data.konfiguration)?;

                crate::python::get_schulden_art_abt3(
                    data.vm.clone(),
                    "",
                    &text_sauber,
                    &saetze_clean,
                    &data.konfiguration,
                )
            });
            let time = std::time::Instant::now() - start;
            let result = match result {
                Ok(o) => {
                    format!("{:?}\r\nAusgabe berechnet in {:?}", o, time)
                }
                Err(e) => {
                    format!("{}", e)
                }
            };
            let _ =
                webview.evaluate_script(&format!("replaceSchuldenArtTestOutput(`{}`);", result));
        }
        Cmd::DeleteNebenbeteiligte => {
            use tinyfiledialogs::YesNo;

            if data.loaded_files.is_empty() {
                return;
            }

            if tinyfiledialogs::message_box_yes_no(
                "Wirklich löschen?",
                &format!("Alle Ordnungsnummern werden aus den Dateien gelöscht. Fortfahren?"),
                MessageBoxIcon::Warning,
                YesNo::No,
            ) == YesNo::No
            {
                return;
            }

            data.loaded_nb.clear();
            data.loaded_nb_paths.clear();
            for pdf_file in data.loaded_files.values_mut() {
                pdf_file.nebenbeteiligte_dateipfade.clear();
                pdf_file.speichern();
            }

            let _ = webview.evaluate_script(&format!(
                "replaceEntireScreen(`{}`)",
                ui::render_entire_screen(data)
            ));
        }
        Cmd::KlassifiziereSeiteNeu {
            seite,
            klassifikation_neu,
        } => {
            use crate::digital::SeitenTyp::*;

            let open_file = match data
                .open_page
                .clone()
                .and_then(|(file, _)| data.loaded_files.get_mut(&file))
            {
                Some(s) => s,
                None => return,
            };

            let seiten_typ_neu = match klassifikation_neu.as_str() {
                "bv-horz" => BestandsverzeichnisHorz,
                "bv-horz-zu-und-abschreibungen" => BestandsverzeichnisHorzZuUndAbschreibungen,
                "bv-vert" => BestandsverzeichnisVert,
                "bv-vert-typ2" => BestandsverzeichnisVertTyp2,
                "bv-vert-zu-und-abschreibungen" => BestandsverzeichnisVertZuUndAbschreibungen,
                "bv-vert-zu-und-abschreibungen-alt" => {
                    BestandsverzeichnisVertZuUndAbschreibungenAlt
                }
                "abt1-horz" => Abt1Horz,
                "abt1-vert" => Abt1Vert,
                "abt1-vert-typ2" => Abt1VertTyp2,
                "abt2-horz-veraenderungen" => Abt2HorzVeraenderungen,
                "abt2-horz" => Abt2Horz,
                "abt2-vert-veraenderungen" => Abt2VertVeraenderungen,
                "abt2-vert" => Abt2Vert,
                "abt2-vert-typ2" => Abt2VertTyp2,
                "abt3-horz-veraenderungen-loeschungen" => Abt3HorzVeraenderungenLoeschungen,
                "abt3-vert-veraenderungen-loeschungen" => Abt3VertVeraenderungenLoeschungen,
                "abt3-horz" => Abt3Horz,
                "abt3-vert-veraenderungen" => Abt3VertVeraenderungen,
                "abt3-vert-loeschungen" => Abt3VertLoeschungen,
                "abt3-vert" => Abt3Vert,
                _ => {
                    return;
                }
            };

            open_file
                .anpassungen_seite
                .entry(format!("{}", *seite))
                .or_insert_with(|| AnpassungSeite::default())
                .klassifikation_neu = Some(seiten_typ_neu);

            data.popover_state = None;

            let open_file = match data
                .open_page
                .clone()
                .and_then(|(file, _)| data.loaded_files.get_mut(&file))
            {
                Some(s) => s,
                None => return,
            };

            // speichern
            open_file.speichern();
            let _ = webview.evaluate_script("saveState();");
            let _ = webview.evaluate_script(&format!(
                "replaceEntireScreen(`{}`);",
                ui::render_entire_screen(data)
            ));
        }
        Cmd::ClosePopOver {} => {
            if let Some(PopoverState::Configuration(_)) = data.popover_state {
                for (k, v) in data.loaded_files.iter_mut() {
                    v.icon = None;
                }
                let _ = webview.evaluate_script(&format!(
                    "replaceFileList(`{}`)",
                    ui::render_file_list(data)
                ));
            }
            data.popover_state = None;
            let _ = webview.evaluate_script(&format!(
                "replacePopOver(`{}`)",
                ui::render_popover_content(data)
            ));
            let _ = webview.evaluate_script("saveState();");
        }
        Cmd::SaveState => {
            let mut open_file = match data
                .open_page
                .clone()
                .and_then(|(file, _)| data.loaded_files.get_mut(&file))
            {
                Some(s) => s,
                None => return,
            };

            let mut current_state = open_file.clone();
            open_file.previous_state = Some(Box::new(current_state));
            open_file.next_state = None;
        }
        Cmd::Undo => {
            let mut open_file = match data
                .open_page
                .clone()
                .and_then(|(file, _)| data.loaded_files.get_mut(&file))
            {
                Some(s) => s,
                None => return,
            };

            let mut previous_state = match open_file.previous_state.clone() {
                Some(s) => s,
                None => return,
            };

            previous_state.next_state = Some(Box::new(open_file.clone()));
            *open_file = *previous_state;
            open_file.speichern();

            let _ = webview.evaluate_script(&format!(
                "replacePageList(`{}`);",
                ui::render_page_list(&data)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceMainNoFiles(`{}`);",
                ui::render_application_main_no_files(data)
            ));
        }
        Cmd::Redo => {
            let mut open_file = match data
                .open_page
                .clone()
                .and_then(|(file, _)| data.loaded_files.get_mut(&file))
            {
                Some(s) => s,
                None => return,
            };

            let mut next_state = match open_file.next_state.clone() {
                Some(s) => s,
                None => return,
            };

            next_state.previous_state = Some(Box::new(open_file.clone()));
            *open_file = *next_state;
            open_file.speichern();

            let _ = webview.evaluate_script(&format!(
                "replacePageList(`{}`);",
                ui::render_page_list(&data)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceMainNoFiles(`{}`);",
                ui::render_application_main_no_files(data)
            ));
        }
        Cmd::ResetOcrSelection => {
            let _ = webview.evaluate_script(&format!("resetOcrSelection()"));
        }
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
            let file = match data.loaded_files.get_mut(file_name.as_str()) {
                Some(s) => s,
                None => {
                    let _ = webview.evaluate_script(&format!("resetOcrSelection()"));
                    return;
                }
            };

            if file.datei.as_ref().is_none() {
                let _ = webview.evaluate_script(&format!("resetOcrSelection()"));
                return;
            }

            if !file.ist_geladen() {
                let _ = webview.evaluate_script(&format!("resetOcrSelection()"));
                return;
            }

            let hocr_page = match file.hocr.seiten.get(&format!("{page}")) {
                Some(s) => s,
                None => {
                    let _ = webview.evaluate_script(&format!("resetOcrSelection()"));
                    return;
                }
            };

            let rect = Rect {
                min_x: *min_x / page_width * hocr_page.breite_mm,
                min_y: *min_y / page_height * hocr_page.hoehe_mm,
                max_x: *max_x / page_width * hocr_page.breite_mm,
                max_y: *max_y / page_height * hocr_page.hoehe_mm,
            };

            let text = hocr_page.get_words_within_bounds(&rect).join("\r\n");

            let text = if data.konfiguration.zeilenumbrueche_in_ocr_text {
                text
            } else {
                let result: Result<String, String> =
                    crate::kurztext::text_saubern(data.vm.clone(), &*text, &data.konfiguration)
                        .map(|s| s.0);
                match result {
                    Ok(o) => o,
                    Err(e) => e,
                }
            };
            let _ = webview.evaluate_script(&format!("copyTextToClipboard(`{}`)", text));
            let _ = webview.evaluate_script(&format!("resetOcrSelection()"));
        }
        Cmd::ReloadGrundbuch => {
            use tinyfiledialogs::YesNo;

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

            if tinyfiledialogs::message_box_yes_no(
                "Grundbuch neu laden?",
                &format!("Wenn das Grundbuch neu analysiert wird, werden alle manuell eingegebenen Daten überschrieben.\r\nFortfahren?"),
                MessageBoxIcon::Warning,
                YesNo::No,
            ) == YesNo::No {
                return;
            }

            *open_file = reload_hocr_files(&open_file);
            crate::digital::insert_zeilen_automatisch(open_file);

            let file_name = format!(
                "{}_{}",
                open_file.analysiert.titelblatt.grundbuch_von,
                open_file.analysiert.titelblatt.blatt
            );
            let output_parent = open_file.get_gbx_datei_parent();
            let cache_output_path = output_parent
                .clone()
                .join(&format!("{}.cache.gbx", file_name));

            let grundbuch_neu =
                match analyse_grundbuch(data.vm.clone(), &open_file, &data.konfiguration) {
                    Some(s) => s,
                    None => return,
                };

            open_file.analysiert = grundbuch_neu;

            open_file.speichern();

            let pdf_path = open_file.datei.clone().unwrap_or_default();

            let _ = webview.evaluate_script(&format!(
                "replaceEntireScreen(`{}`)",
                ui::render_entire_screen(data)
            ));

            let _ = webview.evaluate_script(&format!(
                "startCheckingForPageLoaded(`{}`, `{}`, `{}`)",
                cache_output_path.display(),
                file_name,
                pdf_path,
            ));
        }
        Cmd::ZeileNeu { file, page, y } => {
            if data.loaded_files.is_empty() {
                return;
            }

            let open_file = match data.loaded_files.get_mut(&file.clone()) {
                Some(s) => s,
                None => return,
            };

            let mut ap = open_file
                .anpassungen_seite
                .entry(format!("{}", *page))
                .or_insert_with(|| AnpassungSeite::default());

            let (im_width, im_height, page_width, page_height) =
                match open_file.hocr.seiten.get(&format!("{}", *page)) {
                    Some(o) => (
                        o.parsed.bounds.max_x,
                        o.parsed.bounds.max_y,
                        o.breite_mm,
                        o.hoehe_mm,
                    ),
                    None => return,
                };

            let img_ui_width = 1200.0; // px
            let aspect_ratio = im_height / im_width;
            let img_ui_height = img_ui_width * aspect_ratio;

            if *y > img_ui_height || *y < 0.0 {
                return;
            }

            ap.insert_zeile_manuell(y / img_ui_height * page_height);

            let _ = webview.evaluate_script(&format!(
                "replacePdfImageZeilen(`{}`)",
                crate::ui::render_pdf_image_zeilen(
                    &ap.get_zeilen()
                        .iter()
                        .map(|(a, b)| (*a, *b))
                        .collect::<Vec<_>>(),
                    page_height,
                    img_ui_height
                )
            ));

            // speichern
            open_file.speichern();
            let _ = webview.evaluate_script("saveState();");
        }
        Cmd::ZeileLoeschen {
            file,
            page,
            zeilen_id,
        } => {
            if data.loaded_files.is_empty() {
                return;
            }

            let open_file = match data.loaded_files.get_mut(&file.clone()) {
                Some(s) => s,
                None => return,
            };

            let (im_width, im_height, page_width, page_height) =
                match open_file.hocr.seiten.get(&format!("{}", *page)) {
                    Some(o) => (
                        o.parsed.bounds.max_x,
                        o.parsed.bounds.max_y,
                        o.breite_mm,
                        o.hoehe_mm,
                    ),
                    None => return,
                };

            let img_ui_width = 1200.0; // px
            let aspect_ratio = im_height / im_width;
            let img_ui_height = img_ui_width * aspect_ratio;

            if let Some(ap) = open_file.anpassungen_seite.get_mut(&format!("{}", page)) {
                ap.delete_zeile_manuell(*zeilen_id);
                let _ = webview.evaluate_script(&format!(
                    "replacePdfImageZeilen(`{}`)",
                    crate::ui::render_pdf_image_zeilen(
                        &ap.get_zeilen()
                            .iter()
                            .map(|(a, b)| (*a, *b))
                            .collect::<Vec<_>>(),
                        page_height,
                        img_ui_height
                    )
                ));
            }

            // speichern
            open_file.speichern();
            let _ = webview.evaluate_script("saveState();");
        }
        Cmd::ResizeColumn {
            direction,
            column_id,
            x,
            y,
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

            let seitentyp = open_file
                .get_seiten_typ(&page.to_string())
                .unwrap_or(SeitenTyp::BestandsverzeichnisVert);

            let current_column = match seitentyp
                .get_columns(open_file.anpassungen_seite.get(&format!("{}", page)))
                .iter()
                .find(|col| col.id == column_id)
            {
                Some(s) => s.clone(),
                None => return,
            };

            let (im_width, im_height, page_width, page_height) =
                match open_file.hocr.seiten.get(&format!("{}", page)) {
                    Some(o) => (
                        o.parsed.bounds.max_x,
                        o.parsed.bounds.max_y,
                        o.breite_mm,
                        o.hoehe_mm,
                    ),
                    None => return,
                };

            let img_ui_width = 1200.0; // px
            let aspect_ratio = im_height / im_width;
            let img_ui_height = img_ui_width * aspect_ratio;

            {
                let rect_to_modify = open_file
                    .anpassungen_seite
                    .entry(format!("{}", page))
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
                    }
                    "ne" => {
                        rect_to_modify.min_y = y / img_ui_height * page_height;
                        rect_to_modify.max_x = x / img_ui_width * page_width;
                    }
                    "se" => {
                        rect_to_modify.max_y = y / img_ui_height * page_height;
                        rect_to_modify.max_x = x / img_ui_width * page_width;
                    }
                    "sw" => {
                        rect_to_modify.max_y = y / img_ui_height * page_height;
                        rect_to_modify.min_x = x / img_ui_width * page_width;
                    }
                    _ => return,
                };
            }

            let new_column = match seitentyp
                .get_columns(open_file.anpassungen_seite.get(&format!("{}", page)))
                .iter()
                .find(|col| col.id == column_id)
            {
                Some(s) => s.clone(),
                None => return,
            };

            let new_width = (new_column.max_x - new_column.min_x).abs() / page_width * img_ui_width;
            let new_height =
                (new_column.max_y - new_column.min_y).abs() / page_height * img_ui_height;
            let new_x = new_column.min_x.min(new_column.max_x) / page_width * img_ui_width;
            let new_y = new_column.min_y.min(new_column.max_y) / page_height * img_ui_height;

            // speichern
            open_file.speichern();
            let _ = webview.evaluate_script("saveState();");

            let _ = webview.evaluate_script(&format!(
                "adjustColumn(`{}`,`{}`,`{}`,`{}`,`{}`)",
                column_id, new_width, new_height, new_x, new_y
            ));
        }
        Cmd::ToggleCheckBox { checkbox_id } => {
            match checkbox_id.as_str() {
                "konfiguration-zeilenumbrueche-in-ocr-text" => {
                    data.konfiguration.zeilenumbrueche_in_ocr_text =
                        !data.konfiguration.zeilenumbrueche_in_ocr_text;
                }
                "konfiguration-spalten-ausblenden" => {
                    data.konfiguration.spalten_ausblenden = !data.konfiguration.spalten_ausblenden;
                }
                "konfiguration-keine-roten-linien" => {
                    data.konfiguration.vorschau_ohne_geroetet =
                        !data.konfiguration.vorschau_ohne_geroetet;
                }
                "konfiguration-passwort-speichern" => {
                    data.konfiguration.passwort_speichern = !data.konfiguration.passwort_speichern;
                }
                _ => return,
            }

            data.konfiguration.speichern();
        }
        Cmd::ImportNebenbeteiligte => {
            if data.loaded_files.is_empty() {
                return;
            }

            let file_dialog_result = tinyfiledialogs::open_file_dialog(
                "Nebenbeteiligte Ordnungsnummern auswählen",
                "",
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
                use tinyfiledialogs::YesNo;

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

            let open_file = match data
                .open_page
                .clone()
                .and_then(|(file, _)| data.loaded_files.get(&file))
            {
                Some(s) => s,
                None => return,
            };

            let analyse = open_file.cache.start_analyzing(
                &open_file.analysiert,
                &data.vm,
                &data.loaded_nb,
                &data.konfiguration,
            );
            let _ = webview.evaluate_script(&format!(
                "replaceAnalyseGrundbuch(`{}`);",
                ui::render_analyse_grundbuch(&analyse, false, false)
            ));
        }
        Cmd::ExportNebenbeteiligte => {
            if data.loaded_files.is_empty() {
                return;
            }

            let file_dialog_result =
                tinyfiledialogs::save_file_dialog("Nebenbeteiligte .TSV speichern unter", "");

            let f = match file_dialog_result {
                Some(f) => {
                    if f.ends_with(".tsv") {
                        f
                    } else {
                        format!("{}.tsv", f)
                    }
                }
                None => return,
            };

            let tsv = get_nebenbeteiligte_tsv(&data);

            let _ = std::fs::write(&f, tsv.as_bytes());
        }
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
            use crate::pdf::{GenerateGrundbuchConfig, GrundbuchExportConfig, PdfExportTyp};

            if data.loaded_files.is_empty() {
                return;
            }

            let target = match exportiere_in_eine_einzelne_datei {
                true => {
                    let file_dialog_result =
                        tinyfiledialogs::save_file_dialog("PDF Datei speichern unter", "");

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
                }
                false => {
                    let file_dialog_result =
                        tinyfiledialogs::select_folder_dialog("PDF Dateien speichern unter", "");

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
                }
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

                    open_file.analysiert.titelblatt = open_file.analysiert.titelblatt.clone();
                    PdfExportTyp::OffenesGrundbuch(open_file.analysiert.clone())
                }
                "alle-offen-digitalisiert" => {
                    let files = data
                        .loaded_files
                        .values()
                        .filter_map(|f| {
                            if f.datei.is_none() {
                                return None;
                            }
                            Some(f.clone())
                        })
                        .map(|mut f| {
                            f.analysiert.titelblatt = f.analysiert.titelblatt.clone();
                            f.analysiert
                        })
                        .collect::<Vec<_>>();

                    PdfExportTyp::AlleOffenDigitalisiert(files)
                }
                "alle-offen" => {
                    let files = data
                        .loaded_files
                        .values()
                        .map(|f| f.clone())
                        .map(|mut f| {
                            f.analysiert.titelblatt = f.analysiert.titelblatt.clone();
                            f.analysiert
                        })
                        .collect::<Vec<_>>();

                    PdfExportTyp::AlleOffen(files)
                }
                "alle-original" => {
                    let files = data
                        .loaded_files
                        .values()
                        .filter_map(|f| f.datei.clone())
                        .collect::<Vec<_>>();

                    PdfExportTyp::AlleOriginalPdf(files)
                }
                _ => {
                    return;
                }
            };

            let result = pdf::export_grundbuch(GrundbuchExportConfig {
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
        }
        Cmd::ExportAlleRechte => {
            if data.loaded_files.is_empty() {
                return;
            }

            let file_dialog_result =
                tinyfiledialogs::save_file_dialog("Rechte .HTML speichern unter", "");

            let f = match file_dialog_result {
                Some(f) => {
                    if f.ends_with(".html") {
                        f
                    } else {
                        format!("{}.html", f)
                    }
                }
                None => return,
            };

            let html = format!(
                "<html><head><style>* {{ margin:0px;padding:0px; }}</style></head><body>{}</body>",
                get_alle_rechte_html(&data)
            );

            let _ = std::fs::write(&f, html.as_bytes());
        }
        Cmd::ExportAlleFehler => {
            if data.loaded_files.is_empty() {
                return;
            }

            let file_dialog_result =
                tinyfiledialogs::save_file_dialog("Rechte .HTML speichern unter", "");

            let f = match file_dialog_result {
                Some(f) => {
                    if f.ends_with(".html") {
                        f
                    } else {
                        format!("{}.html", f)
                    }
                }
                None => return,
            };

            let html = format!(
                "<html><head><style>* {{ margin:0px;padding:0px; }}</style></head><body>{}</body>",
                get_alle_fehler_html(&data)
            );

            let _ = std::fs::write(&f, html.as_bytes());
        }
        Cmd::ExportAlleAbt1 => {
            if data.loaded_files.is_empty() {
                return;
            }

            let file_dialog_result =
                tinyfiledialogs::save_file_dialog("Rechte .HTML speichern unter", "");

            let f = match file_dialog_result {
                Some(f) => {
                    if f.ends_with(".html") {
                        f
                    } else {
                        format!("{}.html", f)
                    }
                }
                None => return,
            };

            let html = format!(
                "<html><head><style>* {{ margin:0px;padding:0px; }}</style></head><body>{}</body>",
                get_alle_abt1_html(&data)
            );

            let _ = std::fs::write(&f, html.as_bytes());
        }
        Cmd::ExportAlleTeilbelastungen => {
            if data.loaded_files.is_empty() {
                return;
            }

            let file_dialog_result =
                tinyfiledialogs::save_file_dialog("Teilbelastungen .HTML speichern unter", "");

            let f = match file_dialog_result {
                Some(f) => {
                    if f.ends_with(".html") {
                        f
                    } else {
                        format!("{}.html", f)
                    }
                }
                None => return,
            };

            let html = format!(
                "<html><head><style>* {{ margin:0px;padding:0px; }}</style></head><body>{}</body>",
                get_alle_teilbelastungen_html(&data)
            );

            let _ = std::fs::write(&f, html.as_bytes());
        }
        Cmd::ExportLefis => {
            if data.loaded_files.is_empty() {
                return;
            }

            let analysiert = data
                .loaded_files
                .values()
                .map(|file| LefisDateiExport {
                    rechte: file.cache.start_and_block_until_finished(
                        &file.analysiert,
                        &data.vm,
                        &data.loaded_nb,
                        &data.konfiguration,
                    ),
                    titelblatt: file.analysiert.titelblatt.clone(),
                })
                .collect::<Vec<_>>();

            let json = match serde_json::to_string_pretty(&analysiert) {
                Ok(o) => o,
                Err(_) => return,
            };

            let json = json.lines().collect::<Vec<_>>().join("\r\n");

            // Benutzer warnen, falls Datei noch Fehler enthält
            let mut fehler = analysiert
                .iter()
                .flat_map(|l| {
                    l.rechte
                        .abt2
                        .iter()
                        .filter_map(|f| {
                            if f.fehler.is_empty() {
                                None
                            } else {
                                Some(format!(
                                    "{} Blatt {}, Abt 2 lfd. Nr. {}",
                                    l.titelblatt.grundbuch_von, l.titelblatt.blatt, f.lfd_nr
                                ))
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();

            fehler.extend(analysiert.iter().flat_map(|l| {
                l.rechte
                    .abt3
                    .iter()
                    .filter_map(|f| {
                        if f.fehler.is_empty() {
                            None
                        } else {
                            Some(format!(
                                "{} Blatt {}, Abt 3 lfd. Nr. {}",
                                l.titelblatt.grundbuch_von, l.titelblatt.blatt, f.lfd_nr
                            ))
                        }
                    })
                    .collect::<Vec<_>>()
            }));

            if !fehler.is_empty() {
                use tinyfiledialogs::YesNo;

                if tinyfiledialogs::message_box_yes_no(
                    "Mit Fehlern exportieren?",
                    &format!("Die folgenden Einträge enthalten Fehler:\r\n\r\n{}\r\n\r\nTrotzdem .lefis-Datei exportieren?", fehler.join("\r\n")),
                    MessageBoxIcon::Warning,
                    YesNo::No,
                ) == YesNo::No {
                    return;
                }
            }

            let file_dialog_result =
                tinyfiledialogs::save_file_dialog(".lefis-Datei speichern unter", "");

            let f = match file_dialog_result {
                Some(f) => {
                    if f.ends_with(".lefis") {
                        f
                    } else {
                        format!("{}.lefis", f)
                    }
                }
                None => return,
            };

            let _ = std::fs::write(&f, json.as_bytes());
        }
        Cmd::EditRegexKey { old_key, new_key } => {
            let old_key: String = old_key.chars().filter(|c| !c.is_whitespace()).collect();
            let new_key: String = new_key.chars().filter(|c| !c.is_whitespace()).collect();
            if data.konfiguration.regex.get(&new_key).is_some() {
                return;
            }
            let cur_value = data
                .konfiguration
                .regex
                .get(&old_key)
                .cloned()
                .unwrap_or_default();
            data.konfiguration.regex.remove(&old_key);
            data.konfiguration.regex.insert(new_key, cur_value);
            data.konfiguration.speichern();
        }
        Cmd::EditRegexValue { key, value } => {
            let key: String = key.chars().filter(|c| !c.is_whitespace()).collect();
            let value: String = value.chars().filter(|c| *c != '\n').collect();
            data.konfiguration.regex.insert(key, value);
            data.konfiguration.speichern();
        }
        Cmd::InsertRegex { regex_key } => {
            data.konfiguration
                .regex
                .insert(format!("{}_1", regex_key), "(.*)".to_string());
            data.konfiguration.speichern();
            let _ = webview.evaluate_script(&format!(
                "replaceEntireScreen(`{}`)",
                ui::render_entire_screen(data)
            ));
        }
        Cmd::RegexLoeschen { regex_key } => {
            data.konfiguration.regex.remove(regex_key);
            if data.konfiguration.regex.is_empty() {
                data.konfiguration
                    .regex
                    .insert("REGEX_ID".to_string(), "(.*)".to_string());
            }
            data.konfiguration.speichern();
            let _ = webview.evaluate_script(&format!(
                "replaceEntireScreen(`{}`)",
                ui::render_entire_screen(data)
            ));
        }
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
                }
                Err(e) => {
                    format!("{}", e)
                }
            };
            let _ = webview.evaluate_script(&format!("replaceRegexTestOutput(`{}`);", result));
        }
        Cmd::SetActiveRibbonTab { new_tab } => {
            data.active_tab = *new_tab;
            let _ =
                webview.evaluate_script(&format!("replaceRibbon(`{}`);", ui::render_ribbon(&data)));
        }
        Cmd::SetOpenFile { new_file } => {
            data.open_page = Some((new_file.clone(), 2));

            let open_file = match data
                .open_page
                .clone()
                .and_then(|(file, _)| data.loaded_files.get(&file))
            {
                Some(s) => s,
                None => return,
            };

            let titelblatt = open_file.analysiert.titelblatt.clone();

            let _ = webview.evaluate_script(&format!(
                "replacePageList(`{}`);",
                ui::render_page_list(&data)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceMainNoFiles(`{}`);",
                ui::render_application_main_no_files(data)
            ));
        }
        Cmd::SetOpenPage { active_page } => {
            if let Some(p) = data.open_page.as_mut() {
                p.1 = *active_page;
            }

            let open_file = match data
                .open_page
                .clone()
                .and_then(|(file, _)| data.loaded_files.get(&file))
            {
                Some(s) => s,
                None => return,
            };

            // let _ = webview.evaluate_script(&format!("replaceMainContainer(`{}`);", ui::render_main_container(data)));
            let _ = webview.evaluate_script(&format!(
                "replaceBestandsverzeichnis(`{}`);",
                ui::render_bestandsverzeichnis(open_file, &data.konfiguration)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceBestandsverzeichnisZuschreibungen(`{}`);",
                ui::render_bestandsverzeichnis_zuschreibungen(open_file)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceBestandsverzeichnisAbschreibungen(`{}`);",
                ui::render_bestandsverzeichnis_abschreibungen(open_file)
            ));
            let _ = webview
                .evaluate_script(&format!("replaceAbt1(`{}`);", ui::render_abt_1(open_file)));
            let _ = webview.evaluate_script(&format!(
                "replaceAbt1GrundlagenEintragungen(`{}`);",
                ui::render_abt_1_grundlagen_eintragungen(open_file)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceAbt1Veraenderungen(`{}`);",
                ui::render_abt_1_veraenderungen(open_file)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceAbt1Loeschungen(`{}`);",
                ui::render_abt_1_loeschungen(open_file)
            ));
            let _ = webview
                .evaluate_script(&format!("replaceAbt2(`{}`);", ui::render_abt_2(open_file)));
            let _ = webview.evaluate_script(&format!(
                "replaceAbt2Veraenderungen(`{}`);",
                ui::render_abt_2_veraenderungen(open_file)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceAbt2Loeschungen(`{}`);",
                ui::render_abt_2_loeschungen(open_file)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceAbt3(`{}`);",
                ui::render_abt_3(open_file, data.konfiguration.lefis_analyse_einblenden)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceAbt3Veraenderungen(`{}`);",
                ui::render_abt_3_veraenderungen(open_file)
            ));
            let _ = webview.evaluate_script(&format!(
                "replaceAbt3Loeschungen(`{}`);",
                ui::render_abt_3_loeschungen(open_file)
            ));

            let _ = webview.evaluate_script(&format!(
                "replaceFileList(`{}`);",
                ui::render_file_list(&data)
            ));
            let _ = webview.evaluate_script(&format!(
                "replacePageList(`{}`);",
                ui::render_page_list(&data)
            ));
            let _ = webview.evaluate_script(&format!(
                "replacePageImage(`{}`);",
                ui::render_pdf_image(&data)
            ));
        }
    }
}

fn parse_nb(fs: &str) -> Vec<Nebenbeteiligter> {
    let mut nb = Vec::new();

    for line in fs.lines() {
        if line.starts_with("ORDNUNGSNUMMER") {
            continue;
        }
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
                }
                1 => {
                    if let Some(typ) = NebenbeteiligterTyp::from_type_str(s.trim()) {
                        b.typ = Some(typ);
                    }
                }
                2 => {}
                3 => {
                    b.name = s.trim().to_string();
                }
                4 => {
                    if let Some(anrede) = Anrede::from_str(s.trim()) {
                        b.extra.anrede = Some(anrede);
                    }
                }
                5 => {
                    if !s.trim().is_empty() {
                        b.extra.titel = Some(s.trim().to_string());
                    }
                }
                6 => {
                    if !s.trim().is_empty() {
                        b.extra.vorname = Some(s.trim().to_string());
                    }
                }
                7 => {
                    if !s.trim().is_empty() {
                        b.extra.nachname_oder_firma = Some(s.trim().to_string());
                    }
                }
                8 => {
                    if !s.trim().is_empty() {
                        b.extra.geburtsname = Some(s.trim().to_string());
                    }
                }
                9 => {
                    if let Some(datum) = NebenbeteiligterExtra::geburtsdatum_from_str(s.trim()) {
                        b.extra.geburtsdatum = Some(datum);
                    }
                }
                10 => {
                    if !s.trim().is_empty() {
                        b.extra.wohnort = Some(s.trim().to_string());
                    }
                }
                _ => {}
            }
        }

        nb.push(b);
    }

    nb
}

fn get_alle_rechte_html(data: &RpcData) -> String {
    let mut entries = Vec::new();

    for (f_name, f) in data.loaded_files.iter() {
        let analyse = f.cache.start_and_block_until_finished(
            &f.analysiert,
            &data.vm,
            &data.loaded_nb,
            &data.konfiguration,
        );
        entries.push(crate::ui::render_analyse_grundbuch(&analyse, true, false));
    }

    entries.join("\r\n")
}

fn get_alle_teilbelastungen_html(data: &RpcData) -> String {
    let mut entries = String::new();

    for (f_name, f) in data.loaded_files.iter() {
        let gb_analysiert = f.cache.start_and_block_until_finished(
            &f.analysiert,
            &data.vm,
            &data.loaded_nb,
            &data.konfiguration,
        );

        let mut abt2_entries = String::new();

        for abt2 in gb_analysiert.abt2.iter() {
            let has_nur_lastend_an = abt2
                .lastend_an
                .iter()
                .any(|s1| !s1.nur_lastend_an.is_empty());

            if has_nur_lastend_an {
                let blatt = &f.analysiert.titelblatt.grundbuch_von;
                let nr = &f.analysiert.titelblatt.blatt;
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
                        let gemarkung = &e
                            .gemarkung
                            .as_ref()
                            .unwrap_or(&f.analysiert.titelblatt.grundbuch_von);
                        abt2_entries.push_str(&format!(
                            "<p>Gemarkung {gemarkung}, Flur {flur}, Flurstück {flurstueck}</p>"
                        ));
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
            let has_nur_lastend_an = abt3
                .lastend_an
                .iter()
                .any(|s1| !s1.nur_lastend_an.is_empty());

            if has_nur_lastend_an {
                let blatt = &f.analysiert.titelblatt.grundbuch_von;
                let nr = &f.analysiert.titelblatt.blatt;
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
                        let gemarkung = &e
                            .gemarkung
                            .as_ref()
                            .unwrap_or(&f.analysiert.titelblatt.grundbuch_von);
                        abt3_entries.push_str(&format!(
                            "<p>Gemarkung {gemarkung}, Flur {flur}, Flurstück {flurstueck}</p>"
                        ));
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
        let blatt = &f.analysiert.titelblatt.grundbuch_von;
        let nr = &f.analysiert.titelblatt.blatt;
        entries.push_str(&format!("<div><p>{blatt} Nr. {nr}</p>",));

        for abt1 in f
            .analysiert
            .abt1
            .eintraege
            .iter()
            .filter_map(|a1| match a1 {
                Abt1Eintrag::V2(v2) => Some(v2),
                _ => None,
            })
        {
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
        let gb_analysiert = f.cache.start_and_block_until_finished(
            &f.analysiert,
            &data.vm,
            &data.loaded_nb,
            &data.konfiguration,
        );
        entries.push(crate::ui::render_analyse_grundbuch(
            &gb_analysiert,
            true,
            true,
        ));
    }

    entries.join("\r\n")
}

fn get_rangvermerke_tsv(data: &RpcData) -> String {
    let mut entries = Vec::new();

    for (f_name, f) in data.loaded_files.iter() {
        let analysiert = f.cache.start_and_block_until_finished(
            &f.analysiert,
            &data.vm,
            &[],
            &data.konfiguration,
        );

        for a2 in analysiert.abt2 {
            if let Some(s) = a2.rangvermerk {
                entries.push(format!(
                    "{} A2/{}\t{}\t{}",
                    f_name, a2.lfd_nr, s, a2.text_original
                ));
            }
        }
    }

    format!("RECHT\tRVM\tTEXT\r\n{}", entries.join("\r\n"))
}

fn get_nebenbeteiligte_tsv(data: &RpcData) -> String {
    let mut nb = data
        .loaded_files
        .values()
        .flat_map(|file| file.get_nebenbeteiligte(data.vm.clone(), &data.konfiguration))
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
        rechte
            .entry(n.name.clone())
            .or_insert_with(|| Vec::new())
            .push(n.recht.clone());
        nb_keyed.insert(n.name.clone(), n);
    }

    let mut nb = nb_keyed.into_iter().map(|(k, v)| v).collect::<Vec<_>>();
    nb.sort_by(|a, b| a.name.cmp(&b.name));
    nb.dedup();

    let tsv = nb
        .iter()
        .map(|nb| {
            format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                nb.ordnungsnummer.map(|s| s.to_string()).unwrap_or_default(),
                nb.typ.map(|s| s.get_str()).unwrap_or_default(),
                rechte.get(&nb.name).cloned().unwrap_or_default().join("; "),
                nb.name,
                nb.extra.anrede.map(|s| s.to_string()).unwrap_or_default(),
                nb.extra.titel.clone().unwrap_or_default(),
                nb.extra.vorname.clone().unwrap_or_default(),
                nb.extra.nachname_oder_firma.clone().unwrap_or_default(),
                nb.extra.geburtsname.clone().unwrap_or_default(),
                nb.extra
                    .geburtsdatum
                    .as_ref()
                    .map(|gb| NebenbeteiligterExtra::geburtsdatum_to_str(gb))
                    .unwrap_or_default(),
                nb.extra.wohnort.clone().unwrap_or_default(),
            )
        })
        .collect::<Vec<_>>()
        .join("\r\n");
    let tsv = format!("ORDNUNGSNUMMER\tTYP\tRECHT\tNAME (GRUNDBUCH)\tANREDE\tTITEL\tVORNAME\tNACHNAME_ODER_FIRMA\tGEBURTSNAME\tGEBURTSDATUM\tWOHNORT\r\n{}", tsv);
    tsv
}

fn render_pdf_seiten(webview: &WebView, pdfs: &mut Vec<PdfFile>) {
    for pdf in pdfs {
        let pdf_datei_pfad = match pdf.datei.as_deref() {
            Some(s) => s,
            None => continue,
        };

        let pdf_bytes = match fs::read(&pdf_datei_pfad) {
            Ok(o) => o,
            Err(_) => continue,
        };

        let pdf_path = pdf.datei.clone();
        let seitenzahlen = if pdf_path.clone().unwrap_or_default().is_empty() {
            None
        } else {
            pdf_path
        }
        .and_then(|p| fs::read(p).ok())
        .map(|pdf_bytes| digital::lese_seitenzahlen(&pdf_bytes).ok())
        .unwrap_or_default();

        for seite in pdf.get_seitenzahlen().iter() {
            let _ = crate::digital::konvertiere_pdf_seite_zu_png_prioritaet(
                webview,
                &pdf_bytes,
                *seite,
                &pdf.analysiert.titelblatt,
            );
        }
    }
}

fn reload_hocr_files(pdf_parsed: &PdfFile) -> PdfFile {
    let linien = pdf_parsed
        .datei
        .as_ref()
        .and_then(|d| fs::read(d).ok())
        .and_then(|bytes| crate::digital::get_rote_linien(&bytes).ok())
        .unwrap_or_default();

    let tempdir = std::env::temp_dir()
        .join(&pdf_parsed.analysiert.titelblatt.grundbuch_von)
        .join(&pdf_parsed.analysiert.titelblatt.blatt.to_string());

    let breite_hoehe_mm = pdf_parsed
        .datei
        .clone()
        .and_then(|d| fs::read(d).ok())
        .and_then(|d| crate::digital::get_seiten_dimensionen(&d).ok())
        .unwrap_or_default();

    let hocr_loaded = pdf_parsed
        .get_seitenzahlen()
        .iter()
        .filter_map(|s| {
            let rot = linien.get(&s.to_string()).cloned().unwrap_or_default();
            let p = tempdir.join(format!("{s}.hocr.json"));
            let hocr = std::fs::read_to_string(&p).ok()?;
            let json: ParsedHocr = match serde_json::from_str(&hocr) {
                Ok(o) => o,
                Err(e) => {
                    return None;
                }
            };

            let (breite, hoehe) = breite_hoehe_mm.get(s)?;
            let seite = HocrSeite {
                breite_mm: *breite,
                hoehe_mm: *hoehe,
                parsed: json,
                rote_linien: rot,
            };
            Some((format!("{s}"), seite))
        })
        .collect::<BTreeMap<_, _>>();

    PdfFile {
        hocr: HocrLayout {
            seiten: hocr_loaded,
        },
        ..pdf_parsed.clone()
    }
}

fn analyse_grundbuch(vm: PyVm, pdf: &PdfFile, konfguration: &Konfiguration) -> Option<Grundbuch> {
    let seiten = pdf
        .hocr
        .seiten
        .iter()
        .filter_map(|(sz, seite)| {
            let typ = pdf.get_seiten_typ(sz)?;
            let seite_parsed = seite.get_textbloecke(sz, typ, &pdf.anpassungen_seite);
            Some((sz.clone(), seite_parsed))
        })
        .collect();

    let bestandsverzeichnis = digital::analysiere_bv(
        vm.clone(),
        &pdf.analysiert.titelblatt,
        &seiten,
        &pdf.hocr,
        &pdf.anpassungen_seite,
        konfguration,
    )
    .ok()?;
    let mut abt1 = digital::analysiere_abt1(
        vm.clone(),
        &seiten,
        &pdf.hocr,
        &pdf.anpassungen_seite,
        &bestandsverzeichnis,
        konfguration,
    )
    .ok()?;
    let abt2 = digital::analysiere_abt2(
        vm.clone(),
        &seiten,
        &pdf.hocr,
        &pdf.anpassungen_seite,
        &bestandsverzeichnis,
        konfguration,
    )
    .ok()?;
    let abt3 = digital::analysiere_abt3(
        vm.clone(),
        &seiten,
        &pdf.hocr,
        &pdf.anpassungen_seite,
        &bestandsverzeichnis,
        konfguration,
    )
    .ok()?;

    abt1.migriere_v2();

    let gb = Grundbuch {
        titelblatt: pdf.analysiert.titelblatt.clone(),
        bestandsverzeichnis,
        abt1,
        abt2,
        abt3,
    };

    Some(clean_grundbuch(gb))
}

fn clean_grundbuch(mut grundbuch: Grundbuch) -> Grundbuch {
    // BV-Nr: "." ->
    for zuschreibungen in grundbuch.bestandsverzeichnis.zuschreibungen.iter_mut() {
        zuschreibungen.bv_nr = clean_bv(&zuschreibungen.bv_nr);
    }
    for abschreibung in grundbuch.bestandsverzeichnis.abschreibungen.iter_mut() {
        abschreibung.bv_nr = clean_bv(&abschreibung.bv_nr);
    }

    for a in grundbuch.abt1.grundlagen_eintragungen.iter_mut() {
        a.bv_nr = clean_bv(&a.bv_nr);
    }
    for a in grundbuch.abt1.veraenderungen.iter_mut() {
        a.lfd_nr = clean_bv(&a.lfd_nr);
    }
    for a in grundbuch.abt1.loeschungen.iter_mut() {
        a.lfd_nr = clean_bv(&a.lfd_nr);
    }

    for a in grundbuch.abt2.eintraege.iter_mut() {
        a.bv_nr = clean_bv(&a.bv_nr);
    }
    for a in grundbuch.abt2.veraenderungen.iter_mut() {
        a.lfd_nr = clean_bv(&a.lfd_nr);
    }
    for a in grundbuch.abt2.loeschungen.iter_mut() {
        a.lfd_nr = clean_bv(&a.lfd_nr);
    }

    for a in grundbuch.abt3.eintraege.iter_mut() {
        a.bv_nr = clean_bv(&a.bv_nr);
    }
    for a in grundbuch.abt3.veraenderungen.iter_mut() {
        a.lfd_nr = clean_bv(&a.lfd_nr);
    }
    for a in grundbuch.abt3.loeschungen.iter_mut() {
        a.lfd_nr = clean_bv(&a.lfd_nr);
    }

    grundbuch
}

fn clean_bv(s: &StringOrLines) -> StringOrLines {
    match s {
        StringOrLines::MultiLine(s) => {
            StringOrLines::MultiLine(s.iter().map(|s| s.replace(".", ",")).collect())
        }
        StringOrLines::SingleLine(s) => StringOrLines::SingleLine(s.replace(".", ",")),
    }
}

lazy_static::lazy_static! {
    static ref REGEX_CACHE: Mutex<BTreeMap<String, CompiledRegex>> = Mutex::new(BTreeMap::new());
}

pub fn get_or_insert_regex(all_regex: &[String], regex: &str) -> Result<CompiledRegex, String> {
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
                .build()?,
        })
    }

    pub fn find_all_matches(&self, text: &str) -> Vec<String> {
        self.re
            .find_iter(text)
            .map(|m| m.as_str().to_string())
            .collect()
    }

    pub fn get_captures(&self, text: &str) -> Vec<String> {
        let cap = match self.re.captures_iter(text).next() {
            Some(c) => c,
            None => return Vec::new(),
        };

        cap.iter()
            .skip(1)
            .filter_map(|group| Some(group?.as_str().to_string()))
            .collect()
    }

    pub fn matches(&self, text: &str) -> bool {
        !self.get_captures(text).is_empty()
    }

    pub fn find_in(&self, text: &str, index: usize) -> Option<String> {
        self.get_captures(text).get(index).cloned()
    }

    pub fn find_all(&self, text: &str) -> Vec<String> {
        self.find_all_matches(text)
    }

    pub fn replace_all(&self, text: &str, text_neu: &str) -> String {
        self.re.replace_all(text, text_neu).to_string()
    }
}

fn teste_regex(regex_id: &str, text: &str, konfig: &Konfiguration) -> Result<Vec<String>, String> {
    let regex = match konfig.regex.get(regex_id) {
        Some(regex) => regex.clone(),
        None => return Err(format!("Regex-ID \"{}\" nicht gefunden.", regex_id)),
    };

    let compiled_regex =
        get_or_insert_regex(&konfig.regex.values().cloned().collect::<Vec<_>>(), &regex)?;

    Ok(compiled_regex.get_captures(text))
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "status")]
pub enum PdfFileOrEmpty {
    #[serde(rename = "ok")]
    Pdf(PdfFile),
    #[serde(rename = "error")]
    NichtVorhanden(PdfFileNichtVorhanden),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PdfFileNichtVorhanden {
    pub code: usize,
    pub text: String,
}

// Versucht eine Synchronisierung von  ~/.config/dgb/backup/XXX.gbx mit der Datenbank
fn try_download_file_database(
    konfiguration: Konfiguration,
    titelblatt: Titelblatt,
) -> Result<(), Option<String>> {
    let passwort = match konfiguration.get_passwort() {
        Some(s) => s,
        None => return Err(None),
    };

    let server_url = &konfiguration.server_url;
    let server_email = urlencoding::encode(&konfiguration.server_email);
    let download_id = format!(
        "{}/{}/{}",
        titelblatt.amtsgericht, titelblatt.grundbuch_von, titelblatt.blatt
    );
    let url =
        format!("{server_url}/download/gbx/{download_id}?email={server_email}&passwort={passwort}");

    let resp = reqwest::blocking::get(&url)
        .map_err(|e| Some(format!("Fehler beim Downloaden von {url}: {e}")))?;

    let json = resp
        .json::<PdfFileOrEmpty>()
        .map_err(|e| Some(format!("Ungültige Antwort: {e}")))?;

    match json {
        PdfFileOrEmpty::Pdf(mut json) => {
            let file_name = format!(
                "{}_{}",
                json.analysiert.titelblatt.grundbuch_von, json.analysiert.titelblatt.blatt
            );
            let target_folder_path = Path::new(&Konfiguration::backup_dir()).join("backup");
            if json.gbx_datei_pfad.is_some() {
                json.gbx_datei_pfad = Some(format!("{}", target_folder_path.display()));
            }
            if json.datei.is_some() {
                json.datei = Some(format!(
                    "{}",
                    target_folder_path
                        .join(&format!("{file_name}.pdf"))
                        .display()
                ));
            }
            let _ = std::fs::write(
                target_folder_path.join(&format!("{file_name}.gbx")),
                serde_json::to_string_pretty(&json).unwrap_or_default(),
            );
        }
        PdfFileOrEmpty::NichtVorhanden(err) => {
            let file_name = format!("{}_{}", titelblatt.grundbuch_von, titelblatt.blatt);
            konfiguration.create_empty_diff_save_point(&file_name);
            if err.code == 404 {
                return Ok(());
            } else {
                return Err(Some(format!("E{}: {}", err.code, err.text)));
            }
        }
    }

    Ok(())
}

fn get_program_path() -> Result<String, String> {
    Ok(std::env::current_exe()
        .map_err(|e| format!("{e}"))?
        .parent()
        .ok_or(format!("std::env::current_exe has no parent"))?
        .join("programs")
        .to_str()
        .unwrap_or_default()
        .to_string())
}

pub enum TesseractMode {
    Words,
    Numbers,
}

pub fn tesseract_get_hocr(image: &[u8]) -> Result<ParsedHocr, String> {
    use tesseract_static::tesseract::Tesseract;

    let dir = std::env::temp_dir().join("deu.traineddata");

    if !dir.exists() {
        let _ = std::fs::write(&dir, include_bytes!("../deu.traineddata"));
    }

    let hocr = Tesseract::new(
        Some(&std::env::temp_dir().display().to_string()),
        Some("deu"),
    )
    .unwrap()
    .set_image_from_mem(image)
    .unwrap()
    .get_hocr_text(1)
    .map_err(|e| format!("{e}"))?
    .to_string();

    ParsedHocr::new(&hocr).map_err(|e| format!("{e}"))
}

fn main() -> wry::Result<()> {
    use std::env;
    use wry::{
        application::{
            event::{Event, WindowEvent},
            event_loop::{ControlFlow, EventLoop},
            window::WindowBuilder,
        },
        webview::WebViewBuilder,
    };

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

    let _ = rayon::ThreadPoolBuilder::new()
        .num_threads(max_threads)
        .build_global();

    let original_value = env::var(GTK_OVERLAY_SCROLLING);
    env::set_var(GTK_OVERLAY_SCROLLING, "0"); // disable overlaid scrollbars

    let _ = Konfiguration::neu_laden();

    let mut userdata = RpcData::default();

    let initial_screen = ui::render_entire_screen(&mut userdata);

    let resizable = true;
    let debug = true;
    let app_html = include_str!("app.html")
        .to_string()
        .replace("<!-- REPLACED_ON_STARTUP -->", &initial_screen);

    let event_loop = EventLoop::with_user_event();
    let proxy = event_loop.create_proxy();
    let window = WindowBuilder::new()
        .with_title(APP_TITLE)
        .with_maximized(true)
        .build(&event_loop)?;

    let webview = WebViewBuilder::new(window)?
        .with_html(app_html)?
        .with_devtools(true)
        .with_navigation_handler(|s| s != "http://localhost/?") // ??? - bug?
        .with_ipc_handler(move |_window, cmd| match serde_json::from_str(&cmd) {
            Ok(o) => {
                let _ = proxy.send_event(o);
            }
            Err(e) => {
                println!("{e}");
            }
        })
        .build()?;

    webview.open_devtools();
    webview.focus();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;

                let _ = std::fs::remove_file(std::env::temp_dir().join("dgb").join("passwort.txt"));

                if let Ok(original_value) = original_value.as_ref() {
                    env::set_var(GTK_OVERLAY_SCROLLING, original_value);
                }
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                let _ = webview.resize();
            }
            Event::UserEvent(cmd) => {
                webview_cb(&webview, &cmd, &mut userdata);
            }
            _ => {}
        }
    });
}
