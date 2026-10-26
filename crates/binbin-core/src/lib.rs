//! Application rules and persistence shared by the Binbin server.

mod app;
pub mod crypto;
pub mod error;
pub mod models;
mod services;
mod storage;

pub use app::App;
pub use error::{Error, Result};
pub const DAY: i64 = 86_400;
pub const CHUNK: usize = 8 * 1024 * 1024;

#[cfg(test)]
mod tests;
