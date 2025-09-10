use chrono::Utc;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Note {
    pub id: String,
    pub text: String,
    pub tags: Vec<String>,
    pub created_at: i64, // Changed to i64 to match Swift expectations
    pub is_read: bool,
    pub is_synced: bool,
}

#[derive(Clone, Debug)]
pub struct TagCount {
    pub tag: String,
    pub count: u32,
}

impl Note {
    pub fn from_text(text: String) -> Self {
        // Parse hashtags
        let tags = text
            .split_whitespace()
            .filter(|word| word.starts_with('#') && word.len() > 1)
            .map(|tag| tag[1..].to_lowercase())
            .collect();

        Self {
            id: Uuid::new_v4().to_string(),
            text,
            tags,
            created_at: Utc::now().timestamp(),
            is_read: false,
            is_synced: false,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    Ready, // Sent when Dialog is initialized
    NotesLoaded { notes: Vec<Note> },
    NoteAdded { note: Note },
    NoteUpdated { note: Note },
    NoteDeleted { id: String },
    TagFilterChanged { tag: Option<String> },
    SyncStatusChanged { syncing: bool },
    Error { message: String },
}

#[derive(Clone, Debug)]
pub enum SyncMode {
    Negentropy,
    Subscribe,
}

#[derive(Clone, Debug)]
pub enum Command {
    ConnectRelay { relay_url: String },
    CreateNote { text: String },
    DeleteNote { id: String },
    MarkAsRead { id: String },
    SetTagFilter { tag: Option<String> },
    LoadNotes { limit: u32 },
    SearchNotes { query: String },
    SetSyncMode { mode: SyncMode },
}
