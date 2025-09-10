mod common;

use common::{TestServer, TEST_RELAY_URL};
use dialog_uniffi::{DialogClient, Event, Command, DialogListener};
use std::sync::{mpsc, Arc};
use std::time::Duration;

struct TestListener {
    tx: mpsc::Sender<Event>,
}

impl DialogListener for TestListener {
    fn on_event(&self, event: Event) {
        let _ = self.tx.send(event);
    }
}

#[test]
fn uniffi_end_to_end_note_flow() {
    // Start fresh relay
    let _server = TestServer::new();

    // Create client
    let test_nsec = std::env::var("DIALOG_NSEC_TEST")
        .or_else(|_| std::env::var("DIALOG_NSEC"))
        .expect("Set DIALOG_NSEC_TEST or DIALOG_NSEC in CI/environment");
    let client = Arc::new(DialogClient::new(test_nsec));

    // Wire listener
    let (tx, rx) = mpsc::channel();
    let listener = Box::new(TestListener { tx });
    let client_clone = client.clone();
    client_clone.start(listener);

    // Connect relay
    client.clone().send_command(Command::ConnectRelay {
        relay_url: TEST_RELAY_URL.to_string(),
    });

    // Wait for initial ready/notes
    let _ = rx.recv_timeout(Duration::from_secs(5));

    // Create a note with a tag
    let text = "Hello from uniffi test #uniffi".to_string();
    client.clone().send_command(Command::CreateNote { text: text.clone() });

    // Expect NoteAdded with our content
    let mut saw_added = false;
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        if let Ok(ev) = rx.recv_timeout(Duration::from_millis(200)) {
            match ev {
                Event::NoteAdded { note } => {
                    if note.text == text { saw_added = true; break; }
                }
                _ => {}
            }
        }
    }
    assert!(saw_added, "Should receive NoteAdded for created note");

    // Verify tag list contains our tag
    let tags = client.get_all_tags();
    assert!(tags.contains(&"uniffi".to_string()), "Tag list should include 'uniffi'");
}
