use axum::http::{HeaderMap, header};
use binbin_core::{App, models::Identity};
pub(super) fn identity(app: &App, h: &HeaderMap, mutation: bool) -> binbin_core::Result<Identity> {
    let bearer = h
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "));
    let cookie = h
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| {
            s.split(';')
                .find_map(|c| c.trim().strip_prefix("binbin_session="))
        });
    let raw = bearer
        .or(cookie)
        .ok_or_else(|| binbin_core::Error(401, "Sign in required".into()))?;
    app.authenticate(
        raw,
        h.get("x-binbin-csrf").and_then(|v| v.to_str().ok()),
        mutation,
        bearer.is_none(),
    )
}
