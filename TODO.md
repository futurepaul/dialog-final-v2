# TODO — Implementation Plan

This is a working checklist derived from FUTURE_PLANS.md. I’ll implement as much as possible end-to-end (Rust lib, UniFFI, CLI, iOS bindings), committing iteratively. Anything blocked or needing your input lands in NEED_PAUL.md.

## iOS Setup & Settings Support (UniFFI-facing)
- [x] Expose `validate_nsec(nsec) -> bool` via UniFFI
- [x] Expose `derive_npub(nsec) -> String` via UniFFI (for confirmation UI)
- [x] Expose `clear_data_for_current_pubkey()` via UniFFI (wraps `clean_test_storage`)
- [x] Add `get_tag_counts()` via UniFFI (sequence of `{tag,count}` for Tag Switcher)
- [x] Add `SyncMode` toggle in UniFFI + handler in `ConnectRelay`

## Nostr sync modes
- [x] Env toggle `DIALOG_SYNC_MODE={negentropy|subscribe}` (fallback if UniFFI toggle not used)
- [x] If `negentropy`: call `Dialog::sync_notes()` then ensure watch loop running
- [x] If `subscribe`: skip negentropy; rely on `watch_notes()` and local DB, load current cache

## Read/Unread & Sync status
- [x] Ensure `is_synced=true` when relay echo arrives (already implied via watch update)
- [x] Keep `mark_as_read` flow (already in wrapper) and UI can query counts

## Refactor (dialog_uniffi)
- [ ] Non-breaking split: move conversion helpers and watch loop into modules (`convert.rs`, `watch.rs`), keep public API stable
- [ ] Leave deeper split (`runtime/state/events/commands`) for a follow-up after shipping API changes

## CLI/QOL
- [ ] Ensure CLI prints config with default relay and data dir
- [ ] No functional CLI changes required now

## Build & Test
- [x] `cargo check` / `cargo clippy` clean
- [~] Unit tests pass (`cargo test`) — integration test requires `DIALOG_NSEC_TEST` env; unit tests pass; see NEED_PAUL.md
- [x] Rebuild UniFFI package (`just package`)
- [x] iOS compiles (`just ios-fast`) — packages resolved and XCFramework updated

## Stretch (time permitting)
- [ ] Add `mark_all_read()` in UniFFI + Swift hook
- [ ] Basic iOS Settings scaffold (Swift) to call new UniFFI APIs
