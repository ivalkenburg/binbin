use crate::{App, crypto::*, models::*};
use std::sync::atomic::Ordering;
pub(super) struct Fixture {
    pub(super) _dir: tempfile::TempDir,
    pub(super) app: App,
    pub(super) actor: Identity,
}
impl Fixture {
    pub(super) fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let app = App::open(dir.path(), "http://localhost:8080".into()).unwrap();
        app.clock.store(1_800_000_000, Ordering::Relaxed);
        let u = app.create_admin("admin", "correct-horse-battery").unwrap();
        let (_, actor) = app.issue(&u, "test", false).unwrap();
        Self {
            _dir: dir,
            app,
            actor,
        }
    }
    pub(super) fn draft(&self, limit: Option<u64>, multi: bool) -> ShareView {
        self.app
            .create_share(
                &self.actor,
                NewShare {
                    title: "Project files".into(),
                    files: if multi {
                        vec![
                            NewFile {
                                name: "hello.txt".into(),
                                size: 5,
                            },
                            NewFile {
                                name: "empty.txt".into(),
                                size: 0,
                            },
                        ]
                    } else {
                        vec![NewFile {
                            name: "hello.txt".into(),
                            size: 5,
                        }]
                    },
                    days: Some(7),
                    limit,
                    password: None,
                },
            )
            .unwrap()
    }
    pub(super) fn published(&self, limit: Option<u64>, multi: bool) -> ShareView {
        let s = self.draft(limit, multi);
        self.app
            .upload(&self.actor, &s.files[0].id, 0, b"hello")
            .unwrap();
        self.app
            .publish(&self.actor, s.share["id"].as_str().unwrap())
            .unwrap()
    }
    pub(super) fn transfer(&self, s: &ShareView, resource: &str) -> String {
        self.app
            .new_transfer(
                s.share["id"].as_str().unwrap(),
                resource,
                None,
                &secret(),
                None,
            )
            .unwrap()
            .0
    }
    pub(super) fn advance(&self, t: i64) {
        self.app.clock.fetch_add(t, Ordering::Relaxed);
    }
}
