
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
  reset_ocr_selection: function() { rpc.invoke({ cmd : 'reset_ocr_selection' }); },
  select_ocr: function(file_name, page, min_x, min_y, max_x, max_y, page_width, page_height) { rpc.invoke({ 
      cmd : 'select_ocr', 
      file_name: file_name, 
      page: page, 
      min_x: min_x, 
      min_y: min_y, 
      max_x: max_x, 
      max_y: max_y,
      page_width: page_width,
      page_height: page_height
    });
  },

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
  bv_eintrag_typ_aendern: function(path, value) { rpc.invoke({ cmd: 'bv_eintrag_typ_aendern', path: path, value: value }); },

  klassifiziere_seite_neu: function(seite, klassifikation_neu) { rpc.invoke({ cmd: 'klassifiziere_seite_neu', seite: seite, klassifikation_neu: klassifikation_neu }); },
  resize_column: function(direction, columnId, number) { rpc.invoke({ cmd: 'resize_column', direction: direction, column_id: columnId, number: number }); },
  toggle_checkbox: function(checkbox_id) { rpc.invoke({ cmd: 'toggle_checkbox', checkbox_id: checkbox_id }); },
  reload_grundbuch: function() { rpc.invoke({ cmd: 'reload_grundbuch' }); },
  zeile_neu: function(file, page, y) { rpc.invoke({ cmd: 'zeile_neu', file: file, page: page, y: y }); },
  zeile_loeschen: function(file, page, zeilen_id) { rpc.invoke({ cmd: 'zeile_loeschen', file: file, page: page, zeilen_id: zeilen_id }); },
  
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

let images_to_load = {};

setInterval(function(){
    for (const [key, value] of Object.entries(images_to_load)) {
        rpc.check_for_image_loaded(key, value);
    }
}, 1000);

function startCheckingForImageLoaded(filepath, filename) {
    images_to_load[filepath] = filename;
}

function stopCheckingForImageLoaded(filename) {
    if (images_to_load.hasOwnProperty(filename)) {
        delete images_to_load[filename];
    }
}

let ocr_selection_rect = null;

function onOcrSelectionDragStart(event) {
        
    let selection_rect = document.getElementById("__application_ocr_selection");
    if (!selection_rect)
        return;
        
    let parent = selection_rect.parentElement;
    if (!parent)
        return;
    
    let parent_width = parent.clientWidth;
    let parent_height = parent.clientHeight;

    let bounds = parent.getBoundingClientRect();
    let x = event.clientX - bounds.left;
    let y = event.clientY - bounds.top;
    
    var file = parent.getAttribute("data-fileName");
    if (!file) {
        return;
    }
    
    var page = Number(parent.getAttribute("data-pageNumber"));
    if (!page) {
        return;
    }
        
    if (!ocr_selection_rect) {
        ocr_selection_rect = { 
            page_width: parent_width, 
            page_height: parent_height,
            min_x: x,
            min_y: y,
            max_x: x,
            max_y: y,
            file: file,
            page: page,        
        };
    }
}

function onOcrSelectionDrag(event) {
        
    let selection_rect = document.getElementById("__application_ocr_selection");
    if (!selection_rect)
        return;
        
    let parent = selection_rect.parentElement;
    if (!parent)
        return;
    
    if (!ocr_selection_rect) {
        return;
    }

    let parent_width = parent.clientWidth;
    let parent_height = parent.clientHeight;

    let bounds = parent.getBoundingClientRect();
    let x = event.clientX - bounds.left;
    let y = event.clientY - bounds.top;
    
    var file = parent.getAttribute("data-fileName");
    if (!file) {
        return;
    }
    
    var page = Number(parent.getAttribute("data-pageNumber"));
    if (!page) {
        return;
    }
    
    if (ocr_selection_rect.file != file || ocr_selection_rect.page != page) {
        return;
    }
    
    ocr_selection_rect.max_x = x;
    ocr_selection_rect.max_y = y;
        
    let selection_width = ocr_selection_rect.max_x - ocr_selection_rect.min_x;
    let selection_height = ocr_selection_rect.max_y - ocr_selection_rect.min_y;

    selection_rect.style.background = "#BFEA93";
    selection_rect.style.transform = "translate(" + ocr_selection_rect.min_x +  "px, " + ocr_selection_rect.min_y + "px)";
    selection_rect.style.transform += "scale(" + selection_width + ", " + selection_height + ")";    
}

function onOcrSelectionDragStop(event) {
            
    if (!ocr_selection_rect)
        return;
    
    rpc.select_ocr(
        ocr_selection_rect.file,
        ocr_selection_rect.page,
        ocr_selection_rect.min_x,
        ocr_selection_rect.min_y,
        ocr_selection_rect.max_x,
        ocr_selection_rect.max_y,
        ocr_selection_rect.page_width,
        ocr_selection_rect.page_height
    );
}

function resetOcrSelection() {
        
    let selection_rect = document.getElementById("__application_ocr_selection");
    if (!selection_rect)
        return;
    
    selection_rect.style.background = "transparent";
    selection_rect.style.transform = "translate(0px, 0px)";
    selection_rect.style.transform += "scale(1.0, 1.0)";
    ocr_selection_rect = null;
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
    // console.error(error);
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

function replaceBestandsverzeichnis(s) {
    let e = document.getElementById("__application-bestandsverzeichnis");
    if (e)
        e.innerHTML = s;
}

function replaceBestandsverzeichnisZuschreibungen(s) {
    let e = document.getElementById("__application-bestandsverzeichnis-veraenderungen");
    if (e)
        e.innerHTML = s;
}

function replaceBestandsverzeichnisAbschreibungen(s) {
    let e = document.getElementById("__application-bestandsverzeichnis-loeschungen");
    if (e)
        e.innerHTML = s;
}

function replaceAbt1(s) {
    let e = document.getElementById("__application-abteilung-1");
    if (e)
        e.innerHTML = s;
}

function replaceAbt1Veraenderungen(s) {
    let e = document.getElementById("__application-abteilung-1-veraenderungen");
    if (e)
        e.innerHTML = s;
}

function replaceAbt1Loeschungen(s) {
    let e = document.getElementById("__application-abteilung-1-loeschungen");
    if (e)
        e.innerHTML = s;
}

function replaceAbt2(s) {
    let e = document.getElementById("__application-abteilung-2");
    if (e)
        e.innerHTML = s;
}

function replaceAbt2Veraenderungen(s) {
    let e = document.getElementById("__application-abteilung-2-veraenderungen");
    if (e)
        e.innerHTML = s;
}

function replaceAbt2Loeschungen(s) {
    let e = document.getElementById("__application-abteilung-2-loeschungen");
    if (e)
        e.innerHTML = s;
}

function replaceAbt3(s) {
    let e = document.getElementById("__application-abteilung-3");
    if (e)
        e.innerHTML = s;
}

function replaceAbt3Veraenderungen(s) {
    let e = document.getElementById("__application-abteilung-3-veraenderungen");
    if (e)
        e.innerHTML = s;
}

function replaceAbt3Loeschungen(s) {
    let e = document.getElementById("__application-abteilung-3-loeschungen");
    if (e)
        e.innerHTML = s;
}

function replaceAnalyseGrundbuch(s) {
    let e = document.getElementById("__application-");
    if (e)
        e.innerHTML = s;
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
    rpc.open_context_menu(e.clientX, e.clientY, Number(pn));
    return false;
}

function replacePopOver(s) {
    let page_list = document.getElementById("__application_popover");
    if (page_list)
         page_list.innerHTML = s;
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
    rpc.set_open_page(Number(pn, 10));
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
    
    var seite = Number(event.target.getAttribute("data-seite"));
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

function fallbackCopyTextToClipboard(text) {
  var textArea = document.createElement("textarea");
  textArea.value = text;
  
  // Avoid scrolling to bottom
  textArea.style.top = "0";
  textArea.style.left = "0";
  textArea.style.position = "fixed";

  document.body.appendChild(textArea);
  textArea.focus();
  textArea.select();

  try {
    var successful = document.execCommand('copy');
    var msg = successful ? 'successful' : 'unsuccessful';
  } catch (err) {
  }

  document.body.removeChild(textArea);
}

function copyTextToClipboard(text) {
  if (!navigator.clipboard) {
    fallbackCopyTextToClipboard(text);
    return;
  }
  navigator.clipboard.writeText(text).then(function() {
  }, function(err) {
  });
}

var last_mouse_down_x = null;
var last_mouse_down_y = null;
var dx = null;
var dy = null;

function resizeColumnOnMouseDown(event) {
    last_mouse_down_x = event.clientX;
    last_mouse_down_y = event.clientY;
    
    let target_rect = event.target.getBoundingClientRect();
    let target_center_x = target_rect.x + (target_rect.width / 2.0);
    let target_center_y = target_rect.y + (target_rect.height / 2.0);
    
    dx = event.clientX - target_center_x;
    dy = event.clientY - target_center_y;
}

function resizeColumnOnMouseUp(event) {
    last_mouse_down_x = null;
    last_mouse_down_y = null;
    dx = null;
    dy = null;
}

function resizeColumn(event) {    
    
    if (!event.which) {
        return;
    }
    
    if (!(event.which == 1 || event.which == 3)) {
        return;
    }
    
    var direction = event.target.getAttribute("data-direction");
    if (!direction) {
        return;
    }
    
    var columnId = event.target.getAttribute("data-columnId");
    if (!columnId) {
        return;
    }
    
    let parent = document.getElementById("__application_page_img_inner");
    if (!parent)
        return;

    let ddx = 0.0;
    if (dx) {
        ddx = dx;        
    }
    
    let ddy = 0.0;
    if (dy) {
        ddy = dy;        
    }
    
    let bounds = parent.getBoundingClientRect();
    let x = event.clientX - bounds.left - ddx;
    let y = event.clientY - bounds.top - ddy;
    
    let number = x;
    if ((direction === "n") || (direction === "s")) {
        number = y;
    }
    
    rpc.resize_column(direction, columnId, number);
    
    return;
}

function adjustColumn(columnId, width, height, x, y) {
    
    let column = document.getElementById("__application_spalte_" + columnId);
    if (!column)
        return;
    
    column.style.transform = "translate(" + x +  "px, " + y + "px)";
    column.style.width = width + "px";
    column.style.height = height + "px";    
}

function toggleCheckbox(event) {
    var checkbox_id = event.target.getAttribute("data-checkBoxId");
    if (!checkbox_id) {
        return;
    }
    
    rpc.toggle_checkbox(checkbox_id);
}

function reloadGrundbuch(event) {
    rpc.reload_grundbuch();
}

function zeileNeu(event) {
        
    if (!event.which) {
        return;
    }
    
    if (event.which !== 1) {
        return;
    }
    
    let zeilen_container = document.getElementById("__application_page_lines");
    if (!zeilen_container)
        return;
    
    let bounds = zeilen_container.getBoundingClientRect();
    let y = event.clientY - bounds.top;
    
    var file = zeilen_container.getAttribute("data-fileName");
    if (!file) {
        return;
    }
    
    var page = Number(zeilen_container.getAttribute("data-pageNumber"));
    if (!page) {
        return;
    }
    
    rpc.zeile_neu(file, page, y);
}

function zeileLoeschen(event) {
    
    if (!event.which) {
        return;
    }
    
    event.stopPropagation();
    
    if (event.which !== 3) {
        return;
    }
    
    let zeilen_container = document.getElementById("__application_page_lines");
    if (!zeilen_container)
        return;
    
    var file = zeilen_container.getAttribute("data-fileName");
    if (!file) {
        return;
    }
    
    var page = Number(zeilen_container.getAttribute("data-pageNumber"));
    if (!page) {
        return;
    }
        
    var zeileIdString = event.target.getAttribute("data-zeileId");
    if (!zeileIdString) {
        return;
    }
    var zeileId = 0;
    if (zeileIdString !== "0") {
        zeileId = Number(zeileIdString);
    }
    
    rpc.zeile_loeschen(file, page, zeileId);
}

function zeilePreviewShow(event) {
    let zeile = document.getElementById("__application_zeile_preview");
    if (!zeile)
        return;
    zeile.style.opacity = "0.5";
}

function zeilePreviewHide(event) {
    let zeile = document.getElementById("__application_zeile_preview");
    if (!zeile)
        return;
    zeile.style.opacity = "0";
}

function zeilePreviewMove(event) {
    
    let zeilen_container = document.getElementById("__application_page_lines");
    if (!zeilen_container)
        return;
    
    let bounds = zeilen_container.getBoundingClientRect();
    let y = event.clientY - bounds.top;
    
    let zeile = document.getElementById("__application_zeile_preview");
    if (!zeile)
        return;
    zeile.style.opacity = "0.5";
    zeile.style.transform = "translateY(" + (y - 10.0) + "px)";
}

function replacePdfImageZeilen(s) {
    let zeilen = document.getElementById("__application_zeilen");
    if (!zeilen)
        return;

    zeilen.innerHTML = s;
}

function bvEintragTypAendern(path, value) {
    rpc.bv_eintrag_typ_aendern(path, value);
}

// Init
window.onload = function() { rpc.init(); };

document.querySelectorAll('*').forEach(function(node) {
    node.addEventListener('contextmenu', e => e.preventDefault())
});
