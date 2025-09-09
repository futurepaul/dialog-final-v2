# FAILING_TO_BUILD.md - Complete Analysis of UniFFI Build Issues

## The Core Problem
We integrated `dialog_lib` (working Rust Nostr implementation) with `dialog_uniffi` (iOS UniFFI wrapper that previously worked with mock data). After this integration, the iOS build fails with undefined symbols for `connect_relay` method, despite the method appearing to be properly defined.

## What Was Working Before
- dialog_uniffi with mock data compiled and ran on iOS perfectly
- All methods were properly exported via UniFFI
- The iOS app could call all the mock methods without issues

## What Changed
1. Added `dialog_lib` as a dependency to `dialog_uniffi`
2. Added a new method `connect_relay` to DialogClient
3. Replaced mock implementations with real `dialog_lib` calls
4. Updated from UniFFI 0.28 to 0.29
5. Changed edition from 2021 to 2024

## The Specific Error
```
Undefined symbols for architecture arm64:
  "_uniffi_dialog_uniffi_checksum_method_dialogclient_connect_relay", referenced from:
      closure #1 () -> Dialog.(InitializationResult...) in Dialog.o
  "_uniffi_dialog_uniffi_fn_method_dialogclient_connect_relay", referenced from:
      closure #1 (Swift.UnsafeMutablePointer<__C.RustCallStatus>) -> () in Dialog.DialogClient.connectRelay(relayUrl: Swift.String) -> () in Dialog.o
```

## Critical Discovery: The Symbol Isn't Being Generated

When I run `nm` on the built library, I can't find the connect_relay symbols:
```bash
# This returns nothing:
nm target/aarch64-apple-ios-sim/release/libdialog_uniffi.a | grep "uniffi.*connect_relay"

# But other methods ARE there:
nm target/aarch64-apple-ios-sim/release/libdialog_uniffi.a | grep "uniffi.*send_command"
# Returns: _uniffi_dialog_uniffi_fn_method_dialogclient_send_command
```

## The connect_relay Implementation

In `dialog_uniffi/src/lib.rs`:
```rust
impl DialogClient {
    // ... constructor at line 40

    pub fn connect_relay(self: Arc<Self>, relay_url: String) {
        rt().spawn(async move {
            if let Err(e) = DIALOG.get().unwrap().connect_relay(&relay_url).await {
                eprintln!("Failed to connect to relay: {}", e);
            }
        });
    }
    
    // This method DOES get exported properly:
    pub fn send_command(self: Arc<Self>, cmd: Command) {
        // Fire-and-forget: spawn work on Tokio runtime
        let self_clone = self.clone();
        rt().spawn(async move {
            // ...
        });
    }
}
```

In `dialog_uniffi/src/dialog.udl`:
```
interface DialogClient {
    constructor(string nsec);
    
    // Connect to relay (optional)
    void connect_relay(string relay_url);
    
    // Fire-and-forget: spawns listener on background thread
    void start(DialogListener listener);
    void stop();
    
    // Fire-and-forget: spawns work on Tokio runtime
    void send_command(Command cmd);
    
    // ... other methods
}
```

## Things I've Verified

1. **The method is inside the impl block**: Line 78 is between lines 39-292 of the impl block
2. **The UDL matches the implementation**: Both have the same signature
3. **The generated Swift code includes it**: The Swift bindings in DialogPackage/Sources/Dialog/dialog.swift have:
   ```swift
   open func connectRelay(relayUrl: String) {try! rustCall() {
       uniffi_dialog_uniffi_fn_method_dialogclient_connect_relay(self.uniffiClonePointer(),
           FfiConverterString.lower(relayUrl),$0
       )
   }}
   ```

4. **The scaffolding is generated**: In the build output directory, the generated uniffi code includes references to connect_relay

## What I've Tried and Failed

1. **Changed method signature from `Arc<Self>` to `&self`** - This compiled but didn't fix the export issue
2. **Reverted back to `Arc<Self>`** - Still not exported
3. **Updated UniFFI from 0.28 to 0.29** - Fixed edition 2024 compatibility but didn't fix the export
4. **Clean builds multiple times** - No effect
5. **Manually copied libraries to XCFramework** - The symbols still weren't in the source library
6. **Checked the generated scaffolding** - It has checksums but not the actual implementation function

## Suspicious Findings

1. **The generated scaffolding has the checksum but not the implementation**:
   ```bash
   grep "uniffi_dialog_uniffi_fn_method_dialogclient_connect_relay" \
     target/aarch64-apple-ios-sim/release/build/dialog_uniffi-*/out/dialog.uniffi.rs
   # Returns: nothing
   
   grep "uniffi_dialog_uniffi_checksum_method_dialogclient_connect_relay" \
     target/aarch64-apple-ios-sim/release/build/dialog_uniffi-*/out/dialog.uniffi.rs  
   # Returns: pub extern "C" fn r#uniffi_dialog_uniffi_checksum_method_dialogclient_connect_relay() -> u16 {
   ```

2. **Library size changes**: The old mock-only library was ~5.9MB, the new one with dialog_lib is ~41MB

3. **Inconsistent symbols across builds**: Sometimes I see Rust internal symbols for connect_relay (like `__ZN13dialog_uniffi12DialogClient13connect_relay...`) but never the uniffi export

## Current State of Key Files

### dialog_uniffi/Cargo.toml
```toml
[package]
name = "dialog_uniffi"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["staticlib", "cdylib"]
name = "dialog_uniffi"

[dependencies]
dialog_lib = { path = "../dialog_lib" }
nostr-sdk = { workspace = true }
uniffi = { workspace = true }
uniffi_bindgen = { workspace = true, optional = true }
camino = { version = "1", optional = true }
tokio = { workspace = true }
once_cell = { workspace = true }
chrono = { workspace = true }
serde_json = { workspace = true }
uuid = { workspace = true }

[features]
bindgen-support = ["uniffi_bindgen", "camino"]

[build-dependencies]
uniffi_build = { workspace = true }
```

### Workspace Cargo.toml
```toml
[workspace]
members = ["dialog_lib", "dialog_uniffi", "dialog_cli"]
resolver = "2"

[workspace.dependencies]
nostr-sdk = { version = "0.37", features = ["ndb", "nip44"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros", "time", "sync"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
chrono = "0.4"
directories = "5"
uniffi = { version = "0.29", features = ["tokio"] }
uniffi_bindgen = { version = "0.29" }
uniffi_build = { version = "0.29" }
once_cell = "1"
uuid = { version = "1.11", features = ["v4", "serde"] }
```

### dialog_uniffi/build.rs
```rust
fn main() {
    uniffi_build::generate_scaffolding("./src/dialog.udl").unwrap();
}
```

### Build Script (build-uniffi-package.sh)
```bash
#!/bin/bash
set -e

echo "🦀 Building UniFFI for Swift Package..."

cd dialog_uniffi

# Set deployment targets for iOS 18.4+ support (required by nostrdb)
export IPHONEOS_DEPLOYMENT_TARGET=18.4
export MACOSX_DEPLOYMENT_TARGET=14.0

# Build for iOS architectures
echo "📱 Building for iOS device (arm64)..."
cargo build --release --target aarch64-apple-ios

echo "📱 Building for iOS simulator (arm64)..."
cargo build --release --target aarch64-apple-ios-sim

echo "💻 Building for macOS (arm64)..."
cargo build --release --target aarch64-apple-darwin

# Clean previous generated files
rm -rf ../DialogPackage/Sources/Dialog/*.swift
rm -rf ../DialogPackage/XCFrameworks/*.xcframework

# Generate Swift bindings into the package
echo "🔨 Generating Swift bindings..."
cargo run --features bindgen-support --bin uniffi-bindgen generate \
    --library target/aarch64-apple-ios/release/libdialog_uniffi.a \
    --language swift \
    --out-dir ../DialogPackage/Sources/Dialog \
    src/dialog.udl

# ... rest of XCFramework creation
```

## Theories About What's Wrong

1. **UniFFI isn't processing connect_relay during scaffolding generation**: The checksum is generated but not the actual FFI function. This could be a bug in how UniFFI handles certain method patterns.

2. **The method was added after the initial impl block was parsed**: Maybe there's some caching or incremental compilation issue.

3. **There's something specific about the connect_relay signature that UniFFI doesn't like**: But it's identical to other working methods like `send_command`.

4. **The nostrdb C dependency is interfering**: The library requires iOS 18.4+ and has caused other linking issues. Maybe it's corrupting the symbol table somehow.

5. **Edition 2024 changed something about how symbols are exported**: Though other methods still work fine.

## The Nuclear Option: SQLite

You suggested switching from nostrdb to SQLite to avoid the C dependency issues. This would:
- Remove the iOS 18.4+ requirement
- Eliminate the `___chkstk_darwin` undefined symbol issues we had
- Potentially fix whatever is preventing proper symbol export
- Make the library more portable

## What We Need Help With

1. Why is UniFFI generating a checksum for `connect_relay` but not the actual FFI function?
2. Is there a way to debug UniFFI's scaffolding generation to see why it's skipping this method?
3. Are we hitting some edge case in UniFFI with our specific setup?
4. Would switching to SQLite actually help, or is this a fundamental UniFFI issue?

## Reproduction Steps

1. Clone the repo at current state
2. Run `cargo build --release --target aarch64-apple-ios-sim -p dialog_uniffi`
3. Check symbols: `nm target/aarch64-apple-ios-sim/release/libdialog_uniffi.a | grep uniffi.*connect`
4. Notice that connect_relay symbols are missing while other methods like send_command are present

## The Frustrating Part

This SHOULD work. The method is defined exactly like the others. The UDL matches. The scaffolding generates a checksum. The Swift bindings are generated. But the actual FFI export function just... isn't there.