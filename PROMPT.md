I want to make a note to self ios app. I have a working UI for this in /Users/futurepaul/dev/heavy/fragment/ios

But we're going to use a new tech stack for storing the notes:
rust nostr using nip44 encryption for dms
https://rust-nostr.org/sdk/nips/44.html
https://github.com/rust-nostr/nostr

We'll use the nostrdb db in rust nostr for storage
https://github.com/rust-nostr/nostr/tree/master/database/nostr-ndb

(so we get offline functionality for free, with negentropy for sync)

Here's a full example of integrating nostrdb:
https://github.com/rust-nostr/nostr/blob/master/crates/nostr-sdk/examples/nostrdb.rs

Phase 1 of the project is to make a dialog_lib that we can test locally using a local relay running on ws://localhost:10547

It should:
allow you to create a new note to self (a self-dm using nip-44, so encrypted to self) with text in it
allow you to add topic tags (non-exclusive)
list all your notes
list notes by tag

the lib should handle storage, both on a desktop computer (getting their home dir and storing in there) and on mobile.
It should use the appropriate secrets storage on desktop and mobile for the user's nsec

Once we're sure dialog_lib is rock-solid we'll make a dialog_cli to exercise it on desktop.

The user should be able to run `dialog "this is my not to self" #tag1 #tag2` and the note gets stored to the local nostrdb and published to the relay
`dialog list` should show the last 10 notes to self, `dialog list -l 1` should show the most recent note, and `dialog list -l 2 #tag2` should show the last two notes with the `#tag2` tag. `dialog list --watch` should print the last 10 notes and stay open and stream notes as they come in (using normal nostr subscribe stuff). All these commands should compose well.

Once the cli is rock solid, we'll make a `dialog_uniffi` wrapper for exporting the types to swift. Then we'll take the existing Fragment ios UI and make it instead work with the dialog_lib which we'll import as a package.

We'll use xcodegenerate to create the xcodepackage. We'll need a build-ios.sh script and a clean-ios.sh script because xcode is really bad at handling updates. We'll target ios 18.

Create a detailed plan in PLAN_BY_CLAUDE.md. Make sure each phase is self-contained and iteratively testable. Put an emphasis on the SIMPLEST THING THAT CAN POSSIBLY WORK. No file should be bigger that 300 lines. SIMPLICTY IS THE MOST IMPORTANT FEATURE.

Ask me at least five clarifying questions about the project before writing the plan.
