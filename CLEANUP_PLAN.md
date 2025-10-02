# Cleanup & Simplification Plan

## Guiding Principles
- Run Negentropy (or equivalent bulk sync) only for cold-start/wake catch-up; never for the live tail.
- Drive live updates through long-lived NIP-01 subscriptions with relay filters, no timeouts, no poll/fetch loops.
- Treat the local database as the single source of truth; avoid optimistic UI paths outside the persisted state.

## Recommendations
- **Unify sync orchestration inside `dialog_lib`** — Complexity: High
  Create a `SyncCoordinator` that exposes `initial_catchup()` and `start_live_stream()` (returns a stream handle) so both CLI and UniFFI share the same contract. Handle Negentropy capability probing once and memoize relay state.

- **Split Negentropy catch-up from live subscribe** — Complexity: Medium
  Add an explicit `perform_initial_sync()` that blocks until Negentropy completes (or falls back to a bounded fetch). On success, record the latest event timestamp/id so the live subscriber can anchor `since` filters correctly.

- **Rethink `watch_notes` implementation** — Complexity: Medium
  Replace the current `since(Timestamp::now())` logic with `since(last_seen_created_at)` pulled from the DB. Ensure the live loop auto-resubscribes on disconnect and persists relay events before emitting them to consumers.

- **Expose DB-driven change feed** — Complexity: Medium
  Provide a lightweight notification channel (e.g., wrapping `nostr_sdk` database change hooks or a `tokio::watch`) so higher layers re-query the DB instead of shipping decrypted payloads through bespoke structs.

- **Refactor CLI watch/list flows** — Complexity: Medium
  Update the CLI to call `initial_catchup()` once, then attach to the new change feed. Remove direct decryption/printing paths in the watch loop; re-query the DB on each notification so the terminal always reflects persisted state.

- **Simplify UniFFI `DialogClient` state** — Complexity: High
  Drop the in-memory `HashMap` cache and emit UI updates by querying the DB via shared helpers. Convert the Swift listener flow to consume batched DB snapshots instead of optimistic inserts.

- **Standardize command processing & error surfacing** — Complexity: Medium
  Introduce structured errors/events (e.g., `SyncStatusChanged`, `RelayStatusChanged`) produced by the coordinator so both CLI and Swift can display consistent status without custom logging heuristics.

- **Add integration tests for sync modes** — Complexity: Medium
  Write async tests that spin up a relay fixture, run `initial_catchup()` + live stream, publish events, and assert that both CLI-style and UniFFI-style consumers see updates only after DB persistence.

