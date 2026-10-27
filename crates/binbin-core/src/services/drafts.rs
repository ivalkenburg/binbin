use super::shares::{storage_reservation, validate_file_name};
use crate::{
    App, DAY,
    crypto::secret,
    error::{Error, Result, bad, unavailable},
    models::{EditFiles, Identity, RemovedFile, Settings, Share, ShareView, StoredFile},
    storage::database::{all, get, put, remove},
};
use rusqlite::Connection;
use std::{
    collections::HashSet,
    fs::File,
    io::{Read, Seek, SeekFrom},
};

pub(super) const UNDO_SECONDS: i64 = 8;

fn reservation(files: &[StoredFile], removed: &[RemovedFile]) -> Result<(u64, u64)> {
    let size = files
        .iter()
        .try_fold(0u64, |sum, file| sum.checked_add(file.size))
        .ok_or_else(|| bad("Share is too large"))?;
    let held = removed
        .iter()
        .try_fold(0u64, |sum, file| sum.checked_add(file.file.size))
        .ok_or_else(|| bad("Share is too large"))?;
    Ok((size, storage_reservation(size, files.len(), held)?))
}

impl App {
    pub(crate) fn removed_files_in(&self, db: &Connection, id: &str) -> Result<Vec<RemovedFile>> {
        let mut statement = db.prepare(
            "SELECT data FROM objects WHERE kind='removed_file' AND json_extract(data,'$.file.share')=? ORDER BY rowid",
        )?;
        statement
            .query_map([id], |row| row.get::<_, String>(0))?
            .map(|row| Ok(serde_json::from_str(&row?)?))
            .collect()
    }

    pub(crate) fn purge_removed_in(
        &self,
        db: &Connection,
        share: &mut Share,
        commit: bool,
    ) -> Result<()> {
        for mut removed in self.removed_files_in(db, &share.id)? {
            if commit && removed.removed_at + UNDO_SECONDS > self.now() {
                // Commit the end of Undo before a later transaction unlinks bytes.
                removed.removed_at = self.now() - UNDO_SECONDS;
                put(db, "removed_file", &removed.file.id, &removed)?;
            } else if removed.removed_at + UNDO_SECONDS <= self.now() {
                let path = self.file_path(&removed.file);
                match std::fs::remove_file(path) {
                    Ok(()) => (),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
                    Err(error) => return Err(error.into()),
                }
                remove(db, "removed_file", &removed.file.id)?;
            }
        }
        let (size, reserved) = reservation(
            &self.files_in(db, &share.id)?,
            &self.removed_files_in(db, &share.id)?,
        )?;
        share.original_size = size;
        share.reserved = reserved;
        put(db, "share", &share.id, share)
    }

    pub fn edit_files(&self, actor: &Identity, id: &str, edit: EditFiles) -> Result<ShareView> {
        let mut created = Vec::new();
        let result = self.authenticated_db(actor, |db| {
            let mut share: Share = get(db, "share", id)?.ok_or_else(unavailable)?;
            self.owns(actor, &share)?;
            if share.state != "draft" || share.created + DAY <= self.now() {
                return Err(bad("Only unexpired private drafts can change their files"));
            }
            self.purge_removed_in(db, &mut share, false)?;
            let mut files = self.files_in(db, id)?;
            let previous_count = files.len();
            let previous_size = share.original_size;
            let previous_reservation = share.reserved;
            let mut removed = self.removed_files_in(db, id)?;
            let mut touched = HashSet::new();
            for file_id in &edit.remove {
                if !touched.insert(file_id) {
                    return Err(bad("A file can only be changed once per request"));
                }
                let index = files
                    .iter()
                    .position(|file| &file.id == file_id)
                    .ok_or_else(unavailable)?;
                removed.push(RemovedFile {
                    file: files.remove(index),
                    removed_at: self.now(),
                });
            }
            for file_id in &edit.restore {
                if !touched.insert(file_id) {
                    return Err(bad("A file can only be changed once per request"));
                }
                let index = removed
                    .iter()
                    .position(|file| {
                        &file.file.id == file_id && file.removed_at + UNDO_SECONDS > self.now()
                    })
                    .ok_or_else(|| bad("The undo period has ended"))?;
                files.push(removed.remove(index).file);
            }
            let additions: Vec<_> = edit
                .add
                .into_iter()
                .map(|file| StoredFile {
                    id: secret(),
                    share: id.into(),
                    name: file.name,
                    size: file.size,
                    offset: 0,
                    sha256: None,
                })
                .collect();
            files.extend(additions.iter().cloned());
            let settings: Settings = get(db, "settings", "instance")?.unwrap_or_default();
            if files.len() > settings.file_count && files.len() > previous_count {
                return Err(bad("Too many files in this share"));
            }
            let incoming: HashSet<_> = additions
                .iter()
                .map(|file| file.id.as_str())
                .chain(edit.restore.iter().map(String::as_str))
                .collect();
            let mut names = HashSet::new();
            for file in &files {
                validate_file_name(&file.name, &mut names)?;
                if file.size > settings.file_limit && incoming.contains(file.id.as_str()) {
                    return Err(bad("File exceeds the configured size limit"));
                }
            }
            // Names held for Undo cannot be reused by additions or a different restored file.
            for held in &removed {
                if held.removed_at + UNDO_SECONDS > self.now()
                    && names.contains(&held.file.name.to_lowercase())
                {
                    return Err(bad("A filename is reserved for Undo; use a different name"));
                }
            }
            let (size, reserved) = reservation(&files, &removed)?;
            if size > settings.share_limit && size > previous_size {
                return Err(bad("Share exceeds the configured size limit"));
            }
            let used: u64 = all::<Share>(db, "share")?
                .iter()
                .filter(|s| s.id != id)
                .map(|s| s.reserved)
                .sum();
            if used.saturating_add(reserved) > settings.storage_limit
                && reserved > previous_reservation
            {
                return Err(Error(
                    507,
                    "Not enough available storage for the files and ZIP".into(),
                ));
            }
            // Validation precedes writes, so a rejected edit leaves the collection intact.
            for file in &additions {
                let path = self.file_path(file);
                let output = File::create_new(&path)?;
                created.push(path);
                output.sync_all()?;
            }
            if !additions.is_empty() {
                crate::storage::files::sync_directory(&self.root.join(id))?;
            }
            for file_id in &edit.remove {
                remove(db, "file", file_id)?;
            }
            for file_id in &edit.restore {
                remove(db, "removed_file", file_id)?;
            }
            for file in &files {
                put(db, "file", &file.id, file)?;
            }
            for file in &removed {
                put(db, "removed_file", &file.file.id, file)?;
            }
            share.version += 1;
            share.original_size = size;
            share.reserved = reserved;
            put(db, "share", id, &share)?;
            self.purge_removed_in(db, &mut share, false)?;
            self.view_in(db, &share)
        });
        if result.is_err() {
            for path in created {
                let _ = std::fs::remove_file(path);
            }
        }
        result
    }

    pub fn image_preview(&self, actor: &Identity, id: &str) -> Result<(File, &'static str, u64)> {
        let (mut input, size) = self.authenticated_db(actor, |db| {
            let file: StoredFile = get(db, "file", id)?.ok_or_else(unavailable)?;
            let share: Share = get(db, "share", &file.share)?.ok_or_else(unavailable)?;
            self.owns(actor, &share)?;
            if !matches!(
                self.state(&share).as_str(),
                "draft" | "active" | "exhausted"
            ) || share.state == "draft" && share.created + DAY <= self.now()
                || file.offset != file.size
            {
                return Err(unavailable());
            }
            Ok((File::open(self.file_path(&file))?, file.size))
        })?;
        let mut header = [0u8; 64];
        let count = input.read(&mut header)?;
        let bytes = &header[..count];
        let mime = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            "image/png"
        } else if bytes.starts_with(b"\xff\xd8\xff") {
            "image/jpeg"
        } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
            "image/gif"
        } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
            "image/webp"
        } else if bytes.get(4..8) == Some(b"ftyp")
            && bytes
                .get(8..12)
                .is_some_and(|brand| brand == b"avif" || brand == b"avis")
        {
            "image/avif"
        } else {
            return Err(Error(
                415,
                "Preview is available for PNG, JPEG, GIF, WebP and AVIF images".into(),
            ));
        };
        input.seek(SeekFrom::Start(0))?;
        Ok((input, mime, size))
    }
}
