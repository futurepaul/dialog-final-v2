# Unification Plan: dialog_uniffi + dialog_lib

## Executive Summary

This document outlines the strategy to replace mock data in `dialog_uniffi` with real functionality from `dialog_lib`, enabling the iOS app to use actual Nostr protocol operations.

## Current State Analysis

### dialog_uniffi (Mock Implementation)
- **Storage**: In-memory HashMap
- **State Management**: RwLock for thread safety
- **Events**: broadcast channel for push updates
- **Runtime**: Global Tokio runtime
- **Pattern**: Fire-and-forget synchronous methods

### dialog_lib (Real Implementation)
- **Storage**: NdbDatabase (local-first Nostr database)
- **Protocol**: NIP-44 encrypted self-DMs via Kind 1059 events
- **Networking**: nostr_sdk Client with relay support
- **Sync**: Negentropy protocol for efficient synchronization
- **Operations**: Async throughout

## Key Design Mismatches

### 1. Note Structure Differences

**dialog_uniffi Note:**
```rust
struct Note {
    id: String,           // UUID string
    text: String,
    tags: Vec<String>,
    created_at: u64,      // Unix timestamp
    is_read: bool,        // Local state
    is_synced: bool,      // Local state
}
```

**dialog_lib Note:**
```rust
struct Note {
    id: EventId,          // Nostr EventId
    text: String,
    tags: Vec<String>,
    created_at: Timestamp, // Nostr Timestamp
}
```

**Resolution**: Add adapter layer in dialog_uniffi to track local state (is_read, is_synced) separately.

### 2. Initialization Pattern

**dialog_uniffi**: Simple constructor, no async
```rust
DialogClient::new()
```

**dialog_lib**: Async initialization with keys and optional relay
```rust
Dialog::new(nsec: &str) -> Result<Self>
Dialog::new_with_relay(nsec: &str, relay: &str) -> Result<Self>
```

**Resolution**: Initialize dialog_lib in background during `DialogClient::new()`, store in Arc<RwLock<Option<Dialog>>>.

### 3. Event Streaming

**dialog_uniffi**: Broadcast channel with manual filtering
**dialog_lib**: mpsc channel from watch_notes()

**Resolution**: Bridge watch_notes() receiver to broadcast sender.

## Implementation Strategy

### Phase 1: Infrastructure Setup ✅
Add dialog_lib dependency to dialog_uniffi/Cargo.toml:
```toml
[dependencies]
dialog_lib = { path = "../dialog_lib" }
nostr-sdk = { workspace = true }
```

### Phase 2: State Management Layer

Create `state.rs` in dialog_uniffi:
```rust
pub struct UnifiedState {
    // Core dialog_lib instance
    dialog: Arc<RwLock<Option<Dialog>>>,
    
    // Local state tracking
    read_status: Arc<RwLock<HashMap<EventId, bool>>>,
    sync_status: Arc<RwLock<HashMap<EventId, bool>>>,
    
    // Event broadcasting
    event_tx: broadcast::Sender<Event>,
}
```

### Phase 3: Adapter Implementation

#### 3.1 Note Conversion
```rust
impl From<dialog_lib::Note> for Note {
    fn from(lib_note: dialog_lib::Note) -> Self {
        Note {
            id: lib_note.id.to_hex(),
            text: lib_note.text,
            tags: lib_note.tags,
            created_at: lib_note.created_at.as_u64(),
            is_read: false,  // Check local state
            is_synced: true, // Assume synced if from DB
        }
    }
}
```

#### 3.2 Command Processing
Replace mock implementation with real calls:
```rust
Command::CreateNote { text } => {
    if let Some(dialog) = &*self.dialog.read().await {
        match dialog.create_note(&text).await {
            Ok(event_id) => {
                // Fetch the created note
                // Update local state
                // Broadcast NoteAdded event
            }
            Err(e) => {
                self.event_tx.send(Event::Error { 
                    message: e.to_string() 
                });
            }
        }
    }
}
```

### Phase 4: Watch Integration

Bridge dialog_lib's watch_notes() to UniFFI's event system:
```rust
async fn start_watch(&self) {
    if let Some(dialog) = &*self.dialog.read().await {
        if let Ok(mut receiver) = dialog.watch_notes().await {
            while let Some(note) = receiver.recv().await {
                let uniffi_note = self.convert_with_local_state(note).await;
                self.event_tx.send(Event::NoteAdded { note: uniffi_note });
            }
        }
    }
}
```

### Phase 5: Synchronous Query Methods

Implement fast queries using dialog_lib's database:
```rust
pub fn get_notes(&self, limit: u32, tag: Option<String>) -> Vec<Note> {
    rt().block_on(async {
        if let Some(dialog) = &*self.dialog.read().await {
            let notes = if let Some(tag) = tag {
                dialog.list_by_tag(&tag, limit as usize).await
            } else {
                dialog.list_notes(limit as usize).await
            };
            
            match notes {
                Ok(notes) => notes.into_iter()
                    .map(|n| self.convert_with_local_state(n))
                    .collect(),
                Err(_) => Vec::new()
            }
        } else {
            Vec::new()
        }
    })
}
```

## Critical Implementation Details

### 1. Initialization Sequence
```rust
impl DialogClient {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(1024);
        let dialog = Arc::new(RwLock::new(None));
        let dialog_clone = dialog.clone();
        
        // Initialize dialog_lib in background
        rt().spawn(async move {
            // Get nsec from environment or config
            if let Ok(nsec) = std::env::var("DIALOG_NSEC") {
                if let Ok(d) = Dialog::new(&nsec).await {
                    *dialog_clone.write().await = Some(d);
                }
            }
        });
        
        Self {
            dialog,
            event_tx,
            // ... other fields
        }
    }
}
```

### 2. Error Handling Strategy
- All dialog_lib errors converted to Event::Error
- Operations fail silently if dialog not initialized
- UI shows connection status based on dialog state

### 3. Local State Persistence
Use dialog_lib's database for local state:
```rust
// Store as Kind 30078 (app-specific data) events
// Never sync these to relays
async fn save_local_state(&self, note_id: EventId, is_read: bool) {
    // Create local-only event with app data
    // Save to database but don't publish
}
```

## Potential Pitfalls & Solutions

### Pitfall 1: Async Initialization Race
**Problem**: iOS app might call methods before dialog_lib is initialized.
**Solution**: Queue commands until initialization completes.

### Pitfall 2: Database Path Conflicts
**Problem**: iOS app and tests might use same database path.
**Solution**: Use app group container on iOS, temp dir for tests.

### Pitfall 3: Relay Connection Management
**Problem**: iOS app lifecycle vs persistent connections.
**Solution**: Auto-reconnect on app foreground, disconnect on background.

### Pitfall 4: Thread Safety with Nostr SDK
**Problem**: nostr_sdk Client might not be Send+Sync.
**Solution**: Wrap in Arc<RwLock> and clone for each operation.

### Pitfall 5: Error Message Propagation
**Problem**: Complex error types from dialog_lib.
**Solution**: Convert all errors to user-friendly strings for UI.

## Testing Strategy

### Unit Tests
1. Test note conversion between formats
2. Test local state management
3. Test error handling

### Integration Tests
1. Test full command flow (create → watch → list)
2. Test relay connection/disconnection
3. Test database persistence

### iOS Testing
1. Test app launch → initialization
2. Test background/foreground transitions
3. Test offline mode operation

## Migration Checklist

- [ ] Add dialog_lib dependency
- [ ] Create state management layer
- [ ] Implement note conversion
- [ ] Replace mock create_note
- [ ] Replace mock list_notes
- [ ] Integrate watch_notes
- [ ] Add relay configuration
- [ ] Handle initialization
- [ ] Add error propagation
- [ ] Test with iOS app
- [ ] Handle offline mode
- [ ] Add sync status tracking
- [ ] Implement local state persistence
- [ ] Test background/foreground
- [ ] Performance optimization

## Configuration Requirements

### Environment Variables
```bash
DIALOG_NSEC=nsec1...       # User's private key
DIALOG_RELAY=wss://...     # Relay URL
DIALOG_DATA_DIR=/path/to/db # Database location
```

### iOS Info.plist
```xml
<key>LSEnvironment</key>
<dict>
    <key>DIALOG_DATA_DIR</key>
    <string>$(APP_GROUP_CONTAINER)/dialog</string>
</dict>
```

## Performance Considerations

1. **Database Queries**: Use indexes for tag filtering
2. **Decryption**: Cache decrypted content in memory
3. **Watch Efficiency**: Batch updates to UI
4. **Memory Usage**: Limit in-memory note cache size

## Security Considerations

1. **Key Management**: Never log or expose nsec
2. **Encryption**: All notes use NIP-44 encryption
3. **Local State**: Never sync local app data to relays
4. **Database**: Encrypt database at rest on iOS

## Success Criteria

1. iOS app creates real Nostr events
2. Notes persist across app restarts
3. Sync works with relay connection
4. Offline mode functions correctly
5. No UI freezes or crashes
6. Error messages are user-friendly

## Next Steps

1. Create feature branch: `unify-with-dialog-lib`
2. Implement Phase 1-2 (infrastructure)
3. Test basic integration
4. Implement Phase 3-5 (full functionality)
5. Test with iOS app
6. Performance optimization
7. Merge to master

## Timeline Estimate

- Phase 1-2: 2 hours (setup and state)
- Phase 3: 3 hours (adapters and conversions)
- Phase 4: 2 hours (watch integration)
- Phase 5: 2 hours (query methods)
- Testing: 3 hours
- Debugging: 2 hours
- **Total: ~14 hours**

## Code Examples

### Complete Integration Example
```rust
// dialog_uniffi/src/lib.rs
use dialog_lib::{Dialog, Note as LibNote};

impl DialogClient {
    pub fn send_command(self: Arc<Self>, cmd: Command) {
        let self_clone = self.clone();
        rt().spawn(async move {
            match cmd {
                Command::CreateNote { text } => {
                    // Get dialog instance
                    let dialog_guard = self_clone.dialog.read().await;
                    if let Some(dialog) = dialog_guard.as_ref() {
                        // Create note using dialog_lib
                        match dialog.create_note(&text).await {
                            Ok(event_id) => {
                                // Convert and broadcast
                                if let Some(note) = self_clone.fetch_note(event_id).await {
                                    let _ = self_clone.event_tx.send(Event::NoteAdded { note });
                                }
                            }
                            Err(e) => {
                                let _ = self_clone.event_tx.send(Event::Error {
                                    message: format!("Failed to create note: {}", e)
                                });
                            }
                        }
                    } else {
                        let _ = self_clone.event_tx.send(Event::Error {
                            message: "Dialog not initialized".to_string()
                        });
                    }
                }
                // ... other commands
            }
        });
    }
}
```

This plan provides a complete roadmap for unifying dialog_uniffi with dialog_lib while maintaining the fire-and-forget pattern that works well with Swift 6.