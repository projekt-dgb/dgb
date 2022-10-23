use std::fmt;
use std::path::{Path, PathBuf};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use crate::Konfiguration;
use serde::{Serialize, Deserialize};
use wasmer::{Store, Module, Instance};
use wasmer_wasi::{WasiFunctionEnv, WasiBidirectionalSharedPipePair, WasiState};
use wasmer_vfs::{FileSystem, mem_fs::FileSystem as MemFileSystem};

static PYTHON: &[u8] = include_bytes!("../bin/python.tar.gz");

#[derive(Debug, Clone, PartialEq, Ord, Eq, PartialOrd)]
pub enum DirOrFile {
    File(PathBuf),
    Dir(PathBuf),
}

pub type FileMap = BTreeMap<DirOrFile, Vec<u8>>;

#[derive(Debug, Clone)]
pub struct PyVm {
    // Kompiliertes python.wasm
    python_compiled_module: Vec<u8>,
    file_system: FileMap,
    script_result: Arc<Mutex<BTreeMap<String, PyResult>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "result", content = "data")]
enum PyResult {
    #[serde(rename = "ok")]
    Ok(PyOk),
    #[serde(rename = "err")]
    Err(PyError)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum PyOk {
    #[serde(rename = "str")]
    Str(String),
    #[serde(rename = "list")]
    List(Vec<String>),
    #[serde(rename = "spalte1")]
    Spalte1(Spalte1Eintraege),
    #[serde(rename = "rechteart")]
    RechteArt(RechteArt),
    #[serde(rename = "schuldenart")]
    SchuldenArt(SchuldenArt),
    #[serde(rename = "betrag")]
    Betrag(Betrag),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PyError {
    pub text: String,
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

impl PyVm {

    pub fn new() -> Result<Self, String> {

        println!("starting up PyVm...");

        let mut python_unpacked = unpack_tar_gz(PYTHON.to_vec(), "python/atom/").unwrap();
        let python_wasm = python_unpacked.remove(
            &DirOrFile::File(Path::new("lib/python.wasm").to_path_buf())
        ).expect("cannot find lib/python.wasm?");
        
        let mut store = Store::default();
        let mut module = Module::from_binary(&store, &python_wasm).unwrap();
        module.set_name("python");
        let bytes = module.serialize().unwrap();
        
        Ok(Self {
            python_compiled_module: bytes.to_vec(),
            file_system: python_unpacked,
            script_result: Arc::new(Mutex::new(BTreeMap::new())),
        })
    }

    pub fn execute_script(&self, konfiguration: &Konfiguration, args: ExecuteScriptType) -> Result<PyOk, PyError> {
        use std::io::Read;

        let key = get_script_cache_key(&konfiguration.regex, &args);

        println!("execute script {key}: {:#?}", args);

        match self.script_result.try_lock().ok().and_then(|lock| lock.get(&key).cloned()) {
            Some(PyResult::Ok(o)) => return Ok(o.clone()),
            Some(PyResult::Err(e)) => return Err(e.clone()),
            _ => { },
        }

        let generated = generate_script(konfiguration, &args);
        println!("{generated}");

        let mut python_unpacked = self.file_system.clone();
        python_unpacked.insert(
            DirOrFile::File(Path::new("lib/file.py").to_path_buf()), 
            generated.as_bytes().to_vec(),
        );

        let mut store = Store::default();
        let mut module = unsafe { Module::deserialize(
                &store, 
                self.python_compiled_module.clone()
            ) 
        }.map_err(|e| PyError {
            text: format!("failed to deserialize module: {e}")
        })?;
        
        module.set_name("python");
        
        let mut stdout_pipe = 
            WasiBidirectionalSharedPipePair::new()
            .with_blocking(false);
    
        let wasi_env = prepare_webc_env(
            &mut store, 
            stdout_pipe.clone(),
            &python_unpacked, 
            "python",
        ).map_err(|e| PyError {
            text: format!("{e}")
        })?;
    
        exec_module(&mut store, &module, wasi_env)
        .map_err(|e| PyError { text: format!("{e}") })?;

        let mut buf = Vec::new();
        stdout_pipe.read_to_end(&mut buf).map_err(|e| PyError {
            text: format!("failed to read pipe: {e}")
        })?;

        let result: PyResult = serde_json::from_slice(&buf)
        .map_err(|e| PyError {
            text: format!("serde_json from slice: {e}")
        })?;
        
        self.script_result.try_lock().ok()
        .and_then(|mut lock| {
            lock.insert(key, result.clone())
        });

        println!("{result:?}");

        match result {
            PyResult::Ok(o) => Ok(o),
            PyResult::Err(e) => Err(e),
        }
    }
}

/// Unzips a .tar.gz file, returning the [FileName => FileContents]
fn unpack_tar_gz(bytes: Vec<u8>, prefix: &str) -> Result<FileMap, String> {
    use flate2::read::GzDecoder;
    use std::io::Cursor;
    use tar::{Archive, EntryType};

    let mut cursor = Cursor::new(bytes);
    let mut archive = Archive::new(GzDecoder::new(cursor));

    // TODO(felix): it would be ideal if the .tar.gz file could
    // be unpacked in-memory instead of requiring disk access.

    // Use a random directory name for unpacking: in case the
    // tool is ran in parallel, this would otherwise lead to
    // file conflicts
    let rand_dir = rand::random::<u64>();
    let tempdir = std::env::temp_dir()
        .join("wapm-to-webc")
        .join(&format!("{rand_dir}"));

    let _ = std::fs::remove_dir(&tempdir); // no error if dir doesn't exist
    let _ = std::fs::create_dir_all(&tempdir)
        .map_err(|e| format!("{}: {e}", tempdir.display()))?;

    let mut files = BTreeMap::default();

    for (i, file) in archive.entries().unwrap().enumerate() {
        let mut file = file.map_err(|e| format!("{}: {e}", tempdir.display()))?;

        let file_type = file.header().entry_type();

        let path = file
            .path()
            .map_err(|e| format!("{}: {e}", tempdir.display()))?
            .to_owned()
            .to_path_buf();

        let outpath = tempdir.clone().join(&format!("{i}.bin"));

        let _ = file
            .unpack(&outpath)
            .map_err(|e| format!("{}: {e}", outpath.display()))?;

        let path = match file_type {
            EntryType::Regular => DirOrFile::File(path),
            EntryType::Directory => DirOrFile::Dir(path),
            e => {
                return Err(format!(
                    "Invalid file_type for path \"{}\": {:?}",
                    path.display(),
                    e
                ));
            }
        };

        let bytes = match &path {
            DirOrFile::File(_) => std::fs::read(&outpath)
                .map_err(|e| format!("{}: {e}", outpath.display()))?,
            DirOrFile::Dir(_) => Vec::new(),
        };



        let path = match &path {
            DirOrFile::File(f) => {
                if !format!("{}", f.display()).starts_with(prefix) {
                    continue;
                }
                // python/atom/lib/
                DirOrFile::File(
                    Path::new(&format!("{}", f.display())
                    .replacen(prefix, "", 1)
                ).to_path_buf())
            },
            DirOrFile::Dir(d) => {
                if !format!("{}", d.display()).starts_with(prefix) {
                    continue;
                }
                // python/atom/lib/
                DirOrFile::Dir(
                    Path::new(&format!("{}", d.display())
                    .replacen(prefix, "", 1)
                ).to_path_buf())
            }
        };

        files.insert(path, bytes);
    }

    nuke_dir(tempdir.as_path())?;

    Ok(files)
}

fn prepare_webc_env(
    store: &mut Store,
    stdout: WasiBidirectionalSharedPipePair,
    files: &FileMap,
    command: &str,
) -> Result<WasiFunctionEnv, String> {
    let fs = MemFileSystem::default();
    for key in files.keys() {
        match key {
            DirOrFile::Dir(d) => { 
                let mut s = format!("{}", d.display());
                if s.is_empty() { continue; }
                let s = format!("/{s}");
                let _ = fs.create_dir(Path::new(&s)); 
            },
            DirOrFile::File(f) => {

            },
        }
    }
    for (k, v) in files.iter() {
        match k {
            DirOrFile::Dir(d) => { continue; },
            DirOrFile::File(d) => { 
                let mut s = format!("{}", d.display());
                if s.is_empty() { continue; }
                let s = format!("/{s}");
                let mut file = fs
                    .new_open_options()
                    .read(true)
                    .write(true)
                    .create_new(true)
                    .create(true)
                    .open(&Path::new(&s))
                    .unwrap();
                
                file.write(&v).unwrap();
            },
        }
    }

    let mut wasi_env = WasiState::new(command);
    wasi_env.set_fs(Box::new(fs));

    for key in files.keys() {
        let mut s = match key {
            DirOrFile::Dir(d) => format!("{}", d.display()),
            DirOrFile::File(f) => continue,
        };
        if s.is_empty() { continue; }
        let s = format!("/{s}");
        wasi_env.preopen(|p| {
            p.directory(&s).read(true).write(true).create(true)
        })
        .map_err(|e| format!("E4: {e}"))?;
    }

    let mut wasi_env = wasi_env
    .env("PYTHONHOME", "/")
    .arg("/lib/file.py")
    .stdout(Box::new(stdout));

    Ok(
        wasi_env
        .finalize(store)
        .map_err(|e| format!("E5: {e}"))?    
    )
}

fn exec_module(
    store: &mut Store,
    module: &Module,
    mut wasi_env: wasmer_wasi::WasiFunctionEnv,
) -> Result<(), String> {

    let import_object = wasi_env.import_object(store, &module)
        .map_err(|e| format!("{e}"))?;
    let instance = Instance::new(store, &module, &import_object)
        .map_err(|e| format!("{e}"))?;
    let memory = instance.exports.get_memory("memory")
        .map_err(|e| format!("{e}"))?;
    wasi_env.data_mut(store).set_memory(memory.clone());

    // If this module exports an _initialize function, run that first.
    if let Ok(initialize) = instance.exports.get_function("_initialize") {
        initialize
            .call(store, &[])
            .map_err(|e| format!("failed to run _initialize function: {e}"))?;
    }

    let result = instance.exports
        .get_function("_start")
        .map_err(|e| format!("{e}"))?
        .call(store, &[])
        .map_err(|e| format!("{e}"))?;

        Ok(())
}

fn nuke_dir(path: &Path) -> Result<(), String> {
    use std::fs;
    for entry in
        fs::read_dir(path).map_err(|e| format!("{}: {e}", path.display()))?
    {
        let entry = entry.map_err(|e| format!("{}: {e}", path.display()))?;
        let path = entry.path();

        let file_type = entry
            .file_type()
            .map_err(|e| format!("{}: {e}", path.display()))?;

        if file_type.is_dir() {
            nuke_dir(&path)?;
            fs::remove_dir(&path)
                .map_err(|e| format!("{}: {e}", path.display()))?;
        } else {
            fs::remove_file(&path)
                .map_err(|e| format!("{}: {e}", path.display()))?;
        }
    }

    Ok(())
}

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

impl Default for Betrag {
    fn default() -> Self {
        Betrag {
            wert: 0,
            nachkomma: 0,
            waehrung: Waehrung::Euro,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spalte1Eintrag {
    // Nummer im BV
    pub lfd_nr: usize,
    // "Teil von", "Teil v.", "X tlw."
    pub voll_belastet: bool,    
    // Leer = gesamte lfd. Nr. ist belastet
    pub nur_lastend_an: Vec<FlurFlurstueck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spalte1Eintraege {
    pub eintraege: Vec<Spalte1Eintrag>,
    pub warnungen: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

pub fn text_saubern(
    vm: PyVm, 
    rechtstext: &str, 
    konfiguration: &Konfiguration
) -> Result<String, String> {

    let result = vm.execute_script(
        konfiguration, 
        ExecuteScriptType::TextSaubern { 
            recht: rechtstext.to_string() 
        }
    ).map_err(|e| format!("{:?}", e))?;

    match result {
        PyOk::Str(s) => Ok(s),
        e => Err(format!("{:?}", e)),
    }
}

pub fn get_belastete_flurstuecke(
    vm: PyVm, 
	bv_nr: &str, 
	text_sauber: &str, 
	konfiguration: &Konfiguration,
) -> Result<Spalte1Eintraege, String> {
    
    let result = vm.execute_script(
        konfiguration, 
        ExecuteScriptType::FlurstueckeAuslesen { 
            spalte1: bv_nr.to_string(), 
            text: text_sauber.to_string() 
        }
    ).map_err(|e| format!("{:?}", e))?;

    match result {
        PyOk::Spalte1(s) => Ok(s),
        e => Err(format!("{:?}", e)),
    }
}

pub fn get_abkuerzungen(
    vm: PyVm,
    konfiguration: &Konfiguration,
) -> Result<Vec<String>, String> {
    
    let result = vm.execute_script(
        konfiguration, 
        ExecuteScriptType::GetAbkuerzungen,
    ).map_err(|e| format!("{:?}", e))?;
    
    match result {
        PyOk::List(s) => Ok(s),
        e => Err(format!("{:?}", e)),
    }
}

pub fn get_rechte_art_abt2(
    vm: PyVm,
    recht_id: &str,
    text_sauber: &str,
    saetze_clean: &[String],
    konfiguration: &Konfiguration,
) -> Result<RechteArt, String> {
    let result = vm.execute_script(
        konfiguration, 
        ExecuteScriptType::KlassifiziereRechteArtAbt2 { 
            saetze: saetze_clean.to_vec()  
        }
    ).map_err(|e| format!("{:?}", e))?;
    
    match result {
        PyOk::RechteArt(s) => Ok(s),
        e => Err(format!("{:?}", e)),
    }
}

pub fn get_rangvermerk_abt2(
    vm: PyVm,
    recht_id: &str,
    text_sauber: &str,
    saetze_clean: &[String],
    konfiguration: &Konfiguration,
) -> Result<String, String> {
    let result = vm.execute_script(
        konfiguration, 
        ExecuteScriptType::RangvermerkAuslesen { 
            saetze: saetze_clean.to_vec() 
        }
    ).map_err(|e| format!("{:?}", e))?;
    
    match result {
        PyOk::Str(s) => Ok(s),
        e => Err(format!("{:?}", e)),
    }
}

pub fn get_rechtsinhaber_abt2(
    vm: PyVm,
    recht_id: &str,
    text_sauber: &str,
    saetze_clean: &[String],
    konfiguration: &Konfiguration,
) -> Result<String, String> {
    let result = vm.execute_script(konfiguration, ExecuteScriptType::RechtsinhaberAuslesenAbt2 { 
        saetze: saetze_clean.to_vec() 
    }).map_err(|e| format!("{:?}", e))?;
    
    match result {
        PyOk::Str(s) => Ok(s),
        e => Err(format!("{:?}", e)),
    }
}

pub fn get_betrag_abt3(
    vm: PyVm,
    recht_id: &str,
    text_sauber: &str,
    saetze_clean: &[String],
    konfiguration: &Konfiguration,
) -> Result<Betrag, String> {
    let result = vm.execute_script(
        konfiguration,
        ExecuteScriptType::BetragAuslesen { 
            saetze: saetze_clean.to_vec() 
        }
    ).map_err(|e| format!("{:?}", e))?;
    
    match result {
        PyOk::Betrag(s) => Ok(s),
        e => Err(format!("{:?}", e)),
    }
}

pub fn get_schulden_art_abt3(
    vm: PyVm,
    recht_id: &str,
    text_sauber: &str,
    saetze_clean: &[String],
    konfiguration: &Konfiguration,
) -> Result<SchuldenArt, String> {
    
    let result = vm.execute_script(
        konfiguration,
        ExecuteScriptType::KlassifiziereSchuldenArtAbt3 { 
            saetze: saetze_clean.to_vec() 
        }
    ).map_err(|e| format!("{:?}", e))?;

    match result {
        PyOk::SchuldenArt(s) => Ok(s),
        e => Err(format!("{:?}", e)),
    }
}

pub fn get_rechtsinhaber_abt3(
    vm: PyVm,
    recht_id: &str,
    text_sauber: &str,
    saetze_clean: &[String],
    konfiguration: &Konfiguration,
) -> Result<String, String> {
    let result = vm.execute_script(
        konfiguration,
        ExecuteScriptType::RechtsinhaberAuslesenAbt3 { 
            saetze: saetze_clean.to_vec(), 
            recht_id: recht_id.to_string() 
        }
    ).map_err(|e| format!("{:?}", e))?;

    match result {
        PyOk::Str(s) => Ok(s),
        e => Err(format!("{:?}", e)),
    }
}

pub fn get_kurztext_abt2(
    vm: PyVm,
    recht_id: &str,
    text_sauber: &str,
    rechtsinhaber: Option<String>,
    rangvermerk: Option<String>,
    saetze_clean: &[String],
    konfiguration: &Konfiguration,
) -> Result<String, String> {

    let result = vm.execute_script(
        &konfiguration, 
        ExecuteScriptType::TextKuerzenAbt2 { 
            saetze: saetze_clean.to_vec(), 
            rechtsinhaber: rechtsinhaber.unwrap_or_default(), 
            rangvermerk: rangvermerk.unwrap_or_default(),
        }).map_err(|e| format!("{:?}", e))?;

    match result {
        PyOk::Str(s) => Ok(s),
        e => Err(format!("{:?}", e)),
    }
}

pub fn get_kurztext_abt3(
    vm: PyVm,
    recht_id: &str,
    text_sauber: &str,
    betrag: Option<String>,
    schuldenart: Option<String>,
    rechtsinhaber: Option<String>,
    saetze_clean: &[String],
    konfiguration: &Konfiguration,
) -> Result<String, String> {

    let result = vm.execute_script(
        konfiguration,
        ExecuteScriptType::TextKuerzenAbt3 { 
            saetze: saetze_clean.to_vec(), 
            betrag: betrag.unwrap_or_default(), 
            schuldenart: schuldenart.unwrap_or_default(), 
            rechtsinhaber: rechtsinhaber.unwrap_or_default(),
    }).map_err(|e| format!("{:?}", e))?;

    match result {
        PyOk::Str(s) => Ok(s),
        e => Err(format!("{:?}", e)),
    }
}

/*
fn execute_script_string(
    script_id: &str,
    vm: PyVm,
    recht_id: &str,
    text_sauber: &str,
    saetze_clean: &[String],
    konfiguration: &Konfiguration,
    script: &[String], 
) -> Result<String, String> {
    match execute_script_pyok(
        script_id,
        vm,
        recht_id,
        text_sauber,
        saetze_clean,
        konfiguration,
        script,
    )? {
        PyOk::Str(s) => Ok(s),
        e => Err(format!("{:?}", e)),
    }
}

fn execute_script_pyok(
    script_id: &str,
    vm: PyVm,
    recht_id: &str,
    text_sauber: &str,
    saetze_clean: &[String],
    konfiguration: &Konfiguration,
    script: &[String], 
) -> Result<PyOk, String> {

    let script = script.join("\r\n");
    let script = script.replace("\t", "    ");
    let script = script.replace("\u{00a0}", " ");
    let script = script.lines().map(|s| s.to_string()).collect::<Vec<_>>();

    let result = vm.execute_script(&script, &[
        script_id,
        &serde_json::json!({
            "recht": recht_id,
            "text": text_sauber,
            "saetze": saetze_clean,
            "re": konfiguration.regex,
        }).to_string(),
    ]).map_err(|e| format!("{:?}", e))?;

    Ok(result)
}
*/
#[test]
fn test_pym_script_1() {
    let vm = PyVm::new().unwrap();
    let konfiguration = Konfiguration {
        text_saubern_script: vec![
            "return \"hello\"".to_string()
        ],
        .. Konfiguration::parse_from(Konfiguration::DEFAULT).unwrap()
    };
    let args = ExecuteScriptType::TextSaubern { 
        recht: String::new(), 
    };
    let ok = vm.execute_script(&konfiguration, args).unwrap();
}

pub type RegexMap = BTreeMap<String, String>;

#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
pub enum ExecuteScriptType {
    // text_saubern(recht: String, re: [String -> Regex]) -> String
    TextSaubern {
        recht: String, 
    },
    // abkuerzungen(re: [String -> Regex]) -> [String]
    GetAbkuerzungen,
    // flurstuecke_auslesen(spalte_1: String, text: String, re: [String -> Regex]) -> Spalte1Eintrag
    FlurstueckeAuslesen {
        spalte1: String,
        text: String,
    },
    // klassifiziere_rechteart_abt2(saetze: [String], re: [String -> Regex]) -> RechteArt
    KlassifiziereRechteArtAbt2 {
        saetze: Vec<String>,
    },
    // rechtsinhaber_auslesen_abt2(saetze: [String], re: [String -> Regex], recht_id: String) -> String
    RechtsinhaberAuslesenAbt2 {
        saetze: Vec<String>,
    },
    // rangvermerk_auslesen_abt2(saetze: [String], re: [String -> Regex]) -> String
    RangvermerkAuslesen {
        saetze: Vec<String>,
    },
    // text_kuerzen_abt2(saetze: [String], rechtsinhaber: String, rangvermerk: String, re: [String -> Regex]) -> String
    TextKuerzenAbt2 {
        saetze: Vec<String>,
        rechtsinhaber: String,
        rangvermerk: String,
    },
    // betrag_auslesen(saetze: [String], re: [String -> Regex]) -> Betrag
    BetragAuslesen {
        saetze: Vec<String>,
    },
    // klassifizere_schuldenart_abt3(saetze: [String], re: [String -> Regex]) - SchuldenArt
    KlassifiziereSchuldenArtAbt3 {
        saetze: Vec<String>,
    },
    // rechtsinhaber_auslesen_abt3(saetze: [String], re: [String -> Regex], recht_id: String) -> String
    RechtsinhaberAuslesenAbt3 {
        saetze: Vec<String>,
        recht_id: String,
    },
    // text_kuerzen_abt3(saetze: [String], betrag: String, schuldenart: String, rechtsinhaber: String, re: [String -> Regex]) -> String
    TextKuerzenAbt3 {
        saetze: Vec<String>,
        betrag: String,
        schuldenart: String,
        rechtsinhaber: String, 
    }
}

fn generate_script(konfiguration: &Konfiguration, script: &ExecuteScriptType) -> String {
    static WRAPPER: &str = include_str!("./wrapper.py");

    let script_args = match script {
        ExecuteScriptType::TextSaubern { recht, .. } => {
            let mut s = format!("recht = \"\n\".join([\n");
            for l in recht.lines() {
                s.push_str(&format!("    {:?},\n", l));
            }
            s.push_str("])\n\n");
            s
        },
        ExecuteScriptType::GetAbkuerzungen { .. } => String::new(),
        ExecuteScriptType::FlurstueckeAuslesen { spalte1, text, .. } => {
            let mut s = String::new();
            s.push_str(&format!("spalte_1 = \"\\n\".join([\n"));
            for l in spalte1.lines() {
                s.push_str(&format!("    {:?},\n", l));
            }
            s.push_str("])\n\n");
            s.push_str(&format!("text = \"\\n\".join([\n"));
            for l in text.lines() {
                s.push_str(&format!("    {:?},\n", l));
            }
            s.push_str("])\n\n");
            s
        },
        ExecuteScriptType::KlassifiziereRechteArtAbt2 { saetze, .. } => {
            let mut s = String::new();
            s.push_str(&format!("saetze = [\n"));
            for l in saetze {
                s.push_str(&format!("    {:?},\n", l));
            }
            s.push_str("]\n\n");
            s
        },
        ExecuteScriptType::RechtsinhaberAuslesenAbt2 { saetze, .. } => {
            let mut s = String::new();
            s.push_str(&format!("saetze = [\n"));
            for l in saetze {
                s.push_str(&format!("    {:?},\n", l));
            }
            s.push_str("]\n\n");
            s
        },
        ExecuteScriptType::RangvermerkAuslesen { saetze, .. } => {
            let mut s = String::new();
            s.push_str(&format!("saetze = [\n"));
            for l in saetze {
                s.push_str(&format!("    {:?},\n", l));
            }
            s.push_str("]\n\n");
            s
        },
        ExecuteScriptType::TextKuerzenAbt2 { 
            saetze, 
            rechtsinhaber, 
            rangvermerk, 
            .. 
        } => {
            let mut s = String::new();

            s.push_str(&format!("saetze = [\n"));
            for l in saetze {
                s.push_str(&format!("    {:?},\n", l));
            }
            s.push_str("]\n\n");

            s.push_str(&format!("rechtsinhaber = \"\\n\".join([\n"));
            for l in rechtsinhaber.lines() {
                s.push_str(&format!("    {:?},\n", l));
            }
            s.push_str("])\n\n");

            s.push_str(&format!("rangvermerk = \"\\n\".join([\n"));
            for l in rangvermerk.lines() {
                s.push_str(&format!("    {:?},\n", l));
            }
            s.push_str("])\n\n");

            s
        },
        ExecuteScriptType::BetragAuslesen { saetze, .. } => {
            let mut s = String::new();

            s.push_str(&format!("saetze = \"\\n\".join([\n"));
            for l in saetze {
                s.push_str(&format!("    {:?},\n", l));
            }
            s.push_str("])\n\n");

            s
        },
        ExecuteScriptType::KlassifiziereSchuldenArtAbt3 { saetze, .. } => {
            let mut s = String::new();

            s.push_str(&format!("saetze = \"\\n\".join([\n"));
            for l in saetze {
                s.push_str(&format!("    {:?},\n", l));
            }
            s.push_str("])\n\n");

            s
        },
        ExecuteScriptType::TextKuerzenAbt3 { 
            saetze, 
            betrag, 
            schuldenart, 
            rechtsinhaber, 
            .. 
        } => {

            let mut s = String::new();

            s.push_str(&format!("saetze = [\n"));
            for l in saetze {
                s.push_str(&format!("    {:?},\n", l));
            }
            s.push_str("]\n\n");

            s.push_str(&format!("betrag = \"{betrag}\"\n\n"));
            s.push_str(&format!("schuldenart = \"{schuldenart}\"\n\n"));
            s.push_str(&format!("rechtsinhaber = \"{rechtsinhaber}\"\n\n"));

            s
        },
        ExecuteScriptType::RechtsinhaberAuslesenAbt3 { saetze, recht_id, .. } => {
            
            let mut s = String::new();

            s.push_str(&format!("saetze = [\n"));
            for l in saetze {
                s.push_str(&format!("    {:?},\n", l));
            }
            s.push_str("]\n\n");

            s.push_str(&format!("recht_id = \"{recht_id}\"\n\n"));

            s
        }
    }
    .lines()
    .map(|l| {
        format!("    {}", l)
        .replace("\t", "    ")
        .replace("\u{00a0}", " ")
    })
    .collect::<Vec<_>>()
    .join("\n");

    WRAPPER
    // .replace("## SCRIPT_ARGS", &script_args)
    .replace("## REGEX_LIST", &{
        let mut s = format!("re = {{}}\n");
        for (k, v) in konfiguration.regex.iter() {
            let k = k
                .replace("\"", "")
                .replace("\n", "")
                .replace("\r\n", "")
                .replace("'", "\'");
            let v = v
                .replace("\n", "")
                .replace("\r\n", "");
            s.push_str(&format!("re[\"{k}\"] = Regex(r'{v}')\n"));
        }
        s
    })
    .replace("## MAIN_SCRIPT", &format!("{script_args}\n{}", match script {
        ExecuteScriptType::TextSaubern { .. } => &konfiguration.text_saubern_script,
        ExecuteScriptType::GetAbkuerzungen { .. } => &konfiguration.abkuerzungen_script,
        ExecuteScriptType::FlurstueckeAuslesen { .. } => &konfiguration.flurstuecke_auslesen_script,
        ExecuteScriptType::KlassifiziereRechteArtAbt2 { .. } => &konfiguration.klassifiziere_rechteart,
        ExecuteScriptType::RechtsinhaberAuslesenAbt2 { .. } => &konfiguration.rechtsinhaber_auslesen_abt2_script,
        ExecuteScriptType::RangvermerkAuslesen { .. } => &konfiguration.rangvermerk_auslesen_abt2_script,
        ExecuteScriptType::TextKuerzenAbt2 { .. } => &konfiguration.text_kuerzen_abt2_script,
        ExecuteScriptType::BetragAuslesen { .. } => &konfiguration.betrag_auslesen_script,
        ExecuteScriptType::KlassifiziereSchuldenArtAbt3 { .. } => &konfiguration.klassifiziere_schuldenart,
        ExecuteScriptType::TextKuerzenAbt3 { .. } => &konfiguration.text_kuerzen_abt3_script,
        ExecuteScriptType::RechtsinhaberAuslesenAbt3 { .. } => &konfiguration.rechtsinhaber_auslesen_abt3_script,
    }
    .iter()
    .map(|l| {
        format!("    {}", l)
        .replace("\t", "    ")
        .replace("\u{00a0}", " ")
    })
    .collect::<Vec<_>>()
    .join("\n")))
}

// Um das Ergebnis eines einzelnen Scripts nicht unnötig wiederholen zu müssen,
// speichert die VM das Ergebnis des Scripts mit dem Schlüssel der Inputs,
// da f(x) -> y immer den gleichen Wert ergibt für denselben Input x
fn get_script_cache_key(regex: &RegexMap, script: &ExecuteScriptType) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(serde_json::to_string(script).unwrap_or_default().as_bytes());
    hasher.update(serde_json::to_string(regex).unwrap_or_default().as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}
