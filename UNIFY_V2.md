# Unified Architecture V2: Collapse the Layers

## Core Principle: SIMPLICITY

No translation layers. No Arc<RwLock<Option<Whatever>>>. Just clean, direct integration.

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                         iOS App (Swift)                      │
│  - Sends Commands                                            │
│  - Receives Events                                           │
│  - Never touches async/await                                 │
└─────────────────────────────────────────────────────────────┘
                              │
                    Fire-and-forget calls
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                    dialog_uniffi (Rust)                      │
│  - Thin wrapper                                              │
│  - Global Tokio runtime                                      │
│  - Spawns all work                                           │
│  - Broadcast channel for events                              │
└─────────────────────────────────────────────────────────────┘
                              │
                     Direct function calls
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                     dialog_lib (Rust)                        │
│  - Single instance, initialized at startup                   │
│  - Handles all Nostr protocol                                │
│  - Manages database                                          │
│  - Tracks read/sync status                                   │
│  - Returns results or errors                                 │
└─────────────────────────────────────────────────────────────┘
```

## Key Design Decisions

### 0. Add Ready Event to UDL
```rust
// dialog_uniffi/src/dialog.udl
[Enum]
interface Event {
    Ready();  // Add this - signals Dialog is initialized
    NotesLoaded(sequence<Note> notes);
    NoteAdded(Note note);
    NoteUpdated(Note note);
    NoteDeleted(string id);
    TagFilterChanged(string? tag);
    SyncStatusChanged(boolean syncing);
    Error(string message);
};
```

### 1. Single Note Structure (in dialog_lib)
```rust
// dialog_lib/src/note.rs
pub struct Note {
    pub id: EventId,
    pub text: String,
    pub tags: Vec<String>,
    pub created_at: Timestamp,
    pub is_read: bool,      // Add these to dialog_lib
    pub is_synced: bool,    // Add these to dialog_lib
}
```

No conversion needed. One structure, used everywhere.

### 2. Dialog Initialization at Startup
```rust
// dialog_uniffi/src/lib.rs
static DIALOG: OnceCell<Dialog> = OnceCell::new();

impl DialogClient {
    pub fn new(nsec: String, relay_url: Option<String>) -> Result<Self, String> {
        // Initialize dialog_lib immediately, spawn the async work
        // Try spawn_blocking first to avoid block_on issues
        let (event_tx, _) = broadcast::channel(1024);
        let event_tx_clone = event_tx.clone();
        
        rt().spawn(async move {
            let dialog = match Dialog::new_with_relay(
                &nsec, 
                relay_url.as_deref().unwrap_or("wss://relay.damus.io")
            ).await {
                Ok(d) => d,
                Err(e) => {
                    let _ = event_tx_clone.send(Event::Error {
                        message: format!("Failed to initialize: {}", e)
                    });
                    return;
                }
            };
            
            DIALOG.set(dialog)
                .map_err(|_| "Dialog already initialized")
                .unwrap();
                
            // Signal ready
            let _ = event_tx_clone.send(Event::Ready);
        });
        
        Ok(Self { event_tx })
    }
}
```

**Alternative if spawn doesn't work - use spawn_blocking:**
```rust
impl DialogClient {
    pub fn new(nsec: String, relay_url: Option<String>) -> Result<Self, String> {
        let (event_tx, _) = broadcast::channel(1024);
        let event_tx_clone = event_tx.clone();
        
        // Use spawn_blocking to avoid blocking Swift's thread
        rt().spawn_blocking(move || {
            // Create a new runtime handle for this blocking context
            let handle = tokio::runtime::Handle::current();
            
            let dialog = handle.block_on(async {
                Dialog::new_with_relay(
                    &nsec,
                    relay_url.as_deref().unwrap_or("wss://relay.damus.io")
                ).await
            });
            
            match dialog {
                Ok(d) => {
                    DIALOG.set(d).unwrap();
                    let _ = event_tx_clone.send(Event::Ready);
                }
                Err(e) => {
                    let _ = event_tx_clone.send(Event::Error {
                        message: format!("Failed to initialize: {}", e)
                    });
                }
            }
        });
        
        Ok(Self { event_tx })
    }
}
```

**No Option. No Arc. No RwLock.** Just a static OnceCell with the Dialog instance.

### 3. Command Processing - Direct Calls
```rust
pub fn send_command(self: Arc<Self>, cmd: Command) {
    let event_tx = self.event_tx.clone();
    
    rt().spawn(async move {
        // Wait for dialog to be initialized if needed
        let dialog = match DIALOG.get() {
            Some(d) => d,
            None => {
                let _ = event_tx.send(Event::Error {
                    message: "Dialog not yet initialized".to_string()
                });
                return;
            }
        };
        
        match cmd {
            Command::CreateNote { text } => {
                match dialog.create_note(&text).await {
                    Ok(note) => {
                        let _ = event_tx.send(Event::NoteAdded { note });
                    }
                    Err(e) => {
                        let _ = event_tx.send(Event::Error { 
                            message: e.to_string() 
                        });
                    }
                }
            }
            Command::LoadNotes { limit } => {
                match dialog.list_notes(limit as usize).await {
                    Ok(notes) => {
                        let _ = event_tx.send(Event::NotesLoaded { notes });
                    }
                    Err(e) => {
                        let _ = event_tx.send(Event::Error { 
                            message: e.to_string() 
                        });
                    }
                }
            }
            // ... other commands
        }
    });
}
```

### 4. Synchronous Queries - Also Direct
```rust
pub fn get_notes(&self, limit: u32, tag: Option<String>) -> Vec<Note> {
    let dialog = DIALOG.get().expect("Dialog not initialized");
    
    rt().block_on(async {
        if let Some(tag) = tag {
            dialog.list_by_tag(&tag, limit as usize).await
        } else {
            dialog.list_notes(limit as usize).await
        }
        .unwrap_or_default()
    })
}
```

### 5. Watch Integration - Single Stream
```rust
// Start watching when DialogClient starts
pub fn start(self: Arc<Self>, listener: Box<dyn DialogListener>) {
    let listener = Arc::from(listener);
    let event_tx = self.event_tx.clone();
    
    // Start the watch
    rt().spawn(async move {
        let dialog = DIALOG.get().expect("Dialog not initialized");
        
        if let Ok(mut receiver) = dialog.watch_notes().await {
            while let Some(note) = receiver.recv().await {
                // Note already has is_read/is_synced from dialog_lib
                let _ = event_tx.send(Event::NoteAdded { note });
            }
        }
    });
    
    // Forward events to Swift
    let mut rx = self.event_tx.subscribe();
    rt().spawn(async move {
        while let Ok(event) = rx.recv().await {
            listener.on_event(event);
        }
    });
}
```

## Changes Required in dialog_lib

### 1. Add Local State to Note
```rust
// dialog_lib/src/note.rs
#[derive(Debug, Clone)]
pub struct Note {
    pub id: EventId,
    pub text: String,
    pub tags: Vec<String>,
    pub created_at: Timestamp,
    pub is_read: bool,
    pub is_synced: bool,
}
```

### 2. Track Local State in Database
```rust
impl Dialog {
    pub async fn mark_as_read(&self, note_id: &EventId) -> Result<()> {
        // Store as local-only Kind 30078 event
        let content = json!({
            "type": "read_status",
            "note_id": note_id.to_hex(),
            "is_read": true,
            "timestamp": Timestamp::now()
        });
        
        let event = EventBuilder::new(
            Kind::from(30078),
            content.to_string()
        )
        .tag(Tag::custom(TagKind::d(), vec!["local_state"]))
        .to_event(&self.keys)?;
        
        // Save to database but NEVER publish
        self.client.database().save_event(&event).await?;
        Ok(())
    }
    
    async fn get_read_status(&self, note_id: &EventId) -> bool {
        // Query local database for read status
        let filter = Filter::new()
            .author(self.keys.public_key())
            .kind(Kind::from(30078))
            .custom_tag(TagKind::d(), vec!["local_state"]);
        
        // Check if note has been marked as read
        // Return false if not found
        false
    }
}
```

### 3. Include Local State When Returning Notes
```rust
impl Dialog {
    pub async fn list_notes(&self, limit: usize) -> Result<Vec<Note>> {
        let filter = Filter::new()
            .author(self.keys.public_key())
            .kind(Kind::from(1059))
            .limit(limit);

        let events = self.client.database().query(vec![filter]).await?;
        
        let mut notes = Vec::new();
        for event in events {
            if let Ok(decrypted) = self.decrypt_event(&event) {
                let is_read = self.get_read_status(&event.id).await;
                let is_synced = true; // If it's in DB, it was synced
                
                notes.push(Note {
                    id: event.id,
                    text: decrypted,
                    tags: extract_tags(&event),
                    created_at: event.created_at,
                    is_read,
                    is_synced,
                });
            }
        }
        
        notes.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(notes)
    }
}
```

### 4. Make EventId and Timestamp UniFFI-Compatible
```rust
// dialog_lib/src/note.rs

// Create simple wrapper types that UniFFI can handle
#[derive(Debug, Clone)]
pub struct Note {
    pub id: String,  // EventId.to_hex()
    pub text: String,
    pub tags: Vec<String>,
    pub created_at: u64,  // Timestamp.as_u64()
    pub is_read: bool,
    pub is_synced: bool,
}

impl From<(Event, bool, bool)> for Note {
    fn from((event, is_read, is_synced): (Event, bool, bool)) -> Self {
        Note {
            id: event.id.to_hex(),
            text: decrypt_event(&event).unwrap_or_default(),
            tags: extract_tags(&event),
            created_at: event.created_at.as_u64(),
            is_read,
            is_synced,
        }
    }
}
```

## Relay Connection Management

### Handle iOS App Lifecycle
```rust
// dialog_uniffi/src/lib.rs
impl DialogClient {
    pub fn on_foreground(&self) {
        rt().spawn(async {
            let dialog = DIALOG.get().expect("Dialog not initialized");
            dialog.connect_relay().await;
        });
    }
    
    pub fn on_background(&self) {
        rt().spawn(async {
            let dialog = DIALOG.get().expect("Dialog not initialized");
            dialog.disconnect_relay().await;
        });
    }
}
```

### Add to dialog_lib
```rust
impl Dialog {
    pub async fn connect_relay(&self) -> Result<()> {
        self.client.connect().await;
        Ok(())
    }
    
    pub async fn disconnect_relay(&self) -> Result<()> {
        self.client.disconnect().await?;
        Ok(())
    }
}
```

## Error Handling Philosophy

**Show everything to the user during development:**

```rust
match dialog.create_note(&text).await {
    Ok(note) => {
        event_tx.send(Event::NoteAdded { note });
    }
    Err(e) => {
        // Full error details for debugging
        event_tx.send(Event::Error { 
            message: format!("Failed to create note: {}", e) 
        });
    }
}
```

Later we can add error categories and user-friendly messages, but for now, full transparency.

## Why This Works

1. **No Optional Dialog**: Initialize once at startup, fail fast if config is wrong
2. **Single Source of Truth**: Note structure defined once in dialog_lib
3. **Direct Integration**: dialog_uniffi calls dialog_lib directly, no translation
4. **Fire-and-Forget Maintained**: All async work spawned on Tokio runtime
5. **Clean Error Propagation**: Errors converted to strings and shown to user
6. **Simple State Management**: Local state stored in database, not in memory

## Migration Steps

1. **Update dialog_lib Note structure** - Add is_read/is_synced fields
2. **Add local state tracking** - Kind 30078 events for read/sync status
3. **Update dialog_uniffi initialization** - Remove mock data, initialize Dialog
4. **Connect commands to dialog_lib** - Direct calls, no translation
5. **Test with iOS app** - Ensure fire-and-forget pattern still works
6. **Add lifecycle management** - Connect/disconnect on foreground/background

## What We're NOT Doing

- ❌ No Arc<RwLock<Option<Dialog>>>
- ❌ No conversion layers between Note types
- ❌ No complex state synchronization
- ❌ No async/await in Swift
- ❌ No Sendable complications
- ❌ No uninitialized states

## Success Metrics

1. Single place where Note is defined (dialog_lib)
2. Single place where Dialog is initialized (DialogClient::new)
3. All errors visible in UI during development
4. iOS app continues to work with fire-and-forget pattern
5. Relay connects/disconnects with app lifecycle
6. Local state persists across app restarts

## Configuration

### iOS - Secure Storage
```swift
// iOS DialogApp - Load from Keychain
class DialogViewModel: ObservableObject {
    private let client: DialogClient
    
    init() {
        // Load nsec from iOS Keychain
        guard let nsec = KeychainHelper.loadNsec() else {
            // First run - generate new key or show onboarding
            let newNsec = generateNewNsec()
            KeychainHelper.saveNsec(newNsec)
            self.client = try! DialogClient(
                nsec: newNsec,
                relayUrl: "wss://relay.damus.io"
            )
            return
        }
        
        // Initialize with saved key
        self.client = try! DialogClient(
            nsec: nsec,
            relayUrl: UserDefaults.standard.string(forKey: "relay_url")
        )
        
        // Start listening for events
        self.client.start(self)
    }
}
```

### CLI - Environment Variables
```bash
# dialog_cli uses environment
export DIALOG_NSEC=nsec1...
export DIALOG_RELAY=wss://relay.damus.io
dialog_cli create "My note"
```

### dialog_lib Initialization Paths
```rust
// dialog_cli/src/main.rs
let nsec = std::env::var("DIALOG_NSEC")?;
let dialog = Dialog::new(&nsec).await?;

// dialog_uniffi - called from Swift with Keychain value
impl DialogClient {
    pub fn new(nsec: String, relay_url: Option<String>) -> Result<Self, String> {
        // nsec comes from iOS secure storage
    }
}

## The Final Architecture

```
iOS → Commands → dialog_uniffi → dialog_lib → Nostr Protocol
                      ↓
                 Broadcast Events
                      ↓
iOS ← Events ← dialog_uniffi ← Results from dialog_lib
```

Simple. Direct. No unnecessary layers.