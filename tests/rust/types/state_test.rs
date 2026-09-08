#[cfg(unix)]
use std::{ffi::OsStr, os::unix::ffi::OsStrExt};
use std::{fs, path::PathBuf};

use chrono::{DateTime, Timelike};
use preader::{
    Config, Error, FileMetadata, IteratorBuild, IteratorRead, State, StateData, StateManager,
    Timestamps,
};
use rstest::rstest;
use tempfile::TempDir;

use crate::common::{
    constants::{TEST_FILE_PATH, TEST_FINGERPRINT, TEST_STATE_NAME},
    fixtures::tmp_dir,
    funcs::{reader, write},
};

#[cfg(unix)]
const NON_UTF8_PATH: &[u8] = b"/tmp/data-\xff.bin";

fn state_data(path: PathBuf) -> StateData {
    let stamp = DateTime::from_timestamp(1_700_000_000, 0).unwrap();

    StateData {
        name: TEST_STATE_NAME.to_string(),
        file: FileMetadata {
            path,
            size: 4,
            mtime: stamp,
            fingerprint: TEST_FINGERPRINT.to_string(),
        },
        position: 7,
        timestamps: Timestamps {
            created_at: stamp,
            updated_at: stamp,
        },
        checksum: String::new(),
    }
}

#[cfg(unix)]
fn state_in(tmp_dir: &TempDir) -> State {
    let manager = StateManager::from(&Config {
        state_dir: tmp_dir.path().join("preader"),
        ..Config::default()
    });
    let data = state_data(PathBuf::from(OsStr::from_bytes(NON_UTF8_PATH)));

    State::from((data, manager))
}

#[test]
fn eq_compares_the_data() {
    let one = State::from((
        state_data(PathBuf::from(TEST_FILE_PATH)),
        StateManager::default(),
    ));
    let mut data = state_data(PathBuf::from(TEST_FILE_PATH));

    data.position += 1;

    let other = State::from((data, StateManager::default()));

    assert!(one != other);
}

#[test]
fn eq_ignores_the_manager() {
    let elsewhere = Config {
        state_dir: PathBuf::from("/tmp/preader-elsewhere"),
        ..Config::default()
    };
    let one = State::from((
        state_data(PathBuf::from(TEST_FILE_PATH)),
        StateManager::from(&elsewhere),
    ));
    let other = State::from((
        state_data(PathBuf::from(TEST_FILE_PATH)),
        StateManager::default(),
    ));

    assert_ne!(one.path().unwrap(), other.path().unwrap());
    assert!(one == other);
}

#[cfg(unix)]
#[rstest]
fn verify_fails_when_the_path_is_not_utf8(tmp_dir: TempDir) {
    let error = state_in(&tmp_dir).verify().err().unwrap();

    assert!(error.to_string().contains("invalid UTF-8"));
}

#[cfg(unix)]
#[rstest]
fn save_fails_when_the_path_is_not_utf8(tmp_dir: TempDir) {
    let mut state = state_in(&tmp_dir);
    let error = state.save().err().unwrap();

    assert!(error.to_string().contains("invalid UTF-8"));
}

#[rstest]
fn verify_reports_a_directory_as_not_a_file(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", b"abcdef");
    let state = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap().state().clone();

    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();

    let error = state.verify().err().unwrap();

    assert!(
        matches!(&error, Error::NotAFile(reported) if reported.ends_with("data.bin")),
        "got {error}"
    );
    assert!(error.to_string().contains("not a file"));
}

#[rstest]
fn verify_reports_the_checksum_before_the_file_checks(tmp_dir: TempDir) {
    let unverified = Config {
        verify_state: false,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, unverified);
    let path = write(&tmp_dir, "data.bin", b"abcdef");
    let mut saved = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap().state().clone();
    let payload = saved.save().unwrap();
    let tampered = fs::read_to_string(&payload)
        .unwrap()
        .replace("\"position\": 0", "\"position\": 999");

    fs::write(&payload, tampered).unwrap();

    let loaded = reader.states().load(TEST_STATE_NAME).unwrap();

    fs::remove_file(&path).unwrap();

    let error = loaded.verify().err().unwrap();

    assert!(
        error.to_string().contains("state checksum mismatch"),
        "got {error}"
    );
}

// ───────────── parity with the recorded Python contract ─────────────

const LARGE: usize = 2560;
const LINE: &[u8] = b"foo\n";

fn large(tmp_dir: &TempDir) -> PathBuf {
    write(tmp_dir, "large.bin", &b"foo\n".repeat(LARGE))
}

fn mtime(path: &PathBuf) -> std::time::SystemTime {
    fs::metadata(path).unwrap().modified().unwrap()
}

fn set_mtime(path: &PathBuf, stamp: std::time::SystemTime) {
    fs::OpenOptions::new()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(stamp)
        .unwrap();
}

fn unverified(tmp_dir: &TempDir) -> preader::PReader {
    reader(
        tmp_dir,
        Config {
            verify_state: false,
            ..Config::default()
        },
    )
}

fn stored(tmp_dir: &TempDir, path: &PathBuf) -> State {
    let reader = unverified(tmp_dir);
    let mut state = reader.bytes(path).state(TEST_STATE_NAME).build().unwrap().state().clone();

    state.save().unwrap();

    reader.states().load(TEST_STATE_NAME).unwrap()
}

#[rstest]
fn state_fields_describe_the_tracked_file(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", b"foo\n");
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();
    let state = bytes.state();

    assert_eq!(state.name, TEST_STATE_NAME);
    assert_eq!(state.position, 0);
    assert_eq!(state.file.path, path.canonicalize().unwrap());
    assert_eq!(
        state.path().unwrap(),
        reader.config().state_dir.join(format!("{TEST_STATE_NAME}.state.json"))
    );
    assert_eq!(state.timestamps.created_at, state.timestamps.updated_at);
}

#[rstest]
fn checksum_is_a_stable_hex_digest(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", b"foo\n");
    let mut bytes = reader.bytes(&path).build().unwrap();
    let digest = bytes.state().checksum().unwrap();

    assert_eq!(digest.len(), 64);
    assert!(
        digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    );
    assert_eq!(bytes.state().checksum().unwrap(), digest);
}

#[rstest]
fn save_returns_the_created_path(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", b"foo\n");
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    assert!(!bytes.state().path().unwrap().exists());

    let written = bytes.state().save().unwrap();

    assert_eq!(written, bytes.state().path().unwrap());
    assert!(written.exists());
}

#[rstest]
fn save_updates_updated_at_but_not_created_at(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", b"foo\n");
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();
    let created_at = bytes.state().timestamps.created_at;

    bytes.state().save().unwrap();
    bytes.state().save().unwrap();

    assert_eq!(bytes.state().timestamps.created_at, created_at);
    assert!(bytes.state().timestamps.updated_at > created_at);
}

#[rstest]
fn save_persists_across_readers(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", b"foo\n");
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    while bytes.read().unwrap().is_some() {}

    bytes.state().save().unwrap();
    drop(bytes);

    let fresh = crate::common::funcs::reader(&tmp_dir, Config::default());

    assert_eq!(fresh.states().load(TEST_STATE_NAME).unwrap().position, 4);
}

#[rstest]
fn save_fails_when_the_state_dir_is_a_file(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());

    fs::write(&reader.config().state_dir, b"foo").unwrap();

    let path = write(&tmp_dir, "data.bin", b"foo\n");
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();
    let error = bytes.state().save().err().unwrap();

    assert!(matches!(error, Error::Io(_)), "got {error}");
}

#[rstest]
fn save_fails_when_the_state_path_is_a_directory(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", b"foo\n");
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    fs::create_dir_all(bytes.state().path().unwrap()).unwrap();

    let error = bytes.state().save().err().unwrap();

    assert!(matches!(error, Error::Io(_)), "got {error}");
}

#[rstest]
fn verify_passes_when_the_file_is_untouched(tmp_dir: TempDir) {
    let path = large(&tmp_dir);

    stored(&tmp_dir, &path).verify().unwrap();
}

#[rstest]
fn verify_fails_on_a_tampered_payload(tmp_dir: TempDir) {
    let path = large(&tmp_dir);
    let state = stored(&tmp_dir, &path);
    let payload = state.path().unwrap();
    let tampered = fs::read_to_string(&payload)
        .unwrap()
        .replace("\"position\": 0", "\"position\": 999");

    fs::write(&payload, tampered).unwrap();

    let error = unverified(&tmp_dir).states().load(TEST_STATE_NAME).unwrap().verify().err();

    assert!(error.unwrap().to_string().contains("state checksum mismatch"));
}

#[rstest]
fn verify_fails_when_the_size_changed(tmp_dir: TempDir) {
    let path = large(&tmp_dir);
    let state = stored(&tmp_dir, &path);
    let stamp = mtime(&path);

    fs::OpenOptions::new().write(true).open(&path).unwrap().set_len(4500).unwrap();
    set_mtime(&path, stamp);

    let error = state.verify().err().unwrap();

    assert!(
        error.to_string().contains("file size mismatch"),
        "got {error}"
    );
}

#[rstest]
fn verify_fails_when_the_mtime_changed(tmp_dir: TempDir) {
    let path = large(&tmp_dir);
    let state = stored(&tmp_dir, &path);
    let stamp = mtime(&path) + std::time::Duration::from_secs(3600);

    set_mtime(&path, stamp);

    let error = state.verify().err().unwrap();

    assert!(
        error.to_string().contains("file mtime mismatch"),
        "got {error}"
    );
}

#[rstest]
fn verify_fails_when_the_fingerprint_changed(tmp_dir: TempDir) {
    use std::io::{Seek, SeekFrom, Write};

    let path = large(&tmp_dir);
    let state = stored(&tmp_dir, &path);
    let stamp = mtime(&path);
    let mut file = fs::OpenOptions::new().write(true).open(&path).unwrap();

    file.seek(SeekFrom::Start(10)).unwrap();
    file.write_all(b"\xff").unwrap();
    drop(file);
    set_mtime(&path, stamp);

    let error = state.verify().err().unwrap();

    assert!(
        error.to_string().contains("file fingerprint mismatch"),
        "got {error}"
    );
}

#[rstest]
fn verify_ignores_changes_past_the_fingerprint_window(tmp_dir: TempDir) {
    use std::io::{Seek, SeekFrom, Write};

    let path = large(&tmp_dir);
    let state = stored(&tmp_dir, &path);
    let stamp = mtime(&path);
    let mut file = fs::OpenOptions::new().write(true).open(&path).unwrap();

    file.seek(SeekFrom::Start(4500)).unwrap();
    file.write_all(b"\xff").unwrap();
    drop(file);
    set_mtime(&path, stamp);

    state.verify().unwrap();
}

#[rstest]
fn verify_fails_when_the_file_is_deleted(tmp_dir: TempDir) {
    let path = large(&tmp_dir);
    let state = stored(&tmp_dir, &path);

    fs::remove_file(&path).unwrap();

    let error = state.verify().err().unwrap();

    assert!(matches!(&error, Error::Io(_)), "got {error}");
    assert!(!error.to_string().contains("mismatch"), "got {error}");
}

#[rstest]
fn verify_suggests_a_resync(tmp_dir: TempDir) {
    use std::io::Write;

    let path = large(&tmp_dir);
    let state = stored(&tmp_dir, &path);

    fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap()
        .write_all(b"more")
        .unwrap();

    let error = state.verify().err().unwrap();

    assert!(
        error.to_string().contains("call state.resync(file)"),
        "got {error}"
    );
}

#[rstest]
fn resync_updates_the_file_metadata(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", b"foo\n");
    let state = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap().state().clone();
    let moved = write(&tmp_dir, "moved.bin", b"foo\nbar");
    let resynced = state.resync(&moved).unwrap();

    assert_eq!(resynced.file.path, moved.canonicalize().unwrap());
    assert_eq!(resynced.file.size, 7);
    assert_eq!(
        resynced.file.fingerprint,
        preader::fingerprint(&moved, preader::FINGERPRINT_SAMPLE_BYTES).unwrap()
    );
}

#[rstest]
fn resync_preserves_the_name_position_and_created_at(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", b"foo\n");
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    bytes.read().unwrap();

    let state = bytes.state().clone();
    let moved = write(&tmp_dir, "moved.bin", b"foo\n");
    let resynced = state.resync(&moved).unwrap();

    assert_eq!(resynced.name, state.name);
    assert_eq!(resynced.position, state.position);
    assert_eq!(resynced.timestamps.created_at, state.timestamps.created_at);
    assert!(resynced.timestamps.updated_at > state.timestamps.updated_at);
}

#[rstest]
fn resync_does_not_mutate_the_original(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", b"foo\n");
    let state = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap().state().clone();
    let moved = write(&tmp_dir, "moved.bin", b"foo\n");

    state.resync(&moved).unwrap();

    assert_eq!(state.file.path, path.canonicalize().unwrap());
}

fn recorded(tmp_dir: &TempDir, content: &[u8], read: u64) -> State {
    let reader = reader(tmp_dir, Config::default());
    let path = write(tmp_dir, "tracked.bin", content);
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).limit(read).build().unwrap();

    while bytes.read().unwrap().is_some() {}

    bytes.state().clone()
}

#[rstest]
#[case::truncated_inside_the_window(LINE, 25, LINE, 2)]
#[case::truncated_inside_a_wide_window(LINE, 2560, LINE, 250)]
#[case::truncated_with_a_changed_prefix(LINE, 2560, b"bar\n", 1250)]
#[case::grown_with_a_changed_prefix(LINE, 25, b"bar\n", 27)]
#[case::replaced_at_the_same_size(LINE, 25, b"bar\n", 25)]
#[case::emptied(LINE, 25, b"", 0)]
#[case::truncated_at_the_window(b"a", 4096, b"a", 4095)]
fn resync_rejects_a_lost_prefix(
    tmp_dir: TempDir,
    #[case] recorded_unit: &[u8],
    #[case] before: usize,
    #[case] unit: &[u8],
    #[case] after: usize,
) {
    let saved = recorded(&tmp_dir, &recorded_unit.repeat(before), 5);
    let path = write(&tmp_dir, "tracked.bin", &unit.repeat(after));
    let error = saved.resync(&path).err().unwrap();

    assert!(
        error.to_string().contains("file content differs from the tracked file"),
        "{error}"
    );
    assert!(
        error.to_string().contains("read it under a new state to start over"),
        "{error}"
    );
}

#[rstest]
#[case::appended(25, b"more")]
#[case::unchanged(25, b"")]
#[case::grown_past_a_wide_window(2560, b"more")]
fn resync_accepts_a_kept_prefix(tmp_dir: TempDir, #[case] count: usize, #[case] extra: &[u8]) {
    let before = LINE.repeat(count);
    let after = [&before[..], extra].concat();
    let saved = recorded(&tmp_dir, &before, 5);
    let path = write(&tmp_dir, "tracked.bin", &after);
    let resynced = saved.resync(&path).unwrap();

    assert_eq!(resynced.position, 5);
    assert_eq!(resynced.file.size, after.len() as u64);
    assert!(resynced.position <= resynced.file.size);
    resynced.verify().unwrap();
}

#[rstest]
fn resync_clamps_the_position_to_the_new_size(tmp_dir: TempDir) {
    let saved = recorded(&tmp_dir, &LINE.repeat(2560), 6000);
    let path = write(&tmp_dir, "tracked.bin", &LINE.repeat(2560));

    fs::OpenOptions::new().write(true).open(&path).unwrap().set_len(4500).unwrap();

    let resynced = saved.resync(&path).unwrap();

    assert_eq!(saved.position, 6000);
    assert_eq!(resynced.position, 4500);
    assert_eq!(resynced.percent(), 100.0);
    resynced.verify().unwrap();
}

#[rstest]
fn resync_ignores_changes_past_the_fingerprint_window(tmp_dir: TempDir) {
    let mut after = LINE.repeat(2560);

    after[4500] = b'\xff';

    let saved = recorded(&tmp_dir, &LINE.repeat(2560), 6000);
    let path = write(&tmp_dir, "tracked.bin", &after);
    let resynced = saved.resync(&path).unwrap();

    assert_eq!(resynced.position, 6000);
    assert!(resynced.position <= resynced.file.size);
    resynced.verify().unwrap();
}

#[rstest]
fn resync_accepts_anything_for_an_empty_file(tmp_dir: TempDir) {
    let saved = recorded(&tmp_dir, b"", 0);
    let path = write(&tmp_dir, "tracked.bin", b"foo");
    let resynced = saved.resync(&path).unwrap();

    assert_eq!(resynced.position, 0);
    assert_eq!(resynced.file.size, 3);
}

#[rstest]
#[case::replaced(b"bar\n", 25, 50.0)]
#[case::shrunk(b"foo\n", 2, 625.0)]
#[case::emptied(b"", 0, 5000.0)]
fn resync_trusts_the_path_without_verification(
    tmp_dir: TempDir,
    #[case] unit: &[u8],
    #[case] count: usize,
    #[case] percent: f64,
) {
    let reader = unverified(&tmp_dir);
    let path = write(&tmp_dir, "tracked.bin", &LINE.repeat(25));
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).limit(50).build().unwrap();

    while bytes.read().unwrap().is_some() {}

    let saved = bytes.state().clone();

    drop(bytes);
    write(&tmp_dir, "tracked.bin", &unit.repeat(count));

    let resynced = saved.resync(&path).unwrap();

    assert_eq!(resynced.position, 50);
    assert_eq!(resynced.file.size, (unit.len() * count) as u64);
    assert_eq!(resynced.percent(), percent);
}

#[rstest]
#[case::at_the_window(4096, 4096, 100, 100, 100.0 * 100.0 / 4096.0)]
#[case::past_the_window(4097, 4096, 4097, 4096, 100.0)]
fn resync_accepts_at_the_fingerprint_window(
    tmp_dir: TempDir,
    #[case] before: usize,
    #[case] after: usize,
    #[case] read: u64,
    #[case] position: u64,
    #[case] percent: f64,
) {
    let saved = recorded(&tmp_dir, &b"a".repeat(before), read);
    let path = write(&tmp_dir, "tracked.bin", &b"a".repeat(after));
    let resynced = saved.resync(&path).unwrap();

    assert_eq!(resynced.position, position);
    assert_eq!(resynced.percent(), percent);
    assert!(resynced.position <= resynced.file.size);
}

#[rstest]
fn resync_keeps_an_unsafe_name_for_the_save(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "tracked.bin", &LINE.repeat(25));
    let saved = reader.bytes(&path).state("../../escape").build().unwrap().state().clone();
    let mut resynced = saved.resync(&path).unwrap();

    assert_eq!(resynced.name, "../../escape");

    let error = resynced.save().err().unwrap();

    assert!(error.to_string().contains("path escapes root"), "{error}");
}

#[rstest]
fn resync_fails_when_the_mtime_precedes_the_epoch(tmp_dir: TempDir) {
    use std::time::{Duration, SystemTime};

    let path = write(&tmp_dir, "tracked.bin", &LINE.repeat(25));
    let saved = recorded(&tmp_dir, &LINE.repeat(25), 5);
    let before_epoch = SystemTime::UNIX_EPOCH - Duration::from_secs(86_400);

    if fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(before_epoch)
        .is_err()
    {
        return; // a pre-epoch mtime cannot be set here
    }

    assert!(matches!(saved.resync(&path), Err(Error::Time(_))));
}

#[rstest]
#[case::a_symlink(true)]
#[case::an_unnormalized_path(false)]
fn resync_canonicalizes_the_path(tmp_dir: TempDir, #[case] linked: bool) {
    use std::os::unix::fs::symlink;

    let target = write(&tmp_dir, "tracked.bin", &LINE.repeat(25));
    let saved = recorded(&tmp_dir, &LINE.repeat(25), 5);
    let detour = if linked {
        let alias = tmp_dir.path().join("alias.bin");

        symlink(&target, &alias).unwrap();

        alias
    } else {
        tmp_dir.path().join(".").join("tracked.bin")
    };
    let resynced = saved.resync(&detour).unwrap();

    assert_eq!(resynced.file.path, target.canonicalize().unwrap());
}

#[rstest]
fn resync_keeps_a_position_that_equals_the_size(tmp_dir: TempDir) {
    let saved = recorded(&tmp_dir, &LINE.repeat(25), 100);
    let path = write(&tmp_dir, "tracked.bin", &LINE.repeat(25));
    let resynced = saved.resync(&path).unwrap();

    assert_eq!(saved.position, 100);
    assert_eq!(resynced.position, 100);
    assert_eq!(resynced.file.size, 100);
}

#[rstest]
fn resync_fails_when_the_file_is_unreadable(tmp_dir: TempDir) {
    use std::os::unix::fs::PermissionsExt;

    let path = write(&tmp_dir, "tracked.bin", &LINE.repeat(25));
    let saved = recorded(&tmp_dir, &LINE.repeat(25), 5);

    fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();

    if fs::read(&path).is_ok() {
        return; // permissions are not enforced here
    }

    let error = saved.resync(&path).err().unwrap();

    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();

    assert!(
        matches!(&error, Error::Io(io) if io.kind() == std::io::ErrorKind::PermissionDenied),
        "{error}"
    );
}

#[rstest]
fn resync_keeps_the_state_dir_for_the_save(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "tracked.bin", &LINE.repeat(25));
    let saved = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap().state().clone();
    let written = saved.resync(&path).unwrap().save().unwrap();

    assert!(
        written.starts_with(&reader.config().state_dir),
        "{written:?}"
    );
    assert!(written.is_file(), "{written:?}");
    assert_eq!(
        reader.states().load(TEST_STATE_NAME).unwrap().name,
        TEST_STATE_NAME
    );
}

#[rstest]
#[case::missing("missing.bin")]
#[case::a_directory("elsewhere")]
fn resync_still_checks_the_path_without_verification(tmp_dir: TempDir, #[case] name: &str) {
    let reader = unverified(&tmp_dir);
    let path = write(&tmp_dir, "tracked.bin", &LINE.repeat(25));
    let saved = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap().state().clone();
    let target = tmp_dir.path().join(name);

    if name == "elsewhere" {
        fs::create_dir(&target).unwrap();
    }

    assert!(saved.resync(&target).is_err());
}

#[rstest]
fn resync_accepts_every_path_shape(tmp_dir: TempDir) {
    let path = write(&tmp_dir, "tracked.bin", &LINE.repeat(25));
    let saved = recorded(&tmp_dir, &LINE.repeat(25), 5);
    let text = path.to_str().unwrap().to_owned();
    let canonical = path.canonicalize().unwrap();

    for resynced in [
        saved.resync(text.as_str()).unwrap(),
        saved.resync(&text).unwrap(),
        saved.resync(text.clone()).unwrap(),
        saved.resync(path.as_path()).unwrap(),
        saved.resync(&path).unwrap(),
        saved.resync(path.clone()).unwrap(),
    ] {
        assert_eq!(resynced.file.path, canonical);
    }
}

#[rstest]
fn resync_moves_the_identity_reference_forward(tmp_dir: TempDir) {
    let saved = recorded(&tmp_dir, &LINE.repeat(25), 5);
    let original = tmp_dir.path().join("tracked.bin");
    let grown = write(&tmp_dir, "grown.bin", &LINE.repeat(30));
    let once = saved.resync(&grown).unwrap();

    assert_eq!(once.file.size, 120);

    let error = once.resync(&original).err().unwrap();

    assert!(
        error.to_string().contains("file content differs from the tracked file"),
        "{error}"
    );
    assert!(error.to_string().contains("grown.bin"), "{error}");
    assert!(error.to_string().contains("tracked.bin"), "{error}");
}

#[rstest]
fn resync_is_stable_when_repeated(tmp_dir: TempDir) {
    let saved = recorded(&tmp_dir, &LINE.repeat(25), 5);
    let path = write(&tmp_dir, "tracked.bin", &LINE.repeat(25));
    let once = saved.resync(&path).unwrap();
    let twice = once.resync(&path).unwrap();

    assert_eq!(twice.position, 5);
    assert_eq!(twice.file, once.file);
    twice.verify().unwrap();
}

#[cfg(unix)]
#[rstest]
fn resync_fails_when_the_path_is_not_utf8(tmp_dir: TempDir) {
    let saved = recorded(&tmp_dir, &LINE.repeat(25), 5);
    let odd = tmp_dir.path().join(OsStr::from_bytes(b"data-\xff.bin"));

    if fs::write(&odd, LINE.repeat(25)).is_err() {
        return; // a non-UTF-8 name cannot be created here
    }

    let error = saved.resync(&odd).err().unwrap();

    assert!(error.to_string().contains("invalid UTF-8"), "{error}");
}

#[rstest]
fn resync_reseals_a_tampered_state(tmp_dir: TempDir) {
    let lenient = reader(
        &tmp_dir,
        Config {
            verify_state: false,
            ..Config::default()
        },
    );
    let path = write(&tmp_dir, "data.bin", &b"a".repeat(20));
    let mut state = lenient.bytes(&path).state(TEST_STATE_NAME).build().unwrap().state().clone();
    let payload = state.save().unwrap();
    let tampered = fs::read_to_string(&payload)
        .unwrap()
        .replace("\"position\": 0", "\"position\": 999");

    fs::write(&payload, tampered).unwrap();

    let loaded = lenient.states().load(TEST_STATE_NAME).unwrap();

    assert!(loaded.verify().is_err());

    let resynced = loaded.resync(&path).unwrap();

    assert!(resynced.verify().is_ok());
    assert_eq!(resynced.position, 999);
}

#[rstest]
fn a_resynced_state_is_still_rejected_for_another_file(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let first = write(&tmp_dir, "a.bin", &b"a".repeat(10));
    let second = write(&tmp_dir, "b.bin", &b"b".repeat(10));
    let saved = reader.bytes(&first).state(TEST_STATE_NAME).build().unwrap().state().clone();
    let resynced = saved.resync(&first).unwrap();
    let error = reader.bytes(&second).state(resynced).build().unwrap_err();

    assert!(error.to_string().contains("file path mismatch"), "{error}");
    assert!(
        error.to_string().contains("call state.resync(file)"),
        "{error}"
    );
}

#[rstest]
#[case::directly(false)]
#[case::through_a_symlink(true)]
fn resync_fails_when_the_path_is_a_directory(tmp_dir: TempDir, #[case] linked: bool) {
    use std::os::unix::fs::symlink;

    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", b"foo\n");
    let state = reader.bytes(&path).build().unwrap().state().clone();
    let directory = tmp_dir.path().join("elsewhere");

    fs::create_dir(&directory).unwrap();

    let target = if linked {
        let alias = tmp_dir.path().join("alias");

        symlink(&directory, &alias).unwrap();

        alias
    } else {
        directory
    };
    let error = state.resync(&target).err().unwrap();

    assert!(error.to_string().contains("not a file"), "got {error}");
}

#[rstest]
#[case::missing(false)]
#[case::a_symlink_loop(true)]
fn resync_fails_when_the_path_cannot_be_resolved(tmp_dir: TempDir, #[case] looped: bool) {
    use std::os::unix::fs::symlink;

    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", b"foo\n");
    let state = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap().state().clone();
    let target = if looped {
        let (first, second) = (tmp_dir.path().join("a-link"), tmp_dir.path().join("b-link"));

        symlink(&second, &first).unwrap();
        symlink(&first, &second).unwrap();

        first
    } else {
        tmp_dir.path().join("does-not-exist.bin")
    };
    let error = state.resync(&target).err().unwrap();

    assert!(matches!(&error, Error::Io(_)), "got {error}");
}

#[rstest]
#[case::path_rejected("../../etc/passwd")]
#[case::commit_failed(TEST_STATE_NAME)]
fn save_leaves_the_state_untouched_when_it_fails(tmp_dir: TempDir, #[case] name: &str) {
    let reader = reader(&tmp_dir, Config::default());
    let state_dir = reader.config().state_dir.clone();

    fs::create_dir_all(state_dir.join(format!("{TEST_STATE_NAME}.state.json"))).unwrap();

    let path = write(&tmp_dir, "data.bin", b"foo\n");
    let mut bytes = reader.bytes(&path).state(name).build().unwrap();
    let before = bytes.state().timestamps.updated_at;

    bytes.state().save().unwrap_err();

    assert_eq!(bytes.state().timestamps.updated_at, before);
}

#[rstest]
#[case::when_it_succeeds(false)]
#[case::when_it_fails(true)]
fn save_leaves_no_temporary_file(tmp_dir: TempDir, #[case] blocked: bool) {
    let reader = reader(&tmp_dir, Config::default());
    let state_dir = reader.config().state_dir.clone();

    if blocked {
        fs::create_dir_all(state_dir.join(format!("{TEST_STATE_NAME}.state.json"))).unwrap();
    }

    let path = write(&tmp_dir, "data.bin", b"foo\n");
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();
    let outcome = bytes.state().save();

    assert_eq!(outcome.is_err(), blocked);

    let leftovers: Vec<_> = fs::read_dir(&state_dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().is_some_and(|extension| extension == "tmp"))
        .collect();

    assert!(leftovers.is_empty(), "{leftovers:?}");
}

#[rstest]
fn save_succeeds_after_the_tracked_file_is_deleted(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", b"foo");
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    fs::remove_file(&path).unwrap();

    assert!(bytes.state().save().unwrap().exists());
}

#[rstest]
fn load_fails_when_the_tracked_file_is_deleted(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", b"foo");
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    fs::remove_file(&path).unwrap();
    bytes.state().save().unwrap();
    drop(bytes);

    let error = reader.states().load(TEST_STATE_NAME).err().unwrap();

    assert!(
        matches!(&error, Error::Io(io) if io.kind() == std::io::ErrorKind::NotFound),
        "got {error}"
    );
}

#[rstest]
fn name_drops_the_state_suffix(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", b"foo\n");
    let mut bytes = reader
        .bytes(&path)
        .state(format!("{TEST_STATE_NAME}.state.json"))
        .build()
        .unwrap();

    assert_eq!(bytes.state().name, TEST_STATE_NAME);
    assert_eq!(
        bytes.state().path().unwrap().file_name().unwrap(),
        format!("{TEST_STATE_NAME}.state.json").as_str()
    );
}

#[rstest]
fn save_accepts_a_unicode_name(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", b"foo\n");
    let mut bytes = reader.bytes(&path).state("job-café").build().unwrap();

    assert_eq!(
        bytes.state().save().unwrap().file_name().unwrap(),
        "job-café.state.json"
    );
}

#[rstest]
#[case::past_the_size("\"position\": 3", "\"position\": 999", 33300.0)]
#[case::zero_size("\"size\": 3", "\"size\": 0", 300.0)]
fn percent_reads_the_stored_numbers(
    tmp_dir: TempDir,
    #[case] from: &str,
    #[case] to: &str,
    #[case] expected: f64,
) {
    let reader = unverified(&tmp_dir);
    let path = write(&tmp_dir, "data.bin", b"foo");
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    while bytes.read().unwrap().is_some() {}

    let payload = bytes.state().save().unwrap();

    drop(bytes);

    let patched = fs::read_to_string(&payload).unwrap().replace(from, to);

    assert!(patched.contains(to), "the payload did not contain {from}");
    fs::write(&payload, patched).unwrap();

    assert_eq!(
        reader.states().load(TEST_STATE_NAME).unwrap().percent(),
        expected
    );
}

#[rstest]
fn save_fails_when_the_name_is_too_long(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", b"foo\n");
    let mut bytes = reader.bytes(&path).state("x".repeat(300)).build().unwrap();
    let error = bytes.state().save().err().unwrap();

    assert!(matches!(error, Error::Io(_)), "got {error}");
}

#[rstest]
fn save_does_not_disturb_the_iterator(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = large(&tmp_dir);
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    for _ in 0..5 {
        bytes.read().unwrap();
    }

    bytes.state().save().unwrap();

    assert_eq!(reader.states().load(TEST_STATE_NAME).unwrap().position, 5);

    let mut remaining = 0;

    while bytes.read().unwrap().is_some() {
        remaining += 1;
    }

    assert_eq!(remaining, LARGE * 4 - 5);
}

#[rstest]
fn a_reloaded_state_matches_the_saved_one_to_the_second(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", b"foo\n");
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    bytes.read().unwrap();
    bytes.state().save().unwrap();

    let saved = bytes.state().clone();

    drop(bytes);

    let reloaded = reader.states().load(TEST_STATE_NAME).unwrap();
    let truncated = |stamp: DateTime<chrono::Utc>| stamp.with_nanosecond(0).unwrap();

    // The payload stores whole seconds, so the round trip drops the sub-second
    // part; the checksum still agrees because the digest is computed over the
    // same truncated values on both sides.
    assert_eq!(reloaded.name, saved.name);
    assert_eq!(reloaded.position, saved.position);
    assert_eq!(reloaded.file, saved.file);
    assert_eq!(reloaded.checksum, saved.checksum);
    assert_eq!(
        reloaded.timestamps.created_at,
        truncated(saved.timestamps.created_at)
    );
    assert_eq!(
        reloaded.timestamps.updated_at,
        truncated(saved.timestamps.updated_at)
    );
}

#[rstest]
fn the_saved_payload_has_the_expected_keys(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", b"foo\n");
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();
    let payload = bytes.state().save().unwrap();
    let json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&payload).unwrap()).unwrap();

    let keys = |value: &serde_json::Value| {
        let mut keys: Vec<String> = value.as_object().unwrap().keys().cloned().collect();

        keys.sort();
        keys
    };

    assert_eq!(
        keys(&json),
        ["_checksum", "file", "name", "position", "timestamps"]
    );
    assert_eq!(
        keys(&json["file"]),
        ["fingerprint", "mtime", "path", "size"]
    );
    assert_eq!(keys(&json["timestamps"]), ["created_at", "updated_at"]);
}

#[rstest]
fn the_saved_payload_keeps_the_checksum_last(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", b"foo\n");
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();
    let payload = bytes.state().save().unwrap();
    // The raw text is read rather than a parsed value: a parsed map would only
    // report the file order while serde_json keeps insertion order.
    let text = fs::read_to_string(&payload).unwrap();
    let at = |key: &str| text.find(key).unwrap_or_else(|| panic!("{key} is missing"));

    assert!(at("\"_checksum\"") > at("\"name\""));
    assert!(at("\"_checksum\"") > at("\"file\""));
    assert!(at("\"_checksum\"") > at("\"position\""));
    assert!(at("\"_checksum\"") > at("\"timestamps\""));
}

#[rstest]
fn save_refreshes_the_checksum(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = large(&tmp_dir);
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();
    let before = bytes.state().checksum().unwrap();

    for _ in 0..5 {
        bytes.read().unwrap();
    }

    bytes.state().save().unwrap();
    drop(bytes);

    assert_ne!(
        reader.states().load(TEST_STATE_NAME).unwrap().checksum,
        before
    );
}

#[rstest]
fn a_state_path_is_relative_without_a_state_dir(tmp_dir: TempDir) {
    let reader = preader::PReader::from(Config {
        state_dir: PathBuf::new(),
        ..Config::default()
    });
    let path = write(&tmp_dir, "data.bin", b"foo\n");
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    assert_eq!(
        bytes.state().path().unwrap(),
        PathBuf::from(format!("{TEST_STATE_NAME}.state.json"))
    );
}
