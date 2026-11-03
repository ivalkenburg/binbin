use super::{Result, extractors::Query, filename::safe_download_name, work};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, Method, header},
    response::Response,
};
use binbin_core::{App, DownloadLease};
use futures_util::stream;
use serde::Deserialize;
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
#[derive(Deserialize)]
pub(super) struct DownloadOptions {
    #[serde(default)]
    inline: bool,
}

// Serve only inert formats with a browser MIME type. Active documents such as
// HTML and SVG are shown as source; unknown formats fall back to downloading.
fn inline_content_type(name: &str) -> &'static str {
    let extension = name
        .rsplit_once('.')
        .map_or("", |(_, extension)| extension)
        .to_ascii_lowercase();
    match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "bmp" => "image/bmp",
        "ico" => "image/vnd.microsoft.icon",
        "pdf" => "application/pdf",
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        "m4a" => "audio/mp4",
        "ogg" => "audio/ogg",
        "wav" => "audio/wav",
        "txt" | "md" | "csv" | "json" | "xml" | "html" | "htm" | "svg" | "js" | "css" | "rs"
        | "py" | "log" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

pub(super) async fn download(
    State(app): State<App>,
    Path(token): Path<String>,
    Query(options): Query<DownloadOptions>,
    method: Method,
    h: HeaderMap,
) -> Result<Response> {
    let check = token.clone();
    let (path, size, name, hash) =
        work(app.clone(), move |app| app.download(&check, false)).await?;
    let check = token.clone();
    let deleting = work(app.clone(), move |app| app.deletion_download(&check)).await?;
    if deleting && options.inline {
        return Err(binbin_core::error::bad("File previews are unavailable for this share").into());
    }
    let etag = format!("\"{hash}\"");
    let disposition = if options.inline {
        "inline"
    } else {
        "attachment"
    };
    let content_type = if options.inline {
        inline_content_type(&name)
    } else {
        "application/octet-stream"
    };
    let mut builder = Response::builder()
        .header(header::ETAG, &etag)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_TYPE, content_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!(
                "{disposition}; filename=\"{}\"; filename*=UTF-8''{}",
                safe_download_name(&name).replace('"', "_"),
                url::form_urlencoded::byte_serialize(name.as_bytes())
                    .collect::<String>()
                    .replace('+', "%20")
            ),
        );
    if options.inline && content_type.starts_with("text/plain") {
        // Active document formats are served as text with an opaque origin.
        builder = builder.header(
            header::CONTENT_SECURITY_POLICY,
            "sandbox; default-src 'none'; style-src 'unsafe-inline'; frame-ancestors 'none'",
        );
    }
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
    let (mut start, mut length) = if let Some((start, end)) = range {
        builder = builder
            .status(206)
            .header(header::CONTENT_RANGE, format!("bytes {start}-{end}/{size}"));
        (start, end - start + 1)
    } else {
        (0, size)
    };
    // A one-use response must be polled through EOF before it is considered
    // delivered. Chunked framing also covers zero-byte files; a known length
    // can otherwise let the HTTP stack drop the body without polling EOF.
    if !deleting || method == Method::HEAD {
        builder = builder.header(header::CONTENT_LENGTH, length);
    }
    if method == Method::HEAD {
        return Ok(builder.body(Body::empty()).unwrap());
    }
    let mut file = tokio::fs::File::open(path).await?;
    file.seek(SeekFrom::Start(start)).await?;
    let lease = work(app, move |app| app.begin_download(&token)).await?;
    if lease
        .as_ref()
        .is_some_and(|lease| lease.restart_required(start))
    {
        // The previous process may have emitted an uncheckpointed prefix.
        // A full response makes native download clients restart safely.
        start = 0;
        length = size;
        builder = builder.status(200);
        builder.headers_mut().unwrap().remove(header::CONTENT_RANGE);
        file.seek(SeekFrom::Start(0)).await?;
    }
    tracing::info!(event = "download_started", filename = ?name, size, offset = start, bytes = length, inline = options.inline, "File transfer started");
    let body = if let Some(lease) = lease {
        let state = TrackedDownload {
            lease: Some(lease),
            start,
            sent: 0,
            checkpoint: 0,
            checkpoint_time: std::time::Instant::now(),
        };
        let stream = stream::try_unfold(
            (file.take(length), state),
            move |(mut file, mut state)| async move {
                // Reaching this poll means the previous frame was consumed by the
                // HTTP stack. Persist progress off the async runtime's worker.
                if state.sent - state.checkpoint >= 1024 * 1024
                    || state.checkpoint_time.elapsed().as_secs() >= 1
                {
                    state = tokio::task::spawn_blocking(move || {
                        state.save()?;
                        Ok::<_, binbin_core::Error>(state)
                    })
                    .await
                    .map_err(std::io::Error::other)?
                    .map_err(std::io::Error::other)?;
                }
                let mut bytes = vec![0; 64 * 1024];
                let count = file.read(&mut bytes).await?;
                if count == 0 {
                    if state.sent != length {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "Download ended before the complete range",
                        ));
                    }
                    drop(file);
                    tokio::task::spawn_blocking(move || {
                        if let Some(lease) = state.lease.take() {
                            lease.complete(state.start, state.start + state.sent, size)?;
                        }
                        Ok::<_, binbin_core::Error>(())
                    })
                    .await
                    .map_err(std::io::Error::other)?
                    .map_err(std::io::Error::other)?;
                    return Ok::<_, std::io::Error>(None);
                }
                bytes.truncate(count);
                state.sent += count as u64;
                Ok(Some((axum::body::Bytes::from(bytes), (file, state))))
            },
        );
        Body::from_stream(stream)
    } else {
        Body::from_stream(ReaderStream::new(file.take(length)))
    };
    Ok(builder.body(body).unwrap())
}

struct TrackedDownload {
    lease: Option<DownloadLease>,
    start: u64,
    sent: u64,
    checkpoint: u64,
    checkpoint_time: std::time::Instant,
}
impl TrackedDownload {
    fn save(&mut self) -> binbin_core::Result<()> {
        if let Some(lease) = &self.lease {
            lease.checkpoint(self.start, self.start + self.sent)?;
        }
        self.checkpoint = self.sent;
        self.checkpoint_time = std::time::Instant::now();
        Ok(())
    }
}
impl Drop for TrackedDownload {
    fn drop(&mut self) {
        if let Some(lease) = self.lease.take() {
            let (start, end) = (self.start, self.start + self.sent);
            // A disconnected response may have delivered a prefix. Save it and
            // start the retry window without blocking an async runtime worker.
            tokio::task::spawn_blocking(move || {
                let _ = lease.interrupted(start, end);
            });
        }
    }
}

pub(super) async fn download_status(
    State(app): State<App>,
    Path(token): Path<String>,
) -> Result<Response> {
    let observer = work(app, move |app| app.observe_download(&token)).await?;
    Ok(status_response(observer))
}

pub(super) fn status_response(observer: binbin_core::DownloadObservation) -> Response {
    let events = stream::unfold((Some(observer), true), |(observer, first)| async move {
        let observer = observer?;
        if !first {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
        let (observer, status) = tokio::task::spawn_blocking(move || {
            let status = if first {
                "ready"
            } else {
                observer.status().unwrap_or("unavailable")
            };
            (observer, status)
        })
        .await
        .ok()?;
        let finished = matches!(status, "deleted" | "unavailable" | "blocked");
        Some((
            Ok::<_, std::io::Error>(format!("{{\"state\":\"{status}\"}}\n")),
            (if finished { None } else { Some(observer) }, false),
        ))
    });
    Response::builder()
        .header(header::CONTENT_TYPE, "application/x-ndjson")
        .header("x-accel-buffering", "no")
        .body(Body::from_stream(events))
        .unwrap()
}
