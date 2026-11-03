use crate::{
    App,
    crypto::digest,
    error::{Error, Result, bad, unavailable},
    models::{DownloadReservation, Identity, Share, Transfer},
    storage::database::{get, put},
};
use rusqlite::Connection;
use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};

const RESUME_SECONDS: i64 = 15 * 60;

/// A live page's transient receipt; never written to storage or kept after the
/// page disconnects. 0 = pending, 1 = deleted after delivery, 2 = removed otherwise.
pub struct DownloadObservation {
    app: App,
    share: String,
    outcome: Arc<AtomicU8>,
    version: u64,
    credential: Option<String>,
    transfer: Option<String>,
}
impl DownloadObservation {
    pub fn outcome(&self) -> u8 {
        self.outcome.load(Ordering::Acquire)
    }
    pub fn status(&self) -> Result<&'static str> {
        self.app.db(|db| {
            if let Some(credential) = &self.credential
                && !self.app.credential_valid_in(db, credential)?
            {
                return Ok("unavailable");
            }
            match self.outcome() {
                1 => return Ok("deleted"),
                2 => return Ok("unavailable"),
                _ => {}
            }
            if let Some(transfer) = &self.transfer
                && let Some(reservation) =
                    get::<DownloadReservation>(db, "download_reservation", &self.share)?
                && reservation.transfer != *transfer
                && self.app.download_reserved_in(db, &self.share)?
            {
                return Ok("blocked");
            }
            if let Some(share) = get::<Share>(db, "share", &self.share)? {
                // Revocation is final even if physical cleanup needs a retry.
                if share.state == "deleting" {
                    return Ok("deleting");
                }
                if share.version != self.version || share.state != "active" {
                    return Ok("unavailable");
                }
            }
            if self
                .app
                .active_downloads
                .lock()
                .map_err(|_| Error(500, "Download lock failed".into()))?
                .contains(&self.share)
            {
                return Ok("downloading");
            }
            Ok(
                if get::<DownloadReservation>(db, "download_reservation", &self.share)?
                    .is_some_and(|r| r.until > self.app.now())
                {
                    "interrupted"
                } else {
                    "available"
                },
            )
        })
    }
}
impl Drop for DownloadObservation {
    fn drop(&mut self) {
        if let Ok(mut observers) = self.app.download_observers.lock()
            && let Some(entries) = observers.get_mut(&self.share)
        {
            entries.retain(|(_, entry)| {
                entry.strong_count() > 0 && !entry.ptr_eq(&Arc::downgrade(&self.outcome))
            });
            if entries.is_empty() {
                observers.remove(&self.share);
            }
        }
    }
}

/// Exclusive public response. Drop releases the in-process lock; the durable
/// reservation and delivered ranges survive disconnects and process restarts.
pub struct DownloadLease {
    app: App,
    share: String,
    transfer: String,
    version: u64,
    resume_from: Option<u64>,
}

#[derive(Clone, Copy)]
enum Delivery {
    Checkpoint,
    Interrupted,
    Complete(u64),
}

impl Drop for DownloadLease {
    fn drop(&mut self) {
        if let Ok(mut active) = self.app.active_downloads.lock() {
            active.remove(&self.share);
        }
    }
}

impl DownloadLease {
    pub fn checkpoint(&self, start: u64, end: u64) -> Result<()> {
        self.update(start, end, Delivery::Checkpoint).map(|_| ())
    }

    pub fn interrupted(&self, start: u64, end: u64) -> Result<()> {
        self.update(start, end, Delivery::Interrupted).map(|_| ())
    }

    /// A crashed response may have sent bytes beyond its last checkpoint.
    /// A native resume must retransmit that gap rather than finish silently
    /// while the server still considers the resource incomplete.
    pub fn restart_required(&self, start: u64) -> bool {
        self.resume_from.is_some_and(|prefix| start > prefix)
    }

    /// Called only after the response reaches EOF, never merely on body drop.
    pub fn complete(&self, start: u64, end: u64, size: u64) -> Result<()> {
        if !self.update(start, end, Delivery::Complete(size))? {
            return Ok(());
        }
        // Revocation was committed separately, so failed filesystem cleanup can
        // never roll back access. Maintenance retries the deletion.
        if let Err(error) = self.app.cleanup_deletion_share(&self.share) {
            tracing::error!("Post-download cleanup failed: {error}");
        }
        Ok(())
    }

    fn update(&self, start: u64, end: u64, delivery: Delivery) -> Result<bool> {
        self.app.db(|db| {
            let mut share: Share = get(db, "share", &self.share)?.ok_or_else(unavailable)?;
            if share.state != "active" || share.version != self.version {
                return Err(unavailable());
            }
            let mut reservation: DownloadReservation =
                get(db, "download_reservation", &self.share)?.ok_or_else(unavailable)?;
            if reservation.transfer != self.transfer {
                return Err(unavailable());
            }
            if start > end {
                return Err(bad("Invalid transfer progress"));
            }
            if end > start {
                reservation.ranges.push((start, end));
                reservation.ranges.sort_unstable();
                let mut merged: Vec<(u64, u64)> = Vec::new();
                for (start, end) in reservation.ranges {
                    if let Some(last) = merged.last_mut()
                        && start <= last.1
                    {
                        last.1 = last.1.max(end);
                    } else {
                        merged.push((start, end));
                    }
                }
                if merged.len() > 4096 {
                    return Err(bad("Too many fragmented download ranges"));
                }
                reservation.ranges = merged;
            }
            reservation.until = self.app.now() + RESUME_SECONDS;
            reservation.active = matches!(delivery, Delivery::Checkpoint);
            let mut transfer: Transfer =
                get(db, "transfer", &self.transfer)?.ok_or_else(unavailable)?;
            transfer.expires = reservation.until;
            put(db, "transfer", &transfer.id, &transfer)?;
            put(db, "download_reservation", &self.share, &reservation)?;
            let completed = match delivery {
                Delivery::Complete(size) => {
                    size == 0
                        || reservation
                            .ranges
                            .first()
                            .is_some_and(|r| r.0 == 0 && r.1 >= size)
                }
                _ => false,
            };
            if completed {
                share.state = "deleting".into();
                share.version += 1;
                share.password = None;
                put(db, "share", &share.id, &share)?;
            }
            Ok(completed)
        })
    }
}

impl App {
    fn observation_in(
        &self,
        share: &Share,
        credential: Option<String>,
        transfer: Option<String>,
    ) -> Result<DownloadObservation> {
        let outcome = Arc::new(AtomicU8::new(0));
        let mut observers = self
            .download_observers
            .lock()
            .map_err(|_| Error(500, "Download observer failed".into()))?;
        let entries = observers.entry(share.id.clone()).or_default();
        entries.retain(|(_, entry)| entry.strong_count() > 0);
        entries.push((transfer.clone(), Arc::downgrade(&outcome)));
        Ok(DownloadObservation {
            app: self.clone(),
            share: share.id.clone(),
            outcome,
            version: share.version,
            credential,
            transfer,
        })
    }

    pub fn observe_share_download(
        &self,
        actor: &Identity,
        id: &str,
    ) -> Result<DownloadObservation> {
        self.authenticated_db(actor, |db| {
            let share: Share = get(db, "share", id)?.ok_or_else(unavailable)?;
            self.owns(actor, &share)?;
            if !share.delete_after_download || self.state(&share) != "active" {
                return Err(unavailable());
            }
            self.observation_in(&share, Some(actor.credential.id.clone()), None)
        })
    }
    pub fn observe_download(&self, raw: &str) -> Result<DownloadObservation> {
        self.download(raw, false)?;
        self.db(|db| {
            let transfer: Transfer = get(db, "transfer", &digest(raw))?.ok_or_else(unavailable)?;
            let share: Share = get(db, "share", &transfer.share)?.ok_or_else(unavailable)?;
            if !share.delete_after_download
                || transfer.management
                || share.state != "active"
                || share.version != transfer.version
                || transfer.expires <= self.now()
            {
                return Err(unavailable());
            }
            self.observation_in(&share, None, Some(transfer.id))
        })
    }

    pub(crate) fn notify_download_removed(&self, share: &str, completed_transfer: Option<&str>) {
        if let Ok(mut observers) = self.download_observers.lock()
            && let Some(entries) = observers.remove(share)
        {
            for (transfer, entry) in entries {
                if let Some(outcome) = entry.upgrade() {
                    let completed = completed_transfer.is_some_and(|completed| {
                        transfer
                            .as_deref()
                            .is_none_or(|transfer| transfer == completed)
                    });
                    outcome.store(if completed { 1 } else { 2 }, Ordering::Release);
                }
            }
        }
    }

    pub(crate) fn download_reserved_in(&self, db: &Connection, id: &str) -> Result<bool> {
        Ok(self
            .active_downloads
            .lock()
            .map_err(|_| Error(500, "Download lock failed".into()))?
            .contains(id)
            || get::<DownloadReservation>(db, "download_reservation", id)?
                .is_some_and(|r| r.until > self.now()))
    }

    pub(crate) fn check_download_reservation_in(
        &self,
        db: &Connection,
        share: &Share,
        transfer: &Transfer,
    ) -> Result<()> {
        if share.delete_after_download
            && !transfer.management
            && let Some(reservation) =
                get::<DownloadReservation>(db, "download_reservation", &share.id)?
            && reservation.transfer != transfer.id
            && self.download_reserved_in(db, &share.id)?
        {
            return Err(Error(
                409,
                "Download already in progress. Check again later.".into(),
            ));
        }
        Ok(())
    }

    pub fn deletion_download(&self, raw: &str) -> Result<bool> {
        self.db(|db| {
            let transfer: Transfer = get(db, "transfer", &digest(raw))?.ok_or_else(unavailable)?;
            let share: Share = get(db, "share", &transfer.share)?.ok_or_else(unavailable)?;
            Ok(share.delete_after_download && !transfer.management)
        })
    }

    pub fn begin_download(&self, raw: &str) -> Result<Option<DownloadLease>> {
        // Validate and charge ordinary transfers through the existing path.
        self.download(raw, true)?;
        self.db(|db| {
            let mut transfer: Transfer =
                get(db, "transfer", &digest(raw))?.ok_or_else(unavailable)?;
            let share: Share = get(db, "share", &transfer.share)?.ok_or_else(unavailable)?;
            if !share.delete_after_download || transfer.management {
                return Ok(None);
            }
            if share.state != "active"
                || share.version != transfer.version
                || share.expires.is_some_and(|expires| expires <= self.now())
            {
                return Err(unavailable());
            }
            self.check_download_reservation_in(db, &share, &transfer)?;
            let mut active = self
                .active_downloads
                .lock()
                .map_err(|_| Error(500, "Download lock failed".into()))?;
            if active.contains(&share.id) {
                return Err(Error(
                    409,
                    "Download already in progress. Check again later.".into(),
                ));
            }
            let existing: Option<DownloadReservation> = get(db, "download_reservation", &share.id)?;
            let mut reservation = existing
                .filter(|r| r.until > self.now() && r.transfer == transfer.id)
                .unwrap_or(DownloadReservation {
                    transfer: transfer.id.clone(),
                    until: 0,
                    ranges: vec![],
                    active: false,
                    uncertain: false,
                });
            // A persisted active response without an in-process lease was
            // interrupted before its final progress could be committed.
            reservation.uncertain |= reservation.active;
            reservation.active = true;
            let resume_from = reservation.uncertain.then(|| {
                reservation
                    .ranges
                    .first()
                    .filter(|range| range.0 == 0)
                    .map_or(0, |range| range.1)
            });
            reservation.until = self.now() + RESUME_SECONDS;
            transfer.expires = reservation.until;
            put(db, "transfer", &transfer.id, &transfer)?;
            put(db, "download_reservation", &share.id, &reservation)?;
            active.insert(share.id.clone());
            Ok(Some(DownloadLease {
                app: self.clone(),
                share: share.id,
                transfer: transfer.id,
                version: transfer.version,
                resume_from,
            }))
        })
    }
}
