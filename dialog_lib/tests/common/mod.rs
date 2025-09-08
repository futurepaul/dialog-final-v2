use dialog_lib::{clean_test_storage, Dialog};
use nostr_sdk::prelude::*;
use std::process::{Child, Command};
use std::time::Duration;

pub const TEST_RELAY_URL: &str = "ws://localhost:10548";
pub const TEST_NSEC: &str = "nsec1ufnus6pju578ste3v90xd5m2decpuzpql2295m3sknqcjzyys9ls0qlc85";

pub struct TestServer {
    process: Child,
    pubkey: String,
}

impl TestServer {
    pub async fn new() -> Self {
        // Kill any existing nak servers on our port
        let _ = Command::new("pkill")
            .args(&["-f", "nak.*serve.*10548"])
            .output();

        // Parse keys to get pubkey for cleanup
        let keys = Keys::parse(TEST_NSEC).unwrap();
        let pubkey = keys.public_key().to_hex();

        // Clean up any existing storage
        let _ = clean_test_storage(&pubkey);

        // Start nak server with negentropy support
        println!("Starting nak server with negentropy on port 10548...");
        let process = Command::new("./vendor/nak-negentropy")
            .args(&["serve", "--port", "10548"])
            .spawn()
            .expect("Failed to start nak server");

        // Give server time to start
        tokio::time::sleep(Duration::from_secs(2)).await;
        println!("Nak server with negentropy should be ready");

        Self { process, pubkey }
    }

    pub async fn create_dialog(&self) -> Dialog {
        Dialog::new(TEST_NSEC, TEST_RELAY_URL)
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
