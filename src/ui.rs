use crate::{
    RpcData, PdfFile, 
    Konfiguration,
    digitalisiere::{
        Nebenbeteiligter,
        BvZuschreibung,
        BvAbschreibung,
    },
};

// render entire <body> node depending on the state of the rpc_data
pub fn render_entire_screen(rpc_data: &mut RpcData) -> String {
    normalize_for_js(format!("
            {popover}
            {ribbon_ui}
            <div id='__application-main'>
                {main}
            </div>
        ",
        popover = render_popover(rpc_data),
        ribbon_ui = render_ribbon(rpc_data),
        main = render_main(rpc_data),
    ))
}

pub fn render_popover(rpc_data: &RpcData) -> String {
    
    const ICON_CLOSE: &[u8] = include_bytes!("./img/icons8-close-48.png");
    
    let should_render_popover = 
        rpc_data.configuration_active ||
        rpc_data.info_active ||
        rpc_data.context_menu_active.is_some();
    
    if !should_render_popover {
        return String::new();
    }
    
    let application_popover_color = if rpc_data.configuration_active || rpc_data.info_active {
        "rgba(0, 0, 0, 0.5)"
    } else {
        "transparent"
    };
    
    let icon_close_base64 = base64::encode(ICON_CLOSE);

    let popover = format!("
        <div id='__application_popover' style='background:{application_popover_color};width: 100%;height: 100%;min-height: 100%;position: fixed;z-index:1000' onmousedown='closePopOver()'>
            {popover_content}
        </div>
    ", 
        application_popover_color = application_popover_color, 
        popover_content = 
        if rpc_data.info_active {
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
                <div style='width:1200px;overflow:scroll;display:flex;flex-direction:column;margin:10px auto;border:1px solid grey;background:white;padding:10px 100px;' onmousedown='event.stopPropagation();'>
                    <h2 style='font-size:20px;padding-bottom:10px;font-family:sans-serif;'>Konfiguration</h2>
                    <p style='font-size:12px;padding-bottom:5px;'>Pfad: {konfig_pfad}</p>
                    
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
                            <p style='font-family:sans-serif;font-weight:bold;font-size:16px;padding-bottom:10px;'>
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
            <div style='padding:1px;position:absolute;left:{}px;top:{}px;background:white;border-radius:5px;box-shadow:0px 0px 5px #444;'>
            <div style='border:1px solid #efefef;border-radius:5px;'>
                <p style='padding:5px 10px;font-size:10px;color:#444;margin-bottom:5px;'>Klassifiziere Seite als...
                <div style='line-height:1.5;cursor:pointer;'>
                    <div class='kontextmenü-eintrag' data-seite-neu='bv-horz' data-seite='{seite}' onclick='klassifiziereSeiteNeu(event);'>
                        Bestandsverzeichnis (Querformat)
                    </div>
                    <div class='kontextmenü-eintrag' data-seite-neu='bv-horz-zu-und-abschreibungen' data-seite='{seite}' onclick='klassifiziereSeiteNeu(event);'>
                        Bestandsverzeichnis Zu- und Abschreibungen (Querformat)
                    </div>
                    <div class='kontextmenü-eintrag' data-seite-neu='bv-vert' data-seite='{seite}' onclick='klassifiziereSeiteNeu(event);'>
                        Bestandsverzeichnis (Hochformat)
                    </div>
                    <div class='kontextmenü-eintrag' data-seite-neu='bv-vert-zu-und-abschreibungen' data-seite='{seite}' onclick='klassifiziereSeiteNeu(event);'>
                        Bestandsverzeichnis Zu- und Abschreibungen (Hochformat)
                    </div>
                    <div class='kontextmenü-eintrag' data-seite-neu='abt1-horz' data-seite='{seite}' onclick='klassifiziereSeiteNeu(event);'>
                        Abteilung 1 (Querformat)
                    </div>
                    <div class='kontextmenü-eintrag' data-seite-neu='abt1-vert' data-seite='{seite}' onclick='klassifiziereSeiteNeu(event);'>
                        Abteilung 1 (Hochformat)
                    </div>
                    <div class='kontextmenü-eintrag' data-seite-neu='abt2-horz-veraenderungen' data-seite='{seite}' onclick='klassifiziereSeiteNeu(event);'>
                        Abteilung 2 Veränderungen (Querformat)
                    </div>
                    <div class='kontextmenü-eintrag' data-seite-neu='abt2-horz' data-seite='{seite}' onclick='klassifiziereSeiteNeu(event);'>
                        Abteilung 2 (Querformat)
                    </div>
                    <div class='kontextmenü-eintrag' data-seite-neu='abt2-vert-veraenderungen' data-seite='{seite}' onclick='klassifiziereSeiteNeu(event);'>
                        Abteilung 2 Veränderungen (Hochformat)
                    </div>
                    <div class='kontextmenü-eintrag' data-seite-neu='abt2-vert' data-seite='{seite}' onclick='klassifiziereSeiteNeu(event);'>
                        Abteilung 2 (Hochformat)
                    </div>
                    <div class='kontextmenü-eintrag' data-seite-neu='abt3-horz-veraenderungen' data-seite='{seite}' onclick='klassifiziereSeiteNeu(event);'>
                        Abteilung 3 Veränderungen (Querformat)
                    </div>
                    <div class='kontextmenü-eintrag' data-seite-neu='abt3-horz-loeschungen' data-seite='{seite}' onclick='klassifiziereSeiteNeu(event);'>
                        Abteilung 3 Löschungen (Querformat)
                    </div>
                    <div class='kontextmenü-eintrag' data-seite-neu='abt3-horz' data-seite='{seite}' onclick='klassifiziereSeiteNeu(event);'>
                        Abteilung 3 (Querformat)
                    </div>
                    <div class='kontextmenü-eintrag' data-seite-neu='abt3-vert-veraenderungen' data-seite='{seite}' onclick='klassifiziereSeiteNeu(event);'>
                        <p>Abteilung 3 Veränderungen (Hochformat)
                    </div>
                    <div class='kontextmenü-eintrag' data-seite-neu='abt3-vert-loeschungen' data-seite='{seite}' onclick='klassifiziereSeiteNeu(event);'>
                        Abteilung 3 Löschungen (Hochformat)
                    </div>
                    <div class='kontextmenü-eintrag' data-seite-neu='abt3-vert' data-seite='{seite}' onclick='klassifiziereSeiteNeu(event);'>
                        Abteilung 3 (Hochformat)
                    </div>
                </div>
            </div>
            </div>", cm.x, cm.y, seite = cm.seite_ausgewaehlt)
        } else { 
            format!("") 
        }
    );
    
    normalize_for_js(popover)
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
        return String::new();
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

    normalize_for_js(rpc_data.loaded_files.keys().map(|filename| {
        let datei_ausgewaehlt = rpc_data.open_page.as_ref().map(|s| s.0.as_str()) == Some(filename);
        
        format!("<div class='{file_active}' style='user-select:none;display:flex;flex-direction:row;' data-fileName='{filename}' onmouseup='activateSelectedFile(event);'>
            <p style='flex-grow:0;user-select:none;' data-fileName='{filename}' >{filename}</p>
            <div style='display:flex;flex-grow:1;' data-fileName='{filename}' ></div>
            {close_btn}
            </div>", 
            file_active = if datei_ausgewaehlt { "active" } else { "" },
            filename = filename, 
            close_btn = if datei_ausgewaehlt { 
                format!(
                    "<img style='width: 16px;height: 16px;padding: 2px;flex-grow: 0;cursor: pointer;' data-fileName='{filename}' onmouseup='closeFile(event);'src='{close_str}'></img>", 
                    filename = filename, 
                    close_str = close_str
                ) 
            } else { 
                String::new() 
            },
        )
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
              SeitenTyp::Abt3HorzVeraenderungen
            | SeitenTyp::Abt3HorzLoeschungen
            | SeitenTyp::Abt3Horz
            | SeitenTyp::Abt3VertVeraenderungen
            | SeitenTyp::Abt3VertLoeschungen
            | SeitenTyp::Abt3Vert => {
                "rgb(255,200,167)" // orange
            },        
        }).unwrap_or("white");
        
        format!(
            "<div class='__application-page {loaded} {active}' oncontextmenu='openContextMenu(event);' data-pageNumber='{page_num}' {extra_style} {onclick}>{page_num}</div>",
            loaded = if page_is_loaded { "loaded" } else { "" },
            active = if page_is_active { "active" } else { "" },
            onclick = "onclick='activateSelectedPage(event)'",
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
    
    if !open_file.ist_geladen() {
        normalize_for_js(format!("
                <div style='height: 100%;padding:10px;display:flex;flex-grow:1;align-items:center;justify-content:center;'>
                    <h2 style='font-size: 16px;font-weight:bold;'>Grundbuch wird geladen...</h2>
                </div>
            ",
        ))
    } else {
        normalize_for_js(format!("
                <div style='height:43px;border-bottom: 1px solid #efefef;box-sizing:border-box;'>
                    <div style='display:inline-block;width:50%;overflow:hidden;'>
                        <h4 style='padding:10px;font-size:16px;'>Grundbuch</h4>
                    </div>
                    <div style='display:inline-block;width:50%;overflow:hidden;'>
                        <h4 style='padding:10px;font-size:16px;'>LEFIS</h4>
                    </div>
                </div>
                <div style='display:flex;flex-grow:1;flex-direction:row;padding:0px;'>
                    <div style='display:inline-block;width:50%;overflow:scroll;max-height:557px;'>
                        <div id='__application-bestandsverzeichnis' style='margin:10px;'>{bestandsverzeichnis}</div>
                        <div id='__application-bestandsverzeichnis-aenderungen' style='margin:10px;'>{bestandsverzeichnis_zuschreibungen}</div>
                        <div id='__application-bestandsverzeichnis-loeschungen' style='margin:10px;'>{bestandsverzeichnis_abschreibungen}</div>
                        <div id='__application-abteilung-2' style='margin:10px;'>{abt_2}</div>
                        <div id='__application-abteilung-3' style='margin:10px;'>{abt_3}</div>
                    </div>
                    <div id='__application-analyse-grundbuch' style='display:inline-block;width:50%;overflow:scroll;max-height:557px;'>
                        {analyse}
                    </div>
                </div>
            ",
            bestandsverzeichnis = render_bestandsverzeichnis(open_file),
            bestandsverzeichnis_zuschreibungen = render_bestandsverzeichnis_zuschreibungen(open_file),
            bestandsverzeichnis_abschreibungen = render_bestandsverzeichnis_abschreibungen(open_file),
            abt_2 = render_abt_2(open_file),
            abt_3 = render_abt_3(open_file),
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
                        format!("<span style='display:flex;'>
                            <img src='{pfeil}' style='width:12px;height:12px;'/>
                            <p style='display:inline-block;margin-left:10px;'>Fl. {flur}, Flst. {flurstueck} (BV-Nr. {bv_nr})</p>
                            </span>", 
                            pfeil = pfeil_str,
                            flur = belastet.flur,
                            flurstueck = belastet.flurstueck,
                            bv_nr = belastet.lfd_nr,
                        ) 
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
                        format!("<span style='display:flex;'>
                            <img src='{pfeil}' style='width:12px;height:12px;'/>
                            <p style='display:inline-block;margin-left:10px;'>Fl. {flur}, Flst. {flurstueck} (BV-Nr. {bv_nr})</p>
                            </span>", 
                            pfeil = pfeil_str,
                            flur = belastet.flur,
                            flurstueck = belastet.flurstueck,
                            bv_nr = belastet.lfd_nr,
                        ) 
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

pub fn render_bestandsverzeichnis(open_file: &mut PdfFile) -> String {
    use crate::digitalisiere::BvEintrag;

    crate::analysiere::roete_bestandsverzeichnis_automatisch(&mut open_file.analysiert.bestandsverzeichnis);

    let mut bestandsverzeichnis = open_file.analysiert.bestandsverzeichnis.clone();
    if bestandsverzeichnis.eintraege.is_empty() {
        bestandsverzeichnis.eintraege = vec![BvEintrag::new(1)];
    }
    
    let bv = bestandsverzeichnis.eintraege.iter().enumerate().map(|(zeile_nr, bve)| {
        
        let bv_geroetet = if bve.ist_geroetet() { 
            "background:rgb(255,195,195);" 
        } else { 
            "background:white;" 
        };
        
        format!("
        <div class='__application-bestandsverzeichnis-eintrag' style='display:flex;'>
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
                <button onclick='eintragNeu(\"bv:{zeile_nr}\")' class='btn btn_neu' >neu</button>
                <button onclick='eintragRoeten(\"bv:{zeile_nr}\")' class='btn btn_roeten'>röten</button>
                <button onclick='eintragLoeschen(\"bv:{zeile_nr}\")' class='btn btn_loeschen'>löschen</button>
            </div>
        </div>", 
            bv_geroetet = bv_geroetet,
            zeile_nr = zeile_nr,
            lfd_nr = format!("{}", bve.lfd_nr),
            bisherige_lfd_nr = bve.bisherige_lfd_nr.map(|f| format!("{}", f)).unwrap_or_default(),
            flur = format!("{}", bve.flur),
            flurstueck = format!("{}", bve.flurstueck),
            gemarkung = bve.gemarkung.clone().unwrap_or_default(),
        )
    }).collect::<Vec<String>>().join("\r\n");
    
    normalize_for_js(format!("
        <h4 style='position:sticky;top:0;background:white;padding:10px 0px;'>Bestandsverzeichnis</h4>
        
        <div class='__application-table-header'>
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
        format!("
        <div class='__application-bestandsverzeichnis-eintrag' style='display:flex;'>
            <input type='text' style='width: 90px;' value='{bv_nr}' 
                id='bv-zuschreibung_{zeile_nr}_bv-nr'
                onkeyup='inputOnKeyDown(\"bv-zuschreibung:{zeile_nr}:bv-nr\", event)' 
                oninput='editText(\"bv-zuschreibung:{zeile_nr}:bv-nr\", event)'
            />
            <textarea rows='5' cols='45' 
                id='bv-zuschreibung_{zeile_nr}_text'
                onkeyup='inputOnKeyDown(\"bv-zuschreibung:{zeile_nr}:text\", event)'
                oninput='editText(\"bv-zuschreibung:{zeile_nr}:text\", event)'
            >{text}</textarea>
            <div style='display:flex;flex-direction:row;flex-grow:1;'>
                <div style='display:flex;flex-grow:1'></div>
                <button onclick='eintragNeu(\"bv-zuschreibung:{zeile_nr}\")' class='btn btn_neu' >neu</button>
                <button onclick='eintragRoeten(\"bv-zuschreibung:{zeile_nr}\")' class='btn btn_roeten'>röten</button>
                <button onclick='eintragLoeschen(\"bv-zuschreibung:{zeile_nr}\")' class='btn btn_loeschen'>löschen</button>
            </div>
        </div>", 
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
        format!("
        <div class='__application-bestandsverzeichnis-eintrag' style='display:flex;'>
            <input type='text' style='width: 90px;' value='{bv_nr}' 
                id='bv-abschreibung_{zeile_nr}_bv-nr'
                onkeyup='inputOnKeyDown(\"bv-abschreibung:{zeile_nr}:bv-nr\", event)' 
                oninput='editText(\"bv-abschreibung:{zeile_nr}:bv-nr\", event)'
            />
            <textarea rows='5' cols='45' 
                id='bv-abschreibung_{zeile_nr}_text'
                onkeyup='inputOnKeyDown(\"bv-abschreibung:{zeile_nr}:text\", event)'
                oninput='editText(\"bv-abschreibung:{zeile_nr}:text\", event)'
            >{text}</textarea>
            <div style='display:flex;flex-direction:row;flex-grow:1;'>
                <div style='display:flex;flex-grow:1'></div>
                <button onclick='eintragNeu(\"bv-abschreibung:{zeile_nr}\")' class='btn btn_neu' >neu</button>
                <button onclick='eintragRoeten(\"bv-abschreibung:{zeile_nr}\")' class='btn btn_roeten'>röten</button>
                <button onclick='eintragLoeschen(\"bv-abschreibung:{zeile_nr}\")' class='btn btn_loeschen'>löschen</button>
            </div>
        </div>", 
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
                <button onclick='eintragNeu(\"abt2:{zeile_nr}\")' class='btn btn_neu' >neu</button>
                <button onclick='eintragRoeten(\"abt2:{zeile_nr}\")' class='btn btn_roeten'>röten</button>
                <button onclick='eintragLoeschen(\"abt2:{zeile_nr}\")' class='btn btn_loeschen'>löschen</button>
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
                <button onclick='eintragNeu(\"abt3:{zeile_nr}\")' class='btn btn_neu' >neu</button>
                <button onclick='eintragRoeten(\"abt3:{zeile_nr}\")' class='btn btn_roeten'>röten</button>
                <button onclick='eintragLoeschen(\"abt3:{zeile_nr}\")' class='btn btn_loeschen'>löschen</button>
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

pub fn render_pdf_image(rpc_data: &RpcData) -> String {

    let open_file = match rpc_data.open_page.clone() {
        Some(s) => s,
        None => { return String::new() },
    };
    
    let file = match rpc_data.loaded_files.get(&open_file.0) {
        Some(s) => s,
        None => { return String::new() },
    };
    
    let page = match file.geladen.get(&open_file.1) {
        Some(s) => s,
        None => { return String::new() },
    };
    
    let max_seitenzahl = file.seitenzahlen.iter().copied().max().unwrap_or(0);
    
    let temp_ordner = std::env::temp_dir()
    .join(&format!("{gemarkung}/{blatt}", gemarkung = file.titelblatt.grundbuch_von, blatt = file.titelblatt.blatt));
    
    let temp_pdf_pfad = temp_ordner.clone().join("temp.pdf");
    let pdftoppm_output_path = temp_ordner.clone().join(format!("page-{}.png", crate::digitalisiere::formatiere_seitenzahl(open_file.1, max_seitenzahl)));
    
    if !pdftoppm_output_path.exists() {
        if let Ok(o) = std::fs::read(&file.datei) {
            let _ = crate::digitalisiere::konvertiere_pdf_seiten_zu_png(&o, &[open_file.1], &file.titelblatt);
        }
    }
    
    let pdf_to_ppm_bytes = match std::fs::read(&pdftoppm_output_path) {
        Ok(o) => o,
        Err(_) => return String::new(),
    };
    
    normalize_for_js(format!("<div style='padding:20px;'><img src='data:image/png;base64,{}'/></div>", base64::encode(pdf_to_ppm_bytes)))
}

pub fn normalize_for_js(s: String) -> String {
    s.lines().map(|s| s.trim()).collect::<Vec<_>>().join("")
}

