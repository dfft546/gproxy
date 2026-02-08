//! Built-in upstream provider implementations.
//!
//! This crate does not perform network IO. It builds `UpstreamHttpRequest` for
//! upstream calls (including provider-specific internal calls like `upstream_usage`).

mod auth_extractor;
mod builtin;
mod providers;
mod registry;

pub use builtin::{BuiltinProviderSeed, builtin_provider_seeds};
pub use registry::register_builtin_providers;
