use crate::{Dialog, DialogError, Note, Result};
use nostr_sdk::prelude::*;

impl Dialog {
    pub async fn list_notes(&self, limit: usize) -> Result<Vec<Note>> {
        eprintln!(
            "[lib] list_notes: limit={} for pubkey={}",
            limit,
            self.keys.public_key()
        );
        // Query from local database
        let filter = Filter::new()
            .author(self.keys.public_key())
            .kind(Kind::from(1059))
            .limit(limit);

        let events = self
            .client
            .database()
            .query(vec![filter])
            .await
            .map_err(|e| DialogError::Database(e.to_string()))?;

        // Decrypt and convert to Notes
        let mut notes = Vec::new();
        for event in events {
            if let Ok(decrypted) = self.decrypt_event(&event) {
                let is_read = self.get_read_status(&event.id).await;
                notes.push(Note {
                    id: event.id,
                    text: decrypted,
                    tags: extract_tags(&event),
                    created_at: event.created_at,
                    is_read,
                    is_synced: true, // If it's in DB, it was synced
                });
            }
        }

        // Sort by created_at descending (newest first)
        notes.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        eprintln!("[lib] list_notes: returning {} notes", notes.len());
        Ok(notes)
    }

    pub async fn list_by_tag(&self, tag: &str, limit: usize) -> Result<Vec<Note>> {
        let filter = Filter::new()
            .author(self.keys.public_key())
            .kind(Kind::from(1059))
            .hashtag(tag.to_lowercase())
            .limit(limit);

        let events = self
            .client
            .database()
            .query(vec![filter])
            .await
            .map_err(|e| DialogError::Database(e.to_string()))?;

        // Decrypt and convert to Notes
        let mut notes = Vec::new();
        for event in events {
            if let Ok(decrypted) = self.decrypt_event(&event) {
                let is_read = self.get_read_status(&event.id).await;
                notes.push(Note {
                    id: event.id,
                    text: decrypted,
                    tags: extract_tags(&event),
                    created_at: event.created_at,
                    is_read,
                    is_synced: true, // If it's in DB, it was synced
                });
            }
        }

        // Sort by created_at descending (newest first)
        notes.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(notes)
    }

    pub async fn sync_notes(&self) -> Result<()> {
        // Sync with relay using negentropy
        let filter = Filter::new()
            .author(self.keys.public_key())
            .kind(Kind::from(1059));

        self.client.sync(filter, &SyncOptions::default()).await?;
        Ok(())
    }

    /// Plain NIP-01 subscribe/fetch fallback for relays without Negentropy
    pub async fn sync_notes_plain(&self, limit: Option<usize>) -> Result<()> {
        // Build a standard filter. If a limit is provided, apply it.
        let mut filter = Filter::new()
            .author(self.keys.public_key())
            .kind(Kind::from(1059));
        if let Some(lim) = limit {
            filter = filter.limit(lim);
        }

        // Fetch a snapshot of events and persist to local DB
        // Try a reasonable timeout; network errors are surfaced as DialogError::Database via save.
        let events = self
            .client
            .fetch_events(vec![filter], Some(std::time::Duration::from_secs(10)))
            .await
            .map_err(|e| DialogError::Database(e.to_string()))?;

        for event in events {
            // Save to local DB; ignore duplicates
            self.client
                .database()
                .save_event(&event)
                .await
                .map_err(|e| DialogError::Database(e.to_string()))?;
        }
        Ok(())
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_tags() {
        // Create a test event with hashtags
        let test_keys = Keys::generate();
        let tags = vec![
            Tag::hashtag("test"),
            Tag::hashtag("example"),
            Tag::public_key(test_keys.public_key()),
        ];

        let sig_bytes = [0u8; 64];
        let event = Event::new(
            EventId::all_zeros(),
            test_keys.public_key(),
            Timestamp::now(),
            Kind::from(1059),
            tags,
            "encrypted content",
            Signature::from_slice(&sig_bytes).unwrap(),
        );

        let extracted = extract_tags(&event);
        assert_eq!(extracted, vec!["test", "example"]);
    }
}
