use serde_json::Value;

use gproxy_provider_core::ProviderConfig;

#[derive(Debug, Clone)]
pub struct BuiltinProviderSeed {
    pub name: &'static str,
    pub enabled: bool,
    pub config_json: Value,
}

/// Built-in (a.k.a. "bulletin") providers to seed into storage on first run.
///
/// Notes:
/// - This is just seed data; actual DB writes happen in the bootstrap/app layer.
/// - `config_json` is intentionally minimal and primarily contains `kind` +
///   `channel_settings` defaults.
pub fn builtin_provider_seeds() -> Vec<BuiltinProviderSeed> {
    fn cfg_json(cfg: ProviderConfig) -> Value {
        serde_json::to_value(cfg).expect("serialize ProviderConfig")
    }

    vec![
        BuiltinProviderSeed {
            name: "openai",
            enabled: true,
            config_json: cfg_json(ProviderConfig::OpenAI(Default::default())),
        },
        BuiltinProviderSeed {
            name: "claude",
            enabled: true,
            config_json: cfg_json(ProviderConfig::Claude(Default::default())),
        },
        BuiltinProviderSeed {
            name: "aistudio",
            enabled: true,
            config_json: cfg_json(ProviderConfig::AIStudio(Default::default())),
        },
        BuiltinProviderSeed {
            name: "vertexexpress",
            enabled: true,
            config_json: cfg_json(ProviderConfig::VertexExpress(Default::default())),
        },
        BuiltinProviderSeed {
            name: "vertex",
            enabled: true,
            config_json: cfg_json(ProviderConfig::Vertex(Default::default())),
        },
        BuiltinProviderSeed {
            name: "geminicli",
            enabled: true,
            config_json: cfg_json(ProviderConfig::GeminiCli(Default::default())),
        },
        BuiltinProviderSeed {
            name: "claudecode",
            enabled: true,
            config_json: cfg_json(ProviderConfig::ClaudeCode(Default::default())),
        },
        BuiltinProviderSeed {
            name: "codex",
            enabled: true,
            config_json: cfg_json(ProviderConfig::Codex(Default::default())),
        },
        BuiltinProviderSeed {
            name: "antigravity",
            enabled: true,
            config_json: cfg_json(ProviderConfig::Antigravity(Default::default())),
        },
        BuiltinProviderSeed {
            name: "nvidia",
            enabled: true,
            config_json: cfg_json(ProviderConfig::Nvidia(Default::default())),
        },
        BuiltinProviderSeed {
            name: "deepseek",
            enabled: true,
            config_json: cfg_json(ProviderConfig::DeepSeek(Default::default())),
        },
    ]
}
