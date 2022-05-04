
"use strict";

let rpc = {
    
  invoke : function(arg) { window.webkit.messageHandlers.external.postMessage(JSON.stringify(arg)); },
  init: function() { rpc.invoke({ cmd: 'init' }); },
  
  load_pdf: function() { rpc.invoke({ cmd : 'load_pdf' }); },
  create_new_grundbuch: function() { rpc.invoke({ cmd : 'create_new_grundbuch' }); },
  grundbuch_anlegen: function(land, grundbuch_von, amtsgericht, blatt) { rpc.invoke({ cmd : 'grundbuch_anlegen', land: land, grundbuch_von: grundbuch_von, amtsgericht: amtsgericht, blatt: blatt }); },
  undo:  function() { rpc.invoke({ cmd : 'undo' }); },
  redo:  function() { rpc.invoke({ cmd : 'redo' }); },
  export_nb:  function() { rpc.invoke({ cmd : 'export_nb' }); },
  export_pdf: function() { rpc.invoke({ cmd : 'export_pdf' }); },
  grundbuch_exportieren: function(was_exportieren, exportiere_bv, exportiere_abt_1, exportiere_abt_2, exportiere_abt_3, exportiere_pdf_leere_seite, exportiere_geroetete_eintraege, exportiere_in_eine_einzelne_datei) {
      rpc.invoke({ 
          cmd : 'grundbuch_exportieren', 
          was_exportieren: was_exportieren,
          exportiere_bv: exportiere_bv,
          exportiere_abt_1: exportiere_abt_1,
          exportiere_abt_2: exportiere_abt_2,
          exportiere_abt_3: exportiere_abt_3,
          exportiere_pdf_leere_seite: exportiere_pdf_leere_seite,
          exportiere_geroetete_eintraege: exportiere_geroetete_eintraege,
          exportiere_in_eine_einzelne_datei: exportiere_in_eine_einzelne_datei,
    });
  },
  
  export_alle_rechte: function() { rpc.invoke({ cmd : 'export_alle_rechte' }); },
  export_alle_fehler: function() { rpc.invoke({ cmd : 'export_alle_fehler' }); },
  export_alle_abt1: function() { rpc.invoke({ cmd : 'export_alle_abt1' }); },
  export_alle_teilbelastungen: function() { rpc.invoke({ cmd : 'export_alle_teilbelastungen' }); },
  check_pdf_image_sichtbar: function() { rpc.invoke({ cmd : 'check_pdf_image_sichtbar' }); },
  toggle_lefis_analyse: function() { rpc.invoke({ cmd : 'toggle_lefis_analyse' }); },
  check_pdf_for_errors: function() { rpc.invoke({ cmd : 'check_pdf_for_errors' }); },

  open_grundbuch_suchen_dialog: function()  { rpc.invoke({ cmd : 'open_grundbuch_suchen_dialog' }); },
  search: function(search_text) { rpc.invoke({ cmd : 'search', search_text: search_text }); },
  open_grundbuch_upload_dialog: function()  { rpc.invoke({ cmd : 'open_grundbuch_upload_dialog' }); },

  import_nb:  function() { rpc.invoke({ cmd : 'import_nb' }); },
  export_lefis:  function() { rpc.invoke({ cmd : 'export_lefis' }); },
  delete_nb: function() { rpc.invoke({ cmd : 'delete_nb' }); },
  open_info: function() { rpc.invoke({ cmd : 'open_info' }); },
  open_help: function() { rpc.invoke({ cmd : 'open_help' }); },
  open_export_pdf: function() { rpc.invoke({ cmd : 'open_export_pdf' }); },
  open_configuration: function() { rpc.invoke({ cmd : 'open_configuration' }); },
  set_configuration_view: function(section_id) { rpc.invoke({ cmd : 'set_configuration_view', section_id: section_id }); },
  
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
  edit_flurstuecke_auslesen_script: function(arg) { rpc.invoke({ cmd : 'edit_flurstuecke_auslesen_script', script: arg });},

  edit_text_kuerzen_abt2_script: function(arg) { rpc.invoke({ cmd : 'edit_text_kuerzen_abt2_script', script: arg });},
  kurztext_abt2_script_testen: function(arg) { rpc.invoke({ cmd: 'kurztext_abt2_script_testen', text: arg }); },  
  flurstueck_auslesen_script_testen: function(arg, bv_nr) { rpc.invoke({ cmd: 'flurstueck_auslesen_script_testen', text: arg, bv_nr: bv_nr }); },  

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
  copy_text_to_clipboard: function(text) { rpc.invoke({ cmd: 'copy_text_to_clipboard', text: text }); },
  
  klassifiziere_seite_neu: function(seite, klassifikation_neu) { rpc.invoke({ cmd: 'klassifiziere_seite_neu', seite: seite, klassifikation_neu: klassifikation_neu }); },
  resize_column: function(direction, columnId, x, y) { rpc.invoke({ cmd: 'resize_column', direction: direction, column_id: columnId, x: x, y: y }); },
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
    create_new_grundbuch: function(event) { rpc.create_new_grundbuch(); },
    undo: function(event) { rpc.undo() },
    redo: function(event) { rpc.redo() },
    search_grundbuch: function(event) { rpc.open_grundbuch_suchen_dialog() },
    upload_grundbuch: function(event) { rpc.open_grundbuch_upload_dialog() },
    export_nb: function(event) { rpc.export_nb() },
    import_nb: function(event) { rpc.import_nb() },
    delete_nb: function(event) { rpc.delete_nb() },
    export_alle_rechte: function(event) { rpc.export_alle_rechte() },
    export_alle_fehler: function(event) { rpc.export_alle_fehler() },
    export_alle_teilbelastungen: function(event) { rpc.export_alle_teilbelastungen() },
    export_alle_abt1: function(event) { rpc.export_alle_abt1() },
    export_lefis: function(event) { rpc.export_lefis() },
    export_pdf: function(event) { rpc.export_pdf() },
    open_configuration: function(event) { rpc.open_configuration() },
    open_help: function(event) { rpc.open_help() },
    open_info: function(event) { rpc.open_info() },
    open_export_pdf: function(event) { rpc.open_export_pdf() },
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

setInterval(function(){
    if (!(document.getElementById("__application_page_img_inner"))) {
        rpc.check_pdf_image_sichtbar();
    }
}, 100);

function startCheckingForPdfErrors() {
    setInterval(function(){
        rpc.check_pdf_for_errors();
    }, 1278);
}

function toggleLefisAnalyse(event) {
    rpc.toggle_lefis_analyse();
}

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

function replaceMainContainer(s) {
    let ribbon = document.getElementById("__application-main-container");
    if (ribbon)
         ribbon.innerHTML = s;
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

function replaceMainNoFiles(s) {
    let e = document.getElementById("__application-main-no-files");
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

function replaceAbt1GrundlagenEintragungen(s) {
    let e = document.getElementById("__application-abteilung-1-grundlagen-eintragungen");
    if (e)
        e.innerHTML = s;    
}

function grundbuchSuchen(e) {
    let search_text = document.getElementById("__application_grundbuch_suchen_suchbegriff");
    if (!search_text) {
        return;
    }
    rpc.search(search_text.value);
}

function replaceSuchergebnisse(s) {
    let e = document.getElementById("__application_grundbuch_suchen_suchergebnisse");
    if (e)
        e.innerHTML = s;    
}

function grundbuchHerunterladen(e) {
    e.preventDefault();
    
    let download_id = e.target.dataset.downloadId;
    if (!download_id) {
        return false;
    }
    
    let file_name = e.target.dataset.fileName;
    if (!file_name) {
        return false;
    }
    
    return false;
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
    let e = document.getElementById("__application-analyse-grundbuch");
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

function replaceIcon(id, data) {

    let icon = document.getElementById("__application_file_icon-" + id);
    let icon_inner = document.getElementById("__application_file_icon-inner-" + id);

    if (!icon) {
        return;
    }
    
    if (icon_inner) {
        icon_inner.remove();
    }
    
    let new_icon = window.document.createElement('img');
    new_icon.id = "__application_file_icon-inner-" + id;
    new_icon.style = "width: 16px;height: 16px;margin-right:5px;flex-grow: 0;cursor: pointer;";
    new_icon.src = data;
    new_icon["data-fileName"] = id;
    
    icon.appendChild(new_icon);
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

function insertRegexFromButton(event) {

    let regex_key = event.target.getAttribute("data-regex-id");
    if (!regex_key) { 
        return; 
    }
    
    rpc.insert_regex(regex_key);
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

function editTextarea(event, id) {
    
    // using innerText here because it preserves newlines
    var innerText = event.target.innerText;
    if(innerText[innerText.length-1] === '\n') {
        innerText = innerText.slice(0,-1);     
    }
    
    if (innerText) {
        rpc.edit_textarea(innerText, id);        
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

function editFlurstueckeAuslesenScript(e) {
    // using innerText here because it preserves newlines
    var innerText = e.target.innerText;
    if(innerText[innerText.length-1] === '\n') {
        innerText = innerText.slice(0,-1);     
    }
    
    if (innerText) {
        rpc.edit_flurstuecke_auslesen_script(innerText);        
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

function flurstueckAuslesenScriptTesten(e) {

    let bv_input = document.getElementById("__application_konfiguration_flurstueck_auslesen_bv_nr");
    
    if (!bv_input)
        return;
    
    if (!bv_input.value)
        return;
    
    if (e.target.value) {
        rpc.flurstueck_auslesen_script_testen(e.target.value, bv_input.value);        
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

function replaceFlurstueckAuslesenTestOutput(s) {
    let test_input = document.getElementById("__application_konfiguration_flurstueck_auslesen_test");
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

function resizeColumnOnMouseDown(event) {
    
    if (!event.which) {
        return;
    }
    
    if (!(event.which == 1)) {
        return;
    }
    
    event.target.style.width = '100px';
    event.target.style.height = '100px';

    var direction = event.target.getAttribute("data-direction");
    if (!direction) {
        return;
    }
    
    if (direction == 'nw') {
        event.target.style.top = '-50px';
        event.target.style.left = '-50px';
    } else if (direction == 'ne') {
        event.target.style.top = '-50px';
        event.target.style.right = '-50px';
    } else if (direction == 'se') {
        event.target.style.bottom = '-50px';
        event.target.style.right = '-50px';
    } else if (direction == 'sw') {
        event.target.style.bottom = '-50px';
        event.target.style.left = '-50px';
    }
}

function resizeColumnOnMouseUp(event) {

    if (!event.which) {
        return;
    }
    
    if (!(event.which == 1)) {
        return;
    }
    
    event.target.style.width = '15px';
    event.target.style.height = '15px';

    var direction = event.target.getAttribute("data-direction");
    if (!direction) {
        return;
    }
    
    if (direction == 'nw') {
        event.target.style.top = '-7.5px';
        event.target.style.left = '-7.5px';
    } else if (direction == 'ne') {
        event.target.style.top = '-7.5px';
        event.target.style.right = '-7.5px';
    } else if (direction == 'se') {
        event.target.style.bottom = '-7.5px';
        event.target.style.right = '-7.5px';
    } else if (direction == 'sw') {
        event.target.style.bottom = '-7.5px';
        event.target.style.left = '-7.5px';
    }
    
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
    
    let bounds = parent.getBoundingClientRect();
    let x = event.clientX - bounds.left;
    let y = event.clientY - bounds.top;

    rpc.resize_column(direction, columnId, x, y);
    
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


function replacePdfImage(s) {
    let img = document.getElementById("__application-pdf-page-image");
    if (!img)
        return;

    img.innerHTML = s;
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

function copyToClipboardOnSelectChange(event) {
    if (event.target.value) {
    	rpc.copy_text_to_clipboard("" + event.target.value);
    }
}

function editStringOrLines(event, inputId) {
    
    // using innerText here because it preserves newlines
    var innerText = event.target.innerText;
    if (!innerText) {
        return;
    }
    
    if(innerText[innerText.length-1] === '\n') {
        innerText = innerText.slice(0,-1);     
    }

    rpc.edit_text(inputId, innerText);    
}
function activateConfigurationView(event, section_id) {
    event.stopPropagation();
    event.preventDefault();
    rpc.set_configuration_view(section_id);
}

function grundbuchAnlegen(event) {
    event.preventDefault();
    
    var land = document.getElementById("__application_grundbuch_anlegen_land");
    if (!land)
        return;
    
    var amtsgericht = document.getElementById("__application_grundbuch_anlegen_amtsgericht");
    if (!amtsgericht)
        return;
    
    var grundbuch_von = document.getElementById("__application_grundbuch_anlegen_grundbuch_von");
    if (!grundbuch_von)
        return;
    
    var blatt = document.getElementById("__application_grundbuch_anlegen_blatt_nr");
    if (!blatt)
        return;
    
    rpc.grundbuch_anlegen(land.value, grundbuch_von.value, amtsgericht.value, parseInt(blatt.value));
    
    return false;
}

function grundbuchExportieren(event) {
    event.preventDefault();
        
    var was_exportieren = document.getElementById("__application_export-pdf-was-exportieren");
    if (!was_exportieren)
        return;
    
    var exportiere_bv = document.getElementById("export-pdf-bv");
    if (!exportiere_bv)
        return;
    
    var exportiere_abt_1 = document.getElementById("export-pdf-abt-1");
    if (!exportiere_abt_1)
        return;
        
    var exportiere_abt_2 = document.getElementById("export-pdf-abt-2");
    if (!exportiere_abt_2)
        return;
    
    var exportiere_abt_3 = document.getElementById("export-pdf-abt-3");
    if (!exportiere_abt_3)
        return;
    
    var exportiere_pdf_leere_seite = document.getElementById("export-pdf-leere-seite");
    if (!exportiere_pdf_leere_seite)
        return;
    
    var exportiere_geroetete_eintraege = document.getElementById("export-pdf-geroetete-eintraege");
    if (!exportiere_geroetete_eintraege)
        return;
        
    var exportiere_in_eine_einzelne_datei = document.getElementById("export-pdf-eine-datei");
    if (!exportiere_in_eine_einzelne_datei)
        return;
        
    rpc.grundbuch_exportieren(
        was_exportieren.value,
        exportiere_bv.checked,
        exportiere_abt_1.checked,
        exportiere_abt_2.checked,
        exportiere_abt_3.checked,
        exportiere_pdf_leere_seite.checked,
        exportiere_geroetete_eintraege.checked,
        exportiere_in_eine_einzelne_datei.checked,
    );
    
    return false;
}


// Init
window.onload = function() { rpc.init(); };
/*
document.querySelectorAll('*').forEach(function(node) {
    node.addEventListener('contextmenu', e => e.preventDefault())
});*/

