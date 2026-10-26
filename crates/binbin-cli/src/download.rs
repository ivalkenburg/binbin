use super::{
    api::{Api, decode},
    config::secure_write,
    output::print,
    responses::{Share, Transfer},
};
use anyhow::{Context, Result, bail};
use binbin_core::crypto::secret;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
#[derive(Serialize, Deserialize)]
pub(super) struct LocalTransfer {
    pub(super) server: String,
    pub(super) share: String,
    pub(super) resource: String,
    pub(super) key: String,
    pub(super) token: Option<String>,
    pub(super) hash: String,
    pub(super) size: u64,
}

pub(super) struct OutputLock {
    pub(super) output: PathBuf,
    _file: std::fs::File,
}

impl OutputLock {
    pub(super) fn acquire(output: PathBuf) -> Result<Self> {
        let mut name = output.as_os_str().to_owned();
        name.push(".binbin-transfer.lock");
        let path = PathBuf::from(name);
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .context("Cannot open output lock")?;
        fs2::FileExt::try_lock_exclusive(&file)
            .context("Cannot lock output; another download may be using it")?;
        // Keep the lock file: removing it allows contenders to lock different
        // inodes. The OS releases the lock when this handle closes, even on exit.
        Ok(Self {
            output,
            _file: file,
        })
    }
}
pub(super) fn sidecar(output: &Path) -> PathBuf {
    let mut name = output.as_os_str().to_owned();
    name.push(".binbin-transfer.json");
    PathBuf::from(name)
}
pub(super) async fn hash_prefix(path: &Path, length: u64) -> Result<String> {
    let mut input = tokio::fs::File::open(path).await?.take(length);
    let mut h = Sha256::new();
    let mut b = vec![0u8; 65536];
    loop {
        let n = input.read(&mut b).await?;
        if n == 0 {
            break;
        }
        h.update(&b[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}
#[allow(clippy::too_many_arguments)]
pub(super) async fn download_resource(
    api: &Api,
    id: &str,
    resource: &str,
    output: &OutputLock,
    password: Option<String>,
    management: bool,
    metadata: &serde_json::Value,
    json_output: bool,
) -> Result<()> {
    let output = &output.output;
    let meta: Share = serde_json::from_value(metadata.clone()).context("Invalid share response")?;
    let resource = if resource == "zip" && meta.zip_hash.is_none() {
        meta.files
            .first()
            .context("No downloadable files")?
            .id
            .as_str()
    } else {
        resource
    };
    let (hash, size) = if resource == "zip" {
        (meta.zip_hash.context("No ZIP")?, meta.zip_size)
    } else {
        let file = meta
            .files
            .iter()
            .find(|file| file.id == resource)
            .context("File not found")?;
        (file.sha256.clone().context("Missing digest")?, file.size)
    };
    let state_path = sidecar(output);
    let old: Option<LocalTransfer> = match std::fs::read(&state_path) {
        Ok(bytes) => Some(
            serde_json::from_slice(&bytes)
                .context("Invalid transfer record; preserve it and choose another output path")?,
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error).context("Cannot read transfer record"),
    };
    let mut state = match old {
        Some(s)
            if s.server == api.server
                && s.share == id
                && s.resource == resource
                && s.hash == hash
                && s.size == size =>
        {
            s
        }
        Some(_) => bail!("Output belongs to a different transfer; choose another output path"),
        None => {
            anyhow::ensure!(
                !output.exists(),
                "Output already exists without a transfer record; choose another path"
            );
            LocalTransfer {
                server: api.server.clone(),
                share: id.into(),
                resource: resource.into(),
                key: secret(),
                token: None,
                hash: hash.clone(),
                size,
            }
        }
    };
    secure_write(&state_path, &serde_json::to_vec(&state)?)?;
    if state.token.is_none() {
        let route = if management { "shares" } else { "public" };
        let t = api
            .post(
                &format!("/api/{route}/{id}/transfers"),
                json!({"resource":resource,"password":password,"key":state.key}),
            )
            .await?;
        let transfer: Transfer = serde_json::from_value(t).context("Invalid transfer response")?;
        state.token = Some(transfer.token);
        secure_write(&state_path, &serde_json::to_vec(&state)?)?;
    }
    // Reconstruct the endpoint from the selected server and credential rather
    // than trusting a URL in the local retry record.
    let url = format!(
        "{}/d/{}",
        api.server,
        state
            .token
            .as_ref()
            .context("Missing transfer credential")?
    );
    let mut offset = tokio::fs::metadata(output)
        .await
        .map(|m| m.len())
        .unwrap_or(0);
    anyhow::ensure!(offset <= size, "Local output exceeds expected size");
    if offset < size || size == 0 {
        let mut req = api.client.get(&url);
        if offset > 0 {
            req = req
                .header("range", format!("bytes={offset}-"))
                .header("if-range", format!("\"{hash}\""));
        }
        let mut r = req.send().await?;
        let status = r.status();
        if !status.is_success() {
            let error = decode(r).await;
            return Err(error
                .err()
                .unwrap_or_else(|| anyhow::anyhow!("Download failed")));
        }
        if status == 200 {
            offset = 0;
        } else {
            anyhow::ensure!(
                status == 206
                    && r.headers()
                        .get("content-range")
                        .and_then(|v| v.to_str().ok())
                        .is_some_and(|s| s.starts_with(&format!("bytes {offset}-"))),
                "Invalid range response"
            );
        }
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(offset == 0)
            .append(offset > 0)
            .open(output)
            .await?;
        while let Some(chunk) = r.chunk().await? {
            file.write_all(&chunk).await?;
            offset += chunk.len() as u64;
            if offset > size {
                bail!("Server sent more data than expected")
            }
        }
        file.sync_all().await?;
    }
    anyhow::ensure!(
        offset == size,
        "Incomplete download; rerun the same command to resume"
    );
    let actual = hash_prefix(output, size).await?;
    anyhow::ensure!(
        actual == hash,
        "SHA-256 verification failed; remove only the corrupt output, keep its transfer record, and retry"
    );
    tokio::fs::remove_file(state_path).await?;
    if json_output {
        print(&json!({"output":output,"sha256":actual,"bytes":size}), true)
    } else {
        println!(
            "Saved {} ({size} bytes, SHA-256 verified)",
            output.display()
        )
    }
    Ok(())
}
