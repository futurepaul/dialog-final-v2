# Iterative Implementation Plan

## Overview
We can break this into testable chunks that build on each other. Each phase can be tested independently before moving on.

## Phase 1: Update dialog_lib Note Structure ✅ TESTABLE
**Goal**: Add local state fields to Note, test with CLI

### Changes:
1. Add `is_read` and `is_synced` to Note struct
2. Default them to sensible values for now
3. Update existing functions to compile

### Files to modify:
- `dialog_lib/src/note.rs`
- `dialog_lib/src/query.rs` 
- `dialog_lib/src/watch.rs`

### Test:
```bash
# CLI should still work exactly as before
cargo test --lib dialog_lib
./dialog_cli create "test note"
./dialog_cli list
```

**Can merge to master when complete**

---

## Phase 2: Add Local State Management to dialog_lib ✅ TESTABLE
**Goal**: Store/retrieve read status using Kind 30078 events

### Changes:
1. Add `mark_as_read()` function
2. Add `get_read_status()` helper
3. Update `list_notes()` to include read status
4. Ensure Kind 30078 events NEVER sync to relays

### Files to modify:
- `dialog_lib/src/lib.rs` (add new functions)
- `dialog_lib/src/note.rs` (integrate read status)

### Test:
```bash
# Test with CLI - add a simple "mark-read" command
cargo test --lib dialog_lib
# Could add temporary CLI command to test
```

**Can merge to master when complete**

---

## Phase 3: Create Minimal dialog_uniffi Integration ✅ TESTABLE
**Goal**: Connect real dialog_lib to uniffi WITHOUT removing mock yet

### Changes:
1. Add dialog_lib dependency to dialog_uniffi/Cargo.toml
2. Update DialogClient::new() to accept nsec parameter
3. Add Ready event to UDL
4. Create a "hybrid" mode - mock data + real connection test

### Files to modify:
- `dialog_uniffi/Cargo.toml`
- `dialog_uniffi/src/dialog.udl`
- `dialog_uniffi/src/lib.rs`

### Test:
```bash
# Build uniffi and verify it compiles
cd dialog_uniffi
cargo build --release

# Run the iOS build script
./rebuild.sh

# iOS app should still work with mock data
# But we can add a test button that tries real connection
```

**Can merge to master when complete**

---

## Phase 4: Replace Mock with Real - One Command at a Time ✅ TESTABLE
**Goal**: Incrementally replace mock implementations

### Order of replacement (each can be tested):
1. **LoadNotes** - Replace mock list with real `dialog.list_notes()`
2. **CreateNote** - Real note creation 
3. **Watch** - Hook up real watch_notes()
4. **Tags** - Real tag filtering
5. **Delete/Update** - Final commands

### Test after each command:
```bash
./rebuild.sh --run
# Test that specific command in iOS
# Mock data still works for other commands
```

**Can merge after each command works**

---

## Phase 5: Remove Mock Code ✅ FINAL
**Goal**: Clean up all mock code

### Changes:
1. Remove mock_data.rs
2. Remove HashMap storage
3. Remove mock note generation
4. Clean up imports

### Test:
```bash
./rebuild.sh --clean --run
# Full iOS app test
# All features should work with real Nostr
```

---

## Alternative: Big Bang Approach 💥

If iterative feels too complex, we could:

1. Create a new branch `uniffi-real-integration`
2. Do ALL changes at once
3. Test thoroughly
4. Merge when fully working

**Pros**: 
- No intermediate states
- Cleaner commits
- No temporary code

**Cons**:
- Harder to debug if something breaks
- Longer time before we see it working
- Risk of getting stuck

---

## Recommendation: Iterative with Feature Branch

1. Create branch: `uniffi-integration`
2. Do Phase 1-2 on dialog_lib (can test with CLI)
3. Do Phase 3-4 on dialog_uniffi (can test with iOS)
4. Merge to master when iOS works
5. Clean up in follow-up PR

This gives us:
- ✅ Quick feedback loops
- ✅ Can always roll back
- ✅ Master stays stable
- ✅ Can share progress/get help if stuck

## Which approach do you prefer?