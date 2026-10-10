use std::path::PathBuf;

use preader::{Error, Mismatch};
use rstest::rstest;

const RESYNC_HINT: &str = "(call state.resync(file) if this is expected)";
const RESTART_HINT: &str = "(read it under a new state to start over)";

fn checksum() -> Mismatch {
    Mismatch::Checksum {
        saved: "foo".to_owned(),
        computed: "bar".to_owned(),
    }
}

fn paths() -> (PathBuf, PathBuf) {
    (
        PathBuf::from("/tmp/saved.bin"),
        PathBuf::from("/tmp/current.bin"),
    )
}

#[rstest]
fn checksum_reports_both_digests() {
    assert_eq!(
        checksum().to_string(),
        "state checksum mismatch (saved: foo, computed: bar)"
    );
}

#[rstest]
fn path_suggests_a_resync() {
    let (saved, current) = paths();
    let message = Mismatch::Path { saved, current }.to_string();

    assert_eq!(
        message,
        format!(
            "file path mismatch (saved: '/tmp/saved.bin', current: '/tmp/current.bin') {RESYNC_HINT}"
        )
    );
}

#[rstest]
fn size_suggests_a_resync() {
    let message = Mismatch::Size {
        saved: 10,
        current: 20,
    }
    .to_string();

    assert_eq!(
        message,
        format!("file size mismatch (saved: 10, current: 20) {RESYNC_HINT}")
    );
}

#[rstest]
fn mtime_suggests_a_resync() {
    let message = Mismatch::Mtime {
        saved: 1,
        current: 2,
    }
    .to_string();

    assert_eq!(
        message,
        format!("file mtime mismatch (saved: 1, current: 2) {RESYNC_HINT}")
    );
}

#[rstest]
fn fingerprint_suggests_a_resync() {
    let message = Mismatch::Fingerprint {
        saved: "foo".to_owned(),
        current: "bar".to_owned(),
    }
    .to_string();

    assert_eq!(
        message,
        format!("file fingerprint mismatch (saved: foo, current: bar) {RESYNC_HINT}")
    );
}

#[rstest]
fn identity_suggests_a_restart() {
    let (saved, current) = paths();
    let message = Mismatch::Identity { saved, current }.to_string();

    assert_eq!(
        message,
        format!(
            "file content differs from the tracked file (saved: '/tmp/saved.bin', current: '/tmp/current.bin') {RESTART_HINT}"
        )
    );
}

#[rstest]
fn name_suggests_a_restart() {
    let message = Mismatch::Name {
        saved: "job-2".to_owned(),
        current: "job-1".to_owned(),
    }
    .to_string();

    assert_eq!(
        message,
        format!("state name mismatch (saved: 'job-2', current: 'job-1') {RESTART_HINT}")
    );
}

#[rstest]
fn mismatch_converts_into_an_error() {
    let error = Error::from(checksum());

    assert!(matches!(error, Error::Mismatch(Mismatch::Checksum { .. })));

    assert_eq!(error.to_string(), checksum().to_string());
}
