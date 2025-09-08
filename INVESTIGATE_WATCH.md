# Investigation: Watch Mode Not Receiving Events

## The Problem
- `list` command works perfectly - pulls from local NdbDatabase
- `watch` command shows existing notes but doesn't receive new streaming events
- We know events ARE hitting the relay (user confirmed seeing them in nak server logs)

## What We Know Works
1. Creating notes works - they get sent to relay
2. Listing notes works - they're in the local database
3. The relay IS receiving our events (visible in server logs)
4. Database persistence works across restarts

## What's Not Working
- Subscription isn't receiving new events from the relay
- Even though we call `client.subscribe()`, the notification stream isn't getting events

## Questions to Answer

### 1. How does nostr-sdk handle subscriptions?
- Does `client.subscribe()` automatically handle the WebSocket connection?
- Do we need to explicitly connect/reconnect to the relay?
- Is there a difference between subscribing and then getting notifications?

### 2. Event Flow Understanding
- When we create a note with `client.send_event_builder()`:
  - Does it go to the relay? YES (we see it in logs)
  - Does it get saved to local DB? YES (we can list it)
  - Should it trigger our subscription? UNKNOWN

### 3. Subscription Mechanics
- Are we subscribing correctly?
- Is the filter format correct?
- Do we need to handle the subscription ID differently?
- Are notifications being sent but we're not receiving them?

## Things to Check

### Check 1: Look at nostr-sdk examples
Need to find examples of:
- How they handle subscriptions
- How they receive real-time events
- Pattern for watch/subscribe functionality

### Check 2: Subscription vs Query
- `client.database().query()` - works, gets from local DB
- `client.subscribe()` + `client.notifications()` - not working
- Is there a hybrid approach we're missing?

### Check 3: Event Broadcasting
- When we send an event, does it come back to us via subscription?
- Or do we need to handle self-events differently?
- Maybe the relay doesn't echo our own events back?

### Check 4: Notification Stream
- Is `client.notifications()` the right way to get events?
- Do we need to filter notifications by subscription ID?
- Are we dropping events somehow?

## Hypotheses

### Hypothesis 1: Self-Echo Issue
Maybe the relay doesn't send our own events back to us in subscriptions?
- Test: Create a note from a different client/key and see if it appears

### Hypothesis 2: Notification Stream Not Connected
Maybe `client.notifications()` needs to be set up before subscribing?
- Test: Get notifications stream before creating subscription

### Hypothesis 3: Wrong Event Matching
We're checking `event.kind == Kind::from(1059)` but maybe:
- The Kind value is different in subscriptions vs database
- We need to check something else

### Hypothesis 4: Database Auto-Ingestion
Maybe events that arrive via subscription are automatically saved to DB
- So we should watch the database for changes, not the relay
- Test: Check if there's a database change notification system

## Findings from nostr-sdk Examples

### Finding 1: Two Different Patterns

1. **handle_notifications pattern** (subscriptions.rs):
```rust
client.handle_notifications(|notification| async {
    // Handle each notification
    Ok(false) // return true to exit
}).await?;
```

2. **stream_events pattern** (stream-events.rs):
```rust
let mut stream = client.stream_events(filter, Duration::from_secs(15)).await?;
while let Some(event) = stream.next().await {
    // Handle event
}
```

### Finding 2: Our Current Approach Issues

We're using:
- `client.subscribe()` to create subscription
- `client.notifications()` to get a receiver
- Spawning a task to handle notifications

But the examples show:
- Either `handle_notifications` with a closure
- Or `stream_events` for a more direct stream

### Finding 3: Key Differences

- `handle_notifications`: Blocks and handles ALL notifications from ALL subscriptions
- `stream_events`: Creates a focused stream for specific filter
- Our approach: Trying to filter notifications manually in a spawned task

## New Hypothesis

We should use `stream_events` instead of manual subscription + notification handling!

## Next Steps

1. Try using `client.stream_events()` instead of our current approach
2. If that doesn't work, try `handle_notifications` pattern
3. Ensure we're handling the event stream in the right context (not spawned task?)

## Code to Update
- Replace subscribe_notes implementation with stream_events
- Test if this receives events properly

## SOLUTION IMPLEMENTED - Take 2

After further investigation, I realized the KEY INSIGHT:

**The SDK automatically saves incoming events to the database when you subscribe!**

You don't need to manually handle events and save them. The flow is:
1. Call `client.subscribe()` - sets up subscription with relay
2. Relay sends events matching the filter
3. **SDK automatically calls `database.save_event()` for each one**
4. We can query the database at any time to get the saved events

### New Implementation: `watch_simple.rs`

Three approaches:

1. **watch_notes_simple**: Use subscription notifications as triggers to query specific events from DB
2. **watch_notes_with_sync**: When event arrives, trigger negentropy sync, then query DB
3. **watch_notes_poll**: Simplest - just poll the database periodically after setting up subscription

All leverage the SDK's automatic database integration instead of trying to manually handle the event stream.

## SOLUTION IMPLEMENTED

Created new `watch.rs` module that uses `client.stream_events()` instead of manual subscription handling:

```rust
// Use the proper nostr-sdk API
let mut event_stream = self.client
    .stream_events(vec![filter], Some(Duration::from_secs(60)))
    .await?;

// Process events in a spawned task
while let Some(event) = event_stream.next().await {
    // Decrypt and send to channel
}
```

Key differences from our broken approach:
1. Uses `stream_events` which handles subscription properly
2. Gets a receiver that actually receives events
3. Works with the relay's event streaming mechanism

The CLI now:
1. Shows existing notes from database first
2. Then uses the new watch methods to stream incoming events
3. Should properly receive and display new notes as they arrive!