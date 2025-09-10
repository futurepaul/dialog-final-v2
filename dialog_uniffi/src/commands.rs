use crate::{convert::convert_lib_note_to_uniffi, runtime::{rt, DIALOG}, Event, Command, SyncMode, DialogClient};
use nostr_sdk::prelude::*;
use std::sync::Arc;

impl DialogClient {
    pub fn send_command(self: Arc<Self>, cmd: Command) {
        let self_clone = self.clone();
        eprintln!("[uniffi] send_command: {cmd:?}");
        rt().spawn(async move {
            match cmd {
                Command::ConnectRelay { relay_url } => {
                    eprintln!("[uniffi] Connecting to relay: {relay_url}");
                    if let Err(e) = DIALOG.get().unwrap().connect_relay(&relay_url).await {
                        eprintln!("[uniffi] Failed to connect to relay: {e}");
                    } else {
                        eprintln!("[uniffi] Connected to relay: {relay_url}");
                        // After connecting, either Negentropy sync or plain subscribe based on mode
                        // Decide sync approach
                        let mut mode = self_clone.sync_mode.write().await;
                        match *mode {
                            SyncMode::Negentropy => {
                                match DIALOG.get().unwrap().sync_notes().await {
                                    Ok(_) => {}
                                    Err(e) => {
                                        eprintln!("[uniffi] Negentropy sync failed: {e}; falling back to plain subscribe fetch");
                                        if let Err(e2) = DIALOG.get().unwrap().sync_notes_plain(Some(500)).await {
                                            eprintln!("[uniffi] Plain fetch also failed: {e2}");
                                        } else {
                                            *mode = SyncMode::Subscribe;
                                        }
                                    }
                                }
                            }
                            SyncMode::Subscribe => {
                                eprintln!("[uniffi] Using plain subscribe mode; performing initial fetch");
                                if let Err(e) = DIALOG.get().unwrap().sync_notes_plain(Some(500)).await {
                                    eprintln!("[uniffi] Plain fetch failed: {e}");
                                }
                            }
                        }
                        drop(mode);
                        // Load updated notes and emit NotesLoaded from local cache
                        if let Ok(lib_notes) = DIALOG.get().unwrap().list_notes(100).await {
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
                        // Ensure watch loop is running
                        self_clone.maybe_start_watch().await;
                    }
                }
                Command::CreateNote { text } => {
                    eprintln!("[uniffi] CreateNote len={}", text.len());
                    self_clone.create_note(text).await;
                }
                Command::SetTagFilter { tag } => {
                    eprintln!("[uniffi] SetTagFilter tag={tag:?}");
                    self_clone.set_filter(tag).await;
                }
                Command::MarkAsRead { id } => {
                    eprintln!("[uniffi] MarkAsRead id={id}");
                    self_clone.mark_as_read(id).await;
                }
                Command::LoadNotes { limit } => {
                    eprintln!("[uniffi] LoadNotes limit={limit} (sync from dialog_lib)");
                    if let Ok(lib_notes) = DIALOG.get().unwrap().list_notes(limit as usize).await {
                        let mut notes_map = self_clone.notes.write().await;
                        let mut notes = Vec::new();
                        for lib_note in lib_notes {
                            let note = convert_lib_note_to_uniffi(lib_note);
                            notes_map.insert(note.id.clone(), note.clone());
                            notes.push(note);
                        }
                        let filter = self_clone.current_filter.read().await.clone();
                        if let Some(tag) = filter {
                            notes.retain(|n| n.tags.contains(&tag));
                        }
                        let _ = self_clone.event_tx.send(Event::NotesLoaded { notes });
                    } else {
                        eprintln!("[uniffi] list_notes failed");
                    }
                }
                Command::DeleteNote { id } => {
                    eprintln!("[uniffi] DeleteNote id={id}");
                    self_clone.delete_note(id).await;
                }
                Command::SearchNotes { query } => {
                    eprintln!("[uniffi] SearchNotes query='{query}'");
                    self_clone.search_notes(query).await;
                }
                Command::SetSyncMode { mode } => {
                    eprintln!("[uniffi] SetSyncMode to {mode:?}");
                    *self_clone.sync_mode.write().await = mode;
                }
            }
        });
    }

    // Private async helpers
    pub(crate) async fn create_note(self: Arc<Self>, text: String) {
        match DIALOG.get().unwrap().create_note(&text).await {
            Ok(note_id) => {
                let tags: Vec<String> = text
                    .split_whitespace()
                    .filter(|w| w.starts_with('#') && w.len() > 1)
                    .map(|t| t[1..].to_lowercase())
                    .collect();
                let note = crate::Note {
                    id: note_id.to_hex(),
                    text: text.clone(),
                    tags,
                    created_at: nostr_sdk::prelude::Timestamp::now().as_u64() as i64,
                    is_read: false,
                    is_synced: false,
                };
                self.notes.write().await.insert(note.id.clone(), note.clone());
                let _ = self.event_tx.send(Event::NoteAdded { note });
            }
            Err(e) => {
                eprintln!("[uniffi] create_note() failed: {e}");
            }
        }
    }

    pub(crate) async fn set_filter(self: Arc<Self>, tag: Option<String>) {
        *self.current_filter.write().await = tag.clone();
        let _ = self.event_tx.send(Event::TagFilterChanged { tag: tag.clone() });
        let notes = self.get_notes(100, tag);
        let _ = self.event_tx.send(Event::NotesLoaded { notes });
    }

    pub(crate) async fn mark_as_read(self: Arc<Self>, id: String) {
        if let Ok(event_id) = EventId::from_hex(&id) {
            if (DIALOG.get().unwrap().mark_as_read(&event_id).await).is_ok() {
                let mut notes = self.notes.write().await;
                if let Some(note) = notes.get_mut(&id) {
                    note.is_read = true;
                    let _ = self.event_tx.send(Event::NoteUpdated { note: note.clone() });
                }
            }
        }
    }

    pub(crate) async fn delete_note(self: Arc<Self>, id: String) {
        let mut notes = self.notes.write().await;
        if notes.remove(&id).is_some() {
            let _ = self.event_tx.send(Event::NoteDeleted { id });
        }
    }

    pub(crate) async fn search_notes(self: Arc<Self>, query: String) {
        let notes = self.notes.read().await;
        let query_lower = query.to_lowercase();
        let results: Vec<crate::Note> = notes
            .values()
            .filter(|n| n.text.to_lowercase().contains(&query_lower))
            .cloned()
            .collect();
        let _ = self.event_tx.send(Event::NotesLoaded { notes: results });
    }
}

