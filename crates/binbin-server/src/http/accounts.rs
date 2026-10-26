use super::{Result, auth::identity, extractors::Json, user_view, work};
use axum::{
    extract::State,
    http::{HeaderMap, HeaderValue, header},
    response::{IntoResponse, Response},
};
use binbin_core::{App, DAY};
use serde::Deserialize;
use serde_json::{Value, json};
#[derive(Deserialize)]
pub(super) struct Login {
    username: String,
    password: String,
}
pub(super) async fn login(State(app): State<App>, Json(n): Json<Login>) -> Result<Response> {
    let secure = app.secure_cookie;
    let (raw, a) = work(app, move |app| app.login(&n.username, &n.password)).await?;
    let cookie = format!(
        "binbin_session={raw}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}{}",
        7 * DAY,
        if secure { "; Secure" } else { "" }
    );
    let mut r = Json(json!({"user":user_view(&a.user),"csrf":a.credential.csrf})).into_response();
    r.headers_mut()
        .insert(header::SET_COOKIE, HeaderValue::from_str(&cookie).unwrap());
    Ok(r)
}
pub(super) async fn logout(State(app): State<App>, h: HeaderMap) -> Result<Response> {
    work(app, move |app| {
        let a = identity(&app, &h, true)?;
        app.logout(&a)
    })
    .await?;
    let mut r = Json(json!({"ok":true})).into_response();
    r.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_static("binbin_session=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0"),
    );
    Ok(r)
}
pub(super) async fn me(State(app): State<App>, h: HeaderMap) -> Result<Json<Value>> {
    work(app, move |app| {
        let a = identity(&app, &h, false)?;
        Ok(Json(
            json!({"user":user_view(&a.user),"csrf":a.credential.csrf}),
        ))
    })
    .await
}
