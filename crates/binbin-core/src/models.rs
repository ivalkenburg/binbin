use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub password: String,
    pub admin: bool,
    pub enabled: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credential {
    pub id: String,
    pub user: String,
    pub name: String,
    pub csrf: String,
    pub session: bool,
    pub expires: i64,
}
#[derive(Debug, Clone)]
pub struct Identity {
    pub user: User,
    pub credential: Credential,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub file_limit: u64,
    pub share_limit: u64,
    pub storage_limit: u64,
    pub file_count: usize,
    pub default_days: Option<u32>,
    pub max_days: Option<u32>,
    pub max_downloads: Option<u64>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            file_limit: 5 << 30,
            share_limit: 20 << 30,
            storage_limit: 100 << 30,
            file_count: 100,
            default_days: Some(7),
            max_days: None,
            max_downloads: None,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Share {
    pub id: String,
    pub owner: String,
    pub title: String,
    pub state: String,
    pub created: i64,
    pub published: Option<i64>,
    pub expires: Option<i64>,
    pub days: Option<u32>,
    pub limit: Option<u64>,
    pub used: u64,
    pub password: Option<String>,
    pub version: u64,
    pub reserved: u64,
    pub original_size: u64,
    pub zip_size: u64,
    pub zip_hash: Option<String>,
    pub removed: Option<i64>,
    pub reason: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredFile {
    pub id: String,
    pub share: String,
    pub name: String,
    pub size: u64,
    pub offset: u64,
    pub sha256: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transfer {
    pub id: String,
    pub share: String,
    pub resource: String,
    pub version: u64,
    pub created: i64,
    pub expires: i64,
    pub charged: bool,
    pub management: bool,
    #[serde(default)]
    pub credential: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewFile {
    pub name: String,
    pub size: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewShare {
    pub title: String,
    pub files: Vec<NewFile>,
    pub days: Option<u32>,
    pub limit: Option<u64>,
    pub password: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditShare {
    pub title: String,
    pub expires: Option<i64>,
    pub limit: Option<u64>,
    pub password: Option<String>,
    #[serde(default)]
    pub change_password: bool,
}
#[derive(Debug, Clone, Serialize)]
pub struct ShareView {
    #[serde(flatten)]
    pub share: serde_json::Value,
    pub files: Vec<StoredFile>,
    pub url: String,
}
