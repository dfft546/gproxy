use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use tokio::sync::{RwLock, broadcast};

use super::types::Event;

pub trait EventSink: Send + Sync {
    fn write<'a>(&'a self, event: &'a Event) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
}

#[derive(Clone)]
pub struct EventHub {
    inner: Arc<Inner>,
}

struct Inner {
    tx: broadcast::Sender<Event>,
    sinks: RwLock<Vec<Arc<dyn EventSink>>>,
}

impl EventHub {
    pub fn new(buffer: usize) -> Self {
        let (tx, _) = broadcast::channel(buffer);
        Self {
            inner: Arc::new(Inner {
                tx,
                sinks: RwLock::new(Vec::new()),
            }),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.inner.tx.subscribe()
    }

    pub async fn add_sink(&self, sink: Arc<dyn EventSink>) {
        self.inner.sinks.write().await.push(sink);
    }

    pub async fn emit(&self, event: Event) {
        let _ = self.inner.tx.send(event.clone());
        let sinks = self.inner.sinks.read().await.clone();
        for sink in sinks {
            let event_ref = event.clone();
            tokio::spawn(async move {
                sink.write(&event_ref).await;
            });
        }
    }
}
