use super::support::Fixture;
use crate::{DAY, models::*, storage::database::*};
use std::io::Read;

#[test]
fn draft_edits_preserve_progress_and_undo_until_publication() {
    let f = Fixture::new();
    let draft = f.draft(None, false);
    let id = draft.share["id"].as_str().unwrap();
    let original = &draft.files[0];
    f.app.upload(&f.actor, &original.id, 0, b"he").unwrap();
    let edited = f
        .app
        .edit_files(
            &f.actor,
            id,
            EditFiles {
                add: vec![NewFile {
                    name: "other.txt".into(),
                    size: 0,
                }],
                remove: vec![original.id.clone()],
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(edited.files.len(), 1);
    assert_eq!(edited.removed_files[0].file.offset, 2);
    assert!(f.app.upload(&f.actor, &original.id, 2, b"llo").is_err());
    let restored = f
        .app
        .edit_files(
            &f.actor,
            id,
            EditFiles {
                restore: vec![original.id.clone()],
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(
        restored
            .files
            .iter()
            .find(|file| file.id == original.id)
            .unwrap()
            .offset,
        2
    );
    assert!(f.app.publish(&f.actor, id).is_err());
    f.app.upload(&f.actor, &original.id, 2, b"llo").unwrap();
    f.app
        .edit_files(
            &f.actor,
            id,
            EditFiles {
                remove: vec![original.id.clone()],
                ..Default::default()
            },
        )
        .unwrap();
    let published = f.app.publish(&f.actor, id).unwrap();
    assert_eq!(published.files.len(), 1);
    assert!(published.removed_files.is_empty());
    assert!(!f.app.file_path(original).exists());
    assert!(
        f.app
            .edit_files(&f.actor, id, EditFiles::default())
            .is_err()
    );
}

#[test]
fn failed_edits_are_atomic_and_removal_reserves_bytes_until_undo_ends() {
    let f = Fixture::new();
    let draft = f.draft(None, false);
    let id = draft.share["id"].as_str().unwrap();
    let mut settings = f.app.settings().unwrap();
    settings.storage_limit = 6;
    f.app
        .db(|db| put(db, "settings", "instance", &settings))
        .unwrap();
    assert_eq!(
        f.app
            .edit_files(
                &f.actor,
                id,
                EditFiles {
                    add: vec![NewFile {
                        name: "too-large".into(),
                        size: 7
                    }],
                    remove: vec![draft.files[0].id.clone()],
                    ..Default::default()
                }
            )
            .unwrap_err()
            .0,
        507
    );
    assert_eq!(
        f.app.inspect(&f.actor, id).unwrap().files[0].id,
        draft.files[0].id
    );
    let empty = f
        .app
        .edit_files(
            &f.actor,
            id,
            EditFiles {
                remove: vec![draft.files[0].id.clone()],
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(empty.share["reserved"], 5);
    assert!(f.app.publish(&f.actor, id).is_err());
    f.advance(8);
    f.app.cleanup().unwrap();
    assert_eq!(f.app.inspect(&f.actor, id).unwrap().share["reserved"], 0);
    assert!(!f.app.file_path(&draft.files[0]).exists());
    assert!(
        f.app
            .edit_files(
                &f.actor,
                id,
                EditFiles {
                    restore: vec![draft.files[0].id.clone()],
                    ..Default::default()
                }
            )
            .is_err()
    );
}

#[test]
fn draft_edits_validate_names_ownership_and_expiry() {
    let f = Fixture::new();
    let draft = f.draft(None, false);
    let id = draft.share["id"].as_str().unwrap();
    for name in ["HELLO.TXT", "../x", "bad:name", "bad\nname"] {
        assert!(
            f.app
                .edit_files(
                    &f.actor,
                    id,
                    EditFiles {
                        add: vec![NewFile {
                            name: name.into(),
                            size: 0
                        }],
                        ..Default::default()
                    }
                )
                .is_err()
        );
    }
    let other = f
        .app
        .create_user(&f.actor, "other", "another-long-password", false)
        .unwrap();
    let (_, actor) = f.app.issue(&other, "other", false).unwrap();
    assert_eq!(
        f.app
            .edit_files(&actor, id, EditFiles::default())
            .unwrap_err()
            .0,
        404
    );
    let foreign = f.draft(None, false);
    assert!(
        f.app
            .edit_files(
                &f.actor,
                id,
                EditFiles {
                    remove: vec![foreign.files[0].id.clone()],
                    ..Default::default()
                }
            )
            .is_err()
    );
    f.advance(DAY);
    assert!(
        f.app
            .edit_files(&f.actor, id, EditFiles::default())
            .is_err()
    );
}

#[test]
fn previews_require_complete_authorized_images_and_do_not_charge_downloads() {
    let f = Fixture::new();
    let png = b"\x89PNG\r\n\x1a\npreview";
    let draft = f
        .app
        .create_share(
            &f.actor,
            NewShare {
                title: "Images".into(),
                files: vec![NewFile {
                    name: "image.png".into(),
                    size: png.len() as u64,
                }],
                days: Some(7),
                limit: Some(1),
                password: None,
            },
        )
        .unwrap();
    let file = &draft.files[0];
    assert!(f.app.image_preview(&f.actor, &file.id).is_err());
    f.app.upload(&f.actor, &file.id, 0, png).unwrap();
    let (mut input, mime, size) = f.app.image_preview(&f.actor, &file.id).unwrap();
    assert_eq!(mime, "image/png");
    assert_eq!(size, png.len() as u64);
    let mut body = Vec::new();
    input.read_to_end(&mut body).unwrap();
    assert_eq!(body, png);
    let other = f
        .app
        .create_user(&f.actor, "other", "another-long-password", false)
        .unwrap();
    let (_, actor) = f.app.issue(&other, "other", false).unwrap();
    assert_eq!(f.app.image_preview(&actor, &file.id).unwrap_err().0, 404);
    let id = draft.share["id"].as_str().unwrap();
    f.app.publish(&f.actor, id).unwrap();
    f.app.image_preview(&f.actor, &file.id).unwrap();
    assert_eq!(f.app.inspect(&f.actor, id).unwrap().share["used"], 0);
    let text = f.published(None, false);
    assert_eq!(
        f.app
            .image_preview(&f.actor, &text.files[0].id)
            .unwrap_err()
            .0,
        415
    );
}

#[test]
fn concurrent_draft_additions_cannot_overcommit_storage() {
    let f = Fixture::new();
    let draft = f.draft(None, false);
    let mut settings = f.app.settings().unwrap();
    settings.storage_limit = 4000;
    f.app
        .db(|db| put(db, "settings", "instance", &settings))
        .unwrap();
    let handles: Vec<_> = (0..6)
        .map(|index| {
            let app = f.app.clone();
            let actor = f.actor.clone();
            let id = draft.share["id"].as_str().unwrap().to_owned();
            std::thread::spawn(move || {
                app.edit_files(
                    &actor,
                    &id,
                    EditFiles {
                        add: vec![NewFile {
                            name: format!("{index}.txt"),
                            size: 400,
                        }],
                        ..Default::default()
                    },
                )
                .is_ok()
            })
        })
        .collect();
    assert_eq!(
        handles
            .into_iter()
            .filter_map(|handle| handle.join().unwrap().then_some(1))
            .sum::<u32>(),
        1
    );
}

#[test]
fn failed_or_interrupted_additions_do_not_leave_orphan_files() {
    let f = Fixture::new();
    let draft = f.draft(None, false);
    let id = draft.share["id"].as_str().unwrap();
    f.app
        .db(|db| {
            db.execute_batch(
                "CREATE TEMP TRIGGER fail_added_file BEFORE INSERT ON objects
            WHEN NEW.kind='file' AND json_extract(NEW.data,'$.name')='fail.txt'
            BEGIN SELECT RAISE(FAIL, 'injected insertion failure'); END;",
            )?;
            Ok(())
        })
        .unwrap();
    assert!(
        f.app
            .edit_files(
                &f.actor,
                id,
                EditFiles {
                    add: vec![NewFile {
                        name: "fail.txt".into(),
                        size: 0
                    }],
                    ..Default::default()
                }
            )
            .is_err()
    );
    assert_eq!(std::fs::read_dir(f.app.root.join(id)).unwrap().count(), 1);
    let orphan = f.app.root.join(id).join(crate::crypto::secret());
    std::fs::write(&orphan, b"").unwrap();
    f.app.cleanup().unwrap();
    assert!(!orphan.exists());
    assert!(f.app.file_path(&draft.files[0]).exists());
}

#[test]
fn revocation_removes_files_still_held_for_undo() {
    let f = Fixture::new();
    let draft = f.draft(None, false);
    let id = draft.share["id"].as_str().unwrap();
    f.app
        .edit_files(
            &f.actor,
            id,
            EditFiles {
                remove: vec![draft.files[0].id.clone()],
                ..Default::default()
            },
        )
        .unwrap();
    f.app.revoke(&f.actor, id).unwrap();
    assert!(
        f.app
            .inspect(&f.actor, id)
            .unwrap()
            .removed_files
            .is_empty()
    );
    assert!(!f.app.file_path(&draft.files[0]).exists());
    assert!(
        f.app
            .db(|db| all::<RemovedFile>(db, "removed_file"))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn failed_publication_transaction_keeps_undo_bytes_intact() {
    let f = Fixture::new();
    let draft = f.draft(None, true);
    let id = draft.share["id"].as_str().unwrap();
    let file = &draft.files[0];
    f.app.upload(&f.actor, &file.id, 0, b"hello").unwrap();
    f.app
        .edit_files(
            &f.actor,
            id,
            EditFiles {
                remove: vec![file.id.clone()],
                ..Default::default()
            },
        )
        .unwrap();
    f.app
        .db(|db| {
            db.execute_batch(
                "CREATE TEMP TRIGGER fail_preparing BEFORE INSERT ON objects
            WHEN NEW.kind='share' AND json_extract(NEW.data,'$.state')='preparing'
            BEGIN SELECT RAISE(FAIL, 'injected preparing failure'); END;",
            )?;
            Ok(())
        })
        .unwrap();
    assert!(f.app.publish(&f.actor, id).is_err());
    assert_eq!(std::fs::read(f.app.file_path(file)).unwrap(), b"hello");
    f.app
        .edit_files(
            &f.actor,
            id,
            EditFiles {
                restore: vec![file.id.clone()],
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(
        f.app
            .inspect(&f.actor, id)
            .unwrap()
            .files
            .iter()
            .find(|stored| stored.id == file.id)
            .unwrap()
            .offset,
        5
    );
}

#[test]
fn lowering_caps_does_not_prevent_removing_files_to_repair_a_draft() {
    let f = Fixture::new();
    let draft = f.draft(None, true);
    let id = draft.share["id"].as_str().unwrap();
    let mut settings = f.app.settings().unwrap();
    settings.storage_limit = 1;
    settings.share_limit = 1;
    settings.file_limit = 1;
    settings.file_count = 1;
    f.app
        .db(|db| put(db, "settings", "instance", &settings))
        .unwrap();
    let reduced = f
        .app
        .edit_files(
            &f.actor,
            id,
            EditFiles {
                remove: vec![draft.files[1].id.clone()],
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(reduced.files.len(), 1);
    let empty = f
        .app
        .edit_files(
            &f.actor,
            id,
            EditFiles {
                remove: vec![draft.files[0].id.clone()],
                ..Default::default()
            },
        )
        .unwrap();
    assert!(empty.files.is_empty());
}
