use preader::{Error, Mismatch};
use rstest::rstest;

use crate::common::constants::{TEST_OTHER_STATE_NAME, TEST_STATE_NAME};

#[rstest]
fn checksum_reports_an_outside_change() {
    assert_eq!(
        Mismatch::Checksum.to_string(),
        "state file was modified outside preader"
    );
}

#[rstest]
fn path_suggests_a_resync() {
    assert_eq!(
        Mismatch::Path.to_string(),
        "file path changed (resync the state if the file moved)"
    );
}

#[rstest]
fn size_suggests_a_resync() {
    assert_eq!(
        Mismatch::Size.to_string(),
        "file size changed (resync the state if this is expected)"
    );
}

#[rstest]
fn mtime_suggests_a_resync() {
    assert_eq!(
        Mismatch::Mtime.to_string(),
        "file mtime changed (resync the state if this is expected)"
    );
}

#[rstest]
fn missing_suggests_a_resync() {
    assert_eq!(
        Mismatch::Missing.to_string(),
        "file is missing (resync the state if the file moved)"
    );
}

#[rstest]
fn identity_suggests_a_restart() {
    assert_eq!(
        Mismatch::Identity.to_string(),
        "file is not the tracked file (start over under a new state)"
    );
}

#[rstest]
fn unsealed_points_to_the_saved_state() {
    assert_eq!(
        Mismatch::Unsealed.to_string(),
        "state object has unsaved progress or was modified outside preader \
         (pass the state returned by save())"
    );
}

#[rstest]
fn stale_points_to_a_new_iterator() {
    assert_eq!(
        Mismatch::Stale.to_string(),
        "state file was saved elsewhere since this iterator last loaded or saved it \
         (open a new iterator to continue)"
    );
}

#[rstest]
fn name_reports_the_owning_state() {
    let message = Mismatch::Name {
        saved: TEST_OTHER_STATE_NAME.to_owned(),
    }
    .to_string();

    assert_eq!(
        message,
        format!("state file belongs to '{TEST_OTHER_STATE_NAME}'")
    );
}

#[rstest]
fn mismatch_names_the_state() {
    let error = Error::Mismatch {
        name: TEST_STATE_NAME.to_owned(),
        kind: Mismatch::Checksum,
    };

    assert_eq!(
        error.to_string(),
        format!("state '{TEST_STATE_NAME}': {}", Mismatch::Checksum)
    );
}
