mod mock_data;
mod models;

use models::{Note, Event, Command};

use dialog_lib::{Dialog, Note as LibNote};
use nostr_sdk::prelude::*;
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

// Global Dialog instance
static DIALOG: OnceCell<Dialog> = OnceCell::new();

pub struct DialogClient {
    notes: Arc<RwLock<HashMap<String, Note>>>,
    current_filter: Arc<RwLock<Option<String>>>,
    event_tx: broadcast::Sender<Event>,
    watch_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl DialogClient {
    pub fn new(nsec: String) -> Self {
        // Initialize Dialog once
        let dialog = rt().block_on(async {
            match Dialog::new(&nsec).await {
                Ok(d) => d,
                Err(e) => panic!("Failed to initialize Dialog: {}", e),
            }
        });
        if DIALOG.set(dialog).is_err() {
            panic!("Dialog already initialized");
        }
        
        let (event_tx, _) = broadcast::channel(1024);
        let client = Self {
            notes: Arc::new(RwLock::new(HashMap::new())),
            current_filter: Arc::new(RwLock::new(None)),
            event_tx,
            watch_handle: Arc::new(RwLock::new(None)),
        };
        
        // Load initial notes from dialog_lib
        let notes_clone = client.notes.clone();
        let event_tx_clone = client.event_tx.clone();
        rt().spawn(async move {
            if let Ok(lib_notes) = DIALOG.get().unwrap().list_notes(100).await {
                let mut notes = notes_clone.write().await;
                for lib_note in lib_notes {
                    let note = convert_lib_note_to_uniffi(lib_note);
                    notes.insert(note.id.clone(), note.clone());
                }
                // Send ready event
                let _ = event_tx_clone.send(Event::Ready);
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
        
        // Start watching for new notes
        let self_clone = self.clone();
        let handle = rt().spawn(async move {
            if let Ok(mut receiver) = DIALOG.get().unwrap().watch_notes().await {
                while let Some(lib_note) = receiver.recv().await {
                    let note = convert_lib_note_to_uniffi(lib_note);
                    
                    // Update cache
                    self_clone.notes.write().await.insert(note.id.clone(), note.clone());
                    
                    // Push event
                    let _ = self_clone.event_tx.send(Event::NoteAdded { note });
                }
            }
        });
        
        // Store the watch handle
        let self_clone = self.clone();
        rt().spawn(async move {
            *self_clone.watch_handle.write().await = Some(handle);
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
                Command::ConnectRelay { relay_url } => {
                    if let Err(e) = DIALOG.get().unwrap().connect_relay(&relay_url).await {
                        eprintln!("Failed to connect to relay: {}", e);
                    }
                }
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
                    // Sync from dialog_lib
                    if let Ok(lib_notes) = DIALOG.get().unwrap().list_notes(limit as usize).await {
                        let mut notes_map = self_clone.notes.write().await;
                        let mut notes = Vec::new();
                        
                        for lib_note in lib_notes {
                            let note = convert_lib_note_to_uniffi(lib_note);
                            notes_map.insert(note.id.clone(), note.clone());
                            notes.push(note);
                        }
                        
                        // Apply filter if set
                        let filter = self_clone.current_filter.read().await.clone();
                        if let Some(tag) = filter {
                            notes.retain(|n| n.tags.contains(&tag));
                        }
                        
                        let _ = self_clone.event_tx.send(Event::NotesLoaded { notes });
                    }
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
        // Create note via dialog_lib
        if let Ok(_note_id) = DIALOG.get().unwrap().create_note(&text).await {
            // Fetch the created note
            if let Ok(lib_notes) = DIALOG.get().unwrap().list_notes(1).await {
                if let Some(lib_note) = lib_notes.first() {
                    let note = convert_lib_note_to_uniffi(lib_note.clone());
                    
                    // Update state
                    self.notes.write().await.insert(note.id.clone(), note.clone());
                    
                    // Push event
                    let _ = self.event_tx.send(Event::NoteAdded { note });
                }
            }
        }
    }
    
    async fn set_filter(self: Arc<Self>, tag: Option<String>) {
        *self.current_filter.write().await = tag.clone();
        let _ = self.event_tx.send(Event::TagFilterChanged { tag: tag.clone() });
        
        // Re-send filtered notes
        let notes = self.get_notes(100, tag);
        let _ = self.event_tx.send(Event::NotesLoaded { notes });
    }
    
    async fn mark_as_read(self: Arc<Self>, id: String) {
        // Mark as read via dialog_lib
        if let Ok(event_id) = EventId::from_hex(&id) {
            if let Ok(_) = DIALOG.get().unwrap().mark_as_read(&event_id).await {
                let mut notes = self.notes.write().await;
                if let Some(note) = notes.get_mut(&id) {
                    note.is_read = true;
                    let _ = self.event_tx.send(Event::NoteUpdated { note: note.clone() });
                }
            }
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

// Helper function to convert dialog_lib Note to uniffi Note
fn convert_lib_note_to_uniffi(lib_note: LibNote) -> Note {
    Note {
        id: lib_note.id.to_hex(),
        text: lib_note.text,
        tags: lib_note.tags,
        created_at: lib_note.created_at.as_u64() as i64,
        is_read: lib_note.is_read,
        is_synced: lib_note.is_synced,
    }
}
