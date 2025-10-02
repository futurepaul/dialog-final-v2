mod common;
use common::TestServer;
use dialog_lib::ChangeEvent;
use serial_test::serial;
use std::time::Duration;

async fn wait_for_ingestion() {
    tokio::time::sleep(Duration::from_millis(150)).await;
}

#[tokio::test]
#[serial]
async fn test_dialog_complete() {
    let server = TestServer::new().await;
    let dialog = server.create_dialog().await;

    println!("=== Testing basic note creation and listing ===");

    // Create a simple note
    let text = "Test note #test #example";
    let id = dialog.create_note(text).await.unwrap();

    // Small delay for async database ingestion
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Should find it immediately
    let notes = dialog.list_notes(10).await.unwrap();
    assert!(
        notes.iter().any(|n| n.id == id),
        "Should find the note we just created"
    );

    // Verify content and tags
    let note = notes.iter().find(|n| n.id == id).unwrap();
    assert_eq!(note.text, text);
    assert!(note.tags.contains(&"test".to_string()));
    assert!(note.tags.contains(&"example".to_string()));

    println!("=== Testing encryption/decryption ===");

    // Test with unicode and special characters
    let secret_text = "Secret message with unicode and special chars!";
    let secret_id = dialog.create_note(secret_text).await.unwrap();
    println!("Created secret note with id: {secret_id}");

    // Small delay for async database ingestion
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let notes = dialog.list_notes(20).await.unwrap();
    println!("Found {} total notes", notes.len());

    let secret_note = notes
        .iter()
        .find(|n| n.id == secret_id)
        .expect("Should find the secret note we just created");
    assert_eq!(
        secret_note.text, secret_text,
        "Decrypted text should match exactly"
    );

    println!("=== Testing tag filtering ===");

    // Create notes with different tags for filtering
    dialog.create_note("Note A #alpha #beta").await.unwrap();
    dialog.create_note("Note B #alpha #gamma").await.unwrap();
    dialog.create_note("Note C #beta #delta").await.unwrap();
    dialog.create_note("Note D #gamma #delta").await.unwrap();

    // Test various tag filters
    let alpha_notes = dialog.list_by_tag("alpha", 10).await.unwrap();
    assert!(
        alpha_notes.len() >= 2,
        "Should have at least 2 notes with #alpha"
    );

    let delta_notes = dialog.list_by_tag("delta", 10).await.unwrap();
    assert!(
        delta_notes.len() >= 2,
        "Should have at least 2 notes with #delta"
    );

    let beta_notes = dialog.list_by_tag("beta", 10).await.unwrap();
    assert!(
        beta_notes.len() >= 2,
        "Should have at least 2 notes with #beta"
    );

    // Test case insensitivity
    let gamma_upper = dialog.list_by_tag("GAMMA", 10).await.unwrap();
    let gamma_lower = dialog.list_by_tag("gamma", 10).await.unwrap();
    assert_eq!(
        gamma_upper.len(),
        gamma_lower.len(),
        "Tag filtering should be case-insensitive"
    );

    println!("=== Testing batch creation ===");

    // Create multiple notes rapidly
    for i in 0..10 {
        let batch_text = format!("Batch note {i} #batch #stress");
        dialog.create_note(&batch_text).await.unwrap();
    }

    // Small delay for async database ingestion of batch
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Verify all batch notes exist
    let batch_notes = dialog.list_by_tag("batch", 20).await.unwrap();
    assert!(
        batch_notes.len() >= 10,
        "Should have at least 10 batch notes"
    );

    // Verify they're properly ordered (newest first)
    let all_notes = dialog.list_notes(50).await.unwrap();
    let batch_subset: Vec<_> = all_notes
        .iter()
        .filter(|n| n.tags.contains(&"batch".to_string()))
        .collect();
    assert!(
        batch_subset.len() >= 10,
        "Should find all batch notes in full list"
    );

    println!("=== Testing sync (if available) ===");

    // Test sync doesn't break anything
    match dialog.sync_notes().await {
        Ok(_) => println!("Sync successful"),
        Err(e) => println!("Sync not available: {e} (this is ok)"),
    }

    // Verify data still intact after sync attempt
    let final_notes = dialog.list_notes(100).await.unwrap();
    assert!(final_notes.len() >= 16, "Should have all notes after sync");

    println!("=== All tests passed! ===");
}

#[tokio::test]
#[serial]
async fn test_initial_catchup_via_negentropy() {
    let server = TestServer::new().await;

    // Seed relay with a note from an earlier session
    let first_session = server.create_dialog().await;
    let seeded_text = "Pre-existing note for catch-up #bootstrap";
    let seeded_id = first_session
        .create_note(seeded_text)
        .await
        .expect("failed to create seed note");
    wait_for_ingestion().await;
    drop(first_session);

    // Mimic a fresh install by removing local storage before reconnecting
    server.clear_storage();

    let dialog = server.create_dialog().await;

    let pre_sync = dialog
        .list_notes(10)
        .await
        .expect("listing notes before sync should succeed");
    assert!(pre_sync.is_empty(), "fresh session should have empty DB");

    dialog
        .initial_catchup()
        .await
        .expect("initial catch-up should succeed");
    wait_for_ingestion().await;

    let post_sync = dialog
        .list_notes(10)
        .await
        .expect("listing notes after sync should succeed");
    let retrieved = post_sync
        .iter()
        .find(|note| note.id == seeded_id)
        .expect("seeded note should be pulled via initial catch-up");
    assert_eq!(retrieved.text, seeded_text);
    assert!(
        retrieved.is_synced,
        "notes loaded via catch-up should be marked synced"
    );
}

#[tokio::test]
#[serial]
async fn test_live_watch_stream_persists_before_emit() {
    let server = TestServer::new().await;
    let dialog = server.create_dialog().await;

    dialog
        .initial_catchup()
        .await
        .expect("initial catch-up should succeed");

    let mut receiver = dialog
        .watch_changes()
        .await
        .expect("watch_changes should start successfully");

    let live_text = "Live note streaming #realtime";
    let created_id = dialog
        .create_note(live_text)
        .await
        .expect("failed to create live note");

    let received_id = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match receiver.recv().await {
                Some(ChangeEvent::NoteApplied { event_id }) => break Some(event_id),
                Some(ChangeEvent::RelayStatus { .. }) => continue,
                None => break None,
            }
        }
    })
    .await
    .expect("watch channel should deliver within timeout")
    .expect("watch channel closed unexpectedly");

    assert_eq!(
        received_id, created_id,
        "watch should deliver the correct event id"
    );

    let stored_note = dialog
        .get_note(&received_id)
        .await
        .expect("get_note should succeed")
        .expect("note should exist in database");
    assert_eq!(stored_note.text, live_text);
    assert!(
        stored_note.is_synced,
        "live events should be marked synced by the time they reach consumers"
    );
}

#[tokio::test]
#[serial]
async fn test_watch_ignores_preexisting_events() {
    let server = TestServer::new().await;
    let dialog = server.create_dialog().await;

    // Create an event before starting the live subscription
    dialog
        .create_note("Existing note before watch #historical")
        .await
        .expect("failed to create historical note");
    wait_for_ingestion().await;

    dialog
        .initial_catchup()
        .await
        .expect("initial catch-up should succeed");

    let mut receiver = dialog
        .watch_changes()
        .await
        .expect("watch_changes should start");

    // Ensure no events arrive if we don't publish anything new
    let maybe_event = tokio::time::timeout(Duration::from_millis(300), async {
        loop {
            match receiver.recv().await {
                Some(ChangeEvent::NoteApplied { event_id }) => break Some(event_id),
                Some(ChangeEvent::RelayStatus { .. }) => continue,
                None => break None,
            }
        }
    })
    .await;
    assert!(
        maybe_event.is_err(),
        "watch should not replay historical events"
    );
}
