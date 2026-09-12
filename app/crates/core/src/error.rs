use serde::Serialize;

/// Unified error type surfaced to the frontend (Tauri command result or HTTP body).
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("not signed in")]
    NotAuthenticated,
    #[error("authorization pending")]
    AuthorizationPending,
    #[error("authentication failed: {0}")]
    Auth(String),
    #[error("graph request failed ({status}): {message}")]
    Graph { status: u16, message: String },
    #[error("unknown object type: {0}")]
    UnknownType(String),
    #[error("io error: {0}")]
    Io(String),
    #[error("{0}")]
    Other(String),
}

impl CoreError {
    pub fn code(&self) -> &'static str {
        match self {
            CoreError::NotAuthenticated => "not_authenticated",
            CoreError::AuthorizationPending => "authorization_pending",
            CoreError::Auth(_) => "auth_failed",
            CoreError::Graph { .. } => "graph_error",
            CoreError::UnknownType(_) => "unknown_type",
            CoreError::Io(_) => "io_error",
            CoreError::Other(_) => "error",
        }
    }
}

/// JSON-serializable error shape shared by every transport.
#[derive(Serialize)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
}

impl From<&CoreError> for ErrorBody {
    fn from(e: &CoreError) -> Self {
        ErrorBody { code: e.code().to_string(), message: e.to_string() }
    }
}

impl Serialize for CoreError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        ErrorBody::from(self).serialize(s)
    }
}

impl From<reqwest::Error> for CoreError {
    fn from(e: reqwest::Error) -> Self {
        CoreError::Other(format!("http: {e}"))
    }
}
impl From<std::io::Error> for CoreError {
    fn from(e: std::io::Error) -> Self {
        CoreError::Io(e.to_string())
    }
}

pub type CoreResult<T> = Result<T, CoreError>;
