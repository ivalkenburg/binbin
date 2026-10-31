use crate::{
    App, DAY,
    crypto::{digest, hash_password, secret, verify_password},
    error::{Error, Result, bad, unavailable},
    models::{Credential, Identity, User},
    storage::database::{all, get, put, remove},
};
use rusqlite::Connection;

impl App {
    pub fn create_admin(&self, username: &str, password: &str) -> Result<User> {
        let hash = hash_password(password)?;
        self.db(|db| {
            if !all::<User>(db, "user")?.is_empty() {
                return Err(bad(
                    "An administrator already exists; use the dashboard to create accounts",
                ));
            }
            self.create_user_in(db, username, hash, true)
        })
    }
    pub(crate) fn create_user_in(
        &self,
        db: &Connection,
        username: &str,
        password: String,
        admin: bool,
    ) -> Result<User> {
        if username.is_empty()
            || username.len() > 64
            || !username
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        {
            return Err(bad(
                "Username must use 1–64 letters, digits, periods, hyphens, or underscores",
            ));
        }
        if all::<User>(db, "user")?
            .iter()
            .any(|u| u.username.eq_ignore_ascii_case(username))
        {
            return Err(Error(409, "Username already exists".into()));
        }
        let u = User {
            id: secret(),
            username: username.into(),
            password,
            admin,
            enabled: true,
        };
        put(db, "user", &u.id, &u)?;
        Ok(u)
    }
    pub fn create_user(
        &self,
        actor: &Identity,
        username: &str,
        password: &str,
        admin: bool,
    ) -> Result<User> {
        self.require_admin(actor)?;
        let hash = hash_password(password)?;
        self.authenticated_db(actor, |db| self.create_user_in(db, username, hash, admin))
    }
    pub fn require_admin(&self, actor: &Identity) -> Result<()> {
        if actor.user.admin {
            Ok(())
        } else {
            Err(Error(403, "Administrator access required".into()))
        }
    }
    pub fn login(&self, username: &str, password: &str) -> Result<(String, Identity)> {
        let u = self.db(|db| {
            Ok(all::<User>(db, "user")?
                .into_iter()
                .find(|u| u.username.eq_ignore_ascii_case(username)))
        })?;
        // Run the same expensive password operation for unknown names.
        let dummy = "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHRzb21lc2FsdA$uF0h1UBY0Z2GQfULeWdTaWzGXrHCWw5Z29kPQhO1W4Q";
        let valid = verify_password(password, u.as_ref().map_or(dummy, |u| &u.password));
        let u = u
            .filter(|u| u.enabled && valid)
            .ok_or_else(|| Error(401, "Invalid username or password".into()))?;
        self.issue(&u)
    }
    fn issue_in(&self, db: &Connection, user: &User) -> Result<(String, Identity)> {
        let raw = secret();
        let credential = Credential {
            id: digest(&raw),
            user: user.id.clone(),
            csrf: secret(),
            expires: self.now() + 7 * DAY,
        };
        put(db, "credential", &credential.id, &credential)?;
        Ok((
            raw,
            Identity {
                user: user.clone(),
                credential,
            },
        ))
    }
    pub fn issue(&self, snapshot: &User) -> Result<(String, Identity)> {
        self.db(|db| {
            let current: User = get(db, "user", &snapshot.id)?
                .ok_or_else(|| Error(401, "Sign in required".into()))?;
            if !current.enabled || current.password != snapshot.password {
                return Err(Error(401, "Authentication changed; sign in again".into()));
            }
            self.issue_in(db, &current)
        })
    }
    pub(crate) fn credential_valid_in(&self, db: &Connection, id: &str) -> Result<bool> {
        let Some(credential) = get::<Credential>(db, "credential", id)? else {
            return Ok(false);
        };
        let user: Option<User> = get(db, "user", &credential.user)?;
        Ok(credential.expires > self.now() && user.is_some_and(|user| user.enabled))
    }
    pub(crate) fn validate_identity_in(&self, db: &Connection, actor: &Identity) -> Result<()> {
        let credential: Option<Credential> = get(db, "credential", &actor.credential.id)?;
        let user: Option<User> = get(db, "user", &actor.user.id)?;
        if credential.is_none_or(|credential| {
            credential.user != actor.user.id || credential.expires <= self.now()
        }) || user.is_none_or(|user| !user.enabled || user.admin != actor.user.admin)
        {
            return Err(Error(401, "Sign in required".into()));
        }
        Ok(())
    }
    pub fn authenticate(&self, raw: &str, csrf: Option<&str>, mutation: bool) -> Result<Identity> {
        self.db(|db| {
            let c: Credential = get(db, "credential", &digest(raw))?
                .ok_or_else(|| Error(401, "Sign in required".into()))?;
            let u: User =
                get(db, "user", &c.user)?.ok_or_else(|| Error(401, "Sign in required".into()))?;
            if !u.enabled || c.expires <= self.now() {
                return Err(Error(401, "Sign in required".into()));
            }
            if mutation && csrf != Some(&c.csrf) {
                return Err(Error(
                    403,
                    "Refresh the page and retry; request verification failed".into(),
                ));
            }
            Ok(Identity {
                user: u,
                credential: c,
            })
        })
    }
    pub fn change_user(
        &self,
        actor: &Identity,
        id: &str,
        password: Option<&str>,
        enabled: bool,
    ) -> Result<()> {
        self.require_admin(actor)?;
        let hash = password.map(hash_password).transpose()?;
        self.authenticated_db(actor, |db| {
            let mut u: User = get(db, "user", id)?.ok_or_else(unavailable)?;
            if u.id == actor.user.id && !enabled {
                return Err(bad("You cannot disable your own administrator account"));
            }
            if let Some(h) = hash {
                u.password = h;
            }
            u.enabled = enabled;
            put(db, "user", id, &u)?;
            for c in all::<Credential>(db, "credential")?
                .into_iter()
                .filter(|c| c.user == id)
            {
                remove(db, "credential", &c.id)?;
            }
            Ok(())
        })
    }
    pub fn logout(&self, actor: &Identity) -> Result<()> {
        self.authenticated_db(actor, |db| remove(db, "credential", &actor.credential.id))
    }
    pub fn users(&self, actor: &Identity) -> Result<Vec<User>> {
        self.require_admin(actor)?;
        self.authenticated_db(actor, |db| all(db, "user"))
    }
}
