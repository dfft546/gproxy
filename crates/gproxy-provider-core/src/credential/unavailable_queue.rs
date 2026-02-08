use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;
use std::time::SystemTime;

use tokio::sync::{Mutex, Notify, RwLock};
use tokio::time::{Instant, sleep_until};

use crate::EventHub;
use crate::events::{Event, OperationalEvent, UnavailableEndEvent};

use super::state::{CredentialId, CredentialState};

#[derive(Debug)]
pub struct UnavailableQueue {
    heap: Mutex<BinaryHeap<Reverse<(Instant, CredentialId)>>>,
    notify: Notify,
}

impl UnavailableQueue {
    pub fn new() -> Self {
        Self {
            heap: Mutex::new(BinaryHeap::new()),
            notify: Notify::new(),
        }
    }

    pub async fn push(&self, until: Instant, credential_id: CredentialId) {
        {
            let mut heap = self.heap.lock().await;
            heap.push(Reverse((until, credential_id)));
        }
        // Always notify: the background task will re-compute the next deadline.
        self.notify.notify_one();
    }

    pub fn spawn_recover_task(
        self: Arc<Self>,
        states: Arc<RwLock<HashMap<CredentialId, CredentialState>>>,
        events: EventHub,
    ) {
        tokio::spawn(async move {
            loop {
                let next = {
                    let heap = self.heap.lock().await;
                    heap.peek().map(|Reverse((t, id))| (*t, *id))
                };

                match next {
                    None => {
                        self.notify.notified().await;
                        continue;
                    }
                    Some((deadline, _)) => {
                        sleep_until(deadline).await;
                    }
                }

                let now = Instant::now();
                let mut due: Vec<(Instant, CredentialId)> = Vec::new();

                {
                    let mut heap = self.heap.lock().await;
                    while let Some(Reverse((t, id))) = heap.peek().copied()
                        && t <= now
                    {
                        heap.pop();
                        due.push((t, id));
                    }
                }

                if due.is_empty() {
                    continue;
                }

                // Recover due credentials, but guard against stale queue entries.
                let mut guard = states.write().await;
                for (_t, id) in due {
                    let should_recover = match guard.get(&id) {
                        Some(CredentialState::Unavailable { until, .. }) => *until <= now,
                        _ => false,
                    };
                    if should_recover {
                        guard.insert(id, CredentialState::Active);
                        events
                            .emit(Event::Operational(OperationalEvent::UnavailableEnd(
                                UnavailableEndEvent {
                                    credential_id: id,
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
