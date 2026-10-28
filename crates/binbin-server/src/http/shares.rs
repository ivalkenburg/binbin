use super::{
    Result,
    auth::identity,
    extractors::{BodyBytes, Json, Query},
    work,
};
use axum::{
    extract::{Path, State},
    http::HeaderMap,
};
use binbin_core::{
    App,
    models::{EditFiles, EditShare, Identity, NewShare, ShareView, StoredFile},
};
use serde::Deserialize;
use serde_json::{Value, json};
pub(super) async fn edit_files(
    State(app): State<App>,
    h: HeaderMap,
    Path(id): Path<String>,
    Json(edit): Json<EditFiles>,
) -> Result<Json<ShareView>> {
    work(app, move |app| {
        let actor = identity(&app, &h, true)?;
        let share = app.edit_files(&actor, &id, edit)?;
        tracing::info!(event = "share_files_updated", user_id = %actor.user.id, share_id = %id, "Draft files updated");
        Ok(Json(share))
    })
    .await
}
pub(super) async fn image_preview(
    State(app): State<App>,
    h: HeaderMap,
    Path(id): Path<String>,
) -> Result<axum::response::Response> {
    let (file, mime, size) = work(app, move |app| {
        let actor = identity(&app, &h, false)?;
        app.image_preview(&actor, &id)
    })
    .await?;
    use tokio::io::AsyncReadExt;
    Ok(axum::response::Response::builder()
        .header(axum::http::header::CONTENT_TYPE, mime)
        .header(axum::http::header::CONTENT_LENGTH, size)
        .header(axum::http::header::CONTENT_DISPOSITION, "inline")
        .body(axum::body::Body::from_stream(
            tokio_util::io::ReaderStream::new(tokio::fs::File::from_std(file).take(size)),
        ))
        .unwrap())
}
pub(super) async fn shares(State(app): State<App>, h: HeaderMap) -> Result<Json<Vec<ShareView>>> {
    work(app, move |app| {
        let a = identity(&app, &h, false)?;
        Ok(Json(app.list_shares(&a)?))
    })
    .await
}
pub(super) async fn create_share(
    State(app): State<App>,
    h: HeaderMap,
    Json(n): Json<NewShare>,
) -> Result<Json<ShareView>> {
    work(app, move |app| {
        let a = identity(&app, &h, true)?;
        let share = app.create_share(&a, n)?;
        tracing::info!(event = "share_created", user_id = %a.user.id, share_id = %share.share["id"], files = share.files.len(), "Share created");
        Ok(Json(share))
    })
    .await
}
pub(super) async fn inspect(
    State(app): State<App>,
    h: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ShareView>> {
    work(app, move |app| {
        let a = identity(&app, &h, false)?;
        Ok(Json(app.inspect(&a, &id)?))
    })
    .await
}
#[derive(Deserialize)]
pub(super) struct Offset {
    offset: u64,
}
pub(super) async fn upload(
    State(app): State<App>,
    h: HeaderMap,
    Path(id): Path<String>,
    Query(q): Query<Offset>,
    BodyBytes(body): BodyBytes,
) -> Result<Json<StoredFile>> {
    work(app, move |app| {
        let a = identity(&app, &h, true)?;
        if let Some(prefix) = h
            .get("x-binbin-prefix-sha256")
            .and_then(|v| v.to_str().ok())
        {
            app.verify_upload_prefix(&a, &id, prefix)?;
        }
        let file = app.upload(&a, &id, q.offset, &body)?;
        if file.offset == file.size && q.offset < file.size {
            tracing::info!(event = "upload_completed", user_id = %a.user.id, share_id = %file.share, file_id = %id, size = file.size, "File uploaded");
        }
        Ok(Json(file))
    })
    .await
}
pub(super) async fn upload_proofs(
    State(app): State<App>,
    h: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Vec<String>>> {
    work(app, move |app| {
        let a = identity(&app, &h, false)?;
        Ok(Json(app.upload_proofs(&a, &id)?))
    })
    .await
}
pub(super) async fn publish(
    State(app): State<App>,
    h: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ShareView>> {
    work(app, move |app| {
        let a = identity(&app, &h, true)?;
        let share = app.publish(&a, &id)?;
        tracing::info!(event = "share_published", user_id = %a.user.id, share_id = %id, "Share published");
        Ok(Json(share))
    })
    .await
}
pub(super) async fn edit(
    State(app): State<App>,
    h: HeaderMap,
    Path(id): Path<String>,
    Json(n): Json<EditShare>,
) -> Result<Json<ShareView>> {
    work(app, move |app| {
        let a = identity(&app, &h, true)?;
        let share = app.edit(&a, &id, n)?;
        tracing::info!(event = "share_updated", user_id = %a.user.id, share_id = %id, "Share settings updated");
        Ok(Json(share))
    })
    .await
}
pub(super) async fn revoke(
    State(app): State<App>,
    h: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    work(app, move |app| {
        let a = identity(&app, &h, true)?;
        app.revoke(&a, &id)?;
        tracing::info!(event = "share_revoked", user_id = %a.user.id, share_id = %id, "Share revoked and deleted");
        Ok(Json(json!({"ok":true})))
    })
    .await
}
#[derive(Deserialize)]
pub(super) struct Password {
    password: Option<String>,
}
pub(super) async fn public_share(
    State(app): State<App>,
    Path(id): Path<String>,
    Json(n): Json<Password>,
) -> Result<Json<Value>> {
    work(app, move |app| {
        Ok(Json(app.public_share(&id, n.password.as_deref())?))
    })
    .await
}
#[derive(Deserialize)]
pub(super) struct NewTransfer {
    resource: String,
    password: Option<String>,
    key: String,
}
pub(super) async fn public_transfer(
    State(app): State<App>,
    Path(id): Path<String>,
    Json(n): Json<NewTransfer>,
) -> Result<Json<Value>> {
    transfer_response(app, None, id, n).await
}
pub(super) async fn management_transfer(
    State(app): State<App>,
    h: HeaderMap,
    Path(id): Path<String>,
    Json(n): Json<NewTransfer>,
) -> Result<Json<Value>> {
    let a = work(app.clone(), move |app| identity(&app, &h, true)).await?;
    transfer_response(app, Some(a), id, n).await
}
pub(super) async fn transfer_response(
    app: App,
    a: Option<Identity>,
    id: String,
    n: NewTransfer,
) -> Result<Json<Value>> {
    work(app, move |app| {
        let (raw, t) =
            app.new_transfer(&id, &n.resource, n.password.as_deref(), &n.key, a.as_ref())?;
        tracing::info!(event = "transfer_ready", share_id = %id, resource = %n.resource, management = t.management, "Download transfer ready");
        Ok(Json(
            json!({"token":raw,"url":format!("{}/d/{raw}",app.public_url),"expires":t.expires}),
        ))
    })
    .await
}
