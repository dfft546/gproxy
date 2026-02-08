use std::error::Error;
use std::fmt;

pub type ProviderResult<T> = Result<T, ProviderError>;

#[derive(Debug, Clone)]
pub enum ProviderError {
    Unsupported(&'static str),
    InvalidConfig(String),
    MissingCredentialField(&'static str),
    Other(String),
}

impl fmt::Display for ProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProviderError::Unsupported(what) => write!(f, "unsupported: {what}"),
            ProviderError::InvalidConfig(msg) => write!(f, "invalid config: {msg}"),
            ProviderError::MissingCredentialField(field) => {
                write!(f, "missing credential field: {field}")
            }
            ProviderError::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl Error for ProviderError {}
