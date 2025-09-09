# Swift Async & UniFFI Integration Issues

## Current Situation

We have a Rust crate (`dialog_uniffi`) that successfully compiles and generates UniFFI bindings. The iOS app is trying to use these bindings but we're hitting Swift 6 concurrency issues and module import problems.

### Architecture Goals
- **Rust owns all state** via `dialog_uniffi` crate
- **Push-based updates** from Rust to Swift via callbacks (`DialogListener`)
- **Swift is just UI** - sends commands to Rust, renders what Rust tells it
- **Non-blocking** - all operations async/fire-and-forget

## The Solution: Lean Pattern + Swift Package

After analysis, we're adopting a **simpler, synchronous pattern** that avoids Swift 6 concurrency issues entirely, combined with proper Swift Package management for clean integration.

### What's Working
✅ Rust crate compiles with UniFFI bindings
✅ XCFramework generated successfully
✅ Mock data implemented in Rust
✅ Push-based event system designed in Rust

### What's Not Working
❌ Swift can't properly import the UniFFI-generated types
❌ Swift 6 strict concurrency checking with non-Sendable `DialogClient`
❌ Module/bridging header configuration issues
❌ Mixing generated code directly in the app instead of as a package

## Problems Encountered

### 1. Module Import Issues
- Generated `dialog.swift` expects to import types like `RustBuffer`, `ForeignBytes`, `RustCallStatus`
- These are defined in `dialogFFI.h` but Swift can't find them
- Tried: Adding bridging header - didn't resolve the import issues
- Tried: Module map configuration - types still not found
- Tried: Various XcodeGen settings for include paths - incomplete solution

### 2. Swift 6 Concurrency Issues
The generated `DialogClient` is not `Sendable`, leading to data race warnings when:
- Calling async methods from different actor contexts
- Passing the client between MainActor and Task contexts
- Using callbacks that cross actor boundaries

#### Attempts That Failed:

**Attempt 1: @MainActor on entire ViewModel**
```swift
@MainActor
class InboxViewModel: ObservableObject {
    private let client: DialogClient
    // ...
    Task {
        await client.start(listener: listener) // ❌ "sending 'self.client' risks causing data races"
    }
}
```
Problem: Even with @MainActor, can't safely use non-Sendable types across Task boundaries

**Attempt 2: Selective @MainActor on properties**
```swift
class InboxViewModel: ObservableObject {
    @MainActor @Published var notes: [Note] = []
    private let client: DialogClient
    // ...
    Task {
        await client.sendCommand(cmd: .createNote(text: trimmed)) // ❌ Still data race warnings
    }
}
```
Problem: Doesn't solve the fundamental issue of DialogClient not being Sendable

**Attempt 3: Capturing references before Task**
```swift
let clientRef = client
Task {
    await clientRef.start(listener: listener) // ❌ "passing closure as 'sending' parameter"
}
```
Problem: Just moves the error to a different place

**Attempt 4: Actor wrapper**
```swift
actor DialogClientActor {
    private let client: DialogClient
    // ...
}
```
Problem: Still have to pass non-Sendable types to the actor, and listener callbacks have similar issues

### 3. XcodeGen Configuration Chaos
Currently we have:
- Generated Swift files mixed into the app sources
- XCFramework as a dependency
- Bridging header attempts
- Module map files that aren't being processed correctly
- Various build settings that may or may not be needed

## The New Approach

### 1. Lean, Synchronous UniFFI Pattern

Instead of fighting Swift 6 concurrency with async/await, we adopt a **fire-and-forget pattern**:

**Key Changes:**
- All Rust methods are **synchronous** (non-blocking) - they spawn work internally
- Swift calls are simple method invocations - no `Task`, no `await`
- Callbacks explicitly handle thread boundaries with `Task { @MainActor in ... }`
- Client remains non-Sendable but it doesn't matter since we don't pass it around

**Example UDL:**
```udl
interface Client {
  constructor(string db_path);
  
  // Non-blocking methods (spawn work internally)
  void start(Listener l);
  void post(string body);
  
  // Fast snapshot from memory
  sequence<Message> list(u32 limit);
};

callback interface Listener {
  void on_event(Event e);
};
```

**Swift Usage:**
```swift
// No async/await needed!
client.start(self)  // Fire and forget
client.post(body: text)  // Non-blocking

// Callback handles threading
func onEvent(_ e: Event) {
  Task { @MainActor in 
    // Update UI safely
    self.messages.append(e.message)
  }
}
```

This pattern **completely sidesteps** the Swift 6 issues we encountered.

### 2. Swift Package Setup (Clean Integration)

Instead of mixing generated files into the app, create a **proper Swift Package**:

**Package Structure:**
```
DialogPackage/
├── Package.swift
├── Sources/
│   └── Dialog/
│       ├── dialog.swift (generated)
│       └── Exports.swift  
└── XCFrameworks/
    └── dialogFFI.xcframework/
```

**Package.swift:**
```swift
// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "DialogPackage",
    platforms: [.iOS(.v16)],
    products: [
        .library(
            name: "Dialog",
            targets: ["Dialog", "dialogFFI"]
        )
    ],
    targets: [
        .target(
            name: "Dialog",
            dependencies: ["dialogFFI"]
        ),
        .binaryTarget(
            name: "dialogFFI",
            path: "XCFrameworks/dialogFFI.xcframework"
        )
    ]
)
```

**Benefits:**
- No bridging headers needed
- No module map complexity
- Clean separation of generated code
- Easy to update/regenerate

### 3. Build Script Updates

Modify the build script to generate a Swift Package:

```bash
#!/bin/bash
# Generate bindings and XCFramework
cargo build --release
uniffi-bindgen generate --library target/release/libdialog.dylib --language swift --out-dir swift-package/Sources/Dialog

# Copy XCFramework to package
cp -r build/dialogFFI.xcframework swift-package/XCFrameworks/

# Package is ready to use as local dependency
```

## Implementation Plan

### Phase 1: Convert to Lean Pattern ✅
- [x] Adopt synchronous, fire-and-forget pattern
- [x] Move async work inside Rust (Tokio runtime)
- [x] Use callbacks with explicit `@MainActor` transitions

### Phase 2: Swift Package Setup ✅
- [x] Create Swift Package structure
- [x] Update build script to generate into package
- [x] Configure Package.swift with binary target
- [x] Test package imports work correctly

### Phase 3: Update Rust Implementation ✅
- [x] Convert DialogClient methods to synchronous (non-blocking)
- [x] Set up global Tokio runtime for spawning work
- [x] Implement in-memory snapshots for `list()` operations
- [x] Update callback interface to use simple events

### Phase 4: Clean up iOS Project ✅
- [x] Remove all generated files from iOS app
- [x] Remove bridging header and module maps
- [x] Simplify XcodeGen configuration  
- [x] Add Swift Package as local dependency

### Phase 5: Update Swift ViewModels ✅
- [x] Remove all `Task` and `await` from client calls
- [x] Implement callback with `Task { @MainActor in ... }`
- [x] Update to use fire-and-forget pattern
- [x] Test with Swift 6 strict concurrency enabled

## Why This Approach Works

### Avoiding Swift 6 Concurrency Issues
The synchronous, fire-and-forget pattern completely avoids the problems we encountered:
- No `async`/`await` means no Sendable requirements
- No Task boundaries to cross with non-Sendable types
- Callbacks handle threading explicitly and safely
- Client can remain non-Sendable without issues

### Clean Integration via Swift Package
Using a proper Swift Package solves all our module import issues:
- Binary target handles C type exports automatically
- No bridging headers or module maps needed
- Clean separation between generated and app code
- Easy to regenerate without breaking the app

### Inspired By
This approach takes inspiration from successful UniFFI projects like the `rust-multiplatform` crate shown above, which uses:
- Simple synchronous interfaces
- Callbacks for push updates
- Global statics for state management
- Clean FFI boundaries

## Key Insights

1. **Async isn't always necessary** - Fire-and-forget with callbacks is simpler and avoids Swift 6 issues
2. **Swift Packages are the right abstraction** - They handle module complexity for us
3. **Keep the boundary simple** - Complex async patterns across FFI add unnecessary complexity
4. **Let Rust handle concurrency** - Tokio inside, simple interface outside

## Lessons Learned

- The simplest pattern is often the best one
- Don't fight the platform - use its native packaging system
- Swift 6 concurrency checking exposed our over-complex design
- Generated code needs proper packaging from the start