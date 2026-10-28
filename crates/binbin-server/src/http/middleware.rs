use super::Error;
use axum::{
    extract::State,
    http::{HeaderValue, Method, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use binbin_core::App;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
#[derive(Clone)]
pub(super) struct Guard {
    pub(super) app: App,
    pub(super) limiter: Arc<Mutex<HashMap<String, (i64, u32)>>>,
}
pub(super) async fn guard(
    State(state): State<Guard>,
    req: axum::extract::Request,
    next: Next,
) -> Response {
    if let Some(origin) = req
        .headers()
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        && origin != state.app.public_url
    {
        return Error(403, "Request origin is not allowed".into()).into_response();
    }
    let path = req.uri().path();
    if req.method() == Method::POST && (path == "/api/login" || path.starts_with("/api/public/")) {
        let mut limits = state.limiter.lock().unwrap();
        let bucket = if path == "/api/login" {
            "login"
        } else {
            "public"
        };
        let entry = limits.entry(bucket.into()).or_insert((state.app.now(), 0));
        if state.app.now() - entry.0 >= 60 {
            *entry = (state.app.now(), 0);
        }
        entry.1 += 1;
        if entry.1 > 120 {
            return Error(429, "Too many attempts; wait one minute and retry".into())
                .into_response();
        }
    }
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.entry(header::CONTENT_SECURITY_POLICY).or_insert(HeaderValue::from_static("default-src 'self'; script-src 'self'; style-src 'self'; style-src-attr 'unsafe-inline'; img-src 'self' blob:; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'"));
    response
}
