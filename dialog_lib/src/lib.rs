use nostr_sdk::prelude::*;
use std::path::PathBuf;
use thiserror::Error;

pub mod note;
pub mod query;

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
    #[error("Failed to get project directories")]
    ProjectDirs,
}

pub type Result<T> = std::result::Result<T, DialogError>;

pub struct Dialog {
    pub client: Client,
    pub keys: Keys,
}

impl Dialog {
    pub async fn new(nsec: &str, relay_url: &str) -> Result<Self> {
        let keys = Keys::parse(nsec)?;

        // Use pubkey in path for isolation
        let db_path = get_data_dir(&keys.public_key().to_hex())?;
        let database = NdbDatabase::open(db_path.to_string_lossy())
            .map_err(|e| DialogError::Database(e.to_string()))?;

        let client = Client::builder()
            .signer(keys.clone())
            .database(database)
            .build();

        // Add relay
        client.add_relay(relay_url).await?;
        client.connect().await;

        Ok(Self { client, keys })
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
