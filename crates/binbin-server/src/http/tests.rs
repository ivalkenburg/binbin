use super::*;
use axum::{body::Body, http::Request};
use binbin_core::{crypto::secret, models::*};
use http_body_util::BodyExt;
use tower::ServiceExt;
struct Fixture {
    _dir: tempfile::TempDir,
    app: App,
    actor: Identity,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let app = App::open(dir.path(), "http://localhost:8080".into()).unwrap();
        let u = app.create_admin("admin", "correct-horse-battery").unwrap();
        let (_, actor) = app.issue(&u).unwrap();
        Self {
            _dir: dir,
            app,
            actor,
        }
    }
    fn draft(&self, limit: Option<u64>, multi: bool) -> ShareView {
        self.app
            .create_share(
                &self.actor,
                NewShare {
                    delete_after_download: false,
                    title: "Project files".into(),
                    files: if multi {
                        vec![
                            NewFile {
                                name: "hello.txt".into(),
                                size: 5,
                            },
                            NewFile {
                                name: "empty.txt".into(),
                                size: 0,
                            },
                        ]
                    } else {
                        vec![NewFile {
                            name: "hello.txt".into(),
                            size: 5,
                        }]
                    },
                    days: Some(7),
                    limit,
                    password: None,
                },
            )
            .unwrap()
    }
    fn published(&self, limit: Option<u64>, multi: bool) -> ShareView {
        let s = self.draft(limit, multi);
        self.app
            .upload(&self.actor, &s.files[0].id, 0, b"hello")
            .unwrap();
        self.app
            .publish(&self.actor, s.share["id"].as_str().unwrap())
            .unwrap()
    }
    fn transfer(&self, s: &ShareView, resource: &str) -> String {
        self.app
            .new_transfer(
                s.share["id"].as_str().unwrap(),
                resource,
                None,
                &secret(),
                None,
            )
            .unwrap()
            .0
    }
}
#[test]
fn ranges_cover_closed_open_suffix_and_invalid() {
    use crate::http::downloads::byte_range;
    assert_eq!(byte_range("bytes=1-3", 5), Some((1, 3)));
    assert_eq!(byte_range("bytes=2-", 5), Some((2, 4)));
    assert_eq!(byte_range("bytes=-2", 5), Some((3, 4)));
    for input in [
        "bytes=5-",
        "bytes=3-1",
        "bytes=0-1,3-4",
        "bytes=-0",
        "bytes=+1-3",
        "bytes=1-+3",
        "bytes=+1-",
        "bytes=-+2",
        "bytes=-1-3",
        "bytes=1--3",
        "bytes=--2",
        "bytes=١-3",
        "bytes=1- 3",
        "garbage",
    ] {
        assert_eq!(byte_range(input, 5), None);
    }
    assert_eq!(byte_range("bytes=0-", 0), None);
}

fn deletion_share(f: &Fixture, multi: bool) -> ShareView {
    let draft = f.draft(None, multi);
    let id = draft.share["id"].as_str().unwrap();
    f.app
        .edit(
            &f.actor,
            id,
            EditShare {
                title: "Delete on delivery".into(),
                expires: None,
                days: Some(7),
                limit: None,
                password: None,
                change_password: false,
                delete_after_download: Some(true),
            },
        )
        .unwrap();
    f.app
        .upload(&f.actor, &draft.files[0].id, 0, b"hello")
        .unwrap();
    f.app.publish(&f.actor, id).unwrap()
}

#[tokio::test]
async fn deletion_waits_for_body_eof_and_respects_range_coverage() {
    let f = Fixture::new();
    let s = deletion_share(&f, false);
    let id = s.share["id"].as_str().unwrap();
    let token = f.transfer(&s, &s.files[0].id);
    let path = format!("/d/{token}");
    let observer = f.app.observe_download(&token).unwrap();
    for (method, suffix, headers, expected) in [
        ("HEAD", "", vec![], 200),
        ("GET", "", vec![("range", "bytes=99-")], 416),
        ("GET", "?inline=true", vec![], 400),
        ("GET", "", vec![("if-none-match", "*")], 304),
    ] {
        assert_eq!(
            request(
                &f.app,
                method,
                &format!("{path}{suffix}"),
                &headers,
                json!(null)
            )
            .await
            .0,
            expected
        );
        assert_eq!(observer.status().unwrap(), "available");
    }
    assert_eq!(
        request(&f.app, "GET", &path, &[("range", "bytes=0-1")], json!(null))
            .await
            .2,
        b"he"
    );
    assert!(f.app.inspect(&f.actor, id).is_ok());
    let response = router(f.app.clone())
        .oneshot(
            Request::builder()
                .uri(&path)
                .header("range", "bytes=2-")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 206);
    assert!(
        f.app.inspect(&f.actor, id).is_ok(),
        "Headers alone must not delete the share"
    );
    assert_eq!(
        response.into_body().collect().await.unwrap().to_bytes(),
        "llo"
    );
    assert_eq!(observer.outcome(), 1);
    assert_eq!(f.app.inspect(&f.actor, id).unwrap_err().0, 404);
    assert_eq!(request(&f.app, "GET", &path, &[], json!(null)).await.0, 404);
}

#[tokio::test]
async fn dropped_response_preserves_files_and_reserves_for_the_original_token() {
    let f = Fixture::new();
    let s = deletion_share(&f, false);
    let id = s.share["id"].as_str().unwrap();
    let token = f.transfer(&s, &s.files[0].id);
    let other = f.transfer(&s, &s.files[0].id);
    let response = router(f.app.clone())
        .oneshot(
            Request::builder()
                .uri(format!("/d/{token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        request(&f.app, "GET", &format!("/d/{other}"), &[], json!(null))
            .await
            .0,
        409
    );
    drop(response);
    // Drain blocking work queued by body Drop before checking retry behavior.
    for _ in 0..100 {
        if f.app.observe_download(&token).unwrap().status().unwrap() == "interrupted" {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(f.app.inspect(&f.actor, id).is_ok());
    assert_eq!(
        request(&f.app, "GET", &format!("/d/{token}"), &[], json!(null))
            .await
            .2,
        b"hello"
    );
    assert_eq!(f.app.inspect(&f.actor, id).unwrap_err().0, 404);
}

#[tokio::test]
async fn zero_byte_deletion_share_is_deleted_only_by_a_get_response() {
    let f = Fixture::new();
    let s = f
        .app
        .create_share(
            &f.actor,
            NewShare {
                title: "Empty file".into(),
                files: vec![NewFile {
                    name: "empty.txt".into(),
                    size: 0,
                }],
                days: Some(7),
                limit: None,
                password: None,
                delete_after_download: true,
            },
        )
        .unwrap();
    let id = s.share["id"].as_str().unwrap();
    let s = f.app.publish(&f.actor, id).unwrap();
    let token = f.transfer(&s, &s.files[0].id);
    let path = format!("/d/{token}");
    assert_eq!(
        request(&f.app, "HEAD", &path, &[], json!(null)).await.0,
        200
    );
    assert!(f.app.inspect(&f.actor, id).is_ok());
    assert_eq!(request(&f.app, "GET", &path, &[], json!(null)).await.0, 200);
    assert_eq!(f.app.inspect(&f.actor, id).unwrap_err().0, 404);
}

#[tokio::test]
async fn interrupted_large_response_resumes_its_streamed_prefix_before_deletion() {
    let f = Fixture::new();
    let bytes = vec![7; 2 * 1024 * 1024];
    let s = f
        .app
        .create_share(
            &f.actor,
            NewShare {
                title: "Large one-time file".into(),
                files: vec![NewFile {
                    name: "large.bin".into(),
                    size: bytes.len() as u64,
                }],
                days: Some(7),
                limit: None,
                password: None,
                delete_after_download: true,
            },
        )
        .unwrap();
    let id = s.share["id"].as_str().unwrap();
    f.app.upload(&f.actor, &s.files[0].id, 0, &bytes).unwrap();
    let s = f.app.publish(&f.actor, id).unwrap();
    let token = f.transfer(&s, &s.files[0].id);
    let path = format!("/d/{token}");
    let observer = f.app.observe_download(&token).unwrap();
    let mut body = router(f.app.clone())
        .oneshot(Request::builder().uri(&path).body(Body::empty()).unwrap())
        .await
        .unwrap()
        .into_body();
    let prefix = body.frame().await.unwrap().unwrap().into_data().unwrap();
    assert!(!prefix.is_empty() && prefix.len() < bytes.len());
    drop(body);
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while observer.status().unwrap() != "interrupted" {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    assert!(f.app.inspect(&f.actor, id).is_ok());
    let range = format!("bytes={}-", prefix.len());
    let (status, _, rest) = request(&f.app, "GET", &path, &[("range", &range)], json!(null)).await;
    assert_eq!(status, 206);
    assert_eq!(rest, bytes[prefix.len()..]);
    assert_eq!(observer.outcome(), 1);
    assert_eq!(f.app.inspect(&f.actor, id).unwrap_err().0, 404);
}

#[tokio::test]
async fn restart_retransmits_an_uncheckpointed_prefix_and_deletes_after_delivery() {
    let f = Fixture::new();
    let share = deletion_share(&f, false);
    let id = share.share["id"].as_str().unwrap();
    let token = f.transfer(&share, &share.files[0].id);
    let lease = f.app.begin_download(&token).unwrap().unwrap();
    lease.checkpoint(0, 1).unwrap();
    // The client received bytes 0..3, but the process died before recording
    // bytes 1..3. Dropping the core lease simulates loss without a body callback.
    drop(lease);
    let restarted = App::open(&f.app.root, f.app.public_url.clone()).unwrap();
    let (status, headers, body) = request(
        &restarted,
        "GET",
        &format!("/d/{token}"),
        &[("range", "bytes=3-")],
        json!(null),
    )
    .await;
    assert_eq!(
        status, 200,
        "A resume beyond durable coverage must restart the client"
    );
    assert!(!headers.contains_key("content-range"));
    assert_eq!(body, b"hello");
    assert_eq!(f.app.inspect(&f.actor, id).unwrap_err().0, 404);
    assert!(!f.app.root.join(id).exists());
}

#[tokio::test]
async fn deliberate_disjoint_ranges_keep_their_partial_response_semantics() {
    let f = Fixture::new();
    let share = deletion_share(&f, false);
    let id = share.share["id"].as_str().unwrap();
    let token = f.transfer(&share, &share.files[0].id);
    let path = format!("/d/{token}");
    for (range, expected) in [("bytes=1-1", "e"), ("bytes=3-4", "lo"), ("bytes=0-0", "h")] {
        let (status, _, body) =
            request(&f.app, "GET", &path, &[("range", range)], json!(null)).await;
        assert_eq!(status, 206);
        assert_eq!(body, expected.as_bytes());
        assert!(f.app.inspect(&f.actor, id).is_ok());
    }
    let (status, _, body) =
        request(&f.app, "GET", &path, &[("range", "bytes=2-2")], json!(null)).await;
    assert_eq!(status, 206);
    assert_eq!(body, b"l");
    assert_eq!(f.app.inspect(&f.actor, id).unwrap_err().0, 404);
}

async fn request(
    app: &App,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: Value,
) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let mut req = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    let r = crate::http::router(app.clone())
        .oneshot(
            req.body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = r.status();
    let h = r.headers().clone();
    let bytes = r.into_body().collect().await.unwrap().to_bytes().to_vec();
    (status, h, bytes)
}

#[tokio::test]
async fn inline_files_preserve_download_headers_ranges_and_allowance() {
    let fixture = Fixture::new();
    for (name, content_type) in [
        ("hello.txt", "text/plain; charset=utf-8"),
        ("image.PNG", "image/png"),
        ("document.pdf", "application/pdf"),
        ("untrusted.html", "text/plain; charset=utf-8"),
        ("untrusted.svg", "text/plain; charset=utf-8"),
        ("archive.zip", "application/octet-stream"),
        ("pdf", "application/octet-stream"),
    ] {
        let draft = fixture
            .app
            .create_share(
                &fixture.actor,
                NewShare {
                    delete_after_download: false,
                    title: "Browser file".into(),
                    files: vec![NewFile {
                        name: name.into(),
                        size: 5,
                    }],
                    days: Some(7),
                    limit: Some(1),
                    password: None,
                },
            )
            .unwrap();
        let id = draft.share["id"].as_str().unwrap();
        fixture
            .app
            .upload(&fixture.actor, &draft.files[0].id, 0, b"hello")
            .unwrap();
        let share = fixture.app.publish(&fixture.actor, id).unwrap();
        let token = fixture.transfer(&share, &draft.files[0].id);
        let inline = format!("/d/{token}?inline=true");
        let attachment = format!("/d/{token}");
        let (status, headers, body) =
            request(&fixture.app, "HEAD", &inline, &[], json!(null)).await;
        assert_eq!(status, 200);
        assert!(body.is_empty());
        assert_eq!(headers["content-type"], content_type);
        assert!(
            headers["content-disposition"]
                .to_str()
                .unwrap()
                .starts_with("inline;")
        );
        let policy = headers["content-security-policy"].to_str().unwrap();
        assert_eq!(
            policy.starts_with("sandbox;"),
            content_type.starts_with("text/plain")
        );
        assert_eq!(headers["x-content-type-options"], "nosniff");
        assert_eq!(headers["cache-control"], "no-store");
        assert_eq!(
            fixture.app.inspect(&fixture.actor, id).unwrap().share["used"],
            0
        );
        let (status, _, body) = request(
            &fixture.app,
            "GET",
            &inline,
            &[("range", "bytes=1-3")],
            json!(null),
        )
        .await;
        assert_eq!(status, 206);
        assert_eq!(body, b"ell");
        let (status, headers, body) =
            request(&fixture.app, "GET", &attachment, &[], json!(null)).await;
        assert_eq!(status, 200);
        assert_eq!(headers["content-type"], "application/octet-stream");
        assert!(
            headers["content-disposition"]
                .to_str()
                .unwrap()
                .starts_with("attachment;")
        );
        assert_eq!(body, b"hello");
        assert_eq!(
            fixture.app.inspect(&fixture.actor, id).unwrap().share["used"],
            1
        );
        fixture.app.revoke(&fixture.actor, id).unwrap();
        assert_eq!(
            request(&fixture.app, "GET", &inline, &[], json!(null))
                .await
                .0,
            404
        );
    }
}

#[tokio::test]
async fn editable_draft_and_private_preview_http_contracts() {
    let fixture = Fixture::new();
    let draft = fixture.draft(None, false);
    let id = draft.share["id"].as_str().unwrap();
    let (raw, session) = fixture.app.issue(&fixture.actor.user).unwrap();
    let cookie = format!("binbin_session={raw}");
    let authorization = [
        ("cookie", cookie.as_str()),
        ("x-binbin-csrf", session.credential.csrf.as_str()),
    ];
    let path = format!("/api/shares/{id}/files");
    assert_eq!(
        request(
            &fixture.app,
            "PATCH",
            &path,
            &[],
            json!({"remove":[draft.files[0].id]})
        )
        .await
        .0,
        401
    );
    let (status, _, body) = request(
        &fixture.app,
        "PATCH",
        &path,
        &authorization,
        json!({"add":[{"name":"preview.png","size":8}],"remove":[draft.files[0].id]}),
    )
    .await;
    assert_eq!(status, 200);
    let edited: Value = serde_json::from_slice(&body).unwrap();
    let file = edited["files"][0]["id"].as_str().unwrap();
    let preview = format!("/api/files/{file}/preview");
    assert_eq!(
        request(&fixture.app, "GET", &preview, &authorization, json!(null))
            .await
            .0,
        404
    );
    fixture
        .app
        .upload(&fixture.actor, file, 0, b"\x89PNG\r\n\x1a\n")
        .unwrap();
    assert_eq!(
        request(&fixture.app, "GET", &preview, &[], json!(null))
            .await
            .0,
        401
    );
    let (status, headers, body) =
        request(&fixture.app, "GET", &preview, &authorization, json!(null)).await;
    assert_eq!(status, 200);
    assert_eq!(headers["content-type"], "image/png");
    assert_eq!(headers["cache-control"], "no-store");
    assert_eq!(headers["x-content-type-options"], "nosniff");
    assert_eq!(body, b"\x89PNG\r\n\x1a\n");
    assert_eq!(
        request(
            &fixture.app,
            "PATCH",
            &path,
            &authorization,
            json!({"restore":[draft.files[0].id]})
        )
        .await
        .0,
        200
    );
}
#[tokio::test]
async fn http_ranges_head_invalid_ranges_and_retries() {
    let f = Fixture::new();
    let s = f.published(Some(1), false);
    let token = f.transfer(&s, &s.files[0].id);
    let path = format!("/d/{token}");
    let (status, _, body) = request(&f.app, "HEAD", &path, &[], json!(null)).await;
    assert_eq!(status, 200);
    assert!(body.is_empty());
    let (status, h, _) =
        request(&f.app, "GET", &path, &[("range", "bytes=99-")], json!(null)).await;
    assert_eq!(status, 416);
    assert_eq!(h["content-range"], "bytes */5");
    for range in [
        "bytes=+1-3",
        "bytes=1-+3",
        "bytes=+1-",
        "bytes=-+2",
        "bytes=-1-3",
        "bytes=1--3",
        "bytes=--2",
    ] {
        assert_eq!(
            request(&f.app, "GET", &path, &[("range", range)], json!(null))
                .await
                .0,
            416,
            "{range}"
        );
    }
    assert_eq!(
        f.app
            .inspect(&f.actor, s.share["id"].as_str().unwrap())
            .unwrap()
            .share["used"],
        0
    );
    let (status, h, body) =
        request(&f.app, "GET", &path, &[("range", "bytes=1-3")], json!(null)).await;
    assert_eq!(status, 206);
    assert_eq!(body, b"ell");
    assert_eq!(h["content-range"], "bytes 1-3/5");
    let (status, _, body) =
        request(&f.app, "GET", &path, &[("range", "bytes=4-")], json!(null)).await;
    assert_eq!(status, 206);
    assert_eq!(body, b"o");
    assert_eq!(
        f.app
            .inspect(&f.actor, s.share["id"].as_str().unwrap())
            .unwrap()
            .share["used"],
        1
    );
}

#[tokio::test]
async fn request_concurrency_is_shared_across_routes_and_router_clones() {
    use std::{
        future::Future,
        sync::atomic::{AtomicUsize, Ordering},
        task::Poll,
    };
    let entered = Arc::new(AtomicUsize::new(0));
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let handler = {
        let entered = entered.clone();
        let gate = gate.clone();
        move || {
            let entered = entered.clone();
            let gate = gate.clone();
            async move {
                entered.fetch_add(1, Ordering::SeqCst);
                gate.acquire().await.unwrap().forget();
                "ok"
            }
        }
    };
    let router = Router::new()
        .route("/first", get(handler.clone()))
        .route("/second", get(handler))
        .layer(request_limit());
    let mut requests = Vec::new();
    for index in 0..32 {
        let path = if index < 16 { "/first" } else { "/second" };
        requests.push(Box::pin(
            router
                .clone()
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap()),
        ));
    }
    // Poll every request once while the handlers are held. A per-route limiter
    // incorrectly admits all 32; the shared limiter admits exactly 16.
    for request in &mut requests {
        assert!(
            std::future::poll_fn(|cx| Poll::Ready(request.as_mut().poll(cx)))
                .await
                .is_pending()
        );
    }
    assert_eq!(entered.load(Ordering::SeqCst), 16);
    gate.add_permits(32);
    for request in requests {
        assert_eq!(request.await.unwrap().status(), 200);
    }
    assert_eq!(entered.load(Ordering::SeqCst), 32);
}

#[tokio::test]
async fn extractor_rejections_preserve_status_and_use_json_errors() {
    let fixture = Fixture::new();
    let draft = fixture.draft(None, false);
    let upload = format!("/api/files/{}?offset=0", draft.files[0].id);
    let invalid_offset = format!("/api/files/{}?offset=invalid", draft.files[0].id);
    for (method, path, body, content_type, expected) in [
        ("POST", "/api/login", b"{".to_vec(), "application/json", 400),
        (
            "POST",
            "/api/login",
            b"{}".to_vec(),
            "application/json",
            422,
        ),
        ("POST", "/api/login", b"{}".to_vec(), "text/plain", 415),
        (
            "PATCH",
            invalid_offset.as_str(),
            Vec::new(),
            "application/octet-stream",
            400,
        ),
        (
            "PATCH",
            upload.as_str(),
            vec![0; CHUNK + 1],
            "application/octet-stream",
            413,
        ),
    ] {
        let response = router(fixture.app.clone())
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .header("content-type", content_type)
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), expected, "{method} {path}");
        assert_eq!(response.headers()["content-type"], "application/json");
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let value: Value = serde_json::from_slice(&body).unwrap();
        assert!(!value["error"].as_str().unwrap().is_empty());
        assert_eq!(value.as_object().unwrap().len(), 1);
    }
    assert_eq!(
        fixture
            .app
            .inspect(&fixture.actor, draft.share["id"].as_str().unwrap())
            .unwrap()
            .files[0]
            .offset,
        0
    );
}
#[tokio::test]
async fn http_origin_cookie_csrf_and_session_lifecycle() {
    let f = Fixture::new();
    let (status, _, _) = request(
        &f.app,
        "POST",
        "/api/login",
        &[("origin", "https://evil.example")],
        json!({"username":"admin","password":"correct-horse-battery"}),
    )
    .await;
    assert_eq!(status, 403);
    let (status, h, b) = request(
        &f.app,
        "POST",
        "/api/login",
        &[],
        json!({"username":"admin","password":"correct-horse-battery"}),
    )
    .await;
    assert_eq!(status, 200);
    let cookie = h["set-cookie"].to_str().unwrap().split(';').next().unwrap();
    let v: Value = serde_json::from_slice(&b).unwrap();
    let csrf = v["csrf"].as_str().unwrap();
    let headers = [("cookie", cookie), ("x-binbin-csrf", csrf)];
    assert_eq!(
        request(&f.app, "GET", "/api/me", &headers[..1], json!(null))
            .await
            .0,
        200
    );
    assert_eq!(
        request(&f.app, "POST", "/api/logout", &headers[..1], json!(null))
            .await
            .0,
        403
    );
    assert_eq!(
        request(&f.app, "POST", "/api/logout", &headers, json!(null))
            .await
            .0,
        200
    );
    assert_eq!(
        request(&f.app, "GET", "/api/me", &headers[..1], json!(null))
            .await
            .0,
        401
    );
}

#[tokio::test]
async fn conditional_downloads_do_not_charge_and_if_range_mismatches_restart() {
    let fixture = Fixture::new();
    let share = fixture.published(Some(2), false);
    let token = fixture.transfer(&share, &share.files[0].id);
    let path = format!("/d/{token}");
    let (_, headers, _) = request(&fixture.app, "HEAD", &path, &[], json!(null)).await;
    let etag = headers["etag"].to_str().unwrap();
    for condition in [
        "*".to_owned(),
        format!("W/{etag}"),
        format!("\"other\", W/{etag}"),
    ] {
        for method in ["GET", "HEAD"] {
            let (status, _, body) = request(
                &fixture.app,
                method,
                &path,
                &[("if-none-match", &condition)],
                json!(null),
            )
            .await;
            assert_eq!(status, 304);
            assert!(body.is_empty());
        }
    }
    assert_eq!(
        fixture
            .app
            .inspect(&fixture.actor, share.share["id"].as_str().unwrap())
            .unwrap()
            .share["used"],
        0
    );
    let (status, _, body) = request(
        &fixture.app,
        "GET",
        &path,
        &[("range", "bytes=2-"), ("if-range", "\"different\"")],
        json!(null),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(body, b"hello");
    let (status, _, body) = request(
        &fixture.app,
        "GET",
        &path,
        &[("if-none-match", "\"not-current\"")],
        json!(null),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(body, b"hello");
    assert_eq!(
        fixture
            .app
            .inspect(&fixture.actor, share.share["id"].as_str().unwrap())
            .unwrap()
            .share["used"],
        1
    );
}

#[tokio::test]
async fn embedded_entry_point_modules_and_styles_have_correct_content_types() {
    let fixture = Fixture::new();
    let manifest: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../web/dist/.vite/manifest.json"
    )))
    .unwrap();
    let entry = &manifest["index.html"];
    let javascript = format!("/{}", entry["file"].as_str().unwrap());
    let css = format!("/{}", entry["css"][0].as_str().unwrap());
    for (path, content_type, marker) in [
        ("/", "text/html", "type=\"module\""),
        ("/s/example", "text/html", "id=\"root\""),
        (javascript.as_str(), "text/javascript", "Binbin"),
        (css.as_str(), "text/css", "--background"),
        ("/assets/favicon.svg", "image/svg+xml", "viewBox"),
    ] {
        let (status, headers, body) = request(&fixture.app, "GET", path, &[], json!(null)).await;
        assert_eq!(status, 200);
        assert!(
            headers["content-type"]
                .to_str()
                .unwrap()
                .starts_with(content_type)
        );
        assert!(String::from_utf8(body).unwrap().contains(marker));
    }
    for path in [
        "/assets/missing.js",
        "/assets/../Cargo.toml",
        "/assets/README.md",
        "/assets/src/main.tsx",
        "/assets/.vite/manifest.json",
    ] {
        assert_eq!(
            request(&fixture.app, "GET", path, &[], json!(null)).await.0,
            404
        );
    }
}
