use thiserror::Error;

/// Unified error type for all cosmox-sdk operations.
#[derive(Debug, Clone, Error)]
pub enum SdkError {
    /// No valid authentication token is available.
    #[error("Not authenticated")]
    Unauthenticated,

    /// The server could not be reached (connection refused, timeout, DNS failure, etc.).
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    /// The server returned an unexpected HTTP status (4xx / 5xx).
    #[error("HTTP error ({status}): {message}")]
    HttpError { status: i32, message: String },

    /// JSON / rkyv serialisation or deserialisation failed.
    #[error("Serialization error: {0}")]
    SerdeError(String),

    /// Any other internal / unexpected error.
    #[error("{0}")]
    Internal(String),
}
