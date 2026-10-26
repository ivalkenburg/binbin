//! Typed fields consumed by the client; unknown server fields remain compatible.
use serde::Deserialize;

#[derive(Deserialize)]
pub(super) struct File {
    pub id: String,
    pub name: String,
    pub size: u64,
    #[serde(default)]
    pub offset: u64,
    pub sha256: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct Share {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub state: String,
    pub files: Vec<File>,
    pub zip_hash: Option<String>,
    pub zip_size: u64,
}

#[derive(Deserialize)]
pub(super) struct UploadProgress {
    pub offset: u64,
}

#[derive(Deserialize)]
pub(super) struct Transfer {
    pub token: String,
}
