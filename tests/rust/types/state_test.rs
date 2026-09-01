#[cfg(unix)]
use std::{ffi::OsStr, os::unix::ffi::OsStrExt, path::PathBuf};

#[cfg(unix)]
use chrono::DateTime;
#[cfg(unix)]
use preader::{Config, FileMetadata, State, StateData, StateManager, Timestamps};
#[cfg(unix)]
use rstest::rstest;
#[cfg(unix)]
use tempfile::TempDir;

#[cfg(unix)]
use crate::common::{
    constants::{TEST_FINGERPRINT, TEST_STATE_NAME},
    fixtures::tmp_dir,
};

#[cfg(unix)]
const NON_UTF8_PATH: &[u8] = b"/tmp/data-\xff.bin";

#[cfg(unix)]
fn state_in(tmp_dir: &TempDir) -> State {
    let stamp = DateTime::from_timestamp(1_700_000_000, 0).unwrap();
    let manager = StateManager::from(&Config {
        state_dir: tmp_dir.path().join("preader"),
        ..Config::default()
    });
    let data = StateData {
        name: TEST_STATE_NAME.to_string(),
        file: FileMetadata {
            path: PathBuf::from(OsStr::from_bytes(NON_UTF8_PATH)),
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
    };

    State::from((data, manager))
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
