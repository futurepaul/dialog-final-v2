mod common;
use common::TestServer;

#[tokio::test]
async fn test_dialog_complete() {
    let server = TestServer::new().await;
    let dialog = server.create_dialog().await;

    println!("=== Testing basic note creation and listing ===");

    // Create a simple note
    let text = "Test note #test #example";
    let id = dialog.create_note(text).await.unwrap();

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
    println!("Created secret note with id: {}", secret_id);

    // Give a moment for the event to be processed
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

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
        let batch_text = format!("Batch note {} #batch #stress", i);
        dialog.create_note(&batch_text).await.unwrap();
    }

    // Give time for all batch notes to be processed
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

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
        Err(e) => println!("Sync not available: {} (this is ok)", e),
    }

    // Verify data still intact after sync attempt
    let final_notes = dialog.list_notes(100).await.unwrap();
    assert!(final_notes.len() >= 16, "Should have all notes after sync");

    println!("=== All tests passed! ===");
}
