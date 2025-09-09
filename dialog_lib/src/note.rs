use crate::{Dialog, Result};
use nostr_sdk::prelude::*;

#[derive(Debug, Clone)]
pub struct Note {
    pub id: EventId,
    pub text: String,
    pub tags: Vec<String>,
    pub created_at: Timestamp,
    pub is_read: bool,
    pub is_synced: bool,
}

impl Dialog {
    pub async fn create_note(&self, text: &str) -> Result<EventId> {
        // Parse hashtags from text
        let tags = parse_hashtags(text);

        // Create encrypted content for self-DM using NIP-44
        let encrypted = nip44::encrypt(
            self.keys.secret_key(),
            &self.keys.public_key(), // Encrypt to self
            text,
            nip44::Version::default(),
        )?;

        // Build event with NIP-44 encrypted content
        // Using Kind 1059 for encrypted direct messages
        let mut builder = EventBuilder::new(Kind::from(1059), encrypted);

        // Add t tags for topics (lowercase)
        for tag in &tags {
            builder = builder.tag(Tag::hashtag(tag.to_lowercase()));
        }

        // Add p tag pointing to self (for self-DM)
        builder = builder.tag(Tag::public_key(self.keys.public_key()));

        // Send the event (this also saves to local db)
        let output = self.client.send_event_builder(builder).await?;
        Ok(*output.id())
    }

    pub(crate) fn decrypt_event(&self, event: &Event) -> Result<String> {
        let decrypted = nip44::decrypt(
            self.keys.secret_key(),
            &self.keys.public_key(),
            &event.content,
        )?;
        Ok(decrypted)
    }
}

fn parse_hashtags(text: &str) -> Vec<String> {
    text.split_whitespace()
        .filter(|word| word.starts_with('#') && word.len() > 1)
        .map(|tag| tag[1..].to_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hashtags() {
        let text = "This is a #Test note with #Multiple #TAGS";
        let tags = parse_hashtags(text);
        assert_eq!(tags, vec!["test", "multiple", "tags"]);
    }

    #[test]
    fn test_parse_hashtags_empty() {
        let text = "This has no hashtags";
        let tags = parse_hashtags(text);
        assert!(tags.is_empty());
    }

    #[test]
    fn test_parse_hashtags_just_hash() {
        let text = "This has just # and nothing else";
        let tags = parse_hashtags(text);
        assert!(tags.is_empty());
    }
}
