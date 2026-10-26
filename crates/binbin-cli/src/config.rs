use super::args::Args;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    path::{Path, PathBuf},
};
#[derive(Default, Serialize, Deserialize)]
pub(super) struct Config {
    pub(super) server: String,
}
pub(super) fn config_path() -> Result<PathBuf> {
    Ok(directories::ProjectDirs::from("", "", "binbin")
        .context("Cannot locate configuration directory")?
        .config_dir()
        .join("config.json"))
}
pub(super) fn credential(server: &str) -> Result<keyring::Entry> {
    keyring::Entry::new("binbin", server)
        .context("Credential store unavailable; use --token-file, --token-stdin, or --prompt-token")
}
pub(super) fn read_token(args: &Args, server: &str) -> Result<Option<String>> {
    let count = u8::from(args.token_stdin)
        + u8::from(args.prompt_token)
        + u8::from(args.token_file.is_some());
    anyhow::ensure!(count <= 1, "Choose one token input method");
    let raw = if let Some(path) = &args.token_file {
        std::fs::read_to_string(path).context("Cannot read token file")?
    } else if args.token_stdin {
        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s)?;
        s
    } else if args.prompt_token {
        rpassword::prompt_password("API token: ")?
    } else {
        match credential(server)?.get_password() {
            Ok(t) => t,
            Err(keyring::Error::NoEntry) => return Ok(None),
            Err(e) => bail!(
                "Credential store unavailable: {e}. Use --token-file, --token-stdin, or --prompt-token"
            ),
        }
    };
    let raw = raw.trim().to_string();
    anyhow::ensure!(!raw.is_empty(), "Token is empty");
    Ok(Some(raw))
}
pub(super) fn secure_write(path: &Path, contents: &[u8]) -> Result<()> {
    atomic_write(path, |file| std::io::Write::write_all(file, contents))
}

fn atomic_write(
    path: &Path,
    write: impl FnOnce(&mut std::fs::File) -> std::io::Result<()>,
) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)?;
    // A sibling temporary file keeps replacement on one filesystem. NamedTempFile
    // creates it privately and persist replaces existing files atomically.
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    write(temporary.as_file_mut())?;
    temporary.as_file().sync_all()?;
    temporary.persist(path)?;
    #[cfg(unix)]
    std::fs::File::open(parent)?.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_update_preserves_the_existing_record() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("transfer.json");
        secure_write(&path, b"original credential").unwrap();
        let result = atomic_write(&path, |file| {
            std::io::Write::write_all(file, b"partial")?;
            Err(std::io::Error::other("interrupted write"))
        });
        assert!(result.is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"original credential");
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn replacement_removes_broad_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("transfer.json");
        std::fs::write(&path, b"old").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        secure_write(&path, b"new credential").unwrap();
        assert_eq!(
            std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}
