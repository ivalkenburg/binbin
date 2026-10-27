mod accounts;
mod assets;
mod auth;
mod downloads;
mod extractors;
mod filename;
mod middleware;
mod settings;
mod shares;
mod tokens;
mod users;

use axum::{
    Json, Router,
    extract::DefaultBodyLimit,
    http::StatusCode,
    middleware as axum_middleware,
    routing::{get, patch, post},
};
use binbin_core::{App, CHUNK, models::User};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use accounts::{login, logout, me};
use assets::{asset, index};
use downloads::download;
use middleware::{Guard, guard};
use settings::{settings, update_settings};
use shares::{
    create_share, edit, edit_files, image_preview, inspect, management_transfer, public_share,
    public_transfer, publish, revoke, shares, upload, upload_proofs,
};
use tokens::{create_token, delete_token, tokens};
use users::{create_user, update_user, users};

async fn work<T: Send + 'static>(
    app: App,
    f: impl FnOnce(App) -> binbin_core::Result<T> + Send + 'static,
) -> Result<T> {
    tokio::task::spawn_blocking(move || f(app))
        .await
        .map_err(|e| {
            tracing::error!("worker: {e}");
            Error(500, "Operation failed".into())
        })?
        .map_err(Into::into)
}
fn user_view(u: &User) -> Value {
    json!({"id":u.id,"username":u.username,"admin":u.admin,"enabled":u.enabled})
}

pub(crate) fn router(app: App) -> Router {
    let limiter = Arc::new(Mutex::new(HashMap::<String, (i64, u32)>::new()));
    let guard_state = Guard {
        app: app.clone(),
        limiter,
    };
    Router::new()
        .route("/", get(index))
        .route("/s/{id}", get(index))
        .route("/assets/{*path}", get(asset))
        .route("/healthz", get(|| async { Json(json!({"status":"ok"})) }))
        .route("/api/login", post(login))
        .route("/api/logout", post(logout))
        .route("/api/me", get(me))
        .route("/api/settings", get(settings).put(update_settings))
        .route("/api/users", get(users).post(create_user))
        .route("/api/users/{id}", patch(update_user))
        .route("/api/tokens", get(tokens).post(create_token))
        .route("/api/tokens/{id}", axum::routing::delete(delete_token))
        .route("/api/shares", get(shares).post(create_share))
        .route("/api/shares/{id}", get(inspect).patch(edit).delete(revoke))
        .route("/api/shares/{id}/publish", post(publish))
        .route("/api/shares/{id}/files", patch(edit_files))
        .route("/api/shares/{id}/transfers", post(management_transfer))
        .route(
            "/api/files/{id}",
            patch(upload).layer(DefaultBodyLimit::max(CHUNK)),
        )
        .route("/api/files/{id}/proofs", get(upload_proofs))
        .route("/api/files/{id}/preview", get(image_preview))
        .route("/api/public/{id}", post(public_share))
        .route("/api/public/{id}/transfers", post(public_transfer))
        .route("/d/{token}", get(download))
        .fallback(|| async { (StatusCode::NOT_FOUND, Json(json!({"error":"Not found"}))) })
        .layer(DefaultBodyLimit::max(4 * 1024 * 1024))
        .layer(axum_middleware::from_fn_with_state(guard_state, guard))
        .layer(tower_http::catch_panic::CatchPanicLayer::new())
        // Share permits across routes and router clones. A permit covers the
        // response future, not the subsequent streaming of its response body.
        .layer(request_limit())
        .with_state(app)
}

fn request_limit() -> tower::limit::GlobalConcurrencyLimitLayer {
    tower::limit::GlobalConcurrencyLimitLayer::new(16)
}

mod error;
use error::Error;
type Result<T> = std::result::Result<T, Error>;
#[cfg(test)]
mod tests;
