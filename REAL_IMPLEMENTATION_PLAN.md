# Dialog Real Implementation Plan

## Core Philosophy
**SIMPLEST THING THAT COULD POSSIBLY WORK** - No abstractions until proven necessary. Direct use of rust-nostr APIs. Real NIP-44 v2 encryption.

## Phase 1: Minimal dialog_lib (Local-First) ✅ COMPLETED

### 1.1 Project Setup & Basic Types (~Day 1) ✅

**Cargo.toml**
```toml
[package]
name = "dialog_lib"
version = "0.1.0"
edition = "2021"

[dependencies]
nostr-sdk = { version = "0.37", features = ["ndb", "nip44"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
thiserror = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
directories = "5"
keyring = { version = "3", optional = true }

[features]
default = ["keyring"]
```

**src/lib.rs** (~50 lines)
```rust
use nostr_sdk::prelude::*;
use std::path::PathBuf;

pub struct Dialog {
    client: Client,
    keys: Keys,
}

impl Dialog {
    pub async fn new(nsec: &str) -> Result<Self> {
        let keys = Keys::parse(nsec)?;
        let db_path = get_data_dir()?;
        let database = NdbDatabase::open(db_path)?;
        
        let client = Client::builder()
            .signer(keys.clone())
            .database(database)
            .build();
        
        Ok(Self { client, keys })
    }
    
    pub async fn connect_relay(&self, url: &str) -> Result<()> {
        self.client.add_relay(url).await?;
        self.client.connect().await;
        Ok(())
    }
}

fn get_data_dir() -> Result<PathBuf> {
    let dirs = directories::ProjectDirs::from("", "", "dialog")
        .ok_or("Failed to get project dirs")?;
    let data_dir = dirs.data_dir();
    std::fs::create_dir_all(data_dir)?;
    Ok(data_dir.join("nostrdb"))
}
```

### 1.2 Self-DM Creation with NIP-44 (~Day 2) ✅

**src/note.rs** (~100 lines)
```rust
use nostr_sdk::prelude::*;

pub struct Note {
    pub id: EventId,
    pub text: String,
    pub tags: Vec<String>,
    pub created_at: Timestamp,
}

impl Dialog {
    pub async fn create_note(&self, text: &str) -> Result<EventId> {
        // Parse hashtags from text
        let tags = parse_hashtags(text);
        
        // Create encrypted content for self-DM
        let encrypted = nip44::encrypt(
            self.keys.secret_key(),
            &self.keys.public_key(),  // Encrypt to self
            text,
            nip44::Version::default(),
        )?;
        
        // Build event with NIP-44 encrypted content
        let mut builder = EventBuilder::new(Kind::from(1059), encrypted);
        
        // Add t tags for topics (lowercase)
        for tag in &tags {
            builder = builder.tag(Tag::hashtag(tag.to_lowercase()));
        }
        
        // Add p tag pointing to self (for self-DM)
        builder = builder.tag(Tag::public_key(self.keys.public_key()));
        
        // Send to relay (will also save to local db)
        let output = self.client.send_event_builder(builder).await?;
        Ok(output.id())
    }
}

fn parse_hashtags(text: &str) -> Vec<String> {
    text.split_whitespace()
        .filter(|word| word.starts_with('#'))
        .map(|tag| tag[1..].to_lowercase())
        .collect()
}
```

### 1.3 List & Query Notes (~Day 3) ✅

**src/query.rs** (~120 lines)
```rust
impl Dialog {
    pub async fn list_notes(&self, limit: usize) -> Result<Vec<Note>> {
        // Query from local database
        let filter = Filter::new()
            .author(self.keys.public_key())
            .kind(Kind::from(1059))
            .limit(limit);
            
        let events = self.client.database().query(filter).await?;
        
        // Decrypt and convert to Notes
        let mut notes = Vec::new();
        for event in events {
            if let Ok(decrypted) = self.decrypt_event(&event) {
                notes.push(Note {
                    id: event.id,
                    text: decrypted,
                    tags: extract_tags(&event),
                    created_at: event.created_at,
                });
            }
        }
        
        Ok(notes)
    }
    
    pub async fn list_by_tag(&self, tag: &str, limit: usize) -> Result<Vec<Note>> {
        let filter = Filter::new()
            .author(self.keys.public_key())
            .kind(Kind::from(1059))
            .hashtag(tag.to_lowercase())
            .limit(limit);
            
        let events = self.client.database().query(filter).await?;
        // Same decryption logic...
    }
    
    fn decrypt_event(&self, event: &Event) -> Result<String> {
        nip44::decrypt(
            self.keys.secret_key(),
            &self.keys.public_key(),
            &event.content,
        )
    }
}

fn extract_tags(event: &Event) -> Vec<String> {
    event.tags
        .iter()
        .filter_map(|tag| {
            if let Some(TagStandard::Hashtag(t)) = tag.as_standardized() {
                Some(t.to_string())
            } else {
                None
            }
        })
        .collect()
}
```

### 1.4 Integration Test (~Day 4) ✅

**tests/integration.rs**
```rust
#[tokio::test]
async fn test_offline_first() {
    // Create dialog with test key
    let nsec = "nsec1..."; // Test key
    let dialog = Dialog::new(nsec).await.unwrap();
    
    // Create note (offline - no relay connected)
    let text = "Test note #test #offline";
    let id = dialog.create_note(text).await.unwrap();
    
    // Should be in local db
    let notes = dialog.list_notes(10).await.unwrap();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].text, text);
    
    // Connect to test relay
    dialog.connect_relay("ws://localhost:10548").await.unwrap();
    
    // Sync to relay
    let filter = Filter::new().author(dialog.keys.public_key());
    dialog.client.sync(filter, &SyncOptions::default()).await.unwrap();
    
    // Note should now be on relay
}
```

## Phase 2: CLI Implementation (~Days 5-6) 🚧 NEXT

**dialog_cli/Cargo.toml**
```toml
[dependencies]
dialog_lib = { path = "../dialog_lib" }
clap = { version = "4", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
keyring = "3"
```

**dialog_cli/src/main.rs** (~150 lines)
```rust
use clap::{Parser, Subcommand};
use dialog_lib::Dialog;
use keyring::Entry;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new note
    #[command(arg_required_else_help = true)]
    Create {
        /// Note text (hashtags will be parsed)
        text: String,
    },
    
    /// List notes
    List {
        #[arg(short, long, default_value = "10")]
        limit: usize,
        
        /// Filter by tag
        #[arg(short, long)]
        tag: Option<String>,
        
        /// Watch for new notes
        #[arg(long)]
        watch: bool,
    },
    
    /// Import nsec key
    Key {
        #[command(subcommand)]
        cmd: KeyCommand,
    },
}

#[derive(Subcommand)]
enum KeyCommand {
    Import { nsec: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Get or prompt for key
    let nsec = get_stored_key()?;
    let dialog = Dialog::new(&nsec).await?;
    
    // Connect to local relay
    dialog.connect_relay("ws://localhost:10548").await
        .unwrap_or_else(|_| eprintln!("Warning: Could not connect to relay"));
    
    match cli.command {
        Commands::Create { text } => {
            let id = dialog.create_note(&text).await?;
            println!("Created note: {}", id.to_bech32()?);
        }
        
        Commands::List { limit, tag, watch } => {
            let notes = if let Some(tag) = tag {
                dialog.list_by_tag(&tag, limit).await?
            } else {
                dialog.list_notes(limit).await?
            };
            
            for note in notes {
                println!("[{}] {}", note.created_at, note.text);
                if !note.tags.is_empty() {
                    println!("  Tags: {}", note.tags.join(", "));
                }
            }
            
            if watch {
                // Subscribe and stream new notes
                watch_notes(dialog).await?;
            }
        }
        
        Commands::Key { cmd } => {
            match cmd {
                KeyCommand::Import { nsec } => {
                    store_key(&nsec)?;
                    println!("Key imported successfully");
                }
            }
        }
    }
    
    Ok(())
}

fn get_stored_key() -> Result<String> {
    let entry = Entry::new("dialog", "user")?;
    entry.get_password()
}

fn store_key(nsec: &str) -> Result<()> {
    Keys::parse(nsec)?; // Validate
    let entry = Entry::new("dialog", "user")?;
    entry.set_password(nsec)?;
    Ok(())
}
```

## Phase 3: UniFFI Bindings (~Days 7-8)

**dialog_uniffi/src/lib.rs** (~100 lines)
```rust
use dialog_lib::{Dialog as InnerDialog, Note};

#[uniffi::export]
pub struct DialogFFI {
    inner: Arc<Mutex<InnerDialog>>,
}

#[uniffi::export]
impl DialogFFI {
    #[uniffi::constructor]
    pub async fn new(nsec: String) -> Result<Arc<Self>> {
        let dialog = InnerDialog::new(&nsec).await?;
        Ok(Arc::new(Self {
            inner: Arc::new(Mutex::new(dialog)),
        }))
    }
    
    pub async fn create_note(&self, text: String) -> Result<String> {
        let dialog = self.inner.lock().await;
        let id = dialog.create_note(&text).await?;
        Ok(id.to_hex())
    }
    
    pub async fn list_notes(&self, limit: u32) -> Result<Vec<FfiNote>> {
        let dialog = self.inner.lock().await;
        let notes = dialog.list_notes(limit as usize).await?;
        Ok(notes.into_iter().map(Into::into).collect())
    }
}

#[derive(uniffi::Record)]
pub struct FfiNote {
    pub id: String,
    pub text: String,
    pub tags: Vec<String>,
    pub created_at: u64,
}

impl From<Note> for FfiNote {
    fn from(note: Note) -> Self {
        Self {
            id: note.id.to_hex(),
            text: note.text,
            tags: note.tags,
            created_at: note.created_at.as_u64(),
        }
    }
}
```

## Key Implementation Notes

### Simplicity Principles Applied:
1. **Direct API usage** - No wrappers around rust-nostr unless necessary
2. **NIP-44 for self-DMs** - Using Kind 1059 with p-tag pointing to self
3. **Offline-first** - NdbDatabase handles local storage automatically
4. **Single relay for testing** - ws://localhost:10548 with `nak serve`
5. **No premature abstractions** - Each file does one thing

### File Size Discipline:
- lib.rs: ~50 lines (setup only)
- note.rs: ~100 lines (create logic)
- query.rs: ~120 lines (list/filter)
- main.rs: ~150 lines (CLI interface)
- All under 300 line limit

### Testing Strategy:
1. Unit tests for hashtag parsing
2. Integration test for offline→online flow
3. E2E test with `nak serve --port 10548`

### Real Nostr Best Practices:
- Using Kind 1059 for encrypted messages
- NIP-44 v2 encryption (not NIP-04)
- Proper p-tags for self-DMs
- t-tags for topics (lowercase)
- Negentropy sync built into rust-nostr

This plan delivers the SIMPLEST implementation that:
- Actually works with real rust-nostr APIs
- Uses proper NIP-44 encryption
- Handles offline-first correctly
- Can be incrementally tested at each phase