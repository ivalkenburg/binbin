use crate::{
    App, DAY,
    crypto::digest,
    error::{Result, bad, unavailable},
    models::{Identity, Share, StoredFile, Transfer},
    storage::database::{get, put},
};
use std::path::PathBuf;

impl App {
    pub fn new_transfer(
        &self,
        id: &str,
        resource: &str,
        password: Option<&str>,
        key: &str,
        a: Option<&Identity>,
    ) -> Result<(String, Transfer)> {
        self.new_transfer_after_snapshot(id, resource, password, key, a, || {})
    }
    pub(crate) fn new_transfer_after_snapshot(
        &self,
        id: &str,
        resource: &str,
        password: Option<&str>,
        key: &str,
        a: Option<&Identity>,
        after_snapshot: impl FnOnce(),
    ) -> Result<(String, Transfer)> {
        if key.len() < 16 || key.len() > 200 {
            return Err(bad("An idempotency key of 16–200 characters is required"));
        }
        let issue = |db: &rusqlite::Connection, s: Share| {
            if resource == "zip" {
                if s.zip_hash.is_none() {
                    return Err(unavailable());
                }
            } else {
                let f: StoredFile = get(db, "file", resource)?.ok_or_else(unavailable)?;
                if f.share != id {
                    return Err(unavailable());
                }
            }
            // Derive a stable capability from a client-supplied random retry key; store only its hash.
            let seed = format!(
                "{id}:{resource}:{key}:{}:{}",
                s.version,
                a.map_or("public", |a| a.credential.id.as_str())
            );
            let mut raw = if a.is_none() {
                digest(format!("public-v2:{seed}"))
            } else {
                digest(&seed)
            };
            let mut tid = digest(&raw);
            let mut existing = get::<Transfer>(db, "transfer", &tid)?;
            // Reuse recorded legacy public capabilities, but never recreate a
            // missing legacy URL that an earlier cleanup may have expired.
            if existing.is_none() && a.is_none() {
                let legacy = digest(&seed);
                let legacy_id = digest(&legacy);
                if let Some(transfer) = get::<Transfer>(db, "transfer", &legacy_id)? {
                    raw = legacy;
                    tid = legacy_id;
                    existing = Some(transfer);
                }
            }
            if let Some(t) = existing {
                if t.version != s.version
                    || t.expires <= self.now()
                    || !self.transfer_credential_valid_in(db, &t)?
                    || (!t.charged && !t.management && self.state(&s) != "active")
                {
                    return Err(unavailable());
                }
                return Ok((raw, t));
            }
            if a.is_none() && self.state(&s) != "active" {
                return Err(unavailable());
            }
            let t = Transfer {
                id: tid.clone(),
                share: id.into(),
                resource: resource.into(),
                version: s.version,
                created: self.now(),
                expires: self.now() + DAY,
                charged: false,
                management: a.is_some(),
                credential: a.map(|a| a.credential.id.clone()),
            };
            put(db, "transfer", &tid, &t)?;
            Ok((raw, t))
        };
        if let Some(a) = a {
            self.db(|db| {
                let s: Share = get(db, "share", id)?.ok_or_else(unavailable)?;
                self.validate_identity_in(db, a)?;
                self.owns(a, &s)?;
                if !matches!(self.state(&s).as_str(), "active" | "exhausted") {
                    return Err(unavailable());
                }
                issue(db, s)
            })
        } else {
            self.with_public_access(id, password, true, after_snapshot, issue)
        }
    }
    pub(crate) fn transfer_credential_valid_in(
        &self,
        db: &rusqlite::Connection,
        transfer: &Transfer,
    ) -> Result<bool> {
        if !transfer.management {
            return Ok(true);
        }
        match transfer.credential.as_deref() {
            Some(id) => self.credential_valid_in(db, id),
            None => Ok(false),
        }
    }
    pub fn download(&self, raw: &str, charge: bool) -> Result<(PathBuf, u64, String, String)> {
        self.db(|db| {
            let mut t: Transfer = get(db, "transfer", &digest(raw))?.ok_or_else(unavailable)?;
            let mut s: Share = get(db, "share", &t.share)?.ok_or_else(unavailable)?;
            if !self.transfer_credential_valid_in(db, &t)? {
                return Err(unavailable());
            }
            if t.expires <= self.now()
                || s.version != t.version
                || s.expires.is_some_and(|e| e <= self.now())
                || s.state != "active"
            {
                return Err(unavailable());
            }
            if !t.charged && charge {
                if !t.management && s.limit.is_some_and(|n| s.used >= n) {
                    return Err(unavailable());
                }
                if !t.management {
                    s.used += 1;
                    put(db, "share", &s.id, &s)?;
                }
                t.charged = true;
                t.created = self.now();
                t.expires = self.now() + DAY;
                put(db, "transfer", &t.id, &t)?;
            }
            if !t.charged && !t.management && s.limit.is_some_and(|n| s.used >= n) {
                return Err(unavailable());
            }
            if t.resource == "zip" {
                Ok((
                    self.zip_path(&s),
                    s.zip_size,
                    format!("{}.zip", s.title),
                    s.zip_hash.ok_or_else(unavailable)?,
                ))
            } else {
                let f: StoredFile = get(db, "file", &t.resource)?.ok_or_else(unavailable)?;
                Ok((
                    self.file_path(&f),
                    f.size,
                    f.name,
                    f.sha256.ok_or_else(unavailable)?,
                ))
            }
        })
    }
}
