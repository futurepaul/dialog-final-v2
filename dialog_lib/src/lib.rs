use nostr_sdk::prelude::*;
use std::path::PathBuf;
use thiserror::Error;

pub mod note;
pub mod query;
pub mod watch;

pub use note::Note;

#[derive(Error, Debug)]
pub enum DialogError {
    #[error("Nostr error: {0}")]
    Nostr(#[from] nostr_sdk::client::Error),
    #[error("Keys error: {0}")]
    Keys(#[from] nostr_sdk::key::Error),
    #[error("NIP-44 error: {0}")]
    Nip44(#[from] nostr_sdk::nips::nip44::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Database error: {0}")]
    Database(String),
    #[error("Event builder error: {0}")]
    EventBuilder(#[from] nostr_sdk::event::builder::Error),
    #[error("Failed to get project directories")]
    ProjectDirs,
}

pub type Result<T> = std::result::Result<T, DialogError>;

pub struct Dialog {
    pub client: Client,
    pub keys: Keys,
}

impl Dialog {
    pub async fn new(nsec: &str) -> Result<Self> {
        let keys = Keys::parse(nsec)?;

        // Use pubkey in path for isolation
        let db_path = get_data_dir(&keys.public_key().to_hex())?;
        let database = NdbDatabase::open(db_path.to_string_lossy())
            .map_err(|e| DialogError::Database(e.to_string()))?;

        let client = Client::builder()
            .signer(keys.clone())
            .database(database)
            .build();

        Ok(Self { client, keys })
    }

    pub async fn new_with_relay(nsec: &str, relay_url: &str) -> Result<Self> {
        let dialog = Self::new(nsec).await?;
        dialog.connect_relay(relay_url).await?;
        Ok(dialog)
    }

    pub async fn connect_relay(&self, url: &str) -> Result<()> {
        eprintln!("[lib] connect_relay: adding {}", url);
        self.client.add_relay(url).await?;
        eprintln!("[lib] connect_relay: connecting");
        self.client.connect().await;
        eprintln!("[lib] connect_relay: connected");
        Ok(())
    }

    pub fn public_key(&self) -> PublicKey {
        self.keys.public_key()
    }
}

fn get_data_dir(pubkey: &str) -> Result<PathBuf> {
    // 1) CI / user override
    if let Ok(p) = std::env::var("DIALOG_DATA_DIR") {
        let p = PathBuf::from(p).join(pubkey);
        std::fs::create_dir_all(&p)?;
        return Ok(p.join("nostrdb"));
    }

    // 2) OS-correct per-app location
    if let Some(dirs) = directories::ProjectDirs::from("", "", "dialog") {
        let data_dir = dirs.data_dir().join(pubkey);
        std::fs::create_dir_all(&data_dir)?;
        return Ok(data_dir.join("nostrdb"));
    }

    // 3) Last-resort fallback (containers without HOME, etc.)
    let p = std::env::temp_dir().join("dialog").join(pubkey);
    std::fs::create_dir_all(&p)?;
    Ok(p.join("nostrdb"))
}

pub fn clean_test_storage(pubkey: &str) -> Result<()> {
    // Use same resolution as get_data_dir but just get parent
    let data_dir = if let Ok(p) = std::env::var("DIALOG_DATA_DIR") {
        PathBuf::from(p).join(pubkey)
    } else if let Some(dirs) = directories::ProjectDirs::from("", "", "dialog") {
        dirs.data_dir().join(pubkey)
    } else {
        std::env::temp_dir().join("dialog").join(pubkey)
    };

    if data_dir.exists() {
        std::fs::remove_dir_all(data_dir)?;
    }
    Ok(())
}

pub fn validate_nsec(nsec: &str) -> Result<()> {
    Keys::parse(nsec)?;
    Ok(())
}

impl Dialog {
    /// Mark a note as read - stores in local database only
    pub async fn mark_as_read(&self, note_id: &EventId) -> Result<()> {
        // Create a local-only Kind 30078 event for app data
        let content = serde_json::json!({
            "type": "read_status",
            "note_id": note_id.to_hex(),
            "is_read": true,
            "timestamp": Timestamp::now().as_u64()
        })
        .to_string();

        let event = EventBuilder::new(Kind::from(30078), content)
            .tag(Tag::custom(
                TagKind::SingleLetter(SingleLetterTag::lowercase(Alphabet::D)),
                vec!["dialog_local_state"],
            ))
            .sign(&self.keys)
            .await?;

        // Save to database but NEVER publish to relays
        self.client
            .database()
            .save_event(&event)
            .await
            .map_err(|e| DialogError::Database(e.to_string()))?;

        Ok(())
    }

    /// Get read status for a note from local state
    pub async fn get_read_status(&self, note_id: &EventId) -> bool {
        // Query local database for read status
        let filter = Filter::new()
            .author(self.keys.public_key())
            .kind(Kind::from(30078))
            .custom_tag(
                SingleLetterTag::lowercase(Alphabet::D),
                vec!["dialog_local_state"],
            )
            .limit(1000); // Get recent local state events

        if let Ok(events) = self.client.database().query(vec![filter]).await {
            // Convert to vec first to use rev()
            let events_vec: Vec<_> = events.into_iter().collect();
            // Look through events for this note's read status (most recent first)
            for event in events_vec.iter().rev() {
                // Parse content to check if it's for this note
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&event.content) {
                    if data["type"] == "read_status" && data["note_id"] == note_id.to_hex() {
                        return data["is_read"].as_bool().unwrap_or(false);
                    }
                }
            }
        }

        false // Default to unread
    }

    /// Mark a note as synced locally
    pub async fn mark_as_synced(&self, note_id: &EventId) -> Result<()> {
        let content = serde_json::json!({
            "type": "sync_status",
            "note_id": note_id.to_hex(),
            "is_synced": true,
            "timestamp": Timestamp::now().as_u64()
        })
        .to_string();

        let event = EventBuilder::new(Kind::from(30078), content)
            .tag(Tag::custom(
                TagKind::SingleLetter(SingleLetterTag::lowercase(Alphabet::D)),
                vec!["dialog_local_state"],
            ))
            .sign(&self.keys)
            .await?;

        self.client
            .database()
            .save_event(&event)
            .await
            .map_err(|e| DialogError::Database(e.to_string()))?;

        Ok(())
    }
}
