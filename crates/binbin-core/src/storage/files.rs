use crate::{
    App,
    models::{Share, StoredFile},
};
use std::path::PathBuf;

impl App {
    pub fn file_path(&self, f: &StoredFile) -> PathBuf {
        self.root.join(&f.share).join(&f.id)
    }
    pub fn zip_path(&self, s: &Share) -> PathBuf {
        self.root.join(&s.id).join("archive.zip")
    }
}

pub(crate) fn sync_directory(path: &std::path::Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::fs::File::open(path)?.sync_all()?;
    }
    // Windows does not expose portable directory fsync; file handles are flushed
    // and atomic replacement is delegated to the platform's rename implementation.
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

pub(crate) fn create_directory(path: &std::path::Path) -> std::io::Result<()> {
    let missing: Vec<_> = path
        .ancestors()
        .filter(|ancestor| !ancestor.as_os_str().is_empty() && !ancestor.exists())
        .map(std::path::Path::to_path_buf)
        .collect();
    std::fs::create_dir_all(path)?;
    for directory in missing.iter().rev() {
        sync_directory(directory)?;
        let parent = directory
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(std::path::Path::new("."));
        sync_directory(parent)?;
    }
    Ok(())
}
