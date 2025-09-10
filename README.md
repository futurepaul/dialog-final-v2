# Dialog: Rust + UniFFI + iOS

End-to-end note-to-self app backed by Nostr (nip44 encryption) with a Rust core, a Swift/iOS app via UniFFI, and a CLI for local testing.

## Requirements
- macOS with Xcode 15+
- Rust toolchain with iOS targets: `aarch64-apple-ios`, `aarch64-apple-ios-sim`, `aarch64-apple-darwin`
- Optional: XcodeGen (`brew install xcodegen`)

## Quick Start

CLI
- Set env and run:
  - `export DIALOG_NSEC=nsec1...`
  - `export DIALOG_RELAY=wss://relay.damus.io`
  - `just build` then `./target/release/dialog_cli --print-config`
  - Create: `./target/release/dialog_cli create "Hello #test"`
  - Watch: `./target/release/dialog_cli list --watch`

iOS App
- Build package + app: `just ios` (or `just ios-fast` for quicker rebuilds)
- The app connects to `wss://relay.damus.io`, syncs, and starts live watch.

Relay Support
- The project currently requires relays with Negentropy (NDB) support for sync.
- Default relay: `wss://relay.damus.io` (supports Negentropy).
- Local testing: run a nak relay with our Negentropy patch:
  - `./setup_nak_local.sh` (places `./nak-negentropy` in the repo root)
  - `./nak-negentropy serve --port 10548`
  - Set `DIALOG_RELAY=ws://localhost:10548`

## Useful Commands
- `just package` — regenerate Swift bindings + XCFramework (auto-bumps version)
- `just package-fast` — regenerate without rebuilding Rust
- `just clean-ios` — remove generated Swift + XCFramework (keep Rust target cache)
- `just check` — format and clippy

## Project Structure
- `dialog_lib/` — Rust core (Nostr client, storage, sync)
- `dialog_uniffi/` — UniFFI wrapper exposed to Swift
- `dialog_cli/` — CLI on top of `dialog_lib`
- `ios/DialogPackage/` — Swift Package (generated bindings + XCFramework)
- `ios/` — SwiftUI app (consumes `Dialog` package)

## Troubleshooting
- If the app shows old bindings: `just clean-ios && just ios-fast`
- If Swift logs appear but not UniFFI logs: rebuild package (above); Xcode → Clean Build Folder.
- For local relays (`ws://localhost:10548`), add ATS exceptions; the app defaults to `wss://relay.damus.io`.

See `docs/BUILD_GUIDE.md` for deeper build details and `docs/ARCHITECTURE.md` for design.
