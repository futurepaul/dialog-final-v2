use crate::{Command, DialogClient, Event, SyncMode, runtime::rt};
use dialog_lib::CatchupMethod;
use nostr_sdk::prelude::*;
use std::sync::Arc;

impl DialogClient {
    pub fn send_command(self: Arc<Self>, cmd: Command) {
        let self_clone = self.clone();
        eprintln!("[uniffi] send_command: {cmd:?}");
        rt().spawn(async move {
            match cmd {
                Command::ConnectRelay { relay_url } => {
                    self_clone.handle_connect(relay_url).await;
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
                    self_clone.refresh_notes(limit).await;
                }
                Command::DeleteNote { id } => {
                    self_clone.delete_note(id).await;
                }
                Command::SearchNotes { query } => {
                    self_clone.search_notes(query).await;
                }
                Command::SetSyncMode { mode } => {
                    eprintln!("[uniffi] SetSyncMode to {mode:?}");
                    *self_clone.sync_mode.write().await = mode;
                }
            }
        });
    }

    async fn handle_connect(self: &Arc<Self>, relay_url: String) {
        eprintln!("[uniffi] Connecting to relay: {relay_url}");
        if let Err(e) = self.dialog.connect_relay(&relay_url).await {
            eprintln!("[uniffi] Failed to connect to relay: {e}");
            return;
        }

        eprintln!("[uniffi] Connected to relay: {relay_url}");
        let mode = self.sync_mode.read().await.clone();
        match mode {
            SyncMode::Negentropy => match self.dialog.initial_catchup().await {
                Ok(outcome) => match outcome.method {
                    CatchupMethod::Negentropy => {
                        eprintln!("[uniffi] Initial catch-up via Negentropy");
                    }
                    CatchupMethod::PlainFetch => {
                        eprintln!(
                            "[uniffi] Negentropy unavailable; fell back to plain fetch for catch-up"
                        );
                    }
                },
                Err(err) => {
                    eprintln!(
                        "[uniffi] Initial catch-up failed ({err}); operating with local cache"
                    );
                }
            },
            SyncMode::Subscribe => {
                eprintln!(
                    "[uniffi] SyncMode::Subscribe: performing plain fetch catch-up for compatibility"
                );
                if let Err(err) = self.dialog.sync_notes_plain(Some(500)).await {
                    eprintln!("[uniffi] Plain fetch failed: {err}");
                }
            }
        }

        self.refresh_notes(100).await;
        self.clone().maybe_start_watch().await;
    }

    pub(crate) async fn create_note(self: Arc<Self>, text: String) {
        eprintln!("[uniffi] CreateNote len={}", text.len());
        match self.dialog.create_note(&text).await {
            Ok(event_id) => {
                if let Some(note) = self.fetch_note_async(&event_id).await {
                    {
                        let mut delivered = self.delivered_ids.write().await;
                        delivered.insert(note.id.clone());
                    }
                    let _ = self.event_tx.send(Event::NoteAdded { note });
                }
            }
            Err(err) => {
                eprintln!("[uniffi] create_note() failed: {err}");
            }
        }
    }

    pub(crate) async fn set_filter(self: Arc<Self>, tag: Option<String>) {
        eprintln!("[uniffi] SetTagFilter tag={tag:?}");
        *self.current_filter.write().await = tag.clone().map(|t| t.to_lowercase());
        let _ = self
            .event_tx
            .send(Event::TagFilterChanged { tag: tag.clone() });
        self.refresh_notes(100).await;
    }

    pub(crate) async fn mark_as_read(self: Arc<Self>, id: String) {
        if let Ok(event_id) = EventId::from_hex(&id) {
            if self.dialog.mark_as_read(&event_id).await.is_ok() {
                if let Some(note) = self.fetch_note_async(&event_id).await {
                    let _ = self.event_tx.send(Event::NoteUpdated { note });
                }
            }
        }
    }

    pub(crate) async fn delete_note(self: Arc<Self>, id: String) {
        {
            let mut delivered = self.delivered_ids.write().await;
            delivered.remove(&id);
        }
        let _ = self.event_tx.send(Event::NoteDeleted { id });
    }

    pub(crate) async fn search_notes(self: Arc<Self>, query: String) {
        eprintln!("[uniffi] SearchNotes query='{query}'");
        let query_lower = query.to_lowercase();
        let notes = self.fetch_notes_async(1_000, None).await;
        let results: Vec<crate::Note> = notes
            .into_iter()
            .filter(|n| n.text.to_lowercase().contains(&query_lower))
            .collect();
        let _ = self.event_tx.send(Event::NotesLoaded { notes: results });
    }

    async fn refresh_notes(self: &Arc<Self>, limit: u32) {
        let filter = self.current_filter.read().await.clone();
        let notes = self.fetch_notes_async(limit, filter.clone()).await;
        {
            let mut delivered = self.delivered_ids.write().await;
            for note in &notes {
                delivered.insert(note.id.clone());
            }
        }
        let _ = self.event_tx.send(Event::NotesLoaded { notes });
    }
}
