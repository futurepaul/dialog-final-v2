mod mock_data;
mod models;

use models::{Note, Event, Command};
use mock_data::generate_mock_notes;

use once_cell::sync::OnceCell;
use std::{collections::HashMap, sync::Arc};
use tokio::{
    runtime::Runtime,
    sync::{broadcast, RwLock},
};

uniffi::include_scaffolding!("dialog");

// Global Tokio runtime
fn rt() -> &'static Runtime {
    static RT: OnceCell<Runtime> = OnceCell::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("dialog-uniffi")
            .build()
            .expect("Failed to create Tokio runtime")
    })
}

pub struct DialogClient {
    notes: Arc<RwLock<HashMap<String, Note>>>,
    current_filter: Arc<RwLock<Option<String>>>,
    event_tx: broadcast::Sender<Event>,
}

impl DialogClient {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(1024);
        let client = Self {
            notes: Arc::new(RwLock::new(HashMap::new())),
            current_filter: Arc::new(RwLock::new(None)),
            event_tx,
        };
        
        // Load mock data
        let notes_clone = client.notes.clone();
        rt().spawn(async move {
            let mut notes = notes_clone.write().await;
            for note in generate_mock_notes() {
                notes.insert(note.id.clone(), note);
            }
        });
        
        client
    }
    
    pub fn start(self: Arc<Self>, listener: Box<dyn DialogListener>) {
        // Set up event forwarding to Swift (non-blocking)
        let mut rx = self.event_tx.subscribe();
        
        // Convert Box to Arc for sharing between threads
        let listener: Arc<dyn DialogListener> = Arc::from(listener);
        let listener_clone = listener.clone();
        
        // Spawn listener on background thread
        rt().spawn(async move {
            while let Ok(event) = rx.recv().await {
                // Callback to Swift happens on background thread
                // Swift will handle @MainActor transition
                listener_clone.on_event(event);
            }
        });
        
        // Send initial data
        let notes = self.get_notes(100, None);
        listener.on_event(Event::NotesLoaded { notes });
    }
    
    pub fn stop(&self) {
        // Cleanup if needed
    }
    
    pub fn send_command(self: Arc<Self>, cmd: Command) {
        // Fire-and-forget: spawn work on Tokio runtime
        let self_clone = self.clone();
        rt().spawn(async move {
            match cmd {
                Command::CreateNote { text } => {
                    self_clone.create_note(text).await;
                }
                Command::SetTagFilter { tag } => {
                    self_clone.set_filter(tag).await;
                }
                Command::MarkAsRead { id } => {
                    self_clone.mark_as_read(id).await;
                }
                Command::LoadNotes { limit } => {
                    let filter = self_clone.current_filter.read().await.clone();
                    let notes = self_clone.get_notes(limit, filter);
                    let _ = self_clone.event_tx.send(Event::NotesLoaded { notes });
                }
                Command::DeleteNote { id } => {
                    self_clone.delete_note(id).await;
                }
                Command::SearchNotes { query } => {
                    self_clone.search_notes(query).await;
                }
            }
        });
    }
    
    // Fast synchronous queries
    pub fn get_notes(&self, limit: u32, tag: Option<String>) -> Vec<Note> {
        // Use try_read to avoid blocking in async context
        let notes = match self.notes.try_read() {
            Ok(guard) => guard,
            Err(_) => {
                // If we can't get a read lock immediately, return empty
                // This shouldn't happen in practice since reads don't block each other
                return Vec::new();
            }
        };
        let mut result: Vec<Note> = notes
            .values()
            .filter(|n| {
                tag.as_ref().map_or(true, |t| n.tags.contains(t))
            })
            .cloned()
            .collect();
        
        result.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        result.into_iter().take(limit as usize).collect()
    }
    
    pub fn get_all_tags(&self) -> Vec<String> {
        let notes = match self.notes.try_read() {
            Ok(guard) => guard,
            Err(_) => return Vec::new(),
        };
        let mut tags = std::collections::HashSet::new();
        for note in notes.values() {
            for tag in &note.tags {
                tags.insert(tag.clone());
            }
        }
        let mut result: Vec<String> = tags.into_iter().collect();
        result.sort();
        result
    }
    
    pub fn get_note(&self, id: String) -> Option<Note> {
        self.notes.try_read().ok()?.get(&id).cloned()
    }
    
    pub fn get_unread_count(&self, tag: Option<String>) -> u32 {
        let notes = match self.notes.try_read() {
            Ok(guard) => guard,
            Err(_) => return 0,
        };
        notes
            .values()
            .filter(|n| !n.is_read)
            .filter(|n| tag.as_ref().map_or(true, |t| n.tags.contains(t)))
            .count() as u32
    }
    
    // Private async helpers
    async fn create_note(self: Arc<Self>, text: String) {
        let note = Note::from_text(text);
        let id = note.id.clone();
        
        // Update state
        self.notes.write().await.insert(id, note.clone());
        
        // Push event
        let _ = self.event_tx.send(Event::NoteAdded { note });
    }
    
    async fn set_filter(self: Arc<Self>, tag: Option<String>) {
        *self.current_filter.write().await = tag.clone();
        let _ = self.event_tx.send(Event::TagFilterChanged { tag: tag.clone() });
        
        // Re-send filtered notes
        let notes = self.get_notes(100, tag);
        let _ = self.event_tx.send(Event::NotesLoaded { notes });
    }
    
    async fn mark_as_read(self: Arc<Self>, id: String) {
        let mut notes = self.notes.write().await;
        if let Some(note) = notes.get_mut(&id) {
            note.is_read = true;
            let _ = self.event_tx.send(Event::NoteUpdated { note: note.clone() });
        }
    }
    
    async fn delete_note(self: Arc<Self>, id: String) {
        let mut notes = self.notes.write().await;
        if notes.remove(&id).is_some() {
            let _ = self.event_tx.send(Event::NoteDeleted { id });
        }
    }
    
    async fn search_notes(self: Arc<Self>, query: String) {
        let notes = self.notes.read().await;
        let query_lower = query.to_lowercase();
        let results: Vec<Note> = notes
            .values()
            .filter(|n| n.text.to_lowercase().contains(&query_lower))
            .cloned()
            .collect();
        let _ = self.event_tx.send(Event::NotesLoaded { notes: results });
    }
}

pub trait DialogListener: Send + Sync {
    fn on_event(&self, event: Event);
}
