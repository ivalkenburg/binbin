pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug)]
pub struct Error(pub u16, pub String);
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.1)
    }
}
impl std::error::Error for Error {}
impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        tracing::error!("database: {e}");
        Self(500, "Database operation failed".into())
    }
}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        tracing::error!("storage: {e}");
        Self(
            507,
            "Storage operation failed; check available space and permissions".into(),
        )
    }
}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        tracing::error!("serialization: {e}");
        Self(500, "Stored data could not be read".into())
    }
}
pub fn bad(s: &str) -> Error {
    Error(400, s.into())
}
pub fn unavailable() -> Error {
    Error(404, "This share is unavailable".into())
}
