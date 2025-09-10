# NEED_PAUL — Questions / Inputs

I’m pushing forward on Rust + UniFFI APIs and safe defaults. Here’s what I need from you or where guidance unlocks more progress:

## Relays & Sync
- What should be the default relay for release builds? FUTURE_PLANS mentions `wss://relay.damus.io` — confirm?
- Do you want `DIALOG_SYNC_MODE` default to `negentropy` (current) and only use `subscribe` for non-Negentropy relays? Or auto-detect capability?
- Any specific additional relays we should try in tests?

## iOS Setup & Settings
- Keychain storage: okay to use standard `kSecClassGenericPassword` with service `app.bundle.id.dialog` and account `nsec`? Any specific accessibility class you prefer (e.g., `.afterFirstUnlock`)?
- QR code flows: prefer to scope inside Settings for now, or a dedicated Setup onboarding flow?
- Are we exposing the derived `npub` only for confirmation or also to copy/share?

## Tag counts & UX
- Counts should be global (all notes) regardless of current filter — confirm this is the desired behavior even when viewing a tag-specific feed?
- Any tags to hide or special groupings (e.g., pin Favorites)?

## TestFlight & ATS
- AppIcon assets and screenshots: do you have assets ready, or should I add placeholders to unblock builds only?
- ATS: okay to restrict to TLS-only and hide localhost in Release? Dev scheme can still target localhost.

## Blossom uploads (nice-to-have)
- Which Blossom endpoint(s) do you plan to use? Any auth scheme or capabilities we should assume up front?

## CI
- Should `dialog_cli --print-config` run in CI before builds using `DIALOG_NSEC_TEST`? If yes, confirm the env var name and value source.

## Anything else
- If you have preferences on the UniFFI API naming or where to place new Swift scaffolding files, let me know and I’ll align.

