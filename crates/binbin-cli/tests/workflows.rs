use binbin_core::{
    App,
    crypto::{digest, secret},
    models::{NewFile, NewShare},
};
use serde_json::{Value, json};
use std::process::{Child, Command, Output, Stdio};

fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_binbin"))
        .args(args)
        .output()
        .unwrap()
}
fn success(out: Output) -> Value {
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_upload_public_fetch_and_management_download() {
    let dir = tempfile::tempdir().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let app = App::open(dir.path().join("server"), url.clone()).unwrap();
    let u = app.create_admin("admin", "integration-password").unwrap();
    let (token, actor) = app.issue(&u, "cli-test", false).unwrap();
    let token_file = dir.path().join("token");
    std::fs::write(&token_file, token).unwrap();
    let input = dir.path().join("sample.txt");
    std::fs::write(&input, b"hello CLI").unwrap();
    let http = app.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, binbin_server::router(http))
            .await
            .unwrap()
    });
    let work = dir.path().to_owned();
    let app2 = app.clone();
    let a2 = actor.clone();
    let address = url.clone();
    tokio::task::spawn_blocking(move || {
        let token_file = token_file.to_str().unwrap();
        let input = input.to_str().unwrap();
        let uploaded = success(cli(&[
            "--server",
            &address,
            "--token-file",
            token_file,
            "--json",
            "upload",
            input,
            "--no-expiry",
            "--downloads",
            "1",
        ]));
        let id = uploaded["id"].as_str().unwrap();
        let file = uploaded["files"][0]["id"].as_str().unwrap();
        let listed = success(cli(&[
            "--server",
            &address,
            "--token-file",
            token_file,
            "--json",
            "list",
        ]));
        assert_eq!(listed.as_array().unwrap().len(), 1);
        let management = work.join("management.txt");
        success(cli(&[
            "--server",
            &address,
            "--token-file",
            token_file,
            "--json",
            "download",
            id,
            "--resource",
            file,
            "--output",
            management.to_str().unwrap(),
        ]));
        assert_eq!(app2.inspect(&a2, id).unwrap().share["used"], 0);
        let public = work.join("public.txt");
        success(cli(&[
            "--json",
            "fetch",
            uploaded["url"].as_str().unwrap(),
            "--output",
            public.to_str().unwrap(),
        ]));
        assert_eq!(std::fs::read(public).unwrap(), b"hello CLI");
        assert_eq!(app2.inspect(&a2, id).unwrap().share["used"], 1);
    })
    .await
    .unwrap();
    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_resumes_a_charged_download_after_share_exhaustion() {
    let dir = tempfile::tempdir().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let app = App::open(dir.path().join("server"), url.clone()).unwrap();
    let u = app.create_admin("admin", "integration-password").unwrap();
    let (_, actor) = app.issue(&u, "test", false).unwrap();
    let s = app
        .create_share(
            &actor,
            NewShare {
                title: "Resume".into(),
                files: vec![NewFile {
                    name: "hello.txt".into(),
                    size: 5,
                }],
                days: None,
                limit: Some(1),
                password: None,
            },
        )
        .unwrap();
    let id = s.share["id"].as_str().unwrap();
    app.upload(&actor, &s.files[0].id, 0, b"hello").unwrap();
    app.publish(&actor, id).unwrap();
    let key = secret();
    let (token, _) = app
        .new_transfer(id, &s.files[0].id, None, &key, None)
        .unwrap();
    app.download(&token, true).unwrap();
    let output = dir.path().join("result.txt");
    std::fs::write(&output, b"he").unwrap();
    // Old records may contain a URL. It must remain readable and must never
    // redirect the credential away from the selected server.
    let state = json!({"url":"http://127.0.0.1:1/untrusted","server":url,"share":id,"resource":s.files[0].id,"key":key,"token":token,"hash":digest(b"hello"),"size":5});
    std::fs::write(
        format!("{}.binbin-transfer.json", output.display()),
        serde_json::to_vec(&state).unwrap(),
    )
    .unwrap();
    let link = s.url;
    let http = app.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, binbin_server::router(http))
            .await
            .unwrap()
    });
    tokio::task::spawn_blocking(move || {
        success(cli(&[
            "--json",
            "fetch",
            &link,
            "--output",
            output.to_str().unwrap(),
        ]));
        assert_eq!(std::fs::read(output).unwrap(), b"hello");
    })
    .await
    .unwrap();
    assert_eq!(app.inspect(&actor, id).unwrap().share["used"], 1);
    server.abort();
}

fn start_cli(args: &[&str]) -> Child {
    Command::new(env!("CARGO_BIN_EXE_binbin"))
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

async fn finish_cli(mut child: Child) -> Output {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(15);
    while child.try_wait().unwrap().is_none() {
        if tokio::time::Instant::now() >= deadline {
            child.kill().unwrap();
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    child.wait_with_output().unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_serializes_same_output_and_recovers_after_process_death() {
    use axum::{
        extract::Request,
        middleware::{Next, from_fn},
    };
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let dir = tempfile::tempdir().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let app = App::open(dir.path().join("server"), url).unwrap();
    let user = app.create_admin("admin", "integration-password").unwrap();
    let (_, actor) = app.issue(&user, "test", false).unwrap();
    let draft = app
        .create_share(
            &actor,
            NewShare {
                title: "Concurrent download".into(),
                files: vec![NewFile {
                    name: "hello.txt".into(),
                    size: 5,
                }],
                days: None,
                limit: Some(2),
                password: None,
            },
        )
        .unwrap();
    let id = draft.share["id"].as_str().unwrap();
    app.upload(&actor, &draft.files[0].id, 0, b"hello").unwrap();
    let published = app.publish(&actor, id).unwrap();
    let issuances = Arc::new(AtomicUsize::new(0));
    let download_entered = Arc::new(tokio::sync::Notify::new());
    let release_download = Arc::new(tokio::sync::Semaphore::new(0));
    let http = binbin_server::router(app.clone()).layer(from_fn({
        let issuances = issuances.clone();
        let entered = download_entered.clone();
        let release = release_download.clone();
        move |request: Request, next: Next| {
            let issuances = issuances.clone();
            let entered = entered.clone();
            let release = release.clone();
            async move {
                if request.uri().path().ends_with("/transfers") {
                    issuances.fetch_add(1, Ordering::SeqCst);
                }
                if request.uri().path().starts_with("/d/") {
                    entered.notify_one();
                    release.acquire().await.unwrap().forget();
                }
                next.run(request).await
            }
        }
    }));
    let server = tokio::spawn(async move { axum::serve(listener, http).await.unwrap() });
    let output = dir.path().join("result.txt");
    let args = [
        "--json",
        "fetch",
        &published.url,
        "--output",
        output.to_str().unwrap(),
    ];
    let mut first = start_cli(&args);
    if tokio::time::timeout(
        std::time::Duration::from_secs(15),
        download_entered.notified(),
    )
    .await
    .is_err()
    {
        first.kill().unwrap();
        let result = first.wait_with_output().unwrap();
        panic!(
            "First download did not reach the server: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    // Hold the first request after issuance so the second process must compete
    // while the output/sidecar pair is owned by the first process.
    let competing = finish_cli(start_cli(&args)).await;
    first.kill().unwrap();
    first.wait().unwrap();
    let state_path = format!("{}.binbin-transfer.json", output.display());
    let state: Value = serde_json::from_slice(&std::fs::read(&state_path).unwrap()).unwrap();
    assert!(state.get("url").is_none());
    assert!(state["token"].is_string());
    assert!(!output.exists());
    assert!(!competing.status.success());
    assert!(String::from_utf8_lossy(&competing.stderr).contains("Cannot lock output"));
    // Killing the process releases the OS lock without deleting its lock file.
    // The retry reuses the saved credential and issues no second transfer.
    release_download.add_permits(2);
    success(finish_cli(start_cli(&args)).await);
    assert_eq!(std::fs::read(&output).unwrap(), b"hello");
    assert!(!std::path::Path::new(&state_path).exists());
    let existing = finish_cli(start_cli(&args)).await;
    assert!(!existing.status.success());
    assert!(String::from_utf8_lossy(&existing.stderr).contains("Output already exists"));
    assert_eq!(std::fs::read(&output).unwrap(), b"hello");
    assert_eq!(issuances.load(Ordering::SeqCst), 1);
    assert_eq!(app.inspect(&actor, id).unwrap().share["used"], 1);
    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_resume_validates_the_complete_manifest_before_requests() {
    use axum::{
        extract::Request,
        middleware::{Next, from_fn},
    };
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let dir = tempfile::tempdir().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let app = App::open(dir.path().join("server"), url.clone()).unwrap();
    let user = app.create_admin("admin", "integration-password").unwrap();
    let (token, actor) = app.issue(&user, "test", false).unwrap();
    let token_file = dir.path().join("token");
    std::fs::write(&token_file, token).unwrap();
    let first_file = dir.path().join("a.txt");
    let wrong_file = dir.path().join("c.txt");
    std::fs::write(&first_file, b"aaa").unwrap();
    std::fs::write(&wrong_file, b"ccc").unwrap();
    let draft = app
        .create_share(
            &actor,
            NewShare {
                title: "Complete draft".into(),
                files: vec![
                    NewFile {
                        name: "a.txt".into(),
                        size: 3,
                    },
                    NewFile {
                        name: "b.txt".into(),
                        size: 3,
                    },
                ],
                days: None,
                limit: None,
                password: None,
            },
        )
        .unwrap();
    for (file, contents) in draft.files.iter().zip([b"aaa", b"bbb"]) {
        app.upload(&actor, &file.id, 0, contents).unwrap();
    }
    let mutations = Arc::new(AtomicUsize::new(0));
    let http = binbin_server::router(app.clone()).layer(from_fn({
        let mutations = mutations.clone();
        move |request: Request, next: Next| {
            let mutations = mutations.clone();
            async move {
                if request.method() == "PATCH" || request.uri().path().ends_with("/publish") {
                    mutations.fetch_add(1, Ordering::SeqCst);
                }
                next.run(request).await
            }
        }
    }));
    let server = tokio::spawn(async move { axum::serve(listener, http).await.unwrap() });
    let id = draft.share["id"].as_str().unwrap();
    for (second, message) in [
        (&first_file, "Duplicate local filename"),
        (
            &wrong_file,
            "Local filenames and sizes must match the draft",
        ),
    ] {
        let result = finish_cli(start_cli(&[
            "--server",
            &url,
            "--token-file",
            token_file.to_str().unwrap(),
            "--json",
            "upload",
            "--resume",
            id,
            first_file.to_str().unwrap(),
            second.to_str().unwrap(),
        ]))
        .await;
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains(message));
        assert_eq!(app.inspect(&actor, id).unwrap().share["state"], "draft");
        assert_eq!(mutations.load(Ordering::SeqCst), 0);
    }
    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_non_json_errors_keep_the_http_status() {
    let dir = tempfile::tempdir().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let token_file = dir.path().join("token");
    std::fs::write(&token_file, "test-token").unwrap();
    let http = axum::Router::new().route(
        "/api/shares",
        axum::routing::get(|| async {
            (axum::http::StatusCode::BAD_GATEWAY, "Upstream unavailable")
        }),
    );
    let server = tokio::spawn(async move { axum::serve(listener, http).await.unwrap() });
    let result = finish_cli(start_cli(&[
        "--server",
        &url,
        "--token-file",
        token_file.to_str().unwrap(),
        "list",
    ]))
    .await;
    assert!(!result.status.success());
    let error = String::from_utf8_lossy(&result.stderr);
    assert!(error.contains("502 Bad Gateway"), "{error}");
    assert!(error.contains("Server did not return JSON"), "{error}");
    server.abort();
}
