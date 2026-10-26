use crate::{
    api::{Api, decode},
    args::UploadArgs,
    download::hash_prefix,
    output::print,
    responses::{Share, UploadProgress},
};
use anyhow::{Context, Result};
use binbin_core::{
    CHUNK,
    models::{NewFile, NewShare},
};
use serde_json::json;
use std::collections::HashSet;
use tokio::io::AsyncReadExt;

pub(crate) async fn run(api: &Api, options: UploadArgs, json_output: bool) -> Result<()> {
    let UploadArgs {
        files,
        title,
        days,
        no_expiry,
        downloads,
        password,
        resume,
    } = options;

    let mut descriptions = Vec::new();
    for path in &files {
        let m = tokio::fs::metadata(path)
            .await
            .with_context(|| format!("Cannot read {}", path.display()))?;
        anyhow::ensure!(m.is_file(), "{} is not a regular file", path.display());
        descriptions.push(NewFile {
            name: path
                .file_name()
                .and_then(|n| n.to_str())
                .context("Filenames must be UTF-8")?
                .into(),
            size: m.len(),
        });
    }
    let share = if let Some(id) = resume {
        api.get(&format!("/api/shares/{id}")).await?
    } else {
        let settings = api.get("/api/settings").await?;
        let default = settings["settings"]["default_days"]
            .as_u64()
            .map(|n| n as u32);
        let n = NewShare {
            title: title.unwrap_or_else(|| {
                if descriptions.len() == 1 {
                    descriptions[0].name.clone()
                } else {
                    format!("{} files", descriptions.len())
                }
            }),
            files: descriptions.clone(),
            days: if no_expiry { None } else { days.or(default) },
            limit: downloads,
            password: if password {
                Some(rpassword::prompt_password(
                    "Share password (at least 12 characters): ",
                )?)
            } else {
                None
            },
        };
        api.post("/api/shares", serde_json::to_value(n)?).await?
    };
    let share: Share = serde_json::from_value(share).context("Invalid draft response")?;
    let id = &share.id;
    let token = api.token.as_deref().context("Missing upload credential")?;
    eprintln!("Draft {id}. Resume with: binbin upload --resume {id} <the same files>");
    let stored = &share.files;
    anyhow::ensure!(share.state == "draft", "Share is not a draft");
    anyhow::ensure!(
        stored.len() == files.len(),
        "Provide the same complete file set as the draft"
    );
    // Validate the entire manifest before probing, uploading, or publishing.
    // Matching each input independently would allow [A, A] for draft [A, B].
    let mut names = HashSet::new();
    let mut matched = HashSet::new();
    let mut manifest = Vec::with_capacity(descriptions.len());
    for desc in &descriptions {
        anyhow::ensure!(
            names.insert(&desc.name),
            "Duplicate local filename: {}",
            desc.name
        );
        let f = stored
            .iter()
            .find(|f| f.name == desc.name && f.size == desc.size)
            .context("Local filenames and sizes must match the draft")?;
        anyhow::ensure!(matched.insert(&f.id), "Draft files must match one-to-one");
        manifest.push(f);
    }
    for ((path, desc), f) in files.iter().zip(&descriptions).zip(manifest) {
        let mut input = tokio::fs::File::open(path).await?;
        let mut offset = f.offset;
        // Compare the committed prefix before resuming a same-size local file.
        if offset > 0 {
            use tokio::io::AsyncSeekExt;
            let resource = f.id.as_str();
            let local_hash = hash_prefix(path, offset).await?;
            let mut probe = api
                .client
                .request(
                    reqwest::Method::PATCH,
                    format!("{}/api/files/{resource}?offset={offset}", api.server),
                )
                .bearer_auth(token)
                .header("x-binbin-prefix-sha256", local_hash);
            probe = probe.body(Vec::new());
            decode(probe.send().await?).await?;
            input.seek(std::io::SeekFrom::Start(offset)).await?;
        }
        let mut buffer = vec![0u8; CHUNK];
        while offset < desc.size {
            let wanted = (desc.size - offset).min(CHUNK as u64) as usize;
            let count = input.read(&mut buffer[..wanted]).await?;
            anyhow::ensure!(count > 0, "File became shorter while uploading");
            let response = api
                .client
                .patch(format!(
                    "{}/api/files/{}?offset={offset}",
                    api.server,
                    f.id.as_str()
                ))
                .bearer_auth(token)
                .body(buffer[..count].to_vec())
                .send()
                .await?;
            let uploaded = decode(response).await?;
            let uploaded: UploadProgress =
                serde_json::from_value(uploaded).context("Invalid upload response")?;
            anyhow::ensure!(
                uploaded.offset > offset && uploaded.offset <= desc.size,
                "Invalid committed upload offset"
            );
            offset = uploaded.offset;
            eprintln!("{}: {offset}/{} bytes", desc.name, desc.size);
        }
    }
    eprintln!("Preparing share and ZIP…");
    let result = api
        .post(&format!("/api/shares/{id}/publish"), json!({}))
        .await?;
    if json_output {
        print(&result, true)
    } else {
        println!(
            "{}",
            result["url"]
                .as_str()
                .context("Missing published share URL")?
        )
    }

    Ok(())
}
