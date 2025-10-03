# Future Plans

This checklist outlines next steps with concrete implementation notes to avoid regressions and churn. It builds on the working CLI ↔ iOS ↔ relay pipeline and current CI.

- [ ] iOS setup screen
  - [ ] Input/create nsec
    - Store in iOS Keychain (per-bundle). Add a UniFFI call to validate nsec (reuse `dialog_lib::validate_nsec`). Use Keychain in production; keep Run‑scheme env for local dev only (no migration required).
    - Add QR scan + paste; show derived npub for confirmation.
    - Provide “Show nsec as QR” (explicit, gated by confirmation) to link additional devices. Warn clearly that nsec is secret.

- [ ] iOS settings screen
  - Entry point: from the Tag Switcher sheet/menu; show a "Settings" row pinned at the bottom below the tag list to keep the primary UI clean.
  - [ ] Configure relay
    - Persist in `UserDefaults` (key `DIALOG_RELAY`); default `wss://relay.damus.io`. Call `sendCommand(.connectRelay(...))` on change + resync.
  - [ ] Change nsec / nuke app data
    - Provide “Sign out” (wipe Keychain + in‑app cache). Add UniFFI method to clear data dir for current pubkey (wrap `clean_test_storage`). Require app restart.

- [ ] Investigate standard publish/subscribe (non‑Negentropy)
  - Keep `nostr-ndb` for storage; explore rust‑nostr’s standard publish/subscribe flows (no `sync_notes`) for relays without Negentropy.
  - Provide a runtime toggle (env or settings) between:
    - Negentropy sync (current path for supported relays), and
    - Plain subscribe (Filter by author/kind; incrementally update cache) for broader compatibility.
  - Review rust‑nostr examples to ensure we follow best practices for subscribe lifecycles and backoff.

- [ ] Get ready for TestFlight
  - [ ] Icon (AppIcon asset catalog), screenshots, short/long description.
  - Ensure ATS allows only TLS (no localhost). Hardcode default relay; hide local relay in release.
  - Set bundle id/team in `ios/project.yml`; add signing to CI only if later distributing.

- [ ] Marketing site
  - Simple static site in `site/` with Bun: minimal landing + links to TestFlight and CLI. GH Pages deploy (actions/static). Keep copy in README.

- [ ] MANUAL: Announce on Nostr
  - Post from project npub; include relay requirements (Negentropy for now) and quickstart.

- [ ] Nice-to-haves
  - [ ] Blossom image upload
  - [ ] Blossom voice memo upload
  - [ ] CLI hooks: `hooks/` with Bun scripts; watch tags and invoke on new notes (guard with opt‑in env).

- [ ] Search + Tag navigation
  - [ ] In‑app search UX: debounced text field above the list; show inline results with highlights; clear search resets to current tag filter.
  - [ ] Rust/UniFFI API: expose `search_notes(query)` returning notes from the full cache; keep simple substring match short‑term (already have `Command::SearchNotes`), plan upgrade to DB‑backed search (NDB query) or relay search if available.
  - [ ] Tap a note's tag to jump to that tag's view: in Swift, make tags tappable chips that call `sendCommand(.setTagFilter(tag: ...))` and dismiss the sheet if open.

- [ ] Read/Unread + Sync status
  - Display read/unread badges in the UI (list + detail) using existing fields (`Note.is_read`, `Note.is_synced`).
  - Ensure wrapper flips `is_synced=true` when a relay echo arrives (we already emit `NoteUpdated` on dedupe).
  - Mark as read on selection (already implemented) and propagate to cache + UI immediately; consider a bulk "mark all read" action.

- [ ] Fix tag counts (Switcher accuracy)
  - Centralize count logic in the wrapper (avoid view‑dependent calculations). Add a UniFFI method like `get_tag_counts() -> map<string,u32>` computed from the full cache.
  - Update Swift to use these counts regardless of current filter; refresh counts on `NotesLoaded`/`NoteAdded`/`NoteUpdated`/`NoteDeleted`.

- [ ] Code cleanup
  - Keep files < 300 LOC (run your local "vibe check" regularly).
  - Refactor `dialog_uniffi` into modules (no API change): `runtime.rs`, `state.rs`, `events.rs`, `commands.rs`, `watch.rs`, `convert.rs`. Preserve integration test; run `just package` → `just ios-fast` to validate.

## Guardrails
- Keep relay binary at repo root `./nak-negentropy`; tests start/stops it.
- Never hardcode nsecs. iOS → Keychain; CLI/CI via env.
- Packaging step auto‑bumps XCFramework and validates UniFFI checksums.
- CI: pre‑check `dialog_cli --print-config`; pass `DIALOG_NSEC_TEST` to all integration steps.
