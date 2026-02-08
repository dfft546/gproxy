mod model_unavailable_queue;
mod pool;
mod state;
mod unavailable_queue;

pub use pool::{AcquireError, CredentialPool};
pub use state::{CredentialId, CredentialState, UnavailableReason};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Credential {
    OpenAI(ApiKeyCredential),
    Claude(ApiKeyCredential),
    AIStudio(ApiKeyCredential),
    VertexExpress(ApiKeyCredential),
    Vertex(ServiceAccountCredential),
    GeminiCli(GeminiCliCredential),
    ClaudeCode(ClaudeCodeCredential),
    Codex(CodexCredential),
    Antigravity(AntigravityCredential),
    Nvidia(ApiKeyCredential),
    DeepSeek(ApiKeyCredential),
    Custom(ApiKeyCredential),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyCredential {
    pub api_key: String,
}

/// Google Service Account JSON fields used by Vertex.
/// Extra metadata fields are kept for round-trip compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceAccountCredential {
    pub project_id: String,
    pub client_email: String,
    pub private_key: String,
    pub private_key_id: String,
    pub client_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_provider_x509_cert_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_x509_cert_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub universe_domain: Option<String>,
    pub access_token: String,
    pub expires_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiCliCredential {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
    pub project_id: String,
    pub client_id: String,
    pub client_secret: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexCredential {
    pub access_token: String,
    pub refresh_token: String,
    pub id_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_email: Option<String>,
    pub account_id: String,
    pub expires_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeCodeCredential {
    #[serde(default, alias = "accessToken")]
    pub access_token: String,
    #[serde(default, alias = "refreshToken")]
    pub refresh_token: String,
    #[serde(default)]
    pub expires_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_claude_1m_sonnet: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_claude_1m_opus: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supports_claude_1m_sonnet: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supports_claude_1m_opus: Option<bool>,
    #[serde(default, alias = "subscriptionType")]
    pub subscription_type: String,
    #[serde(default, alias = "rateLimitTier")]
    pub rate_limit_tier: String,
    #[serde(default, alias = "sessionKey", skip_serializing_if = "Option::is_none")]
    pub session_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntigravityCredential {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
    pub project_id: String,
    pub client_id: String,
    pub client_secret: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_email: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claudecode_allows_session_key_only() {
        let value = serde_json::json!({
            "ClaudeCode": {
                "session_key": "sess_123"
            }
        });
        let cred: Credential = serde_json::from_value(value).expect("credential should parse");
        match cred {
            Credential::ClaudeCode(secret) => {
                assert_eq!(secret.access_token, "");
                assert_eq!(secret.refresh_token, "");
                assert_eq!(secret.expires_at, 0);
                assert_eq!(secret.session_key.as_deref(), Some("sess_123"));
            }
            other => panic!("unexpected credential variant: {other:?}"),
        }
    }
}
