use super::support::Fixture;
use crate::{App, crypto::secret, models::*, storage::database::*};
use std::sync::{Arc, Barrier, atomic::Ordering};

fn draft(f: &Fixture, multi: bool) -> ShareView {
    let mut files = vec![NewFile {
        name: "hello.txt".into(),
        size: 5,
    }];
    if multi {
        files.push(NewFile {
            name: "empty.txt".into(),
            size: 0,
        });
    }
    f.app
        .create_share(
            &f.actor,
            NewShare {
                title: "Private handover".into(),
                files,
                days: Some(7),
                limit: None,
                password: Some("share-password".into()),
                delete_after_download: true,
            },
        )
        .unwrap()
}
fn published(f: &Fixture, multi: bool) -> ShareView {
    let s = draft(f, multi);
    f.app.upload(&f.actor, &s.files[0].id, 0, b"hello").unwrap();
    f.app
        .publish(&f.actor, s.share["id"].as_str().unwrap())
        .unwrap()
}
fn token(f: &Fixture, s: &ShareView, resource: &str) -> String {
    f.app
        .new_transfer(
            s.share["id"].as_str().unwrap(),
            resource,
            Some("share-password"),
            &secret(),
            None,
        )
        .unwrap()
        .0
}
fn assert_gone(f: &Fixture, s: &ShareView) {
    let id = s.share["id"].as_str().unwrap();
    assert_eq!(f.app.inspect(&f.actor, id).unwrap_err().0, 404);
    assert_eq!(f.app.public_share(id, None).unwrap_err().0, 404);
    assert!(!f.app.root.join(id).exists());
    assert!(
        !f.app
            .list_shares(&f.actor)
            .unwrap()
            .iter()
            .any(|s| s.share["id"] == id)
    );
    f.app
        .db(|db| {
            for kind in [
                "share",
                "file",
                "removed_file",
                "transfer",
                "download_reservation",
            ] {
                assert!(
                    all::<serde_json::Value>(db, kind)?.is_empty(),
                    "Remaining {kind}"
                );
            }
            Ok(())
        })
        .unwrap();
}

#[test]
fn completed_delivery_purges_every_record_and_only_notifies_live_observers() {
    let f = Fixture::new();
    let s = published(&f, false);
    let raw = token(&f, &s, &s.files[0].id);
    let observer = f.app.observe_download(&raw).unwrap();
    let other = token(&f, &s, &s.files[0].id);
    let competing = f.app.observe_download(&other).unwrap();
    let owner = f
        .app
        .observe_share_download(&f.actor, s.share["id"].as_str().unwrap())
        .unwrap();
    assert_eq!(observer.status().unwrap(), "available");
    f.app.download(&raw, false).unwrap();
    assert_eq!(observer.status().unwrap(), "available");
    let lease = f.app.begin_download(&raw).unwrap().unwrap();
    assert_eq!(observer.status().unwrap(), "downloading");
    assert_eq!(competing.status().unwrap(), "blocked");
    lease.complete(0, 5, 5).unwrap();
    assert_eq!(observer.outcome(), 1);
    assert_eq!(competing.outcome(), 2);
    assert_eq!(owner.outcome(), 1);
    assert_gone(&f, &s);
    assert_eq!(f.app.download(&raw, false).unwrap_err().0, 404);
    drop(observer);
    assert!(f.app.download_observers.lock().unwrap().is_empty());
}

#[test]
fn incomplete_delivery_resumes_after_restart_and_competitors_stay_blocked() {
    let f = Fixture::new();
    let s = published(&f, false);
    let raw = token(&f, &s, &s.files[0].id);
    let other = token(&f, &s, &s.files[0].id);
    let lease = f.app.begin_download(&raw).unwrap().unwrap();
    lease.checkpoint(0, 2).unwrap();
    assert_eq!(f.app.begin_download(&raw).err().unwrap().0, 409);
    assert_eq!(f.app.begin_download(&other).err().unwrap().0, 409);
    drop(lease);
    let app = App::open(&f.app.root, f.app.public_url.clone()).unwrap();
    app.clock.store(f.app.now(), Ordering::Relaxed);
    assert_eq!(app.begin_download(&other).err().unwrap().0, 409);
    let resumed = app.begin_download(&raw).unwrap().unwrap();
    resumed.complete(2, 5, 5).unwrap();
    assert_gone(&f, &s);
}

#[test]
fn expired_reservations_release_and_never_reuse_another_recipients_ranges() {
    let f = Fixture::new();
    let s = published(&f, false);
    let raw = token(&f, &s, &s.files[0].id);
    let other = token(&f, &s, &s.files[0].id);
    let lease = f.app.begin_download(&raw).unwrap().unwrap();
    lease.checkpoint(0, 2).unwrap();
    drop(lease);
    f.advance(899);
    f.app.download(&raw, false).unwrap(); // HEAD must not extend the reservation.
    assert_eq!(f.app.begin_download(&other).err().unwrap().0, 409);
    f.advance(1);
    assert_eq!(f.app.download(&raw, false).unwrap_err().0, 404);
    let lease = f.app.begin_download(&other).unwrap().unwrap();
    lease.complete(2, 5, 5).unwrap();
    drop(lease);
    assert!(
        f.app
            .inspect(&f.actor, s.share["id"].as_str().unwrap())
            .is_ok()
    );
    assert_eq!(f.app.begin_download(&raw).err().unwrap().0, 409);
    let lease = f.app.begin_download(&other).unwrap().unwrap();
    lease.complete(0, 2, 5).unwrap();
    assert_gone(&f, &s);
}

#[test]
fn reservation_is_exclusive_even_for_the_same_capability_and_long_running_streams() {
    let f = Fixture::new();
    let s = published(&f, false);
    let raw = token(&f, &s, &s.files[0].id);
    let barrier = Arc::new(Barrier::new(8));
    let results: Vec<_> = (0..8)
        .map(|_| {
            let app = f.app.clone();
            let raw = raw.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                let lease = app.begin_download(&raw).ok().flatten();
                barrier.wait();
                lease.is_some()
            })
        })
        .collect();
    assert_eq!(
        results
            .into_iter()
            .map(|t| u32::from(t.join().unwrap()))
            .sum::<u32>(),
        1
    );
    let lease = f.app.begin_download(&raw).unwrap().unwrap();
    f.advance(901);
    let other = token(&f, &s, &s.files[0].id);
    assert_eq!(f.app.begin_download(&other).err().unwrap().0, 409);
    lease.checkpoint(0, 0).unwrap();
}

#[test]
fn bundles_are_mandatory_and_management_downloads_never_delete() {
    let f = Fixture::new();
    let s = published(&f, true);
    let id = s.share["id"].as_str().unwrap();
    assert_eq!(
        f.app
            .new_transfer(id, &s.files[0].id, Some("share-password"), &secret(), None)
            .unwrap_err()
            .0,
        400
    );
    let raw = f
        .app
        .new_transfer(id, &s.files[0].id, None, &secret(), Some(&f.actor))
        .unwrap()
        .0;
    assert!(f.app.begin_download(&raw).unwrap().is_none());
    assert_eq!(f.app.inspect(&f.actor, id).unwrap().share["used"], 0);
    let zip = token(&f, &s, "zip");
    let size = s.share["zip_size"].as_u64().unwrap();
    f.app
        .begin_download(&zip)
        .unwrap()
        .unwrap()
        .complete(0, size, size)
        .unwrap();
    assert_gone(&f, &s);
}

#[test]
fn draft_policy_can_change_but_publication_locks_it() {
    let f = Fixture::new();
    let s = draft(&f, false);
    let id = s.share["id"].as_str().unwrap();
    let edit = |delete| EditShare {
        title: "Updated draft".into(),
        expires: None,
        days: Some(2),
        limit: None,
        password: None,
        change_password: false,
        delete_after_download: Some(delete),
    };
    assert_eq!(
        f.app.edit(&f.actor, id, edit(false)).unwrap().share["delete_after_download"],
        false
    );
    assert_eq!(
        f.app.edit(&f.actor, id, edit(true)).unwrap().share["days"],
        2
    );
    f.app.upload(&f.actor, &s.files[0].id, 0, b"hello").unwrap();
    f.app.publish(&f.actor, id).unwrap();
    assert_eq!(f.app.edit(&f.actor, id, edit(false)).unwrap_err().0, 400);
    assert!(f.app.edit(&f.actor, id, edit(true)).is_ok());
}

#[test]
fn expiration_and_manual_deletion_leave_no_history_or_capabilities() {
    for expire in [true, false] {
        let f = Fixture::new();
        let s = published(&f, false);
        let id = s.share["id"].as_str().unwrap();
        let raw = token(&f, &s, &s.files[0].id);
        let observer = f.app.observe_download(&raw).unwrap();
        if expire {
            f.advance(7 * crate::DAY);
            f.app.cleanup().unwrap();
        } else {
            f.app.revoke(&f.actor, id).unwrap();
        }
        assert_eq!(observer.outcome(), 2);
        assert_gone(&f, &s);
    }
}

#[test]
fn cleanup_failure_cannot_reopen_a_completed_share() {
    let f = Fixture::new();
    let s = published(&f, false);
    let id = s.share["id"].as_str().unwrap();
    let raw = token(&f, &s, &s.files[0].id);
    let observer = f.app.observe_download(&raw).unwrap();
    let lease = f.app.begin_download(&raw).unwrap().unwrap();
    let path = f.app.root.join(id);
    let held = f.app.root.join("held");
    std::fs::rename(&path, &held).unwrap();
    std::fs::write(&path, b"block cleanup").unwrap();
    lease.complete(0, 5, 5).unwrap();
    assert_eq!(f.app.download(&raw, false).unwrap_err().0, 404);
    assert_eq!(f.app.inspect(&f.actor, id).unwrap_err().0, 404);
    assert!(f.app.list_shares(&f.actor).unwrap().is_empty());
    assert_eq!(observer.outcome(), 0);
    assert_eq!(observer.status().unwrap(), "deleting");
    std::fs::remove_file(&path).unwrap();
    std::fs::rename(&held, &path).unwrap();
    f.app.cleanup().unwrap();
    assert_eq!(observer.outcome(), 1);
    assert_gone(&f, &s);
}

#[test]
fn completed_share_cleanup_is_independent_of_unrelated_filesystem_failures() {
    let f = Fixture::new();
    let unrelated = f.published(None, false);
    let unrelated_id = unrelated.share["id"].as_str().unwrap();
    let path = f.app.root.join(unrelated_id);
    std::fs::rename(&path, f.app.root.join("unrelated-held")).unwrap();
    std::fs::write(&path, b"prevent unrelated cleanup").unwrap();
    assert!(f.app.revoke(&f.actor, unrelated_id).is_err());
    let share = published(&f, false);
    let id = share.share["id"].as_str().unwrap();
    let raw = token(&f, &share, &share.files[0].id);
    let observer = f.app.observe_download(&raw).unwrap();
    f.app
        .begin_download(&raw)
        .unwrap()
        .unwrap()
        .complete(0, 2, 5)
        .unwrap();
    assert!(f.app.inspect(&f.actor, id).is_ok());
    f.app
        .begin_download(&raw)
        .unwrap()
        .unwrap()
        .complete(2, 5, 5)
        .unwrap();
    assert_eq!(observer.outcome(), 1);
    assert!(!f.app.root.join(id).exists());
    assert_eq!(f.app.inspect(&f.actor, id).unwrap_err().0, 404);
    assert!(
        path.is_file(),
        "Unrelated cleanup must not be retried by a completed download"
    );

    // If this share's initial purge also fails, maintenance must later retry
    // it independently, even while the unrelated directory remains broken.
    let retry = published(&f, false);
    let retry_id = retry.share["id"].as_str().unwrap();
    let retry_raw = token(&f, &retry, &retry.files[0].id);
    let retry_observer = f.app.observe_download(&retry_raw).unwrap();
    let retry_path = f.app.root.join(retry_id);
    let held = f.app.root.join("retry-held");
    std::fs::rename(&retry_path, &held).unwrap();
    std::fs::write(&retry_path, b"prevent initial cleanup").unwrap();
    f.app
        .begin_download(&retry_raw)
        .unwrap()
        .unwrap()
        .complete(0, 5, 5)
        .unwrap();
    assert_eq!(retry_observer.status().unwrap(), "deleting");
    std::fs::remove_file(&retry_path).unwrap();
    std::fs::rename(&held, &retry_path).unwrap();
    assert!(
        f.app.cleanup().is_err(),
        "The unrelated directory is still broken"
    );
    assert_eq!(retry_observer.outcome(), 1);
    assert!(!retry_path.exists());
}

#[test]
fn policy_replaces_allowance_and_respects_administrator_maxima_at_publication() {
    let f = Fixture::new();
    let mut settings = f.app.settings().unwrap();
    settings.max_downloads = Some(1);
    f.app
        .db(|db| put(db, "settings", "instance", &settings))
        .unwrap();
    let s = published(&f, false);
    assert_eq!(s.share["delete_after_download"], true);
    let n = NewShare {
        title: "Conflicting limits".into(),
        files: vec![NewFile {
            name: "a".into(),
            size: 0,
        }],
        days: Some(7),
        limit: Some(1),
        password: None,
        delete_after_download: true,
    };
    assert_eq!(f.app.create_share(&f.actor, n).unwrap_err().0, 400);
}

#[test]
fn password_change_invalidates_old_observers_and_releases_an_interrupted_reservation() {
    let f = Fixture::new();
    let s = published(&f, false);
    let id = s.share["id"].as_str().unwrap();
    let raw = token(&f, &s, &s.files[0].id);
    let observer = f.app.observe_download(&raw).unwrap();
    let lease = f.app.begin_download(&raw).unwrap().unwrap();
    lease.checkpoint(0, 2).unwrap();
    drop(lease);
    f.app
        .edit(
            &f.actor,
            id,
            EditShare {
                title: "New access".into(),
                expires: None,
                days: None,
                limit: None,
                password: None,
                change_password: true,
                delete_after_download: None,
            },
        )
        .unwrap();
    assert_eq!(observer.status().unwrap(), "unavailable");
    assert_eq!(f.app.download(&raw, false).unwrap_err().0, 404);
    let raw = f
        .app
        .new_transfer(id, &s.files[0].id, None, &secret(), None)
        .unwrap()
        .0;
    f.app
        .begin_download(&raw)
        .unwrap()
        .unwrap()
        .complete(0, 5, 5)
        .unwrap();
    assert_gone(&f, &s);
}

#[test]
fn overlapping_ranges_do_not_count_missing_bytes_twice() {
    let f = Fixture::new();
    let s = published(&f, false);
    let id = s.share["id"].as_str().unwrap();
    let raw = token(&f, &s, &s.files[0].id);
    for (start, end) in [(1, 4), (2, 5), (1, 5)] {
        f.app
            .begin_download(&raw)
            .unwrap()
            .unwrap()
            .complete(start, end, 5)
            .unwrap();
        assert!(f.app.inspect(&f.actor, id).is_ok());
    }
    f.app
        .begin_download(&raw)
        .unwrap()
        .unwrap()
        .complete(0, 1, 5)
        .unwrap();
    assert_gone(&f, &s);
}
