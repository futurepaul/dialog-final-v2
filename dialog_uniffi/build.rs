fn main() {
    // Ensure Cargo rebuilds when UDL changes
    println!("cargo:rerun-if-changed=src/dialog.udl");
    println!("cargo:rerun-if-changed=../ios/DialogPackage/.xcframework-version");

    if let Ok(version) = std::fs::read_to_string("../ios/DialogPackage/.xcframework-version") {
        let trimmed = version.trim();
        if !trimmed.is_empty() {
            println!("cargo:rustc-env=UNIFFI_FRAMEWORK_VERSION={}", trimmed);
        }
    }

    uniffi_build::generate_scaffolding("./src/dialog.udl").unwrap();
}
