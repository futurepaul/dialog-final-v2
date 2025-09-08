# Dialog CLI

A privacy-first note-taking CLI using Nostr with NIP-44 encryption.

## Configuration

The CLI is configured entirely through environment variables:

```bash
# Required: Your Nostr private key
export DIALOG_NSEC=nsec1...

# Optional: Custom relay URL (default: ws://localhost:10548)
export DIALOG_RELAY=wss://relay.damus.io

# Optional: Custom data directory (default: OS-specific)
export DIALOG_DATA_DIR=/path/to/data
```

## Usage

### Create a note
```bash
dialog_cli create "My note with #tags"
```

### List notes
```bash
# List recent notes
dialog_cli list

# List more notes
dialog_cli list --limit 20

# Filter by tag
dialog_cli list --tag myproject

# Watch for new notes in real-time
dialog_cli list --watch

# Watch for notes with specific tag
dialog_cli list --watch --tag important
```

### Show your public key
```bash
dialog_cli pubkey
```

### Override relay per-command
```bash
dialog_cli --relay wss://nos.lol create "Note to different relay"
```

## Features

- **Privacy-first**: All notes are encrypted with NIP-44
- **Offline-first**: Works without internet using local NdbDatabase
- **Real-time subscriptions**: Watch mode for streaming new notes
- **Tag filtering**: Organize notes with hashtags
- **Fully configurable**: No config files, just environment variables

## Setting Up Local Relay (Optional)

For testing without a public relay, you can use a local nak relay with GiftWrap support:

```bash
# From the project root, setup nak with our GiftWrap patch
./setup_nak_local.sh

# Run local relay on port 10548
~/go/bin/nak serve --port 10548

# In another terminal, use the CLI with local relay
export DIALOG_RELAY=ws://localhost:10548
dialog create "Testing with local relay"
```

Alternatively, install standard nak (without GiftWrap support):
```bash
go install github.com/fiatjaf/nak@latest
nak serve --port 10548
```

## Building

```bash
cd dialog_cli
cargo build --release
```

The binary will be at `target/release/dialog_cli`