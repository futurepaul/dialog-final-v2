use dialog_lib::{clean_test_storage, Dialog};
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
    pub async fn new() -> Self {
        // Kill any existing nak servers on our port
        let _ = Command::new("pkill")
            .args(["-f", "nak.*serve.*10548"])
            .output();

        // Parse keys to get pubkey for cleanup
        let test_nsec = std::env::var("DIALOG_NSEC_TEST")
            .or_else(|_| std::env::var("DIALOG_NSEC"))
            .expect("Set DIALOG_NSEC_TEST or DIALOG_NSEC in CI/environment");
        let keys = Keys::parse(&test_nsec).unwrap();
        let pubkey = keys.public_key().to_hex();

        // Clean up any existing storage
        let _ = clean_test_storage(&pubkey);

        // Start nak server with negentropy support
        // Resolve nak binary at repository root (parent of this crate)
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
            "Starting nak server with negentropy on port 10548 using {}...",
            nak_path.display()
        );
        let process = Command::new(nak_path)
            .args(["serve", "--port", "10548"])
            .spawn()
            .expect("Failed to start nak server");

        // Give server time to start
        tokio::time::sleep(Duration::from_secs(2)).await;
        println!("Nak server with negentropy should be ready");

        Self { process, pubkey }
    }

    pub async fn create_dialog(&self) -> Dialog {
        let test_nsec = std::env::var("DIALOG_NSEC_TEST")
            .or_else(|_| std::env::var("DIALOG_NSEC"))
            .expect("Set DIALOG_NSEC_TEST or DIALOG_NSEC in CI/environment");
        Dialog::new_with_relay(&test_nsec, TEST_RELAY_URL)
            .await
            .expect("Failed to create Dialog")
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        // Kill the server
        let _ = self.process.kill();
        let _ = self.process.wait();

        // Clean up storage
        let _ = clean_test_storage(&self.pubkey);
    }
}
