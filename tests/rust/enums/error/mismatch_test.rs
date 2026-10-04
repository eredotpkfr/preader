use std::path::PathBuf;

use preader::{Error, Mismatch};
use rstest::rstest;

const RESYNC_HINT: &str = "(call state.resync(file) if this is expected)";
const RESTART_HINT: &str = "(read it under a new state to start over)";

fn checksum() -> Mismatch {
    Mismatch::Checksum {
        saved: "aaa".to_owned(),
        computed: "bbb".to_owned(),
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
        "state checksum mismatch (saved: aaa, computed: bbb)"
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
        saved: "aaa".to_owned(),
        current: "bbb".to_owned(),
    }
    .to_string();

    assert_eq!(
        message,
        format!("file fingerprint mismatch (saved: aaa, current: bbb) {RESYNC_HINT}")
    );
}

#[rstest]
fn identity_suggests_a_restart() {
    let (saved, current) = paths();
    let message = Mismatch::Identity { saved, current }.to_string();

    assert!(message.starts_with("file content differs from the tracked file"));
    assert!(message.ends_with(RESTART_HINT));
    assert!(!message.contains(RESYNC_HINT));
}

#[rstest]
fn a_mismatch_converts_into_an_error() {
    let error = Error::from(checksum());

    assert!(matches!(error, Error::Mismatch(Mismatch::Checksum { .. })));
    assert_eq!(error.to_string(), checksum().to_string());
}
