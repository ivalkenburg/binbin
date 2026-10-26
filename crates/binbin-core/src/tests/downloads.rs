use super::support::Fixture;
use crate::{DAY, crypto::*, models::*, storage::database::*};
use std::sync::Arc;

#[test]
fn idempotency_does_not_consume_downloads() {
    let f = Fixture::new();
    let s = f.published(Some(1), false);
    let id = s.share["id"].as_str().unwrap();
    let key = secret();
    let first = f
        .app
        .new_transfer(id, &s.files[0].id, None, &key, None)
        .unwrap();
    let retry = f
        .app
        .new_transfer(id, &s.files[0].id, None, &key, None)
        .unwrap();
    assert_eq!(first.0, retry.0);
    assert_eq!(f.app.inspect(&f.actor, id).unwrap().share["used"], 0);
    f.app.download(&first.0, true).unwrap();
    f.app.download(&retry.0, true).unwrap();
    assert_eq!(f.app.inspect(&f.actor, id).unwrap().share["used"], 1);
}
#[test]
fn concurrent_last_allowance_is_atomic() {
    let f = Fixture::new();
    let s = f.published(Some(1), false);
    let tokens: Vec<_> = (0..12).map(|_| f.transfer(&s, &s.files[0].id)).collect();
    let barrier = Arc::new(std::sync::Barrier::new(tokens.len()));
    let handles: Vec<_> = tokens
        .into_iter()
        .map(|raw| {
            let app = f.app.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                app.download(&raw, true).is_ok()
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
fn exhausted_shares_retain_existing_transfers_then_reclaim() {
    let f = Fixture::new();
    let s = f.published(Some(1), false);
    let token = f.transfer(&s, &s.files[0].id);
    f.app.download(&token, true).unwrap();
    f.app.cleanup().unwrap();
    assert!(f.app.file_path(&s.files[0]).exists());
    assert!(f.app.download(&token, true).is_ok());
    assert_eq!(
        f.app
            .public_share(s.share["id"].as_str().unwrap(), None)
            .unwrap_err()
            .0,
        404
    );
    f.advance(DAY);
    f.app.cleanup().unwrap();
    assert!(!f.app.file_path(&s.files[0]).exists());
    assert!(f.app.download(&token, true).is_err());
}
#[test]
fn time_deadline_overrides_retry_credentials() {
    let f = Fixture::new();
    let s = f.published(None, false);
    let token = f.transfer(&s, &s.files[0].id);
    f.app.download(&token, true).unwrap();
    let id = s.share["id"].as_str().unwrap();
    f.app
        .edit(
            &f.actor,
            id,
            EditShare {
                title: "Project files".into(),
                expires: Some(f.app.now() + 10),
                limit: None,
                password: None,
                change_password: false,
            },
        )
        .unwrap();
    f.advance(10);
    assert!(f.app.download(&token, true).is_err());
    f.app.cleanup().unwrap();
    assert!(!f.app.file_path(&s.files[0]).exists());
}
#[test]
fn password_changes_invalidate_existing_transfers() {
    let f = Fixture::new();
    let s = f.published(None, false);
    let id = s.share["id"].as_str().unwrap();
    let token = f.transfer(&s, &s.files[0].id);
    f.app
        .edit(
            &f.actor,
            id,
            EditShare {
                title: "Project files".into(),
                expires: None,
                limit: None,
                password: Some("new-share-password".into()),
                change_password: true,
            },
        )
        .unwrap();
    assert!(f.app.download(&token, true).is_err());
    assert_eq!(f.app.public_share(id, None).unwrap_err().0, 401);
    assert!(f.app.public_share(id, Some("new-share-password")).is_ok());
    assert!(
        !f.app
            .inspect(&f.actor, id)
            .unwrap()
            .share
            .as_object()
            .unwrap()
            .contains_key("password")
    );
}
#[test]
fn management_downloads_bypass_allowance() {
    let f = Fixture::new();
    let s = f.published(Some(1), false);
    let (raw, _) = f
        .app
        .new_transfer(
            s.share["id"].as_str().unwrap(),
            &s.files[0].id,
            None,
            &secret(),
            Some(&f.actor),
        )
        .unwrap();
    f.app.download(&raw, true).unwrap();
    assert_eq!(
        f.app
            .inspect(&f.actor, s.share["id"].as_str().unwrap())
            .unwrap()
            .share["used"],
        0
    );
}
#[test]
fn revocation_blocks_retries_and_removes_contents() {
    let f = Fixture::new();
    let s = f.published(None, false);
    let token = f.transfer(&s, &s.files[0].id);
    f.app.download(&token, true).unwrap();
    f.app
        .revoke(&f.actor, s.share["id"].as_str().unwrap())
        .unwrap();
    assert!(f.app.download(&token, true).is_err());
    assert!(!f.app.file_path(&s.files[0]).exists());
}
#[test]
fn revoking_an_api_token_invalidates_its_management_transfers() {
    let f = Fixture::new();
    let share = f.published(None, false);
    let (token, _) = f
        .app
        .new_transfer(
            share.share["id"].as_str().unwrap(),
            &share.files[0].id,
            None,
            &secret(),
            Some(&f.actor),
        )
        .unwrap();
    f.app.download(&token, true).unwrap();
    f.app
        .db(|db| remove(db, "credential", &f.actor.credential.id))
        .unwrap();
    assert!(f.app.download(&token, true).is_err());
}

#[test]
fn issuance_is_idempotent_after_the_final_public_download() {
    let fixture = Fixture::new();
    let share = fixture.published(Some(1), false);
    let id = share.share["id"].as_str().unwrap();
    let key = secret();
    let (token, _) = fixture
        .app
        .new_transfer(id, &share.files[0].id, None, &key, None)
        .unwrap();
    fixture.app.download(&token, true).unwrap();
    let (retry, _) = fixture
        .app
        .new_transfer(id, &share.files[0].id, None, &key, None)
        .unwrap();
    assert_eq!(retry, token);
    assert!(
        fixture
            .app
            .new_transfer(id, &share.files[0].id, None, &secret(), None)
            .is_err()
    );
    fixture.app.revoke(&fixture.actor, id).unwrap();
    assert!(
        fixture
            .app
            .new_transfer(id, &share.files[0].id, None, &key, None)
            .is_err()
    );
}

#[test]
fn invalid_management_credentials_cannot_retain_exhausted_files() {
    for mode in ["logout", "revoke", "disable", "reset", "expiry"] {
        let fixture = Fixture::new();
        let share = fixture.published(Some(1), false);
        let id = share.share["id"].as_str().unwrap();
        let user = fixture
            .app
            .create_user(&fixture.actor, "manager", "manager-password", true)
            .unwrap();
        let (_, manager) = fixture.app.issue(&user, "management", false).unwrap();
        let (management, _) = fixture
            .app
            .new_transfer(id, &share.files[0].id, None, &secret(), Some(&manager))
            .unwrap();
        fixture.app.download(&management, true).unwrap();
        let public = fixture.transfer(&share, &share.files[0].id);
        fixture.app.download(&public, true).unwrap();
        fixture
            .app
            .db(|db| {
                crate::storage::database::remove(db, "transfer", &crate::crypto::digest(&public))
            })
            .unwrap();
        match mode {
            "logout" => fixture.app.logout(&manager).unwrap(),
            "revoke" => fixture
                .app
                .revoke_token(&manager, &manager.credential.id)
                .unwrap(),
            "disable" => fixture
                .app
                .change_user(&fixture.actor, &user.id, None, false)
                .unwrap(),
            "reset" => fixture
                .app
                .change_user(&fixture.actor, &user.id, Some("new-manager-password"), true)
                .unwrap(),
            "expiry" => {
                fixture
                    .app
                    .db(|db| {
                        let mut credential: crate::models::Credential =
                            get(db, "credential", &manager.credential.id)?.unwrap();
                        credential.expires = fixture.app.now();
                        put(db, "credential", &credential.id, &credential)
                    })
                    .unwrap();
            }
            _ => unreachable!(),
        }
        assert!(fixture.app.download(&management, true).is_err(), "{mode}");
        fixture.app.cleanup().unwrap();
        assert!(!fixture.app.file_path(&share.files[0]).exists(), "{mode}");
    }
}

#[test]
fn expired_idempotency_keys_cannot_resurrect_download_urls_after_cleanup() {
    for management in [false, true] {
        for charged in [false, true] {
            let f = Fixture::new();
            let share = f.published(None, false);
            let id = share.share["id"].as_str().unwrap();
            let key = secret();
            let actor = management.then_some(&f.actor);
            let (old, transfer) = f
                .app
                .new_transfer(id, &share.files[0].id, None, &key, actor)
                .unwrap();
            if charged {
                f.app.download(&old, true).unwrap();
            }
            let used = u64::from(charged && !management);
            f.advance(DAY);
            f.app.cleanup().unwrap();
            f.app
                .db(|db| {
                    assert!(get::<Transfer>(db, "transfer", &transfer.id)?.is_some());
                    Ok(())
                })
                .unwrap();
            assert_eq!(
                f.app
                    .new_transfer(id, &share.files[0].id, None, &key, actor)
                    .unwrap_err()
                    .0,
                404
            );
            assert_eq!(f.app.download(&old, true).unwrap_err().0, 404);
            assert_eq!(f.app.inspect(&f.actor, id).unwrap().share["used"], used);
            let (fresh, _) = f
                .app
                .new_transfer(id, &share.files[0].id, None, &secret(), actor)
                .unwrap();
            assert_ne!(old, fresh);
            f.app.download(&fresh, true).unwrap();
            assert_eq!(f.app.download(&old, true).unwrap_err().0, 404);
            assert_eq!(
                f.app.inspect(&f.actor, id).unwrap().share["used"],
                used + u64::from(!management)
            );
        }
    }
}

#[test]
fn management_idempotency_is_scoped_to_independently_revocable_credentials() {
    let f = Fixture::new();
    let share = f.published(None, false);
    let id = share.share["id"].as_str().unwrap();
    let (_, second) = f.app.create_token(&f.actor, "second credential").unwrap();
    let key = secret();
    let (first, _) = f
        .app
        .new_transfer(id, &share.files[0].id, None, &key, Some(&f.actor))
        .unwrap();
    f.app.download(&first, true).unwrap();
    // A second live credential has its own scope even before either is revoked.
    let (other, _) = f
        .app
        .new_transfer(id, &share.files[0].id, None, &key, Some(&second))
        .unwrap();
    assert_ne!(first, other);
    f.app.revoke_token(&second, &f.actor.credential.id).unwrap();
    assert_eq!(f.app.download(&first, true).unwrap_err().0, 404);
    let (retry, transfer) = f
        .app
        .new_transfer(id, &share.files[0].id, None, &key, Some(&second))
        .unwrap();
    assert_eq!(retry, other);
    assert_eq!(
        transfer.credential.as_deref(),
        Some(second.credential.id.as_str())
    );
    f.app.download(&retry, true).unwrap();
    assert_eq!(f.app.inspect(&second, id).unwrap().share["used"], 0);

    // A fresh credential can use a key held only by now-revoked credentials.
    let (_, third) = f.app.create_token(&second, "third credential").unwrap();
    f.app.revoke_token(&third, &second.credential.id).unwrap();
    let (independent, _) = f
        .app
        .new_transfer(id, &share.files[0].id, None, &key, Some(&third))
        .unwrap();
    assert_ne!(independent, retry);
    f.app.download(&independent, true).unwrap();
}

#[test]
fn already_issued_account_scoped_management_urls_stay_valid() {
    let f = Fixture::new();
    let share = f.published(None, false);
    let id = share.share["id"].as_str().unwrap();
    let key = secret();
    let (current, mut transfer) = f
        .app
        .new_transfer(id, &share.files[0].id, None, &key, Some(&f.actor))
        .unwrap();
    let legacy = digest(format!(
        "{id}:{}:{key}:{}:{}",
        share.files[0].id, transfer.version, f.actor.user.id
    ));
    f.app
        .db(|db| {
            remove(db, "transfer", &transfer.id)?;
            transfer.id = digest(&legacy);
            put(db, "transfer", &transfer.id, &transfer)
        })
        .unwrap();
    f.app.download(&legacy, true).unwrap();
    f.app.cleanup().unwrap();
    assert!(f.app.download(&legacy, true).is_ok());
    let (new, _) = f
        .app
        .new_transfer(id, &share.files[0].id, None, &key, Some(&f.actor))
        .unwrap();
    assert_eq!(new, current);
    assert_ne!(new, legacy);
    f.app.download(&new, true).unwrap();
    assert_eq!(f.app.inspect(&f.actor, id).unwrap().share["used"], 0);
}

#[test]
fn public_access_rechecks_a_concurrent_password_change_without_holding_the_database() {
    use std::{sync::mpsc, time::Duration};

    for metadata in [false, true] {
        for old_password in [None, Some("old-share-password")] {
            let f = Fixture::new();
            let share = f.published(None, false);
            let id = share.share["id"].as_str().unwrap();
            if let Some(password) = old_password {
                f.app
                    .edit(
                        &f.actor,
                        id,
                        EditShare {
                            title: "Project files".into(),
                            expires: None,
                            limit: None,
                            password: Some(password.into()),
                            change_password: true,
                        },
                    )
                    .unwrap();
            }
            let (snapshot_send, snapshot_receive) = mpsc::channel();
            let (continue_send, continue_receive) = mpsc::channel();
            let app = f.app.clone();
            let request_id = id.to_owned();
            let resource = share.files[0].id.clone();
            let request = std::thread::spawn(move || {
                let pause = || {
                    snapshot_send.send(()).unwrap();
                    continue_receive
                        .recv_timeout(Duration::from_secs(10))
                        .unwrap();
                };
                if metadata {
                    app.public_share_after_snapshot(&request_id, old_password, pause)
                        .map(|_| ())
                } else {
                    app.new_transfer_after_snapshot(
                        &request_id,
                        &resource,
                        old_password,
                        &secret(),
                        None,
                        pause,
                    )
                    .map(|_| ())
                }
            });
            snapshot_receive
                .recv_timeout(Duration::from_secs(5))
                .unwrap();
            let app = f.app.clone();
            let actor = f.actor.clone();
            let edit_id = id.to_owned();
            let (edited_send, edited_receive) = mpsc::channel();
            let edit = std::thread::spawn(move || {
                edited_send
                    .send(app.edit(
                        &actor,
                        &edit_id,
                        EditShare {
                            title: "New title".into(),
                            expires: None,
                            limit: None,
                            password: Some("replacement-share-password".into()),
                            change_password: true,
                        },
                    ))
                    .unwrap();
            });
            let changed_without_lock = edited_receive.recv_timeout(Duration::from_secs(5));
            continue_send.send(()).unwrap();
            let result = request.join().unwrap();
            edit.join().unwrap();
            changed_without_lock.unwrap().unwrap();
            assert_eq!(result.unwrap_err().0, 401);
            assert!(
                f.app
                    .public_share(id, Some("replacement-share-password"))
                    .is_ok()
            );
            f.app
                .db(|db| {
                    assert!(all::<Transfer>(db, "transfer")?.is_empty());
                    Ok(())
                })
                .unwrap();
            assert_eq!(f.app.inspect(&f.actor, id).unwrap().share["used"], 0);
        }
    }
}

#[test]
fn public_access_rechecks_quota_deadline_and_revocation_after_its_snapshot() {
    for metadata in [false, true] {
        for change in ["quota", "deadline", "revoke"] {
            let f = Fixture::new();
            let share = f.published(Some(1), false);
            let id = share.share["id"].as_str().unwrap();
            let token = f.transfer(&share, &share.files[0].id);
            let change_state = || match change {
                "quota" => {
                    f.app.download(&token, true).unwrap();
                }
                "deadline" => f.advance(7 * DAY),
                "revoke" => f.app.revoke(&f.actor, id).unwrap(),
                _ => unreachable!(),
            };
            let result = if metadata {
                f.app
                    .public_share_after_snapshot(id, None, change_state)
                    .map(|_| ())
            } else {
                f.app
                    .new_transfer_after_snapshot(
                        id,
                        &share.files[0].id,
                        None,
                        &secret(),
                        None,
                        change_state,
                    )
                    .map(|_| ())
            };
            assert_eq!(result.unwrap_err().0, 404, "{change}");
            assert_eq!(
                f.app.inspect(&f.actor, id).unwrap().share["used"],
                u64::from(change == "quota")
            );
        }
    }
}

#[test]
fn unavailable_public_shares_return_404_before_password_errors() {
    let f = Fixture::new();
    let share = f.published(None, false);
    let id = share.share["id"].as_str().unwrap();
    f.app
        .edit(
            &f.actor,
            id,
            EditShare {
                title: "Private files".into(),
                expires: Some(f.app.now() + 10),
                limit: None,
                password: Some("share-password".into()),
                change_password: true,
            },
        )
        .unwrap();
    for password in [None, Some("wrong-password")] {
        assert_eq!(f.app.public_share(id, password).unwrap_err().0, 401);
        assert_eq!(
            f.app
                .new_transfer(id, &share.files[0].id, password, &secret(), None)
                .unwrap_err()
                .0,
            401
        );
    }
    f.advance(10);
    assert_eq!(f.app.public_share(id, None).unwrap_err().0, 404);
    assert_eq!(
        f.app
            .new_transfer(id, &share.files[0].id, None, &secret(), None)
            .unwrap_err()
            .0,
        404
    );
}

#[test]
fn legacy_public_retries_keep_their_recorded_capability_and_charge() {
    let f = Fixture::new();
    let share = f.published(None, false);
    let id = share.share["id"].as_str().unwrap();
    let resource = &share.files[0].id;
    let key = secret();
    let (_, mut transfer) = f.app.new_transfer(id, resource, None, &key, None).unwrap();
    let legacy = digest(format!("{id}:{resource}:{key}:{}:public", transfer.version));
    f.app
        .db(|db| {
            remove(db, "transfer", &transfer.id)?;
            transfer.id = digest(&legacy);
            put(db, "transfer", &transfer.id, &transfer)
        })
        .unwrap();
    f.app.download(&legacy, true).unwrap();
    let (retry, _) = f.app.new_transfer(id, resource, None, &key, None).unwrap();
    assert_eq!(retry, legacy);
    f.app.download(&retry, true).unwrap();
    assert_eq!(f.app.inspect(&f.actor, id).unwrap().share["used"], 1);
    f.advance(DAY);
    f.app.cleanup().unwrap();
    assert_eq!(
        f.app
            .new_transfer(id, resource, None, &key, None)
            .unwrap_err()
            .0,
        404
    );
    assert_eq!(f.app.download(&legacy, true).unwrap_err().0, 404);
}

#[test]
fn previously_cleaned_legacy_public_urls_cannot_revive_after_upgrade() {
    let f = Fixture::new();
    let share = f.published(None, false);
    let id = share.share["id"].as_str().unwrap();
    let resource = &share.files[0].id;
    let key = secret();
    let (_, mut transfer) = f.app.new_transfer(id, resource, None, &key, None).unwrap();
    let legacy = digest(format!("{id}:{resource}:{key}:{}:public", transfer.version));
    f.app
        .db(|db| {
            remove(db, "transfer", &transfer.id)?;
            transfer.id = digest(&legacy);
            put(db, "transfer", &transfer.id, &transfer)
        })
        .unwrap();
    f.app.download(&legacy, true).unwrap();
    f.advance(DAY);
    // Model the previous server's cleanup, which deleted expired records.
    f.app
        .db(|db| remove(db, "transfer", &digest(&legacy)))
        .unwrap();
    assert_eq!(f.app.download(&legacy, true).unwrap_err().0, 404);
    let (fresh, _) = f.app.new_transfer(id, resource, None, &key, None).unwrap();
    assert_ne!(fresh, legacy);
    assert_eq!(f.app.download(&legacy, true).unwrap_err().0, 404);
    assert_eq!(f.app.inspect(&f.actor, id).unwrap().share["used"], 1);
    f.app.download(&fresh, true).unwrap();
    assert_eq!(f.app.download(&legacy, true).unwrap_err().0, 404);
    assert_eq!(f.app.inspect(&f.actor, id).unwrap().share["used"], 2);
    let (retry, _) = f.app.new_transfer(id, resource, None, &key, None).unwrap();
    assert_eq!(fresh, retry);
}
