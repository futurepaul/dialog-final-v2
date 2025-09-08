# Dialog Note-to-Self App Implementation Plan (Refined)

## Project Overview
Build a cross-platform note-to-self application using Nostr with NIP-44 encryption, progressing from a Rust library to CLI to iOS app.

**Core Principle**: SIMPLICITY IS THE MOST IMPORTANT FEATURE. Each phase must be self-contained, iteratively testable, and follow the "simplest thing that can possibly work" philosophy.

## Architecture Overview

```
dialog_lib (Rust) → dialog_cli (Rust) → dialog_uniffi (UniFFI) → iOS App (Swift)
       ↓                    ↓                     ↓
   nostrdb storage    CLI interface        Swift bindings
   NIP-44 encryption   Local testing      Fragment UI migration
```

---

## Phase 1: dialog_lib Core Library (~1-2 weeks)

**Goal**: Create a rock-solid Rust library that handles all core functionality with offline-first approach.

### Incremental Milestones (SIMPLEST first)
1. **Milestone 1.1**: Local-only create/list (no network) (~2 days)
2. **Milestone 1.2**: Add NIP-44 v2 encrypt/decrypt to self (~2 days)  
3. **Milestone 1.3**: Add relay publish/subscribe with failure warnings (~2 days)
4. **Milestone 1.4**: Add tag parsing + filtering (~1 day)

### File Structure (each ≤300 lines)
- **`dialog_lib/src/lib.rs`** (~100 lines)
  - Public API surface
  - Main Dialog struct and core methods

- **`dialog_lib/src/config.rs`** (~80 lines) 
  - Platform-specific storage paths
  - macOS: `~/Library/Application Support/dialog`
  - Linux: `~/.local/share/dialog`  
  - iOS: App sandbox Documents
  - Relay configuration

- **`dialog_lib/src/keys.rs`** (~120 lines)
  - Key management + Keychain integration
  - One auth per session, cache in-memory
  - Import nsec functionality
  - Platform-specific secure storage

- **`dialog_lib/src/db.rs`** (~150 lines)
  - nostr-ndb wrapper
  - Local storage with created_at and t-tag indices
  - Offline-first writes

- **`dialog_lib/src/crypto.rs`** (~100 lines)
  - NIP-44 v2 helpers for self-DMs
  - Encrypt/decrypt to own pubkey

- **`dialog_lib/src/model.rs`** (~80 lines)
  - Note struct, Tag definitions
  - Core data types

- **`dialog_lib/src/runtime.rs`** (~120 lines)
  - Background tasks/subscriptions
  - Relay connection management
  - Watch functionality

### Public API
```rust
pub struct Dialog { /* ... */ }

impl Dialog {
    pub fn new(opts: DialogOptions) -> Result<Self>;
    pub fn set_relays(&self, urls: &[Url]) -> Result<()>;
    pub fn create_note(&self, text: &str, tags: &[String]) -> Result<NoteId>;
    pub fn list_notes(&self, limit: usize, tags: &[String]) -> Result<Vec<Note>>;
    pub fn watch_notes(&self, limit: usize, tags: &[String]) -> WatchHandle;
}
```

### Error Handling Philosophy
- **ALWAYS save to local nostrdb first**
- **Warn on sync failure, don't fail the operation**
- Exit code 0 if local save succeeded
- Nonzero only if local DB write fails

### Testing Strategy
- **Unit tests**: Tag parsing from content, self-DM encrypt/decrypt roundtrip
- **Integration test**: Create offline → present in list; bring relay up → publish succeeds
- **Example**: `examples/smoke.rs` to print last 10 and watch

**Phase 1 Success Criteria**:
- Import nsec and create encrypted self-DMs locally
- Store notes in local NostrDB with proper indexing
- Publish to ws://localhost:10547 when available
- List and filter notes by tags (case-insensitive)
- All milestones pass incrementally

---

## Phase 2: dialog_cli Command-Line Interface (~3-4 days)

**Goal**: Exercise dialog_lib with a clean, composable CLI interface.

### Commands
- `dialog "this is my note to self" #tag1 #tag2`
  - Parses hashtags, saves to nostrdb, attempts publish
  - Warns on network failure but exits 0 if local save succeeds
- `dialog list` → last 10 notes
- `dialog list -l 1` → most recent 1 note  
- `dialog list -l 2 #tag2` → last 2 notes filtered by tag2
- `dialog list --watch [-l N] [#tag ...]` → print last N then stream new

### Key Features
- **Key Management**: `dialog key import nsec1...` command
- **First-run**: Prompt to store nsec in Keychain, cache decrypted key in-memory
- **Configuration**: 
  - macOS: `~/Library/Application Support/dialog/config.toml`
  - Linux: `~/.config/dialog/config.toml`
  - Override with `DIALOG_RELAYS` env var
- **Clap** for concise command parsing

### File Structure  
- **`dialog_cli/src/main.rs`** (~150 lines)
  - Command parsing and dispatch
  - Error handling and user feedback

- **`dialog_cli/src/config.rs`** (~100 lines)
  - Config file management
  - Keychain integration for CLI

### Testing
- **Snapshot tests**: List output format including --watch backlog
- **E2E tests**: Against local nak relay up/down
- **Verification**: Offline-first save with warning-only on publish failure

**Phase 2 Success Criteria**:
- All commands work reliably with local relay
- Secure nsec storage and session caching
- Proper exit codes and user-friendly warnings

---

## Phase 3: dialog_uniffi UniFFI Bindings (~2-3 days)

**Goal**: Create minimal, stable Swift bindings.

### Exported Types (lean)
```rust
// UniFFI types
pub struct FfiDialog; // Opaque handle
pub struct FfiNote {
    pub id: String,
    pub created_at: u64, 
    pub text: String,
    pub tags: Vec<String>,
}
```

### Exported Functions
```rust
pub fn dialog_new(opts: FfiDialogOptions) -> Result<FfiDialog>;
pub fn dialog_create_note(dialog: &FfiDialog, text: String, tags: Vec<String>) -> Result<String>; // returns NoteId
pub fn dialog_list_notes(dialog: &FfiDialog, limit: u32, tags: Vec<String>) -> Result<Vec<FfiNote>>;
pub fn dialog_watch_notes(dialog: &FfiDialog, limit: u32, tags: Vec<String>, callback: Box<dyn FfiWatchCallback>);
```

### Threading Model
- All I/O on background runtime inside Rust
- Callbacks marshalled to Swift main thread by Swift wrapper

### File Structure
- **`dialog_uniffi/src/lib.rs`** (~150 lines)
  - UniFFI wrapper around dialog_lib
  - Handle async operations properly

- **`dialog_uniffi/uniffi.udl`** (~80 lines)
  - Interface definition for Swift bindings

**Phase 3 Success Criteria**:
- Swift bindings generate successfully  
- XCFramework builds for iOS targets
- Smoke test: create, list, receive callback works from Swift

---

## Phase 4: iOS App Integration (~1 week)

**Goal**: Replace Fragment's CoreData/server architecture with dialog_lib.

### Swift Integration Strategy
- **`DialogStore: ObservableObject`** (~200 lines)
  - Holds FfiDialog handle
  - `@Published var notes: [UINote]` 
  - On init: calls `dialog_list_notes(limit: 50)`
  - Starts `dialog_watch_notes` with callback updating notes on main thread
  - Methods like `create(text: String)` call Rust on background queue

### Key Benefits
- **No complex Combine/FFI bridging** - just callbacks and @Published
- **Rust owns I/O, Swift owns rendering**
- **Clean separation of concerns**

### Keychain Integration  
- iOS: `kSecAttrAccessibleAfterFirstUnlock`
- Prompt on first launch or nsec import
- Keep in-memory for app session
- Zeroize on background termination

### File Updates
- **Update `ContentView.swift`** (~250 lines)
  - Replace server calls with DialogStore
  - Maintain existing UI/UX patterns
  - Handle async operations cleanly

- **Remove CoreData dependencies**
  - Delete MessageEntity, PersistenceController
  - Update settings and sync management

**Phase 4 Success Criteria**:
- Fragment UI works with dialog_lib backend
- Notes encrypt/decrypt properly on device
- iOS app connects to local relay
- UI updates live via watch functionality

---

## Phase 5: XcodeGen + Build Scripts (~2-3 days)

**Goal**: Reproducible iOS builds with proper tooling.

### Build System
- **`project.yml`** (~100 lines)
  - Minimal, versioned XcodeGen configuration
  - iOS 18 target
  - UniFFI Swift module integration

### Build Scripts
- **`build-ios.sh`** (~150 lines)
  - Clean, run xcodegen
  - Generate UniFFI Swift bindings  
  - Build and produce XCFramework for:
    - `aarch64-apple-ios`
    - `x86_64-apple-ios-simulator`

- **`clean-ios.sh`** (~50 lines)
  - Remove derived data
  - Clean all generated UniFFI bindings and XCFramework artifacts

**Phase 5 Success Criteria**:
- `build-ios.sh` succeeds on clean machine
- `clean-ios.sh` restores clean state
- Deterministic builds via versioned project.yml

---

## Key Implementation Details

### Secrets Management (One Prompt Per Session)
- **macOS CLI**: Keychain via `keyring` crate, cache decrypted key in-memory for process lifetime
- **iOS**: Keychain with `kSecAttrAccessibleAfterFirstUnlock`, cache for app session
- **Security**: Zeroize keys on termination, use PID guards to avoid concurrent prompts

### Nostr Implementation
- **Encryption**: rust-nostr NIP-44 v2 for self-DMs (pubkey = own pubkey)
- **Tags**: Parse `#tag` → lowercase `t` tag values, dedupe before emit
- **Storage**: nostr-ndb with minimal indices (created_at, tags)
- **Sync**: Negentropy when relay available, local-only when offline

### Performance Optimizations
- Batch writes where possible via nostr-ndb API
- Keep indices minimal to reduce overhead
- Avoid blocking UI thread in iOS app

---

## Future TODOs (Post-Phase 5)

### Enhanced Functionality
- Optional relay authentication
- Multi-relay support and prioritization  
- Note editing/deletion capabilities
- Export/import functionality

### Advanced Features
- Relay discovery mechanisms
- User-configurable relay management UI
- Advanced tag autocomplete
- Full-text search across notes

### Performance & Security
- Database encryption at rest
- Memory usage optimization
- Background sync improvements
- Enhanced key derivation options

---

## Testing Strategy Summary

### Per-Phase Testing
- **Phase 1**: Unit tests (crypto, parsing), integration tests (offline→online)
- **Phase 2**: Snapshot tests (CLI output), E2E tests (relay up/down scenarios)  
- **Phase 3**: Swift binding smoke tests (create/list/callback)
- **Phase 4**: SwiftUI rendering tests, live update verification
- **Phase 5**: Build script reliability tests (clean machine verification)

### Continuous Validation
- Each phase must pass all tests before proceeding
- Manual CLI testing workflows documented
- Performance regression testing at each milestone
- Real NIP-44 crypto validation (no placeholders)

The refined plan prioritizes offline-first functionality, platform-native conventions, and incremental testing while maintaining the core simplicity principle.