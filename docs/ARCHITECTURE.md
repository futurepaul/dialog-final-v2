# Architecture: Rust → UniFFI → Swift → iOS

## Overview

This project demonstrates a **lean, synchronous pattern** for integrating Rust with iOS via UniFFI, avoiding Swift 6 concurrency issues entirely.

## Architecture Principles

- **Rust owns all state** via `dialog_uniffi` crate
- **Push-based updates** from Rust to Swift via callbacks
- **Swift is just UI** - sends commands to Rust, renders what Rust tells it
- **Non-blocking** - all operations are fire-and-forget

## The Solution: Synchronous Pattern

Instead of fighting Swift 6 concurrency with async/await, we use a **fire-and-forget pattern**:

### Key Changes
- All Rust methods are **synchronous** (non-blocking) - they spawn work internally
- Swift calls are simple method invocations - no `Task`, no `await`
- Callbacks explicitly handle thread boundaries with `Task { @MainActor in ... }`
- Client remains non-Sendable but it doesn't matter since we don't pass it around

### Rust Side (dialog_uniffi)

```rust
pub fn start(self: Arc<Self>, listener: Box<dyn DialogListener>) {
    // Non-blocking: spawn listener on background thread
    rt().spawn(async move {
        while let Ok(event) = rx.recv().await {
            listener.on_event(event); // Callback to Swift
        }
    });
    
    // Send initial data immediately
    let notes = self.get_notes(100, None);
    listener.on_event(Event::NotesLoaded { notes });
}

pub fn send_command(self: Arc<Self>, cmd: Command) {
    // Fire-and-forget: spawn work on Tokio runtime
    rt().spawn(async move {
        match cmd {
            Command::CreateNote { text } => {
                self.create_note(text).await;
            }
            // ... handle other commands
        }
    });
}
```

### Swift Side (iOS App)

```swift
// No async/await needed!
client.start(self)  // Fire and forget
client.sendCommand(cmd: .createNote(text: text))  // Non-blocking

// Callback handles threading
func onEvent(_ e: Event) {
    Task { @MainActor in 
        // Update UI safely
        self.messages.append(e.message)
    }
}
```

## Component Breakdown

### 1. Rust Core (`dialog_uniffi/`)

**Purpose:** State management and business logic

- `dialog.udl` - UniFFI interface definition
- `lib.rs` - Main implementation with global Tokio runtime
- `models.rs` - Shared data structures
- `mock_data.rs` - Mock data for development

**Key Features:**
- Global Tokio runtime for async work
- In-memory state with fast synchronous reads (`try_read()`)
- Broadcast channel for push updates

### 2. Swift Package (`ios/DialogPackage/`)

**Purpose:** Clean interface between Rust and iOS app

- `Package.swift` - Swift Package Manager configuration
- `Sources/Dialog/` - Generated UniFFI bindings
- `XCFrameworks/` - Binary Rust libraries with C headers

**Critical Details:**
- `module.modulemap` (not `dialogFFI.modulemap`) for proper C module exposure
- Binary target approach avoids bridging header complexity
- ARM64-only builds (x86_64 excluded)

### 3. iOS App (`ios/`)

**Purpose:** SwiftUI interface

- `DialogApp/Sources/` - Swift UI code
- `project.yml` - XcodeGen configuration using Swift Package
- No generated files mixed into app source

**Architecture:**
- ViewModels use fire-and-forget calls to Rust
- Callbacks update UI via `@MainActor`
- No Swift concurrency complexity

## Why This Works

### Avoids Swift 6 Concurrency Issues
- No `async`/`await` means no Sendable requirements
- No Task boundaries to cross with non-Sendable types
- Callbacks handle threading explicitly and safely
- Client can remain non-Sendable without issues

### Clean Integration via Swift Package
- Binary target handles C type exports automatically
- No bridging headers or module maps needed in app
- Clean separation between generated and app code
- Easy to regenerate without breaking the app

### Performance Benefits
- Fast synchronous queries from in-memory state
- Async work happens in Rust (optimal)
- Minimal Swift overhead
- Push updates eliminate polling

## Build Pipeline

```
Rust Code Changes
       ↓
   cargo build (3 architectures)
       ↓
   uniffi-bindgen (Swift + C header)
       ↓
   XCFramework creation (versioned)
       ↓
   xcodegen generate (optional)
       ↓
   xcodebuild (iOS app)
```

All automated via `./rebuild.sh`

## Comparison to Failed Approaches

| Approach | Problem | Solution |
|----------|---------|----------|
| Async DialogClient | Swift 6 Sendable issues | Synchronous fire-and-forget |
| Direct generated files | Module import failures | Swift Package with binary target |
| Bridging headers | Complex configuration | XCFramework with module.modulemap |
| x86_64 support | Architecture mismatches | ARM64-only builds |

## Future Extensions

The architecture is ready for:
- Real backend integration (replace mock data)
- Additional UniFFI interfaces
- More complex state management
- Background sync capabilities

## References

- [UniFFI Documentation](https://mozilla.github.io/uniffi-rs/)
- [Swift Package Binary Targets](https://developer.apple.com/documentation/xcode/distributing-binary-frameworks-as-swift-packages)
- Implementation details in `docs/archive/ASYNC_PROBLEMS.md`
