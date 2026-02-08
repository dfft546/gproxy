use std::collections::HashMap;
use std::sync::Arc;

use crate::UpstreamProvider;

#[derive(Default)]
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn UpstreamProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, provider: Arc<dyn UpstreamProvider>) {
        self.providers.insert(provider.name().to_string(), provider);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn UpstreamProvider>> {
        self.providers.get(name).cloned()
    }
}
