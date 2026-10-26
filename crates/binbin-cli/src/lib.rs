//! Cross-platform command-line client.
mod api;
mod args;
mod config;
mod download;
mod output;
mod responses;
mod upload;

use anyhow::{Context, Result, bail};
use api::Api;
use args::{Args, Command};
use clap::Parser;
use config::{Config, config_path, credential, read_token, secure_write};
use download::{LocalTransfer, OutputLock, download_resource, sidecar};
use output::{origin, print};
use reqwest::Client;
use serde_json::json;

pub async fn run() -> Result<()> {
    let args = Args::parse();
    let path = config_path()?;
    let cfg: Config = std::fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    let server = origin(args.server.as_deref().unwrap_or(if cfg.server.is_empty() {
        "http://localhost:8080"
    } else {
        &cfg.server
    }))?;
    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(std::time::Duration::from_secs(15))
        .build()?;
    if matches!(args.command, Command::Login) {
        let token = if args.token_file.is_some() || args.token_stdin || args.prompt_token {
            read_token(&args, &server)?.context("No token")?
        } else {
            rpassword::prompt_password("API token (create one in Settings → API tokens): ")?
                .trim()
                .to_owned()
        };
        let api = Api {
            client,
            server: server.clone(),
            token: Some(token.clone()),
        };
        api.get("/api/me").await?;
        credential(&server)?.set_password(&token).context(
            "Cannot save token in OS credential store; use a non-persistent token option instead",
        )?;
        secure_write(&path, &serde_json::to_vec(&Config { server })?)?;
        print(&json!({"authenticated":true}), args.json);
        return Ok(());
    }
    if matches!(args.command, Command::Logout) {
        match credential(&server)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(e) => return Err(e.into()),
        }
        print(
            &json!({"logged_out":true,"note":"The server token remains valid until revoked in the dashboard"}),
            args.json,
        );
        return Ok(());
    }
    let public = matches!(args.command, Command::Fetch { .. });
    let token = if public {
        None
    } else {
        read_token(&args, &server)?
    };
    if !public && token.is_none() {
        bail!("Run binbin login or provide --token-file, --token-stdin, or --prompt-token")
    }
    let api = Api {
        client,
        server,
        token,
    };
    match args.command {
        Command::List => print(&api.get("/api/shares").await?, args.json),
        Command::Inspect { id } => print(&api.get(&format!("/api/shares/{id}")).await?, args.json),
        Command::Revoke { id } | Command::Delete { id } => print(
            &api.call(reqwest::Method::DELETE, &format!("/api/shares/{id}"), None)
                .await?,
            args.json,
        ),
        Command::Upload(options) => upload::run(&api, options, args.json).await?,
        Command::Download {
            id,
            resource,
            output,
        } => {
            let output = OutputLock::acquire(output)?;
            let meta = api.get(&format!("/api/shares/{id}")).await?;
            download_resource(&api, &id, &resource, &output, None, true, &meta, args.json).await?;
        }
        Command::Fetch {
            url,
            resource,
            output,
            password,
        } => {
            let parsed = url::Url::parse(&url)?;
            let id = parsed
                .path()
                .strip_prefix("/s/")
                .filter(|id| !id.contains('/'))
                .context("Expected a Binbin /s/<id> link")?;
            let api = Api {
                client: api.client,
                server: origin(&url)?,
                token: None,
            };
            let output = OutputLock::acquire(output)?;
            let p = if password {
                Some(rpassword::prompt_password("Share password: ")?)
            } else {
                None
            };
            // A saved transfer remains usable after the public allowance is exhausted.
            let sidecar = sidecar(&output.output);
            let saved: Option<LocalTransfer> = std::fs::read(&sidecar)
                .ok()
                .and_then(|b| serde_json::from_slice(&b).ok());
            let meta = if let Some(s) = saved
                .as_ref()
                .filter(|s| s.server == api.server && s.share == id && s.token.is_some())
            {
                json!({"files":[{"id":s.resource,"name":s.resource,"size":s.size,"sha256":s.hash}],"zip_size":s.size,"zip_hash":if s.resource == "zip" { Some(&s.hash) } else { None }})
            } else {
                api.post(&format!("/api/public/{id}"), json!({"password":p}))
                    .await?
            };
            let resource = resource.unwrap_or_else(|| {
                if meta["zip_hash"].is_string() {
                    "zip".into()
                } else {
                    meta["files"][0]["id"].as_str().unwrap_or("").into()
                }
            });
            download_resource(&api, id, &resource, &output, p, false, &meta, args.json).await?;
        }
        Command::Login | Command::Logout => unreachable!(),
    }
    Ok(())
}
