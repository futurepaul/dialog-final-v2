fn main() {
    // Ensure Cargo rebuilds when UDL changes
    println!("cargo:rerun-if-changed=src/dialog.udl");
    uniffi_build::generate_scaffolding("./src/dialog.udl").unwrap();
}
