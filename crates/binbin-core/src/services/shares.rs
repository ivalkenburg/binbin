use crate::{
    App, DAY,
    crypto::{hash_password, secret, verify_password},
    error::{Error, Result, bad, unavailable},
    models::{EditShare, Identity, NewShare, Settings, Share, ShareView, StoredFile},
    storage::database::{all, get, put},
};
use rusqlite::Connection;
use std::{
    collections::HashSet,
    fs::{self, File},
};

pub(super) fn validate_file_name(name: &str, names: &mut HashSet<String>) -> Result<()> {
    if name.is_empty()
        || name.len() > 240
        || name == "."
        || name == ".."
        || name.chars().any(|c| c.is_control() || "/\\:".contains(c))
        || !names.insert(name.to_lowercase())
    {
        return Err(bad(
            "Filenames must be unique, safe basenames of at most 240 bytes",
        ));
    }
    Ok(())
}

pub(super) fn storage_reservation(size: u64, count: usize, held: u64) -> Result<u64> {
    let archive = if count > 1 {
        size.checked_add(count as u64 * 1024 + 1024)
            .ok_or_else(|| bad("Share is too large"))?
    } else {
        0
    };
    size.checked_add(held)
        .and_then(|total| total.checked_add(archive))
        .ok_or_else(|| bad("Share is too large"))
}

impl App {
    pub(crate) fn owns(&self, a: &Identity, s: &Share) -> Result<()> {
        if a.user.admin || a.user.id == s.owner {
            Ok(())
        } else {
            Err(unavailable())
        }
    }
    pub(crate) fn state(&self, s: &Share) -> String {
        if s.state == "active" {
            if s.expires.is_some_and(|t| t <= self.now()) {
                return "expired".into();
            }
            if s.limit.is_some_and(|n| s.used >= n) {
                return "exhausted".into();
            }
        }
        s.state.clone()
    }
    pub(crate) fn check_limits(
        &self,
        settings: &Settings,
        days: Option<u32>,
        limit: Option<u64>,
    ) -> Result<()> {
        if days == Some(0) || limit == Some(0) {
            return Err(bad("Limits must be positive, or null for unlimited"));
        }
        if settings
            .max_days
            .is_some_and(|max| days.is_none_or(|n| n > max))
        {
            return Err(bad("Expiry exceeds the administrator's maximum lifetime"));
        }
        if settings
            .max_downloads
            .is_some_and(|max| limit.is_none_or(|n| n > max))
        {
            return Err(bad(
                "Download allowance exceeds the administrator's maximum",
            ));
        }
        Ok(())
    }
    pub fn create_share(&self, a: &Identity, n: NewShare) -> Result<ShareView> {
        if n.title.trim().is_empty() || n.title.len() > 200 {
            return Err(bad("Title must contain 1–200 characters"));
        }
        let password = n
            .password
            .filter(|p| !p.is_empty())
            .map(|p| hash_password(&p))
            .transpose()?;
        self.authenticated_db(a, |db| {
            let settings: Settings = get(db, "settings", "instance")?.unwrap_or_default();
            self.check_limits(&settings, n.days, n.limit)?;
            if n.files.is_empty() || n.files.len() > settings.file_count {
                return Err(bad("Invalid number of files"));
            }
            let mut names = HashSet::new();
            let mut size = 0u64;
            let mut files = Vec::new();
            let id = secret();
            for f in &n.files {
                validate_file_name(&f.name, &mut names)?;
                if f.size > settings.file_limit {
                    return Err(bad("File exceeds the configured size limit"));
                }
                size = size
                    .checked_add(f.size)
                    .ok_or_else(|| bad("Share is too large"))?;
                files.push(StoredFile {
                    id: secret(),
                    share: id.clone(),
                    name: f.name.clone(),
                    size: f.size,
                    offset: 0,
                    sha256: None,
                });
            }
            if size > settings.share_limit {
                return Err(bad("Share exceeds the configured size limit"));
            }
            let reserved = storage_reservation(size, files.len(), 0)?;
            let used: u64 = all::<Share>(db, "share")?.iter().map(|s| s.reserved).sum();
            if used.saturating_add(reserved) > settings.storage_limit {
                return Err(Error(
                    507,
                    "Not enough available storage for the files and ZIP".into(),
                ));
            }
            let s = Share {
                id: id.clone(),
                owner: a.user.id.clone(),
                title: n.title,
                state: "draft".into(),
                created: self.now(),
                published: None,
                expires: None,
                days: n.days,
                limit: n.limit,
                used: 0,
                password,
                version: 0,
                reserved,
                original_size: size,
                zip_size: 0,
                zip_hash: None,
                removed: None,
                reason: None,
            };
            fs::create_dir_all(self.root.join(&id))?;
            for f in files {
                File::create(self.file_path(&f))?.sync_all()?;
                put(db, "file", &f.id, &f)?;
            }
            crate::storage::files::sync_directory(&self.root.join(&id))?;
            crate::storage::files::sync_directory(&self.root)?;
            put(db, "share", &id, &s)?;
            self.view_in(db, &s)
        })
    }
    pub(crate) fn files_in(&self, db: &Connection, id: &str) -> Result<Vec<StoredFile>> {
        crate::storage::database::files_for_share(db, id)
    }
    pub(crate) fn view_in(&self, db: &Connection, s: &Share) -> Result<ShareView> {
        let mut value = serde_json::to_value(s)?;
        let obj = value.as_object_mut().unwrap();
        obj.remove("password");
        obj.insert("password_protected".into(), s.password.is_some().into());
        obj.insert("state".into(), self.state(s).into());
        Ok(ShareView {
            share: value,
            files: self.files_in(db, &s.id)?,
            removed_files: self
                .removed_files_in(db, &s.id)?
                .into_iter()
                .filter(|f| f.removed_at + super::drafts::UNDO_SECONDS > self.now())
                .collect(),
            server_time: self.now(),
            url: format!("{}/s/{}", self.public_url, s.id),
        })
    }
    pub fn list_shares(&self, a: &Identity) -> Result<Vec<ShareView>> {
        self.authenticated_db(a, |db| {
            all::<Share>(db, "share")?
                .iter()
                .filter(|s| a.user.admin || s.owner == a.user.id)
                .rev()
                .map(|s| self.view_in(db, s))
                .collect()
        })
    }
    pub fn inspect(&self, a: &Identity, id: &str) -> Result<ShareView> {
        self.authenticated_db(a, |db| {
            let s: Share = get(db, "share", id)?.ok_or_else(unavailable)?;
            self.owns(a, &s)?;
            self.view_in(db, &s)
        })
    }
    pub fn edit(&self, a: &Identity, id: &str, e: EditShare) -> Result<ShareView> {
        let password = if e.change_password {
            e.password
                .filter(|s| !s.is_empty())
                .map(|p| hash_password(&p))
                .transpose()?
        } else {
            None
        };
        self.authenticated_db(a, |db| {
            let mut s: Share = get(db, "share", id)?.ok_or_else(unavailable)?;
            self.owns(a, &s)?;
            if self.state(&s) != "active" {
                return Err(bad("Only active shares can be edited"));
            }
            if e.title.trim().is_empty() || e.title.len() > 200 {
                return Err(bad("Invalid title"));
            }
            let settings: Settings = get(db, "settings", "instance")?.unwrap_or_default();
            if e.limit == Some(0)
                || settings
                    .max_downloads
                    .is_some_and(|m| e.limit.is_none_or(|n| n > m))
            {
                return Err(bad("Invalid download allowance"));
            }
            if settings.max_days.is_some_and(|d| {
                e.expires
                    .is_none_or(|t| t > s.published.unwrap_or(s.created) + i64::from(d) * DAY)
            }) {
                return Err(bad("Deadline exceeds maximum lifetime"));
            }
            s.title = e.title;
            s.expires = e.expires;
            s.limit = e.limit;
            if e.change_password {
                s.password = password;
                s.version += 1;
            }
            put(db, "share", id, &s)?;
            self.view_in(db, &s)
        })
    }
    pub fn revoke(&self, a: &Identity, id: &str) -> Result<()> {
        self.authenticated_db(a, |db| {
            let mut s: Share = get(db, "share", id)?.ok_or_else(unavailable)?;
            self.owns(a, &s)?;
            s.state = "revoked".into();
            s.removed = Some(self.now());
            s.reason = Some("revoked".into());
            s.version += 1;
            put(db, "share", id, &s)
        })?;
        self.cleanup()
    }
    pub fn public_share(&self, id: &str, password: Option<&str>) -> Result<serde_json::Value> {
        self.public_share_after_snapshot(id, password, || {})
    }
    pub(crate) fn public_share_after_snapshot(
        &self,
        id: &str,
        password: Option<&str>,
        after_snapshot: impl FnOnce(),
    ) -> Result<serde_json::Value> {
        self.with_public_access(id, password, false, after_snapshot, |db, s| {
            let files = self.files_in(db, id)?;
            Ok(serde_json::json!({
                "id": id, "title": s.title, "files": files,
                "zip_size": s.zip_size, "zip_hash": s.zip_hash,
                "expires": s.expires, "remaining": s.limit.map(|n| n.saturating_sub(s.used))
            }))
        })
    }
    pub(crate) fn with_public_access<T>(
        &self,
        id: &str,
        password: Option<&str>,
        allow_exhausted: bool,
        after_snapshot: impl FnOnce(),
        action: impl FnOnce(&Connection, Share) -> Result<T>,
    ) -> Result<T> {
        let available = |s: &Share| {
            let state = self.state(s);
            state == "active" || allow_exhausted && state == "exhausted"
        };
        let snapshot = self.db(|db| {
            let s: Share = get(db, "share", id)?.ok_or_else(unavailable)?;
            if !available(&s) {
                return Err(unavailable());
            }
            Ok((s.version, s.password))
        })?;
        // Password work and the deterministic test hook run without the database lock.
        after_snapshot();
        let verified = snapshot
            .1
            .as_ref()
            .is_none_or(|hash| password.is_some_and(|p| verify_password(p, hash)));
        self.db(|db| {
            let s: Share = get(db, "share", id)?.ok_or_else(unavailable)?;
            if !available(&s) {
                return Err(unavailable());
            }
            if s.version != snapshot.0 || s.password != snapshot.1 || !verified {
                return Err(Error(401, "Enter the share password".into()));
            }
            action(db, s)
        })
    }
}
