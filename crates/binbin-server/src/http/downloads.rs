use super::{Result, filename::safe_download_name, work};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, Method, header},
    response::Response,
};
use binbin_core::App;
use std::io::SeekFrom;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::io::ReaderStream;
pub(super) fn byte_range(value: &str, size: u64) -> Option<(u64, u64)> {
    let value = value.strip_prefix("bytes=")?;
    if value.contains(',') || size == 0 {
        return None;
    }
    let (a, b) = value.split_once('-')?;
    if !a.bytes().all(|byte| byte.is_ascii_digit()) || !b.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    if a.is_empty() {
        let suffix = b.parse::<u64>().ok()?;
        if suffix == 0 {
            return None;
        }
        Some((size.saturating_sub(suffix), size - 1))
    } else {
        let start = a.parse::<u64>().ok()?;
        let end = if b.is_empty() {
            size - 1
        } else {
            b.parse::<u64>().ok()?.min(size - 1)
        };
        if start >= size || end < start {
            None
        } else {
            Some((start, end))
        }
    }
}
fn none_match(headers: &HeaderMap, etag: &str) -> bool {
    headers
        .get_all(header::IF_NONE_MATCH)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .any(|value| {
            value.split(',').map(str::trim).any(|candidate| {
                candidate == "*" || candidate.strip_prefix("W/").unwrap_or(candidate) == etag
            })
        })
}
pub(super) async fn download(
    State(app): State<App>,
    Path(token): Path<String>,
    method: Method,
    h: HeaderMap,
) -> Result<Response> {
    let check = token.clone();
    let (path, size, name, hash) =
        work(app.clone(), move |app| app.download(&check, false)).await?;
    let etag = format!("\"{hash}\"");
    let mut builder = Response::builder()
        .header(header::ETAG, &etag)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(
            header::CONTENT_DISPOSITION,
            format!(
                "attachment; filename=\"{}\"; filename*=UTF-8''{}",
                safe_download_name(&name).replace('"', "_"),
                url::form_urlencoded::byte_serialize(name.as_bytes())
                    .collect::<String>()
                    .replace('+', "%20")
            ),
        );
    if none_match(&h, &etag) {
        return Ok(builder.status(304).body(Body::empty()).unwrap());
    }
    let mut range = None;
    if method != Method::HEAD
        && h.get(header::IF_RANGE)
            .is_none_or(|v| v.to_str().ok() == Some(&etag))
        && let Some(v) = h.get(header::RANGE)
    {
        range = byte_range(v.to_str().unwrap_or(""), size);
        if range.is_none() {
            return Ok(builder
                .status(416)
                .header(header::CONTENT_RANGE, format!("bytes */{size}"))
                .body(Body::empty())
                .unwrap());
        }
    }
    let (start, length) = if let Some((start, end)) = range {
        builder = builder
            .status(206)
            .header(header::CONTENT_RANGE, format!("bytes {start}-{end}/{size}"));
        (start, end - start + 1)
    } else {
        (0, size)
    };
    builder = builder.header(header::CONTENT_LENGTH, length);
    if method == Method::HEAD {
        return Ok(builder.body(Body::empty()).unwrap());
    }
    let mut file = tokio::fs::File::open(path).await?;
    file.seek(SeekFrom::Start(start)).await?;
    work(app, move |app| app.download(&token, true)).await?;
    Ok(builder
        .body(Body::from_stream(ReaderStream::new(file.take(length))))
        .unwrap())
}
