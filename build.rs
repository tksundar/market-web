use std::fs;
use std::path::Path;

fn main() {
    let out_dir = "target/release";
    let dest_path = Path::new(&out_dir).join("static");
    fs::create_dir_all(&dest_path).unwrap();
    fs::copy("static/index.html", dest_path.join("index.html")).unwrap();
    fs::copy("static/main.html", dest_path.join("main.html")).unwrap();
    fs::copy("static/file_upload.html", dest_path.join("file_upload.html")).unwrap();
    fs::copy("static/market_simulator.html", dest_path.join("market_simulator.html")).unwrap();
}
