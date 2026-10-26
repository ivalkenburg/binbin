use crate::{
    error::{Error, Result, bad},
    models::Settings,
    storage::database::{get, put},
};
use rusqlite::{Connection, Transaction};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicI64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone)]
pub struct App {
    pub(crate) db: Arc<Mutex<Connection>>,
    pub root: PathBuf,
    pub public_url: String,
    pub secure_cookie: bool,
    pub(crate) clock: Arc<AtomicI64>,
}

impl App {
    pub fn open(root: impl AsRef<Path>, public_url: String) -> Result<Self> {
        crate::storage::files::create_directory(root.as_ref())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(root.as_ref(), fs::Permissions::from_mode(0o700))?;
        }
        let db = Connection::open(root.as_ref().join("binbin.sqlite"))?;
        db.busy_timeout(std::time::Duration::from_secs(30))?;
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")?;
        db.execute_batch(include_str!("../migrations/0001_initial.sql"))?;
        if get::<Settings>(&db, "settings", "instance")?.is_none() {
            put(&db, "settings", "instance", &Settings::default())?;
        }
        let parsed = url::Url::parse(&public_url).map_err(|_| bad("Invalid public URL"))?;
        if !matches!(parsed.scheme(), "http" | "https")
            || parsed.host_str().is_none()
            || parsed.path() != "/"
            || parsed.query().is_some()
            || parsed.fragment().is_some()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
        {
            return Err(bad(
                "Public URL must be an HTTP(S) origin without a path or credentials",
            ));
        }
        let secure_cookie = parsed.scheme() == "https";
        Ok(Self {
            db: Arc::new(Mutex::new(db)),
            root: root.as_ref().to_owned(),
            public_url: parsed.origin().ascii_serialization(),
            secure_cookie,
            clock: Arc::new(AtomicI64::new(0)),
        })
    }
    pub fn now(&self) -> i64 {
        let t = self.clock.load(Ordering::Relaxed);
        if t != 0 {
            t
        } else {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64
        }
    }
    pub(crate) fn db<T>(&self, f: impl FnOnce(&Transaction<'_>) -> Result<T>) -> Result<T> {
        let mut db = self
            .db
            .lock()
            .map_err(|_| Error(500, "Database lock failed".into()))?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let value = f(&tx)?;
        tx.commit()?;
        Ok(value)
    }
    pub(crate) fn authenticated_db<T>(
        &self,
        actor: &crate::models::Identity,
        action: impl FnOnce(&Transaction<'_>) -> Result<T>,
    ) -> Result<T> {
        self.db(|db| {
            self.validate_identity_in(db, actor)?;
            action(db)
        })
    }
    pub fn settings(&self) -> Result<Settings> {
        self.db(|db| Ok(get(db, "settings", "instance")?.unwrap_or_default()))
    }
    pub fn backup_database(&self, destination: &Path) -> Result<()> {
        if destination.exists() {
            return Err(bad("Backup destination already exists"));
        }
        let db = self
            .db
            .lock()
            .map_err(|_| Error(500, "Database lock failed".into()))?;
        db.execute("VACUUM INTO ?", [destination.to_string_lossy().as_ref()])?;
        Ok(())
    }
}
