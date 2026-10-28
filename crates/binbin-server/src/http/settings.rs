use super::{Result, auth::identity, extractors::Json, work};
use axum::{extract::State, http::HeaderMap};
use binbin_core::{App, CHUNK, models::Settings};
use serde_json::{Value, json};
pub(super) async fn settings(State(app): State<App>, headers: HeaderMap) -> Result<Json<Value>> {
    work(app, move |app| {
        identity(&app, &headers, false)?;
        Ok(Json(
            json!({"settings":app.settings()?,"used":app.storage_usage()?,"chunk_size":CHUNK}),
        ))
    })
    .await
}
pub(super) async fn update_settings(
    State(app): State<App>,
    headers: HeaderMap,
    Json(settings): Json<Settings>,
) -> Result<Json<Settings>> {
    work(app, move |app| {
        let actor = identity(&app, &headers, true)?;
        let settings = app.update_settings(&actor, settings)?;
        tracing::info!(event = "settings_updated", user_id = %actor.user.id, "Instance settings updated");
        Ok(Json(settings))
    })
    .await
}
