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
        let (_, actor) = app.issue(&u, "test", false).unwrap();
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
async fn editable_draft_and_private_preview_http_contracts() {
    let fixture = Fixture::new();
    let draft = fixture.draft(None, false);
    let id = draft.share["id"].as_str().unwrap();
    let (token, _) = fixture
        .app
        .issue(&fixture.actor.user, "http", false)
        .unwrap();
    let bearer = format!("Bearer {token}");
    let authorization = [("authorization", bearer.as_str())];
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
async fn http_origin_cookie_csrf_and_token_lifecycle() {
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
    let (status, _, _) = request(
        &f.app,
        "POST",
        "/api/tokens",
        &[("cookie", cookie)],
        json!({"name":"cli"}),
    )
    .await;
    assert_eq!(status, 403);
    let (status, _, b) = request(
        &f.app,
        "POST",
        "/api/tokens",
        &[("cookie", cookie), ("x-binbin-csrf", csrf)],
        json!({"name":"cli"}),
    )
    .await;
    assert_eq!(status, 200);
    let t: Value = serde_json::from_slice(&b).unwrap();
    let auth = format!("Bearer {}", t["token"].as_str().unwrap());
    assert_eq!(
        request(
            &f.app,
            "GET",
            "/api/me",
            &[("authorization", &auth)],
            json!(null)
        )
        .await
        .0,
        200
    );
    assert_eq!(
        request(
            &f.app,
            "DELETE",
            &format!("/api/tokens/{}", t["id"].as_str().unwrap()),
            &[("authorization", &auth)],
            json!(null)
        )
        .await
        .0,
        200
    );
    assert_eq!(
        request(
            &f.app,
            "GET",
            "/api/me",
            &[("authorization", &auth)],
            json!(null)
        )
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
