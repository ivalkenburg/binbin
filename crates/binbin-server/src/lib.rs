//! HTTP transport and runtime for Binbin.
mod http;

pub fn router(app: binbin_core::App) -> axum::Router {
    http::router(app)
}
mod runtime;
pub use runtime::run;
