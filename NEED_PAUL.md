# NEED_PAUL — Questions / Inputs

I’m pushing forward on Rust + UniFFI APIs and safe defaults. Here’s what I need from you or where guidance unlocks more progress:

## Relays & Sync
- What should be the default relay for release builds? FUTURE_PLANS mentions `wss://relay.damus.io` — confirm?
PAUL: confirm
- Do you want `DIALOG_SYNC_MODE` default to `negentropy` (current) and only use `subscribe` for non-Negentropy relays? Or auto-detect capability?
PAUL: we should auto-detect capability if we can. also it's not just for SEEING the notes that we have to change things, it's also for publlishing (currently our sync function does both... in normal nostr without negentropy you have to `publish` notes... see relevant examples inexamples/nostr)
- Any specific additional relays we should try in tests?
PAUL: let's put wss://nos.lol as a non-negentropy option

## iOS Setup & Settings
- Keychain storage: okay to use standard `kSecClassGenericPassword` with service `app.bundle.id.dialog` and account `nsec`? Any specific accessibility class you prefer (e.g., `.afterFirstUnlock`)?
PAUL: sounds good to me
- QR code flows: prefer to scope inside Settings for now, or a dedicated Setup onboarding flow?
PAUL: let's keep our screens minimal for now. so whatever results in the fewest screens do that.
- Are we exposing the derived `npub` only for confirmation or also to copy/share?
PAUL: it's nice to be able to copy every key and setting and even the text notes themselves! (with those it should be a press and hold context menu kind of thing) 

## Tag counts & UX
- Counts should be global (all notes) regardless of current filter — confirm this is the desired behavior even when viewing a tag-specific feed?
PAUL: confirm
- Any tags to hide or special groupings (e.g., pin Favorites)?
PAUL: no let's not complicate it for now

## TestFlight & ATS
- AppIcon assets and screenshots: do you have assets ready, or should I add placeholders to unblock builds only?
PAUL: this is blocked on me. skip for now
- ATS: okay to restrict to TLS-only and hide localhost in Release? Dev scheme can still target localhost.
PAUL: yes that makes sense

## Blossom uploads (nice-to-have)
- Which Blossom endpoint(s) do you plan to use? Any auth scheme or capabilities we should assume up front?
PAUL: this is blocked on me, ignore for now. I need to get a server deployed to test with

## CI
- Should `dialog_cli --print-config` run in CI before builds using `DIALOG_NSEC_TEST`? If yes, confirm the env var name and value source.
PAUL: isn't that what we currently do? tbh that was just for debugging some stuff we don't need it anymore

## Anything else
- If you have preferences on the UniFFI API naming or where to place new Swift scaffolding files, let me know and I’ll align.

