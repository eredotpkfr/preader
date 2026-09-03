use std::path::PathBuf;
#[cfg(unix)]
use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

use chrono::DateTime;
#[cfg(unix)]
use preader::Config;
use preader::{FileMetadata, State, StateData, StateManager, Timestamps};
#[cfg(unix)]
use rstest::rstest;
#[cfg(unix)]
use tempfile::TempDir;

use crate::common::constants::{TEST_FILE_PATH, TEST_FINGERPRINT, TEST_STATE_NAME};
#[cfg(unix)]
use crate::common::fixtures::tmp_dir;

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
    let mut one = State::from((
        state_data(PathBuf::from(TEST_FILE_PATH)),
        StateManager::default(),
    ));
    let other = State::from((
        state_data(PathBuf::from(TEST_FILE_PATH)),
        StateManager::default(),
    ));

    one.manager.last_saved_position = 100;

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
