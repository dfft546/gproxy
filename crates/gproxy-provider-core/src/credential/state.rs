use tokio::time::Instant;

use serde::{Deserialize, Serialize};

pub type CredentialId = i64;

#[derive(Debug, Clone)]
pub enum CredentialState {
    Active,
    Unavailable {
        until: Instant,
        reason: UnavailableReason,
    },
}

impl CredentialState {
    pub fn is_active(&self) -> bool {
        matches!(self, CredentialState::Active)
    }

    pub fn unavailable_until(&self) -> Option<Instant> {
        match self {
            CredentialState::Unavailable { until, .. } => Some(*until),
            CredentialState::Active => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnavailableReason {
    RateLimit,
    Timeout,
    Upstream5xx,
    AuthInvalid,
    ModelDisallow,
    Manual,
    Unknown,
}
