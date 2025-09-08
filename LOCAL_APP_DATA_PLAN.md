# Local App Data Plan

## Problem Statement
We need to track local state for:
1. **Read/Unread status** - Which messages have been viewed
2. **Sync status** - Which messages have been synced to relays

This state should:
- Be stored locally only (never synced to relays)
- Persist across app restarts
- Be efficient to update and query

## Option 1: Kind 30078 Events (Local-Only)
Use NIP-78 "Arbitrary custom app data" events stored in local database but never published.

### Implementation
```rust
// Create a local-only Kind 30078 event
let app_data = EventBuilder::new(
    Kind::from(30078),
    json!({
        "unread": ["note_id_1", "note_id_2"],
        "unsynced": ["note_id_3", "note_id_4"]
    }).to_string(),
    vec![
        Tag::custom(TagKind::d(), vec!["dialog_local_state"]),
    ]
);

// Save to database but NEVER publish
client.database().save_event(&event).await?;
```

### Pros
- Uses existing nostr event structure
- Automatically versioned with timestamps
- Can query history if needed
- Works with existing database

### Cons
- Risk of accidentally syncing (must be careful with filters)
- Overhead of event structure for simple data
- Need to ensure negentropy NEVER includes Kind 30078

## Option 2: Separate SQLite Table
Create a dedicated local state table alongside NdbDatabase.

### Implementation
```rust
// Create local_state table
CREATE TABLE dialog_state (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

// Store as JSON
INSERT OR REPLACE INTO dialog_state (key, value, updated_at) 
VALUES ('unread', '["id1", "id2"]', ?);
```

### Pros
- Complete isolation from nostr events
- No risk of accidental sync
- More efficient for frequent updates
- Can add indexes for performance

### Cons
- Need separate database connection
- More code to maintain
- Not using existing infrastructure

## Option 3: File-Based State
Store state in a JSON file in the data directory.

### Implementation
```rust
#[derive(Serialize, Deserialize)]
struct LocalState {
    unread: HashSet<EventId>,
    unsynced: HashSet<EventId>,
    updated_at: Timestamp,
}

// Save to ~/.dialog/{pubkey}/local_state.json
let state_path = data_dir.join("local_state.json");
fs::write(&state_path, serde_json::to_string(&state)?)?;
```

### Pros
- Simple and straightforward
- Easy to debug (human-readable)
- No database complexity
- Fast for small datasets

### Cons
- Need file locking for concurrent access
- Whole file rewrite on every update
- Less efficient for large datasets

## Option 4: In-Memory with Periodic Persistence
Keep state in memory, persist periodically and on shutdown.

### Implementation
```rust
struct DialogState {
    unread: Arc<RwLock<HashSet<EventId>>>,
    unsynced: Arc<RwLock<HashSet<EventId>>>,
    persist_handle: JoinHandle<()>,
}

impl DialogState {
    fn mark_read(&self, id: EventId) {
        self.unread.write().unwrap().remove(&id);
        self.schedule_persist();
    }
}
```

### Pros
- Fastest for reads/writes
- Can batch persistence
- Good for temporary state

### Cons
- Risk of data loss on crash
- More complex shutdown handling
- Memory usage for large datasets

## Recommendation: Hybrid Approach

**Primary:** Use Option 1 (Kind 30078) with strict filtering
**Fallback:** Add file-based checkpoint for recovery

### Why Kind 30078?
1. **Consistency** - Uses same storage as other data
2. **Atomicity** - Database handles transactions
3. **Queryability** - Can use existing query infrastructure
4. **Versioning** - Natural event ordering

### Safety Measures
```rust
// CRITICAL: Never sync Kind 30078
impl Dialog {
    pub async fn sync_notes(&self) -> Result<()> {
        // ONLY sync our encrypted notes, NEVER app data
        let filter = Filter::new()
            .author(self.keys.public_key())
            .kind(Kind::from(1059)); // ONLY GiftWrap
        
        self.client.sync(filter, &SyncOptions::default()).await?;
    }
    
    pub async fn get_local_state(&self) -> Result<LocalState> {
        let filter = Filter::new()
            .author(self.keys.public_key())
            .kind(Kind::from(30078))
            .custom_tag(TagKind::d(), vec!["dialog_local_state"])
            .limit(1);
        
        // Query LOCAL database only
        let events = self.client.database().query(vec![filter]).await?;
        // Parse and return state
    }
}
```

### Migration Path
1. Start with Kind 30078 for simplicity
2. If performance issues, add in-memory cache
3. If sync accidents occur, move to SQLite table

## Implementation Priority
1. Fix immediate issues (sorting, stream staying open)
2. Add basic unread tracking with Kind 30078
3. Add sync status tracking
4. Optimize with caching if needed