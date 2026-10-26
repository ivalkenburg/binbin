use crate::{
    App, CHUNK, DAY,
    crypto::digest,
    error::{Error, Result, bad, unavailable},
    models::{Identity, Settings, Share, ShareView, StoredFile},
    storage::database::{get, put},
};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
};

impl App {
    fn resume_snapshot(&self, actor: &Identity, id: &str) -> Result<(File, u64)> {
        self.authenticated_db(actor, |db| {
            let file: StoredFile = get(db, "file", id)?.ok_or_else(unavailable)?;
            let share: Share = get(db, "share", &file.share)?.ok_or_else(unavailable)?;
            self.owns(actor, &share)?;
            if share.state != "draft" || share.created + DAY <= self.now() {
                return Err(bad("Draft is no longer resumable"));
            }
            Ok((File::open(self.file_path(&file))?, file.offset))
        })
    }
    pub fn upload_proofs(&self, actor: &Identity, id: &str) -> Result<Vec<String>> {
        let (file, offset) = self.resume_snapshot(actor, id)?;
        Self::proofs_from_reader(file.take(offset), offset)
    }
    fn proofs_from_reader(mut input: impl Read, mut remaining: u64) -> Result<Vec<String>> {
        let mut proofs = Vec::new();
        while remaining > 0 {
            let mut bytes = vec![0u8; remaining.min(CHUNK as u64) as usize];
            input.read_exact(&mut bytes)?;
            proofs.push(digest(&bytes));
            remaining -= bytes.len() as u64;
        }
        Ok(proofs)
    }
    pub fn verify_upload_prefix(&self, actor: &Identity, id: &str, expected: &str) -> Result<()> {
        let (file, offset) = self.resume_snapshot(actor, id)?;
        let mut input = file.take(offset);
        let mut hash = Sha256::new();
        let mut buffer = [0u8; 65536];
        loop {
            let count = input.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            hash.update(&buffer[..count]);
        }
        if format!("{:x}", hash.finalize()) != expected {
            return Err(bad(
                "Local file differs from the uploaded prefix; start a new draft",
            ));
        }
        Ok(())
    }
    pub fn upload(&self, a: &Identity, id: &str, offset: u64, data: &[u8]) -> Result<StoredFile> {
        if data.len() > CHUNK {
            return Err(Error(413, "Upload chunks must be at most 8 MiB".into()));
        }
        self.authenticated_db(a, |db| {
            let mut f: StoredFile = get(db, "file", id)?.ok_or_else(unavailable)?;
            let s: Share = get(db, "share", &f.share)?.ok_or_else(unavailable)?;
            self.owns(a, &s)?;
            if s.state != "draft" || s.created + DAY <= self.now() {
                return Err(bad("Draft is no longer uploadable"));
            }
            if f.offset != offset {
                return Err(Error(
                    409,
                    format!(
                        "Upload offset is {}; refresh the draft and resume",
                        f.offset
                    ),
                ));
            }
            if offset.saturating_add(data.len() as u64) > f.size {
                return Err(bad("Chunk exceeds declared file size"));
            }
            let mut out = OpenOptions::new().write(true).open(self.file_path(&f))?;
            out.set_len(offset)?;
            out.seek(SeekFrom::Start(offset))?;
            out.write_all(data)?;
            out.sync_data()?;
            f.offset += data.len() as u64;
            put(db, "file", id, &f)?;
            Ok(f)
        })
    }
    pub fn publish(&self, a: &Identity, id: &str) -> Result<ShareView> {
        let (mut s, mut files) = self.authenticated_db(a, |db| {
            let mut share: Share = get(db, "share", id)?.ok_or_else(unavailable)?;
            self.owns(a, &share)?;
            if share.state != "draft" || share.created + DAY <= self.now() {
                return Err(bad("Draft is no longer publishable"));
            }
            let settings: Settings = get(db, "settings", "instance")?.unwrap_or_default();
            self.check_limits(&settings, share.days, share.limit)?;
            let files = self.files_in(db, id)?;
            if files.iter().any(|f| f.offset != f.size) {
                return Err(bad("Finish uploading every file before publishing"));
            }
            share.state = "preparing".into();
            put(db, "share", id, &share)?;
            Ok((share, files))
        })?;
        // Immutable files can be hashed and archived without holding SQLite's write lock.
        let result = (|| {
            for f in &mut files {
                let mut input = File::open(self.file_path(f))?;
                if input.metadata()?.len() != f.size {
                    return Err(bad(
                        "Stored file length differs from upload; resume the draft",
                    ));
                }
                let mut hash = Sha256::new();
                let mut buf = [0u8; 65536];
                loop {
                    let n = input.read(&mut buf)?;
                    if n == 0 {
                        break;
                    }
                    hash.update(&buf[..n]);
                }
                f.sha256 = Some(format!("{:x}", hash.finalize()));
            }
            if files.len() > 1 {
                let temp = self.root.join(id).join("archive.tmp");
                let out = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(&temp)?;
                let mut zip = zip::ZipWriter::new(out);
                for f in &files {
                    let opts = zip::write::SimpleFileOptions::default()
                        .compression_method(zip::CompressionMethod::Stored)
                        .large_file(f.size > u32::MAX as u64);
                    zip.start_file(&f.name, opts)
                        .map_err(|e| Error(500, format!("ZIP creation failed: {e}")))?;
                    std::io::copy(&mut File::open(self.file_path(f))?, &mut zip)?;
                }
                let mut out = zip
                    .finish()
                    .map_err(|e| Error(500, format!("ZIP finalization failed: {e}")))?;
                out.sync_all()?;
                s.zip_size = out.metadata()?.len();
                if s.original_size + s.zip_size > s.reserved {
                    return Err(Error(507, "ZIP exceeded reserved capacity".into()));
                }
                out.seek(SeekFrom::Start(0))?;
                let mut h = Sha256::new();
                let mut buf = [0u8; 65536];
                loop {
                    let n = out.read(&mut buf)?;
                    if n == 0 {
                        break;
                    }
                    h.update(&buf[..n]);
                }
                s.zip_hash = Some(format!("{:x}", h.finalize()));
                fs::rename(temp, self.zip_path(&s))?;
                crate::storage::files::sync_directory(&self.root.join(id))?;
            }

            self.authenticated_db(a, |db| {
                let current: Share = get(db, "share", id)?.ok_or_else(unavailable)?;
                if current.state != "preparing" {
                    return Err(unavailable());
                }
                let settings: Settings = get(db, "settings", "instance")?.unwrap_or_default();
                self.check_limits(&settings, s.days, s.limit)?;
                for file in &files {
                    put(db, "file", &file.id, file)?;
                }
                s.state = "active".into();
                s.published = Some(self.now());
                s.expires = s.days.map(|days| self.now() + i64::from(days) * DAY);
                s.reserved = s.original_size + s.zip_size;
                put(db, "share", id, &s)?;
                self.view_in(db, &s)
            })
        })();
        if result.is_err() {
            // Keep a failed draft resumable; revocation during preparation stays final.
            self.db(|db| {
                if let Some(mut current) = get::<Share>(db, "share", id)?
                    && current.state == "preparing"
                {
                    for name in ["archive.tmp", "archive.zip"] {
                        let path = self.root.join(id).join(name);
                        if path.exists() {
                            fs::remove_file(path)?;
                        }
                    }
                    current.state = "draft".into();
                    put(db, "share", id, &current)?;
                }
                Ok(())
            })?;
        }
        result
    }
}
