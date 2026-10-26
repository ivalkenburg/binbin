use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
#[derive(Debug)]
pub(super) struct Error(pub u16, pub String);
type CoreError = binbin_core::Error;
impl From<CoreError> for Error {
    fn from(error: CoreError) -> Self {
        Self(error.0, error.1)
    }
}
impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        CoreError::from(error).into()
    }
}
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (
            StatusCode::from_u16(self.0).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(json!({"error":self.1})),
        )
            .into_response()
    }
}
