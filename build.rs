use std::fs::File;
use std::io::Write;

fn main() {
    let main_html = include_str!("src/html/main.html");
    let main_css = concat!(
        include_str!("src/css/webkit-normalize.css"),
    	include_str!("src/css/body.css"),
    	include_str!("src/css/ribbon.css"),
    	include_str!("src/css/view-grundbuch.css"),
    );
    let main_script = include_str!("src/view.js");
    let main_html = main_html.replace("{{styles}}", &format!("<style type=\"text/css\">{}</style>", main_css));
    let main_html = main_html.replace("{{scripts}}", &format!("<script type=\"text/javascript\">{}</script>", main_script));
    
    let mut file = File::create("src/dist/app.html").unwrap();
    file.write_all(main_html.as_bytes()).unwrap();
}
