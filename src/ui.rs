use crate::{
    RpcData, PdfFile, 
    Konfiguration,
    digitalisiere::{
        Nebenbeteiligter,
        BvZuschreibung,
        BvAbschreibung,
        Abt1Veraenderung,
        Abt1Loeschung,
        Abt2Veraenderung,
        Abt2Loeschung,
        Abt3Veraenderung,
        Abt3Loeschung,
    },
};

// render entire <body> node depending on the state of the rpc_data
pub fn render_entire_screen(rpc_data: &mut RpcData) -> String {
    normalize_for_js(format!("
            {popover}
            {ribbon_ui}
            <div id='__application-main' style='overflow:hidden;'>
                {main}
            </div>
        ",
        popover = render_popover(rpc_data),
        ribbon_ui = render_ribbon(rpc_data),
        main = render_main(rpc_data),
    ))
}

pub fn render_popover(rpc_data: &RpcData) -> String {
        
    let should_render_popover = 
        rpc_data.configuration_active ||
        rpc_data.info_active ||
        rpc_data.context_menu_active.is_some();
    
            
        
    if !should_render_popover {
        return normalize_for_js(format!("<div id='__application_popover' style='
            pointer-events:none;
            width: 100%;
            height: 100%;
            min-height: 100%;
            position: fixed;
            z-index:999;
        '></div>"));
    }
    
    let popover = format!("<div id='__application_popover' style='
        pointer-events:none;
        width: 100%;
        height: 100%;
        min-height: 100%;
        position: fixed;
        z-index:999;
    '>{}</div>", 
        render_popover_content(rpc_data)
    );
    
    normalize_for_js(popover)
}

pub fn render_popover_content(rpc_data: &RpcData) -> String {

    const ICON_CLOSE: &[u8] = include_bytes!("./img/icons8-close-48.png");

    let application_popover_color = if rpc_data.configuration_active || rpc_data.info_active {
        "rgba(0, 0, 0, 0.5)"
    } else {
        "transparent"
    };
    
    let icon_close_base64 = base64::encode(ICON_CLOSE);

    let pc = if rpc_data.info_active {
        format!("
        <div style='width:800px;display:flex;flex-direction:column;margin:10px auto;border:1px solid grey;background:white;padding:10px;' onmousedown='event.stopPropagation();'>
                <h2 style='font-size:24px;font-family:sans-serif;'>Digitales Grundbuch Version {version}</h2>
                
                <div style='padding:5px 0px;display:flex;flex-grow:1;'>
                    <iframe width='100%' height='100%' src='data:text/html;base64,{license_base64}'></iframe>                       
                </div>
            </div>
        ",version = env!("CARGO_PKG_VERSION"),
        license_base64 = base64::encode(include_bytes!("../licenses.html")))
    } else if rpc_data.configuration_active {
        format!("
            <div style='pointer-events:unset;width:1200px;overflow:scroll;display:flex;flex-direction:column;margin:10px auto;border:1px solid grey;background:white;padding:10px 100px;' onmousedown='event.stopPropagation();'>
                <h2 style='font-size:20px;padding-bottom:10px;font-family:sans-serif;'>Konfiguration</h2>
                <p style='font-size:12px;padding-bottom:5px;'>Pfad: {konfig_pfad}</p>
                
                <div style='padding:5px 0px;'>
                        <div style='display:flex;flex-direction:row;'>
                        <input type='checkbox' id='__application_konfiguration_spalten_ausblenden' {spalten_ausblenden} data-checkBoxId='konfiguration-spalten-ausblenden' onchange='toggleCheckbox(event)'>
                        <label for='__application_konfiguration_spalten_ausblenden'>Spalten ausblenden</label>
                        </div>
                        
                        <div style='display:flex;flex-direction:row;'>
                        <input type='checkbox' id='__application_konfiguration_zeilenumbrueche-in-ocr-text' data-checkBoxId='konfiguration-zeilenumbrueche-in-ocr-text' {zeilenumbrueche_in_ocr_text} onchange='toggleCheckbox(event)'>
                        <label for='__application_konfiguration_zeilenumbrueche-in-ocr-text'>Beim Kopieren von OCR-Text Zeilenumbrüche beibehalten</label>
                        </div>
                        
                        <div style='display:flex;flex-direction:row;'>
                        <input type='checkbox' id='__application_konfiguration_hide_red_lines' data-checkBoxId='konfiguration-keine-roten-linien' {vorschau_ohne_geroetet} onchange='toggleCheckbox(event)'>
                        <label for='__application_konfiguration_hide_red_lines'>PDF ohne geröteten Linien darstellen</label>
                        </div>
                </div>
                <div style='padding:5px 0px;'>

                    <div>
                        <p style='font-family:sans-serif;font-weight:bold;font-size:16px;padding-bottom:10px;'>Reguläre Ausdrücke</p>                        
                    </div>
                    
                    <div style='background:white;border:1px solid #efefef;margin-top:5px;font-weight:bold;font-size:14px;font-family:monospace;color:black;padding:10px;overflow-y:scroll;'>
                    {regex}
                    </div>

                    <div style='display:flex;flex-direction:row;'>
                    <input id='__application_konfiguration_regex_id' style='border-radius:5px;padding:5px;border:1px solid #efefef;' style='flex-grow:1;margin-right:10px;' placeholder='Regex ID'></input>
                        <textarea id='__application_konfiguration_regex_test_text' style='border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' oninput='testeRegex(event);'style='flex-grow:1;margin-right:10px;' placeholder='Test Eingabe...'></textarea>
                        <textarea style='border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' id='__application_konfiguration_regex_test_output' style='flex-grow:1;' placeholder='Regex Ausgabe'></textarea>
                    </div>
                </div>
                
                
                <div style='padding:5px 0px;'>
                
                    <div>
                        <p style='font-family:sans-serif;font-weight:bold;font-size:16px;padding-bottom:10px;'>Abkürzungen</p>                        
                    </div>
                    
                    <div style='background:white;border:1px solid #efefef;margin-top:5px;font-weight:bold;font-size:14px;font-family:monospace;color:black;padding:10px;max-height:200px;overflow-y:scroll;'>
                        <p style='color:#4a4e6a;user-select:none;'>def abkuerzungen() -> [String]:</p>
                        <div style='padding-left:34px;caret-color: #4a4e6a;' contenteditable='true' onkeydown='insertTabAtCaret(event);' oninput='editAbkuerzungenScript(event);'>{konfig_abkuerzungen_script}</div>
                    </div>
                </div>
                
                <div style='padding:5px 0px;'>
                
                    <div>
                        <p style='font-family:sans-serif;font-weight:bold;font-size:16px;padding-bottom:10px;'>Text säubern</p>                        
                    </div>
                    
                    <div style='background:white;border:1px solid #efefef;margin-top:5px;font-weight:bold;font-size:14px;font-family:monospace;color:black;padding:10px;max-height:200px;overflow-y:scroll;'>
                        <p style='color:#4a4e6a;user-select:none;'>def text_säubern(recht: String) -> String:</p>
                        <div style='padding-left:34px;caret-color: #4a4e6a;' contenteditable='true' onkeydown='insertTabAtCaret(event);' oninput='editTextSaubernScript(event);'>{konfig_text_saubern_script}</div>
                    </div>
                </div>                    
                
                <div style='padding:5px 0px;'>
                
                    <div>
                        <p style='font-family:sans-serif;font-weight:bold;font-size:16px;padding-bottom:10px;'>Flurstücke aus Spalte 1 auslesen</p>                        
                    </div>
                    
                    <div style='background:white;border:1px solid #efefef;margin-top:5px;font-weight:bold;font-size:14px;font-family:monospace;color:black;padding:10px;max-height:200px;overflow-y:scroll;'>
                        <p style='color:#4a4e6a;user-select:none;'>def flurstuecke_auslesen(spalte_1: String, text: String, re: Mapping[String, Regex]) -> [Spalte1Eintrag]:</p>
                        <div style='padding-left:34px;caret-color: #4a4e6a;' contenteditable='true' onkeydown='insertTabAtCaret(event);' oninput='editFlurstueckeAuslesenScript(event);'>{konfig_flurstuecke_auslesen_script}</div>
                    </div>
                </div>     
                
                <div style='padding:5px 0px;'>
                
                    <div>
                        <p style='font-family:sans-serif;font-weight:bold;font-size:16px;padding-bottom:10px;'>
                            Klassifizierung RechteArt (Abteilung 2)
                        </p>                        
                    </div>
                    
                    <div style='background:white;border:1px solid #efefef;margin-top:5px;font-weight:bold;font-size:14px;font-family:monospace;color:black;padding:10px;max-height:200px;overflow-y:scroll;'>
                        <p style='color:#4a4e6a;user-select:none;'>def klassifiziere_rechteart_abt2(saetze: [String], re: Mapping[String, Regex]) -> RechteArt:</p>
                        <div style='padding-left:34px;caret-color: #4a4e6a;'contenteditable='true' onkeydown='insertTabAtCaret(event);' oninput='editRechteArtScript(event);'>{konfig_rechteart_script}</div>
                    </div>
                    
                    <div style='display:flex;flex-direction:row;'>
                        <textarea style='border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' oninput='rechteArtScriptTesten(event);'style='flex-grow:1;margin-right:10px;' placeholder='Test Eingabe...'></textarea>
                        <textarea style='border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' id='__application_konfiguration_rechteart_test' style='flex-grow:1;' placeholder='Test Ausgabe der Funktion'></textarea>
                    </div>
                </div>
                
                <div style='padding:5px 0px;'>
                
                    <div>
                        <p style='font-family:sans-serif;font-weight:bold;font-size:16px;padding-bottom:10px;'>
                            Rechtsinhaber auslesen (Abteilung 2)
                        </p>                        
                    </div>
                    
                    <div style='background:white;border:1px solid #efefef;margin-top:5px;font-weight:bold;font-size:14px;font-family:monospace;color:black;padding:10px;max-height:200px;overflow-y:scroll;'>
                        <p style='color:#4a4e6a;user-select:none;'>def rechtsinhaber_auslesen_abt2(saetze: [String], re: Mapping[String, Regex]) -> String:</p>
                        <div style='padding-left:34px;caret-color: #4a4e6a;' contenteditable='true' onkeydown='insertTabAtCaret(event);' oninput='editRechtsinhaberAbt2Script(event);'>{konfig_rechtsinhaber_abt2_script}</div>
                    </div>
                    
                    <div style='display:flex;flex-direction:row;'>
                        <textarea style='border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' oninput='rechtsinhaberAbt2ScriptTesten(event);'style='flex-grow:1;margin-right:10px;' placeholder='Test Eingabe...'></textarea>
                        <textarea style='border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' id='__application_konfiguration_rechtsinhaber_abt2_test' style='flex-grow:1;' placeholder='Test Ausgabe der Funktion'></textarea>
                    </div>
                </div>
                            
                <div style='padding:5px 0px;'>
                
                    <div>
                        <p style='font-faspalten_ausblendenmily:sans-serif;font-weight:bold;font-size:16px;padding-bottom:10px;'>
                            Rangvermerk auslesen (Abteilung 2)
                        </p>                        
                    </div>
                    
                    <div style='background:white;border:1px solid #efefef;margin-top:5px;font-weight:bold;font-size:14px;font-family:monospace;color:black;padding:10px;max-height:200px;overflow-y:scroll;'>
                        <p style='color:#4a4e6a;user-select:none;'>def rangvermerk_auslesen_abt2(saetze: [String], re: Mapping[String, Regex]) -> String:</p>
                        <div style='padding-left:34px;caret-color: #4a4e6a;' contenteditable='true' onkeydown='insertTabAtCaret(event);' oninput='editRangvermerkAuslesenAbt2Script(event);'>{konfig_rangvermerk_abt2_script}</div>
                    </div>
                    
                    <div style='display:flex;flex-direction:row;'>
                        <textarea style='border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' oninput='rangvermerkAuslesenAbt2ScriptTesten(event);'style='flex-grow:1;margin-right:10px;' placeholder='Test Eingabe...'></textarea>
                        <textarea style='border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' id='__application_konfiguration_rangvermerk_auslesen_abt2_test' style='flex-grow:1;' placeholder='Test Ausgabe der Funktion'></textarea>
                    </div>
                </div>
                
                <div style='padding:5px 0px;'>
                
                    <div>
                        <p style='font-family:sans-serif;font-weight:bold;font-size:16px;padding-bottom:10px;'>
                            Text kürzen (Abteilung 2)
                        </p>                        
                    </div>
                    
                    <div style='background:white;border:1px solid #efefef;margin-top:5px;font-weight:bold;font-size:14px;font-family:monospace;color:black;padding:10px;max-height:200px;overflow-y:scroll;'>
                        <p style='color:#4a4e6a;user-select:none;'>def text_kuerzen_abt2(saetze: [String], rechtsinhaber: String, rangvermerk: String, re: Mapping[String, Regex]) -> String:</p>
                        <div style='padding-left:34px;caret-color: #4a4e6a;'contenteditable='true' onkeydown='insertTabAtCaret(event);' oninput='editTextKuerzenAbt2Script(event);'>{konfig_text_kuerzen_abt2_script}</div>
                    </div>
                    
                    <div style='display:flex;flex-direction:row;'>
                        <textarea style='border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' oninput='textKuerzenAbt2ScriptTesten(event);'style='flex-grow:1;margin-right:10px;' placeholder='Test Eingabe...'></textarea>
                        <textarea style='border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' id='__application_konfiguration_text_kuerzen_abt2_test' style='flex-grow:1;' placeholder='Test Ausgabe der Funktion text_kuerzen_abt2()'></textarea>
                    </div>
                </div>
                
                <hr/>
                
                <div style='padding:5px 0px;'>
                
                    <div>
                        <p style='font-family:sans-serif;font-weight:bold;font-size:16px;padding-bottom:10px;'>
                            Betrag auslesen (Abteilung 3)
                        </p>                        
                    </div>
                    
                    <div style='background:white;border:1px solid #efefef;margin-top:5px;font-weight:bold;font-size:14px;font-family:monospace;color:black;padding:10px;max-height:200px;overflow-y:scroll;'>
                        <p style='color:#4a4e6a;user-select:none;'>def betrag_auslesen(saetze: [String], re: Mapping[String, Regex]) -> Betrag:</p>
                        <div style='padding-left:34px;caret-color: #4a4e6a;' contenteditable='true' onkeydown='insertTabAtCaret(event);' oninput='editBetragAuslesenScript(event);'>{konfig_betrag_script}</div>
                    </div>
                    
                    <div style='display:flex;flex-direction:row;'>
                        <textarea style='border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' oninput='betragAuslesenScriptTesten(event);'style='flex-grow:1;margin-right:10px;' placeholder='Test Eingabe...'></textarea>
                        <textarea style='border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' id='__application_konfiguration_betrag_auslesen_test' style='flex-grow:1;' placeholder='Test Ausgabe der Funktion'></textarea>
                    </div>
                </div>
                                                                
                <div style='padding:5px 0px;'>
                
                    <div>
                        <p style='font-family:sans-serif;font-weight:bold;font-size:16px;padding-bottom:10px;'>
                            Klassifizierung SchuldenArt (Abteilung 3)
                        </p>                        
                    </div>
                    
                    <div style='background:white;border:1px solid #efefef;margin-top:5px;font-weight:bold;font-size:14px;font-family:monospace;color:black;padding:10px;max-height:200px;overflow-y:scroll;'>
                        <p style='color:#4a4e6a;user-select:none;'>def klassifiziere_schuldenart_abt3(saetze: [String], re: Mapping[String, Regex]) -> SchuldenArt:</p>
                        <div style='padding-left:34px;caret-color: #4a4e6a;' contenteditable='true' onkeydown='insertTabAtCaret(event);' oninput='editSchuldenArtScript(event);'>{konfig_schuldenart_script}</div>
                    </div>
                    
                    <div style='display:flex;flex-direction:row;'>
                        <textarea style='border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' oninput='schuldenArtScriptTesten(event);'style='flex-grow:1;margin-right:10px;' placeholder='Test Eingabe...'></textarea>
                        <textarea style='border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' id='__application_konfiguration_schuldenart_test' style='flex-grow:1;' placeholder='Test Ausgabe der Funktion'></textarea>
                    </div>
                </div>
                
                <div style='padding:5px 0px;'>
                
                    <div>
                        <p style='font-family:sans-serif;font-weight:bold;font-size:16px;padding-bottom:10px;'>
                            Rechtsinhaber auslesen (Abteilung 3)
                        </p>                        
                    </div>
                    
                    <div style='background:white;border:1px solid #efefef;margin-top:5px;font-weight:bold;font-size:14px;font-family:monospace;color:black;padding:10px;max-height:200px;overflow-y:scroll;'>
                        <p style='color:#4a4e6a;user-select:none;'>def rechtsinhaber_auslesen_abt3(saetze: [String], re: Mapping[String, Regex]) -> String:</p>
                        <div style='padding-left:34px;caret-color: #4a4e6a;' contenteditable='true' onkeydown='insertTabAtCaret(event);' oninput='editRechtsinhaberAbt3Script(event);'>{konfig_rechtsinhaber_abt3_script}</div>
                    </div>
                    
                    <div style='display:flex;flex-direction:row;'>
                        <textarea style='border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' oninput='rechtsinhaberAbt3ScriptTesten(event);'style='flex-grow:1;margin-right:10px;' placeholder='Test Eingabe...'></textarea>
                        <textarea style='border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' id='__application_konfiguration_rechtsinhaber_abt3_test' style='flex-grow:1;' placeholder='Test Ausgabe der Funktion'></textarea>
                    </div>
                </div>
                
                
                <div style='padding:5px 0px;'>
                
                    <div>
                        <p style='font-family:sans-serif;font-weight:bold;font-size:16px;padding-bottom:10px;'>
                            Text kürzen (Abteilung 3)
                        </p>                        
                    </div>
                    
                    <div style='background:white;border:1px solid #efefef;margin-top:5px;font-weight:bold;font-size:14px;font-family:monospace;color:black;padding:10px;max-height:200px;overflow-y:scroll;'>
                        <p style='color:#4a4e6a;user-select:none;'>def text_kuerzen_abt3(saetze: [String], betrag: String, schuldenart: String, rechtsinhaber: String, re: Mapping[String, Regex]) -> String:</p>
                        <div style='padding-left:34px;caret-color: #4a4e6a;' contenteditable='true' onkeydown='insertTabAtCaret(event);' oninput='editTextKuerzenAbt3Script(event);'>{konfig_text_kuerzen_abt3_script}</div>
                    </div>
                    
                    <div style='display:flex;flex-direction:row;'>
                        <textarea style='border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' oninput='textKuerzenAbt3ScriptTesten(event);'style='flex-grow:1;margin-right:10px;' placeholder='Test Eingabe...'></textarea>
                        <textarea style='border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' id='__application_konfiguration_text_kuerzen_abt3_test' style='flex-grow:1;' placeholder='Test Ausgabe der Funktion text_kuerzen_abt3()'></textarea>
                    </div>
                </div>
            </div>
        ", 
            konfig_pfad = Konfiguration::konfiguration_pfad(),
            
            vorschau_ohne_geroetet = if rpc_data.konfiguration.vorschau_ohne_geroetet { "checked" } else { "" },
            spalten_ausblenden = if rpc_data.konfiguration.spalten_ausblenden { "checked" } else { "" },
            zeilenumbrueche_in_ocr_text = if rpc_data.konfiguration.zeilenumbrueche_in_ocr_text { "checked" } else { "" },
            
            regex = {
                
                let r = if rpc_data.konfiguration.regex.is_empty() {
                    use std::collections::BTreeMap;
                    let mut a = BTreeMap::new();
                    a.insert("REGEX_ID".to_string(), "(.*)".to_string());
                    a
                } else {
                    rpc_data.konfiguration.regex.clone()
                };
                
                r.iter().enumerate().map(|(idx, (k, v))| format!("
                    <div style='display:flex;'>
                        <div id='__application_konfiguration_regex_key_{idx}' style='display:inline;min-width:250px;caret-color: #4a4e6a;' contenteditable='true' data-regex-key='{k}' oninput='editRegexKey(event);' onkeydown='neueRegexOnEnter(event);' data-key-id='__application_konfiguration_regex_key_{idx}'>{k}</div>
                        <p style='display:inline;color:#4a4e6a;user-select:none;'>&nbsp;= re.compile(\"</p>
                        <div id='__application_konfiguration_regex_value_{idx}' data-key-id='__application_konfiguration_regex_key_{idx}' style='display:inline;caret-color: #4a4e6a;' onkeydown='neueRegexOnEnter(event);' contenteditable='true' oninput='editRegexValue(event);'>{v}</div>
                        <p style='display:inline;color:#4a4e6a;user-select:none;'>\")</p>
                        <div style='display:inline-flex;flex-grow:1;'></div>
                        <img style='width:16px;height:16px;cursor:pointer;' data-key-id='__application_konfiguration_regex_key_{idx}' onclick='regexLoeschen(event);' src='data:image/png;base64,{icon_close_base64}'>
                    </div>
                ", k = k, v = v.replace("\\", "&bsol;"), idx = idx, icon_close_base64 = icon_close_base64))
                .collect::<Vec<_>>()
                .join("\r\n")
            },
            
            konfig_rangvermerk_abt2_script = 
            rpc_data.konfiguration.rangvermerk_auslesen_abt2_script.iter()
            .map(|l| l.replace(" ", "\u{00a0}"))
            .map(|l| l.replace("\\", "&bsol;"))
            .map(|l| if l.is_empty() { format!("<div>&nbsp;</div>") } else { format!("<div>{}</div>", l) })
            .collect::<Vec<String>>()
            .join("\r\n"),
            
            konfig_rechtsinhaber_abt3_script = 
            rpc_data.konfiguration.rechtsinhaber_auslesen_abt3_script.iter()
            .map(|l| l.replace(" ", "\u{00a0}"))
            .map(|l| l.replace("\\", "&bsol;"))
            .map(|l| if l.is_empty() { format!("<div>&nbsp;</div>") } else { format!("<div>{}</div>", l) })
            .collect::<Vec<String>>()
            .join("\r\n"),
            
            konfig_rechtsinhaber_abt2_script = 
            rpc_data.konfiguration.rechtsinhaber_auslesen_abt2_script.iter()
            .map(|l| l.replace(" ", "\u{00a0}"))
            .map(|l| l.replace("\\", "&bsol;"))
            .map(|l| if l.is_empty() { format!("<div>&nbsp;</div>") } else { format!("<div>{}</div>", l) })
            .collect::<Vec<String>>()
            .join("\r\n"),
            
            konfig_betrag_script = 
            rpc_data.konfiguration.betrag_auslesen_script.iter()
            .map(|l| l.replace(" ", "\u{00a0}"))
            .map(|l| l.replace("\\", "&bsol;"))
            .map(|l| if l.is_empty() { format!("<div>&nbsp;</div>") } else { format!("<div>{}</div>", l) })
            .collect::<Vec<String>>()
            .join("\r\n"),
            
            konfig_schuldenart_script = 
            rpc_data.konfiguration.klassifiziere_schuldenart.iter()
            .map(|l| l.replace(" ", "\u{00a0}"))
            .map(|l| l.replace("\\", "&bsol;"))
            .map(|l| if l.is_empty() { format!("<div>&nbsp;</div>") } else { format!("<div>{}</div>", l) })
            .collect::<Vec<String>>()
            .join("\r\n"),
            
            konfig_rechteart_script = 
            rpc_data.konfiguration.klassifiziere_rechteart.iter()
            .map(|l| l.replace(" ", "\u{00a0}"))
            .map(|l| l.replace("\\", "&bsol;"))
            .map(|l| if l.is_empty() { format!("<div>&nbsp;</div>") } else { format!("<div>{}</div>", l) })
            .collect::<Vec<String>>()
            .join("\r\n"),
            
            konfig_abkuerzungen_script = 
                rpc_data.konfiguration.abkuerzungen_script.iter()
                .map(|l| l.replace(" ", "\u{00a0}"))
                .map(|l| l.replace("\\", "&bsol;"))
                .map(|l| if l.is_empty() { format!("<div>&nbsp;</div>") } else { format!("<div>{}</div>", l) })
                .collect::<Vec<String>>()
            .join("\r\n"),
            
            konfig_text_saubern_script = 
                rpc_data.konfiguration.text_saubern_script.iter()
                .map(|l| l.replace(" ", "\u{00a0}"))
                .map(|l| l.replace("\\", "&bsol;"))
                .map(|l| if l.is_empty() { format!("<div>&nbsp;</div>") } else { format!("<div>{}</div>", l) })
                .collect::<Vec<String>>()
            .join("\r\n"),
            
            konfig_flurstuecke_auslesen_script = 
                rpc_data.konfiguration.flurstuecke_auslesen_script.iter()
                .map(|l| l.replace(" ", "\u{00a0}"))
                .map(|l| l.replace("\\", "&bsol;"))
                .map(|l| if l.is_empty() { format!("<div>&nbsp;</div>") } else { format!("<div>{}</div>", l) })
                .collect::<Vec<String>>()
            .join("\r\n"),
            
            konfig_text_kuerzen_abt2_script = 
                rpc_data.konfiguration.text_kuerzen_abt2_script.iter()
                .map(|l| l.replace(" ", "\u{00a0}"))
                .map(|l| l.replace("\\", "&bsol;"))
                .map(|l| if l.is_empty() { format!("<div>&nbsp;</div>") } else { format!("<div>{}</div>", l) })
                .collect::<Vec<String>>()
            .join("\r\n"),
            
            konfig_text_kuerzen_abt3_script = 
                rpc_data.konfiguration.text_kuerzen_abt3_script.iter()
                .map(|l| l.replace(" ", "\u{00a0}"))
                .map(|l| l.replace("\\", "&bsol;"))
                .map(|l| if l.is_empty() { format!("<div>&nbsp;</div>") } else { format!("<div>{}</div>", l) })
                .collect::<Vec<String>>()
            .join("\r\n"),
        )
    } else if let Some(cm) = rpc_data.context_menu_active.clone() {
        format!("
        <div style='pointer-events:unset;padding:1px;position:absolute;left:{}px;top:{}px;{}background:white;border-radius:5px;box-shadow:0px 0px 5px #444;'>
        <div style='border:1px solid #efefef;border-radius:5px;'>
            <p style='padding:5px 10px;font-size:10px;color:#444;margin-bottom:5px;'>Klassifiziere Seite als...</p>
            <div style='line-height:1.5;cursor:pointer;'>
                <div class='kontextmenü-eintrag' data-seite-neu='bv-horz' data-seite='{seite}' onmousedown='klassifiziereSeiteNeu(event);'>
                    Bestandsverzeichnis (Querformat)
                </div>
                <div class='kontextmenü-eintrag' data-seite-neu='bv-horz-zu-und-abschreibungen' data-seite='{seite}' onmousedown='klassifiziereSeiteNeu(event);'>
                    Bestandsverzeichnis Zu- und Abschreibungen (Querformat)
                </div>
                <div class='kontextmenü-eintrag' data-seite-neu='bv-vert' data-seite='{seite}' onmousedown='klassifiziereSeiteNeu(event);'>
                    Bestandsverzeichnis (Hochformat)
                </div>
                <div class='kontextmenü-eintrag' data-seite-neu='bv-vert-typ2' data-seite='{seite}' onmousedown='klassifiziereSeiteNeu(event);'>
                    Bestandsverzeichnis Variante 2 (Hochformat)
                </div>
                <div class='kontextmenü-eintrag' data-seite-neu='bv-vert-zu-und-abschreibungen' data-seite='{seite}' onmousedown='klassifiziereSeiteNeu(event);'>
                    Bestandsverzeichnis Zu- und Abschreibungen (Hochformat)
                </div>
                <div class='kontextmenü-eintrag' data-seite-neu='abt1-horz' data-seite='{seite}' onmousedown='klassifiziereSeiteNeu(event);'>
                    Abteilung 1 (Querformat)
                </div>
                <div class='kontextmenü-eintrag' data-seite-neu='abt1-vert' data-seite='{seite}' onmousedown='klassifiziereSeiteNeu(event);'>
                    Abteilung 1 (Hochformat)
                </div>
                <div class='kontextmenü-eintrag' data-seite-neu='abt2-horz-veraenderungen' data-seite='{seite}' onmousedown='klassifiziereSeiteNeu(event);'>
                    Abteilung 2 Veränderungen (Querformat)
                </div>
                <div class='kontextmenü-eintrag' data-seite-neu='abt2-horz' data-seite='{seite}' onmousedown='klassifiziereSeiteNeu(event);'>
                    Abteilung 2 (Querformat)
                </div>
                <div class='kontextmenü-eintrag' data-seite-neu='abt2-vert-veraenderungen' data-seite='{seite}' onmousedown='klassifiziereSeiteNeu(event);'>
                    Abteilung 2 Veränderungen (Hochformat)
                </div>
                <div class='kontextmenü-eintrag' data-seite-neu='abt2-vert' data-seite='{seite}' onmousedown='klassifiziereSeiteNeu(event);'>
                    Abteilung 2 (Hochformat)
                </div>
                <div class='kontextmenü-eintrag' data-seite-neu='abt3-horz-veraenderungen-loeschungen' data-seite='{seite}' onmousedown='klassifiziereSeiteNeu(event);'>
                    Abteilung 3 Veränderungen / Löschungen (Querformat)
                </div>
                <div class='kontextmenü-eintrag' data-seite-neu='abt3-horz' data-seite='{seite}' onmousedown='klassifiziereSeiteNeu(event);'>
                    Abteilung 3 (Querformat)
                </div>
                <div class='kontextmenü-eintrag' data-seite-neu='abt3-vert-veraenderungen' data-seite='{seite}' onmousedown='klassifiziereSeiteNeu(event);'>
                    <p>Abteilung 3 Veränderungen (Hochformat)
                </div>
                <div class='kontextmenü-eintrag' data-seite-neu='abt3-vert-loeschungen' data-seite='{seite}' onmousedown='klassifiziereSeiteNeu(event);'>
                    Abteilung 3 Löschungen (Hochformat)
                </div>
                <div class='kontextmenü-eintrag' data-seite-neu='abt3-vert-veraenderungen-loeschungen' data-seite='{seite}' onmousedown='klassifiziereSeiteNeu(event);'>
                    Abteilung 3 Veränderungen / Löschungen (Hochformat)
                </div>
                <div class='kontextmenü-eintrag' data-seite-neu='abt3-vert' data-seite='{seite}' onmousedown='klassifiziereSeiteNeu(event);'>
                    Abteilung 3 (Hochformat)
                </div>
            </div>
        </div>
        </div>", 
            cm.x, 
            cm.y,
            if cm.seite_ausgewaehlt > 20 { "transform:translateY(-100%);" } else { "" }, 
            seite = cm.seite_ausgewaehlt
        )
    } else { 
        format!("") 
    };
    
    let pc = format!("<div style='
        background:{};
        width: 100%;
        height: 100%;
        min-height: 100%;
        z-index:1001;
        pointer-events:all;' onmousedown='closePopOver()'>{}</div>", application_popover_color, pc
    );
    
    normalize_for_js(pc)
}

pub fn render_ribbon(rpc_data: &RpcData) -> String {

    static ICON_EINSTELLUNGEN: &[u8] = include_bytes!("./img/icons8-settings-48.png");
    static ICON_INFO: &[u8] = include_bytes!("./img/icons8-info-48.png");
    static ICON_GRUNDBUCH_OEFFNEN: &[u8] = include_bytes!("./img/icons8-book-96.png");
    static ICON_ZURUECK: &[u8] = include_bytes!("./img/icons8-back-48.png");
    static ICON_VORWAERTS: &[u8] = include_bytes!("./img/icons8-forward-48.png");
    static ICON_EXPORT_CSV: &[u8] = include_bytes!("./img/icons8-microsoft-excel-2019-96.png");
    static ICON_EXPORT_LEFIS: &[u8] = include_bytes!("./img/icons8-export-96.png");
    static ICON_DOWNLOAD: &[u8] = include_bytes!("./img/icons8-desktop-download-48.png");
    static ICON_DELETE: &[u8] = include_bytes!("./img/icons8-delete-trash-48.png");

    let ribbon_body = format!("
        <div class='__application-ribbon-body'>
            <div class='__application-ribbon-section 1'>
                <div style='display:flex;flex-direction:row;'>
                    <div class='__application-ribbon-section-content'>
                        <label onmouseup='tab_functions.load_new_pdf(event)' class='__application-ribbon-action-vertical-large'>
                            <div class='icon-wrapper'>
                                <img class='icon' src='data:image/png;base64,{icon_open_base64}'>
                            </div>
                            <div>
                                <p>PDF</p>
                                <p>laden</p>
                            </div>
                        </label>
                    </div>
                </div>
            </div>
            
            <div class='__application-ribbon-section 2'>
                <div style='display:flex;flex-direction:row;'>
                    <div class='__application-ribbon-section-content'>
                        <label onmouseup='tab_functions.undo(event)' class='__application-ribbon-action-vertical-large'>
                            <div class='icon-wrapper'>
                                <img class='icon {disabled}' src='data:image/png;base64,{icon_back_base64}'>
                            </div>
                            <div>
                                <p>Zurück</p>
                            </div>
                        </label>
                    </div>
                    
                    <div class='__application-ribbon-section-content'>
                        <label onmouseup='tab_functions.redo(event)' class='__application-ribbon-action-vertical-large'>
                            <div class='icon-wrapper'>
                                <img class='icon {disabled}' src='data:image/png;base64,{icon_forward_base64}'>
                            </div>
                            <div>
                                <p>Vorwärts</p>
                            </div>
                        </label>
                    </div>
                </div>
            </div>
            
            
            <div class='__application-ribbon-section 3'>
                <div style='display:flex;flex-direction:row;'>
                    <div class='__application-ribbon-section-content'>
                        <label onmouseup='tab_functions.export_nb(event)' class='__application-ribbon-action-vertical-large'>
                            <div class='icon-wrapper'>
                                <img class='icon {disabled}' src='data:image/png;base64,{icon_export_csv}'>
                            </div>
                            <div>
                                <p>Nebenbet.</p>
                                <p>in CSV</p>
                            </div>
                        </label>
                    </div>
                    
                    <div class='__application-ribbon-section-content'>
                        <label onmouseup='tab_functions.import_nb(event)' class='__application-ribbon-action-vertical-large'>
                            <div class='icon-wrapper'>
                                <img class='icon {disabled}' src='data:image/png;base64,{icon_download_base64}'>
                            </div>
                            <div>
                                <p>Nebenbet.</p>
                                <p>importieren</p>
                            </div>
                        </label>
                    </div>
                    
                    <div class='__application-ribbon-section-content'>
                        <label onmouseup='tab_functions.delete_nb(event)' class='__application-ribbon-action-vertical-large'>
                            <div class='icon-wrapper'>
                                <img class='icon {disabled}' src='data:image/png;base64,{icon_delete_base64}'>
                            </div>
                            <div>
                                <p>Nebenbet.</p>
                                <p>entfernen</p>
                            </div>
                        </label>
                    </div>
                </div>
            </div>
            
            <div class='__application-ribbon-section 4'>
                <div style='display:flex;flex-direction:row;'>
                    <div class='__application-ribbon-section-content'>
                        <label onmouseup='tab_functions.export_lefis(event)' class='__application-ribbon-action-vertical-large'>
                            <div class='icon-wrapper'>
                                <img class='icon {disabled}' src='data:image/png;base64,{icon_export_lefis}'>
                            </div>
                            <div>
                                <p>Export</p>
                                <p>(.lefis)</p>
                            </div>
                        </label>
                    </div>
                </div>
            </div>
            
            <div class='__application-ribbon-section 5'>
                <div style='display:flex;flex-direction:row;'>
                    <div class='__application-ribbon-section-content'>
                        <label onmouseup='tab_functions.open_configuration(event);' class='__application-ribbon-action-vertical-large'>
                            <div class='icon-wrapper'>
                                <img class='icon' src='data:image/png;base64,{icon_settings_base64}'>
                            </div>
                            <div>
                                <p>Einstellungen</p>
                                <p>bearbeiten</p>
                            </div>
                        </label>
                    </div>
                </div>
            </div>
            
            <div style='display:flex;flex-grow:1;'></div>
            
            <div class='__application-ribbon-section 6'>
                <div style='display:flex;flex-direction:row;'>
                    <div class='__application-ribbon-section-content'>
                        <label onmouseup='tab_functions.open_info(event);' class='__application-ribbon-action-vertical-large'>
                            <div class='icon-wrapper'>
                                <img class='icon' src='data:image/png;base64,{icon_info_base64}'>
                            </div>
                            <div>
                                <p>Info</p>
                            </div>
                        </label>
                    </div>
                </div>
            </div>
        </div>
        ", 
        disabled = if rpc_data.loaded_files.is_empty() { " disabled" } else { "" },
        icon_open_base64 = base64::encode(ICON_GRUNDBUCH_OEFFNEN),
        icon_back_base64 = base64::encode(ICON_ZURUECK),
        icon_forward_base64 = base64::encode(ICON_VORWAERTS),
        icon_settings_base64 = base64::encode(ICON_EINSTELLUNGEN),
        icon_info_base64 = base64::encode(ICON_INFO),
        icon_download_base64 = base64::encode(ICON_DOWNLOAD),
        icon_delete_base64 = base64::encode(ICON_DELETE),

        icon_export_csv = base64::encode(ICON_EXPORT_CSV),
        icon_export_lefis = base64::encode(ICON_EXPORT_LEFIS),
    );

    normalize_for_js(ribbon_body)
}

pub fn render_main(rpc_data: &mut RpcData) -> String {

    if rpc_data.loaded_files.is_empty() {
        return format!("<div style='width:100%;height:1000px;background:red;user-select: text;-webkit-user-select: text;'></div>");
    }

    normalize_for_js(format!("
        <div id='__application-file-list'>{file_list}</div>
        <div id='__application-page-list'>{page_list}</div>
        <div style='display:flex;flex-direction:column;flex-grow:1;'>
            <div id='__application-main-container'>{main_container}</div>
            <div id='__application-pdf-page-image'>{pdf_image}</div>
        </div>
    ",
        file_list = render_file_list(rpc_data),
        page_list = render_page_list(rpc_data),
        main_container = render_main_container(rpc_data),
        pdf_image = render_pdf_image(rpc_data),
    ))
}

pub fn render_file_list(rpc_data: &RpcData) -> String {
    
    const CLOSE_PNG: &[u8] = include_bytes!("../src/img/icons8-close-48.png");
    let close_str = format!("data:image/png;base64,{}", base64::encode(&CLOSE_PNG));
    
    const WARNING_CHECK_PNG: &[u8] = include_bytes!("../src/img/icons8-warning-48.png");
    let warning_check_str = format!("data:image/png;base64,{}", base64::encode(&WARNING_CHECK_PNG));
    
    const HALF_CHECK_PNG: &[u8] = include_bytes!("../src/img/icons8-in-progress-48.png");
    let half_check_str = format!("data:image/png;base64,{}", base64::encode(&HALF_CHECK_PNG));
    
    const FULL_CHECK_PNG: &[u8] = include_bytes!("../src/img/icons8-ok-48.png");
    let full_check_str = format!("data:image/png;base64,{}", base64::encode(&FULL_CHECK_PNG));

    normalize_for_js(rpc_data.loaded_files.keys().filter_map(|filename| {
        
        let datei_ausgewaehlt = rpc_data.open_page.as_ref().map(|s| s.0.as_str()) == Some(filename);
        
        let datei = rpc_data.loaded_files.get(filename)?;
        
        let datei_hat_keine_fehler = datei.hat_keine_fehler(&rpc_data.loaded_nb, &rpc_data.konfiguration);
        let datei_hat_alle_onr_zugewiesen = datei.alle_ordnungsnummern_zugewiesen(&rpc_data.loaded_nb, &rpc_data.konfiguration);
        
        Some(format!("<div class='{file_active}' style='user-select:none;display:flex;flex-direction:row;' data-fileName='{filename}' onmouseup='activateSelectedFile(event);'>
            {check}
            <p style='flex-grow:0;user-select:none;' data-fileName='{filename}' >{filename}</p>
            <div style='display:flex;flex-grow:1;' data-fileName='{filename}' ></div>
            {close_btn}
            </div>", 
            check = if datei_hat_keine_fehler && datei_hat_alle_onr_zugewiesen {
                format!(
                    "<img style='width: 16px;height: 16px;margin-right:5px;flex-grow: 0;cursor: pointer;' data-fileName='{filename}' src='{check}'></img>", 
                    filename = filename, 
                    check = full_check_str
                )
            } else if datei_hat_keine_fehler {
                format!(
                    "<img style='width: 16px;height: 16px;margin-right:5px;flex-grow: 0;cursor: pointer;' data-fileName='{filename}' src='{check}'></img>", 
                    filename = filename, 
                    check = half_check_str
                ) 
            } else if datei.ist_geladen() { 
                format!(
                    "<img style='width: 16px;height: 16px;margin-right:5px;flex-grow: 0;cursor: pointer;' data-fileName='{filename}' src='{check}'></img>", 
                    filename = filename, 
                    check = warning_check_str
                ) 
            } else {
                format!(
                    "<div style='width: 16px;height: 16px;margin-right:5px;flex-grow: 0;cursor: pointer;' data-fileName='{filename}'></div>", 
                    filename = filename, 
                )
            },
            file_active = if datei_ausgewaehlt { "active" } else { "" },
            filename = filename, 
            close_btn = if datei_ausgewaehlt { 
                format!(
                    "<img style='width: 16px;height: 16px;padding: 2px;flex-grow: 0;cursor: pointer;' data-fileName='{filename}' onmouseup='closeFile(event);' src='{close_str}'></img>", 
                    filename = filename, 
                    close_str = close_str
                ) 
            } else { 
                String::new() 
            },
        ))
    }).collect::<Vec<_>>().join("\r\n"))
}

pub fn render_page_list(rpc_data: &RpcData) -> String {
    
    let open_file = match rpc_data.open_page.as_ref().and_then(|(of, _)| rpc_data.loaded_files.get(of)) {
        Some(s) => s,
        None => return String::new(),
    };
    
    let pages_div = open_file.seitenzahlen.iter().map(|page_num| {
    
        use crate::digitalisiere::SeitenTyp;
        
        let page_is_loaded = open_file.geladen.contains_key(page_num);
        let page_is_active = rpc_data.open_page.as_ref().map(|s| s.1) == Some(*page_num);
        let seiten_typ = open_file.klassifikation_neu
            .get(&(*page_num as usize)).cloned()
            .or(open_file.geladen.get(page_num).map(|p| p.typ.clone()));
        
        let page_color = seiten_typ.map(|t| match t {
              SeitenTyp::BestandsverzeichnisHorz
            | SeitenTyp::BestandsverzeichnisHorzZuUndAbschreibungen
            | SeitenTyp::BestandsverzeichnisVert
            | SeitenTyp::BestandsverzeichnisVertTyp2
            | SeitenTyp::BestandsverzeichnisVertZuUndAbschreibungen => {
                "rgb(167,224,255)" // blau
            },
              SeitenTyp::Abt1Horz
            | SeitenTyp::Abt1Vert => {
                "rgb(167,255,185)" // grün
            },
              SeitenTyp::Abt2HorzVeraenderungen
            | SeitenTyp::Abt2Horz
            | SeitenTyp::Abt2VertVeraenderungen
            | SeitenTyp::Abt2Vert => {
                "rgb(255,255,167)" // gelb
            },
              SeitenTyp::Abt3HorzVeraenderungenLoeschungen
            |  SeitenTyp::Abt3VertVeraenderungenLoeschungen
            | SeitenTyp::Abt3Horz
            | SeitenTyp::Abt3VertVeraenderungen
            | SeitenTyp::Abt3VertLoeschungen
            | SeitenTyp::Abt3Vert => {
                "rgb(255,200,167)" // orange
            },        
        }).unwrap_or("white");
        
        format!(
            "<div class='__application-page {loaded} {active}' oncontextmenu='openContextMenu(event);' data-pageNumber='{page_num}' {extra_style} onclick='activateSelectedPage(event)'>{page_num}</div>",
            loaded = if page_is_loaded { "loaded" } else { "" },
            active = if page_is_active { "active" } else { "" },
            extra_style = if !page_is_active { format!("style='background:{}'", page_color) } else { String::new() },
            page_num = page_num
        )
    }).collect::<Vec<_>>().join("\r\n");    

    normalize_for_js(format!("
        <div><h5>Seite</h5></div>
        <div style='margin:10px;'>
            <div><div style='display:inline-block;width:10px;height:10px;border-radius:50%;background:rgb(167,224,255);'></div><p style='display:inline-block;'>&nbsp;Bestandsverz.</p></div>
            <div><div style='display:inline-block;width:10px;height:10px;border-radius:50%;background:rgb(167,255,185);'></div><p style='display:inline-block;'>&nbsp;Abt. 1</p></div>
            <div><div style='display:inline-block;width:10px;height:10px;border-radius:50%;background:rgb(255,255,167);'></div><p style='display:inline-block;'>&nbsp;Abt. 2</p></div>
            <div><div style='display:inline-block;width:10px;height:10px;border-radius:50%;background:rgb(255,200,167);'></div><p style='display:inline-block;'>&nbsp;Abt. 3</p></div>
        </div>

        <div>{}</div>
    ", pages_div))
}

pub fn render_main_container(rpc_data: &mut RpcData) -> String {
    
    let open_file = match rpc_data.open_page.as_mut().and_then(|of| rpc_data.loaded_files.get_mut(&of.0)) {
        Some(s) => s,
        None => return String::new(),
    };
    
    const RELOAD_PNG: &[u8] = include_bytes!("../src/img/icons8-synchronize-48.png");
    
    if !open_file.ist_geladen() {
        normalize_for_js(format!("
                <div style='height: 100%;padding:10px;display:flex;flex-grow:1;align-items:center;justify-content:center;'>
                    <h2 style='font-size: 16px;font-weight:bold;'>Grundbuch wird geladen...</h2>
                </div>
            ",
        ))
    } else {
        
        let reload_str = format!("data:image/png;base64,{}", base64::encode(&RELOAD_PNG));
    
        normalize_for_js(format!("
                <div style='height:43px;border-bottom: 1px solid #efefef;box-sizing:border-box;'>
                    <div style='display:inline-block;width:50%;overflow:hidden;'>
                        <div style='display:flex;flex-direction:row;'>
                        <h4 style='padding:10px;font-size:16px;'>Grundbuch</h4>
                        <div style='display:flex;flex-grow:1;'></div>
                        <div style='padding:6px;'>
                            <img src='{reload_icon}' style='width:24px;height:24px;cursor:pointer;' onmouseup='reloadGrundbuch(event);'></img>
                        </div>
                        </div>
                    </div>
                    <div style='display:inline-block;width:50%;overflow:hidden;'>
                        <h4 style='padding:10px;font-size:16px;'>LEFIS</h4>
                    </div>
                </div>
                <div style='display:flex;flex-grow:1;flex-direction:row;padding:0px;'>
                    <div style='display:inline-block;width:50%;overflow:scroll;max-height:557px;'>
                        <div id='__application-bestandsverzeichnis' style='margin:10px;'>{bestandsverzeichnis}</div>
                        <div id='__application-bestandsverzeichnis-veraenderungen' style='margin:10px;'>{bestandsverzeichnis_zuschreibungen}</div>
                        <div id='__application-bestandsverzeichnis-loeschungen' style='margin:10px;'>{bestandsverzeichnis_abschreibungen}</div>
                        <div id='__application-abteilung-1' style='margin:10px;'>{abt_1}</div>
                        <div id='__application-abteilung-1-veraenderungen' style='margin:10px;'>{abt_1_zuschreibungen}</div>
                        <div id='__application-abteilung-1-loeschungen' style='margin:10px;'>{abt_1_abschreibungen}</div>
                        <div id='__application-abteilung-2' style='margin:10px;'>{abt_2}</div>
                        <div id='__application-abteilung-2-veraenderungen' style='margin:10px;'>{abt_2_zuschreibungen}</div>
                        <div id='__application-abteilung-2-loeschungen' style='margin:10px;'>{abt_2_abschreibungen}</div>
                        <div id='__application-abteilung-3' style='margin:10px;'>{abt_3}</div>
                        <div id='__application-abteilung-3-veraenderungen' style='margin:10px;'>{abt_3_zuschreibungen}</div>
                        <div id='__application-abteilung-3-loeschungen' style='margin:10px;'>{abt_3_abschreibungen}</div>
                    </div>
                    <div id='__application-analyse-grundbuch' style='display:inline-block;width:50%;overflow:scroll;max-height:557px;'>
                        {analyse}
                    </div>
                </div>
            ",
            
            reload_icon = reload_str,
            bestandsverzeichnis = render_bestandsverzeichnis(open_file),
            bestandsverzeichnis_zuschreibungen = render_bestandsverzeichnis_zuschreibungen(open_file),
            bestandsverzeichnis_abschreibungen = render_bestandsverzeichnis_abschreibungen(open_file),
            
            abt_1 = render_abt_1(open_file),
            abt_1_zuschreibungen = render_abt_1_veraenderungen(open_file),
            abt_1_abschreibungen = render_abt_1_loeschungen(open_file),
            
            abt_2 = render_abt_2(open_file),
            abt_2_zuschreibungen = render_abt_2_veraenderungen(open_file),
            abt_2_abschreibungen = render_abt_2_loeschungen(open_file),
            
            abt_3 = render_abt_3(open_file),
            abt_3_zuschreibungen = render_abt_3_veraenderungen(open_file),
            abt_3_abschreibungen = render_abt_3_loeschungen(open_file),
            
            analyse = render_analyse_grundbuch(open_file, &rpc_data.loaded_nb, &rpc_data.konfiguration),
        ))
    }
}

pub fn render_analyse_grundbuch(open_file: &PdfFile, nb: &[Nebenbeteiligter], konfiguration: &Konfiguration) -> String {
    
    const PFEIL_PNG: &[u8] = include_bytes!("../src/img/icons8-arrow-48.png");
    const WARNUNG_PNG: &[u8] = include_bytes!("../src/img/icons8-warning-48.png");
    const FEHLER_PNG: &[u8] = include_bytes!("../src/img/icons8-high-priority-48.png");

    let pfeil_str = format!("data:image/png;base64,{}", base64::encode(&PFEIL_PNG));
    let warnung_str = format!("data:image/png;base64,{}", base64::encode(&WARNUNG_PNG));
    let fehler_str = format!("data:image/png;base64,{}", base64::encode(&FEHLER_PNG));

    let gb_analysiert = crate::analysiere::analysiere_grundbuch(&open_file.analysiert, nb, konfiguration);
    
    normalize_for_js(format!("
        <div style='margin:10px;'>
            <h4>Analyse Abt. 2</h4>
            {a2_analyse}
            
            <h4>Analyse Abt. 3</h4>
            {a3_analyse}
        </div>
        ",
        a2_analyse = gb_analysiert.abt2.iter().map(|a2a| {
            format!("
            <div class='__application-abt2-analysiert' style='margin:5px;padding:10px;border:1px solid #efefef;'>
                <h5 style='margin-bottom: 10px;'>{lfd_nr}&nbsp;{rechteart}</h5>
                <div style='display:flex;flex-direction:row;'>
                    <div style='min-width:380px;max-width:380px;margin-right:20px;'>
                        <p>{text_kurz}</p>
                    </div>
                    <div style='flex-grow:1;'>
                        <p style='font-style:italic'>{rechtsinhaber}</p>
                        {rangvermerk}
                        <div>{belastete_flurstuecke}</div>
                    </div>
                </div>
                <div class='__application-warnungen-und-fehler'>
                    {fehler}
                    {warnungen}
                </div>
                </div>",
                lfd_nr = format!("{}", a2a.lfd_nr),
                text_kurz = a2a.text_kurz,
                rechteart = format!("{:?}", a2a.rechteart).to_uppercase(),
                rechtsinhaber = match a2a.nebenbeteiligter.ordnungsnummer.as_ref() { 
                    Some(onr) => format!("{}/00 - {}", onr, a2a.rechtsinhaber),
                    None => a2a.rechtsinhaber.clone(),
                },
                rangvermerk = match a2a.rangvermerk.as_ref() {
                    Some(s) => format!("<span style='display:flex;'>
                        <img src='{warnung}' style='width:12px;height:12px;'/>
                        <p style='display:inline-block;margin-left:10px;'>{rang}</p>
                        </span>", 
                        warnung = warnung_str,
                        rang = s,
                    ),
                    None => String::new(),
                },
                belastete_flurstuecke = 
                    a2a.belastete_flurstuecke.iter().map(|belastet| {
                        use crate::digitalisiere::BvEintrag;
                        match belastet {
                            BvEintrag::Flurstueck(flst) => {
                                format!("<span style='display:flex;'>
                                    <img src='{pfeil}' style='width:12px;height:12px;'/>
                                    <p style='display:inline-block;margin-left:10px;'>Fl. {flur}, Flst. {flurstueck} (BV-Nr. {bv_nr})</p>
                                    </span>", 
                                    pfeil = pfeil_str,
                                    flur = flst.flur,
                                    flurstueck = flst.flurstueck,
                                    bv_nr = flst.lfd_nr,
                                ) 
                            },
                            BvEintrag::Recht(recht) => {
                                format!("<span style='display:flex;'>
                                    <img src='{pfeil}' style='width:12px;height:12px;'/>
                                    <p style='display:inline-block;margin-left:10px;'>Grundstücksgl. Recht (BV-Nr. {bv_nr})</p>
                                    </span>", 
                                    pfeil = pfeil_str,
                                    bv_nr = recht.lfd_nr,
                                ) 
                            },
                        }
                    })
                    .collect::<Vec<String>>()
                    .join("\r\n"),
                fehler = {
                    let mut fehler = a2a.fehler.clone();
                    fehler.sort();
                    fehler.dedup();
                    
                    fehler.iter().map(|w| {
                        format!("<span style='display:flex;margin-top:5px;padding: 4px 8px; background:rgb(255,195,195);'>
                            <img src='{fehler_icon}' style='width:12px;height:12px;'/>
                                <p style='display:inline-block;margin-left:10px;color:rgb(129,8,8);'>{text}</p>
                            </span>", 
                            fehler_icon = fehler_str,
                            text = w,
                        )
                    }).collect::<Vec<_>>().join("\r\n")
                },
                warnungen = {
                    let mut warnungen = a2a.warnungen.clone();
                    warnungen.sort();
                    warnungen.dedup();
                    
                    warnungen.iter().map(|w| {
                    format!("<span style='display:flex;margin-top:5px;padding: 4px 8px; background:rgb(255,255,167);'>
                            <img src='{warnung_icon}' style='width:12px;height:12px;'/>
                                <p style='display:inline-block;margin-left:10px;'>{text}</p>
                            </span>", 
                            warnung_icon = warnung_str,
                            text = w,
                        )
                    }).collect::<Vec<_>>().join("\r\n")
                },
            )
        }).collect::<Vec<String>>().join("\r\n"),
    
        a3_analyse = gb_analysiert.abt3.iter().map(|a3a| {
                    
            let waehrung_str = a3a.betrag.waehrung.to_string();
            
            format!("
            <div class='__application-abt2-analysiert' style='margin:5px;padding:10px;border:1px solid #efefef;'>
                <h5 style='margin-bottom: 10px;'>{lfd_nr}&nbsp;{schuldenart}&nbsp;{betrag}</h5>
                    <div style='display:flex;flex-direction:row;'>
                        <div style='min-width:380px;max-width:380px;margin-right:20px;'>
                            <p>{text_kurz}</p>
                        </div>
                        <div style='flex-grow:1;'>
                            <p style='font-style:italic'>{rechtsinhaber}</p>
                            <div>{belastete_flurstuecke}</div>
                        </div>
                    </div>
                    <div class='__application-warnungen-und-fehler'>
                        {fehler}
                        {warnungen}
                    </div>
                </div>",
                lfd_nr = format!("{}", a3a.lfd_nr),
                text_kurz = a3a.text_kurz,
                betrag = format!("{} {}", crate::kurztext::formatiere_betrag(&a3a.betrag), waehrung_str),
                schuldenart = format!("{:?}", a3a.schuldenart).to_uppercase(),
                rechtsinhaber = match a3a.nebenbeteiligter.ordnungsnummer.as_ref() { 
                    Some(onr) => format!("{}/00 - {}", onr, a3a.rechtsinhaber),
                    None => a3a.rechtsinhaber.clone(),
                },
                belastete_flurstuecke = 
                    a3a.belastete_flurstuecke.iter().map(|belastet| {
                        use crate::digitalisiere::BvEintrag;
                        match belastet {
                            BvEintrag::Flurstueck(flst) => {
                                format!("<span style='display:flex;'>
                                    <img src='{pfeil}' style='width:12px;height:12px;'/>
                                    <p style='display:inline-block;margin-left:10px;'>Fl. {flur}, Flst. {flurstueck} (BV-Nr. {bv_nr})</p>
                                    </span>", 
                                    pfeil = pfeil_str,
                                    flur = flst.flur,
                                    flurstueck = flst.flurstueck,
                                    bv_nr = flst.lfd_nr,
                                ) 
                            },
                            BvEintrag::Recht(recht) => {
                                format!("<span style='display:flex;'>
                                    <img src='{pfeil}' style='width:12px;height:12px;'/>
                                    <p style='display:inline-block;margin-left:10px;'>Grundstücksgl. Recht (BV-Nr. {bv_nr})</p>
                                    </span>", 
                                    pfeil = pfeil_str,
                                    bv_nr = recht.lfd_nr,
                                ) 
                            },
                        }
                    })
                    .collect::<Vec<String>>()
                    .join("\r\n"),
                fehler = {
                    let mut fehler = a3a.fehler.clone();
                    fehler.sort();
                    fehler.dedup();
                    
                    fehler.iter().map(|w| {
                        format!("<span style='display:flex;margin-top:5px;padding: 4px 8px; background:rgb(255,195,195);'>
                            <img src='{fehler_icon}' style='width:12px;height:12px;'/>
                                <p style='display:inline-block;margin-left:10px;color:rgb(129,8,8);'>{text}</p>
                            </span>", 
                            fehler_icon = fehler_str,
                            text = w,
                        )
                    }).collect::<Vec<_>>().join("\r\n")
                },
                warnungen = {
                    let mut warnungen = a3a.warnungen.clone();
                    warnungen.sort();
                    warnungen.dedup();
                    
                    warnungen.iter().map(|w| {
                    format!("<span style='display:flex;margin-top:5px;padding: 4px 8px; background:rgb(255,255,167);'>
                            <img src='{warnung_icon}' style='width:12px;height:12px;'/>
                                <p style='display:inline-block;margin-left:10px;'>{text}</p>
                            </span>", 
                            warnung_icon = warnung_str,
                            text = w,
                        )
                    }).collect::<Vec<_>>().join("\r\n")
                },
            )
        }).collect::<Vec<String>>().join("\r\n"),
    ))

}

pub fn render_bestandsverzeichnis(open_file: &PdfFile) -> String {
    
    use crate::digitalisiere::BvEintrag;

    let mut bestandsverzeichnis = open_file.analysiert.bestandsverzeichnis.clone();
    if bestandsverzeichnis.eintraege.is_empty() {
        bestandsverzeichnis.eintraege = vec![BvEintrag::neu(1)];
    }
    
    let bv = bestandsverzeichnis.eintraege.iter().enumerate().map(|(zeile_nr, bve)| {
                
        let bv_geroetet = if bve.ist_geroetet() { 
            "background:rgb(255,195,195);" 
        } else { 
            "background:white;" 
        };
        
        match bve {
            BvEintrag::Flurstueck(flst) => {
                format!("
                <div class='__application-bestandsverzeichnis-eintrag' style='display:flex;'>
                    <select style='width: 60px;{bv_geroetet}' id='bv_{zeile_nr}_typ' onchange='bvEintragTypAendern(\"bv:{zeile_nr}:typ\", this.options[this.selectedIndex].value)'>
                        <option value='flst' selected='selected'>Flst.</option>
                        <option value='recht'>Recht</option>
                    </select>
                    <input type='number' style='width: 30px;{bv_geroetet}' value='{lfd_nr}' 
                        id='bv_{zeile_nr}_lfd-nr'
                        onkeyup='inputOnKeyDown(\"bv:{zeile_nr}:lfd-nr\", event)' 
                        oninput='editText(\"bv:{zeile_nr}:lfd-nr\", event)'
                    />
                    <input type='number' style='width: 80px;{bv_geroetet}' value='{bisherige_lfd_nr}' 
                        id='bv_{zeile_nr}_bisherige-lfd-nr'
                        onkeyup='inputOnKeyDown(\"bv:{zeile_nr}:bisherige-lfd-nr\", event)'
                        oninput='editText(\"bv:{zeile_nr}:bisherige-lfd-nr\", event)'
                    />
                    <input type='text' style='width: 160px;{bv_geroetet}'  value='{gemarkung}' 
                        id='bv_{zeile_nr}_gemarkung'
                        onkeyup='inputOnKeyDown(\"bv:{zeile_nr}:gemarkung\", event)'
                        oninput='editText(\"bv:{zeile_nr}:gemarkung\", event)'
                    />
                    <input type='number' style='width: 80px;{bv_geroetet}'  value='{flur}' 
                        id='bv_{zeile_nr}_flur'
                        onkeyup='inputOnKeyDown(\"bv:{zeile_nr}:flur\", event)'
                        oninput='editText(\"bv:{zeile_nr}:flur\", event)'
                    />
                    <input type='text' style='width: 80px;{bv_geroetet}'  value='{flurstueck}' 
                        id='bv_{zeile_nr}_flurstueck'
                        onkeyup='inputOnKeyDown(\"bv:{zeile_nr}:flurstueck\", event)'
                        oninput='editText(\"bv:{zeile_nr}:flurstueck\", event)'
                    />
                    <div style='display:flex;flex-direction:row;flex-grow:1;'>
                        <div style='display:flex;flex-grow:1'></div>
                        <button onclick='eintragNeu(\"bv:{zeile_nr}\")' tabindex='-1' class='btn btn_neu' >neu</button>
                        <button onclick='eintragRoeten(\"bv:{zeile_nr}\")' tabindex='-1' class='btn btn_roeten'>röten</button>
                        <button onclick='eintragLoeschen(\"bv:{zeile_nr}\")' tabindex='-1' class='btn btn_loeschen'>löschen</button>
                    </div>
                </div>",
                    bv_geroetet = bv_geroetet,
                    zeile_nr = zeile_nr,
                    lfd_nr = format!("{}", flst.lfd_nr),
                    bisherige_lfd_nr = flst.bisherige_lfd_nr.map(|f| format!("{}", f)).unwrap_or_default(),
                    flur = format!("{}", flst.flur),
                    flurstueck = format!("{}", flst.flurstueck),
                    gemarkung = flst.gemarkung.clone().unwrap_or_default(),
                )
            },
            BvEintrag::Recht(recht) => {
                format!("
                <div class='__application-bestandsverzeichnis-eintrag' style='display:flex;'>
                    <select style='width: 60px;{bv_geroetet}' id='bv_{zeile_nr}_typ' onchange='bvEintragTypAendern(\"bv:{zeile_nr}:typ\", this.options[this.selectedIndex].value)'>
                        <option value='flst'>Flst.</option>
                        <option value='recht' selected='selected'>Recht</option>
                    </select>
                    <input type='number' style='width: 30px;{bv_geroetet}' value='{lfd_nr}' 
                        id='bv_{zeile_nr}_lfd-nr'
                        onkeyup='inputOnKeyDown(\"bv:{zeile_nr}:lfd-nr\", event)' 
                        oninput='editText(\"bv:{zeile_nr}:lfd-nr\", event)'
                    />
                    <input type='number' style='width: 80px;{bv_geroetet}' value='{bisherige_lfd_nr}' 
                        id='bv_{zeile_nr}_bisherige-lfd-nr'
                        onkeyup='inputOnKeyDown(\"bv:{zeile_nr}:bisherige-lfd-nr\", event)'
                        oninput='editText(\"bv:{zeile_nr}:bisherige-lfd-nr\", event)'
                    />
                    <input type='text' style='width: 30px;{bv_geroetet}' value='{zu_nr}' 
                        id='bv_{zeile_nr}_zu-nr'
                        onkeyup='inputOnKeyDown(\"bv:{zeile_nr}:zu-nr\", event)' 
                        oninput='editText(\"bv:{zeile_nr}:zu-nr\", event)'
                    />
                    <textarea rows='5' cols='45' style='width: 320px;{bv_geroetet}'
                        id='bv_{zeile_nr}_recht-text'
                        onkeyup='inputOnKeyDown(\"bv:{zeile_nr}:recht-text\", event)'
                        oninput='editText(\"bv:{zeile_nr}:recht-text\", event)'
                    >{recht_text}</textarea>
                    <div style='display:flex;flex-direction:row;flex-grow:1;'>
                        <div style='display:flex;flex-grow:1'></div>
                        <button onclick='eintragNeu(\"bv:{zeile_nr}\")' tabindex='-1' class='btn btn_neu' >neu</button>
                        <button onclick='eintragRoeten(\"bv:{zeile_nr}\")' tabindex='-1' class='btn btn_roeten'>röten</button>
                        <button onclick='eintragLoeschen(\"bv:{zeile_nr}\")' tabindex='-1' class='btn btn_loeschen'>löschen</button>
                    </div>
                </div>", 
                    bv_geroetet = bv_geroetet,
                    zeile_nr = zeile_nr,
                    lfd_nr = format!("{}", recht.lfd_nr),
                    zu_nr = recht.zu_nr,
                    bisherige_lfd_nr = recht.bisherige_lfd_nr.map(|f| format!("{}", f)).unwrap_or_default(),
                    recht_text = recht.text,
                )
            },
        }
    }).collect::<Vec<String>>().join("\r\n");
    
    normalize_for_js(format!("
        <h4 style='position:sticky;top:0;background:white;padding:10px 0px;'>Bestandsverzeichnis</h4>
        
        <div class='__application-table-header'>
            <p style='width: 60px;'>Typ</p>
            <p style='width: 30px;'>Nr.</p>
            <p style='width: 80px;'>Nr. (alt)</p>
            <p style='width: 160px;'>Gemarkung</p>
            <p style='width: 80px;'>Flur</p>
            <p style='width: 80px;'>Flurstück</p>
        </div>
        {bv}
    ", bv = bv))
}

pub fn render_bestandsverzeichnis_zuschreibungen(open_file: &PdfFile) -> String {

    let mut bv_zuschreibungen = open_file.analysiert.bestandsverzeichnis.zuschreibungen.clone();
    if bv_zuschreibungen.is_empty() {
        bv_zuschreibungen = vec![BvZuschreibung::default()];
    }
    
    let bv = bv_zuschreibungen.iter().enumerate().map(|(zeile_nr, bvz)| {
        
        let bv_geroetet = if bvz.ist_geroetet() { 
            "background:rgb(255,195,195);" 
        } else { 
            "background:white;" 
        };
        
        format!("
        <div class='__application-bestandsverzeichnis-eintrag' style='display:flex;'>
            <input type='text' style='width: 90px;{bv_geroetet}' value='{bv_nr}' 
                id='bv-zuschreibung_{zeile_nr}_bv-nr'
                onkeyup='inputOnKeyDown(\"bv-zuschreibung:{zeile_nr}:bv-nr\", event)' 
                oninput='editText(\"bv-zuschreibung:{zeile_nr}:bv-nr\", event)'
            />
            <textarea rows='5' cols='45' style='{bv_geroetet}'
                id='bv-zuschreibung_{zeile_nr}_text'
                onkeyup='inputOnKeyDown(\"bv-zuschreibung:{zeile_nr}:text\", event)'
                oninput='editText(\"bv-zuschreibung:{zeile_nr}:text\", event)'
            >{text}</textarea>
            <div style='display:flex;flex-direction:row;flex-grow:1;'>
                <div style='display:flex;flex-grow:1'></div>
                <button onclick='eintragNeu(\"bv-zuschreibung:{zeile_nr}\")' tabindex='-1' class='btn btn_neu' >neu</button>
                <button onclick='eintragRoeten(\"bv-zuschreibung:{zeile_nr}\")' tabindex='-1' class='btn btn_roeten'>röten</button>
                <button onclick='eintragLoeschen(\"bv-zuschreibung:{zeile_nr}\")' tabindex='-1' class='btn btn_loeschen'>löschen</button>
            </div>
        </div>", 
            bv_geroetet = bv_geroetet,
            zeile_nr = zeile_nr,
            bv_nr = bvz.bv_nr,
            text = bvz.text,
        )
    }).collect::<Vec<String>>().join("\r\n");
    
    normalize_for_js(format!("
        <h4 style='position:sticky;top:0;background:white;padding:10px 0px;'>Bestandsverzeichnis - Zuschreibungen</h4>
        
        <div class='__application-table-header'>
            <p style='width: 90px;'>BV-Nr.</p>
            <p style='width: 160px;'>Text</p>
        </div>
        
        {bv}
    ", bv = bv))
}

pub fn render_bestandsverzeichnis_abschreibungen(open_file: &PdfFile) -> String {

    let mut bv_abschreibungen = open_file.analysiert.bestandsverzeichnis.abschreibungen.clone();
    if bv_abschreibungen.is_empty() {
        bv_abschreibungen = vec![BvAbschreibung::default()];
    }
    
    let bv = bv_abschreibungen.iter().enumerate().map(|(zeile_nr, bva)| {
        
        let bv_geroetet = if bva.ist_geroetet() { 
            "background:rgb(255,195,195);" 
        } else { 
            "background:white;" 
        };
        
        format!("
        <div class='__application-bestandsverzeichnis-eintrag' style='display:flex;'>
            <input type='text' style='width: 90px;{bv_geroetet}' value='{bv_nr}' 
                id='bv-abschreibung_{zeile_nr}_bv-nr'
                onkeyup='inputOnKeyDown(\"bv-abschreibung:{zeile_nr}:bv-nr\", event)' 
                oninput='editText(\"bv-abschreibung:{zeile_nr}:bv-nr\", event)'
            />
            <textarea rows='5' cols='45' style='{bv_geroetet}'
                id='bv-abschreibung_{zeile_nr}_text'
                onkeyup='inputOnKeyDown(\"bv-abschreibung:{zeile_nr}:text\", event)'
                oninput='editText(\"bv-abschreibung:{zeile_nr}:text\", event)'
            >{text}</textarea>
            <div style='display:flex;flex-direction:row;flex-grow:1;'>
                <div style='display:flex;flex-grow:1'></div>
                <button onclick='eintragNeu(\"bv-abschreibung:{zeile_nr}\")' tabindex='-1' class='btn btn_neu' >neu</button>
                <button onclick='eintragRoeten(\"bv-abschreibung:{zeile_nr}\")' tabindex='-1' class='btn btn_roeten'>röten</button>
                <button onclick='eintragLoeschen(\"bv-abschreibung:{zeile_nr}\")' tabindex='-1' class='btn btn_loeschen'>löschen</button>
            </div>
        </div>", 
            bv_geroetet = bv_geroetet,
            zeile_nr = zeile_nr,
            bv_nr = bva.bv_nr,
            text = bva.text,
        )
    }).collect::<Vec<String>>().join("\r\n");
    
    normalize_for_js(format!("
        <h4 style='position:sticky;top:0;background:white;padding:10px 0px;'>Bestandsverzeichnis - Abschreibungen</h4>
        
        <div class='__application-table-header'>
            <p style='width: 90px;'>BV-Nr.</p>
            <p style='width: 160px;'>Text</p>
        </div>
        
        {bv}
    ", bv = bv))

}

pub fn render_abt_1(open_file: &PdfFile) -> String {
    use crate::digitalisiere::Abt1Eintrag;
    
    let mut abt1_eintraege = open_file.analysiert.abt1.eintraege.clone();
    if abt1_eintraege.is_empty() {
        abt1_eintraege = vec![Abt1Eintrag::new(1)];
    }
    
    let abt1 = abt1_eintraege.iter().enumerate().map(|(zeile_nr, abt1)| {
    
        let bv_geroetet = if abt1.ist_geroetet() { 
            "background:rgb(255,195,195);" 
        } else { 
            "background:white;" 
        };
        
        format!("
        <div class='__application-abt1-eintrag' style='display:flex;margin-top:5px;'>
        
            <input type='number' style='width: 30px;{bv_geroetet}' value='{lfd_nr}' 
                id='abt1_{zeile_nr}_lfd-nr'
                onkeyup='inputOnKeyDown(\"abt1:{zeile_nr}:lfd-nr\", event)'
                oninput='editText(\"abt1:{zeile_nr}:lfd-nr\", event)'
            />
            
            <textarea rows='3' cols='16' style='margin-bottom:2px;{bv_geroetet}'
                id='abt1_{zeile_nr}_eigentuemer'
                onkeyup='inputOnKeyDown(\"abt1:{zeile_nr}:eigentuemer\", event)'
                oninput='editText(\"abt1:{zeile_nr}:eigentuemer\", event)'
            >{eigentuemer}</textarea>
            
            <input type='text' style='margin-left:10px;width: 60px;{bv_geroetet}' value='{bv_nr}' 
                id='abt1_{zeile_nr}_bv-nr'
                onkeyup='inputOnKeyDown(\"abt1:{zeile_nr}:bv-nr\", event)'
                oninput='editText(\"abt1:{zeile_nr}:bv-nr\", event)'
            />
            
            <textarea rows='3' cols='25' style='margin-bottom:2px;{bv_geroetet}'
                id='abt1_{zeile_nr}_grundlage-der-eintragung'
                onkeyup='inputOnKeyDown(\"abt1:{zeile_nr}:grundlage-der-eintragung\", event)'
                oninput='editText(\"abt1:{zeile_nr}:grundlage-der-eintragung\", event)'
            >{grundlage_der_eintragung}</textarea>
            
            <div style='display:flex;flex-direction:row;flex-grow:1;'>
                <div style='display:flex;flex-grow:1'></div>
                <button onclick='eintragNeu(\"abt1:{zeile_nr}\")' tabindex='-1' class='btn btn_neu' >neu</button>
                <button onclick='eintragRoeten(\"abt1:{zeile_nr}\")' tabindex='-1' class='btn btn_roeten'>röten</button>
                <button onclick='eintragLoeschen(\"abt1:{zeile_nr}\")' tabindex='-1' class='btn btn_loeschen'>löschen</button>
            </div>
            
        </div>", 
            bv_geroetet = bv_geroetet,
            zeile_nr = zeile_nr,
            lfd_nr = abt1.lfd_nr,
            eigentuemer = abt1.eigentuemer,
            bv_nr = abt1.bv_nr,
            grundlage_der_eintragung = abt1.grundlage_der_eintragung,
        )
    }).collect::<Vec<String>>().join("\r\n");
    
    normalize_for_js(format!("
           <h4 style='position:sticky;top:0;background:white;padding:10px 0px;'>Abteilung 1</h4>
          
          <div class='__application-table-header'>
            <p style='width: 30px;'>Nr.</p>
            <p style='width: 160px;'>Eigentümer</p>
            <p style='width: 60px;'>BV-Nr.</p>
            <p style='width: 160px;'>Grundlage d. Eintragung</p>
          </div>
          
          {abt1}", abt1 = abt1))
}

pub fn render_abt_1_veraenderungen(open_file: &PdfFile) -> String {

    let mut abt1_veraenderungen = open_file.analysiert.abt1.veraenderungen.clone();
    if abt1_veraenderungen.is_empty() {
        abt1_veraenderungen = vec![Abt1Veraenderung::default()];
    }
    
    let abt1_veraenderungen = abt1_veraenderungen.iter().enumerate().map(|(zeile_nr, abt1_a)| {
        
        let bv_geroetet = if abt1_a.ist_geroetet() { 
            "background:rgb(255,195,195);" 
        } else { 
            "background:white;" 
        };
        
        format!("
        <div class='__application-bestandsverzeichnis-eintrag' style='display:flex;'>
            
            <input type='text' style='width: 90px;{bv_geroetet}' value='{lfd_nr}' 
                id='abt1-veraenderung_{zeile_nr}_lfd-nr'
                onkeyup='inputOnKeyDown(\"abt1-veraenderung:{zeile_nr}:lfd-nr\", event)' 
                oninput='editText(\"abt1-veraenderung:{zeile_nr}:lfd-nr\", event)'
            />
            
            <textarea rows='5' cols='45' style='{bv_geroetet}'
                id='abt1-veraenderung_{zeile_nr}_text'
                onkeyup='inputOnKeyDown(\"abt1-veraenderung:{zeile_nr}:text\", event)'
                oninput='editText(\"abt1-veraenderung:{zeile_nr}:text\", event)'
            >{text}</textarea>
            
            <div style='display:flex;flex-direction:row;flex-grow:1;'>
                <div style='display:flex;flex-grow:1'></div>
                <button onclick='eintragNeu(\"abt1-veraenderung:{zeile_nr}\")' tabindex='-1' class='btn btn_neu' >neu</button>
                <button onclick='eintragRoeten(\"abt1-veraenderung:{zeile_nr}\")' tabindex='-1' class='btn btn_roeten'>röten</button>
                <button onclick='eintragLoeschen(\"abt1-veraenderung:{zeile_nr}\")' tabindex='-1' class='btn btn_loeschen'>löschen</button>
            </div>
        </div>", 
            bv_geroetet = bv_geroetet,
            zeile_nr = zeile_nr,
            lfd_nr = abt1_a.lfd_nr,
            text = abt1_a.text,
        )
    }).collect::<Vec<String>>().join("\r\n");
    
    normalize_for_js(format!("
        <h4 style='position:sticky;top:0;background:white;padding:10px 0px;'>Abteilung 1 - Veränderungen</h4>
        
        <div class='__application-table-header'>
            <p style='width: 90px;'>lfd. Nr.</p>
            <p style='width: 160px;'>Text</p>
        </div>
        
        {abt1_veraenderungen}
    ", abt1_veraenderungen = abt1_veraenderungen))
}

pub fn render_abt_1_loeschungen(open_file: &PdfFile) -> String {

    let mut abt1_loeschungen = open_file.analysiert.abt1.loeschungen.clone();
    if abt1_loeschungen.is_empty() {
        abt1_loeschungen = vec![Abt1Loeschung::default()];
    }

    let abt1_loeschungen = abt1_loeschungen.iter().enumerate().map(|(zeile_nr, abt1_l)| {
        
        let bv_geroetet = if abt1_l.ist_geroetet() { 
            "background:rgb(255,195,195);" 
        } else { 
            "background:white;" 
        };
        
        format!("
        <div class='__application-bestandsverzeichnis-eintrag' style='display:flex;'>
            <input type='text' style='width: 90px;{bv_geroetet}' value='{lfd_nr}' 
                id='abt1-loeschung_{zeile_nr}_lfd-nr'
                onkeyup='inputOnKeyDown(\"abt1-loeschung:{zeile_nr}:lfd-nr\", event)' 
                oninput='editText(\"abt1-loeschung:{zeile_nr}:lfd-nr\", event)'
            />
            <textarea rows='5' cols='45' style='{bv_geroetet}'
                id='abt1-loeschung_{zeile_nr}_text'
                onkeyup='inputOnKeyDown(\"abt1-loeschung:{zeile_nr}:text\", event)'
                oninput='editText(\"abt1-loeschung:{zeile_nr}:text\", event)'
            >{text}</textarea>
            
            <div style='display:flex;flex-direction:row;flex-grow:1;'>
                <div style='display:flex;flex-grow:1'></div>
                <button onclick='eintragNeu(\"abt1-loeschung:{zeile_nr}\")' tabindex='-1' class='btn btn_neu' >neu</button>
                <button onclick='eintragRoeten(\"abt1-loeschung:{zeile_nr}\")' tabindex='-1' class='btn btn_roeten'>röten</button>
                <button onclick='eintragLoeschen(\"abt1-loeschung:{zeile_nr}\")' tabindex='-1' class='btn btn_loeschen'>löschen</button>
            </div>
        </div>", 
            bv_geroetet = bv_geroetet,
            zeile_nr = zeile_nr,
            lfd_nr = abt1_l.lfd_nr,
            text = abt1_l.text,
        )
    }).collect::<Vec<String>>().join("\r\n");
    
    normalize_for_js(format!("
        <h4 style='position:sticky;top:0;background:white;padding:10px 0px;'>Abteilung 1 - Löschungen</h4>
        
        <div class='__application-table-header'>
            <p style='width: 90px;'>lfd. Nr.</p>
            <p style='width: 160px;'>Text</p>
        </div>
        
        {abt1_loeschungen}
    ", abt1_loeschungen = abt1_loeschungen))
}

pub fn render_abt_2(open_file: &PdfFile) -> String {
    use crate::digitalisiere::Abt2Eintrag;
    
    let mut abt2_eintraege = open_file.analysiert.abt2.eintraege.clone();
    if abt2_eintraege.is_empty() {
        abt2_eintraege = vec![Abt2Eintrag::new(1)];
    }
    
    let abt2 = abt2_eintraege.iter().enumerate().map(|(zeile_nr, abt2)| {
    
        let bv_geroetet = if abt2.ist_geroetet() { 
            "background:rgb(255,195,195);" 
        } else { 
            "background:white;" 
        };
        
        format!("
        <div class='__application-abt2-eintrag' style='display:flex;margin-top:5px;'>
            
            <input type='number' style='width: 30px;{bv_geroetet}' value='{lfd_nr}' 
                id='abt2_{zeile_nr}_lfd-nr'
                onkeyup='inputOnKeyDown(\"abt2:{zeile_nr}:lfd-nr\", event)'
                oninput='editText(\"abt2:{zeile_nr}:lfd-nr\", event)'
            />
            
            <input type='text' style='width: 90px;{bv_geroetet}' value='{bv_nr}' 
                id='abt2_{zeile_nr}_bv-nr'
                onkeyup='inputOnKeyDown(\"abt2:{zeile_nr}:bv-nr\", event)'
                oninput='editText(\"abt2:{zeile_nr}:bv-nr\", event)'
            />
            
            <textarea rows='5' cols='45' style='{bv_geroetet}'
                id='abt2_{zeile_nr}_text'
                onkeyup='inputOnKeyDown(\"abt2:{zeile_nr}:text\", event)'
                oninput='editText(\"abt2:{zeile_nr}:text\", event)'
            >{recht}</textarea>
            
            <div style='display:flex;flex-direction:row;flex-grow:1;'>
                <div style='display:flex;flex-grow:1'></div>
                <button onclick='eintragNeu(\"abt2:{zeile_nr}\")' tabindex='-1' class='btn btn_neu' >neu</button>
                <button onclick='eintragRoeten(\"abt2:{zeile_nr}\")' tabindex='-1' class='btn btn_roeten'>röten</button>
                <button onclick='eintragLoeschen(\"abt2:{zeile_nr}\")' tabindex='-1' class='btn btn_loeschen'>löschen</button>
            </div>
        </div>", 
            bv_geroetet = bv_geroetet,
            zeile_nr = zeile_nr,
            lfd_nr = abt2.lfd_nr,
            bv_nr = abt2.bv_nr,
            recht = abt2.text,
        )
    }).collect::<Vec<String>>().join("\r\n");
    
    normalize_for_js(format!("
           <h4 style='position:sticky;top:0;background:white;padding:10px 0px;'>Abteilung 2</h4>
          
          <div class='__application-table-header'>
            <p style='width: 30px;'>Nr.</p>
            <p style='width: 90px;'>BV-Nr.</p>
            <p style='width: 160px;'>Recht</p>
          </div>
          
          {abt2}", abt2 = abt2))
}

pub fn render_abt_2_veraenderungen(open_file: &PdfFile) -> String {

    let mut abt2_veraenderungen = open_file.analysiert.abt2.veraenderungen.clone();
    if abt2_veraenderungen.is_empty() {
        abt2_veraenderungen = vec![Abt2Veraenderung::default()];
    }
    
    let abt2_veraenderungen = abt2_veraenderungen.iter().enumerate().map(|(zeile_nr, abt2_a)| {
        
        let bv_geroetet = if abt2_a.ist_geroetet() { 
            "background:rgb(255,195,195);" 
        } else { 
            "background:white;" 
        };
        
        
        format!("
        <div class='__application-bestandsverzeichnis-eintrag' style='display:flex;'>
            
            <input type='text' style='width: 90px;{bv_geroetet}' value='{lfd_nr}' 
                id='abt2-veraenderung_{zeile_nr}_lfd-nr'
                onkeyup='inputOnKeyDown(\"abt2-veraenderung:{zeile_nr}:lfd-nr\", event)' 
                oninput='editText(\"abt2-veraenderung:{zeile_nr}:lfd-nr\", event)'
            />
            
            <textarea rows='5' cols='45' style='{bv_geroetet}'
                id='abt2-veraenderung_{zeile_nr}_text'
                onkeyup='inputOnKeyDown(\"abt2-veraenderung:{zeile_nr}:text\", event)'
                oninput='editText(\"abt2-veraenderung:{zeile_nr}:text\", event)'
            >{text}</textarea>
            
            <div style='display:flex;flex-direction:row;flex-grow:1;'>
                <div style='display:flex;flex-grow:1'></div>
                <button onclick='eintragNeu(\"abt2-veraenderung:{zeile_nr}\")' tabindex='-1' class='btn btn_neu' >neu</button>
                <button onclick='eintragRoeten(\"abt2-veraenderung:{zeile_nr}\")' tabindex='-1' class='btn btn_roeten'>röten</button>
                <button onclick='eintragLoeschen(\"abt2-veraenderung:{zeile_nr}\")' tabindex='-1' class='btn btn_loeschen'>löschen</button>
            </div>
        </div>", 
            bv_geroetet = bv_geroetet,
            zeile_nr = zeile_nr,
            lfd_nr = abt2_a.lfd_nr,
            text = abt2_a.text,
        )
    }).collect::<Vec<String>>().join("\r\n");
    
    normalize_for_js(format!("
        <h4 style='position:sticky;top:0;background:white;padding:10px 0px;'>Abteilung 2 - Veränderungen</h4>
        
        <div class='__application-table-header'>
            <p style='width: 90px;'>lfd. Nr.</p>
            <p style='width: 160px;'>Text</p>
        </div>
        
        {abt2_veraenderungen}
    ", abt2_veraenderungen = abt2_veraenderungen))
}

pub fn render_abt_2_loeschungen(open_file: &PdfFile) -> String {

    let mut abt2_loeschungen = open_file.analysiert.abt2.loeschungen.clone();
    if abt2_loeschungen.is_empty() {
        abt2_loeschungen = vec![Abt2Loeschung::default()];
    }

    let abt2_loeschungen = abt2_loeschungen.iter().enumerate().map(|(zeile_nr, abt2_l)| {
    
        let bv_geroetet = if abt2_l.ist_geroetet() { 
            "background:rgb(255,195,195);" 
        } else { 
            "background:white;" 
        };
        
        format!("
        <div class='__application-bestandsverzeichnis-eintrag' style='display:flex;'>
            <input type='text' style='width: 90px;{bv_geroetet}' value='{lfd_nr}' 
                id='abt2-loeschung_{zeile_nr}_lfd-nr'
                onkeyup='inputOnKeyDown(\"abt2-loeschung:{zeile_nr}:lfd-nr\", event)' 
                oninput='editText(\"abt2-loeschung:{zeile_nr}:lfd-nr\", event)'
            />
            <textarea rows='5' cols='45' style='{bv_geroetet}'
                id='abt2-loeschung_{zeile_nr}_text'
                onkeyup='inputOnKeyDown(\"abt2-loeschung:{zeile_nr}:text\", event)'
                oninput='editText(\"abt2-loeschung:{zeile_nr}:text\", event)'
            >{text}</textarea>
            
            <div style='display:flex;flex-direction:row;flex-grow:1;'>
                <div style='display:flex;flex-grow:1'></div>
                <button onclick='eintragNeu(\"abt2-loeschung:{zeile_nr}\")' tabindex='-1' class='btn btn_neu' >neu</button>
                <button onclick='eintragRoeten(\"abt2-loeschung:{zeile_nr}\")' tabindex='-1' class='btn btn_roeten'>röten</button>
                <button onclick='eintragLoeschen(\"abt2-loeschung:{zeile_nr}\")' tabindex='-1' class='btn btn_loeschen'>löschen</button>
            </div>
        </div>", 
            bv_geroetet = bv_geroetet,
            zeile_nr = zeile_nr,
            lfd_nr = abt2_l.lfd_nr,
            text = abt2_l.text,
        )
    }).collect::<Vec<String>>().join("\r\n");
    
    normalize_for_js(format!("
        <h4 style='position:sticky;top:0;background:white;padding:10px 0px;'>Abteilung 2 - Löschungen</h4>
        
        <div class='__application-table-header'>
            <p style='width: 90px;'>lfd. Nr.</p>
            <p style='width: 160px;'>Text</p>
        </div>
        
        {abt2_loeschungen}
    ", abt2_loeschungen = abt2_loeschungen))
}

pub fn render_abt_3(open_file: &PdfFile) -> String {
    use crate::digitalisiere::Abt3Eintrag;

    let mut abt3_eintraege = open_file.analysiert.abt3.eintraege.clone();
    if abt3_eintraege.is_empty() {
        abt3_eintraege = vec![Abt3Eintrag::new(1)];
    }
    
    let abt3 = abt3_eintraege.iter().enumerate().map(|(zeile_nr, abt3)| {
            
        let bv_geroetet = if abt3.ist_geroetet() { 
            "background:rgb(255,195,195);" 
        } else { 
            "background:white;" 
        };
        
        format!("
        <div class='__application-abt2-eintrag' style='display:flex;margin-top:5px;'>
            
            <input type='number' style='width: 30px;{bv_geroetet}' value='{lfd_nr}' 
                id='abt3_{zeile_nr}_lfd-nr'
                onkeyup='inputOnKeyDown(\"abt3:{zeile_nr}:lfd-nr\", event)' 
                oninput='editText(\"abt3:{zeile_nr}:lfd-nr\", event)' 
            />
            <input type='text' style='width: 60px;{bv_geroetet}' value='{bv_nr}'
                id='abt3_{zeile_nr}_bv-nr'
                onkeyup='inputOnKeyDown(\"abt3:{zeile_nr}:bv-nr\", event)' 
                oninput='editText(\"abt3:{zeile_nr}:bv-nr\", event)' 
            />
            <input type='text' style='width: 120px;{bv_geroetet}' value='{betrag}' 
                id='abt3_{zeile_nr}_betrag'
                onkeyup='inputOnKeyDown(\"abt3:{zeile_nr}:betrag\", event)' 
                oninput='editText(\"abt3:{zeile_nr}:betrag\", event)' 
            />
            <textarea rows='5' cols='40' style='{bv_geroetet}'
                id='abt3_{zeile_nr}_text'
                onkeyup='inputOnKeyDown(\"abt3:{zeile_nr}:text\", event)'
                oninput='editText(\"abt3:{zeile_nr}:text\", event)'
            >{recht}</textarea>
            
            <div style='display:flex;flex-direction:row;flex-grow:1;'>
                <div style='display:flex;flex-grow:1'></div>
                <button onclick='eintragNeu(\"abt3:{zeile_nr}\")' tabindex='-1' class='btn btn_neu' >neu</button>
                <button onclick='eintragRoeten(\"abt3:{zeile_nr}\")' tabindex='-1' class='btn btn_roeten'>röten</button>
                <button onclick='eintragLoeschen(\"abt3:{zeile_nr}\")' tabindex='-1' class='btn btn_loeschen'>löschen</button>
            </div>
        </div>",
            bv_geroetet = bv_geroetet,
            zeile_nr = zeile_nr,
            lfd_nr = abt3.lfd_nr,
            bv_nr = abt3.bv_nr,
            betrag = abt3.betrag,
            recht = abt3.text,
        )
    }).collect::<Vec<String>>().join("\r\n");
    
    normalize_for_js(format!("
           <h4 style='position:sticky;top:0;background:white;padding:10px 0px;'>Abteilung 3</h4>
          
          <div class='__application-table-header'>
            <p style='width: 30px;'>Nr.</p>
            <p style='width: 60px;'>BV-Nr.</p>
            <p style='width: 120px;'>Betrag</p>
            <p style='width: 160px;'>Text</p>
          </div>
          
          {abt3}", abt3 = abt3))
}

pub fn render_abt_3_veraenderungen(open_file: &PdfFile) -> String {

    let mut abt3_veraenderungen = open_file.analysiert.abt3.veraenderungen.clone();
    if abt3_veraenderungen.is_empty() {
        abt3_veraenderungen = vec![Abt3Veraenderung::default()];
    }
    
    let abt3_veraenderungen = abt3_veraenderungen.iter().enumerate().map(|(zeile_nr, abt3_a)| {
        
        let bv_geroetet = if abt3_a.ist_geroetet() { 
            "background:rgb(255,195,195);" 
        } else { 
            "background:white;" 
        };
        
        format!("
        <div class='__application-abt3-eintrag' style='display:flex;'>
            
            <input type='text' style='width: 90px;{bv_geroetet}' value='{lfd_nr}' 
                id='abt3-veraenderung_{zeile_nr}_lfd-nr'
                onkeyup='inputOnKeyDown(\"abt3-veraenderung:{zeile_nr}:lfd-nr\", event)' 
                oninput='editText(\"abt3-veraenderung:{zeile_nr}:lfd-nr\", event)'
            />
            
            <input type='text' style='width: 120px;{bv_geroetet}' value='{betrag}' 
                id='abt3-veraenderung_{zeile_nr}_betrag'
                onkeyup='inputOnKeyDown(\"abt3-veraenderung:{zeile_nr}:betrag\", event)' 
                oninput='editText(\"abt3-veraenderung:{zeile_nr}:betrag\", event)' 
            />
            
            <textarea rows='5' cols='45' style='{bv_geroetet}'
                id='abt3-veraenderung_{zeile_nr}_text'
                onkeyup='inputOnKeyDown(\"abt3-veraenderung:{zeile_nr}:text\", event)'
                oninput='editText(\"abt3-veraenderung:{zeile_nr}:text\", event)'
            >{text}</textarea>
            
            <div style='display:flex;flex-direction:row;flex-grow:1;'>
                <div style='display:flex;flex-grow:1'></div>
                <button onclick='eintragNeu(\"abt3-veraenderung:{zeile_nr}\")' tabindex='-1' class='btn btn_neu' >neu</button>
                <button onclick='eintragRoeten(\"abt3-veraenderung:{zeile_nr}\")' tabindex='-1' class='btn btn_roeten'>röten</button>
                <button onclick='eintragLoeschen(\"abt3-veraenderung:{zeile_nr}\")' tabindex='-1' class='btn btn_loeschen'>löschen</button>
            </div>
        </div>",
            bv_geroetet = bv_geroetet,
            zeile_nr = zeile_nr,
            betrag = abt3_a.betrag,
            lfd_nr = abt3_a.lfd_nr,
            text = abt3_a.text,
        )
    }).collect::<Vec<String>>().join("\r\n");
    
    normalize_for_js(format!("
        <h4 style='position:sticky;top:0;background:white;padding:10px 0px;'>Abteilung 3 - Veränderungen</h4>
        
        <div class='__application-table-header'>
            <p style='width: 90px;'>lfd. Nr.</p>
            <p style='width: 120px;'>Betrag</p>
            <p style='width: 160px;'>Text</p>
        </div>
        
        {abt3_veraenderungen}
    ", abt3_veraenderungen = abt3_veraenderungen))
}

pub fn render_abt_3_loeschungen(open_file: &PdfFile) -> String {

    let mut abt3_loeschungen = open_file.analysiert.abt3.loeschungen.clone();
    if abt3_loeschungen.is_empty() {
        abt3_loeschungen = vec![Abt3Loeschung::default()];
    }

    let abt3_loeschungen = abt3_loeschungen.iter().enumerate().map(|(zeile_nr, abt3_l)| {
            
        let bv_geroetet = if abt3_l.ist_geroetet() { 
            "background:rgb(255,195,195);" 
        } else { 
            "background:white;" 
        };
        
        format!("
        <div class='__application-bestandsverzeichnis-eintrag' style='display:flex;'>
            <input type='text' style='width: 90px;{bv_geroetet}' value='{lfd_nr}' 
                id='abt3-loeschung_{zeile_nr}_lfd-nr'
                onkeyup='inputOnKeyDown(\"abt3-loeschung:{zeile_nr}:lfd-nr\", event)' 
                oninput='editText(\"abt3-loeschung:{zeile_nr}:lfd-nr\", event)'
            />
                        
            <input type='text' style='width: 120px;{bv_geroetet}' value='{betrag}' 
                id='abt3-loeschung_{zeile_nr}_betrag'
                onkeyup='inputOnKeyDown(\"abt3-loeschung:{zeile_nr}:betrag\", event)' 
                oninput='editText(\"abt3-loeschung:{zeile_nr}:betrag\", event)' 
            />
            
            <textarea rows='5' cols='45' style='{bv_geroetet}'
                id='abt3-loeschung_{zeile_nr}_text'
                onkeyup='inputOnKeyDown(\"abt3-loeschung:{zeile_nr}:text\", event)'
                oninput='editText(\"abt3-loeschung:{zeile_nr}:text\", event)'
            >{text}</textarea>
            
            <div style='display:flex;flex-direction:row;flex-grow:1;'>
                <div style='display:flex;flex-grow:1'></div>
                <button onclick='eintragNeu(\"abt3-loeschung:{zeile_nr}\")' tabindex='-1' class='btn btn_neu' >neu</button>
                <button onclick='eintragRoeten(\"abt3-loeschung:{zeile_nr}\")' tabindex='-1' class='btn btn_roeten'>röten</button>
                <button onclick='eintragLoeschen(\"abt3-loeschung:{zeile_nr}\")' tabindex='-1' class='btn btn_loeschen'>löschen</button>
            </div>
        </div>", 
            bv_geroetet = bv_geroetet,
            zeile_nr = zeile_nr,
            betrag = abt3_l.betrag,
            lfd_nr = abt3_l.lfd_nr,
            text = abt3_l.text,
        )
    }).collect::<Vec<String>>().join("\r\n");
    
    normalize_for_js(format!("
        <h4 style='position:sticky;top:0;background:white;padding:10px 0px;'>Abteilung 3 - Löschungen</h4>
        
        <div class='__application-table-header'>
            <p style='width: 90px;'>lfd. Nr.</p>
            <p style='width: 120px;'>Betrag</p>
            <p style='width: 160px;'>Text</p>
        </div>
        
        {abt3_loeschungen}
    ", abt3_loeschungen = abt3_loeschungen))
}

pub fn render_pdf_image(rpc_data: &RpcData) -> String {

    let open_file = match rpc_data.open_page.clone() {
        Some(s) => s,
        None => { return String::new() },
    };
    
    let file = match rpc_data.loaded_files.get(&open_file.0) {
        Some(s) => s,
        None => { return String::new() },
    };
    
    let max_seitenzahl = file.seitenzahlen.iter().copied().max().unwrap_or(0);
    
    let temp_ordner = std::env::temp_dir()
    .join(&format!("{gemarkung}/{blatt}", gemarkung = file.titelblatt.grundbuch_von, blatt = file.titelblatt.blatt));
    
    let temp_pdf_pfad = temp_ordner.clone().join("temp.pdf");
    let pdftoppm_output_path = if rpc_data.konfiguration.vorschau_ohne_geroetet {
        temp_ordner.clone().join(format!("page-clean-{}.png", crate::digitalisiere::formatiere_seitenzahl(open_file.1, max_seitenzahl)))
    } else {
        temp_ordner.clone().join(format!("page-{}.png", crate::digitalisiere::formatiere_seitenzahl(open_file.1, max_seitenzahl)))
    };
    
    if !pdftoppm_output_path.exists() {
        if let Ok(o) = std::fs::read(&file.datei) {
            let _ = crate::digitalisiere::konvertiere_pdf_seite_zu_png_prioritaet(
                &o, 
                &[open_file.1], 
                &file.titelblatt, 
                !rpc_data.konfiguration.vorschau_ohne_geroetet
            );
        }
    }
    
    let pdf_to_ppm_bytes = match std::fs::read(&pdftoppm_output_path) {
        Ok(o) => o,
        Err(_) => return String::new(),
    };

    let (im_width, im_height, page_width, page_height) = match file.pdftotext_layout.seiten.get(&open_file.1) {
        Some(o) => (o.breite_mm as f32 / 25.4 * 600.0, o.hoehe_mm as f32 / 25.4 * 600.0, o.breite_mm, o.hoehe_mm),
        None => return String::new(),
    };
    
    let img_ui_width = 1200.0; // px
    let aspect_ratio = im_height / im_width;
    let img_ui_height = img_ui_width * aspect_ratio;
    
    let columns = match file.geladen.get(&open_file.1) {
        Some(page) =>  {
            
            let seitentyp = match file.klassifikation_neu.get(&(open_file.1 as usize)) {
                Some(s) => *s,
                None => page.typ,
            };
                                                    
            seitentyp
            .get_columns(file.anpassungen_seite.get(&(open_file.1 as usize)))
            .into_iter()
            .map(|col| {
            
                let x = col.min_x / page_width * img_ui_width;
                let y = col.min_y / page_height * img_ui_height;
                let width = (col.max_x - col.min_x) / page_width * img_ui_width;
                let height = (col.max_y - col.min_y) / page_height * img_ui_height;
                
                format!("
                    <div class='__application_spalte' id='__application_spalte_{id}' style='
                        position:absolute;
                        width:{width}px;
                        height:{height}px;
                        opacity: 0.5;
                        background:none;
                        border: 3px solid blue;
                        top: 0px;
                        transform-origin: top left;
                        left: 0px;
                        transform: translate({x}px, {y}px);
                        pointer-events:none;
                    '>
                        <div style='
                            position:absolute;
                            width:15px;
                            height:15px;
                            background:none;
                            top:-7.5px;
                            left:-7.5px;
                            cursor:nw-resize;
                            z-index:1;
                            pointer-events: initial;
                        '   
                            data-columnId = '{id}' 
                            data-direction='nw' 
                            onmousedown='resizeColumnOnMouseDown(event);' 
                            onmouseup='resizeColumnOnMouseUp(event);'
                            onmouseout='resizeColumnOnMouseUp(event);'
                            onmousemove='resizeColumn(event);'></div>
                        
                        <div style='
                            position:absolute;
                            width:15px;
                            height:15px;
                            background:none;
                            top:-7.5px;
                            right:-7.5px;
                            cursor:ne-resize;
                            z-index:1;
                            pointer-events: initial;
                        ' 
                            data-columnId = '{id}' 
                            data-direction='ne' 
                            onmousedown='resizeColumnOnMouseDown(event);' 
                            onmouseup='resizeColumnOnMouseUp(event);'
                            onmouseout='resizeColumnOnMouseUp(event);'
                            onmousemove='resizeColumn(event);'></div>
                        
                        <div style='
                            position:absolute;
                            width:15px;
                            height:15px;
                            background:none;
                            bottom:-7.5px;
                            right:-7.5px;
                            cursor:se-resize;
                            z-index:1;
                            pointer-events: initial;
                        ' 
                        data-columnId = '{id}' 
                        data-direction='se' 
                            onmousedown='resizeColumnOnMouseDown(event);' 
                            onmouseup='resizeColumnOnMouseUp(event);'
                            onmouseout='resizeColumnOnMouseUp(event);'
                            onmousemove='resizeColumn(event);'></div>
                        
                        <div style='
                            position:absolute;
                            width:15px;
                            height:15px;
                            background:none;
                            bottom:-7.5px;
                            left:-7.5px;
                            cursor:sw-resize;
                            z-index:1;
                            pointer-events: initial;
                        ' 
                            data-columnId = '{id}' 
                            data-direction='sw' 
                            onmousedown='resizeColumnOnMouseDown(event);'
                            onmouseup='resizeColumnOnMouseUp(event);'
                            onmouseout='resizeColumnOnMouseUp(event);'
                            onmousemove='resizeColumn(event);'
                        ></div>
                    </div>
                ",
                    id = col.id,
                    x = x,
                    y = y,
                    width = width.abs(),
                    height = height.abs(),
                )
            })
            .collect::<Vec<_>>()
            .join("\r\n")
        },
        None => { String::new() },
    };
    
    let zeilen = file.anpassungen_seite
    .get(&(open_file.1 as usize))
    .map(|ap| ap.zeilen.clone())
    .unwrap_or_default();
    
    let zeilen = render_pdf_image_zeilen(&zeilen, page_height, img_ui_height);
    
    normalize_for_js(format!("
        <div style='padding:20px;user-select:none;-webkit-user-select:none;'>
            <div data-fileName='{file_name}' data-pageNumber='{page_number}' style='position:relative;user-select:none;-webkit-user-select:none;margin:0 auto;'>
                
                <img id='__application_page_img_inner' 
                src='data:image/png;base64,{img_src}'
                onmousedown='onOcrSelectionDragStart(event);'
                onmousemove='onOcrSelectionDrag(event);' 
                onmouseup='onOcrSelectionDragStop(event);' 
                onmouseout='onOcrSelectionDragStop(event);' 
                style='
                    user-select:none;
                    width:1200px;
                    height:{img_ui_height}px;
                    -webkit-user-select:none;
                    cursor:crosshair;
                ' />            
            
                {spalten}
                
                <div id='__application_page_lines' style='
                    height:{img_ui_height}px;
                    position:absolute;
                    top:0px;
                    left:-20px;
                    width:40px;
                    cursor:pointer;
                    background:repeating-linear-gradient(to bottom, #ccc 0%, #ccc 10%, white 11%, white 100%);
                    background-size: 10px 10px;
                ' data-fileName='{file_name}' data-pageNumber='{page_number}' 
                onmouseenter='zeilePreviewShow(event);' 
                onmouseleave='zeilePreviewHide(event);'
                onmouseover='zeilePreviewMove(event);'
                onmousemove='zeilePreviewMove(event);'
                onmouseup='zeileNeu(event);'>{zeilen}</div>
                
                <div id='__application_ocr_selection' style='
                    position:absolute;
                    width:1px;
                    height:1px;
                    opacity: 0.5;
                    background:transparent;
                    top: 0px;
                    transform-origin: top left;
                    left: 0px;
                    transform: translate(0px, 0px) scale(1.0, 1.0);
                    pointer-events:none;
                '></div>
                
            </div>
        </div>", 
            file_name = open_file.0,
            page_number = open_file.1,
            img_src = base64::encode(pdf_to_ppm_bytes),
            img_ui_height = img_ui_height,
            zeilen = format!("
                <div id='__application_zeilen' style='
                    position:absolute;
                    top: 0px;
                    left: 0px;
                    transform-origin: top left;
                    width:0px;
                    height:0px;
                    overflow:visible;
                    background: transparent;
                    pointer-events:all;
                '  data-fileName='{file_name}' data-pageNumber='{page_number}'>
                    {zeilen}
                </div>", 
                    zeilen = zeilen, 
                    file_name = open_file.0,
                    page_number = open_file.1,
                ),
            spalten = if !rpc_data.konfiguration.spalten_ausblenden {
                format!("
                <div id='__application_spalten' style='
                    position:absolute;
                    top: 0px;
                    left: 0px;
                    transform-origin: top left;
                    width:0px;
                    height:0px;
                    overflow:visible;
                    background: transparent;
                '>
                    {columns}
                </div>
                ", columns = columns)
            } else {
                String::new()
            }
        ))
}

pub fn render_pdf_image_zeilen(zeilen: &[f32], page_height: f32, img_ui_height: f32) -> String {
    
    let mut z1 = zeilen.iter().enumerate().map(|(zeile_id, y)| format!("
        <div class='__application_zeile' id='__application_zeile_{id}' style='
            position:absolute;
            width:50px;
            height:20px;
            left:-5px;
            background:white;
            border: 1px solid #222;
            box-shadow:0px 0px 3px #ccccccee;
            transform-origin: top left;
            transform: translateY({y}px);
        ' data-zeileId='{id}' onmouseup='zeileLoeschen(event);'>
            <div style='pointer-events:none;width:1195px;position:absolute;height:2px;background:blue;opacity:0.5;left:50px;top:9px;'></div>
        </div>
    ", id = zeile_id, y = (y / page_height * img_ui_height) - 10.0))
    .collect::<Vec<_>>()
    .join("\r\n");
    
    
    z1.push_str("
        <div class='__application_zeile' id='__application_zeile_preview' style='
            position:absolute;
            width:50px;
            height:20px;
            left:-5px;
            background:white;
            border: 1px solid #222;
            box-shadow:0px 0px 3px #ccccccee;
            transform-origin: top left;
            transform: translateY(0px);
            pointer-events:none;
            opacity:0;
        '>
            <div style='pointer-events:none;width:1195px;position:absolute;height:2px;background:blue;opacity:0.5;left:50px;top:9px;'></div>
        </div>
    ");
    
    normalize_for_js(z1)
}

pub fn normalize_for_js(s: String) -> String {
    s.lines().map(|s| s.trim()).collect::<Vec<_>>().join("")
}

