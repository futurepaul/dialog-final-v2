use crate::{Dialog, Note, Result};
use nostr_sdk::prelude::*;
use std::collections::HashSet;
use tokio::sync::mpsc;

impl Dialog {
    pub async fn watch_notes(&self) -> Result<mpsc::Receiver<Note>> {
        let (tx, rx) = mpsc::channel(100);
        
        let client = self.client.clone();
        let keys = self.keys.clone();
        let pubkey = self.keys.public_key();
        
        // Set up subscription
        let filter = Filter::new()
            .author(pubkey)
            .kind(Kind::from(1059))
            .since(Timestamp::now());
        
        eprintln!("DEBUG: Creating subscription with filter: {:?}", filter);
        let output = self.client.subscribe(vec![filter], None).await?;
        let sub_id = output.val;
        eprintln!("DEBUG: Subscription created with id: {}", sub_id);
        
        tokio::spawn(async move {
            let mut notifications = client.notifications();
            let mut seen_ids = HashSet::new();
            
            eprintln!("DEBUG: Watch task started, entering loop");
            loop {
                eprintln!("DEBUG: Waiting for notification...");
                match notifications.recv().await {
                    Ok(RelayPoolNotification::Message { message, .. }) => {
                        if let RelayMessage::Event { subscription_id, event } = message {
                            eprintln!("DEBUG: Got event from subscription: {} (our id: {})", subscription_id, sub_id);
                            if subscription_id == sub_id && 
                               event.kind == Kind::from(1059) && 
                               event.pubkey == pubkey &&
                               !seen_ids.contains(&event.id) {
                                
                                if let Ok(decrypted) = decrypt_event(&keys, &event) {
                                    let note = Note {
                                        id: event.id,
                                        text: decrypted,
                                        tags: extract_tags(&event),
                                        created_at: event.created_at,
                                    };
                                    
                                    seen_ids.insert(event.id);
                                    let _ = tx.send(note).await;
                                    eprintln!("DEBUG: Sent note to channel");
                                }
                            }
                        }
                    }
                    Ok(RelayPoolNotification::Event { event, .. }) => {
                        // Try the old pattern too just in case
                        eprintln!("DEBUG: Got direct event notification");
                        if event.kind == Kind::from(1059) && 
                           event.pubkey == pubkey &&
                           !seen_ids.contains(&event.id) {
                            
                            if let Ok(decrypted) = decrypt_event(&keys, &event) {
                                let note = Note {
                                    id: event.id,
                                    text: decrypted,
                                    tags: extract_tags(&event),
                                    created_at: event.created_at,
                                };
                                
                                seen_ids.insert(event.id);
                                let _ = tx.send(note).await;
                            }
                        }
                    }
                    Ok(other) => {
                        eprintln!("DEBUG: Got other notification: {:?}", other);
                        continue;
                    }
                    Err(e) => {
                        eprintln!("DEBUG: Error receiving notification: {:?}", e);
                        break;
                    }
                }
            }
            eprintln!("DEBUG: Watch loop exited!");
        });
        
        eprintln!("DEBUG: Returning receiver");
        Ok(rx)
    }
}

// Helper function to decrypt events
fn decrypt_event(keys: &Keys, event: &Event) -> Result<String> {
    let decrypted = nip44::decrypt(
        keys.secret_key(),
        &keys.public_key(),
        &event.content,
    )?;
    Ok(decrypted)
}

// Helper function to extract tags
fn extract_tags(event: &Event) -> Vec<String> {
    event
        .tags
        .iter()
        .filter_map(|tag| {
            if let Some(TagStandard::Hashtag(t)) = tag.as_standardized() {
                Some(t.to_string())
            } else {
                None
            }
        })
        .collect()
}