use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use gproxy_provider_core::{Event, EventSink};

use crate::Storage;

/// Persist events into DB via `Storage::append_event`.
pub struct DbEventSink<S: Storage> {
    storage: Arc<S>,
}

impl<S: Storage> DbEventSink<S> {
    pub fn new(storage: Arc<S>) -> Self {
        Self { storage }
    }
}

impl<S: Storage> EventSink for DbEventSink<S> {
    fn write<'a>(&'a self, event: &'a Event) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            // Event persistence must not block the request path; best-effort is fine.
            let _ = self.storage.append_event(event).await;
        })
    }
}
