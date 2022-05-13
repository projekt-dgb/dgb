use crate::{
    RpcData, PdfFile, GrundbuchSucheResponse,
    Konfiguration, PopoverState, GbxAenderungen,
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
        TextInputType,
        StringOrLines,
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
    
    let should_render_popover = rpc_data.popover_state.is_some();
        
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

pub fn render_aenderungen_dateien(aenderungen: &GbxAenderungen, aktiv: usize) -> String {
    let mut out = String::new();
    
    for (i, file_name) in aenderungen.neue_dateien.keys().enumerate() {
        let selected = match aktiv == i {
            true => "class='selected'",
            false => "",
        };
        out.push_str(&format!("<p {selected} onmousedown='switchAenderungView({i})'>{file_name}.gbx</p>\r\n"));
    }
    
    for (i, file_name) in aenderungen.geaenderte_dateien.keys().enumerate() {
        let i = i + aenderungen.neue_dateien.len();
        let selected = match aktiv == i {
            true => "class='selected'",
            false => "",
        };
        out.push_str(&format!("<p {selected} onmousedown='switchAenderungView({i})'>{file_name}.gbx</p>\r\n"));
    }
    
    normalize_for_js(out)
}

pub fn render_aenderung_diff(aenderungen: &GbxAenderungen, aktiv: usize) -> String {
        
    if aktiv < aenderungen.neue_dateien.len() {
        
        let neu = match aenderungen.neue_dateien.iter().nth(aktiv) {
            Some((_, file)) => file,
            None => return String::new(),
        };
        
        let neu_json = match serde_json::to_string_pretty(&neu) {
            Ok(o) => o,
            Err(_) => return String::new(),
        };
        
        let mut out = format!("<div>");
        for line in neu_json.lines() {
            out.push_str(&format!("<span class='insert'><p>+</p><p>{}</p></span>", line.replace(" ", "&nbsp;")))
        }
        out.push_str("</div>");
        out
        
    } else if aktiv < aenderungen.neue_dateien.len() + aenderungen.geaenderte_dateien.len() {

        use crate::GbxAenderung;
        use prettydiff::basic::DiffOp;
        
        let (alt, neu) = match aenderungen.geaenderte_dateien.iter().nth(aktiv - aenderungen.neue_dateien.len()) {
            Some((_, GbxAenderung { alt, neu })) => (alt, neu),
            None => return String::new(),
        };
        
        let alt_json = match serde_json::to_string_pretty(&alt) {
            Ok(o) => o,
            Err(_) => return String::new(),
        };
        
        let neu_json = match serde_json::to_string_pretty(&neu) {
            Ok(o) => o,
            Err(_) => return String::new(),
        };
        
        let diff = prettydiff::diff_lines(&alt_json, &neu_json);
        let diff = diff.diff();
        let mut out = format!("<div>");
        let mut lines = Vec::new();
        
        for c in diff {
            let _ = match c {
                DiffOp::Insert(i) => {
                    for i in i.iter() {
                        lines.push(format!("<span class='insert'><p>+</p><p>{}</p></span>", i.replace(" ", "&nbsp;")));
                    }
                },
                DiffOp::Replace(old, new) => {
                    for old in old.iter() {
                        lines.push(format!("<span class='remove'><p>-</p><p>{}</p></span>", old.replace(" ", "&nbsp;")));
                    }
                    for new in new.iter() {
                        lines.push(format!("<span class='insert'><p>+</p><p>{}</p></span>", new.replace(" ", "&nbsp;")));
                    }
                },
                DiffOp::Remove(r) => {
                    for r in r.iter() {
                        lines.push(format!("<span class='remove'><p>-</p><p>{}</p></span>", r.replace(" ", "&nbsp;")))
                    }
                },
                DiffOp::Equal(e) => {
                    for e in e.iter() {
                        lines.push(format!("<span class='equal'><p>&nbsp;</p><p>{}</p></span>", e.replace(" ", "&nbsp;")))
                    }
                },
            };
        }
        
        let lines_equal = lines.iter().enumerate().filter_map(|(num, line)| {
            if line.contains("<span class='equal'>") { Some(num) } else { None }
        }).collect::<Vec<_>>();
        
        let mut ranges = vec![(0, 0)];
        let mut last_l = 0;
        
        for l in lines_equal.iter() {
            
            if l - last_l > 1 {
                // start new range 
                ranges.push((*l, *l));
            }
            
            last_l = *l;
            
            if let Some((start, end)) = ranges.last_mut() {
                *end = *l; 
            }
        }
                
        for (start, end) in ranges {
            if end.saturating_sub(start) >= 5 {
                for i in (start + 1)..(end - 1) {
                    lines[i].clear();
                }
                lines[start + 2] = format!("<span class='snip'><p>&nbsp;</p><p>----</p></span>");
            }
        }
        
        out.push_str(&lines.join("\r\n"));
        out.push_str("</div>");
        out
    } else {
        String::new()
    }
}

pub fn render_popover_content(rpc_data: &RpcData) -> String {

    const ICON_CLOSE: &[u8] = include_bytes!("./img/icons8-close-96.png");

    let application_popover_color = if !rpc_data.is_context_menu_open() {
        "rgba(0, 0, 0, 0.5)"
    } else {
        "transparent"
    };
    
    let icon_close_base64 = base64::encode(ICON_CLOSE);
    
    let close_button = format!("
    <div style='position:absolute;top:50px;z-index:9999;right:-25px;background:white;border-radius:10px;box-shadow: 0px 0px 10px #cccccc88;cursor:pointer;' onmouseup='closePopOver()'>
        <img src='data:image/png;base64,{icon_close_base64}' style='width:50px;height:50px;cursor:pointer;' />
    </div>");
    
    let pc = match rpc_data.popover_state {
        None => return String::new(),
        Some(PopoverState::GrundbuchUploadDialog(i)) => {
            
            let upload = rpc_data.get_aenderungen();
            let dateien = render_aenderungen_dateien(&upload, i);
            let diff = render_aenderung_diff(&upload, i);
            
            let commit_title = if rpc_data.commit_title.is_empty() { 
                String::new() 
            } else { 
                format!("value='{}'", rpc_data.commit_title) 
            };
            
            let commit_description = if rpc_data.commit_msg.is_empty() { 
                String::new() 
            } else { 
                rpc_data.commit_msg.lines().map(|l| format!("<p>{l}</p>")).collect::<Vec<_>>().join("\r\n")
            };
            
            format!("
            <div style='box-shadow:0px 0px 100px #22222288;pointer-events:initial;width:1200px;display:flex;flex-direction:column;position:relative;margin:10px auto;border:1px solid grey;background:white;padding:100px;border-radius:5px;' onmousedown='event.stopPropagation();' onmouseup='event.stopPropagation();'>
                
                {close_button}

                <h2 style='font-size:24px;font-family:sans-serif;margin-bottom:25px;'>Änderungen in Datenbank hochladen</h2>
                
                <div style='padding:5px 0px;display:flex;flex-grow:1;flex-direction:column;'>
                    <form onsubmit='grundbuchHochladen(event)' action=''>
                    
                    <div style='display:flex;font-size:16px;flex-direction:column;'>
                        <p style='font-size:16px;line-height:2;'>Beschreiben Sie ihre Änderungen:</p>
                        <input type='text' oninput='editCommitTitle(event);' id='__application_grundbuch_aenderung_commit_titel' required placeholder='z.B. \"Korrektur aufgrund von Kaufvertrag XXX/XXXX\"' style='font-size:18px;font-family:monospace;font-weight:bold;border:1px solid #ccc;cursor:text;display:flex;flex-grow:1;' {commit_title}></input>
                    </div>
                    
                    <div style='display:flex;font-size:16px;flex-direction:column;'>
                        <p style='font-size:16px;line-height:2;'>Ausführliche Beschreibung der Änderung:</p>
                        
                        <div style='display:flex;flex-grow:1;flex-direction:column;background:white;border:1px solid #efefef;margin-top:5px;font-weight:bold;font-size:14px;font-family:monospace;color:black;padding:0px;min-height:200px;max-height:250px;overflow-y:scroll;'>
                            <div style='padding-left:2px;caret-color: #4a4e6a;' contenteditable='true' onkeydown='insertTabAtCaret(event);' oninput='editCommitDescription(event);' id='__application_grundbuch_aenderung_commit_description'>{commit_description}</div>
                        </div>
                    </div>
                    
                    <div id='__application_grundbuch_upload_aenderungen' style='display:flex;flex-direction:row;min-height:300px;max-height:400px;flex-grow:1;overflow-y:scroll;'>
                        <div id='__application_aenderung_dateien' style='padding: 10px 0px;margin-right:10px;overflow-y: scroll;height: 300px;min-width: 300px;'>
                            {dateien}
                        </div>
                        <div id='__application_aenderungen_diff'>
                            {diff}
                        </div>
                    </div>
                    
                    <div style='display:flex;flex-direction:row;justify-content: flex-end;margin-top: 20px;'>
                        <input type='submit' value='Änderungen übernehmen' class='btn btn_neu' style='cursor:pointer;font-size:20px;height:unset;display:inline-block;flex-grow:0;max-width:320px;' />
                    </div>
                    </form>
                </div>
            </div>
            ")
        },
        Some(PopoverState::GrundbuchSuchenDialog) => {
            format!("
            <div style='box-shadow:0px 0px 100px #22222288;pointer-events:initial;width:800px;display:flex;flex-direction:column;position:relative;margin:10px auto;border:1px solid grey;background:white;padding:100px;border-radius:5px;' onmousedown='event.stopPropagation();' onmouseup='event.stopPropagation();'>
                
                {close_button}

                <h2 style='font-size:24px;font-family:sans-serif;'>Grundbuchblatt suchen</h2>
                
                <div style='padding:5px 0px;display:flex;flex-grow:1;flex-direction:column;'>
                    <form onsubmit='grundbuchSuchen(event)' action=''>
                    <div style='display:flex;justify-content:space-between;padding:10px 0px;font-size:16px;flex-direction:row;margin-bottom:20px;'>
                        <input type='text' id='__application_grundbuch_suchen_suchbegriff' required placeholder='Suchbegriff (z.B. \"Ludwigsburg Blatt 10\" oder \"Max Mustermann\")' style='font-size:14px;font-weight:bold;border-bottom:1px solid black;cursor:text;display:flex;flex-grow:1;'></input>
                        <input type='submit' value='Suchen' class='btn btn_neu' style='cursor:pointer;font-size:20px;height:unset;display:flex;flex-grow:0;margin-left:20px;' />
                        </div>
                    </form>
                    
                    <div id='__application_grundbuch_suchen_suchergebnisse' style='display:flex;flex-grow:1;min-height:500px;flex-direction:column;max-height:700px;overflow-y:scroll;'>
                    </div>
                </div>
            </div>
            ")
        },
        Some(PopoverState::CreateNewGrundbuch) => {
            format!("
            <div style='box-shadow:0px 0px 100px #22222288;pointer-events:initial;width:800px;display:flex;flex-direction:column;position:relative;margin:10px auto;border:1px solid grey;background:white;padding:100px;border-radius:5px;' onmousedown='event.stopPropagation();' onmouseup='event.stopPropagation();'>
                
                {close_button}

                <h2 style='font-size:24px;font-family:sans-serif;margin-bottom:25px;'>Neues Grundbuchblatt anlegen</h2>
                
                <div style='padding:5px 0px;display:flex;flex-grow:1;flex-direction:column;'>
                    <form onsubmit='grundbuchAnlegen(event)' action=''>
                    <div style='display:flex;justify-content:space-between;padding:10px 0px;font-size:16px;'>
                        <label style='font-size:20px;font-style:italic;'>Land</label>
                        <select id='__application_grundbuch_anlegen_land' style='font-size:20px;font-weight:bold;border-bottom:1px solid black;cursor:pointer;'>
                            <option value='Baden-Württemberg'>Baden-Württemberg</option>
                            <option value='Bayern'>Bayern</option>
                            <option value='Berlin'>Berlin</option>
                            <option value='Brandenburg' selected='selected'>Brandenburg</option>
                            <option value='Bremen'>Bremen</option>
                            <option value='Hamburg'>Hamburg</option>
                            <option value='Hessen'>Hessen</option>
                            <option value='Mecklenburg-Vorpommern'>Mecklenburg-Vorpommern</option>
                            <option value='Niedersachsen'>Niedersachsen</option>
                            <option value='Nordrhein-Westfalen'>Nordrhein-Westfalen</option>
                            <option value='Rheinland-Pfalz'>Rheinland-Pfalz</option>
                            <option value='Saarland'>Saarland</option>
                            <option value='Sachsen'>Sachsen</option>
                            <option value='Sachsen-Anhalt'>Sachsen-Anhalt</option>
                            <option value='Schleswig-Holstein'>Schleswig-Holstein</option>
                            <option value='Thüringen'>Thüringen</option>
                        </select>
                    </div>
                    <div style='display:flex;justify-content:space-between;padding:10px 0px;font-size:16px;'>
                        <label style='font-size:20px;font-style:italic;'>Amtsgericht</label>
                        <input type='text' id='__application_grundbuch_anlegen_amtsgericht' required style='font-size:20px;font-weight:bold;border-bottom:1px solid black;cursor:text;'></input>
                    </div>
                    <div style='display:flex;justify-content:space-between;padding:10px 0px;font-size:16px;'>
                        <label style='font-size:20px;font-style:italic;'>Grundbuch von</label>
                        <input type='text' id='__application_grundbuch_anlegen_grundbuch_von' required style='font-size:20px;font-weight:bold;border-bottom:1px solid black;cursor:text;'></input>
                    </div>
                    <div style='display:flex;justify-content:space-between;padding:10px 0px;font-size:16px;'>
                        <label style='font-size:20px;font-style:italic;'>Blatt-Nr.</label>
                        <input type='number' id='__application_grundbuch_anlegen_blatt_nr' required style='font-size:20px;font-weight:bold;border-bottom:1px solid black;cursor:text;'></input>
                    </div>
                    <br/>
                    <input type='submit' value='Speichern' class='btn btn_neu' style='cursor:pointer;font-size:20px;height:unset;display:inline-block;flex-grow:0;max-width:320px;margin-top:20px;' />
                    </form>
                </div>
            </div>
            ")
        },
        Some(PopoverState::ExportPdf) => {
            format!("
            <div style='box-shadow:0px 0px 100px #22222288;pointer-events:initial;width:800px;display:flex;flex-direction:column;position:relative;margin:10px auto;border:1px solid grey;background:white;padding:100px;border-radius:5px;' onmousedown='event.stopPropagation();' onmouseup='event.stopPropagation();'>
                
                {close_button}

                <h2 style='font-size:24px;font-family:sans-serif;margin-bottom:25px;'>PDF-Export</h2>
                
                <div style='padding:5px 0px;display:flex;flex-grow:1;flex-direction:column;'>
                    <form onsubmit='grundbuchExportieren(event)'  action=''>
                    
                    <div style='display:flex;justify-content:space-between;padding:10px 0px;font-size:16px;'>
                        <label style='font-size:20px;font-style:italic;'>Exportiere:</label>
                        
                        <select id='__application_export-pdf-was-exportieren' style='font-size:20px;font-weight:bold;border-bottom:1px solid black;cursor:pointer;'>
                            <option value='offen'>Offenes Grundbuch</option>
                            <option value='alle-offen-digitalisiert'>Alle offenen, digitalisierten Grundbücher</option>
                            <option value='alle-offen'>Alle offenen Grundbücher</option>
                            <option value='alle-original'>Alle Original-PDFs</option>
                        </select>
                    </div>

                    <div style='display:flex;justify-content:space-between;padding:10px 0px;font-size:16px;'>
                        <label style='font-size:20px;font-style:italic;'>Exportiere Abteilungen:</label>
                    </div>
                    <div style='display:flex;justify-content:space-between;padding:10px 0px;font-size:16px;'>
                        <label for='export-pdf-bv' style='font-size:16px;margin-left:10px;'>Bestandsverzeichnis</label>
                        <input id='export-pdf-bv' type='checkbox' style='width:20px;height:20px;cursor:pointer;' checked='checked'/>
                    </div>
                    <div style='display:flex;justify-content:space-between;padding:10px 0px;font-size:16px;'>
                        <label for='export-pdf-abt-1' style='font-size:16px;margin-left:10px;'>Abteilung 1</label>
                        <input id='export-pdf-abt-1' type='checkbox' style='width:20px;height:20px;cursor:pointer;' checked='checked'/>
                    </div>
                    <div style='display:flex;justify-content:space-between;padding:10px 0px;font-size:16px;'>
                        <label for='export-pdf-abt-2' style='font-size:16px;margin-left:10px;'>Abteilung 2</label>
                        <input id='export-pdf-abt-2' type='checkbox' style='width:20px;height:20px;cursor:pointer;' checked='checked'/>
                    </div>
                    <div style='display:flex;justify-content:space-between;padding:10px 0px;font-size:16px;'>
                        <label for='export-pdf-abt-3' style='font-size:16px;margin-left:10px;'>Abteilung 3</label>
                        <input id='export-pdf-abt-3' type='checkbox' style='width:20px;height:20px;cursor:pointer;' checked='checked'/>
                    </div>
                    <br/>
                    <div style='display:flex;justify-content:space-between;padding:10px 0px;font-size:16px;'>
                        <input id='export-pdf-leere-seite' type='checkbox' style='width:20px;height:20px;cursor:pointer;' checked='checked'/>                        
                        <label for='export-pdf-leere-seite' style='font-size:20px;font-style:italic;'>Leere Seite nach Titelblatt einfügen</label>
                    </div>
                    <div style='display:flex;justify-content:space-between;padding:10px 0px;font-size:16px;'>
                        <input id='export-pdf-geroetete-eintraege' type='checkbox' style='width:20px;height:20px;cursor:pointer;' checked='checked'/>                        
                        <label for='export-pdf-geroetete-eintraege' style='font-size:20px;font-style:italic;'>Gerötete Einträge ausgeben</label>
                    </div>
                    <div style='display:flex;justify-content:space-between;padding:10px 0px;font-size:16px;'>
                        <input id='export-pdf-eine-datei' type='checkbox' style='width:20px;height:20px;cursor:pointer;' checked='checked'/>                        
                        <label for='export-pdf-eine-datei' style='font-size:20px;font-style:italic;'>Als ein PDF ausgeben</label>
                    </div>
                    <input type='submit' value='Speichern' class='btn btn_neu' style='cursor:pointer;font-size:20px;height:unset;display:inline-block;flex-grow:0;max-width:320px;margin-top:20px;' />
                        
                    </form>
                </div>
            </div>
            ")
        },
        Some(PopoverState::Info) => {
            format!("
            <div style='box-shadow:0px 0px 100px #22222288;pointer-events:initial;width:800px;display:flex;flex-direction:column;position:relative;margin:10px auto;border:1px solid grey;background:white;padding:100px;border-radius:5px;' onmousedown='event.stopPropagation();' onmouseup='event.stopPropagation();'>
                
                {close_button}

                <h2 style='font-size:24px;font-family:sans-serif;'>Digitales Grundbuch Version {version}</h2>
                
                <div style='padding:5px 0px;display:flex;flex-grow:1;min-height:750px;'>
                    <iframe width='auto' height='auto' src='data:text/html;base64,{license_base64}' style='min-width:100%;min-height:100%;'></iframe>                       
                </div>
                                
            </div>
            ",version = env!("CARGO_PKG_VERSION"),
            license_base64 = base64::encode(include_bytes!("../licenses.html")))
        },
        Some(PopoverState::Help) => {            
            
            static DOKU: &str = include_str!("../doc/Handbuch.html");
            
            static IMG_1: &[u8] = include_bytes!("../doc/IMG_1.png");
            static IMG_2: &[u8] = include_bytes!("../doc/IMG_2.png");
            static IMG_3: &[u8] = include_bytes!("../doc/IMG_3.png");
            static IMG_4: &[u8] = include_bytes!("../doc/IMG_4.png");
            static IMG_5: &[u8] = include_bytes!("../doc/IMG_5.png");
            static IMG_6: &[u8] = include_bytes!("../doc/IMG_6.png");
            static IMG_7: &[u8] = include_bytes!("../doc/IMG_7.png");
            static IMG_8: &[u8] = include_bytes!("../doc/IMG_8.png");

            let base64_dok = base64::encode(DOKU
                .replace("$$DATA_IMG_1$$", &base64::encode(IMG_1))
                .replace("$$DATA_IMG_2$$", &base64::encode(IMG_2))
                .replace("$$DATA_IMG_3$$", &base64::encode(IMG_3))
                .replace("$$DATA_IMG_4$$", &base64::encode(IMG_4))
                .replace("$$DATA_IMG_5$$", &base64::encode(IMG_5))
                .replace("$$DATA_IMG_6$$", &base64::encode(IMG_6))
                .replace("$$DATA_IMG_7$$", &base64::encode(IMG_7))
                .replace("$$DATA_IMG_8$$", &base64::encode(IMG_8))
            );
            
            format!("
            <div style='box-shadow:0px 0px 100px #22222288;pointer-events:initial;width:800px;display:flex;flex-direction:column;position:relative;margin:10px auto;border:1px solid grey;background:white;padding:100px;border-radius:5px;' onmousedown='event.stopPropagation();' onmouseup='event.stopPropagation();'>

                {close_button}
                
                <h2 style='font-size:24px;font-family:sans-serif;margin-bottom:25px;'>Benutzerhandbuch</h2>
                <div style='padding:5px 0px;display:flex;flex-grow:1;line-height:1.5;min-height:750px;'>
                    <iframe src='data:text/html;base64,{base64_dok}' width='100%' height='100%' style='min-width:100%;min-height:750px;display:flex;flex-grow:1;'/>
                </div>

            </div>")
        },
        Some(PopoverState::Configuration(cw)) => {
        
            use crate::ConfigurationView::*;
            
            static IMG_SETTINGS: &[u8] = include_bytes!("./img/icons8-settings-system-daydream-96.png");
            let img_settings = base64::encode(IMG_SETTINGS);
            
            static IMG_REGEX: &[u8] = include_bytes!("./img/icons8-select-96.png");
            let img_regex = base64::encode(IMG_REGEX);
            
            static IMG_CLEAN: &[u8] = include_bytes!("./img/icons8-broom-96.png");
            let img_clean = base64::encode(IMG_CLEAN);
            
            static IMG_ABK: &[u8] = include_bytes!("./img/icons8-shortcut-96.png");
            let img_abk = base64::encode(IMG_ABK);
                        
            static IMG_FX: &[u8] = include_bytes!("./img/icons8-formula-fx-96.png");
            let img_fx = base64::encode(IMG_FX);
            
            let active_allgemein = if cw == Allgemein { " active" } else { "" };
            let active_regex = if cw == RegEx { " active" } else { "" };
            let active_text_saubern = if cw == TextSaubern { " active" } else { "" };
            let active_abkuerzungen = if cw == Abkuerzungen { " active" } else { "" };
            let active_flst_auslesen = if cw == FlstAuslesen { " active" } else { "" };
            let active_klassifizierung_rechteart = if cw == KlassifizierungRechteArt { " active" } else { "" };
            let active_rechtsinhaber_auslesen_abt2 = if cw == RechtsinhaberAuslesenAbt2 { " active" } else { "" };
            let active_rangvermerk_auslesen_abt2 = if cw == RangvermerkAuslesenAbt2 { " active" } else { "" };
            let active_text_kuerzen_abt2 = if cw == TextKuerzenAbt2 { " active" } else { "" };
            let active_betrag_auslesen_abt3 = if cw == BetragAuslesenAbt3 { " active" } else { "" };
            let active_klassifizierung_schuldenart_abt3 = if cw == KlassifizierungSchuldenArtAbt3 { " active" } else { "" };
            let active_rechtsinhaber_auslesen_abt3 = if cw == RechtsinhaberAuslesenAbt3 { " active" } else { "" };
            let active_text_kuerzen_abt3 = if cw == TextKuerzenAbt3 { " active" } else { "" };

            let sidebar = format!("
                <div class='__application_configuration_sidebar' style='display:flex;flex-direction:column;width:160px;min-height:750px;'>
                    
                    <div class='__application_configuration_sidebar_section{active_allgemein}' onmouseup='activateConfigurationView(event, \"allgemein\")'>
                        <img style='width:25px;height:25px;' src='data:image/png;base64,{img_settings}'></img>
                        <p>Allgemein</p>
                    </div>
                    
                    <hr/>
                    
                    <div class='__application_configuration_sidebar_section{active_regex}' onmouseup='activateConfigurationView(event, \"regex\")'>
                        <img style='width:25px;height:25px;' src='data:image/png;base64,{img_regex}'></img>
                        <p>Reguläre Ausdrücke</p>
                    </div>
                    
                    <div class='__application_configuration_sidebar_section{active_text_saubern}' onmouseup='activateConfigurationView(event, \"text-saubern\")'>
                        <img style='width:25px;height:25px;' src='data:image/png;base64,{img_clean}'></img>
                        <p>Text säubern</p>
                    </div>
                    
                    <div class='__application_configuration_sidebar_section{active_abkuerzungen}' onmouseup='activateConfigurationView(event, \"abkuerzungen\")'>
                        <img style='width:25px;height:25px;' src='data:image/png;base64,{img_abk}'></img>
                        <p>Abkürzungen</p>
                    </div>
                    
                    <div class='__application_configuration_sidebar_section{active_flst_auslesen}' onmouseup='activateConfigurationView(event, \"flst-auslesen\")'>
                        <img style='width:25px;height:25px;' src='data:image/png;base64,{img_fx}'></img>
                        <p>Flurstücke auslesen</p>
                    </div>
                    
                    <hr/>

                    <div class='__application_configuration_sidebar_section{active_klassifizierung_rechteart}' onmouseup='activateConfigurationView(event, \"klassifizierung-rechteart-abt2\")'>
                        <img style='width:25px;height:25px;' src='data:image/png;base64,{img_fx}'></img>
                        <p>Klassifizierung RechteArt (Abt. 2)</p>
                    </div>
                    
                    <div class='__application_configuration_sidebar_section{active_rechtsinhaber_auslesen_abt2}' onmouseup='activateConfigurationView(event, \"rechtsinhaber-auslesen-abt2\")'>
                        <img style='width:25px;height:25px;' src='data:image/png;base64,{img_fx}'></img>
                        <p>Rechtsinhaber auslesen (Abt. 2)</p>
                    </div>
                    
                    <div class='__application_configuration_sidebar_section{active_rangvermerk_auslesen_abt2}' onmouseup='activateConfigurationView(event, \"rangvermerk-auslesen-abt2\")'>
                        <img style='width:25px;height:25px;' src='data:image/png;base64,{img_fx}'></img>
                        <p>Rangvermerk auslesen (Abt. 2)</p>
                    </div>
                    
                    <div class='__application_configuration_sidebar_section{active_text_kuerzen_abt2}' onmouseup='activateConfigurationView(event, \"text-kuerzen-abt2\")'>
                        <img style='width:25px;height:25px;' src='data:image/png;base64,{img_fx}'></img>
                        <p>Text kürzen (Abt. 2)</p>
                    </div>
                    
                    <hr/>

                    <div class='__application_configuration_sidebar_section{active_betrag_auslesen_abt3}' onmouseup='activateConfigurationView(event, \"betrag-auslesen-abt3\")'>
                        <img style='width:25px;height:25px;' src='data:image/png;base64,{img_fx}'></img>
                        <p>Betrag auslesen (Abt. 3)</p>
                    </div>
                    <div class='__application_configuration_sidebar_section{active_klassifizierung_schuldenart_abt3}' onmouseup='activateConfigurationView(event, \"klassifizierung-schuldenart-abt3\")'>
                        <img style='width:25px;height:25px;' src='data:image/png;base64,{img_fx}'></img>
                        <p>Klassifizierung SchuldenArt (Abt. 3)</p>
                    </div>
                    <div class='__application_configuration_sidebar_section{active_rechtsinhaber_auslesen_abt3}' onmouseup='activateConfigurationView(event, \"rechtsinhaber-auslesen-abt3\")'>
                        <img style='width:25px;height:25px;' src='data:image/png;base64,{img_fx}'></img>
                        <p>Rechtsinhaber auslesen (Abt. 3)</p>
                    </div>
                    <div class='__application_configuration_sidebar_section{active_text_kuerzen_abt3}' onmouseup='activateConfigurationView(event, \"text-kuerzen-abt3\")'>
                        <img style='width:25px;height:25px;' src='data:image/png;base64,{img_fx}'></img>
                        <p>Text kürzen (Abt. 3)</p>
                    </div>
                </div>
            ");
            
            let main_content = match cw {
                Allgemein => format!("
                    <div style='padding:5px 0px;display:flex;flex-direction:column;flex-grow:1;'>
                        <div>
                            <div style='display:flex;flex-direction:row;'>
                                <input style='width:20px;height:20px;cursor:pointer;' type='checkbox' id='__application_konfiguration_spalten_ausblenden' {spalten_einblenden} data-checkBoxId='konfiguration-spalten-ausblenden' onchange='toggleCheckbox(event)'>
                                <label style='font-size:20px;font-style:italic;' for='__application_konfiguration_spalten_ausblenden'>Formularspalten einblenden</label>
                            </div>
                            
                            <div style='display:flex;flex-direction:row;'>
                                <input style='width:20px;height:20px;cursor:pointer;' type='checkbox' id='__application_konfiguration_zeilenumbrueche-in-ocr-text' data-checkBoxId='konfiguration-zeilenumbrueche-in-ocr-text' {zeilenumbrueche_in_ocr_text} onchange='toggleCheckbox(event)'>
                                <label style='font-size:20px;font-style:italic;' for='__application_konfiguration_zeilenumbrueche-in-ocr-text'>Beim Kopieren von OCR-Text Zeilenumbrüche beibehalten</label>
                            </div>
                            
                            <div style='display:flex;flex-direction:row;'>
                                <input style='width:20px;height:20px;cursor:pointer;' type='checkbox' id='__application_konfiguration_hide_red_lines' data-checkBoxId='konfiguration-keine-roten-linien' {vorschau_ohne_geroetet} onchange='toggleCheckbox(event)'>
                                <label style='font-size:20px;font-style:italic;' for='__application_konfiguration_hide_red_lines'>PDF ohne geröteten Linien darstellen</label>
                            </div>
                        </div>
                        
                        <div style='margin-top:25px;'>
                            <h2 style='font-size:20px;'>Datenbank</h2>
                            
                            <div style='display:flex;justify-content:space-between;padding:10px 0px;font-size:16px;'>
                                <label style='font-size:20px;font-style:italic;'>Server-URL</label>
                                <input type='text' id='__application_konfiguration_datenbank_server' style='font-size:20px;font-weight:bold;border-bottom:1px solid black;cursor:text;min-width:300px;' value='{server_url}' data-konfiguration-textfield='server-url' onchange='editKonfigurationTextField(event)'></input>
                            </div>
                    
                            <div style='display:flex;justify-content:space-between;padding:10px 0px;font-size:16px;'>
                                <label style='font-size:20px;font-style:italic;'>Benutzername</label>
                                <input type='text' id='__application_konfiguration_datenbank_benutzername' style='font-size:20px;font-weight:bold;border-bottom:1px solid black;cursor:text;min-width:300px;' value='{server_benutzername}' data-konfiguration-textfield='benutzername' onchange='editKonfigurationTextField(event)'></input>
                            </div>
                            
                            <div style='display:flex;justify-content:space-between;padding:10px 0px;font-size:16px;'>
                                <label style='font-size:20px;font-style:italic;'>E-Mail</label>
                                <input type='text' id='__application_konfiguration_datenbank_email' style='font-size:20px;font-weight:bold;border-bottom:1px solid black;cursor:text;min-width:300px;' value='{server_email}' data-konfiguration-textfield='email' onchange='editKonfigurationTextField(event)'></input>
                            </div>
                            
                            <div style='display:flex;flex-direction:row;justify-content:space-between;padding:10px 0px;font-size:16px;'>
                                <label style='font-size:20px;font-style:italic;'>Zertifikatsdatei</label>
                                <div style='width:200px;'><p>{cert_sig}</p></div>
                                <input type='file' class='btn btn_neu' id='__application_konfiguration_datenbank_private_key' onchange='editKonfigurationSchluesseldatei(event)' accept='.pfx'></input>
                                <input type='button' value='Datei auswählen...' class='btn btn_neu' data-file-input-id='__application_konfiguration_datenbank_private_key' onclick='document.getElementById(event.target.dataset.fileInputId).click();' />
                            </div>´
                        </div>
                    </div>
                ",
                    server_url = rpc_data.konfiguration.server_url,
                    server_benutzername = rpc_data.konfiguration.server_benutzer,
                    server_email = rpc_data.konfiguration.server_email,
                    cert_sig = rpc_data.konfiguration.get_cert().map(|cert| cert.fingerprint().to_spaced_hex()).unwrap_or_default(),
                    vorschau_ohne_geroetet = if rpc_data.konfiguration.vorschau_ohne_geroetet { "checked" } else { "" },
                    spalten_einblenden = if !rpc_data.konfiguration.spalten_ausblenden { "checked" } else { "" },
                    zeilenumbrueche_in_ocr_text = if rpc_data.konfiguration.zeilenumbrueche_in_ocr_text { "checked" } else { "" },
                ),
                RegEx => format!("
                    <div style='padding:5px 0px;display:flex;flex-direction:column;flex-grow:1;'>
                        
                        <div style='display:block;'>
                            <button class='btn-grad' data-regex-id='{next_regex_id}' onclick='insertRegexFromButton(event)'>Neuen regulären Ausdruck anlegen</button>
                        </div>
                        
                        <div style='display:block;max-height:450px;flex-grow:1;background:white;border:1px solid #efefef;margin-top:5px;font-weight:bold;font-size:14px;font-family:monospace;color:black;padding:10px;overflow-y:scroll;width:700px;'>
                        {regex}
                        </div>

                        <div style='display:flex;flex-direction:column;margin-top:10px;'>
                            <input id='__application_konfiguration_regex_id' style='margin-bottom:5px;border-radius:5px;padding:5px;border:1px solid #efefef;flex-grow:1;margin-right:10px;' placeholder='Regex ID'></input>
                            <textarea id='__application_konfiguration_regex_test_text' style='margin-bottom:5px;border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' oninput='testeRegex(event);'style='flex-grow:1;margin-right:10px;' placeholder='Test Eingabe...'></textarea>
                            <textarea style='border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' id='__application_konfiguration_regex_test_output' style='flex-grow:1;' placeholder='Regex Ausgabe'></textarea>
                        </div>
                    </div>
                ",
                next_regex_id = format!("A_{}", 9999999_usize.wrapping_sub(rpc_data.konfiguration.regex.len())),
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
                        <div style='display:flex;flex-direction:row;'>
                            <div id='__application_konfiguration_regex_key_{idx}' style='display:inline;width:220px;caret-color: #4a4e6a;' contenteditable='true' data-regex-key='{k}' oninput='editRegexKey(event);' onkeydown='neueRegexOnEnter(event);' data-key-id='__application_konfiguration_regex_key_{idx}'>{k}</div>
                            
                            <p style='display:inline;color:#4a4e6a;user-select:none;'>&nbsp;=&nbsp;</p>
                            
                            <div id='__application_konfiguration_regex_value_{idx}' data-key-id='__application_konfiguration_regex_key_{idx}' style='display:inline;caret-color: #4a4e6a;width:400px;' onkeydown='neueRegexOnEnter(event);' contenteditable='true' oninput='editRegexValue(event);'>{v}</div>
                            
                            <div style='display:inline-flex;flex-grow:1;'></div>
                            
                            <img style='width:16px;height:16px;cursor:pointer;' data-key-id='__application_konfiguration_regex_key_{idx}' onclick='regexLoeschen(event);' src='data:image/png;base64,{icon_close_base64}'>
                        </div>
                    ", k = k, v = v.replace("\\", "&bsol;"), idx = idx, icon_close_base64 = icon_close_base64))
                    .collect::<Vec<_>>()
                    .join("\r\n")
                }),
                TextSaubern => format!("
                    <div style='padding:5px 0px;display:flex;flex-direction:column;flex-grow:1;'>
                        <div style='display:flex;flex-grow:1;flex-direction:column;background:white;border:1px solid #efefef;margin-top:5px;font-weight:bold;font-size:14px;font-family:monospace;color:black;padding:10px;min-height:200px;max-height:650px;overflow-y:scroll;'>
                            <p style='color:#4a4e6a;user-select:none;'>def text_säubern(recht: String) -> String:</p>
                            <div style='padding-left:34px;caret-color: #4a4e6a;' contenteditable='true' onkeydown='insertTabAtCaret(event);' oninput='editTextSaubernScript(event);'>{konfig_text_saubern_script}</div>
                        </div>
                    </div>              
                ", konfig_text_saubern_script = 
                    rpc_data.konfiguration.text_saubern_script.iter()
                    .map(|l| l.replace(" ", "\u{00a0}"))
                    .map(|l| l.replace("\\", "&bsol;"))
                    .map(|l| if l.is_empty() { format!("<div>&nbsp;</div>") } else { format!("<div>{}</div>", l) })
                    .collect::<Vec<String>>()
                    .join("\r\n"),
                ),
                Abkuerzungen => format!("
                    <div style='padding:5px 0px;display:flex;flex-direction:column;flex-grow:1;'>
                        <div style='display:flex;flex-grow:1;flex-direction:column;background:white;border:1px solid #efefef;margin-top:5px;font-weight:bold;font-size:14px;font-family:monospace;color:black;padding:10px;min-height:200px;max-height:650px;overflow-y:scroll;'>
                            <p style='color:#4a4e6a;user-select:none;'>def abkuerzungen() -> [String]:</p>
                            <div style='padding-left:34px;caret-color: #4a4e6a;' contenteditable='true' onkeydown='insertTabAtCaret(event);' oninput='editAbkuerzungenScript(event);'>{konfig_abkuerzungen_script}</div>
                        </div>
                    </div>
                ", konfig_abkuerzungen_script = 
                        rpc_data.konfiguration.abkuerzungen_script.iter()
                        .map(|l| l.replace(" ", "\u{00a0}"))
                        .map(|l| l.replace("\\", "&bsol;"))
                        .map(|l| if l.is_empty() { format!("<div>&nbsp;</div>") } else { format!("<div>{}</div>", l) })
                        .collect::<Vec<String>>()
                    .join("\r\n"),
                ),
                FlstAuslesen => format!("
                    
                    <div style='padding:5px 0px;display:flex;flex-direction:column;flex-grow:1;'>
                    
                        <div style='display:flex;flex-grow:1;flex-direction:column;background:white;border:1px solid #efefef;margin-top:5px;font-weight:bold;font-size:14px;font-family:monospace;color:black;padding:10px;min-height:200px;max-height:450px;overflow-y:scroll;'>
                            <p style='color:#4a4e6a;user-select:none;'>def flurstuecke_auslesen(spalte_1: String, text: String, re: Mapping[String, Regex]) -> [Spalte1Eintrag]:</p>
                            <div style='padding-left:34px;caret-color: #4a4e6a;' contenteditable='true' onkeydown='insertTabAtCaret(event);' oninput='editFlurstueckeAuslesenScript(event);'>{konfig_flurstuecke_auslesen_script}</div>
                        </div>
                        
                        <div style='display:flex;flex-direction:column;margin-top:10px;'>
                            <input type='text' style='margin-bottom:5px;border-radius:5px;padding:5px;border:1px solid #efefef;flex-grow:1;margin-right:10px;' id='__application_konfiguration_flurstueck_auslesen_bv_nr' placeholder='BV-Nr. (Spalte 1) eingeben...' />
                            <textarea style='margin-bottom:5px;border-radius:5px;padding:5px;border:1px solid #efefef;'  rows='5' cols='45' oninput='flurstueckAuslesenScriptTesten(event);'style='flex-grow:1;margin-right:10px;' placeholder='Test Eingabe...'></textarea>
                            <textarea style='border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' id='__application_konfiguration_flurstueck_auslesen_test' style='flex-grow:1;' placeholder='Ausgabe der Funktion'></textarea>
                        </div>
                    </div>     
                ", konfig_flurstuecke_auslesen_script = 
                    rpc_data.konfiguration.flurstuecke_auslesen_script.iter()
                    .map(|l| l.replace(" ", "\u{00a0}"))
                    .map(|l| l.replace("\\", "&bsol;"))
                    .map(|l| if l.is_empty() { format!("<div>&nbsp;</div>") } else { format!("<div>{}</div>", l) })
                    .collect::<Vec<String>>()
                    .join("\r\n")
                ),
                KlassifizierungRechteArt => format!("
                
                    <div style='padding:5px 0px;display:flex;flex-direction:column;flex-grow:1;'>
                    
                        <div>{rechteart_select}</div>

                        <div style='display:flex;flex-grow:1;flex-direction:column;background:white;border:1px solid #efefef;margin-top:5px;font-weight:bold;font-size:14px;font-family:monospace;color:black;padding:10px;min-height:200px;max-height:450px;overflow-y:scroll;'>
                            <p style='color:#4a4e6a;user-select:none;'>def klassifiziere_rechteart_abt2(saetze: [String], re: Mapping[String, Regex]) -> RechteArt:</p>
                            <div style='padding-left:34px;caret-color: #4a4e6a;'contenteditable='true' onkeydown='insertTabAtCaret(event);' oninput='editRechteArtScript(event);'>{konfig_rechteart_script}</div>
                        </div>
                        
                        <div style='display:flex;flex-direction:column;margin-top:10px;'>
                            <textarea style='margin-bottom:5px;border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' oninput='rechteArtScriptTesten(event);'style='flex-grow:1;margin-right:10px;' placeholder='Test Eingabe...'></textarea>
                            <textarea style='border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' id='__application_konfiguration_rechteart_test' style='flex-grow:1;' placeholder='Ausgabe der Funktion'></textarea>
                        </div>
                    </div>
                ", 
                    rechteart_select = render_rechteart_select(),
                    konfig_rechteart_script = 
                        rpc_data.konfiguration.klassifiziere_rechteart.iter()
                        .map(|l| l.replace(" ", "\u{00a0}"))
                        .map(|l| l.replace("\\", "&bsol;"))
                        .map(|l| if l.is_empty() { format!("<div>&nbsp;</div>") } else { format!("<div>{}</div>", l) })
                        .collect::<Vec<String>>()
                        .join("\r\n")
                ),
                RechtsinhaberAuslesenAbt2 => format!("
                
                    <div style='padding:5px 0px;display:flex;flex-direction:column;flex-grow:1;'>
                        
                        <div style='display:flex;flex-grow:1;flex-direction:column;background:white;border:1px solid #efefef;margin-top:5px;font-weight:bold;font-size:14px;font-family:monospace;color:black;padding:10px;min-height:200px;max-height:450px;overflow-y:scroll;'>
                            <p style='color:#4a4e6a;user-select:none;'>def rechtsinhaber_auslesen_abt2(saetze: [String], re: Mapping[String, Regex], recht_id: String) -> String:</p>
                            <div style='padding-left:34px;caret-color: #4a4e6a;' contenteditable='true' onkeydown='insertTabAtCaret(event);' oninput='editRechtsinhaberAbt2Script(event);'>{konfig_rechtsinhaber_abt2_script}</div>
                        </div>
                        
                        <div style='display:flex;flex-direction:column;margin-top:10px;'>
                            <textarea style='margin-bottom:5px;border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' oninput='rechtsinhaberAbt2ScriptTesten(event);'style='flex-grow:1;margin-right:10px;' placeholder='Test Eingabe...'></textarea>
                            <textarea style='border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' id='__application_konfiguration_rechtsinhaber_abt2_test' style='flex-grow:1;' placeholder='Ausgabe der Funktion'></textarea>
                        </div>
                    </div>
                ", konfig_rechtsinhaber_abt2_script = 
                    rpc_data.konfiguration.rechtsinhaber_auslesen_abt2_script.iter()
                    .map(|l| l.replace(" ", "\u{00a0}"))
                    .map(|l| l.replace("\\", "&bsol;"))
                    .map(|l| if l.is_empty() { format!("<div>&nbsp;</div>") } else { format!("<div>{}</div>", l) })
                    .collect::<Vec<String>>()
                    .join("\r\n"),
                ),
                RangvermerkAuslesenAbt2 => format!("
                                    
                    <div style='padding:5px 0px;display:flex;flex-direction:column;flex-grow:1;'>
        
                        
                        <div style='display:flex;flex-grow:1;flex-direction:column;background:white;border:1px solid #efefef;margin-top:5px;font-weight:bold;font-size:14px;font-family:monospace;color:black;padding:10px;min-height:200px;max-height:450px;overflow-y:scroll;'>
                            <p style='color:#4a4e6a;user-select:none;'>def rangvermerk_auslesen_abt2(saetze: [String], re: Mapping[String, Regex]) -> String:</p>
                            <div style='padding-left:34px;caret-color: #4a4e6a;' contenteditable='true' onkeydown='insertTabAtCaret(event);' oninput='editRangvermerkAuslesenAbt2Script(event);'>{konfig_rangvermerk_abt2_script}</div>
                        </div>
                        
                        <div style='display:flex;flex-direction:column;margin-top:10px;'>
                            <textarea style='margin-bottom:5px;border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' oninput='rangvermerkAuslesenAbt2ScriptTesten(event);'style='flex-grow:1;margin-right:10px;' placeholder='Test Eingabe...'></textarea>
                            <textarea style='border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' id='__application_konfiguration_rangvermerk_auslesen_abt2_test' style='flex-grow:1;' placeholder='Ausgabe der Funktion'></textarea>
                        </div>
                    </div>
                ", konfig_rangvermerk_abt2_script = 
                    rpc_data.konfiguration.rangvermerk_auslesen_abt2_script.iter()
                    .map(|l| l.replace(" ", "\u{00a0}"))
                    .map(|l| l.replace("\\", "&bsol;"))
                    .map(|l| if l.is_empty() { format!("<div>&nbsp;</div>") } else { format!("<div>{}</div>", l) })
                    .collect::<Vec<String>>()
                    .join("\r\n"),
                ),
                TextKuerzenAbt2 => format!("
                    
                    <div style='padding:5px 0px;display:flex;flex-direction:column;flex-grow:1;'>
                        
                        <div style='display:flex;flex-grow:1;flex-direction:column;background:white;border:1px solid #efefef;margin-top:5px;font-weight:bold;font-size:14px;font-family:monospace;color:black;padding:10px;min-height:200px;max-height:450px;overflow-y:scroll;'>
                            <p style='color:#4a4e6a;user-select:none;'>def text_kuerzen_abt2(saetze: [String], rechtsinhaber: String, rangvermerk: String, re: Mapping[String, Regex]) -> String:</p>
                            <div style='padding-left:34px;caret-color: #4a4e6a;'contenteditable='true' onkeydown='insertTabAtCaret(event);' oninput='editTextKuerzenAbt2Script(event);'>{konfig_text_kuerzen_abt2_script}</div>
                        </div>
                        
                        <div style='display:flex;flex-direction:column;margin-top:10px;'>
                            <textarea  style='margin-bottom:5px;border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' oninput='textKuerzenAbt2ScriptTesten(event);'style='flex-grow:1;margin-right:10px;' placeholder='Test Eingabe...'></textarea>
                            <textarea style='border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' id='__application_konfiguration_text_kuerzen_abt2_test' style='flex-grow:1;' placeholder='Ausgabe der Funktion'></textarea>
                        </div>
                    </div>
                ", konfig_text_kuerzen_abt2_script = 
                    rpc_data.konfiguration.text_kuerzen_abt2_script.iter()
                    .map(|l| l.replace(" ", "\u{00a0}"))
                    .map(|l| l.replace("\\", "&bsol;"))
                    .map(|l| if l.is_empty() { format!("<div>&nbsp;</div>") } else { format!("<div>{}</div>", l) })
                    .collect::<Vec<String>>()
                    .join("\r\n")
                ),
                BetragAuslesenAbt3 => format!("
                    <div style='padding:5px 0px;display:flex;flex-direction:column;flex-grow:1;'>
                        
                        <div style='display:flex;flex-grow:1;flex-direction:column;background:white;border:1px solid #efefef;margin-top:5px;font-weight:bold;font-size:14px;font-family:monospace;color:black;padding:10px;min-height:200px;max-height:450px;overflow-y:scroll;'>
                            <p style='color:#4a4e6a;user-select:none;'>def betrag_auslesen(saetze: [String], re: Mapping[String, Regex]) -> Betrag:</p>
                            <div style='padding-left:34px;caret-color: #4a4e6a;' contenteditable='true' onkeydown='insertTabAtCaret(event);' oninput='editBetragAuslesenScript(event);'>{konfig_betrag_script}</div>
                        </div>
                        
                        <div style='display:flex;flex-direction:column;margin-top:10px;'>
                            <textarea  style='margin-bottom:5px;border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' oninput='betragAuslesenScriptTesten(event);'style='flex-grow:1;margin-right:10px;' placeholder='Test Eingabe...'></textarea>
                            <textarea style='border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' id='__application_konfiguration_betrag_auslesen_test' style='flex-grow:1;' placeholder='Ausgabe der Funktion'></textarea>
                        </div>
                    </div>
                ",  konfig_betrag_script = 
                    rpc_data.konfiguration.betrag_auslesen_script.iter()
                    .map(|l| l.replace(" ", "\u{00a0}"))
                    .map(|l| l.replace("\\", "&bsol;"))
                    .map(|l| if l.is_empty() { format!("<div>&nbsp;</div>") } else { format!("<div>{}</div>", l) })
                    .collect::<Vec<String>>()
                    .join("\r\n"),                
                ),
                KlassifizierungSchuldenArtAbt3 => format!("               
                    <div style='padding:5px 0px;display:flex;flex-direction:column;flex-grow:1;'>
                    
                        <div>{schuldenart_select}</div>
                        
                        <div style='display:flex;flex-grow:1;flex-direction:column;background:white;border:1px solid #efefef;margin-top:5px;font-weight:bold;font-size:14px;font-family:monospace;color:black;padding:10px;min-height:200px;max-height:450px;overflow-y:scroll;'>
                            <p style='color:#4a4e6a;user-select:none;'>def klassifiziere_schuldenart_abt3(saetze: [String], re: Mapping[String, Regex]) -> SchuldenArt:</p>
                            <div style='padding-left:34px;caret-color: #4a4e6a;' contenteditable='true' onkeydown='insertTabAtCaret(event);' oninput='editSchuldenArtScript(event);'>{konfig_schuldenart_script}</div>
                        </div>
                        
                        <div style='display:flex;flex-direction:column;margin-top:10px;'>
                            <textarea style='margin-bottom:5px;border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' oninput='schuldenArtScriptTesten(event);'style='flex-grow:1;margin-right:10px;' placeholder='Test Eingabe...'></textarea>
                            <textarea style='border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' id='__application_konfiguration_schuldenart_test' style='flex-grow:1;' placeholder='Ausgabe der Funktion'></textarea>
                        </div>
                    </div>
                ", konfig_schuldenart_script = 
                    rpc_data.konfiguration.klassifiziere_schuldenart.iter()
                    .map(|l| l.replace(" ", "\u{00a0}"))
                    .map(|l| l.replace("\\", "&bsol;"))
                    .map(|l| if l.is_empty() { format!("<div>&nbsp;</div>") } else { format!("<div>{}</div>", l) })
                    .collect::<Vec<String>>()
                    .join("\r\n"),
                    schuldenart_select = render_schuldenart_select(),
                ),
                RechtsinhaberAuslesenAbt3 => format!("
                    <div style='padding:5px 0px;display:flex;flex-direction:column;flex-grow:1;'>
            
                        <div style='display:flex;flex-grow:1;flex-direction:column;background:white;border:1px solid #efefef;margin-top:5px;font-weight:bold;font-size:14px;font-family:monospace;color:black;padding:10px;min-height:200px;max-height:450px;overflow-y:scroll;'>
                            <p style='color:#4a4e6a;user-select:none;'>def rechtsinhaber_auslesen_abt3(saetze: [String], re: Mapping[String, Regex], recht_id: String) -> String:</p>
                            <div style='padding-left:34px;caret-color: #4a4e6a;' contenteditable='true' onkeydown='insertTabAtCaret(event);' oninput='editRechtsinhaberAbt3Script(event);'>{konfig_rechtsinhaber_abt3_script}</div>
                        </div>
                        
                        <div style='display:flex;flex-direction:column;margin-top:10px;'>
                            <textarea style='margin-bottom:5px;border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' oninput='rechtsinhaberAbt3ScriptTesten(event);'style='flex-grow:1;margin-right:10px;' placeholder='Test Eingabe...'></textarea>
                            <textarea style='border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' id='__application_konfiguration_rechtsinhaber_abt3_test' style='flex-grow:1;' placeholder='Ausgabe der Funktion'></textarea>
                        </div>
                    </div>
                ", konfig_rechtsinhaber_abt3_script = 
                    rpc_data.konfiguration.rechtsinhaber_auslesen_abt3_script.iter()
                    .map(|l| l.replace(" ", "\u{00a0}"))
                    .map(|l| l.replace("\\", "&bsol;"))
                    .map(|l| if l.is_empty() { format!("<div>&nbsp;</div>") } else { format!("<div>{}</div>", l) })
                    .collect::<Vec<String>>()
                    .join("\r\n"),                
                ),
                TextKuerzenAbt3 => format!("
                    <div style='padding:5px 0px;display:flex;flex-direction:column;flex-grow:1;'>
                        
                        <div style='display:flex;flex-grow:1;flex-direction:column;background:white;border:1px solid #efefef;margin-top:5px;font-weight:bold;font-size:14px;font-family:monospace;color:black;padding:10px;min-height:200px;max-height:450px;overflow-y:scroll;'>
                            <p style='color:#4a4e6a;user-select:none;'>def text_kuerzen_abt3(saetze: [String], betrag: String, schuldenart: String, rechtsinhaber: String, re: Mapping[String, Regex]) -> String:</p>
                            <div style='padding-left:34px;caret-color: #4a4e6a;' contenteditable='true' onkeydown='insertTabAtCaret(event);' oninput='editTextKuerzenAbt3Script(event);'>{konfig_text_kuerzen_abt3_script}</div>
                        </div>
                        
                        <div style='display:flex;flex-direction:column;margin-top:10px;'>
                            <textarea style='margin-bottom:5px;border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' oninput='textKuerzenAbt3ScriptTesten(event);'style='flex-grow:1;margin-right:10px;' placeholder='Test Eingabe...'></textarea>
                            <textarea style='border-radius:5px;padding:5px;border:1px solid #efefef;' rows='5' cols='45' id='__application_konfiguration_text_kuerzen_abt3_test' style='flex-grow:1;' placeholder='Ausgabe der Funktion'></textarea>
                        </div>
                    </div>
                ", konfig_text_kuerzen_abt3_script = 
                    rpc_data.konfiguration.text_kuerzen_abt3_script.iter()
                    .map(|l| l.replace(" ", "\u{00a0}"))
                    .map(|l| l.replace("\\", "&bsol;"))
                    .map(|l| if l.is_empty() { format!("<div>&nbsp;</div>") } else { format!("<div>{}</div>", l) })
                    .collect::<Vec<String>>()
                    .join("\r\n"),
                ),
            };
            
            let main = format!("<div style='display:flex;flex-grow:1;padding:0px 20px;line-height: 1.2;'>{main_content}</div>");
            
            format!("
                <div style='box-shadow:0px 0px 100px #22222288;pointer-events:initial;width:1000px;position:relative;display:flex;flex-direction:column;margin:10px auto;border:1px solid grey;background:white;padding:100px;border-radius:5px;' onmousedown='event.stopPropagation();' onmouseup='event.stopPropagation();'>
                
                    {close_button}
                    
                    <h2 style='font-size:24px;margin-bottom:15px;font-family:sans-serif;'>Konfiguration</h2>
                    <p style='font-size:12px;padding-bottom:10px;'>Pfad: {konfig_pfad}</p>
                    
                    <div style='display:flex;flex-direction:row;flex-grow:1;width:100%;'>
                        {sidebar}
                        {main}
                    </div>
                </div>
            ", 
                konfig_pfad = Konfiguration::konfiguration_pfad(),
            )
        },
        Some(PopoverState::ContextMenu(cm)) => {
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
        },
    };
    
    let pc = format!("
        <div style='background:{application_popover_color};width: 100%;height: 100%;min-height: 100%;z-index:1001;pointer-events:all;{overflow}' onmouseup='closePopOver()'>
            {pc}
        </div>", 
        overflow = if rpc_data.is_context_menu_open() { "" } else { "overflow:auto;" }, 
        application_popover_color = application_popover_color,
        pc = pc,
    );
    
    normalize_for_js(pc)
}

pub fn render_suchergebnisse_liste(data: &GrundbuchSucheResponse) -> String {
    let pc = match data {
        GrundbuchSucheResponse::StatusOk(ok) => {
            
            if ok.ergebnisse.is_empty() {
                return format!("
                    <div class='__application_suchergebnis'>
                        <div class='__application_suchergebnis_description'>
                            <h5>Keine Ergebnisse gefunden</h5>
                            <span><p>Bitte versuchen Sie es mit einem anderen Suchbegriff erneut oder wenden Sie sich an Ihren Administrator.</p></span>
                        </div>
                    </div>
                ");
            }
            
            ok.ergebnisse.iter().map(|e| {
                
                let file_name = format!("{}_{}", e.titelblatt.grundbuch_von, e.titelblatt.blatt);
                let text = if !e.gefunden_text.is_empty() {
                    e.ergebnis_text.as_str()
                    .replace(&e.gefunden_text, &format!("<strong>{}</strong>", e.gefunden_text))
                } else {
                    e.ergebnis_text.clone()
                };
                
                let download_id = &e.download_id;
                
                format!("
                    <div class='__application_suchergebnis'>
                        <div class='__application_suchergebnis_description'>
                            <h5>{file_name}.gbx</h5>
                            <span style='max-width:300px;'><p>{text}</p></span>
                        </div>
                        <div style='display: flex; flex-direction: row;flex-grow: 0;'>
                            <button class='btn' data-download-id='{download_id}' onclick='grundbuchAbbonieren(event)'>Abbonieren</button>
                            <button class='btn btn_neu' data-download-id='{download_id}' onclick='grundbuchHerunterladen(event)'>Herunterladen</button>
                        </div>
                    </div>
                ")
            })
            .collect::<Vec<_>>()
            .join("\r\n")
        },
        GrundbuchSucheResponse::StatusErr(err) => {
            let code = &err.code;
            let text = &err.text;
            format!("<div class='__application_suchergebnis'><p style='width: 500px;word-break: break-all;'>E{code}: {text}</p></div>")
        }
    };
    
    normalize_for_js(pc)
}

pub fn render_schuldenart_select() -> String {
    format!("
        <select onchange='copyToClipboardOnSelectChange(event);'>
            <option value='Grundschuld'>Grundschuld</option>
            <option value='Hypothek'>Hypothek</option>
            <option value='Rentenschuld'>Rentenschuld</option>
            <option value='Aufbauhypothek'>Aufbauhypothek</option>
            <option value='Sicherungshypothek'>Sicherungshypothek</option>
            <option value='Widerspruch'>Widerspruch</option>
            <option value='Arresthypothek'>Arresthypothek</option>
            <option value='SicherungshypothekGem128ZVG'>SicherungshypothekGem128ZVG</option>
            <option value='Hoechstbetragshypothek'>Hoechstbetragshypothek</option>
            <option value='Sicherungsgrundschuld'>Sicherungsgrundschuld</option>
            <option value='Zwangssicherungshypothek'>Zwangssicherungshypothek</option>
            <option value='NichtDefiniert'>NichtDefiniert</option>
        </select>
    ")
}

pub fn render_rechteart_select() -> String {
    format!("
        <select onchange='copyToClipboardOnSelectChange(event);'>
            <option value='Abwasserleitungsrecht'>Abwasserleitungsrecht</option>
            <option value='Auflassungsvormerkung'>Auflassungsvormerkung</option>
            <option value='Ausbeutungsrecht'>Ausbeutungsrecht</option>
            <option value='AusschlussDerAufhebungDerGemeinschaftGem1010BGB'>AusschlussDerAufhebungDerGemeinschaftGem1010BGB</option>
            <option value='Baubeschraenkung'>Baubeschraenkung</option>
            <option value='Bebauungsverbot'>Bebauungsverbot</option>
            <option value='Benutzungsrecht'>Benutzungsrecht</option>
            <option value='BenutzungsregelungGem1010BGB'>BenutzungsregelungGem1010BGB</option>
            <option value='Bepflanzungsverbot'>Bepflanzungsverbot</option>
            <option value='Bergschadenverzicht'>Bergschadenverzicht</option>
            <option value='Betretungsrecht'>Betretungsrecht</option>
            <option value='Bewässerungsrecht'>Bewässerungsrecht</option>
            <option value='BpD'>BpD</option>
            <option value='BesitzrechtNachEGBGB'>BesitzrechtNachEGBGB</option>
            <option value='BohrUndSchuerfrecht'>BohrUndSchuerfrecht</option>
            <option value='Brunnenrecht'>Brunnenrecht</option>
            <option value='Denkmalschutz'>Denkmalschutz</option>
            <option value='DinglichesNutzungsrecht'>DinglichesNutzungsrecht</option>
            <option value='DuldungVonEinwirkungenDurchBaumwurf'>DuldungVonEinwirkungenDurchBaumwurf</option>
            <option value='DuldungVonFernmeldeanlagen'>DuldungVonFernmeldeanlagen</option>
            <option value='Durchleitungsrecht'>Durchleitungsrecht</option>
            <option value='EinsitzInsitzrecht'>EinsitzInsitzrecht</option>
            <option value='Entwasserungsrecht'>Entwasserungsrecht</option>
            <option value='Erbbaurecht'>Erbbaurecht</option>
            <option value='Erwerbsvormerkung'>Erwerbsvormerkung</option>
            <option value='Fensterrecht'>Fensterrecht</option>
            <option value='Fensterverbot'>Fensterverbot</option>
            <option value='Fischereirecht'>Fischereirecht</option>
            <option value='Garagenrecht'>Garagenrecht</option>
            <option value='Gartenbenutzungsrecht'>Gartenbenutzungsrecht</option>
            <option value='GasleitungGasreglerstationFerngasltg'>GasleitungGasreglerstationFerngasltg</option>
            <option value='GehWegeFahrOderLeitungsrecht'>GehWegeFahrOderLeitungsrecht</option>
            <option value='Gewerbebetriebsbeschrankung'>Gewerbebetriebsbeschrankung</option>
            <option value='GewerblichesBenutzungsrecht'>GewerblichesBenutzungsrecht</option>
            <option value='Grenzbebauungsrecht'>Grenzbebauungsrecht</option>
            <option value='Grunddienstbarkeit'>Grunddienstbarkeit</option>
            <option value='Hochspannungsleitungsrecht'>Hochspannungsleitungsrecht</option>
            <option value='Immissionsduldungsverpflichtung'>Immissionsduldungsverpflichtung</option>
            <option value='Insolvenzvermerk'>Insolvenzvermerk</option>
            <option value='Kabelrecht'>Kabelrecht</option>
            <option value='Kanalrecht'>Kanalrecht</option>
            <option value='Kiesabbauberechtigung'>Kiesabbauberechtigung</option>
            <option value='Kraftfahrzeugabstellrecht'>Kraftfahrzeugabstellrecht</option>
            <option value='LeibgedingAltenteilsrechtAuszugsrecht'>LeibgedingAltenteilsrechtAuszugsrecht</option>
            <option value='LeitungsOderAnlagenrecht'>LeitungsOderAnlagenrecht</option>
            <option value='Mauerrecht'>Mauerrecht</option>
            <option value='Mitbenutzungsrecht'>Mitbenutzungsrecht</option>
            <option value='Mobilfunkstationsrecht'>Mobilfunkstationsrecht</option>
            <option value='Muehlenrecht'>Muehlenrecht</option>
            <option value='Mulltonnenabstellrecht'>Mulltonnenabstellrecht</option>
            <option value='Nacherbenvermerk'>Nacherbenvermerk</option>
            <option value='Niessbrauchrecht'>Niessbrauchrecht</option>
            <option value='Nutzungsbeschrankung'>Nutzungsbeschrankung</option>
            <option value='Pfandung'>Pfandung</option>
            <option value='Photovoltaikanlagenrecht'>Photovoltaikanlagenrecht</option>
            <option value='Pumpenrecht'>Pumpenrecht</option>
            <option value='Reallast'>Reallast</option>
            <option value='RegelungUeberDieHöheDerNotwegrenteGemaess912Bgb'>RegelungUeberDieHöheDerNotwegrenteGemaess912Bgb</option>
            <option value='RegelungUeberDieHöheDerUeberbaurenteGemaess912Bgb'>RegelungUeberDieHöheDerUeberbaurenteGemaess912Bgb</option>
            <option value='Rueckauflassungsvormerkung'>Rueckauflassungsvormerkung</option>
            <option value='Ruckerwerbsvormerkung'>Ruckerwerbsvormerkung</option>
            <option value='Sanierungsvermerk'>Sanierungsvermerk</option>
            <option value='Schachtrecht'>Schachtrecht</option>
            <option value='SonstigeDabagrechteart'>SonstigeDabagrechteart</option>
            <option value='SonstigeRechte'>SonstigeRechte</option>
            <option value='Tankstellenrecht'>Tankstellenrecht</option>
            <option value='Testamentsvollstreckervermerk'>Testamentsvollstreckervermerk</option>
            <option value='Transformatorenrecht'>Transformatorenrecht</option>
            <option value='Ueberbaurecht'>Ueberbaurecht</option>
            <option value='UebernahmeVonAbstandsflachen'>UebernahmeVonAbstandsflachen</option>
            <option value='Umlegungsvermerk'>Umlegungsvermerk</option>
            <option value='Umspannanlagenrecht'>Umspannanlagenrecht</option>
            <option value='Untererbbaurecht'>Untererbbaurecht</option>
            <option value='VerausserungsBelastungsverbot'>VerausserungsBelastungsverbot</option>
            <option value='Verfuegungsverbot'>Verfuegungsverbot</option>
            <option value='VerwaltungsUndBenutzungsregelung'>VerwaltungsUndBenutzungsregelung</option>
            <option value='VerwaltungsregelungGem1010Bgb'>VerwaltungsregelungGem1010Bgb</option>
            <option value='VerzichtAufNotwegerente'>VerzichtAufNotwegerente</option>
            <option value='VerzichtAufUeberbaurente'>VerzichtAufUeberbaurente</option>
            <option value='Viehtrankerecht'>Viehtrankerecht</option>
            <option value='Viehtreibrecht'>Viehtreibrecht</option>
            <option value='Vorkaufsrecht'>Vorkaufsrecht</option>
            <option value='Wasseraufnahmeverpflichtung'>Wasseraufnahmeverpflichtung</option>
            <option value='Wasserentnahmerecht'>Wasserentnahmerecht</option>
            <option value='Weiderecht'>Weiderecht</option>
            <option value='Widerspruch'>Widerspruch</option>
            <option value='Windkraftanlagenrecht'>Windkraftanlagenrecht</option>
            <option value='Wohnrecht'>Wohnrecht</option>
            <option value='WohnungsOderMitbenutzungsrecht'>WohnungsOderMitbenutzungsrecht</option>
            <option value='Wohnungsbelegungsrecht'>Wohnungsbelegungsrecht</option>
            <option value='WohnungsrechtNach1093Bgb'>WohnungsrechtNach1093Bgb</option>
            <option value='Zaunerrichtungsverbot'>Zaunerrichtungsverbot</option>
            <option value='Zaunrecht'>Zaunrecht</option>
            <option value='Zustimmungsvorbehalt'>Zustimmungsvorbehalt</option>
            <option value='Zwangsversteigerungsvermerk'>Zwangsversteigerungsvermerk</option>
            <option value='Zwangsverwaltungsvermerk'>Zwangsverwaltungsvermerk</option>
        </select>
    ")
}

pub fn render_ribbon(rpc_data: &RpcData) -> String {

    static ICON_EINSTELLUNGEN: &[u8] = include_bytes!("./img/icons8-settings-48.png");
    static ICON_HELP: &[u8] = include_bytes!("./img/icons8-help-96.png");
    static ICON_INFO: &[u8] = include_bytes!("./img/icons8-info-48.png");
    static ICON_GRUNDBUCH_OEFFNEN: &[u8] = include_bytes!("./img/icons8-book-96.png");
    static ICON_ZURUECK: &[u8] = include_bytes!("./img/icons8-back-48.png");
    static ICON_VORWAERTS: &[u8] = include_bytes!("./img/icons8-forward-48.png");
    static ICON_EXPORT_CSV: &[u8] = include_bytes!("./img/icons8-microsoft-excel-2019-96.png");
    static ICON_EXPORT_LEFIS: &[u8] = include_bytes!("./img/icons8-export-96.png");
    static ICON_DOWNLOAD: &[u8] = include_bytes!("./img/icons8-desktop-download-48.png");
    static ICON_DELETE: &[u8] = include_bytes!("./img/icons8-delete-trash-48.png");
    static ICON_PDF: &[u8] = include_bytes!("./img/icons8-pdf-48.png");
    static ICON_RECHTE_AUSGEBEN: &[u8] = include_bytes!("./img/icons8-scales-96.png");
    static ICON_FEHLER_AUSGEBEN: &[u8] = include_bytes!("./img/icons8-high-priority-96.png");
    static ICON_ABT1_AUSGEBEN: &[u8] = include_bytes!("./img/icons8-person-96.png");
    static ICON_TEILBELASTUNGEN_AUSGEBEN: &[u8] = include_bytes!("./img/icons8-pass-fail-96.png");
    static ICON_NEU: &[u8] = include_bytes!("./img/icons8-add-file-96.png");
    static ICON_SEARCH: &[u8] = include_bytes!("./img/icons8-search-in-cloud-96.png");
    static ICON_UPLOAD: &[u8] = include_bytes!("./img/icons8-upload-to-cloud-96.png");

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
                                <p>Grundbuch</p>
                                <p>laden</p>
                            </div>
                        </label>
                    </div>
                    
                    <div class='__application-ribbon-section-content'>
                        <label onmouseup='tab_functions.create_new_grundbuch(event)' class='__application-ribbon-action-vertical-large'>
                            <div class='icon-wrapper'>
                                <img class='icon' src='data:image/png;base64,{icon_neu_base64}'>
                            </div>
                            <div>
                                <p>Neues</p>
                                <p>Grundbuch</p>
                            </div>
                        </label>
                    </div>
                    
                    <div class='__application-ribbon-section-content'>
                        <label onmouseup='tab_functions.search_grundbuch(event)' class='__application-ribbon-action-vertical-large'>
                            <div class='icon-wrapper'>
                                <img class='icon' src='data:image/png;base64,{icon_search_base64}'>
                            </div>
                            <div>
                                <p>Grundbuch</p>
                                <p>suchen</p>
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
                                <p>&nbsp;</p>
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
                                <p>&nbsp;</p>
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
                        <label onmouseup='tab_functions.export_alle_rechte(event)' class='__application-ribbon-action-vertical-large'>
                            <div class='icon-wrapper'>
                                <img class='icon {disabled}' src='data:image/png;base64,{icon_rechte_speichern}'>
                            </div>
                            <div>
                                <p>Alle Rechte</p>
                                <p>speichern unter</p>
                            </div>
                        </label>
                    </div> 
                    
                    <div class='__application-ribbon-section-content'>
                        <label onmouseup='tab_functions.export_alle_fehler(event)' class='__application-ribbon-action-vertical-large'>
                            <div class='icon-wrapper'>
                                <img class='icon {disabled}' src='data:image/png;base64,{icon_fehler_speichern}'>
                            </div>
                            <div>
                                <p>Alle Fehler</p>
                                <p>speichern unter</p>
                            </div>
                        </label>
                    </div> 
                    
                    <div class='__application-ribbon-section-content'>
                        <label onmouseup='tab_functions.export_alle_teilbelastungen(event)' class='__application-ribbon-action-vertical-large'>
                            <div class='icon-wrapper'>
                                <img class='icon {disabled}' src='data:image/png;base64,{icon_export_teilbelastungen}'>
                            </div>
                            <div>
                                <p>Alle Teilbelast.</p>
                                <p>speichern unter</p>
                            </div>
                        </label>
                    </div> 
                    
                    <div class='__application-ribbon-section-content'>
                        <label onmouseup='tab_functions.export_alle_abt1(event)' class='__application-ribbon-action-vertical-large'>
                            <div class='icon-wrapper'>
                                <img class='icon {disabled}' src='data:image/png;base64,{icon_export_abt1}'>
                            </div>
                            <div>
                                <p>Alle Abt. 1</p>
                                <p>speichern unter</p>
                            </div>
                        </label>
                    </div> 
                    
                </div>
            </div>            
            
            <div class='__application-ribbon-section 5'>
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
                    
                    
                    <div class='__application-ribbon-section-content'>
                        <label onmouseup='tab_functions.open_export_pdf(event)' class='__application-ribbon-action-vertical-large'>
                            <div class='icon-wrapper'>
                                <img class='icon {disabled}' src='data:image/png;base64,{icon_export_pdf}'>
                            </div>
                            <div>
                                <p>Export</p>
                                <p>als PDF</p>
                            </div>
                        </label>
                    </div>   
                </div>
            </div>
            
            <div class='__application-ribbon-section 6'>
                <div style='display:flex;flex-direction:row;'>
                    <div class='__application-ribbon-section-content'>
                        <label onmouseup='tab_functions.upload_grundbuch(event)' class='__application-ribbon-action-vertical-large'>
                            <div class='icon-wrapper'>
                                <img class='icon {disabled}' src='data:image/png;base64,{icon_upload_lefis}'>
                            </div>
                            <div>
                                <p>Änderungen</p>
                                <p>übernehmen</p>
                            </div>
                        </label>
                    </div>
                </div>
            </div>
            
            <div class='__application-ribbon-section 7'>
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
                        <label onmouseup='tab_functions.open_help(event);' class='__application-ribbon-action-vertical-large'>
                            <div class='icon-wrapper'>
                                <img class='icon' src='data:image/png;base64,{icon_help_base64}'>
                            </div>
                            <div>
                                <p>Hilfe</p>
                                <p>&nbsp;</p>
                            </div>
                        </label>
                    </div>    
                    <div class='__application-ribbon-section-content'>
                        <label onmouseup='tab_functions.open_info(event);' class='__application-ribbon-action-vertical-large'>
                            <div class='icon-wrapper'>
                                <img class='icon' src='data:image/png;base64,{icon_info_base64}'>
                            </div>
                            <div>
                                <p>Info</p>
                                <p>&nbsp;</p>
                            </div>
                        </label>
                    </div>
                </div>
            </div>
        </div>
        ", 
        disabled = if rpc_data.loaded_files.is_empty() { " disabled" } else { "" },
        icon_open_base64 = base64::encode(ICON_GRUNDBUCH_OEFFNEN),
        icon_neu_base64 = base64::encode(ICON_NEU),
        icon_back_base64 = base64::encode(ICON_ZURUECK),
        icon_forward_base64 = base64::encode(ICON_VORWAERTS),
        icon_settings_base64 = base64::encode(ICON_EINSTELLUNGEN),
        icon_help_base64 = base64::encode(ICON_HELP),
        icon_info_base64 = base64::encode(ICON_INFO),
        icon_download_base64 = base64::encode(ICON_DOWNLOAD),
        icon_delete_base64 = base64::encode(ICON_DELETE),
        icon_export_pdf = base64::encode(ICON_PDF),
        icon_rechte_speichern = base64::encode(ICON_RECHTE_AUSGEBEN),
        icon_fehler_speichern = base64::encode(ICON_FEHLER_AUSGEBEN),
        icon_export_teilbelastungen = base64::encode(ICON_TEILBELASTUNGEN_AUSGEBEN),
        icon_export_abt1 = base64::encode(ICON_ABT1_AUSGEBEN),
        icon_search_base64 = base64::encode(ICON_SEARCH),
        icon_upload_lefis = base64::encode(ICON_UPLOAD),
        
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
        <div id='__application-main-no-files' style='display:flex;width:100%;height:100%;flex-direction:row;'>
            {main_no_files}
        </div>
    ",
        file_list = render_file_list(rpc_data),
        main_no_files = render_application_main_no_files(rpc_data),
    ))
}

pub fn render_application_main_no_files(rpc_data: &mut RpcData) -> String {
    format!("
        {page_list}
        <div style='display:flex;flex-direction:column;flex-grow:1;'>
            <div id='__application-main-container' style='{height}'>{main_container}</div>
            {pdf_image}
        </div>
    ",
        page_list = if rpc_data.loaded_file_has_no_pdf() {
            String::new()
        } else {
            format!("<div id='__application-page-list'>{page_list}</div>", page_list = render_page_list(rpc_data))
        },
        height = if rpc_data.loaded_file_has_no_pdf() { "" } else { "height: 600px;" },
        main_container = render_main_container(rpc_data),
        pdf_image = if rpc_data.loaded_file_has_no_pdf() {
            String::new()
        } else {
            format!("<div id='__application-pdf-page-image'>{pdf_image}</div>", pdf_image = render_pdf_image(rpc_data))
        }
    )
}

pub fn render_file_list(rpc_data: &RpcData) -> String {

    const CLOSE_PNG: &[u8] = include_bytes!("../src/img/icons8-close-48.png");
    let close_str = format!("data:image/png;base64,{}", base64::encode(&CLOSE_PNG));
    
    normalize_for_js(rpc_data.loaded_files.keys().filter_map(|filename| {
        
        let datei_ausgewaehlt = rpc_data.open_page.as_ref().map(|s| s.0.as_str()) == Some(filename);
        
        let datei = rpc_data.loaded_files.get(filename)?;

        let check = match datei.icon {
            None => format!("<div id='__application_file_icon-{filename}' style='width: 16px;height: 16px;margin-right:5px;flex-grow: 0;cursor: pointer;' data-fileName='{filename}'></div>"),
            Some(i) => {
                format!(
                    "<div id='__application_file_icon-{filename}' style='width: 16px;height: 16px;margin-right:5px;flex-grow: 0;cursor: pointer;'>
                        <img id='__application_file_icon-inner-{filename}' style='width: 16px;height: 16px;margin-right:5px;flex-grow: 0;cursor: pointer;' data-fileName='{filename}' src='{check}'>
                        </img>
                    </div>", 
                    filename = filename, 
                    check = i.get_base64()
                ) 
            }
        };
        
        Some(format!("<div class='{file_active}' style='user-select:none;display:flex;flex-direction:row;' data-fileName='{filename}' onmouseup='activateSelectedFile(event);'>
            {check}
            <p style='flex-grow:0;user-select:none;' data-fileName='{filename}' >{filename}</p>
            <div style='display:flex;flex-grow:1;' data-fileName='{filename}' ></div>
            {close_btn}
            </div>", 
            check = check,
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
    
    if open_file.datei.is_none() {
        return String::new();
    }
    
    let pages_div = open_file.seitenzahlen.iter().map(|page_num| {
    
        use crate::digitalisiere::SeitenTyp;
        
        let page_is_loaded = open_file.geladen.contains_key(&format!("{}", page_num));
        let page_is_active = rpc_data.open_page.as_ref().map(|s| s.1) == Some(*page_num);
        let seiten_typ = open_file.klassifikation_neu
            .get(&format!("{}", page_num)).cloned()
            .or(open_file.geladen.get(&format!("{}", page_num)).map(|p| p.typ.clone()));
        
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
    
    let has_no_pdf = rpc_data.loaded_file_has_no_pdf();

    let open_file = match rpc_data.open_page.as_mut().and_then(|of| rpc_data.loaded_files.get_mut(&of.0)) {
        Some(s) => s,
        None => return String::new(),
    };
    
    static RELOAD_PNG: &[u8] = include_bytes!("../src/img/icons8-synchronize-48.png");
    static EXPAND_PNG: &[u8] = include_bytes!("../src/img/icons8-double-left-96.png");
    static COLLAPSE_PNG: &[u8] = include_bytes!("../src/img/icons8-double-right-96.png");

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
                <div style='display:flex;flex-direction:row;height:43px;border-bottom: 1px solid #efefef;box-sizing:border-box;'>
                    <div style='display:flex;flex-grow:1;min-width:50%;overflow:hidden;'>
                        <div style='display:flex;flex-direction:row;'>
                            <h4 style='padding:10px;font-size:16px;'>Grundbuch</h4>
                            <div style='display:flex;flex-grow:1;'></div>
                            {reload_grundbuch_button}
                        </div>
                    </div>
                    {lefis_analyse}
                </div>
                <div style='max-height:calc(100% - 43px);display:flex;flex-grow:1;min-width:50%;flex-direction:row;padding:0px;'>
                    <div style='display:flex:flex-direction:column;flex-grow:1;overflow:scroll;{max_height}'>
                        <div id='__application-bestandsverzeichnis' style='margin:10px;'>{bestandsverzeichnis}</div>
                        <div id='__application-bestandsverzeichnis-veraenderungen' style='margin:10px;'>{bestandsverzeichnis_zuschreibungen}</div>
                        <div id='__application-bestandsverzeichnis-loeschungen' style='margin:10px;'>{bestandsverzeichnis_abschreibungen}</div>
                        <div id='__application-abteilung-1' style='margin:10px;'>{abt_1}</div>
                        <div id='__application-abteilung-1-grundlagen-eintragungen' style='margin:10px;'>{abt_1_grundlagen_eintragungen}</div>
                        <div id='__application-abteilung-1-veraenderungen' style='margin:10px;'>{abt_1_zuschreibungen}</div>
                        <div id='__application-abteilung-1-loeschungen' style='margin:10px;'>{abt_1_abschreibungen}</div>
                        <div id='__application-abteilung-2' style='margin:10px;'>{abt_2}</div>
                        <div id='__application-abteilung-2-veraenderungen' style='margin:10px;'>{abt_2_zuschreibungen}</div>
                        <div id='__application-abteilung-2-loeschungen' style='margin:10px;'>{abt_2_abschreibungen}</div>
                        <div id='__application-abteilung-3' style='margin:10px;'>{abt_3}</div>
                        <div id='__application-abteilung-3-veraenderungen' style='margin:10px;'>{abt_3_zuschreibungen}</div>
                        <div id='__application-abteilung-3-loeschungen' style='margin:10px;'>{abt_3_abschreibungen}</div>
                    </div>
                    {analyse_grundbuch}
                </div>
            ",
            max_height = if has_no_pdf { "max-height:calc(100% - 43px);" } else { "max-height:525px;" },
            reload_grundbuch_button = if has_no_pdf { String::new() } else { format!("
                <div style='padding:6px;'>
                    <img src='{reload_icon}' style='width:24px;height:24px;cursor:pointer;' onmouseup='reloadGrundbuch(event);'></img>
                </div>
            ", reload_icon = reload_str) },
            lefis_analyse = if rpc_data.konfiguration.lefis_analyse_einblenden {
                let collapse_icon = format!("data:image/png;base64,{}", base64::encode(&COLLAPSE_PNG));
                format!("
                    <div style='height:100%;display:flex;flex-grow:1;min-width:50%;overflow:hidden;'>
                        <div style='display:flex;flex-direction:row;'>
                            <h4 style='padding:10px;font-size:16px;'>LEFIS</h4>
                            <div style='padding:6px;'>
                                <img src='{collapse_icon}' style='width:24px;height:24px;cursor:pointer;' onmouseup='toggleLefisAnalyse(event);'></img>
                            </div>
                        </div>
                    </div>")
            } else {
                let expand_icon = format!("data:image/png;base64,{}", base64::encode(&EXPAND_PNG));
                format!("
                <div style='height:100%;display:flex;flex-grow:1;min-width:50%;overflow:hidden;'>
                    <div style='display:flex;flex-direction:row;'>
                        <h4 style='padding:10px;font-size:16px;'>LEFIS</h4>
                        <div style='display:flex;flex-grow:1;'></div>
                        <div style='padding:6px;'>
                            <img src='{expand_icon}' style='width:24px;height:24px;cursor:pointer;' onmouseup='toggleLefisAnalyse(event);'></img>
                        </div>
                    </div>
                </div>")
            },
            
            bestandsverzeichnis = render_bestandsverzeichnis(open_file, &rpc_data.konfiguration),
            bestandsverzeichnis_zuschreibungen = render_bestandsverzeichnis_zuschreibungen(open_file),
            bestandsverzeichnis_abschreibungen = render_bestandsverzeichnis_abschreibungen(open_file),
            
            abt_1 = render_abt_1(open_file),
            abt_1_grundlagen_eintragungen = render_abt_1_grundlagen_eintragungen(open_file),
            abt_1_zuschreibungen = render_abt_1_veraenderungen(open_file),
            abt_1_abschreibungen = render_abt_1_loeschungen(open_file),
            
            abt_2 = render_abt_2(open_file),
            abt_2_zuschreibungen = render_abt_2_veraenderungen(open_file),
            abt_2_abschreibungen = render_abt_2_loeschungen(open_file),
            
            abt_3 = render_abt_3(open_file, rpc_data.konfiguration.lefis_analyse_einblenden),
            abt_3_zuschreibungen = render_abt_3_veraenderungen(open_file),
            abt_3_abschreibungen = render_abt_3_loeschungen(open_file),
            analyse_grundbuch = if rpc_data.konfiguration.lefis_analyse_einblenden {
                format!("
                    <div id='__application-analyse-grundbuch' style='display:flex;flex-grow:1;min-width:50%;overflow:scroll;{max_height}'>
                        {analyse}
                    </div>
                ", 
                    max_height = if has_no_pdf { "max-height:calc(100% - 43px);" } else { "max-height:525px;" },
                    analyse = render_analyse_grundbuch(open_file, &rpc_data.loaded_nb, &rpc_data.konfiguration, false, false)
                )
            } else {
                format!("")
            }
        ))
    }
}

pub fn render_analyse_grundbuch(open_file: &PdfFile, nb: &[Nebenbeteiligter], konfiguration: &Konfiguration, fuer_druck: bool, nur_fehlerhafte_rechte: bool) -> String {
    
    const PFEIL_PNG: &[u8] = include_bytes!("../src/img/icons8-arrow-48.png");
    const WARNUNG_PNG: &[u8] = include_bytes!("../src/img/icons8-warning-48.png");
    const FEHLER_PNG: &[u8] = include_bytes!("../src/img/icons8-high-priority-48.png");

    let pfeil_str = format!("data:image/png;base64,{}", base64::encode(&PFEIL_PNG));
    let warnung_str = format!("data:image/png;base64,{}", base64::encode(&WARNUNG_PNG));
    let fehler_str = format!("data:image/png;base64,{}", base64::encode(&FEHLER_PNG));

    let gb_analysiert = crate::analysiere::analysiere_grundbuch(&open_file.analysiert, nb, konfiguration);
    
    normalize_for_js(format!("
        <div style='margin:10px;min-width:600px;'>
            {a2_header}
            {a2_analyse}
            {a3_header}
            {a3_analyse}
        </div>
        ",
        a2_header = if fuer_druck { "" } else { "<h4>Analyse Abt. 2</h4>" },
        a3_header = if fuer_druck { "" } else { "<h4>Analyse Abt. 3</h4>" },

        a2_analyse = gb_analysiert.abt2.iter()
        .filter(|a2a| if nur_fehlerhafte_rechte { !a2a.fehler.is_empty() } else { true })
        .map(|a2a| {
            format!("
            <div class='__application-abt2-analysiert' style='margin:5px;padding:10px;border:1px solid #efefef;page-break-inside:avoid;'>
                <h5 style='font-family:sans-serif;font-size:14px;margin: 0px;margin-bottom: 10px;'>{lfd_nr}&nbsp;{rechteart}</h5>
                <div style='display:flex;flex-direction:row;'>
                    <div style='min-width:{max_width};max-width:{max_width};margin-right:20px;'>
                        <p style='font-family:sans-serif;'>{text_kurz}</p>
                        {text_original}
                    </div>
                    <div style='flex-grow:1;'>
                        <p style='font-family:sans-serif;font-style:italic;display:flex;flex-grow:1;max-width:200px;'>{rechtsinhaber}</p>
                        {rangvermerk}
                        <div>{belastete_flurstuecke}</div>
                    </div>
                </div>
                <div class='__application-warnungen-und-fehler'>
                    {fehler}
                    {warnungen}
                </div>
                </div>",
                lfd_nr = if fuer_druck { format!("{} Bl. {} A2/{}",  open_file.titelblatt.grundbuch_von, open_file.titelblatt.blatt, a2a.lfd_nr) } else { format!("{}", a2a.lfd_nr) },
                text_original = if fuer_druck { format!("<p style='margin-top:10px;font-family:sans-serif;'>{}</p>", a2a.text_original) } else { String::new() }, 
                max_width = if fuer_druck { "600px" } else { "380px" },
                text_kurz = a2a.text_kurz,
                rechteart = format!("{:?}", a2a.rechteart).to_uppercase(),
                rechtsinhaber = match a2a.nebenbeteiligter.ordnungsnummer.as_ref() { 
                    Some(onr) => format!("{}/00 - {}", onr, a2a.rechtsinhaber),
                    None => a2a.rechtsinhaber.clone(),
                },
                rangvermerk = match a2a.rangvermerk.as_ref() {
                    Some(s) => format!("<span style='display:flex;align-items:center;'>
                        <img src='{warnung}' style='width:12px;height:12px;'/>
                        <p style='font-family:sans-serif;display:flex;flex-grow:1;margin-left:10px;max-width:190px;'>{rang}</p>
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
                                format!("<span style='display:flex;align-items:center;max-width:200px;'>
                                    <img src='{pfeil}' style='width:12px;height:12px;'/>
                                    <p style='font-family:sans-serif;display:inline-block;margin-left:10px;'>BV-Nr. {bv_nr}: Fl. {flur}, Flst. {flurstueck}</p>
                                    </span>", 
                                    pfeil = pfeil_str,
                                    flur = flst.flur,
                                    flurstueck = flst.flurstueck,
                                    bv_nr = flst.lfd_nr,
                                ) 
                            },
                            BvEintrag::Recht(recht) => {
                                format!("<span style='display:flex;align-items:center;max-width:200px;'>
                                    <img src='{pfeil}' style='width:12px;height:12px;'/>
                                    <p style='font-family:sans-serif;display:inline-block;margin-left:10px;'>BV-Nr. {bv_nr}: Grundstücksgl. Recht</p>
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
                    
                    fehler
                    .iter()
                    .map(|w| {
                        format!("<span style='display:flex;margin-top:5px;padding: 4px 8px; background:rgb(255,195,195);'>
                            <img src='{fehler_icon}' style='width:12px;height:12px;'/>
                                <p style='display:inline-block;margin-left:10px;color:rgb(129,8,8);'>{text}</p>
                            </span>", 
                            fehler_icon = fehler_str,
                            text = w,
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\r\n")
                },
                warnungen = {
                
                    let mut warnungen = a2a.warnungen.clone();
                    warnungen.sort();
                    warnungen.dedup();
                    
                    warnungen
                    .iter()
                    .filter(|w| if fuer_druck && w.as_str() == "Konnte keine Ordnungsnummer finden" { false } else { true })
                    .map(|w| {
                    format!("<span style='display:flex;margin-top:5px;padding: 4px 8px; background:rgb(255,255,167);'>
                            <img src='{warnung_icon}' style='width:12px;height:12px;'/>
                                <p style='display:inline-block;margin-left:10px;'>{text}</p>
                            </span>", 
                            warnung_icon = warnung_str,
                            text = w,
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\r\n")
                },
            )
        }).collect::<Vec<String>>().join("\r\n"),
    
        a3_analyse = gb_analysiert.abt3.iter()
        .filter(|a3a| if nur_fehlerhafte_rechte { !a3a.fehler.is_empty() } else { true })
        .map(|a3a| {
                    
            let waehrung_str = a3a.betrag.waehrung.to_string();
            
            format!("
            <div class='__application-abt2-analysiert' style='margin:5px;padding:10px;border:1px solid #efefef;page-break-inside:avoid;'>
                <h5 style='font-family:sans-serif;margin: 0px;margin-bottom: 10px;'>{lfd_nr}&nbsp;{schuldenart}&nbsp;{betrag}</h5>
                    <div style='font-family:sans-serif;display:flex;flex-direction:row;'>
                        <div style='min-width:{max_width};max-width:{max_width};margin-right:20px;'>
                            <p style='font-family:sans-serif;'>{text_kurz}</p>
                            {text_original}
                        </div>
                        <div style='flex-grow:1;'>
                            <p style='font-family:sans-serif;font-style:italic'>{rechtsinhaber}</p>
                            <div>{belastete_flurstuecke}</div>
                        </div>
                    </div>
                    <div class='__application-warnungen-und-fehler'>
                        {fehler}
                        {warnungen}
                    </div>
                </div>",
                lfd_nr = if fuer_druck { format!("{} Bl. {} A3/{}",  open_file.titelblatt.grundbuch_von, open_file.titelblatt.blatt, a3a.lfd_nr) } else { format!("{}", a3a.lfd_nr) },
                text_original = if fuer_druck { format!("<p style='margin-top:10px;font-family:sans-serif;'>{}</p>", a3a.text_original) } else { String::new() }, 
                max_width = if fuer_druck { "600px" } else { "380px" },
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
                                format!("<span style='display:flex;align-items:center;'>
                                    <img src='{pfeil}' style='width:12px;height:12px;'/>
                                    <p style='font-family:sans-serif;display:inline-block;margin-left:10px;'>BV-Nr. {bv_nr}: Fl. {flur}, Flst. {flurstueck}</p>
                                    </span>", 
                                    pfeil = pfeil_str,
                                    flur = flst.flur,
                                    flurstueck = flst.flurstueck,
                                    bv_nr = flst.lfd_nr,
                                ) 
                            },
                            BvEintrag::Recht(recht) => {
                                format!("<span style='display:flex;align-items:center;'>
                                    <img src='{pfeil}' style='width:12px;height:12px;'/>
                                    <p style='font-family:sans-serif;display:inline-block;margin-left:10px;'>BV-Nr. {bv_nr}: Grundstücksgl. Recht</p>
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
                    
                    warnungen
                    .iter()
                    .filter(|w| if fuer_druck && w.as_str() == "Konnte keine Ordnungsnummer finden" { false } else { true })
                    .map(|w| {
                    format!("<span style='display:flex;margin-top:5px;padding: 4px 8px; background:rgb(255,255,167);'>
                            <img src='{warnung_icon}' style='width:12px;height:12px;'/>
                                <p style='display:inline-block;margin-left:10px;'>{text}</p>
                            </span>", 
                            warnung_icon = warnung_str,
                            text = w,
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\r\n")
                },
            )
        }).collect::<Vec<String>>().join("\r\n"),
    ))

}

pub fn render_bestandsverzeichnis(open_file: &PdfFile, konfiguration: &Konfiguration) -> String {
    
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
                
                    <div style='display:flex;flex-direction:row;flex-grow:1;max-width: none;width: 100%;margin-left:10px;'>
                        {lfd_nr_textfield}
                        {bisherige_lfd_nr_textfield}
                        {gemarkung_textfield}
                        {flur_textfield}
                        {flurstueck_textfield}
                        {input_beschreibung_textfield}
                    </div>

                    <div style='display:flex;flex-direction:row;flex-grow:1;'>
                        <div style='display:flex;flex-grow:1'></div>
                        <button onclick='eintragNeu(\"bv:{zeile_nr}\")' tabindex='-1' class='btn btn_neu' >neu</button>
                        <button onclick='eintragRoeten(\"bv:{zeile_nr}\")' tabindex='-1' class='btn btn_roeten'>röten</button>
                        <button onclick='eintragLoeschen(\"bv:{zeile_nr}\")' tabindex='-1' class='btn btn_loeschen'>löschen</button>
                    </div>
                </div>",
                    zeile_nr = zeile_nr,
                    lfd_nr_textfield = StringOrLines::SingleLine(flst.lfd_nr.to_string()).get_html_editable_textfield(
                        60, // px width
                        bve.ist_geroetet(),
                        format!("bv_{zeile_nr}_lfd-nr"),
                        format!("bv:{zeile_nr}:lfd-nr"),
                        TextInputType::Number
                    ),
                    bisherige_lfd_nr_textfield = StringOrLines::SingleLine(flst.bisherige_lfd_nr.map(|f| format!("{}", f)).unwrap_or_default()).get_html_editable_textfield(
                        60, // px width
                        bve.ist_geroetet(),
                        format!("bv_{zeile_nr}_bisherige-lfd-nr"),
                        format!("bv:{zeile_nr}:bisherige-lfd-nr"),
                        TextInputType::Number
                    ),
                    gemarkung_textfield = StringOrLines::SingleLine(flst.gemarkung.clone().unwrap_or_default()).get_html_editable_textfield(
                        150, // px width
                        bve.ist_geroetet(),
                        format!("bv_{zeile_nr}_gemarkung"),
                        format!("bv:{zeile_nr}:gemarkung"),
                        TextInputType::Text
                    ),
                    flur_textfield = StringOrLines::SingleLine(flst.flur.to_string()).get_html_editable_textfield(
                        60, // px width
                        bve.ist_geroetet(),
                        format!("bv_{zeile_nr}_flur"),
                        format!("bv:{zeile_nr}:flur"),
                        TextInputType::Number
                    ),
                    flurstueck_textfield = StringOrLines::SingleLine(flst.flurstueck.clone()).get_html_editable_textfield(
                        60, // px width
                        bve.ist_geroetet(),
                        format!("bv_{zeile_nr}_flurstueck"),
                        format!("bv:{zeile_nr}:flurstueck"),
                        TextInputType::Text
                    ),
                    input_beschreibung_textfield = if konfiguration.lefis_analyse_einblenden {
                        String::new()
                    } else {
                        format!("
                            {beschreibung_textfield}
                            {groesse_textfield}
                        ", 
                            beschreibung_textfield = flst.bezeichnung.clone().unwrap_or_default().get_html_editable_textfield(
                                0, // px width
                                bve.ist_geroetet(),
                                format!("bv_{zeile_nr}_bezeichnung"),
                                format!("bv:{zeile_nr}:bezeichnung"),
                                TextInputType::Text
                            ),
                            groesse_textfield = StringOrLines::SingleLine(flst.groesse.get_m2().to_string()).get_html_editable_textfield(
                                90, // px width
                                bve.ist_geroetet(),
                                format!("bv_{zeile_nr}_groesse"),
                                format!("bv:{zeile_nr}:groesse"),
                                TextInputType::Number
                            ),
                        )
                    }
                )
            },
            BvEintrag::Recht(recht) => {
                format!("
                <div class='__application-bestandsverzeichnis-eintrag' style='display:flex;'>
                    <select style='width: 60px;{bv_geroetet}' id='bv_{zeile_nr}_typ' onchange='bvEintragTypAendern(\"bv:{zeile_nr}:typ\", this.options[this.selectedIndex].value)'>
                        <option value='flst'>Flst.</option>
                        <option value='recht' selected='selected'>Recht</option>
                    </select>
                    
                    <input type='number' style='margin-left:10px;width: 30px;{bv_geroetet}' value='{lfd_nr}' 
                        id='bv_{zeile_nr}_lfd-nr'
                        onkeyup='inputOnKeyDown(\"bv:{zeile_nr}:lfd-nr\", event)' 
                        oninput='editText(\"bv:{zeile_nr}:lfd-nr\", event)'
                    />
                    
                    <input type='number' placeholder='Bisherige lfd. Nr.' style='width: 80px;{bv_geroetet}' value='{bisherige_lfd_nr}' 
                        id='bv_{zeile_nr}_bisherige-lfd-nr'
                        onkeyup='inputOnKeyDown(\"bv:{zeile_nr}:bisherige-lfd-nr\", event)'
                        oninput='editText(\"bv:{zeile_nr}:bisherige-lfd-nr\", event)'
                    />
                    {zu_nr_textfield}

                    {text_recht_textfield}

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
                    zu_nr_textfield = recht.zu_nr.clone().get_html_editable_textfield(
                        320, // px width
                        bve.ist_geroetet(),
                        format!("bv_{zeile_nr}_zu-nr"),
                        format!("bv:{zeile_nr}:zu-nr"),
                        TextInputType::Text
                    ),
                    text_recht_textfield = recht.text.clone().get_html_editable_textfield(
                        320, // px width
                        bve.ist_geroetet(),
                        format!("bv_{zeile_nr}_recht-text"),
                        format!("bv:{zeile_nr}:recht-text"),
                        TextInputType::Text
                    ),
                    bisherige_lfd_nr = recht.bisherige_lfd_nr.map(|f| format!("{}", f)).unwrap_or_default(),
                )
            },
        }
    }).collect::<Vec<String>>().join("\r\n");
    
    normalize_for_js(format!("
        <h4 style='position:sticky;top:0;background:white;padding:10px 0px;'>Bestandsverzeichnis</h4>
        
        <div class='__application-table-header' style='display:flex;flex-direction:row;'>
            <p style='width: 60px;'>Typ</p>
            <p style='width: 60px;'>Nr.</p>
            <p style='width: 60px;'>Nr. (alt)</p>
            <p style='width: 150px;'>Gemarkung</p>
            <p style='width: 60px;'>Flur</p>
            <p style='width: 60px;'>Flst.</p>
            {p_bezeichnung}
        </div>
        {bv}
    ", 
        bv = bv, 
        p_bezeichnung = if konfiguration.lefis_analyse_einblenden { 
            "" 
        } else { 
            "
            <p style='flex-grow:1;display:flex;'>Bezeichnung</p>
            <p style='width: 90px;margin-right:162px;'>Größe (m2)</p>
            " 
        }
    ))
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
        
            <div style='display:flex;flex-direction:row;flex-grow:1;max-width: none;width: 100%;'>
                {bv_nr_textfield}
                {bv_veraenderung_text_textfield}
            </div>
            
            <div style='display:flex;flex-direction:row;flex-grow:1;'>
                <div style='display:flex;flex-grow:1'></div>
                <button onclick='eintragNeu(\"bv-zuschreibung:{zeile_nr}\")' tabindex='-1' class='btn btn_neu' >neu</button>
                <button onclick='eintragRoeten(\"bv-zuschreibung:{zeile_nr}\")' tabindex='-1' class='btn btn_roeten'>röten</button>
                <button onclick='eintragLoeschen(\"bv-zuschreibung:{zeile_nr}\")' tabindex='-1' class='btn btn_loeschen'>löschen</button>
            </div>
        </div>", 
            zeile_nr = zeile_nr,
            bv_nr_textfield = bvz.bv_nr.clone().get_html_editable_textfield(
                90, // px width
                bvz.ist_geroetet(),
                format!("bv-zuschreibung_{zeile_nr}_bv-nr"),
                format!("bv-zuschreibung:{zeile_nr}:bv-nr"),
                TextInputType::Text
            ),
            bv_veraenderung_text_textfield = bvz.text.clone().get_html_editable_textfield(
                0, // px width
                bvz.ist_geroetet(),
                format!("bv-zuschreibung_{zeile_nr}_text"),
                format!("bv-zuschreibung:{zeile_nr}:text"),
                TextInputType::Text
            ),
        )
    }).collect::<Vec<String>>().join("\r\n");
    
    normalize_for_js(format!("
        <h4 style='position:sticky;top:0;background:white;padding:10px 0px;'>Bestandsverzeichnis - Zuschreibungen</h4>
        
        <div class='__application-table-header' style='display:flex;flex-direction:row;'>
            <p style='width: 90px;'>BV-Nr.</p>
            <p style='display:flex;flex-grow:1;'>Text</p>
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
        
            <div style='display:flex;flex-direction:row;flex-grow:1;max-width: none;width: 100%;'>
                {bv_nr_textfield}
                {bv_abschreibung_text_textfield}
            </div>
            
            <div style='display:flex;flex-direction:row;flex-grow:1;'>
                <div style='display:flex;flex-grow:1'></div>
                <button onclick='eintragNeu(\"bv-abschreibung:{zeile_nr}\")' tabindex='-1' class='btn btn_neu' >neu</button>
                <button onclick='eintragRoeten(\"bv-abschreibung:{zeile_nr}\")' tabindex='-1' class='btn btn_roeten'>röten</button>
                <button onclick='eintragLoeschen(\"bv-abschreibung:{zeile_nr}\")' tabindex='-1' class='btn btn_loeschen'>löschen</button>
            </div>
        </div>", 
            zeile_nr = zeile_nr,
            bv_nr_textfield = bva.bv_nr.clone().get_html_editable_textfield(
                90, // px width
                bva.ist_geroetet(),
                format!("bv-abschreibung_{zeile_nr}_bv-nr"),
                format!("bv-abschreibung:{zeile_nr}:bv-nr"),
                TextInputType::Text
            ),
            bv_abschreibung_text_textfield = bva.text.clone().get_html_editable_textfield(
                0, // px width
                bva.ist_geroetet(),
                format!("bv-abschreibung_{zeile_nr}_text"),
                format!("bv-abschreibung:{zeile_nr}:text"),
                TextInputType::Text
            ),
        )
    }).collect::<Vec<String>>().join("\r\n");
    
    normalize_for_js(format!("
        <h4 style='position:sticky;top:0;background:white;padding:10px 0px;'>Bestandsverzeichnis - Abschreibungen</h4>
        
        <div class='__application-table-header' style='display:flex;flex-direction:row;'>
            <p style='width: 90px;'>BV-Nr.</p>
            <p style='display:flex;flex-grow:1;'>Text</p>
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
    
    let abt1 = abt1_eintraege
    .iter()
    .enumerate()
    .filter_map(|(zeile_nr, abt1)| match abt1 {
        Abt1Eintrag::V1(_) => None,
        Abt1Eintrag::V2(v2) => Some((zeile_nr, v2)),
    })
    .map(|(zeile_nr, abt1)| {
    
        let bv_geroetet = if abt1.ist_geroetet() { 
            "background:rgb(255,195,195);" 
        } else { 
            "background:white;" 
        };
        
        format!("
        <div class='__application-abt1-eintrag' style='display:flex;margin-top:5px;'>
            
            <div style='display:flex;flex-direction:row;flex-grow:1;max-width: none;width: 100%;'>
                {lfd_nr_textfield}
                {eigentuemer_textfield}
            </div>
            
            <div style='display:flex;flex-direction:row;flex-grow:1;'>
                <div style='display:flex;flex-grow:1'></div>
                <button onclick='eintragNeu(\"abt1:{zeile_nr}\")' tabindex='-1' class='btn btn_neu' >neu</button>
                <button onclick='eintragRoeten(\"abt1:{zeile_nr}\")' tabindex='-1' class='btn btn_roeten'>röten</button>
                <button onclick='eintragLoeschen(\"abt1:{zeile_nr}\")' tabindex='-1' class='btn btn_loeschen'>löschen</button>
            </div>
            
        </div>", 
            zeile_nr = zeile_nr,
            lfd_nr_textfield = StringOrLines::SingleLine(abt1.lfd_nr.to_string()).get_html_editable_textfield(
                90, // px width
                abt1.ist_geroetet(),
                format!("abt1_{zeile_nr}_lfd-nr"),
                format!("abt1:{zeile_nr}:lfd-nr"),
                TextInputType::Number
            ),
            eigentuemer_textfield = abt1.eigentuemer.get_html_editable_textfield(
                0, // px width
                abt1.ist_geroetet(),
                format!("abt1_{zeile_nr}_eigentuemer"),
                format!("abt1:{zeile_nr}:eigentuemer"),
                TextInputType::Text
            ),
        )
    })
    .collect::<Vec<String>>()
    .join("\r\n");
    
    normalize_for_js(format!("
    <h4 style='position:sticky;top:0;background:white;padding:10px 0px;'>Abteilung 1</h4>
    
    <div class='__application-table-header' style='display:flex;flex-grow:1;'>
    <p style='width: 90px;'>Nr.</p>
    <p style='display:flex;flex-grow:1'>Eigentümer</p>
    </div>
    
    {abt1}", abt1 = abt1))
}

pub fn render_abt_1_grundlagen_eintragungen(open_file: &PdfFile) -> String {
    
    use crate::digitalisiere::Abt1GrundEintragung;
    
    let mut abt1_eintraege = open_file.analysiert.abt1.grundlagen_eintragungen.clone();
    if abt1_eintraege.is_empty() {
        abt1_eintraege = vec![Abt1GrundEintragung::new()];
    }
    
    let abt1 = abt1_eintraege
    .iter()
    .enumerate()
    .map(|(zeile_nr, abt1)| {
    
        let bv_geroetet = if abt1.ist_geroetet() { 
            "background:rgb(255,195,195);" 
        } else { 
            "background:white;" 
        };

        format!("
        <div class='__application-abt1-grundlage-eintragung' style='display:flex;margin-top:5px;'>
            
            <div style='display:flex;flex-direction:row;flex-grow:1;max-width: none;width: 100%;'>
                {bv_nr_textfield}
                {grundlage_der_eintragung_textfield}
            </div>
            
            <div style='display:flex;flex-direction:row;flex-grow:1;'>
                <div style='display:flex;flex-grow:1'></div>
                <button onclick='eintragNeu(\"abt1-grundlage-eintragung:{zeile_nr}\")' tabindex='-1' class='btn btn_neu' >neu</button>
                <button onclick='eintragRoeten(\"abt1-grundlage-eintragung:{zeile_nr}\")' tabindex='-1' class='btn btn_roeten'>röten</button>
                <button onclick='eintragLoeschen(\"abt1-grundlage-eintragung:{zeile_nr}\")' tabindex='-1' class='btn btn_loeschen'>löschen</button>
            </div>
            
        </div>", 
            zeile_nr = zeile_nr,
            
            bv_nr_textfield = abt1.bv_nr.get_html_editable_textfield(
                60, // px width
                abt1.ist_geroetet(),
                format!("abt1-grundlage-eintragung_{zeile_nr}_bv-nr"),
                format!("abt1-grundlage-eintragung:{zeile_nr}:bv-nr"),
                TextInputType::Text
            ),
            
            grundlage_der_eintragung_textfield = abt1.text.get_html_editable_textfield(
                0, // px width
                abt1.ist_geroetet(),
                format!("abt1-grundlage-eintragung_{zeile_nr}_text"),
                format!("abt1-grundlage-eintragung:{zeile_nr}:text"),
                TextInputType::Text
            ),
        )
    })
    
    .collect::<Vec<String>>()
    .join("\r\n");
    
    normalize_for_js(format!("
           <h4 style='position:sticky;top:0;background:white;padding:10px 0px;'>Abteilung 1 - Grundlagen der Eintragungen</h4>
          
          <div class='__application-table-header' style='display:flex;flex-direction:row;flex-grow:1;'>
            <p style='width: 60px;'>BV-Nr.</p>
            <p style='display:flex;flex-grow:1;'>Grundlage d. Eintragung</p>
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
            
            <div style='display:flex;flex-direction:row;flex-grow:1;max-width: none;width: 100%;'>
                {lfd_nr_textfield}
                {text_textfield}
            </div>
            
            <div style='display:flex;flex-direction:row;flex-grow:1;'>
                <div style='display:flex;flex-grow:1'></div>
                <button onclick='eintragNeu(\"abt1-veraenderung:{zeile_nr}\")' tabindex='-1' class='btn btn_neu' >neu</button>
                <button onclick='eintragRoeten(\"abt1-veraenderung:{zeile_nr}\")' tabindex='-1' class='btn btn_roeten'>röten</button>
                <button onclick='eintragLoeschen(\"abt1-veraenderung:{zeile_nr}\")' tabindex='-1' class='btn btn_loeschen'>löschen</button>
            </div>
        </div>", 
            zeile_nr = zeile_nr,
            lfd_nr_textfield = abt1_a.lfd_nr.get_html_editable_textfield(
                90, // px width
                abt1_a.ist_geroetet(),
                format!("abt1-veraenderung_{zeile_nr}_lfd-nr"),
                format!("abt1-veraenderung:{zeile_nr}:lfd-nr"),
                TextInputType::Text
            ),
            text_textfield = abt1_a.text.get_html_editable_textfield(
                0, // px width
                abt1_a.ist_geroetet(),
                format!("abt1-veraenderung_{zeile_nr}_text"),
                format!("abt1-veraenderung:{zeile_nr}:text"),
                TextInputType::Text
            ),
        )
    }).collect::<Vec<String>>().join("\r\n");
    
    normalize_for_js(format!("
        <h4 style='position:sticky;top:0;background:white;padding:10px 0px;'>Abteilung 1 - Veränderungen</h4>
        
        <div class='__application-table-header' style='display:flex;flex-grow:1;'>
            <p style='width: 90px;'>lfd. Nr.</p>
            <p style='flex-grow:1;'>Text</p>
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

            <div style='display:flex;flex-direction:row;flex-grow:1;max-width: none;width: 100%;'>
                {lfd_nr_textfield}
                {text_textfield}
            </div>
            
            <div style='display:flex;flex-direction:row;flex-grow:1;'>
                <div style='display:flex;flex-grow:1'></div>
                <button onclick='eintragNeu(\"abt1-loeschung:{zeile_nr}\")' tabindex='-1' class='btn btn_neu' >neu</button>
                <button onclick='eintragRoeten(\"abt1-loeschung:{zeile_nr}\")' tabindex='-1' class='btn btn_roeten'>röten</button>
                <button onclick='eintragLoeschen(\"abt1-loeschung:{zeile_nr}\")' tabindex='-1' class='btn btn_loeschen'>löschen</button>
            </div>
        </div>", 
            zeile_nr = zeile_nr,
            lfd_nr_textfield = abt1_l.lfd_nr.get_html_editable_textfield(
                90, // px width
                abt1_l.ist_geroetet(),
                format!("abt1-loeschung_{zeile_nr}_lfd-nr"),
                format!("abt1-loeschung:{zeile_nr}:lfd-nr"),
                TextInputType::Text
            ),
            text_textfield = abt1_l.text.get_html_editable_textfield(
                0, // px width
                abt1_l.ist_geroetet(),
                format!("abt1-loeschung_{zeile_nr}_text"),
                format!("abt1-loeschung:{zeile_nr}:text"),
                TextInputType::Text
            ),
        )
    }).collect::<Vec<String>>().join("\r\n");
    
    normalize_for_js(format!("
        <h4 style='position:sticky;top:0;background:white;padding:10px 0px;'>Abteilung 1 - Löschungen</h4>
        
        <div class='__application-table-header' style='display:flex;'>
            <p style='width: 90px;'>lfd. Nr.</p>
            <p style='flex-grow:1;display:flex;'>Text</p>
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
            
            <div style='display:flex;flex-direction:row;flex-grow:1;max-width: none;width: 100%;'>
                {lfd_nr_textfield}
                {bv_nr_textfield}
                {recht_textfield}
            </div>
            
            <div style='display:flex;flex-direction:row;flex-grow:1;'>
                <div style='display:flex;flex-grow:1'></div>
                <button onclick='eintragNeu(\"abt2:{zeile_nr}\")' tabindex='-1' class='btn btn_neu' >neu</button>
                <button onclick='eintragRoeten(\"abt2:{zeile_nr}\")' tabindex='-1' class='btn btn_roeten'>röten</button>
                <button onclick='eintragLoeschen(\"abt2:{zeile_nr}\")' tabindex='-1' class='btn btn_loeschen'>löschen</button>
            </div>
        </div>", 
            zeile_nr = zeile_nr,
            lfd_nr_textfield =  StringOrLines::SingleLine(abt2.lfd_nr.to_string()).get_html_editable_textfield(
                90, // px width
                abt2.ist_geroetet(),
                format!("abt2_{zeile_nr}_lfd-nr"),
                format!("abt2:{zeile_nr}:lfd-nr"),
                TextInputType::Text
            ),
            bv_nr_textfield = abt2.bv_nr.get_html_editable_textfield(
                90, // px width
                abt2.ist_geroetet(),
                format!("abt2_{zeile_nr}_bv-nr"),
                format!("abt2:{zeile_nr}:bv-nr"),
                TextInputType::Text
            ),
            recht_textfield = abt2.text.get_html_editable_textfield(
                0, // px width
                abt2.ist_geroetet(),
                format!("abt2_{zeile_nr}_text"),
                format!("abt2:{zeile_nr}:text"),
                TextInputType::Text
            ),
        )
    })
    .collect::<Vec<String>>()
    .join("\r\n");
    
    normalize_for_js(format!("
           <h4 style='position:sticky;top:0;background:white;padding:10px 0px;'>Abteilung 2</h4>
          
          <div class='__application-table-header' style='display:flex;flex-grow:1;'>
            <p style='width: 90px;'>Nr.</p>
            <p style='width: 90px;'>BV-Nr.</p>
            <p style='flex-grow:1;'>Recht</p>
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
            
            <div style='display:flex;flex-direction:row;flex-grow:1;max-width: none;width: 100%;'>
                {lfd_nr_textfield}
                {recht_textfield}
            </div>
            
            <div style='display:flex;flex-direction:row;flex-grow:1;'>
                <div style='display:flex;flex-grow:1'></div>
                <button onclick='eintragNeu(\"abt2-veraenderung:{zeile_nr}\")' tabindex='-1' class='btn btn_neu' >neu</button>
                <button onclick='eintragRoeten(\"abt2-veraenderung:{zeile_nr}\")' tabindex='-1' class='btn btn_roeten'>röten</button>
                <button onclick='eintragLoeschen(\"abt2-veraenderung:{zeile_nr}\")' tabindex='-1' class='btn btn_loeschen'>löschen</button>
            </div>
        </div>", 
            zeile_nr = zeile_nr,
            
            lfd_nr_textfield = abt2_a.lfd_nr.get_html_editable_textfield(
                90, // px width
                abt2_a.ist_geroetet(),
                format!("abt2-veraenderung_{zeile_nr}_lfd-nr"),
                format!("abt2-veraenderung:{zeile_nr}:lfd-nr"),
                TextInputType::Text
            ),
            recht_textfield = abt2_a.text.get_html_editable_textfield(
                320, // px width
                abt2_a.ist_geroetet(),
                format!("abt2-veraenderung_{zeile_nr}_text"),
                format!("abt2-veraenderung:{zeile_nr}:text"),
                TextInputType::Text
            ),
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

            <div style='display:flex;flex-direction:row;flex-grow:1;max-width: none;width: 100%;'>
                {lfd_nr_textfield}
                {recht_textfield}
            </div>
            
            <div style='display:flex;flex-direction:row;flex-grow:1;'>
                <div style='display:flex;flex-grow:1'></div>
                <button onclick='eintragNeu(\"abt2-loeschung:{zeile_nr}\")' tabindex='-1' class='btn btn_neu' >neu</button>
                <button onclick='eintragRoeten(\"abt2-loeschung:{zeile_nr}\")' tabindex='-1' class='btn btn_roeten'>röten</button>
                <button onclick='eintragLoeschen(\"abt2-loeschung:{zeile_nr}\")' tabindex='-1' class='btn btn_loeschen'>löschen</button>
            </div>
        </div>", 
            zeile_nr = zeile_nr,
            lfd_nr_textfield = abt2_l.lfd_nr.get_html_editable_textfield(
                90, // px width
                abt2_l.ist_geroetet(),
                format!("abt2-loeschung_{zeile_nr}_lfd-nr"),
                format!("abt2-loeschung:{zeile_nr}:lfd-nr"),
                TextInputType::Text
            ),
            recht_textfield = abt2_l.text.get_html_editable_textfield(
                320, // px width
                abt2_l.ist_geroetet(),
                format!("abt2-loeschung_{zeile_nr}_text"),
                format!("abt2-loeschung:{zeile_nr}:text"),
                TextInputType::Text
            ),
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

pub fn render_abt_3(open_file: &PdfFile, show_lefis: bool) -> String {
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
            
            <div style='display:flex;flex-direction:row;flex-grow:1;max-width: none;width: 100%;'>
                {bv_nr_textfield}
                {betrag_textfield}
                {recht_textfield}
            </div>
            
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
            
            bv_nr_textfield = abt3.bv_nr.get_html_editable_textfield(
                if show_lefis { 40 } else { 60 }, // px width
                abt3.ist_geroetet(),
                format!("abt3_{zeile_nr}_bv-nr"),
                format!("abt3:{zeile_nr}:bv-nr"),
                TextInputType::Text
            ),
            
            betrag_textfield = abt3.betrag.get_html_editable_textfield(
                if show_lefis { 90 } else { 180 }, // px width
                abt3.ist_geroetet(),
                format!("abt3_{zeile_nr}_betrag"),
                format!("abt3:{zeile_nr}:betrag"),
                TextInputType::Text
            ),
            
            recht_textfield = abt3.text.get_html_editable_textfield(
                0, // px width
                abt3.ist_geroetet(),
                format!("abt3_{zeile_nr}_text"),
                format!("abt3:{zeile_nr}:text"),
                TextInputType::Text
            ),
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
            
            <div style='display:flex;flex-direction:row;flex-grow:1;max-width: none;width: 100%;'>
                {lfd_nr_textfield}
                {betrag_textfield}
                {recht_textfield}
            </div>
            
            <div style='display:flex;flex-direction:row;flex-grow:1;'>
                <div style='display:flex;flex-grow:1'></div>
                <button onclick='eintragNeu(\"abt3-veraenderung:{zeile_nr}\")' tabindex='-1' class='btn btn_neu' >neu</button>
                <button onclick='eintragRoeten(\"abt3-veraenderung:{zeile_nr}\")' tabindex='-1' class='btn btn_roeten'>röten</button>
                <button onclick='eintragLoeschen(\"abt3-veraenderung:{zeile_nr}\")' tabindex='-1' class='btn btn_loeschen'>löschen</button>
            </div>
        </div>",
            zeile_nr = zeile_nr,
            
            lfd_nr_textfield = abt3_a.lfd_nr.get_html_editable_textfield(
                90, // px width
                abt3_a.ist_geroetet(),
                format!("abt3-veraenderung_{zeile_nr}_lfd-nr"),
                format!("abt3-veraenderung:{zeile_nr}:lfd-nr"),
                TextInputType::Text
            ),
            
            betrag_textfield = abt3_a.betrag.get_html_editable_textfield(
                120, // px width
                abt3_a.ist_geroetet(),
                format!("abt3-veraenderung_{zeile_nr}_betrag"),
                format!("abt3-veraenderung:{zeile_nr}:betrag"),
                TextInputType::Text
            ),
            
            recht_textfield = abt3_a.text.get_html_editable_textfield(
                320, // px width
                abt3_a.ist_geroetet(),
                format!("abt3-veraenderung_{zeile_nr}_text"),
                format!("abt3-veraenderung:{zeile_nr}:text"),
                TextInputType::Text
            ),
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

            <div style='display:flex;flex-direction:row;flex-grow:1;max-width: none;width: 100%;'>
                {lfd_nr_textfield}
                {betrag_textfield}
                {recht_textfield}
            </div>
            
            <div style='display:flex;flex-direction:row;flex-grow:1;'>
                <div style='display:flex;flex-grow:1'></div>
                <button onclick='eintragNeu(\"abt3-loeschung:{zeile_nr}\")' tabindex='-1' class='btn btn_neu' >neu</button>
                <button onclick='eintragRoeten(\"abt3-loeschung:{zeile_nr}\")' tabindex='-1' class='btn btn_roeten'>röten</button>
                <button onclick='eintragLoeschen(\"abt3-loeschung:{zeile_nr}\")' tabindex='-1' class='btn btn_loeschen'>löschen</button>
            </div>
        </div>", 
            zeile_nr = zeile_nr,
            lfd_nr_textfield = abt3_l.lfd_nr.get_html_editable_textfield(
                90, // px width
                abt3_l.ist_geroetet(),
                format!("abt3-loeschung_{zeile_nr}_lfd-nr"),
                format!("abt3-loeschung:{zeile_nr}:lfd-nr"),
                TextInputType::Text
            ),
            
            betrag_textfield = abt3_l.betrag.get_html_editable_textfield(
                120, // px width
                abt3_l.ist_geroetet(),
                format!("abt3-loeschung_{zeile_nr}_betrag"),
                format!("abt3-loeschung:{zeile_nr}:betrag"),
                TextInputType::Text
            ),
            
            recht_textfield = abt3_l.text.get_html_editable_textfield(
                320, // px width
                abt3_l.ist_geroetet(),
                format!("abt3-loeschung_{zeile_nr}_text"),
                format!("abt3-loeschung:{zeile_nr}:text"),
                TextInputType::Text
            ),
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
    
    if file.datei.is_none() {
        return String::new();
    }
    
    let max_seitenzahl = file.seitenzahlen.iter().copied().max().unwrap_or(0);
    
    let temp_ordner = std::env::temp_dir()
    .join(&format!("{gemarkung}/{blatt}", gemarkung = file.titelblatt.grundbuch_von, blatt = file.titelblatt.blatt));
    
    let temp_pdf_pfad = temp_ordner.clone().join("temp.pdf");
    let pdftoppm_output_path = if rpc_data.konfiguration.vorschau_ohne_geroetet {
        temp_ordner.clone().join(format!("page-clean-{}.png", crate::digitalisiere::formatiere_seitenzahl(open_file.1, max_seitenzahl)))
    } else {
        temp_ordner.clone().join(format!("page-{}.png", crate::digitalisiere::formatiere_seitenzahl(open_file.1, max_seitenzahl)))
    };
    
    let pdf_to_ppm_bytes = match std::fs::read(&pdftoppm_output_path) {
        Ok(o) => o,
        Err(_) => return String::new(),
    };

    let (im_width, im_height, page_width, page_height) = match file.pdftotext_layout.seiten.get(&format!("{}", open_file.1)) {
        Some(o) => (o.breite_mm as f32 / 25.4 * 600.0, o.hoehe_mm as f32 / 25.4 * 600.0, o.breite_mm, o.hoehe_mm),
        None => return String::new(),
    };
    
    let img_ui_width = 1200.0; // px
    let aspect_ratio = im_height / im_width;
    let img_ui_height = img_ui_width * aspect_ratio;
    
    let columns = match file.geladen.get(&format!("{}", open_file.1)) {
        Some(page) =>  {
            
            let seitentyp = match file.klassifikation_neu.get(&format!("{}", open_file.1)) {
                Some(s) => *s,
                None => page.typ,
            };
                                                    
            seitentyp
            .get_columns(file.anpassungen_seite.get(&format!("{}", open_file.1)))
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
    .get(&format!("{}", open_file.1))
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

