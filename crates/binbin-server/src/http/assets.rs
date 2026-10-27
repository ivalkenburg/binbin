use axum::{
    extract::Path,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use include_dir::{Dir, include_dir};
static WEB: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../web/dist");

pub(super) async fn index() -> Response {
    serve("index.html")
}
pub(super) async fn asset(Path(path): Path<String>) -> Response {
    serve(&format!("assets/{path}"))
}
fn serve(path: &str) -> Response {
    let Some(file) = WEB.get_file(path) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let content_type = match std::path::Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
    {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("svg") => "image/svg+xml",
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    ([(header::CONTENT_TYPE, content_type)], file.contents()).into_response()
}
