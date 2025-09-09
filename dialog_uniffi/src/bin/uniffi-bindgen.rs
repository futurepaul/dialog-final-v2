use std::env;
use camino::Utf8PathBuf;
use uniffi_bindgen::{generate_bindings, bindings::SwiftBindingGenerator};

fn main() {
    let args: Vec<String> = env::args().collect();
    
    // Parse simple arguments for our use case
    if args.len() < 6 || args[1] != "generate" {
        eprintln!("Usage: uniffi-bindgen generate --library <lib> --language swift --out-dir <dir> <udl>");
        std::process::exit(1);
    }
    
    let mut library_path = None;
    let mut out_dir = None;
    let mut udl_file = None;
    
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--library" => {
                library_path = Some(Utf8PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--language" => {
                // We only support Swift for now
                i += 2;
            }
            "--out-dir" => {
                out_dir = Some(Utf8PathBuf::from(&args[i + 1]));
                i += 2;
            }
            _ => {
                udl_file = Some(Utf8PathBuf::from(&args[i]));
                i += 1;
            }
        }
    }
    
    let library_path = library_path.expect("--library required");
    let out_dir = out_dir.expect("--out-dir required");
    let udl_file = udl_file.expect("UDL file required");
    
    // Generate Swift bindings
    let generator = SwiftBindingGenerator;
    generate_bindings(
        &udl_file,
        None,
        generator,
        Some(&out_dir),
        Some(&library_path),
        None,
        false,
    ).expect("Failed to generate bindings");
}