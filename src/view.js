
"use strict";

let rpc = {
    
  invoke : function(arg) { window.webkit.messageHandlers.external.postMessage(JSON.stringify(arg)); },
  init: function() { rpc.invoke({ cmd: 'init' }); },
  
  load_pdf: function() { rpc.invoke({ cmd : 'load_pdf' }); },
  undo:  function() { rpc.invoke({ cmd : 'undo' }); },
  redo:  function() { rpc.invoke({ cmd : 'redo' }); },
  export_nb:  function() { rpc.invoke({ cmd : 'export_nb' }); },
  import_nb:  function() { rpc.invoke({ cmd : 'import_nb' }); },
  export_lefis:  function() { rpc.invoke({ cmd : 'export_lefis' }); },
  delete_nb: function() { rpc.invoke({ cmd : 'delete_nb' }); },
  open_info: function() { rpc.invoke({ cmd : 'open_info' }); },
  open_configuration: function() { rpc.invoke({ cmd : 'open_configuration' }); },
  
  check_for_pdf_loaded: function(arg, arg2) { rpc.invoke({ cmd : 'check_for_pdf_loaded', file_path: arg, file_name: arg2 }); },
  edit_text: function(arg, arg2) { rpc.invoke({ cmd : 'edit_text', path: arg, new_value: arg2 }); },
  eintrag_neu: function(arg) { rpc.invoke({ cmd : 'eintrag_neu', path: arg }); },
  eintrag_loeschen: function(arg) { rpc.invoke({ cmd : 'eintrag_loeschen', path: arg }); },
  eintrag_roeten: function(arg) { rpc.invoke({ cmd : 'eintrag_roeten', path: arg }); },
  open_context_menu: function(x, y, seite) { rpc.invoke({ cmd : 'open_context_menu', x: x, y: y, seite: seite }); },
  close_pop_over:  function() { rpc.invoke({ cmd : 'close_pop_over' }); },
  close_file: function(arg) { rpc.invoke({ cmd : 'close_file', file_name: arg }); },
 
  edit_regex_key: function(old_key, new_key) { rpc.invoke({ cmd: 'edit_regex_key', old_key: old_key, new_key: new_key }); },
  edit_regex_value: function(key, value) { rpc.invoke({ cmd: 'edit_regex_value', key: key, value: value }); },
  insert_regex: function(arg) { rpc.invoke({ cmd : 'insert_regex', regex_key: arg });},
  teste_regex: function(regex_id, text) { rpc.invoke({ cmd: 'teste_regex', regex_id: regex_id, text: text }); },
  regex_loeschen: function(arg) { rpc.invoke({ cmd : 'regex_loeschen', regex_key: arg });},

  edit_abkuerzungen_script: function(arg) { rpc.invoke({ cmd : 'edit_abkuerzungen_script', script: arg });},
  edit_text_saubern_script: function(arg) { rpc.invoke({ cmd : 'edit_text_saubern_script', script: arg });},

  edit_text_kuerzen_abt2_script: function(arg) { rpc.invoke({ cmd : 'edit_text_kuerzen_abt2_script', script: arg });},
  kurztext_abt2_script_testen: function(arg) { rpc.invoke({ cmd: 'kurztext_abt2_script_testen', text: arg }); },  
  edit_rechteart_script: function(neu) { rpc.invoke({ cmd: 'edit_rechteart_script', neu: neu }); },
  rechteart_script_testen: function(arg) { rpc.invoke({ cmd: 'rechteart_script_testen', text: arg }); },
  edit_rechtsinhaber_auslesen_abt2_script: function(neu) { rpc.invoke({ cmd: 'edit_rechtsinhaber_auslesen_abt2_script', neu: neu }); },
  rechtsinhaber_auslesen_abt2_script_testen: function(arg) { rpc.invoke({ cmd: 'rechtsinhaber_auslesen_abt2_script_testen', text: arg }); },
  edit_rangvermerk_auslesen_abt2_script: function(neu) { rpc.invoke({ cmd: 'edit_rangvermerk_auslesen_abt2_script', neu: neu }); },
  rangvermerk_auslesen_abt2_script_testen: function(arg) { rpc.invoke({ cmd: 'rangvermerk_auslesen_abt2_script_testen', text: arg }); },
  
  edit_text_kuerzen_abt3_script: function(arg) { rpc.invoke({ cmd : 'edit_text_kuerzen_abt3_script', script: arg });},
  kurztext_abt3_script_testen: function(arg) { rpc.invoke({ cmd: 'kurztext_abt3_script_testen', text: arg }); },
  edit_betrag_auslesen_script: function(neu) { rpc.invoke({ cmd: 'edit_betrag_auslesen_script', neu: neu }); },
  betrag_auslesen_script_testen: function(arg) { rpc.invoke({ cmd: 'betrag_auslesen_script_testen', text: arg }); },
  edit_schuldenart_script: function(neu) { rpc.invoke({ cmd: 'edit_schuldenart_script', neu: neu }); },
  schuldenart_script_testen: function(arg) { rpc.invoke({ cmd: 'schuldenart_script_testen', text: arg }); },
  edit_rechtsinhaber_auslesen_abt3_script: function(neu) { rpc.invoke({ cmd: 'edit_rechtsinhaber_auslesen_abt3_script', neu: neu }); },
  rechtsinhaber_auslesen_abt3_script_testen: function(arg) { rpc.invoke({ cmd: 'rechtsinhaber_auslesen_abt3_script_testen', text: arg }); },
  
  klassifiziere_seite_neu: function(seite, klassifikation_neu) { rpc.invoke({ cmd: 'klassifiziere_seite_neu', seite: seite, klassifikation_neu: klassifikation_neu }); },
  
  set_active_ribbon_tab: function(arg) { rpc.invoke({ cmd : 'set_active_ribbon_tab', new_tab: arg }); },
  set_open_file: function(arg) { rpc.invoke({ cmd : 'set_open_file', new_file: arg }); },
  set_open_page: function(arg) { rpc.invoke({ cmd : 'set_open_page', active_page: arg }); },
};

let tab_functions = {
    load_new_pdf: function(event) { rpc.load_pdf(); },
    undo: function(event) { rpc.undo() },
    redo: function(event) { rpc.redo() },
    export_nb: function(event) { rpc.export_nb() },
    import_nb: function(event) { rpc.import_nb() },
    delete_nb: function(event) { rpc.delete_nb() },
    export_lefis: function(event) { rpc.export_lefis() },
    open_configuration: function(event) { rpc.open_configuration() },
    open_info: function(event) { rpc.open_info() },
};

let files_to_check = {};

setInterval(function(){
    for (const [key, value] of Object.entries(files_to_check)) {
        rpc.check_for_pdf_loaded(key, value);
    }
}, 1000);

function startCheckingForPageLoaded(filepath, filename) {
    files_to_check[filepath] = filename;
}

function stopCheckingForPageLoaded(filename) {
    if (files_to_check.hasOwnProperty(filename)) {
        delete files_to_check[filename];
    }
}

function eintragNeu(path) {
    rpc.eintrag_neu(path);        
}

function eintragRoeten(path) {
    rpc.eintrag_roeten(path);        
}

function eintragLoeschen(path) {
    rpc.eintrag_loeschen(path);        
}

function inputOnKeyDown(path, e) {
    if ((event.keyCode == 10 || event.keyCode == 13) && event.ctrlKey) {
        // CTRL + Enter
        rpc.eintrag_neu(path);        
    } else if (event.keyCode == 32 && event.ctrlKey) {
        // CTRL + Leer
        rpc.eintrag_roeten(path);        
    } else if (event.keyCode == 46 && event.ctrlKey) {
        // CTRL + Entf
        rpc.eintrag_loeschen(path);        
    }
}

function editText(path, e) {
    rpc.edit_text(path, e.target.value);
}

function displayError(error) {
    console.error(error);
}

function logInfo(info) {
    // console.log(info);
}

// Function called from Rust on init to initialize the entire screen
function replaceEntireScreen(html) {
    document.body.innerHTML = html;

}

function replaceRibbon(s) {
    let ribbon = document.getElementById("__application-ribbon");
    if (ribbon)
         ribbon.innerHTML = s;
}

function replaceMain(s) {
    let ribbon = document.getElementById("__application-main");
    if (ribbon)
         ribbon.innerHTML = s;
}

function replaceFileList(s) {
    let file_list = document.getElementById("__application-file-list");
    if (file_list)
         file_list.innerHTML = s;

}

function replacePageImage(s) {
    let page_list = document.getElementById("__application-pdf-page-image");
    if (page_list)
         page_list.innerHTML = s;
}

function replacePageList(s) {
    let page_list = document.getElementById("__application-page-list");
    if (page_list)
         page_list.innerHTML = s;
}

function replaceMainContainer(s) {
    let page_list = document.getElementById("__application-main-container");
    if (page_list)
         page_list.innerHTML = s;
}

function replaceBestandsverzeichnis(s) {
    let page_list = document.getElementById("__application-bestandsverzeichnis");
    if (page_list)
         page_list.innerHTML = s;
}

function replaceAbt2(s) {
    let page_list = document.getElementById("__application-abteilung-2");
    if (page_list)
         page_list.innerHTML = s;
}

function replaceAbt3(s) {
    let page_list = document.getElementById("__application-abteilung-3");
    if (page_list)
         page_list.innerHTML = s;
}

function replaceAnalyseGrundbuch(s) {
    let page_list = document.getElementById("__application-analyse-grundbuch");
    if (page_list)
         page_list.innerHTML = s;
}

function openConfiguration(e) {
    rpc.open_configuration();

}

function openInfo(e) {
    rpc.open_info();
}

function openContextMenu(e) {
    var pn = e.target.getAttribute("data-pageNumber");
    if (!pn) {
        return;
    }
    rpc.open_context_menu(e.clientX, e.clientY, parseInt(pn));
    return false;
}

function closePopOver(s) {
    rpc.close_pop_over();
}

function activateSelectedFile(event) {
    var file = event.target.getAttribute("data-fileName");
    if (!file) {
        console.log(event.target);
        return;
    }
    rpc.set_open_file(file);
}

function activateSelectedPage(event) {
    var pn = event.target.getAttribute("data-pageNumber");
    if (!pn) {
        return;
    }
    rpc.set_open_page(parseInt(pn, 10));
}

function regexLoeschen(event) {
    
    let key_id = event.target.getAttribute("data-key-id");
    if (!key_id) { 
        return; 
    }
    
    let regex_key_dom = document.getElementById(key_id);
    if (!regex_key_dom) { 
        return; 
    }
    
    let regex_key = regex_key_dom.innerText;
    if (!regex_key) { 
        return; 
    }
    
    rpc.regex_loeschen(regex_key);
}

function neueRegexOnEnter(event) {
    if(event.keyCode === 13){
        
        event.preventDefault();
        
        let key_id = event.target.getAttribute("data-key-id");
        if (!key_id) { 
            return; 
        }
        
        let regex_key_dom = document.getElementById(key_id);
        if (!regex_key_dom) { 
            return; 
        }
        
        let regex_key = regex_key_dom.innerText;
        if (!regex_key) { 
            return; 
        }
        
        rpc.insert_regex(regex_key);
    }
}

function insertTabAtCaret(event){
   if(event.keyCode === 9){
       event.preventDefault();
       if (event.shiftKey) {
            event.target.dispatchEvent(new KeyboardEvent('keypress',{'key':'Backspace'}));
            event.target.dispatchEvent(new KeyboardEvent('keypress',{'key':'Backspace'}));
            event.target.dispatchEvent(new KeyboardEvent('keypress',{'key':'Backspace'}));
            event.target.dispatchEvent(new KeyboardEvent('keypress',{'key':'Backspace'}));
       } else {
            var editor = event.target;
            var doc = editor.ownerDocument.defaultView;
            var sel = doc.getSelection();
            var range = sel.getRangeAt(0);

            var tabNode = document.createTextNode("\u00a0\u00a0\u00a0\u00a0");
            range.insertNode(tabNode);

            range.setStartAfter(tabNode);
            range.setEndAfter(tabNode); 
            sel.removeAllRanges();
            sel.addRange(range);
            
       }        
    }
}

function editAbkuerzungenScript(e) {
    // using innerText here because it preserves newlines
    var innerText = e.target.innerText;
    if(innerText[innerText.length-1] === '\n') {
        innerText = innerText.slice(0,-1);     
    }
    
    if (innerText) {
        rpc.edit_abkuerzungen_script(innerText);        
    }

}


function editTextSaubernScript(e) {
    // using innerText here because it preserves newlines
    var innerText = e.target.innerText;
    if(innerText[innerText.length-1] === '\n') {
        innerText = innerText.slice(0,-1);     
    }
    
    if (innerText) {
        rpc.edit_text_saubern_script(innerText);        
    }

}

function editTextKuerzenAbt2Script(e) {
    // using innerText here because it preserves newlines
    var innerText = e.target.innerText;
    if(innerText[innerText.length-1] === '\n') {
        innerText = innerText.slice(0,-1);     
    }
    
    if (innerText) {
        rpc.edit_text_kuerzen_abt2_script(innerText);        
    }

}

function textKuerzenAbt2ScriptTesten(e) {
    if (e.target.value) {
        rpc.kurztext_abt2_script_testen(e.target.value);        
    }
}

function replaceTextKuerzenAbt2TestOutput(s) {
    let test_input = document.getElementById("__application_konfiguration_text_kuerzen_abt2_test");
    if (test_input)
         test_input.value = s;
}

function editRechteArtScript(e) {
    // using innerText here because it preserves newlines
    var innerText = e.target.innerText;
    if(innerText[innerText.length-1] === '\n') {
        innerText = innerText.slice(0,-1);     
    }
    
    if (innerText) {
        rpc.edit_rechteart_script(innerText);        
    }

}

function editRangvermerkAuslesenAbt2Script(e) {
    // using innerText here because it preserves newlines
    var innerText = e.target.innerText;
    if(innerText[innerText.length-1] === '\n') {
        innerText = innerText.slice(0,-1);     
    }
    
    if (innerText) {
        rpc.edit_rangvermerk_auslesen_abt2_script(innerText);        
    }

}

function rangvermerkAuslesenAbt2ScriptTesten(e) {
    if (e.target.value) {
        rpc.rangvermerk_auslesen_abt2_script_testen(e.target.value);        
    }
}

function replaceRangvermerkAuslesenAbt2TestOutput(s) {
    let test_input = document.getElementById("__application_konfiguration_rangvermerk_auslesen_abt2_test");
    if (test_input)
         test_input.value = s;
}

function testeRegex(event) {
    let regex_id = document.getElementById("__application_konfiguration_regex_id");
    if (!regex_id) {
        return;
    }
    
    let regex_id_value = regex_id.value;
    if (!regex_id_value) {
        return;
    }

    let test_text = event.target.value;
    if (!test_text) {
        return;
    }
    
    rpc.teste_regex(regex_id_value, test_text);
}

function editRegexKey(event) {
    let new_key = event.target.innerText;
    if (!new_key) { 
        return;
    }
    let old_key = event.target.getAttribute("data-regex-key");
    if (!old_key) { 
        return; 
    }
    event.target.setAttribute("data-regex-key", new_key);
    rpc.edit_regex_key(old_key, new_key);
}

function editRegexValue(event) {
    let regex_value = event.target.innerText;
    if (!regex_value) { 
        return; 
    }
    let key_id = event.target.getAttribute("data-key-id");
    if (!key_id) { 
        return; 
    }
    let regex_key_dom = document.getElementById(key_id);
    if (!regex_key_dom) { 
        return; 
    }
    let regex_key = regex_key_dom.innerText;
    if (!regex_key) { 
        return; 
    }
    rpc.edit_regex_value(regex_key, regex_value);
    
    let regex_id = document.getElementById("__application_konfiguration_regex_id");
    if (!regex_id) {
        return;
    }
    
    let regex_id_value = regex_id.value;
    if (!regex_id_value) {
        return;
    }
    
    let test_text_dom = document.getElementById("__application_konfiguration_regex_test_text");
    if (!test_text_dom) {
        return;
    }
    
    let test_text = test_text_dom.value;
    if (!test_text) {
        return;
    }
    
    rpc.teste_regex(regex_id_value, test_text);
}

function replaceRegexTestOutput(s) {
    let test_input = document.getElementById("__application_konfiguration_regex_test_output");
    if (test_input)
         test_input.value = s;
}

function rechteArtScriptTesten(e) {
    if (e.target.value) {
        rpc.rechteart_script_testen(e.target.value);        
    }
}

function replaceRechteArtTestOutput(s) {
    let test_input = document.getElementById("__application_konfiguration_rechteart_test");
    if (test_input)
         test_input.value = s;
}

function editRechtsinhaberAbt2Script(e) {
    // using innerText here because it preserves newlines
    var innerText = e.target.innerText;
    if(innerText[innerText.length-1] === '\n') {
        innerText = innerText.slice(0,-1);     
    }
    
    if (innerText) {
        rpc.edit_rechtsinhaber_auslesen_abt2_script(innerText);        
    }

}


function rechtsinhaberAbt2ScriptTesten(e) {
    if (e.target.value) {
        rpc.rechtsinhaber_auslesen_abt2_script_testen(e.target.value);        
    }
}

function replaceRechtsinhaberAbt2TestOutput(s) {
    let test_input = document.getElementById("__application_konfiguration_rechtsinhaber_abt2_test");
    if (test_input)
         test_input.value = s;
}

// ---


function editTextKuerzenAbt3Script(e) {
    // using innerText here because it preserves newlines
    var innerText = e.target.innerText;
    if(innerText[innerText.length-1] === '\n') {
        innerText = innerText.slice(0,-1);     
    }
    
    if (innerText) {
        rpc.edit_text_kuerzen_abt3_script(innerText);        
    }

}

function textKuerzenAbt3ScriptTesten(e) {
    if (e.target.value) {
        rpc.kurztext_abt3_script_testen(e.target.value);        
    }
}

function replaceTextKuerzenAbt3TestOutput(s) {
    let test_input = document.getElementById("__application_konfiguration_text_kuerzen_abt3_test");
    if (test_input)
         test_input.value = s;
}

function editBetragAuslesenScript(e) {
    // using innerText here because it preserves newlines
    var innerText = e.target.innerText;
    if(innerText[innerText.length-1] === '\n') {
        innerText = innerText.slice(0,-1);     
    }
    
    if (innerText) {
        rpc.edit_betrag_auslesen_script(innerText);        
    }

}

function betragAuslesenScriptTesten(e) {
    if (e.target.value) {
        rpc.betrag_auslesen_script_testen(e.target.value);        
    }
}

function replaceBetragAuslesenTestOutput(s) {
    let test_input = document.getElementById("__application_konfiguration_betrag_auslesen_test");
    if (test_input)
         test_input.value = s;
}

function editSchuldenArtScript(e) {
    // using innerText here because it preserves newlines
    var innerText = e.target.innerText;
    if(innerText[innerText.length-1] === '\n') {
        innerText = innerText.slice(0,-1);     
    }
    
    if (innerText) {
        rpc.edit_schuldenart_script(innerText);        
    }

}

function schuldenArtScriptTesten(e) {
    if (e.target.value) {
        rpc.schuldenart_script_testen(e.target.value);        
    }
}

function replaceSchuldenArtTestOutput(s) {
    let test_input = document.getElementById("__application_konfiguration_schuldenart_test");
    if (test_input)
         test_input.value = s;
}


function editRechtsinhaberAbt3Script(e) {
    // using innerText here because it preserves newlines
    var innerText = e.target.innerText;
    if(innerText[innerText.length-1] === '\n') {
        innerText = innerText.slice(0,-1);     
    }
    
    if (innerText) {
        rpc.edit_rechtsinhaber_auslesen_abt3_script(innerText);        
    }

}


function rechtsinhaberAbt3ScriptTesten(e) {
    if (e.target.value) {
        rpc.rechtsinhaber_auslesen_abt3_script_testen(e.target.value);        
    }
}

function replaceRechtsinhaberAbt3TestOutput(s) {
    let test_input = document.getElementById("__application_konfiguration_rechtsinhaber_abt3_test");
    if (test_input)
         test_input.value = s;
}


// ---

function klassifiziereSeiteNeu(event) {
            
    var klassifikation = event.target.getAttribute("data-seite-neu");
    if (!klassifikation) {
        return;
    }
    
    var seite = parseInt(event.target.getAttribute("data-seite"));
    if (!seite) {
        return;
    }
    
    rpc.klassifiziere_seite_neu(seite, klassifikation);
}

function closeFile(event, path) {
    var path = event.target.getAttribute("data-fileName");
    if (!path) {
        return;
    }
    
    rpc.close_file(path);
}

// Init
window.onload = function() { rpc.init(); };

/*
document.querySelectorAll('*').forEach(function(node) {
    node.addEventListener('contextmenu', e => e.preventDefault())
});
*/
