use dialog_lib::clean_test_storage;
use nostr_sdk::prelude::*;
use portpicker::pick_unused_port;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::Duration;

pub struct TestServer {
    process: Child,
    pubkey: String,
    relay_url: String,
}

impl TestServer {
    pub fn new() -> Self {
        let port = pick_unused_port().expect("failed to allocate unused port");
        let relay_url = format!("ws://127.0.0.1:{port}");

        let pattern = format!("nak.*serve.*{port}");
        let _ = Command::new("pkill").args(["-f", &pattern]).output();

        let test_nsec = std::env::var("DIALOG_NSEC_TEST")
            .or_else(|_| std::env::var("DIALOG_NSEC"))
            .expect("Set DIALOG_NSEC_TEST or DIALOG_NSEC in CI/environment");
        let keys = Keys::parse(&test_nsec).expect("TEST_NSEC invalid");
        let pubkey = keys.public_key().to_hex();
        let _ = clean_test_storage(&pubkey);

        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let nak_path = repo_root.join("nak-negentropy");
        assert!(
            nak_path.exists(),
            "nak-negentropy not found at {}. Run ./setup_nak_local.sh at repo root.",
            nak_path.display()
        );
        println!(
            "Starting nak server with negentropy on port {port} using {} ...",
            nak_path.display()
        );
        let process = Command::new(nak_path)
            .args(["serve", "--port", &port.to_string()])
            .spawn()
            .expect("Failed to start nak server. Build it via ./setup_nak_local.sh");

        std::thread::sleep(Duration::from_secs(2));
        println!("Nak server with negentropy should be ready");

        Self {
            process,
            pubkey,
            relay_url,
        }
    }

    pub fn relay_url(&self) -> &str {
        &self.relay_url
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
        let _ = clean_test_storage(&self.pubkey);
    }
}
