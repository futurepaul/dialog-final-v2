use crate::{Dialog, DialogError, Result};
use nostr_sdk::prelude::*;
use std::collections::HashSet;
use std::time::Duration;
use tokio::sync::mpsc;

const DB_CONFIRM_ATTEMPTS: usize = 10;
const DB_CONFIRM_DELAY_MS: u64 = 25;

#[derive(Debug, Clone)]
pub enum CatchupMethod {
    Negentropy,
    PlainFetch,
}

#[derive(Debug, Clone)]
pub struct CatchupOutcome {
    pub method: CatchupMethod,
}

#[derive(Debug, Clone)]
pub enum ChangeEvent {
    NoteApplied { event_id: EventId },
    RelayStatus { status: RelayStatus },
}

#[derive(Debug, Clone)]
pub enum RelayStatus {
    Subscribed,
    Disconnected,
}

impl Dialog {
    pub async fn initial_catchup(&self) -> Result<CatchupOutcome> {
        match self.sync_notes().await {
            Ok(_) => Ok(CatchupOutcome {
                method: CatchupMethod::Negentropy,
            }),
            Err(err) => {
                eprintln!(
                    "[lib] initial_catchup: negentropy sync failed: {err}; falling back to plain fetch"
                );
                self.sync_notes_plain(None).await.map_err(|fallback_err| {
                    DialogError::Database(format!(
                        "fallback plain fetch failed after negentropy error ({err}): {fallback_err}"
                    ))
                })?;
                Ok(CatchupOutcome {
                    method: CatchupMethod::PlainFetch,
                })
            }
        }
    }

    pub async fn watch_changes(&self) -> Result<mpsc::Receiver<ChangeEvent>> {
        let (tx, rx) = mpsc::channel(256);

        let client = self.client.clone();
        let author = self.keys.public_key();
        let mut since = self
            .latest_note_timestamp()
            .await?
            .map(|ts| Timestamp::from(ts.as_u64().saturating_add(1)))
            .unwrap_or_else(Timestamp::now);

        tokio::spawn(async move {
            let mut notifications = client.notifications();
            let mut seen = HashSet::new();
            let note_kind = Kind::from(1059);

            loop {
                let filter = Filter::new().author(author).kind(note_kind).since(since);

                let subscription_id = match client.subscribe(vec![filter], None).await {
                    Ok(output) => {
                        let _ = tx
                            .send(ChangeEvent::RelayStatus {
                                status: RelayStatus::Subscribed,
                            })
                            .await;
                        output.val
                    }
                    Err(err) => {
                        eprintln!("[lib] watch_changes: subscribe failed: {err}");
                        let _ = tx
                            .send(ChangeEvent::RelayStatus {
                                status: RelayStatus::Disconnected,
                            })
                            .await;
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        continue;
                    }
                };

                loop {
                    match notifications.recv().await {
                        Ok(RelayPoolNotification::Message { message, .. }) => match message {
                            RelayMessage::Event {
                                subscription_id: sub_id,
                                event,
                            } if sub_id == subscription_id => {
                                process_event(
                                    &client, &mut seen, &mut since, author, note_kind, *event, &tx,
                                )
                                .await;
                            }
                            RelayMessage::Closed {
                                subscription_id: sub_id,
                                ..
                            } if sub_id == subscription_id => {
                                let _ = tx
                                    .send(ChangeEvent::RelayStatus {
                                        status: RelayStatus::Disconnected,
                                    })
                                    .await;
                                break;
                            }
                            _ => {}
                        },
                        Ok(RelayPoolNotification::Event {
                            subscription_id: sub_id,
                            event,
                            ..
                        }) if sub_id == subscription_id => {
                            process_event(
                                &client, &mut seen, &mut since, author, note_kind, *event, &tx,
                            )
                            .await;
                        }
                        Ok(RelayPoolNotification::Shutdown) => {
                            let _ = tx
                                .send(ChangeEvent::RelayStatus {
                                    status: RelayStatus::Disconnected,
                                })
                                .await;
                            return;
                        }
                        Err(err) => {
                            eprintln!("[lib] watch_changes: notifications channel closed: {err}");
                            let _ = tx
                                .send(ChangeEvent::RelayStatus {
                                    status: RelayStatus::Disconnected,
                                })
                                .await;
                            return;
                        }
                        _ => {}
                    }
                }
            }
        });

        Ok(rx)
    }
}

async fn process_event(
    client: &Client,
    seen: &mut HashSet<EventId>,
    since: &mut Timestamp,
    author: PublicKey,
    note_kind: Kind,
    event: Event,
    tx: &mpsc::Sender<ChangeEvent>,
) {
    if event.kind != note_kind || event.pubkey != author {
        return;
    }

    if !seen.insert(event.id) {
        return;
    }

    if let Err(err) = client.database().save_event(&event).await {
        let message = err.to_string();
        if !message.contains("MDB_KEYEXIST") {
            eprintln!(
                "[lib] watch_changes: failed to persist event {}: {message}",
                event.id
            );
        }
    }

    let mut confirmed = false;
    for attempt in 0..DB_CONFIRM_ATTEMPTS {
        match client
            .database()
            .query(vec![Filter::new().ids(vec![event.id])])
            .await
        {
            Ok(results) => {
                if results.into_iter().any(|e| e.id == event.id) {
                    confirmed = true;
                    break;
                }
            }
            Err(err) => {
                eprintln!(
                    "[lib] watch_changes: confirmation query failed for {}: {err}",
                    event.id
                );
                break;
            }
        }

        if attempt + 1 < DB_CONFIRM_ATTEMPTS {
            tokio::time::sleep(Duration::from_millis(DB_CONFIRM_DELAY_MS)).await;
        }
    }

    if !confirmed {
        eprintln!(
            "[lib] watch_changes: event {} not visible after confirmation window",
            event.id
        );
    }

    let next_since = event.created_at.as_u64().saturating_add(1);
    let candidate = Timestamp::from(next_since);
    if candidate > *since {
        *since = candidate;
    }

    let _ = tx
        .send(ChangeEvent::NoteApplied { event_id: event.id })
        .await;
}
