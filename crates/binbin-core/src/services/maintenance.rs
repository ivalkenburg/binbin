use crate::{
    App, DAY,
    error::Result,
    models::{Credential, Share, Transfer},
    storage::database::{all, put, remove},
};
use std::{
    collections::{HashMap, HashSet},
    fs,
};

impl App {
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
        self.db(|db| {
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
                let dead =
                    draft_dead || matches!(state.as_str(), "expired" | "exhausted" | "revoked");
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
            Ok(())
        })
    }
}
