use chrono::Utc;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Note {
    pub id: String,
    pub text: String,
    pub tags: Vec<String>,
    pub created_at: u64,
    pub is_read: bool,
    pub is_synced: bool,
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
            created_at: Utc::now().timestamp() as u64,
            is_read: false,
            is_synced: false,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    NotesLoaded { notes: Vec<Note> },
    NoteAdded { note: Note },
    NoteUpdated { note: Note },
    NoteDeleted { id: String },
    TagFilterChanged { tag: Option<String> },
    SyncStatusChanged { syncing: bool },
    Error { message: String },
}

#[derive(Clone, Debug)]
pub enum Command {
    CreateNote { text: String },
    DeleteNote { id: String },
    MarkAsRead { id: String },
    SetTagFilter { tag: Option<String> },
    LoadNotes { limit: u32 },
    SearchNotes { query: String },
}