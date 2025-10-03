use dialog_lib::{clean_test_storage, Dialog};
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
    pub async fn new() -> Self {
        // Allocate a dedicated relay port to avoid conflicts with developer instances
        let port = pick_unused_port().expect("Failed to allocate an unused port");
        let relay_url = format!("ws://127.0.0.1:{port}");

        // Kill any lingering nak servers on our allocated port
        let port_pattern = format!("nak.*serve.*{port}");
        let _ = Command::new("pkill").args(["-f", &port_pattern]).output();

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
            "Starting nak server with negentropy on port {port} using {}...",
            nak_path.display()
        );
        let port_str = port.to_string();
        let process = Command::new(nak_path)
            .args(["serve", "--port", &port_str])
            .spawn()
            .expect("Failed to start nak server");

        // Give server time to start
        tokio::time::sleep(Duration::from_secs(2)).await;
        println!("Nak server with negentropy should be ready");

        Self {
            process,
            pubkey,
            relay_url,
        }
    }

    pub async fn create_dialog(&self) -> Dialog {
        let test_nsec = std::env::var("DIALOG_NSEC_TEST")
            .or_else(|_| std::env::var("DIALOG_NSEC"))
            .expect("Set DIALOG_NSEC_TEST or DIALOG_NSEC in CI/environment");
        Dialog::new_with_relay(&test_nsec, &self.relay_url)
            .await
            .expect("Failed to create Dialog")
    }

    pub fn clear_storage(&self) {
        let _ = clean_test_storage(&self.pubkey);
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
