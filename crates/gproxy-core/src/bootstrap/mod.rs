use std::collections::HashSet;
use std::sync::Arc;

use anyhow::Context;
use clap::Parser;

use gproxy_common::{GlobalConfig, GlobalConfigPatch};
use gproxy_provider_core::{EventHub, ProviderRegistry, TerminalEventSink};
use gproxy_provider_impl::builtin_provider_seeds;
use gproxy_provider_impl::register_builtin_providers;
use gproxy_storage::{DbEventSink, SeaOrmStorage, Storage};

use crate::state::AppState;

#[derive(Debug, Clone, Parser)]
#[command(
    name = "gproxy",
    version,
    about = "High-performance multi-provider LLM proxy"
)]
pub struct CliArgs {
    /// Database DSN (required to bootstrap the rest of config).
    #[arg(
        long,
        env = "GPROXY_DSN",
        default_value = "sqlite://gproxy.db?mode=rwc"
    )]
    pub dsn: String,

    /// Bind host.
    #[arg(long, env = "GPROXY_HOST")]
    pub host: Option<String>,

    /// Bind port.
    #[arg(long, env = "GPROXY_PORT")]
    pub port: Option<u16>,

    /// Admin key (plaintext). Stored as hash in DB and memory.
    #[arg(long, env = "GPROXY_ADMIN_KEY")]
    pub admin_key: Option<String>,

    /// Optional outbound proxy for upstream requests.
    #[arg(long, env = "GPROXY_PROXY")]
    pub proxy: Option<String>,

    /// Redact sensitive headers/body fields in emitted events.
    #[arg(long, env = "GPROXY_EVENT_REDACT_SENSITIVE")]
    pub event_redact_sensitive: Option<bool>,
}

pub struct Bootstrap {
    pub storage: Arc<SeaOrmStorage>,
    pub state: Arc<AppState>,
    pub registry: Arc<ProviderRegistry>,
}

pub async fn bootstrap_from_env() -> anyhow::Result<Bootstrap> {
    let args = CliArgs::parse();
    bootstrap(args).await
}

pub async fn bootstrap(args: CliArgs) -> anyhow::Result<Bootstrap> {
    // 1) connect DB from CLI/ENV DSN (required).
    let storage = Arc::new(
        SeaOrmStorage::connect(&args.dsn)
            .await
            .context("connect storage")?,
    );
    storage.sync().await.context("schema sync")?;

    // 2) load DB global config (if any), then merge once: CLI > ENV > DB.
    // clap already applies CLI > ENV precedence for each field; we then overlay on DB.
    let db_global = storage
        .load_global_config()
        .await
        .context("load db global_config")?;

    let mut merged = db_global
        .map(|row| GlobalConfigPatch::from(row.config))
        .unwrap_or_default();

    // Select admin key source:
    // - CLI/ENV provided key wins and overwrites DB (hash stored)
    // - else, if DB missing admin_key_hash, generate one and persist (print plaintext once)
    let mut admin_key_hash_override: Option<String> = None;
    if let Some(key_plain) = args.admin_key.as_deref() {
        admin_key_hash_override = Some(hash_admin_key(key_plain));
    } else if merged.admin_key_hash.is_none() {
        let key_plain = generate_admin_key();
        eprintln!("generated admin key: {key_plain}");
        admin_key_hash_override = Some(hash_admin_key(&key_plain));
    }

    let cli_patch = GlobalConfigPatch {
        host: args.host.clone(),
        port: args.port,
        admin_key_hash: admin_key_hash_override,
        proxy: args.proxy.clone(),
        dsn: Some(args.dsn.clone()),
        event_redact_sensitive: args.event_redact_sensitive,
    };
    merged.overlay(cli_patch);

    let global: GlobalConfig = merged
        .into_config()
        .context("finalize merged global config")?;

    // 3) persist merged global config back to DB.
    storage
        .upsert_global_config(&global)
        .await
        .context("upsert global_config")?;

    // 3.1) bootstrap default user/key if needed (user0 + admin key as API key).
    // Bootstrap default user/key if needed (user_id=0, name=user0).
    storage
        .upsert_user_by_id(0, "user0", true)
        .await
        .context("upsert user0")?;
    let user0_id = 0_i64;
    // If it already exists (unique constraint), ignore the error.
    let _ = storage
        .insert_user_key(user0_id, &global.admin_key_hash, Some("bootstrap"), true)
        .await;

    // 3.2) seed builtin providers (bulletin list) into storage if missing.
    let existing_provider_names: HashSet<String> = storage
        .provider_names()
        .await
        .context("list provider names")?
        .into_iter()
        .collect();

    for seed in builtin_provider_seeds() {
        if existing_provider_names.contains(seed.name) {
            continue;
        }
        storage
            .upsert_provider(seed.name, &seed.config_json, seed.enabled)
            .await
            .with_context(|| format!("seed provider {}", seed.name))?;
    }

    // 4) load the rest of data once (providers/credentials/users/keys).
    let snapshot = storage.load_snapshot().await.context("load snapshot")?;

    // 5) build in-memory state (all runtime reads come from here).
    let events = EventHub::new(1024);
    events.add_sink(Arc::new(TerminalEventSink::new())).await;
    events
        .add_sink(Arc::new(DbEventSink::new(storage.clone())))
        .await;
    let state = AppState::from_bootstrap(global, snapshot, events.clone())
        .await
        .context("build app state")?;

    Ok(Bootstrap {
        storage,
        state: Arc::new(state),
        registry: Arc::new({
            let mut r = ProviderRegistry::new();
            register_builtin_providers(&mut r);
            r
        }),
    })
}

fn hash_admin_key(key: &str) -> String {
    blake3::hash(key.as_bytes()).to_hex().to_string()
}

fn generate_admin_key() -> String {
    // Random enough for a bootstrap key; stored only in memory/printed once.
    uuid::Uuid::new_v4().to_string()
}
