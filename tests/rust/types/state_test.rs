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
    let moved = write(&tmp_dir, "moved.bin", b"foobar\n");
    let resynced = state.resync(&moved).unwrap();

    assert_eq!(resynced.file.path, moved.canonicalize().unwrap());
    assert_eq!(resynced.file.size, 7);
    assert_eq!(
        resynced.file.fingerprint,
        preader::fingerprint(&moved).unwrap()
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

#[rstest]
fn resync_keeps_a_position_past_the_new_file_end(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "big.bin", &b"x".repeat(100));
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).limit(50).build().unwrap();

    while bytes.read().unwrap().is_some() {}

    bytes.state().save().unwrap();

    let saved = bytes.state().clone();

    drop(bytes);

    let smaller = write(&tmp_dir, "small.bin", &b"y".repeat(10));
    let resynced = saved.resync(&smaller).unwrap();

    assert_eq!(resynced.position, 50);
    assert_eq!(resynced.file.size, 10);
    assert_eq!(resynced.percent(), 500.0);

    let mut resumed = reader.bytes(&smaller).state(resynced).build().unwrap();

    assert!(resumed.read().unwrap().is_none());
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
fn resync_carries_the_position_onto_an_unrelated_file(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", &b"a".repeat(30));
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).limit(10).build().unwrap();

    while bytes.read().unwrap().is_some() {}

    bytes.state().save().unwrap();

    let saved = bytes.state().clone();

    drop(bytes);

    let unrelated = write(&tmp_dir, "other.bin", &b"Z".repeat(30));
    let resynced = saved.resync(&unrelated).unwrap();

    assert_eq!(resynced.position, 10);

    let resumed = reader.bytes(&unrelated).state(resynced).build().unwrap();
    let collected: Vec<u8> = resumed.map(|byte| byte.unwrap()).collect();

    assert_eq!(collected, b"Z".repeat(20));
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
fn resync_fails_when_the_path_is_a_directory(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", b"foo\n");
    let state = reader.bytes(&path).build().unwrap().state().clone();
    let directory = tmp_dir.path().join("elsewhere");

    fs::create_dir(&directory).unwrap();

    let error = state.resync(&directory).err().unwrap();

    assert!(error.to_string().contains("not a file"), "got {error}");
}

#[rstest]
fn resync_fails_when_the_file_is_missing(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", b"foo\n");
    let state = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap().state().clone();
    let error = state.resync(tmp_dir.path().join("does-not-exist.bin")).err().unwrap();

    assert!(
        matches!(&error, Error::Io(io) if io.kind() == std::io::ErrorKind::NotFound),
        "got {error}"
    );
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
fn save_removes_the_temporary_file_when_it_fails(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let state_dir = reader.config().state_dir.clone();

    fs::create_dir_all(state_dir.join(format!("{TEST_STATE_NAME}.state.json"))).unwrap();

    let path = write(&tmp_dir, "data.bin", b"foo\n");
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    bytes.state().save().unwrap_err();

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
