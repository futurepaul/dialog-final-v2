use crate::{convert::convert_lib_note_to_uniffi, runtime::{rt, DIALOG}, DialogClient, Event};
use std::sync::Arc;

impl DialogClient {
    pub(crate) async fn maybe_start_watch(self: Arc<Self>) {
        if self.watch_handle.read().await.is_some() {
            return;
        }
        match DIALOG.get().unwrap().watch_notes().await {
            Ok(mut receiver) => {
                eprintln!("[uniffi] watch_notes receiver acquired; entering loop");
                let this = self.clone();
                let handle = rt().spawn(async move {
                    while let Some(lib_note) = receiver.recv().await {
                        let note = convert_lib_note_to_uniffi(lib_note);
                        let mut notes_guard = this.notes.write().await;
                        if notes_guard.contains_key(&note.id) {
                            notes_guard.insert(note.id.clone(), note.clone());
                            let _ = this.event_tx.send(Event::NoteUpdated { note });
                        } else {
                            notes_guard.insert(note.id.clone(), note.clone());
                            let _ = this.event_tx.send(Event::NoteAdded { note });
                        }
                    }
                });
                *self.watch_handle.write().await = Some(handle);
            }
            Err(e) => {
                eprintln!("[uniffi] watch_notes() failed to start: {e}");
            }
        }
    }
}

