use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use thiserror::Error;

/// Every failure the relay can surface over HTTP. Messages here are safe to
/// show to the mobile/desktop UI; nothing sensitive (password hashes,
/// tokens, SQL) is ever included.
#[derive(Debug, Error)]
pub enum RelayError {
    #[error("an account with that email already exists")]
    EmailTaken,
    #[error("invalid email or password")]
    InvalidCredentials,
    #[error("that session has expired or was signed out; please sign in again")]
    InvalidRefreshToken,
    #[error("missing or invalid authorization")]
    Unauthorized,
    #[error("device not found")]
    DeviceNotFound,
    #[error("that pairing code is invalid or has expired")]
    InvalidPairingCode,
    #[error("too many attempts; please wait a moment and try again")]
    RateLimited,
    #[error("{0}")]
    Validation(String),
    #[error("internal error")]
    Internal(String),
}

impl RelayError {
    fn code(&self) -> &'static str {
        match self {
            RelayError::EmailTaken => "EMAIL_TAKEN",
            RelayError::InvalidCredentials => "INVALID_CREDENTIALS",
            RelayError::InvalidRefreshToken => "INVALID_REFRESH_TOKEN",
            RelayError::Unauthorized => "UNAUTHORIZED",
            RelayError::DeviceNotFound => "DEVICE_NOT_FOUND",
            RelayError::InvalidPairingCode => "INVALID_PAIRING_CODE",
            RelayError::RateLimited => "RATE_LIMITED",
            RelayError::Validation(_) => "VALIDATION",
            RelayError::Internal(_) => "INTERNAL",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            RelayError::EmailTaken => StatusCode::CONFLICT,
            RelayError::InvalidCredentials
            | RelayError::InvalidRefreshToken
            | RelayError::Unauthorized => StatusCode::UNAUTHORIZED,
            RelayError::DeviceNotFound => StatusCode::NOT_FOUND,
            RelayError::InvalidPairingCode => StatusCode::BAD_REQUEST,
            RelayError::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            RelayError::Validation(_) => StatusCode::BAD_REQUEST,
            RelayError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Serialize)]
struct ErrorDetail {
    code: &'static str,
    message: String,
}

impl IntoResponse for RelayError {
    fn into_response(self) -> Response {
        if let RelayError::Internal(details) = &self {
            tracing::error!(error = %details, "internal relay error");
        }
        let status = self.status();
        let message = self.to_string();
        (
            status,
            Json(ErrorBody {
                error: ErrorDetail {
                    code: self.code(),
                    message,
                },
            }),
        )
            .into_response()
    }
}

impl From<rusqlite::Error> for RelayError {
    fn from(e: rusqlite::Error) -> Self {
        RelayError::Internal(e.to_string())
    }
}

pub type RelayResult<T> = Result<T, RelayError>;
