use crate::{
    App, DAY,
    error::Result,
    models::{Credential, DownloadReservation, Share, Transfer},
    storage::database::{all, get, put, remove, remove_share_records},
};
use rusqlite::Connection;
use std::{
    collections::{HashMap, HashSet},
    fs,
};

impl App {
    fn deletion_cleanup_due(&self, share: &Share) -> bool {
        share.delete_after_download
            && (matches!(
                self.state(share).as_str(),
                "expired" | "exhausted" | "revoked" | "deleting"
            ) || share.state == "draft" && share.created + DAY <= self.now())
    }

    fn purge_deletion_share_in(&self, db: &Connection, share: &Share) -> Result<Option<String>> {
        let path = self.root.join(&share.id);
        if path.exists() {
            fs::remove_dir_all(path)?;
        }
        let completed = if share.state == "deleting" {
            get::<DownloadReservation>(db, "download_reservation", &share.id)?
                .map(|reservation| reservation.transfer)
        } else {
            None
        };
        remove_share_records(db, &share.id)?;
        Ok(completed)
    }

    pub(crate) fn cleanup_deletion_share(&self, id: &str) -> Result<()> {
        let removed = self.db(|db| {
            let Some(share) = get::<Share>(db, "share", id)? else {
                return Ok(None);
            };
            if !self.deletion_cleanup_due(&share) {
                return Ok(None);
            }
            let completed = self.purge_deletion_share_in(db, &share)?;
            Ok(Some((share.id, completed)))
        })?;
        if let Some((id, completed)) = removed {
            self.notify_download_removed(&id, completed.as_deref());
        }
        Ok(())
    }

    pub fn recover_publications(&self) -> Result<()> {
        self.db(|db| {
            for mut share in all::<Share>(db, "share")? {
                if share.state == "preparing" {
                    for name in ["archive.tmp", "archive.zip"] {
                        let path = self.root.join(&share.id).join(name);
                        if path.exists() {
                            fs::remove_file(path)?;
                        }
                    }
                    share.state = "draft".into();
                    put(db, "share", &share.id, &share)?;
                }
            }
            Ok(())
        })
    }
    pub fn cleanup(&self) -> Result<()> {
        // Commit each deletion independently before ordinary maintenance. One
        // inaccessible directory must not roll back other shares' removals or
        // prevent a completed share's later cleanup retry.
        let pending = self.db(|db| {
            Ok(all::<Share>(db, "share")?
                .into_iter()
                .filter(|share| self.deletion_cleanup_due(share))
                .map(|share| share.id)
                .collect::<Vec<_>>())
        })?;
        let mut deletion_error = None;
        for id in pending {
            if let Err(error) = self.cleanup_deletion_share(&id) {
                deletion_error.get_or_insert(error);
            }
        }
        let removed = self.db(|db| {
            let mut removed = Vec::new();
            let now = self.now();
            for c in all::<Credential>(db, "credential")? {
                if c.expires <= now {
                    remove(db, "credential", &c.id)?
                }
            }
            let transfers = all::<Transfer>(db, "transfer")?;
            let mut retained_versions = HashSet::new();
            for transfer in &transfers {
                if transfer.charged
                    && transfer.expires > now
                    && self.transfer_credential_valid_in(db, transfer)?
                {
                    retained_versions.insert((transfer.share.as_str(), transfer.version));
                }
            }
            let mut live_versions = HashMap::new();
            let mut ids = HashSet::new();
            for mut s in all::<Share>(db, "share")? {
                if s.state == "draft" {
                    self.purge_removed_in(db, &mut s, false)?;
                    // A crash between creating an added file and committing its
                    // metadata can leave an empty orphan inside an existing draft.
                    let known: HashSet<_> = self
                        .files_in(db, &s.id)?
                        .into_iter()
                        .map(|f| f.id)
                        .chain(
                            self.removed_files_in(db, &s.id)?
                                .into_iter()
                                .map(|f| f.file.id),
                        )
                        .collect();
                    let directory = self.root.join(&s.id);
                    if directory.exists() {
                        for entry in fs::read_dir(directory)? {
                            let entry = entry?;
                            let name = entry.file_name().to_string_lossy().into_owned();
                            if entry.file_type()?.is_file()
                                && name.len() == 43
                                && name
                                    .bytes()
                                    .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
                                && !known.contains(&name)
                            {
                                fs::remove_file(entry.path())?;
                            }
                        }
                    }
                }
                let state = self.state(&s);
                let draft_dead = s.state == "draft" && s.created + DAY <= now;
                let dead = draft_dead
                    || matches!(
                        state.as_str(),
                        "expired" | "exhausted" | "revoked" | "deleting"
                    );
                if dead && s.delete_after_download {
                    let completed = self.purge_deletion_share_in(db, &s)?;
                    removed.push((s.id.clone(), completed));
                    continue;
                }
                if dead {
                    if s.removed.is_none() {
                        s.removed = Some(now);
                        s.reason = Some(if draft_dead {
                            "abandoned".into()
                        } else {
                            state.clone()
                        });
                    }
                    let retain = state == "exhausted"
                        && retained_versions.contains(&(s.id.as_str(), s.version));
                    if !retain {
                        let path = self.root.join(&s.id);
                        if path.exists() {
                            fs::remove_dir_all(path)?;
                        }
                        for f in self.files_in(db, &s.id)? {
                            remove(db, "file", &f.id)?;
                        }
                        for f in self.removed_files_in(db, &s.id)? {
                            remove(db, "removed_file", &f.file.id)?;
                        }
                        s.reserved = 0;
                        s.zip_hash = None;
                        s.zip_size = 0;
                        s.password = None;
                        s.state = if draft_dead {
                            "abandoned".into()
                        } else {
                            state
                        };
                    }
                    put(db, "share", &s.id, &s)?;
                }
                if s.reserved == 0 && s.removed.is_some_and(|t| t + 30 * DAY <= now) {
                    remove(db, "share", &s.id)?;
                } else {
                    // Keep expired deterministic keys as tombstones while this version
                    // can still issue capabilities. Otherwise an old URL could revive.
                    if s.state == "active" {
                        live_versions.insert(s.id.clone(), s.version);
                    }
                    ids.insert(s.id);
                }
            }
            for transfer in &transfers {
                if live_versions.get(&transfer.share) != Some(&transfer.version) {
                    remove(db, "transfer", &transfer.id)?;
                }
            }
            // Remove directories left by interrupted draft transactions.
            for entry in fs::read_dir(&self.root)? {
                let entry = entry?;
                let name = entry.file_name().to_string_lossy().into_owned();
                if entry.file_type()?.is_dir()
                    && name.len() == 43
                    && name
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
                    && !ids.contains(&name)
                {
                    fs::remove_dir_all(entry.path())?;
                }
            }
            Ok(removed)
        })?;
        for (share, completed) in removed {
            self.notify_download_removed(&share, completed.as_deref());
        }
        deletion_error.map_or(Ok(()), Err)
    }
}
