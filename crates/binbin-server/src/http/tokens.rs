use super::{Result, auth::identity, extractors::Json, work};
use axum::{
    extract::{Path, State},
    http::HeaderMap,
};
use binbin_core::App;
use serde::Deserialize;
use serde_json::{Value, json};
pub(super) async fn tokens(State(app): State<App>, h: HeaderMap) -> Result<Json<Value>> {
    work(app, move |app| {
        let a = identity(&app, &h, false)?;
        Ok(Json(json!(app.tokens(&a)?.iter().map(|credential|json!({"id":credential.id,"name":credential.name,"expires":credential.expires})).collect::<Vec<_>>())))
    })
    .await
}
#[derive(Deserialize)]
pub(super) struct TokenName {
    name: String,
}
pub(super) async fn create_token(
    State(app): State<App>,
    h: HeaderMap,
    Json(n): Json<TokenName>,
) -> Result<Json<Value>> {
    work(app, move |app| {
        let a = identity(&app, &h, true)?;
        let (raw, a) = app.create_token(&a, &n.name)?;
        Ok(Json(
            json!({"token":raw,"id":a.credential.id,"expires":a.credential.expires}),
        ))
    })
    .await
}
pub(super) async fn delete_token(
    State(app): State<App>,
    h: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    work(app, move |app| {
        let a = identity(&app, &h, true)?;
        app.revoke_token(&a, &id)?;
        Ok(Json(json!({"ok":true})))
    })
    .await
}
