use crate::{convert::convert_lib_note_to_uniffi, runtime::{rt, DIALOG}, Event, Note, SyncMode, TagCount};
use dialog_lib::Dialog;
use std::{collections::{HashMap, HashSet}, sync::Arc};
use tokio::sync::{RwLock, broadcast};

pub struct DialogClient {
    pub(crate) notes: Arc<RwLock<HashMap<String, Note>>>,
    pub(crate) current_filter: Arc<RwLock<Option<String>>>,
    pub(crate) event_tx: broadcast::Sender<Event>,
    pub(crate) watch_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    pub(crate) sync_mode: Arc<RwLock<SyncMode>>, // Default from env or Negentropy
}

impl DialogClient {
    pub fn new(nsec: String) -> Self {
        eprintln!("[uniffi] DialogClient::new - initializing with nsec len={} chars", nsec.len());
        // Initialize Dialog once
        let dialog: Dialog = rt().block_on(async {
            match Dialog::new(&nsec).await {
                Ok(d) => {
                    eprintln!("[uniffi] Dialog initialized; pubkey={}", d.public_key());
                    d
                }
                Err(e) => panic!("[uniffi] Failed to initialize Dialog: {e}"),
            }
        });
        if DIALOG.set(dialog).is_err() {
            panic!("[uniffi] Dialog already initialized");
        }

        let (event_tx, _) = broadcast::channel(1024);
        // Resolve sync mode from env (DIALOG_SYNC_MODE)
        let sync_mode = match std::env::var("DIALOG_SYNC_MODE").ok().as_deref() {
            Some("subscribe") => SyncMode::Subscribe,
            _ => SyncMode::Negentropy,
        };
        let client = Self {
            notes: Arc::new(RwLock::new(HashMap::new())),
            current_filter: Arc::new(RwLock::new(None)),
            event_tx,
            watch_handle: Arc::new(RwLock::new(None)),
            sync_mode: Arc::new(RwLock::new(sync_mode)),
        };

        // Load initial notes from dialog_lib
        eprintln!("[uniffi] Loading initial notes...");
        let notes_clone = client.notes.clone();
        let event_tx_clone = client.event_tx.clone();
        rt().spawn(async move {
            if let Ok(lib_notes) = DIALOG.get().unwrap().list_notes(100).await {
                eprintln!("[uniffi] Initial notes loaded: {}", lib_notes.len());
                let mut notes = notes_clone.write().await;
                for lib_note in lib_notes {
                    let note = convert_lib_note_to_uniffi(lib_note);
                    notes.insert(note.id.clone(), note.clone());
                }
                // Send ready event
                eprintln!("[uniffi] Sending Event::Ready");
                let _ = event_tx_clone.send(Event::Ready);
            } else {
                eprintln!("[uniffi] Failed to load initial notes");
            }
        });

        client
    }

    pub fn start(self: Arc<Self>, listener: Box<dyn DialogListener>) {
        eprintln!("[uniffi] start() called; wiring listener and watch loop");
        // Set up event forwarding to Swift (non-blocking)
        let mut rx = self.event_tx.subscribe();

        // Convert Box to Arc for sharing between threads
        let listener: Arc<dyn DialogListener> = Arc::from(listener);
        let listener_clone = listener.clone();

        // Spawn listener on background thread
        rt().spawn(async move {
            while let Ok(event) = rx.recv().await {
                eprintln!("[uniffi] Dispatching event to Swift: {event:?}");
                listener_clone.on_event(event);
            }
        });

        // Attempt to start watch loop immediately; if not connected yet, we'll try again after connect.
        let self_clone = self.clone();
        rt().spawn(async move {
            self_clone.maybe_start_watch().await;
        });

        // Send initial data
        let notes = self.get_notes(100, None);
        eprintln!("[uniffi] Emitting initial Event::NotesLoaded count={}", notes.len());
        listener.on_event(Event::NotesLoaded { notes });
    }

    // Fast synchronous queries
    pub fn get_notes(&self, limit: u32, tag: Option<String>) -> Vec<Note> {
        let notes = match self.notes.try_read() {
            Ok(guard) => guard,
            Err(_) => return Vec::new(),
        };
        let mut result: Vec<Note> = notes
            .values()
            .filter(|n| tag.as_ref().is_none_or(|t| n.tags.contains(t)))
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
        let mut tags = HashSet::new();
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
            .filter(|n| tag.as_ref().is_none_or(|t| n.tags.contains(t)))
            .count() as u32
    }

    pub fn get_tag_counts(&self) -> Vec<TagCount> {
        let notes = match self.notes.try_read() {
            Ok(guard) => guard,
            Err(_) => return Vec::new(),
        };
        let mut counts: HashMap<String, u32> = HashMap::new();
        for note in notes.values() {
            for tag in &note.tags {
                *counts.entry(tag.clone()).or_insert(0) += 1;
            }
        }
        let mut result: Vec<TagCount> = counts
            .into_iter()
            .map(|(tag, count)| TagCount { tag, count })
            .collect();
        result.sort_by(|a, b| a.tag.cmp(&b.tag));
        result
    }

    // Data management
    pub fn clear_data_for_current_pubkey(&self) {
        if let Some(dialog) = DIALOG.get() {
            let pubkey = dialog.public_key().to_hex();
            if let Err(e) = dialog_lib::clean_test_storage(&pubkey) {
                eprintln!("[uniffi] clear_data_for_current_pubkey error: {e}");
            }
        }
    }

    // Instance helpers expected by UniFFI UDL
    pub fn stop(&self) { /* no-op for now */ }

    pub fn validate_nsec(&self, nsec: String) -> bool {
        dialog_lib::validate_nsec(&nsec).is_ok()
    }

    pub fn derive_npub(&self, nsec: String) -> String {
        use nostr_sdk::{prelude::Keys, ToBech32};
        match Keys::parse(&nsec) {
            Ok(keys) => keys.public_key().to_bech32().unwrap_or_default(),
            Err(_) => String::new(),
        }
    }
}

pub trait DialogListener: Send + Sync {
    fn on_event(&self, event: Event);
}
