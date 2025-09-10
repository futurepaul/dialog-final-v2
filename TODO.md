# TODO — Implementation Plan

This is a working checklist derived from FUTURE_PLANS.md. I’ll implement as much as possible end-to-end (Rust lib, UniFFI, CLI, iOS bindings), committing iteratively. Anything blocked or needing your input lands in NEED_PAUL.md.

## iOS Setup & Settings Support (UniFFI-facing)
- [ ] Expose `validate_nsec(nsec) -> bool` via UniFFI
- [ ] Expose `derive_npub(nsec) -> String` via UniFFI (for confirmation UI)
- [ ] Expose `clear_data_for_current_pubkey()` via UniFFI (wraps `clean_test_storage`)
- [ ] Add `get_tag_counts() -> Map<String,u32>` via UniFFI (for Tag Switcher counts)
- [ ] Add `SyncMode` toggle in UniFFI + handler in `ConnectRelay`

## Nostr sync modes
- [ ] Env toggle `DIALOG_SYNC_MODE={negentropy|subscribe}` (fallback if UniFFI toggle not used)
- [ ] If `negentropy`: call `Dialog::sync_notes()` then ensure watch loop running
- [ ] If `subscribe`: skip negentropy; rely on `watch_notes()` and local DB, load current cache

## Read/Unread & Sync status
- [ ] Ensure `is_synced=true` when relay echo arrives (already implied via watch update)
- [ ] Keep `mark_as_read` flow (already in wrapper) and UI can query counts

## Refactor (dialog_uniffi)
- [ ] Non-breaking split: move conversion helpers and watch loop into modules (`convert.rs`, `watch.rs`), keep public API stable
- [ ] Leave deeper split (`runtime/state/events/commands`) for a follow-up after shipping API changes

## CLI/QOL
- [ ] Ensure CLI prints config with default relay and data dir
- [ ] No functional CLI changes required now

## Build & Test
- [ ] `cargo check` / `cargo clippy` clean
- [ ] Unit tests pass (`cargo test`)
- [ ] Rebuild UniFFI package (`just package`)
- [ ] iOS compiles (`just ios-fast`)

## Stretch (time permitting)
- [ ] Add `mark_all_read()` in UniFFI + Swift hook
- [ ] Basic iOS Settings scaffold (Swift) to call new UniFFI APIs

