use super::{Result, auth::identity, extractors::Json, user_view, work};
use axum::{
    extract::{Path, State},
    http::HeaderMap,
};
use binbin_core::App;
use serde::Deserialize;
use serde_json::{Value, json};
pub(super) async fn users(State(app): State<App>, h: HeaderMap) -> Result<Json<Value>> {
    work(app, move |app| {
        let a = identity(&app, &h, false)?;
        Ok(Json(json!(
            app.users(&a)?.iter().map(user_view).collect::<Vec<_>>()
        )))
    })
    .await
}
#[derive(Deserialize)]
pub(super) struct CreateUser {
    username: String,
    password: String,
    #[serde(default)]
    admin: bool,
}
pub(super) async fn create_user(
    State(app): State<App>,
    h: HeaderMap,
    Json(n): Json<CreateUser>,
) -> Result<Json<Value>> {
    work(app, move |app| {
        let a = identity(&app, &h, true)?;
        Ok(Json(user_view(&app.create_user(
            &a,
            &n.username,
            &n.password,
            n.admin,
        )?)))
    })
    .await
}
#[derive(Deserialize)]
pub(super) struct UpdateUser {
    password: Option<String>,
    enabled: bool,
}
pub(super) async fn update_user(
    State(app): State<App>,
    h: HeaderMap,
    Path(id): Path<String>,
    Json(n): Json<UpdateUser>,
) -> Result<Json<Value>> {
    work(app, move |app| {
        let a = identity(&app, &h, true)?;
        app.change_user(&a, &id, n.password.as_deref(), n.enabled)?;
        Ok(Json(json!({"ok":true})))
    })
    .await
}
