use super::support::Fixture;
use crate::{models::*, storage::database::*};

#[test]
fn reservations_include_drafts_and_zip_and_do_not_evict() {
    let f = Fixture::new();
    let mut settings = f.app.settings().unwrap();
    settings.storage_limit = 15;
    f.app
        .db(|db| put(db, "settings", "instance", &settings))
        .unwrap();
    let s = f.draft(None, false);
    let mut n = NewShare {
        title: "Too large".into(),
        files: vec![
            NewFile {
                name: "a".into(),
                size: 6,
            },
            NewFile {
                name: "b".into(),
                size: 5,
            },
        ],
        days: Some(7),
        limit: None,
        password: None,
    };
    assert_eq!(f.app.create_share(&f.actor, n.clone()).unwrap_err().0, 507);
    n.files = vec![NewFile {
        name: "a".into(),
        size: 11,
    }];
    assert_eq!(f.app.create_share(&f.actor, n).unwrap_err().0, 507);
    assert!(f.app.file_path(&s.files[0]).exists());
}
#[test]
fn concurrent_reservations_cannot_overcommit() {
    let f = Fixture::new();
    let mut settings = f.app.settings().unwrap();
    settings.storage_limit = 5;
    f.app
        .db(|db| put(db, "settings", "instance", &settings))
        .unwrap();
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let app = f.app.clone();
            let actor = f.actor.clone();
            std::thread::spawn(move || {
                app.create_share(
                    &actor,
                    NewShare {
                        title: "test".into(),
                        files: vec![NewFile {
                            name: "a".into(),
                            size: 5,
                        }],
                        days: None,
                        limit: None,
                        password: None,
                    },
                )
                .is_ok()
            })
        })
        .collect();
    assert_eq!(
        handles
            .into_iter()
            .map(|h| u32::from(h.join().unwrap()))
            .sum::<u32>(),
        1
    );
}
#[test]
fn unsafe_and_duplicate_filenames_rejected() {
    let f = Fixture::new();
    for name in ["../a", "a/b", "a\\b", ".", "..", "a\n", "C:file"] {
        assert!(
            f.app
                .create_share(
                    &f.actor,
                    NewShare {
                        title: "bad".into(),
                        files: vec![NewFile {
                            name: name.into(),
                            size: 0
                        }],
                        days: None,
                        limit: None,
                        password: None
                    }
                )
                .is_err()
        );
    }
}
