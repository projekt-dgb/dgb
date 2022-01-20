
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
  edit_rechteart_script: function(neu) { rpc.invoke({ cmd: 'edit_rechteart_script', neu: neu }); },
  kurztext_testen: function(arg) { rpc.invoke({ cmd: 'kurztext_testen', text: arg }); },
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
    // console.error(info);
    alert(error);    
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
    rpc.open_context_menu(e.clientX, e.clientY, parseInt(e.target.getAttribute("data-pageNumber")));
    return false;
}

function closePopOver(s) {
    rpc.close_pop_over();
}

function activateSelectedFile(event) {
    rpc.set_open_file(event.target.getAttribute("data-fileName"));
}

function activateSelectedPage(event) {
    rpc.set_open_page(parseInt(event.target.getAttribute("data-pageNumber"), 10));
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

function kurztextTesten(e) {
    if (e.target.value) {
        rpc.kurztext_testen(e.target.value);        
    }
}

function replaceKurzTextTestString(s) {
    let test_input = document.getElementById("__application_konfiguration_kurztext_test");
    if (test_input)
         test_input.value = s;
}


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

// Init
window.onload = function() { rpc.init(); };

document.querySelectorAll('*').forEach(function(node) {
    node.addEventListener('contextmenu', e => e.preventDefault())
});
