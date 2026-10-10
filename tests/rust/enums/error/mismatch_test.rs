use preader::{Error, Mismatch};
use rstest::rstest;

const RESYNC_HINT: &str = "(call state.resync(file) if this is expected)";
const RESTART_HINT: &str = "(read it under a new state to start over)";

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
        format!("state tracks a different file {RESYNC_HINT}")
    );
}

#[rstest]
fn size_suggests_a_resync() {
    assert_eq!(
        Mismatch::Size.to_string(),
        format!("file size changed {RESYNC_HINT}")
    );
}

#[rstest]
fn mtime_suggests_a_resync() {
    assert_eq!(
        Mismatch::Mtime.to_string(),
        format!("file mtime changed {RESYNC_HINT}")
    );
}

#[rstest]
fn fingerprint_suggests_a_resync() {
    assert_eq!(
        Mismatch::Fingerprint.to_string(),
        format!("file content changed {RESYNC_HINT}")
    );
}

#[rstest]
fn identity_suggests_a_restart() {
    assert_eq!(
        Mismatch::Identity.to_string(),
        format!("file is not the tracked file {RESTART_HINT}")
    );
}

#[rstest]
fn name_reports_the_owning_state() {
    let message = Mismatch::Name {
        saved: "job-2".to_owned(),
    }
    .to_string();

    assert_eq!(message, "state file belongs to 'job-2'");
}

#[rstest]
fn mismatch_converts_into_an_error() {
    let error = Error::from(Mismatch::Checksum);

    assert!(matches!(error, Error::Mismatch(Mismatch::Checksum)));

    assert_eq!(error.to_string(), Mismatch::Checksum.to_string());
}
