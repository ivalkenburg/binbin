use super::support::Fixture;
use crate::{App, DAY, crypto::*, models::*, storage::database::*};
use rusqlite::Connection;
use std::fs::{self};

#[test]
fn abandoned_drafts_and_history_are_cleaned() {
    let f = Fixture::new();
    let s = f.draft(None, false);
    let id = s.share["id"].as_str().unwrap();
    f.advance(DAY);
    f.app.cleanup().unwrap();
    assert!(!f.app.file_path(&s.files[0]).exists());
    assert_eq!(
        f.app.inspect(&f.actor, id).unwrap().share["state"],
        "abandoned"
    );
    f.advance(30 * DAY);
    f.app.cleanup().unwrap();
    assert!(f.app.inspect(&f.actor, id).is_err());
}
#[test]
fn database_backup_is_readable_and_never_overwrites_a_file() {
    let f = Fixture::new();
    let path = f._dir.path().join("snapshot.sqlite");
    f.app.backup_database(&path).unwrap();
    let connection = Connection::open(&path).unwrap();
    assert_eq!(all::<User>(&connection, "user").unwrap().len(), 1);
    assert!(f.app.backup_database(&path).is_err());
}

#[test]
fn empty_files_are_downloadable_and_their_directories_are_reclaimed() {
    let f = Fixture::new();
    let draft = f
        .app
        .create_share(
            &f.actor,
            NewShare {
                title: "Empty".into(),
                files: vec![NewFile {
                    name: "empty.txt".into(),
                    size: 0,
                }],
                days: Some(1),
                limit: None,
                password: None,
            },
        )
        .unwrap();
    let id = draft.share["id"].as_str().unwrap();
    let share = f.app.publish(&f.actor, id).unwrap();
    let token = f.transfer(&share, &share.files[0].id);
    let (_, size, _, hash) = f.app.download(&token, true).unwrap();
    assert_eq!(size, 0);
    assert_eq!(hash, digest([]));
    f.advance(DAY);
    f.app.cleanup().unwrap();
    assert!(!f.app.root.join(id).exists());
}

#[test]
fn public_origin_is_canonical_and_rejects_embedded_credentials() {
    let dir = tempfile::tempdir().unwrap();
    assert!(App::open(dir.path(), "https://:secret@example.com".into()).is_err());
    assert!(App::open(dir.path(), "https://example.com/files".into()).is_err());
    let app = App::open(dir.path(), "http://LOCALHOST:80/".into()).unwrap();
    assert_eq!(app.public_url, "http://localhost");
}

#[test]
fn orphan_cleanup_preserves_unrelated_directories() {
    let f = Fixture::new();
    let unrelated = f.app.root.join("operator-notes");
    fs::create_dir(&unrelated).unwrap();
    fs::write(unrelated.join("readme.txt"), b"keep me").unwrap();
    let orphan = f.app.root.join(secret());
    fs::create_dir(&orphan).unwrap();
    f.app.cleanup().unwrap();
    assert_eq!(fs::read(unrelated.join("readme.txt")).unwrap(), b"keep me");
    assert!(!orphan.exists());
}

#[test]
fn failed_draft_commit_leaves_no_records_and_orphan_files_are_reclaimed() {
    let fixture = Fixture::new();
    fixture
        .app
        .db(|db| {
            db.execute_batch(
                "CREATE TEMP TRIGGER fail_creation BEFORE INSERT ON objects
            WHEN NEW.kind='share' BEGIN SELECT RAISE(FAIL, 'injected draft commit failure'); END;",
            )?;
            Ok(())
        })
        .unwrap();
    assert!(
        fixture
            .app
            .create_share(
                &fixture.actor,
                NewShare {
                    title: "Interrupted creation".into(),
                    files: vec![NewFile {
                        name: "file.txt".into(),
                        size: 5
                    }],
                    days: None,
                    limit: None,
                    password: None,
                }
            )
            .is_err()
    );
    fixture
        .app
        .db(|db| {
            assert!(all::<Share>(db, "share")?.is_empty());
            assert!(all::<StoredFile>(db, "file")?.is_empty());
            Ok(())
        })
        .unwrap();
    fixture.app.cleanup().unwrap();
    assert!(
        fs::read_dir(&fixture.app.root).unwrap().all(|entry| !entry
            .unwrap()
            .file_type()
            .unwrap()
            .is_dir())
    );
}

#[test]
fn transfer_tombstones_are_reclaimed_with_obsolete_versions_and_deleted_shares() {
    let f = Fixture::new();
    let share = f.published(None, false);
    let id = share.share["id"].as_str().unwrap();
    let key = secret();
    let (old, old_transfer) = f
        .app
        .new_transfer(id, &share.files[0].id, None, &key, None)
        .unwrap();
    f.advance(DAY);
    f.app.cleanup().unwrap();
    f.app
        .db(|db| {
            assert!(get::<Transfer>(db, "transfer", &old_transfer.id)?.is_some());
            Ok(())
        })
        .unwrap();
    f.app
        .edit(
            &f.actor,
            id,
            EditShare {
                title: "New password".into(),
                expires: None,
                limit: None,
                password: Some("replacement-password".into()),
                change_password: true,
            },
        )
        .unwrap();
    f.app.cleanup().unwrap();
    f.app
        .db(|db| {
            assert!(get::<Transfer>(db, "transfer", &old_transfer.id)?.is_none());
            Ok(())
        })
        .unwrap();
    let (new, _) = f
        .app
        .new_transfer(
            id,
            &share.files[0].id,
            Some("replacement-password"),
            &key,
            None,
        )
        .unwrap();
    assert_ne!(old, new);
    assert_eq!(f.app.download(&old, true).unwrap_err().0, 404);
    f.app.download(&new, true).unwrap();
    f.advance(DAY);
    f.app.cleanup().unwrap();
    f.app.revoke(&f.actor, id).unwrap();
    f.app
        .db(|db| {
            assert!(all::<Transfer>(db, "transfer")?.is_empty());
            Ok(())
        })
        .unwrap();
    f.advance(30 * DAY);
    f.app.cleanup().unwrap();
    assert_eq!(f.app.inspect(&f.actor, id).unwrap_err().0, 404);
    assert_eq!(f.app.download(&new, true).unwrap_err().0, 404);
}

#[test]
fn cleanup_removes_transfers_whose_share_record_is_missing() {
    let f = Fixture::new();
    let share = f.published(None, false);
    f.transfer(&share, &share.files[0].id);
    f.app
        .db(|db| remove(db, "share", share.share["id"].as_str().unwrap()))
        .unwrap();
    f.app.cleanup().unwrap();
    f.app
        .db(|db| {
            assert!(all::<Transfer>(db, "transfer")?.is_empty());
            Ok(())
        })
        .unwrap();
}

#[test]
fn cleanup_retains_only_shares_with_eligible_charged_current_version_transfers() {
    let f = Fixture::new();
    let public_share = f.published(Some(1), false);
    let public = f.transfer(&public_share, &public_share.files[0].id);
    f.app.download(&public, true).unwrap();

    let management_share = f.published(Some(1), false);
    let (management, _) = f
        .app
        .new_transfer(
            management_share.share["id"].as_str().unwrap(),
            &management_share.files[0].id,
            None,
            &secret(),
            Some(&f.actor),
        )
        .unwrap();
    f.app.download(&management, true).unwrap();

    let stale_share = f.published(Some(2), false);
    let stale = f.transfer(&stale_share, &stale_share.files[0].id);
    f.app.download(&stale, true).unwrap();
    f.app
        .edit(
            &f.actor,
            stale_share.share["id"].as_str().unwrap(),
            EditShare {
                title: "New version".into(),
                expires: None,
                limit: Some(1),
                password: None,
                change_password: true,
            },
        )
        .unwrap();

    let uncharged_share = f.published(Some(1), false);
    let uncharged = f.transfer(&uncharged_share, &uncharged_share.files[0].id);
    let expired_share = f.published(Some(1), false);
    let expired = f.transfer(&expired_share, &expired_share.files[0].id);
    f.app.download(&expired, true).unwrap();
    let revoked_share = f.published(Some(1), false);
    let (_, revoked_actor) = f.app.create_token(&f.actor, "revoked manager").unwrap();
    let (revoked, _) = f
        .app
        .new_transfer(
            revoked_share.share["id"].as_str().unwrap(),
            &revoked_share.files[0].id,
            None,
            &secret(),
            Some(&revoked_actor),
        )
        .unwrap();
    f.app.download(&revoked, true).unwrap();
    f.app
        .revoke_token(&f.actor, &revoked_actor.credential.id)
        .unwrap();
    f.app
        .db(|db| {
            for share in [&management_share, &uncharged_share, &revoked_share] {
                let mut s: Share = get(db, "share", share.share["id"].as_str().unwrap())?.unwrap();
                s.used = 1;
                put(db, "share", &s.id, &s)?;
            }
            let mut t: Transfer = get(db, "transfer", &digest(&expired))?.unwrap();
            t.expires = f.app.now();
            put(db, "transfer", &t.id, &t)
        })
        .unwrap();

    f.app.cleanup().unwrap();
    for (share, token) in [(&public_share, &public), (&management_share, &management)] {
        assert!(f.app.file_path(&share.files[0]).exists());
        f.app.download(token, true).unwrap();
    }
    for (share, token) in [
        (&stale_share, &stale),
        (&uncharged_share, &uncharged),
        (&expired_share, &expired),
        (&revoked_share, &revoked),
    ] {
        assert!(!f.app.file_path(&share.files[0]).exists());
        assert_eq!(f.app.download(token, true).unwrap_err().0, 404);
    }
    f.app
        .db(|db| {
            assert_eq!(all::<Transfer>(db, "transfer")?.len(), 2);
            Ok(())
        })
        .unwrap();
    f.advance(DAY);
    f.app.cleanup().unwrap();
    assert!(!f.app.file_path(&public_share.files[0]).exists());
    assert!(!f.app.file_path(&management_share.files[0]).exists());
    f.app
        .db(|db| {
            assert!(all::<Transfer>(db, "transfer")?.is_empty());
            Ok(())
        })
        .unwrap();
}
