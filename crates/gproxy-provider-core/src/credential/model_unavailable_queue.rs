use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;
use std::time::SystemTime;

use tokio::sync::{Mutex, Notify, RwLock};
use tokio::time::{Instant, sleep_until};

use crate::EventHub;
use crate::events::{Event, ModelUnavailableEndEvent, OperationalEvent};

use super::state::CredentialId;

type ModelKey = (CredentialId, String);

#[derive(Debug)]
pub struct ModelUnavailableQueue {
    heap: Mutex<BinaryHeap<Reverse<(Instant, CredentialId, String)>>>,
    notify: Notify,
}

impl ModelUnavailableQueue {
    pub fn new() -> Self {
        Self {
            heap: Mutex::new(BinaryHeap::new()),
            notify: Notify::new(),
        }
    }

    pub async fn push(&self, until: Instant, credential_id: CredentialId, model: String) {
        {
            let mut heap = self.heap.lock().await;
            heap.push(Reverse((until, credential_id, model)));
        }
        self.notify.notify_one();
    }

    pub fn spawn_recover_task(
        self: Arc<Self>,
        states: Arc<RwLock<HashMap<ModelKey, (Instant, crate::UnavailableReason)>>>,
        events: EventHub,
    ) {
        tokio::spawn(async move {
            loop {
                let next = {
                    let heap = self.heap.lock().await;
                    heap.peek()
                        .map(|Reverse((t, id, model))| (*t, *id, model.clone()))
                };

                match next {
                    None => {
                        self.notify.notified().await;
                        continue;
                    }
                    Some((deadline, _id, _model)) => {
                        sleep_until(deadline).await;
                    }
                }

                let now = Instant::now();
                let mut due: Vec<(Instant, CredentialId, String)> = Vec::new();

                {
                    let mut heap = self.heap.lock().await;
                    while let Some(Reverse((t, id, model))) = heap.peek().cloned()
                        && t <= now
                    {
                        heap.pop();
                        due.push((t, id, model));
                    }
                }

                if due.is_empty() {
                    continue;
                }

                let mut guard = states.write().await;
                for (_t, id, model) in due {
                    let key = (id, model.clone());
                    let should_recover = match guard.get(&key) {
                        Some((until, _reason)) => *until <= now,
                        _ => false,
                    };
                    if should_recover {
                        guard.remove(&key);
                        events
                            .emit(Event::Operational(OperationalEvent::ModelUnavailableEnd(
                                ModelUnavailableEndEvent {
                                    credential_id: id,
                                    model,
                                    at: SystemTime::now(),
                                },
                            )))
                            .await;
                    }
                }
            }
        });
    }
}
