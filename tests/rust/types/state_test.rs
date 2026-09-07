#[cfg(unix)]
use std::{ffi::OsStr, os::unix::ffi::OsStrExt};
use std::{fs, path::PathBuf};

use chrono::DateTime;
use preader::{
    Config, Error, FileMetadata, IteratorBuild, State, StateData, StateManager, Timestamps,
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
