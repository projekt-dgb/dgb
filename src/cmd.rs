use serde::{Deserialize, Serialize};

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
    KlassifiziereSeiteNeu {
        seite: usize,
        klassifikation_neu: String,
    },
    #[serde(rename = "check_pdf_image_sichtbar")]
    CheckPdfImageSichtbar,
    #[serde(rename = "toggle_lefis_analyse")]
    ToggleLefisAnalyse,
    #[serde(rename = "check_pdf_for_errors")]
    CheckPdfForErrors,
    #[serde(rename = "search")]
    Search { search_text: String },
    #[serde(rename = "grundbuch_abonnieren")]
    GrundbuchAbonnieren { download_id: String },
    #[serde(rename = "download_gbx")]
    DownloadGbx { download_id: String },
    #[serde(rename = "upload_gbx")]
    UploadGbx,
    #[serde(rename = "edit_abkuerzungen_script")]
    EditAbkuerzungenScript { script: String },
    #[serde(rename = "edit_text_saubern_script")]
    EditTextSaubernScript { script: String },
    #[serde(rename = "edit_flurstuecke_auslesen_script")]
    EditFlurstueckeAuslesenScript { script: String },
    #[serde(rename = "edit_commit_description")]
    EditCommitDescription { value: String },
    #[serde(rename = "edit_commit_title")]
    EditCommitTitle { value: String },
    #[serde(rename = "edit_konfiguration_textfield")]
    EditKonfigurationTextField { id: String, value: String },
    #[serde(rename = "edit_konfiguration_schluesseldatei")]
    EditKonfigurationSchluesseldatei { base64: String },

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
    #[serde(rename = "switch_aenderung_view")]
    SwitchAenderungView { i: usize },

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
    CheckForPdfLoaded {
        file_path: String,
        file_name: String,
    },
    // Check whether a "{file_name}".json with analyzed texts exists
    #[serde(rename = "check_for_image_loaded")]
    CheckForImageLoaded {
        file_path: String,
        file_name: String,
    },

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
    ZeileLoeschen {
        file: String,
        page: usize,
        zeilen_id: usize,
    },
    #[serde(rename = "bv_eintrag_typ_aendern")]
    BvEintragTypAendern { path: String, value: String },

    #[serde(rename = "copy_text_to_clipboard")]
    CopyTextToClipboard { text: String },
    #[serde(rename = "save_state")]
    SaveState,

    // UI stuff
    #[serde(rename = "set_active_ribbon_tab")]
    SetActiveRibbonTab { new_tab: usize },
    #[serde(rename = "set_open_file")]
    SetOpenFile { new_file: String },
    #[serde(rename = "set_open_page")]
    SetOpenPage { active_page: u32 },
    #[serde(rename = "signal_pdf_page_rendered")]
    SignalPdfPageRendered {
        pdf_amtsgericht: String,
        pdf_grundbuch_von: String,
        pdf_blatt: String,
        seite: usize,
        geroetet: bool,
        image_data_base64: String,
        image_filename: String,
    },
}
