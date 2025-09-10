use dialog_lib::clean_test_storage;
use nostr_sdk::prelude::*;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::Duration;

pub const TEST_RELAY_URL: &str = "ws://localhost:10548";

pub struct TestServer {
    process: Child,
    pubkey: String,
}

impl TestServer {
    pub fn new() -> Self {
        // Kill any existing nak servers on our port
        let _ = Command::new("pkill").args(["-f", "nak.*serve.*10548"]).output();

        // Clean any prior test storage for the test pubkey
        let test_nsec = std::env::var("DIALOG_NSEC_TEST").or_else(|_| std::env::var("DIALOG_NSEC")).expect("Set DIALOG_NSEC_TEST or DIALOG_NSEC in CI/environment");
        let keys = Keys::parse(&test_nsec).expect("TEST_NSEC invalid");
        let pubkey = keys.public_key().to_hex();
        let _ = clean_test_storage(&pubkey);

        // Start patched nak from repo root (../nak-negentropy relative to this crate)
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
        let nak_path = repo_root.join("nak-negentropy");
        assert!(nak_path.exists(), "nak-negentropy not found at {}. Run ./setup_nak_local.sh at repo root.", nak_path.display());
        println!("Starting nak server with negentropy on port 10548 using {} ...", nak_path.display());
        let process = Command::new(nak_path)
            .args(["serve", "--port", "10548"])
            .spawn()
            .expect("Failed to start nak server. Build it via ./setup_nak_local.sh");

        // Give server time to start
        std::thread::sleep(Duration::from_secs(2));
        println!("Nak server with negentropy should be ready");

        Self { process, pubkey }
    }
}

impl Default for TestServer {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
        // Clean test storage for pubkey
        let _ = clean_test_storage(&self.pubkey);
    }
}
