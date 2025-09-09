# UniFFI Integration Plan: Mock Data Migration

## Overview
Move all mock data and state management from Swift into a Rust `dialog_uniffi` crate using the push-based architecture pattern. Swift becomes a thin, reactive UI layer that renders Rust state and sends user intents back to Rust.

**Key Principle**: Rust owns the truth, Swift just watches and renders.

---

## Architecture Goals

### What We're Building
- **Rust (`dialog_uniffi` crate)**: Owns all state, mock data, and business logic
- **UniFFI Bridge**: Type-safe bindings with push events via callbacks
- **Swift**: Minimal `@Observable` ViewModel that subscribes to Rust events
- **Non-blocking**: All operations are fire-and-forget or return snapshots
- **Push-based**: No polling; Rust pushes changes to Swift via callbacks

### What We're NOT Building (Yet)
- No dependency on `dialog_lib` 
- No real Nostr functionality
- No network calls
- No persistence beyond mock data

---

## Phase 1: Create `dialog_uniffi` Crate Structure

### 1.1 Crate Setup
```bash
cargo new dialog_uniffi --lib
cd dialog_uniffi
```

### 1.2 Cargo.toml
```toml
[package]
name = "dialog_uniffi"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["staticlib", "cdylib"]
name = "dialog_uniffi"

[dependencies]
uniffi = { version = "0.28", features = ["tokio"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros", "time", "sync"] }
once_cell = "1"
chrono = "0.4"
uuid = { version = "1", features = ["v4"] }

[build-dependencies]
uniffi = { version = "0.28", features = ["build"] }

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = true
```

### 1.3 build.rs
```rust
fn main() {
    uniffi::generate_scaffolding("./src/dialog.udl").unwrap();
}
```

---

## Phase 2: UniFFI Interface Definition

### 2.1 src/dialog.udl
```udl
namespace dialog;

// Core data model matching Swift's Note
dictionary Note {
    string id;           // EventId in hex format
    string text;
    sequence<string> tags;
    u64 created_at;      // Unix timestamp in seconds
    boolean is_read;
    boolean is_synced;
};

// Events pushed from Rust to Swift
[Enum]
interface Event {
    // Data events
    NotesLoaded(sequence<Note> notes);
    NoteAdded(Note note);
    NoteUpdated(Note note);
    NoteDeleted(string id);
    
    // View state events
    TagFilterChanged(string? tag);
    
    // Status events
    SyncStatusChanged(boolean syncing);
    Error(string message);
};

// Commands sent from Swift to Rust
[Enum]
interface Command {
    // Note operations
    CreateNote(string text);
    DeleteNote(string id);
    MarkAsRead(string id);
    
    // View operations
    SetTagFilter(string? tag);
    LoadNotes(u32 limit);
    
    // Search
    SearchNotes(string query);
};

// Callback interface for push updates
callback interface DialogListener {
    void on_event(Event event);
};

// Main client interface
interface DialogClient {
    constructor();
    
    // Lifecycle
    [Async]
    void start(DialogListener listener);
    void stop();
    
    // Commands (all non-blocking)
    [Async]
    void send_command(Command cmd);
    
    // Queries (fast snapshots from memory)
    sequence<Note> get_notes(u32 limit, string? tag);
    sequence<string> get_all_tags();
    Note? get_note(string id);
    u32 get_unread_count(string? tag);
};
```

---

## Phase 3: Rust Implementation

### 3.1 src/lib.rs
```rust
mod mock_data;
mod models;

use models::{Note, Event, Command};
use mock_data::generate_mock_notes;

use once_cell::sync::OnceCell;
use std::{collections::HashMap, sync::Arc};
use tokio::{
    runtime::Runtime,
    sync::{broadcast, RwLock},
};

uniffi::include_scaffolding!("dialog");

// Global Tokio runtime
fn rt() -> &'static Runtime {
    static RT: OnceCell<Runtime> = OnceCell::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("dialog-uniffi")
            .build()
            .expect("Failed to create Tokio runtime")
    })
}

pub struct DialogClient {
    notes: Arc<RwLock<HashMap<String, Note>>>,
    current_filter: Arc<RwLock<Option<String>>>,
    event_tx: broadcast::Sender<Event>,
}

#[uniffi::export]
impl DialogClient {
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        let (event_tx, _) = broadcast::channel(1024);
        let client = Arc::new(Self {
            notes: Arc::new(RwLock::new(HashMap::new())),
            current_filter: Arc::new(RwLock::new(None)),
            event_tx,
        });
        
        // Load mock data
        let notes_clone = client.notes.clone();
        rt().spawn(async move {
            let mut notes = notes_clone.write().await;
            for note in generate_mock_notes() {
                notes.insert(note.id.clone(), note);
            }
        });
        
        client
    }
    
    pub async fn start(self: Arc<Self>, listener: Arc<dyn DialogListener>) {
        // Set up event forwarding to Swift
        let mut rx = self.event_tx.subscribe();
        
        rt().spawn(async move {
            while let Ok(event) = rx.recv().await {
                listener.on_event(event);
            }
        });
        
        // Send initial data
        let notes = self.get_notes(100, None);
        listener.on_event(Event::NotesLoaded { notes });
    }
    
    pub fn stop(&self) {
        // Cleanup if needed
    }
    
    pub async fn send_command(self: Arc<Self>, cmd: Command) {
        match cmd {
            Command::CreateNote { text } => {
                self.create_note(text).await;
            }
            Command::SetTagFilter { tag } => {
                self.set_filter(tag).await;
            }
            Command::MarkAsRead { id } => {
                self.mark_as_read(id).await;
            }
            Command::LoadNotes { limit } => {
                let filter = self.current_filter.read().await.clone();
                let notes = self.get_notes(limit, filter);
                let _ = self.event_tx.send(Event::NotesLoaded { notes });
            }
            _ => {} // Handle other commands
        }
    }
    
    // Fast synchronous queries
    pub fn get_notes(&self, limit: u32, tag: Option<String>) -> Vec<Note> {
        let notes = self.notes.blocking_read();
        let mut result: Vec<Note> = notes
            .values()
            .filter(|n| {
                tag.as_ref().map_or(true, |t| n.tags.contains(t))
            })
            .cloned()
            .collect();
        
        result.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        result.into_iter().take(limit as usize).collect()
    }
    
    pub fn get_all_tags(&self) -> Vec<String> {
        let notes = self.notes.blocking_read();
        let mut tags = std::collections::HashSet::new();
        for note in notes.values() {
            for tag in &note.tags {
                tags.insert(tag.clone());
            }
        }
        let mut result: Vec<String> = tags.into_iter().collect();
        result.sort();
        result
    }
    
    pub fn get_note(&self, id: String) -> Option<Note> {
        self.notes.blocking_read().get(&id).cloned()
    }
    
    pub fn get_unread_count(&self, tag: Option<String>) -> u32 {
        let notes = self.notes.blocking_read();
        notes
            .values()
            .filter(|n| !n.is_read)
            .filter(|n| tag.as_ref().map_or(true, |t| n.tags.contains(t)))
            .count() as u32
    }
    
    // Private async helpers
    async fn create_note(self: Arc<Self>, text: String) {
        let note = Note::from_text(text);
        let id = note.id.clone();
        
        // Update state
        self.notes.write().await.insert(id, note.clone());
        
        // Push event
        let _ = self.event_tx.send(Event::NoteAdded { note });
    }
    
    async fn set_filter(self: Arc<Self>, tag: Option<String>) {
        *self.current_filter.write().await = tag.clone();
        let _ = self.event_tx.send(Event::TagFilterChanged { tag });
        
        // Re-send filtered notes
        let notes = self.get_notes(100, tag);
        let _ = self.event_tx.send(Event::NotesLoaded { notes });
    }
    
    async fn mark_as_read(self: Arc<Self>, id: String) {
        let mut notes = self.notes.write().await;
        if let Some(note) = notes.get_mut(&id) {
            note.is_read = true;
            let _ = self.event_tx.send(Event::NoteUpdated { note: note.clone() });
        }
    }
}
```

### 3.2 src/models.rs
```rust
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone, Debug, uniffi::Record)]
pub struct Note {
    pub id: String,
    pub text: String,
    pub tags: Vec<String>,
    pub created_at: u64,
    pub is_read: bool,
    pub is_synced: bool,
}

impl Note {
    pub fn from_text(text: String) -> Self {
        // Parse hashtags
        let tags = text
            .split_whitespace()
            .filter(|word| word.starts_with('#') && word.len() > 1)
            .map(|tag| tag[1..].to_lowercase())
            .collect();
        
        Self {
            id: Uuid::new_v4().to_string(),
            text,
            tags,
            created_at: Utc::now().timestamp() as u64,
            is_read: false,
            is_synced: false,
        }
    }
}

#[derive(Clone, Debug, uniffi::Enum)]
pub enum Event {
    NotesLoaded { notes: Vec<Note> },
    NoteAdded { note: Note },
    NoteUpdated { note: Note },
    NoteDeleted { id: String },
    TagFilterChanged { tag: Option<String> },
    SyncStatusChanged { syncing: bool },
    Error { message: String },
}

#[derive(Clone, Debug, uniffi::Enum)]
pub enum Command {
    CreateNote { text: String },
    DeleteNote { id: String },
    MarkAsRead { id: String },
    SetTagFilter { tag: Option<String> },
    LoadNotes { limit: u32 },
    SearchNotes { query: String },
}
```

### 3.3 src/mock_data.rs
```rust
use crate::models::Note;
use chrono::{Duration, Utc};

pub fn generate_mock_notes() -> Vec<Note> {
    let now = Utc::now();
    vec![
        // Work topic cluster (recent)
        Note {
            id: hex_id(1),
            text: "Need to review the Q4 roadmap before tomorrow's meeting".to_string(),
            tags: vec!["work".to_string(), "planning".to_string()],
            created_at: (now - Duration::minutes(2)).timestamp() as u64,
            is_read: true,
            is_synced: true,
        },
        Note {
            id: hex_id(2),
            text: "Actually, let's push the deadline to Friday".to_string(),
            tags: vec!["work".to_string()],
            created_at: (now - Duration::minutes(1)).timestamp() as u64,
            is_read: true,
            is_synced: true,
        },
        Note {
            id: hex_id(3),
            text: "Don't forget to include the mobile strategy slides".to_string(),
            tags: vec!["work".to_string(), "planning".to_string()],
            created_at: (now - Duration::seconds(30)).timestamp() as u64,
            is_read: false,
            is_synced: true,
        },
        
        // Personal cluster (1 hour ago)
        Note {
            id: hex_id(4),
            text: "Remember to call mom about thanksgiving plans 🦃".to_string(),
            tags: vec!["personal".to_string(), "family".to_string()],
            created_at: (now - Duration::hours(1)).timestamp() as u64,
            is_read: true,
            is_synced: true,
        },
        Note {
            id: hex_id(5),
            text: "Also need to book flights ✈️".to_string(),
            tags: vec!["personal".to_string(), "travel".to_string()],
            created_at: (now - Duration::minutes(59)).timestamp() as u64,
            is_read: true,
            is_synced: false,
        },
        
        // Ideas (2 hours ago)
        Note {
            id: hex_id(6),
            text: "App idea: AI that suggests recipes based on what's in your fridge. Could use vision API to scan items, then GPT to suggest combinations. Maybe partner with grocery delivery services? Could also track expiration dates and suggest meals to avoid waste. Premium feature: meal planning for the week with automatic shopping list generation.".to_string(),
            tags: vec!["ideas".to_string(), "ai".to_string(), "startup".to_string()],
            created_at: (now - Duration::hours(2)).timestamp() as u64,
            is_read: true,
            is_synced: true,
        },
        
        // Coffee notes (yesterday)
        Note {
            id: hex_id(7),
            text: "The new coffee shop on 5th street is amazing! Great wifi too ☕".to_string(),
            tags: vec!["coffee".to_string(), "places".to_string()],
            created_at: (now - Duration::days(1)).timestamp() as u64,
            is_read: true,
            is_synced: true,
        },
        Note {
            id: hex_id(8),
            text: "Ethiopian single origin, notes of blueberry and chocolate".to_string(),
            tags: vec!["coffee".to_string()],
            created_at: (now - Duration::days(1) + Duration::minutes(1)).timestamp() as u64,
            is_read: true,
            is_synced: true,
        },
        Note {
            id: hex_id(9),
            text: "Maybe we should have our 1:1s there instead of the office".to_string(),
            tags: vec!["work".to_string(), "coffee".to_string()],
            created_at: (now - Duration::days(1) + Duration::minutes(2)).timestamp() as u64,
            is_read: true,
            is_synced: true,
        },
        
        // Random thoughts
        Note {
            id: hex_id(10),
            text: "Why do we say 'heads up' when we mean 'duck'? 🤔".to_string(),
            tags: vec!["random".to_string()],
            created_at: (now - Duration::days(2)).timestamp() as u64,
            is_read: true,
            is_synced: true,
        },
        Note {
            id: hex_id(11),
            text: "Learn Rust".to_string(),
            tags: vec!["todo".to_string()],
            created_at: (now - Duration::days(3)).timestamp() as u64,
            is_read: false,
            is_synced: true,
        },
        Note {
            id: hex_id(12),
            text: "That thing Sarah said about compound interest was really insightful. The example about starting to invest at 25 vs 35 was eye-opening. Even small amounts compound significantly over decades.".to_string(),
            tags: vec!["finance".to_string(), "learning".to_string()],
            created_at: (now - Duration::days(4)).timestamp() as u64,
            is_read: true,
            is_synced: true,
        },
        
        // Dev testing burst
        Note {
            id: hex_id(13),
            text: "Testing".to_string(),
            tags: vec!["dev".to_string()],
            created_at: (now - Duration::seconds(10)).timestamp() as u64,
            is_read: false,
            is_synced: false,
        },
        Note {
            id: hex_id(14),
            text: "One".to_string(),
            tags: vec!["dev".to_string()],
            created_at: (now - Duration::seconds(9)).timestamp() as u64,
            is_read: false,
            is_synced: false,
        },
        Note {
            id: hex_id(15),
            text: "Two".to_string(),
            tags: vec!["dev".to_string()],
            created_at: (now - Duration::seconds(8)).timestamp() as u64,
            is_read: false,
            is_synced: false,
        },
        Note {
            id: hex_id(16),
            text: "Three".to_string(),
            tags: vec!["dev".to_string()],
            created_at: (now - Duration::seconds(7)).timestamp() as u64,
            is_read: false,
            is_synced: false,
        },
    ]
}

fn hex_id(n: u32) -> String {
    format!("{:064x}", n)
}
```

---

## Phase 4: Swift Integration

### 4.1 Updated InboxViewModel
```swift
import Foundation
import DialogFFI

@Observable
final class InboxViewModel: DialogListener {
    private let client: DialogClient
    var notes: [Note] = []
    var currentTag: String? = nil
    var allTags: [String] = []
    var isLoading = false
    var errorMessage: String?
    
    init() {
        self.client = DialogClient()
    }
    
    func start() {
        Task {
            // Start listening for events
            await client.start(listener: self)
            
            // Get initial data
            self.notes = client.getNotes(limit: 100, tag: currentTag)
            self.allTags = client.getAllTags()
        }
    }
    
    func stop() {
        client.stop()
    }
    
    // MARK: - DialogListener
    func onEvent(_ event: Event) {
        Task { @MainActor in
            switch event {
            case .notesLoaded(let notes):
                self.notes = notes
                self.isLoading = false
                
            case .noteAdded(let note):
                self.notes.append(note)
                self.notes.sort { $0.createdAt < $1.createdAt }
                
            case .noteUpdated(let note):
                if let index = notes.firstIndex(where: { $0.id == note.id }) {
                    notes[index] = note
                }
                
            case .noteDeleted(let id):
                notes.removeAll { $0.id == id }
                
            case .tagFilterChanged(let tag):
                self.currentTag = tag
                
            case .syncStatusChanged(let syncing):
                // Update UI sync indicator
                break
                
            case .error(let message):
                self.errorMessage = message
            }
        }
    }
    
    // MARK: - User Actions
    @MainActor
    func createNote(text: String) {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }
        
        Task {
            await client.sendCommand(.createNote(text: trimmed))
        }
    }
    
    @MainActor
    func setTagFilter(_ tag: String?) {
        Task {
            await client.sendCommand(.setTagFilter(tag: tag))
        }
    }
    
    @MainActor
    func markAsRead(_ noteId: String) {
        Task {
            await client.sendCommand(.markAsRead(id: noteId))
        }
    }
    
    // Computed properties for UI
    var unreadCount: Int {
        client.getUnreadCount(tag: currentTag)
    }
    
    var displayedNotes: [Note] {
        notes.sorted { $0.createdAt < $1.createdAt }
    }
    
    func bubblePosition(for index: Int) -> BubblePosition {
        // Same logic as before but using Note from Rust
        guard index >= 0 && index < displayedNotes.count else { return .solo }
        
        let note = displayedNotes[index]
        let hasNext = index + 1 < displayedNotes.count && 
                     isWithinTimeThreshold(note, displayedNotes[index + 1])
        let hasPrev = index > 0 && 
                     isWithinTimeThreshold(displayedNotes[index - 1], note)
        
        switch (hasNext, hasPrev) {
        case (false, false): return .solo
        case (true, false):  return .top
        case (false, true):  return .bottom
        case (true, true):   return .middle
        }
    }
    
    private func isWithinTimeThreshold(_ note1: Note, _ note2: Note) -> Bool {
        abs(Int(note1.createdAt) - Int(note2.createdAt)) <= 60
    }
}
```

### 4.2 Build Script for iOS
```bash
#!/bin/bash
# build-uniffi-ios.sh

echo "🦀 Building UniFFI for iOS..."

cd dialog_uniffi

# Build for all iOS architectures
cargo build --release --target aarch64-apple-ios
cargo build --release --target x86_64-apple-ios-sim
cargo build --release --target aarch64-apple-ios-sim

# Generate Swift bindings
cargo run --bin uniffi-bindgen generate \
    --library target/aarch64-apple-ios/release/libdialog_uniffi.a \
    --language swift \
    --out-dir ../ios/DialogApp/Generated

# Create XCFramework
xcodebuild -create-xcframework \
    -library target/aarch64-apple-ios/release/libdialog_uniffi.a \
    -headers ../ios/DialogApp/Generated/dialogFFI.h \
    -library target/x86_64-apple-ios-sim/release/libdialog_uniffi.a \
    -headers ../ios/DialogApp/Generated/dialogFFI.h \
    -library target/aarch64-apple-ios-sim/release/libdialog_uniffi.a \
    -headers ../ios/DialogApp/Generated/dialogFFI.h \
    -output ../ios/DialogApp/Frameworks/DialogFFI.xcframework

echo "✅ UniFFI iOS build complete!"
```

---

## Phase 5: Testing & Validation

### Key Tests
1. **Non-blocking UI**: Create 100 notes rapidly, UI should remain responsive
2. **Event ordering**: Events arrive in correct order
3. **Filter changes**: Switching tags updates view immediately
4. **Memory**: No retain cycles between Swift and Rust
5. **Background updates**: Notes can arrive while app is in background

### Performance Metrics
- Note creation: < 1ms to return to UI
- Filter change: < 10ms to update view
- Initial load of 1000 notes: < 100ms
- Memory usage: < 50MB for 10,000 notes

---

## Migration Path

### Step 1: Create `dialog_uniffi` crate (Week 1)
- Set up crate structure
- Implement mock data
- Create UniFFI bindings

### Step 2: Update iOS app (Week 1)
- Add DialogFFI.xcframework
- Update ViewModel to use DialogClient
- Remove Swift mock data

### Step 3: Validate (Week 2)
- Test all UI interactions
- Verify no blocking
- Check memory usage

### Step 4: Future - Real Integration (Week 3+)
- Add dependency on `dialog_lib`
- Replace mock data with real Nostr operations
- Add actual encryption/sync

---

## Success Criteria

✅ All mock data lives in Rust
✅ Swift UI never blocks on Rust operations
✅ Push-based updates via callbacks work reliably
✅ Clean separation: Rust owns state, Swift just renders
✅ Type-safe bindings with no manual marshalling
✅ Can add new event/command types without breaking Swift

---

## Key Architecture Decisions

### Why Push-Based?
- No polling overhead
- Instant updates
- Clean separation of concerns
- Natural fit for SwiftUI's reactive model

### Why Commands/Events Pattern?
- Single source of truth for all interactions
- Easy to add new features
- Great for debugging (can log all commands/events)
- Natural audit trail

### Why Async Everywhere?
- UI never blocks
- Can handle thousands of notes
- Ready for real network operations
- Matches Swift's async/await model

### Why Separate Crate?
- Clean separation from `dialog_lib`
- Can iterate on API without touching core
- Easy to test in isolation
- Clear migration path

This architecture ensures we can move fast on UI iteration while keeping all business logic in Rust, setting us up perfectly for the real Nostr integration later!