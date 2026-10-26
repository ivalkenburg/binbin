use super::support::Fixture;
use crate::models::*;

#[test]
fn account_disable_invalidates_sessions_and_tokens_but_not_shares() {
    let f = Fixture::new();
    let u = f
        .app
        .create_user(&f.actor, "alice", "alice-secret-pass", false)
        .unwrap();
    let (raw, a) = f.app.issue(&u, "laptop", false).unwrap();
    let s = f
        .app
        .create_share(
            &a,
            NewShare {
                title: "hello".into(),
                files: vec![NewFile {
                    name: "empty".into(),
                    size: 0,
                }],
                days: None,
                limit: None,
                password: None,
            },
        )
        .unwrap();
    let id = s.share["id"].as_str().unwrap();
    f.app.publish(&a, id).unwrap();
    f.app.change_user(&f.actor, &u.id, None, false).unwrap();
    assert!(f.app.authenticate(&raw, None, false, false).is_err());
    assert!(f.app.public_share(id, None).is_ok());
}
#[test]
fn users_cannot_manage_other_owners() {
    let f = Fixture::new();
    let s = f.published(None, false);
    let u = f
        .app
        .create_user(&f.actor, "alice", "alice-secret-pass", false)
        .unwrap();
    let (_, a) = f.app.issue(&u, "laptop", false).unwrap();
    assert!(f.app.inspect(&a, s.share["id"].as_str().unwrap()).is_err());
    assert!(f.app.list_shares(&a).unwrap().is_empty());
    assert!(
        f.app
            .create_user(&a, "bob", "bob-secret-password", false)
            .is_err()
    );
}
#[test]
fn csrf_and_session_types_are_enforced() {
    let f = Fixture::new();
    let (raw, a) = f.app.login("admin", "correct-horse-battery").unwrap();
    assert!(f.app.authenticate(&raw, None, true, true).is_err());
    assert!(
        f.app
            .authenticate(&raw, Some(&a.credential.csrf), true, true)
            .is_ok()
    );
    let (token, _) = f.app.issue(&a.user, "cli", false).unwrap();
    assert!(f.app.authenticate(&token, None, true, false).is_ok());
    assert!(f.app.authenticate(&token, None, false, true).is_err());
}

#[test]
fn stale_password_snapshot_cannot_issue_a_credential() {
    let fixture = Fixture::new();
    let stale = fixture.actor.user.clone();
    fixture
        .app
        .change_user(
            &fixture.actor,
            &stale.id,
            Some("replacement-password"),
            true,
        )
        .unwrap();
    assert!(fixture.app.issue(&stale, "stale login", true).is_err());
    assert!(
        fixture
            .app
            .create_token(&fixture.actor, "stale API token")
            .is_err()
    );
    assert!(fixture.app.login("admin", "replacement-password").is_ok());
}

#[test]
fn a_revoked_source_cannot_issue_tokens_or_mutate_shares() {
    let fixture = Fixture::new();
    fixture
        .app
        .revoke_token(&fixture.actor, &fixture.actor.credential.id)
        .unwrap();
    assert!(
        fixture
            .app
            .create_token(&fixture.actor, "new token")
            .is_err()
    );
    assert!(
        fixture
            .app
            .create_share(
                &fixture.actor,
                crate::models::NewShare {
                    title: "Unauthorized draft".into(),
                    files: vec![crate::models::NewFile {
                        name: "empty".into(),
                        size: 0
                    }],
                    days: None,
                    limit: None,
                    password: None,
                }
            )
            .is_err()
    );
}
