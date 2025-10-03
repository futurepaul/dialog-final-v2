use crate::{Event, Note, SyncMode, TagCount, convert::convert_lib_note_to_uniffi, runtime::rt};
use dialog_lib::Dialog;
use nostr_sdk::prelude::EventId;
use std::{collections::HashSet, sync::Arc};
use tokio::sync::{RwLock, broadcast};
use tokio::task::JoinHandle;

fn framework_version() -> &'static str {
    option_env!("UNIFFI_FRAMEWORK_VERSION").unwrap_or("unknown")
}

pub struct DialogClient {
    pub(crate) dialog: Arc<Dialog>,
    pub(crate) current_filter: Arc<RwLock<Option<String>>>,
    pub(crate) event_tx: broadcast::Sender<Event>,
    pub(crate) watch_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    pub(crate) sync_mode: Arc<RwLock<SyncMode>>, // Default from env or Negentropy
    pub(crate) delivered_ids: Arc<RwLock<HashSet<String>>>,
    pub(crate) listener_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
}

impl DialogClient {
    pub fn new(nsec: String) -> Self {
        eprintln!(
            "[uniffi] DialogClient::new - initializing with nsec len={} chars",
            nsec.len()
        );

        let dialog = rt().block_on(async {
            match Dialog::new(&nsec).await {
                Ok(d) => {
                    eprintln!("[uniffi] Dialog initialized; pubkey={}", d.public_key());
                    Arc::new(d)
                }
                Err(e) => panic!("[uniffi] Failed to initialize Dialog: {e}"),
            }
        });

        let (event_tx, _) = broadcast::channel(1024);
        // Resolve sync mode from env (DIALOG_SYNC_MODE)
        let sync_mode = match std::env::var("DIALOG_SYNC_MODE").ok().as_deref() {
            Some("subscribe") => SyncMode::Subscribe,
            _ => SyncMode::Negentropy,
        };

        let client = Self {
            dialog,
            current_filter: Arc::new(RwLock::new(None)),
            event_tx,
            watch_handle: Arc::new(RwLock::new(None)),
            sync_mode: Arc::new(RwLock::new(sync_mode)),
            delivered_ids: Arc::new(RwLock::new(HashSet::new())),
            listener_handle: Arc::new(RwLock::new(None)),
        };

        eprintln!(
            "[uniffi] DialogClient::new - XCFramework version {}",
            framework_version()
        );

        // Emit ready event asynchronously so Swift knows the bridge is live
        let event_tx_clone = client.event_tx.clone();
        rt().spawn(async move {
            eprintln!("[uniffi] Sending Event::Ready");
            let _ = event_tx_clone.send(Event::Ready);
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
        let listener_handle = rt().spawn(async move {
            while let Ok(event) = rx.recv().await {
                eprintln!("[uniffi] Dispatching event to Swift: {event:?}");
                listener_clone.on_event(event);
            }
        });
        rt().block_on(async {
            let mut handle = self.listener_handle.write().await;
            *handle = Some(listener_handle);
        });

        // Attempt to start watch loop immediately; if not connected yet, we'll try again after connect.
        let self_clone = self.clone();
        rt().spawn(async move {
            self_clone.maybe_start_watch().await;
        });

        // Send initial data
        let notes = self.get_notes(100, None);
        let delivered_ids = self.delivered_ids.clone();
        let ids_to_record: Vec<String> = notes.iter().map(|n| n.id.clone()).collect();
        rt().block_on(async move {
            let mut guard = delivered_ids.write().await;
            for id in ids_to_record {
                guard.insert(id);
            }
        });
        eprintln!(
            "[uniffi] Emitting initial Event::NotesLoaded count={}",
            notes.len()
        );
        listener.on_event(Event::NotesLoaded { notes });
    }

    // Fast synchronous queries
    pub fn get_notes(&self, limit: u32, tag: Option<String>) -> Vec<Note> {
        rt().block_on(self.fetch_notes_async(limit, tag))
    }

    pub fn get_all_tags(&self) -> Vec<String> {
        let notes = self.get_notes(1_000, None);
        let mut tags = HashSet::new();
        for note in notes {
            for tag in note.tags {
                tags.insert(tag);
            }
        }
        let mut result: Vec<String> = tags.into_iter().collect();
        result.sort();
        result
    }

    pub fn get_note(&self, id: String) -> Option<Note> {
        let event_id = EventId::from_hex(&id).ok()?;
        rt().block_on(self.fetch_note_async(&event_id))
    }

    pub fn get_unread_count(&self, tag: Option<String>) -> u32 {
        let notes = self.get_notes(1_000, tag);
        notes.into_iter().filter(|n| !n.is_read).count() as u32
    }

    pub fn get_tag_counts(&self) -> Vec<TagCount> {
        let notes = self.get_notes(1_000, None);
        let mut counts = std::collections::HashMap::new();
        for note in notes {
            for tag in note.tags {
                *counts.entry(tag).or_insert(0) += 1;
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
        let pubkey = self.dialog.public_key().to_hex();
        if let Err(e) = dialog_lib::clean_test_storage(&pubkey) {
            eprintln!("[uniffi] clear_data_for_current_pubkey error: {e}");
        }
    }

    // Instance helpers expected by UniFFI UDL
    pub fn stop(&self) {
        let mut guard = rt().block_on(self.watch_handle.write());
        if let Some(handle) = guard.take() {
            handle.abort();
        }
        drop(guard);

        let mut listener = rt().block_on(self.listener_handle.write());
        if let Some(handle) = listener.take() {
            handle.abort();
        }
    }

    pub fn validate_nsec(&self, nsec: String) -> bool {
        dialog_lib::validate_nsec(&nsec).is_ok()
    }

    pub fn derive_npub(&self, nsec: String) -> String {
        use nostr_sdk::{ToBech32, prelude::Keys};
        match Keys::parse(&nsec) {
            Ok(keys) => keys.public_key().to_bech32().unwrap_or_default(),
            Err(_) => String::new(),
        }
    }

    pub(crate) async fn fetch_notes_async(&self, limit: u32, tag: Option<String>) -> Vec<Note> {
        let tag_normalized = tag.map(|t| t.to_lowercase());
        let result = if let Some(ref tag_value) = tag_normalized {
            self.dialog.list_by_tag(tag_value, limit as usize).await
        } else {
            self.dialog.list_notes(limit as usize).await
        };

        match result {
            Ok(notes) => notes.into_iter().map(convert_lib_note_to_uniffi).collect(),
            Err(err) => {
                eprintln!("[uniffi] fetch_notes error: {err}");
                Vec::new()
            }
        }
    }

    pub(crate) async fn fetch_note_async(&self, event_id: &EventId) -> Option<Note> {
        match self.dialog.get_note(event_id).await {
            Ok(Some(note)) => Some(convert_lib_note_to_uniffi(note)),
            Ok(None) => None,
            Err(err) => {
                eprintln!("[uniffi] fetch_note error: {err}");
                None
            }
        }
    }
}

pub trait DialogListener: Send + Sync {
    fn on_event(&self, event: Event);
}
