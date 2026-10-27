use crate::{
    App,
    models::Settings,
    storage::database::{get, put},
};
use rusqlite::Connection;

#[test]
fn existing_version_one_database_upgrades_once_without_changing_records() {
    let root = tempfile::tempdir().unwrap();
    let mut database = Connection::open(root.path().join("binbin.sqlite")).unwrap();
    database
        .execute_batch(include_str!("../../migrations/0001_initial.sql"))
        .unwrap();
    let settings = Settings {
        storage_limit: 1234567,
        ..Settings::default()
    };
    put(&database, "settings", "instance", &settings).unwrap();
    let existing = serde_json::json!({"title":"Deployed share","files":["original-id"]});
    put(&database, "migration_fixture", "deployed", &existing).unwrap();
    crate::storage::database::migrate(&mut database).unwrap();
    crate::storage::database::migrate(&mut database).unwrap();
    assert_eq!(
        database
            .pragma_query_value::<u32, _>(None, "user_version", |row| row.get(0))
            .unwrap(),
        2
    );
    assert_eq!(
        get::<serde_json::Value>(&database, "migration_fixture", "deployed")
            .unwrap()
            .unwrap(),
        existing
    );
    assert_eq!(
        get::<Settings>(&database, "settings", "instance")
            .unwrap()
            .unwrap()
            .storage_limit,
        1234567
    );
    let index: String = database
        .query_row(
            "SELECT name FROM sqlite_master WHERE type='index' AND name='removed_files_by_share'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(index, "removed_files_by_share");
    drop(database);
    let app = App::open(root.path(), "http://localhost:8080".into()).unwrap();
    assert_eq!(app.settings().unwrap().storage_limit, 1234567);
}

#[test]
fn newer_database_versions_are_rejected_without_resetting_them() {
    let mut database = Connection::open_in_memory().unwrap();
    database.pragma_update(None, "user_version", 3).unwrap();
    assert!(crate::storage::database::migrate(&mut database).is_err());
    assert_eq!(
        database
            .pragma_query_value::<u32, _>(None, "user_version", |row| row.get(0))
            .unwrap(),
        3
    );
}

#[test]
fn deployed_accounts_shares_and_uploaded_bytes_survive_upgrade() {
    let fixture = super::support::Fixture::new();
    let share = fixture.published(None, false);
    let file = &share.files[0];
    let database = Connection::open(fixture.app.root.join("binbin.sqlite")).unwrap();
    database
        .execute_batch("DROP INDEX removed_files_by_share; PRAGMA user_version=1;")
        .unwrap();
    drop(database);
    let upgraded = App::open(&fixture.app.root, fixture.app.public_url.clone()).unwrap();
    upgraded
        .clock
        .store(fixture.app.now(), std::sync::atomic::Ordering::Relaxed);
    let current = upgraded
        .inspect(&fixture.actor, share.share["id"].as_str().unwrap())
        .unwrap();
    assert_eq!(current.share, share.share);
    assert_eq!(current.files[0].id, file.id);
    assert_eq!(current.files[0].offset, 5);
    assert_eq!(current.files[0].sha256, file.sha256);
    assert_eq!(std::fs::read(upgraded.file_path(file)).unwrap(), b"hello");
}
