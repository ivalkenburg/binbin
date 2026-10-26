use super::support::Fixture;
use crate::{DAY, crypto::*, models::*, storage::database::*};
use std::{
    fs::{self, File},
    io::Read,
};

#[test]
fn private_drafts_and_explicit_publication() {
    let f = Fixture::new();
    let s = f.draft(None, false);
    let id = s.share["id"].as_str().unwrap();
    assert_eq!(f.app.public_share(id, None).unwrap_err().0, 404);
    assert!(f.app.publish(&f.actor, id).is_err());
    f.app.upload(&f.actor, &s.files[0].id, 0, b"hello").unwrap();
    let published = f.app.publish(&f.actor, id).unwrap();
    assert_eq!(published.share["expires"], f.app.now() + 7 * DAY);
    assert_eq!(
        published.files[0].sha256.as_deref(),
        Some(digest(b"hello").as_str())
    );
    assert!(f.app.upload(&f.actor, &s.files[0].id, 5, b"").is_err());
}
#[test]
fn resumable_upload_enforces_offsets_sizes_and_contents() {
    let f = Fixture::new();
    let s = f.draft(None, false);
    let file = &s.files[0];
    f.app.upload(&f.actor, &file.id, 0, b"he").unwrap();
    assert_eq!(
        f.app.upload(&f.actor, &file.id, 0, b"he").unwrap_err().0,
        409
    );
    assert!(f.app.upload(&f.actor, &file.id, 2, b"longer").is_err());
    assert!(
        f.app
            .verify_upload_prefix(&f.actor, &file.id, &digest(b"he"))
            .is_ok()
    );
    assert!(
        f.app
            .verify_upload_prefix(&f.actor, &file.id, &digest(b"no"))
            .is_err()
    );
    assert_eq!(
        f.app.upload_proofs(&f.actor, &file.id).unwrap(),
        vec![digest(b"he")]
    );
    f.app.upload(&f.actor, &file.id, 2, b"llo").unwrap();
    assert_eq!(fs::read(f.app.file_path(file)).unwrap(), b"hello");
}
#[test]
fn zip_contains_exact_original_files() {
    let f = Fixture::new();
    let s = f.published(None, true);
    let (raw, _) = f
        .app
        .new_transfer(
            s.share["id"].as_str().unwrap(),
            "zip",
            None,
            &secret(),
            Some(&f.actor),
        )
        .unwrap();
    let (path, size, _, hash) = f.app.download(&raw, true).unwrap();
    let data = fs::read(&path).unwrap();
    assert_eq!(size, data.len() as u64);
    assert_eq!(digest(&data), hash);
    let mut zip = zip::ZipArchive::new(File::open(path).unwrap()).unwrap();
    assert_eq!(zip.len(), 2);
    let mut contents = String::new();
    zip.by_name("hello.txt")
        .unwrap()
        .read_to_string(&mut contents)
        .unwrap();
    assert_eq!(contents, "hello");
    assert_eq!(zip.by_name("empty.txt").unwrap().size(), 0);
}
#[test]
fn administrator_caps_rechecked_at_publication() {
    let f = Fixture::new();
    let s = f.draft(None, false);
    f.app.upload(&f.actor, &s.files[0].id, 0, b"hello").unwrap();
    let mut settings = f.app.settings().unwrap();
    settings.max_downloads = Some(10);
    f.app
        .db(|db| put(db, "settings", "instance", &settings))
        .unwrap();
    assert!(
        f.app
            .publish(&f.actor, s.share["id"].as_str().unwrap())
            .is_err()
    );
}
#[test]
fn interrupted_publication_recovers_without_losing_originals() {
    let f = Fixture::new();
    let draft = f.draft(None, true);
    let id = draft.share["id"].as_str().unwrap();
    f.app
        .upload(&f.actor, &draft.files[0].id, 0, b"hello")
        .unwrap();
    f.app
        .db(|db| {
            let mut share: Share = get(db, "share", id)?.unwrap();
            share.state = "preparing".into();
            put(db, "share", id, &share)
        })
        .unwrap();
    fs::write(f.app.root.join(id).join("archive.tmp"), b"partial").unwrap();
    fs::write(
        f.app.root.join(id).join("archive.zip"),
        b"renamed but not active",
    )
    .unwrap();
    f.app.recover_publications().unwrap();
    assert_eq!(f.app.inspect(&f.actor, id).unwrap().share["state"], "draft");
    assert!(!f.app.root.join(id).join("archive.tmp").exists());
    assert!(!f.app.root.join(id).join("archive.zip").exists());
    assert_eq!(
        fs::read(f.app.file_path(&draft.files[0])).unwrap(),
        b"hello"
    );
    assert!(f.app.publish(&f.actor, id).is_ok());
}

#[test]
fn failed_publication_leaves_a_repairable_private_draft() {
    let f = Fixture::new();
    let draft = f.draft(None, false);
    let id = draft.share["id"].as_str().unwrap();
    f.app
        .upload(&f.actor, &draft.files[0].id, 0, b"hello")
        .unwrap();
    fs::write(f.app.file_path(&draft.files[0]), b"bad").unwrap();
    assert!(f.app.publish(&f.actor, id).is_err());
    assert_eq!(f.app.inspect(&f.actor, id).unwrap().share["state"], "draft");
    assert!(f.app.public_share(id, None).is_err());
    fs::write(f.app.file_path(&draft.files[0]), b"hello").unwrap();
    assert!(f.app.publish(&f.actor, id).is_ok());
}

#[cfg(unix)]
#[test]
fn resume_hashing_does_not_hold_the_database_transaction() {
    use std::{io::Write, sync::mpsc, time::Duration};
    let fixture = Fixture::new();
    let share = fixture.draft(None, false);
    let file = &share.files[0];
    fixture
        .app
        .upload(&fixture.actor, &file.id, 0, b"hello")
        .unwrap();
    let path = fixture.app.file_path(file);
    std::fs::remove_file(&path).unwrap();
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&path)
            .status()
            .unwrap()
            .success()
    );
    let app = fixture.app.clone();
    let actor = fixture.actor.clone();
    let id = file.id.clone();
    let verification = std::thread::spawn(move || app.upload_proofs(&actor, &id));
    // Opening the writer synchronizes with the verifier's read handle. Keep the
    // read blocked on data while another thread performs a real DB transaction.
    let mut writer = std::fs::OpenOptions::new().write(true).open(path).unwrap();
    let (send, receive) = mpsc::channel();
    let app = fixture.app.clone();
    let lookup = std::thread::spawn(move || send.send(app.settings().is_ok()).unwrap());
    let responsive = receive.recv_timeout(Duration::from_secs(2));
    writer.write_all(b"hello").unwrap();
    drop(writer);
    assert_eq!(
        verification.join().unwrap().unwrap(),
        vec![crate::crypto::digest(b"hello")]
    );
    lookup.join().unwrap();
    assert!(responsive.unwrap());
}

#[test]
fn activation_failure_reclaims_the_archive_and_preserves_originals() {
    let fixture = Fixture::new();
    let draft = fixture.draft(None, true);
    let id = draft.share["id"].as_str().unwrap();
    fixture
        .app
        .upload(&fixture.actor, &draft.files[0].id, 0, b"hello")
        .unwrap();
    fixture
        .app
        .db(|db| {
            db.execute_batch(
                "CREATE TEMP TRIGGER fail_activation BEFORE INSERT ON objects
            WHEN NEW.kind='share' AND json_extract(NEW.data,'$.state')='active'
            BEGIN SELECT RAISE(FAIL, 'injected activation failure'); END;",
            )?;
            Ok(())
        })
        .unwrap();
    assert!(fixture.app.publish(&fixture.actor, id).is_err());
    assert_eq!(
        fixture.app.inspect(&fixture.actor, id).unwrap().share["state"],
        "draft"
    );
    assert!(!fixture.app.root.join(id).join("archive.zip").exists());
    assert!(fixture.app.public_share(id, None).is_err());
    assert_eq!(
        fs::read(fixture.app.file_path(&draft.files[0])).unwrap(),
        b"hello"
    );
    fixture
        .app
        .db(|db| {
            db.execute_batch("DROP TRIGGER fail_activation")?;
            Ok(())
        })
        .unwrap();
    assert!(fixture.app.publish(&fixture.actor, id).is_ok());
}
