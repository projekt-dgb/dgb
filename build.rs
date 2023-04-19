use std::fs::File;
use std::io::Write;

const BODY_CSS: &'static str = r#"
    * {
        padding: 0px;
        margin: 0px;
        font-size: 14px;

        cursor: default;
        border: none;
        outline: none;
        color: #222222;
        box-sizing: border-box;
    }

    body, html {
    display: flex;
    flex-direction: column;
    flex-grow: 1;
    height: 100%;
    }

    body, body > * {
    display: flex;
    flex-direction: row;
    margin: 0px;
    }

    body {
    background: white;
    max-height: 100%;
    overflow: hidden;
    flex-direction: column;
    }

    html {
    height: 100%;
    max-height: 100%;
    overflow: hidden;
    }

    textarea {
        resize: none;
    }

    script {
        display: none;
    }

    /* Chrome, Safari, Edge, Opera */
    input::-webkit-outer-spin-button,
    input::-webkit-inner-spin-button {
    -webkit-appearance: none;
    margin: 0;
    }

    /* Firefox */
    input[type=number] {
    -moz-appearance: textfield;
    }

    input[type="file"] {
        display: none;
    }
"#;

const RIBBON_CSS: &'static str = r#"

    /* RIBBON BODY */
    .__application-ribbon-body {
        height: 90px;
        display: flex;
        overflow:hidden;
        flex-direction: row;
        border-bottom: 1px solid #D5D5D5;
        font-family: sans-serif;
        padding: 2px;
        font-size: 12px;
        user-select:none;
        -webkit-user-select:none;
    }

    .__application-ribbon-section {
        padding: 0px 2px;
        border-right: 1px solid #E1E1E1;
        display: flex;
        flex-direction: column;
    }

    .__application-ribbon-section-content {
        flex-grow: 1;
    }

    .__application-ribbon-section-name {
        font-size: 11px;
        color: #444444;
        text-align: center;
        min-width: 80px;
    }

    /* RIBBON BUTTONS*/
    .__application-ribbon-action-vertical-large {
        flex-direction: column;
        padding: 4px;
        margin: 4px;
        display: flex;
        background: white;
        cursor: pointer;
        border-radius: 2px;
    }

    .__application-ribbon-action-vertical-large:hover {
        box-shadow: 0px 0px 10px #ccc;
        cursor: pointer;
    }

    .__application-ribbon-action-vertical-large .icon-wrapper {
        flex-direction: row;
        align-items: center;
        justify-content: center;
        align-content: center;
        font-size: 12px;
        display: flex;
        cursor: pointer;
    }

    .__application-ribbon-action-vertical-large .icon-wrapper img.icon {
        height: 32px;
        width: 32px;
        margin: 4px;
        cursor: pointer;
    }

    .__application-ribbon-action-vertical-large p {
        text-align: center;
        align-items: center;
        font-size: 12px;
        cursor: pointer;
    }

    .__application-ribbon-action-vertical-large .dropdown {
        flex-direction: row;
        align-items: center;
        justify-content: center;
        align-content: center;
        cursor: pointer;
    }

    .__application-ribbon-action-vertical-large .dropdown .icon {
        height: 5px;
        width: 5px;
        background: salmon;
        cursor: pointer;
    }
"#;

fn main() {
    
    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-lib=static:-bundle=c++"); // link libstdc++ for tesseract

    let mut main_css = include_str!("src/css/webkit-normalize.css").to_string();
    main_css.push_str(BODY_CSS);
    main_css.push_str(RIBBON_CSS);
    main_css.push_str(include_str!("src/css/view-grundbuch.css"));

    let main_script = include_str!("src/view.js")
        .replace(
            "// INJECT_PDFJS_WORKER_SCRIPT",
            include_str!("bin/pdfjs-3.0.279-legacy-dist/build/pdf.worker.js"),
        )
        .replace(
            "// INJECT_PDFJS_SCRIPT",
            include_str!("bin/pdfjs-3.0.279-legacy-dist/build/pdf.js"),
        );

    let main_html = format!(
        "
    <!doctype html>
    <html>
        <head>
            <meta charset=\"utf-8\" />
            <style type=\"text/css\">{main_css}</style>
            <script type=\"text/javascript\">{main_script}</script>
        </head>
        <body></body>
    </html>
    "
    );

    let mut file = File::create("src/app.html").unwrap();
    file.write_all(main_html.as_bytes()).unwrap();
}
