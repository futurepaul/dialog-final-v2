use crate::{
    DialogClient, Event,
    convert::convert_lib_note_to_uniffi,
    runtime::{DIALOG, rt},
};
use dialog_lib::{ChangeEvent, RelayStatus};
use std::sync::Arc;

impl DialogClient {
    pub(crate) async fn maybe_start_watch(self: Arc<Self>) {
        if self.watch_handle.read().await.is_some() {
            return;
        }

        match DIALOG.get().unwrap().watch_changes().await {
            Ok(mut receiver) => {
                eprintln!("[uniffi] watch_changes receiver acquired; entering loop");
                let this = self.clone();
                let handle = rt().spawn(async move {
                    while let Some(change) = receiver.recv().await {
                        match change {
                            ChangeEvent::NoteApplied { event_id } => {
                                let id_hex = event_id.to_hex();
                                {
                                    let mut delivered = this.delivered_ids.write().await;
                                    if !delivered.insert(id_hex.clone()) {
                                        continue;
                                    }
                                }

                                match DIALOG
                                    .get()
                                    .unwrap()
                                    .get_note(&event_id)
                                    .await
                                {
                                    Ok(Some(lib_note)) => {
                                        let note = convert_lib_note_to_uniffi(lib_note);
                                        let _ = this.event_tx.send(Event::NoteAdded { note });
                                    }
                                    Ok(None) => {
                                        eprintln!(
                                            "[uniffi] watch_changes: received note id {id_hex} but not found in DB"
                                        );
                                    }
                                    Err(err) => {
                                        eprintln!(
                                            "[uniffi] watch_changes: failed to load note {id_hex}: {err}"
                                        );
                                    }
                                }
                            }
                            ChangeEvent::RelayStatus { status } => match status {
                                RelayStatus::Subscribed => {
                                    eprintln!(
                                        "[uniffi] watch_changes: relay subscription confirmed"
                                    );
                                    let _ = this
                                        .event_tx
                                        .send(Event::SyncStatusChanged { syncing: false });
                                }
                                RelayStatus::Disconnected => {
                                    eprintln!(
                                        "[uniffi] watch_changes: relay disconnected; waiting to resubscribe"
                                    );
                                    let _ = this
                                        .event_tx
                                        .send(Event::SyncStatusChanged { syncing: true });
                                }
                            },
                        }
                    }
                });
                *self.watch_handle.write().await = Some(handle);
            }
            Err(e) => {
                eprintln!("[uniffi] watch_changes() failed to start: {e}");
            }
        }
    }
}
