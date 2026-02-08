use std::sync::Arc;

use gproxy_provider_core::ProviderRegistry;

use crate::providers::{
    AIStudioProvider, AntigravityProvider, ClaudeCodeProvider, ClaudeProvider, CodexProvider,
    CustomProvider, DeepSeekProvider, GeminiCliProvider, NvidiaProvider, OpenAIProvider,
    VertexExpressProvider, VertexProvider,
};

pub fn register_builtin_providers(registry: &mut ProviderRegistry) {
    registry.register(Arc::new(CustomProvider::new()));
    registry.register(Arc::new(OpenAIProvider::new()));
    registry.register(Arc::new(ClaudeProvider::new()));
    registry.register(Arc::new(AIStudioProvider::new()));
    registry.register(Arc::new(VertexExpressProvider::new()));
    registry.register(Arc::new(VertexProvider::new()));
    registry.register(Arc::new(GeminiCliProvider::new()));
    registry.register(Arc::new(CodexProvider::new()));
    registry.register(Arc::new(ClaudeCodeProvider::new()));
    registry.register(Arc::new(AntigravityProvider::new()));
    registry.register(Arc::new(NvidiaProvider::new()));
    registry.register(Arc::new(DeepSeekProvider::new()));
}
