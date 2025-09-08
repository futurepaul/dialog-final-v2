## Dialog: Plan by GPT-5

Assumptions confirmed by you:
- NIP-44 v2 for self-DM encryption.
- Tags: parse `#tag` in content → emit `t` tags with value `tag`.
- Secrets: use Keychain on macOS/iOS; require auth once per process/app session, then cache in-memory.
- Storage paths: macOS `~/Library/Application Support/dialog`, Linux `~/.local/share/dialog`, iOS app sandbox.
- Dev relay: `ws://localhost:10547` (nak, supports negentropy). No auth initially.
- If relay down: ALWAYS save to local nostrdb, warn on sync failure.
- Watch: print last N, then stream new self-DMs from self (filtered by tags if provided).
- iOS target: iOS 18.
- Scope: only self → self DMs to our pubkey for now.

Design values:
- Simplicity first. The simplest thing that can possibly work.
- Each file ≤ 300 lines.
- Phases are self-contained and shippable; add value each step.

---

## Phase 1: dialog_lib (Rust)
Goal: A minimal Rust library that can create/list/watch self-notes using rust-nostr + nostr-ndb with NIP-44 v2. Works offline and syncs via a relay when available.

Crate layout (each file stays small):
- `dialog_lib/`
  - `src/lib.rs` (public API surface)
  - `src/config.rs` (paths, relay cfg)
  - `src/keys.rs` (key mgmt + Keychain integration behind feature flags)
  - `src/db.rs` (nostr-ndb wrapper)
  - `src/crypto.rs` (NIP-44 v2 helpers)
  - `src/model.rs` (Note struct, Tag)
  - `src/runtime.rs` (background tasks/subscriptions)

Public API (initial):
- `Dialog::new(opts: DialogOptions) -> Result<Self>`
- `set_relays(&self, urls: &[Url]) -> Result<()>`
- `create_note(&self, text: &str, tags: &[String]) -> Result<NoteId>`
- `list_notes(&self, limit: usize, tags: &[String]) -> Result<Vec<Note>>`
- `watch_notes(&self, limit: usize, tags: &[String]) -> WatchHandle` (iterator/callbacks)

Key points:
- Keys: generate/import `nsec`, store in Keychain. On process/app start, unlock once (Keychain prompt or password) → keep decrypted key in-memory until process exit. Use `kSecAttrAccessibleAfterFirstUnlock` on iOS/macOS; on CLI, cache in-process.
- DB: use `nostr-ndb` for local storage; write on create immediately (before network). Index by `created_at` and `t` tags.
- Crypto: use rust-nostr NIP-44 v2 helpers for encrypting DM to self (pubkey = own pubkey). Keep API narrow.
- Networking: Connect to `ws://localhost:10547` by default. Use negentropy sync when relay available; failures only warn.
- Tags: parse `#foo` → `t` tag `foo`. Also accept explicit tags in API.

Testing (local):
- Unit tests for tag parsing and model conversions.
- Integration test: start library with empty DB, create note (offline), assert present; then start relay, call a sync function, assert published.

Deliverables of Phase 1:
- `dialog_lib` crate with APIs above.
- Example `examples/smoke.rs` to print last 10 and watch.

---

## Phase 2: dialog_cli (Rust)
Goal: A tiny CLI that exercises the lib.

Commands:
- `dialog "this is my note to self" #tag1 #tag2`
  - Parses hashtags, saves to nostrdb, attempts publish; warns on network failure.
- `dialog list` → last 10
- `dialog list -l 1` → most recent 1
- `dialog list -l 2 #tag2` → last 2 filtered by `tag2`
- `dialog list --watch [-l N] [#tag ...]` → print last N then stream

Notes:
- Use `clap` for concise command parsing.
- Exit code 0 if local save succeeded. Nonzero only if local write fails.
- First-run key import: support `dialog key import nsec1...` and/or prompt to store in Keychain; cache decrypted key in-memory for the session.
- Config: macOS `~/Library/Application Support/dialog/config.toml`; Linux `~/.config/dialog/config.toml`; env override `DIALOG_RELAYS`.

Testing:
- Snapshot tests for `list` output, including `--watch` initial backlog format (stream smoke-checked).
- E2E: run against local nak relay up/down; verify offline-first save and warning-only on publish failure.

---

## Phase 3: dialog_uniffi (Rust + UniFFI)
Goal: Bind a minimal, stable surface to Swift.

Exported types (lean):
- `FfiDialog` (handle)
- `FfiNote { id, created_at, text, tags: Vec<String> }`

Exported funcs:
- `dialog_new(opts) -> FfiDialog`
- `dialog_create_note(text, tags) -> FfiNoteId`
- `dialog_list_notes(limit, tags) -> Vec<FfiNote>`
- `dialog_watch_notes(limit, tags, on_event: callback)`

Threading model:
- All I/O on a background runtime inside Rust.
- Callbacks marshalled onto Swift main thread by the Swift wrapper.

---

## Phase 4: iOS integration (SwiftUI)
Goal: Replace Core Data pipeline with a thin Swift wrapper around `dialog_uniffi` that is Observable.

Approach (simple and non-blocking):
- Create `DialogStore: ObservableObject` in Swift that:
  - Holds `FfiDialog` handle.
  - Exposes `@Published var notes: [UINote]`.
  - On init: calls `dialog_list_notes(limit: 50)`; then starts `dialog_watch_notes` with a callback updating `notes` on main thread.
  - Methods `create(text:String)` that call into Rust on a background queue.
- UI remains SwiftUI; remove Core Data models that are now redundant.
- Keychain: on first app foreground, prompt/unlock once; keep key in memory for the app session.

Why this is the simplest:
- No complex bridging of Combine across FFI; just a callback and `@Published`.
- Rust owns I/O; Swift owns rendering.

---

## Phase 5: Xcodegen + scripts
Goal: Reproducible iOS builds and easy clean.

- Use `xcodegen` to generate the project including the UniFFI-produced Swift module.
- Keep a minimal, versioned `project.yml` so generation is deterministic.
- Scripts:
  - `build-ios.sh`: clean, run `xcodegen`, generate UniFFI Swift bindings, build, and produce an XCFramework for `aarch64-apple-ios` and `x86_64-apple-ios-simulator`.
  - `clean-ios.sh`: remove derived data and all generated UniFFI bindings and XCFramework artifacts.
- Target iOS 18 SDK; verify minimal deployment target compatible with your devices.

---

## Key implementation details

Secrets (one prompt per session):
- macOS CLI: use Keychain (e.g., `keyring` crate or `security-framework`). On first run, prompt user (or accept `nsec` import). Keep decrypted key in memory for process lifetime. Use a lock file or PID guard to avoid multiple prompts from concurrent subcommands.
- iOS: use Keychain with `kSecAttrAccessibleAfterFirstUnlock`. Prompt on first launch (or when importing `nsec`). Keep in-memory for app session; zeroize on background-termination.

Nostr specifics:
- Use rust-nostr NIP-44 v2 for encrypt/decrypt to self (pubkey = own).
- Use `t` tags for topics; store lowercase values; dedupe before emit; filter server-side and client-side.
- Use nostr-ndb for storage and negentropy sync via the relay; when offline, writes are local-only.

Error handling:
- Library returns structured errors; CLI only exits nonzero on local DB write failure; otherwise prints warnings.

Performance:
- Keep indices minimal (created_at, tags). Batch writes where available from nostr-ndb API. Avoid blocking UI.

File size discipline:
- Keep modules very small; prefer many tiny files. Break up before 300 lines.

---

## Phase-by-phase test plans

Phase 1 tests:
- Unit: tag parsing from content.
- Unit: self-DM encrypt/decrypt roundtrip.
- Integration: create offline → present in list; bring relay up → publish succeeds.

Phase 2 tests:
- CLI creates and lists notes; respects `-l` and tag filters.
- `--watch` prints last N then streams new ones.
- Relay down → save local, print warning, exit 0.

Phase 3 tests:
- Swift binding smoke test: create, list, receive callback.

Phase 4 tests:
- SwiftUI view renders notes and updates live; actions are non-blocking.

Phase 5 tests:
- `build-ios.sh` succeeds on a clean machine; `clean-ios.sh` restores to clean.

---

## Milestone checklist (SIMPLEST first)
1) Minimal `dialog_lib` with local-only create/list (no network).
2) Add NIP-44 v2 encrypt/decrypt to self; confirm persistence.
3) Add relay publish/subscribe; warn on failure.
4) Add tag parsing + filtering.
5) Ship `dialog_cli` basic commands.
6) UniFFI wrapper exposing create/list/watch.
7) Swift `DialogStore` + simple list UI.
8) Xcodegen + scripts.

---

## Open follow-ups (tracked as TODOs)
- Add optional relay auth later.
- Multi-relay support and prioritization.
- Export edit/delete if needed later (out of scope now).


