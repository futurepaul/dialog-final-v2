# PURE UI Plan

Goal: make the Swift UI a pure function of Rust/UniFFI state. Avoid ad‑hoc patching from views; state changes should happen in one place and views should render based on published state.

Principles
- Single source of truth: dialog_uniffi emits events; the ViewModel derives all Swift state from those + stable sync queries.
- Unidirectional flow: Views send commands; ViewModel mutates model (in Rust) and publishes new state; Views re-render.
- No state writes during `body`: Never set `@State` or `@Published` inside computed properties used by `body`.
- Derived data only: npub, counts, filtered lists are derived once and stored; no lazy state that mutates during render.

Short-term Tasks
- [x] Derive `npub` in ViewModel (not in View) and publish it.
- [x] Remove inline search; move to its own sheet to avoid list state coupling.
- [x] Stop mutating state from Settings text-field `.onChange`; use explicit actions (`onSubmit` / button) instead.
- [ ] Create an app-level `AppState` struct in ViewModel that mirrors UniFFI state (notes, tags, counts, filter, syncing).
- [ ] Centralize all event handling in ViewModel (`handleEvent`), and make Views read from published properties only.

Medium-term Tasks
- [ ] Move Settings relay connection into a dedicated intent method that also updates a `connectionStatus` published field.
- [ ] Add a small state machine for sync mode (`negentropy|subscribe`) and publish it for UI indicators.
- [ ] Ensure all note updates come from events or from a single post-command refresh; avoid in-place list mutations across views.

Longer-term Ideas
- [ ] Consider mirroring a minimal store of Dialog state in Swift (struct), updated by events, to simplify derived selectors.
- [ ] Add a thin test harness for ViewModel to validate event → state transformations deterministically.

