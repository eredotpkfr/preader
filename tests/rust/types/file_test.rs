use std::path::Path;

use preader::{Error, FINGERPRINT_SAMPLE_BYTES, FileMetadata};
use rstest::rstest;
use serde_json::{Value, json};

use crate::common::{
    constants::{TEST_EMPTY_FINGERPRINT, TEST_LINE, TEST_LINE_FINGERPRINT},
    fixtures::sandbox,
    funcs::digest,
    guards::set_pre_epoch_mtime,
    macros::asserts::assert_err_is,
    sandbox::Sandbox,
};

#[rstest]
fn reads_the_file_facts(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let metadata = FileMetadata::try_from(path.as_path()).unwrap();

    assert_eq!(metadata.path, path);
    assert_eq!(metadata.size, TEST_LINE.len() as u64);
    assert_eq!(metadata.fingerprint, TEST_LINE_FINGERPRINT);
    assert_eq!(metadata.mtime.timestamp_subsec_nanos(), 0);
}

#[rstest]
fn fingerprints_an_empty_file(sandbox: Sandbox) {
    let metadata = FileMetadata::try_from(sandbox.empty_file().as_path()).unwrap();

    assert_eq!(metadata.size, 0);
    assert_eq!(metadata.fingerprint, TEST_EMPTY_FINGERPRINT);
}

#[rstest]
fn fingerprint_ignores_bytes_past_the_window(sandbox: Sandbox) {
    let path = sandbox.large_file();
    let metadata = FileMetadata::try_from(path.as_path()).unwrap();
    let window = FINGERPRINT_SAMPLE_BYTES as usize;

    assert!(metadata.size > FINGERPRINT_SAMPLE_BYTES);

    assert_eq!(
        metadata.fingerprint,
        digest(&TEST_LINE.repeat(window)[..window])
    );
}

#[rstest]
fn compares_by_value(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let metadata = FileMetadata::try_from(path.as_path()).unwrap();

    assert_eq!(metadata, FileMetadata::try_from(path.as_path()).unwrap());

    assert_ne!(
        metadata,
        FileMetadata::try_from(sandbox.large_file().as_path()).unwrap()
    );
}

#[rstest]
fn fails_when_the_path_is_a_directory(sandbox: Sandbox) {
    let path = sandbox.dir_at("folder");

    assert_err_is!(FileMetadata::try_from(path.as_path()), Error::NotAFile(found) if *found == path);

    assert_err_is!(
        FileMetadata::try_from(path.as_path()),
        Error::NotAFile(found) if *found == path
    );
}

#[rstest]
fn fails_when_the_file_is_missing(sandbox: Sandbox) {
    let path = sandbox.path().join("missing.bin");

    assert_err_is!(
        FileMetadata::try_from(path.as_path()),
        Error::Io(error) if error.kind() == std::io::ErrorKind::NotFound
    );
}

#[rstest]
fn fails_when_the_mtime_precedes_the_epoch(sandbox: Sandbox) {
    let path = sandbox.line_file();

    if !set_pre_epoch_mtime(&path) {
        return;
    }

    assert_err_is!(FileMetadata::try_from(path.as_path()), Error::Time(_));
}

#[rstest]
fn serializes_the_mtime_as_unix_seconds(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let metadata = FileMetadata::try_from(path.as_path()).unwrap();
    let payload: Value = serde_json::to_value(&metadata).unwrap();

    assert_eq!(
        payload,
        json!({
            "path": path.to_str().unwrap(),
            "size": metadata.size,
            "mtime": metadata.mtime.timestamp(),
            "fingerprint": TEST_LINE_FINGERPRINT,
        })
    );
    assert_eq!(
        serde_json::from_value::<FileMetadata>(payload).unwrap(),
        metadata
    );
}

#[rstest]
fn reads_a_relative_path(sandbox: Sandbox) {
    sandbox.line_file();

    let metadata = FileMetadata::try_from(Path::new("Cargo.toml")).unwrap();

    assert!(metadata.size > 0);

    assert_eq!(metadata.path, Path::new("Cargo.toml"));
}
