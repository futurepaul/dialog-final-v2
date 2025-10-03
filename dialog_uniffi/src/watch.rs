use crate::{DialogClient, Event, runtime::rt};
use dialog_lib::{ChangeEvent, RelayStatus};
use std::sync::Arc;

impl DialogClient {
    pub(crate) async fn maybe_start_watch(self: Arc<Self>) {
        if self.watch_handle.read().await.is_some() {
            return;
        }

        match self.dialog.watch_changes().await {
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

                                match this.fetch_note_async(&event_id).await {
                                    Some(note) => {
                                        let _ = this.event_tx.send(Event::NoteAdded { note });
                                    }
                                    None => {
                                        eprintln!(
                                            "[uniffi] watch_changes: received note id {id_hex} but not found in DB"
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
