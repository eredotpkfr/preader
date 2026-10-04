use std::path::Path;

use preader::{Error, FINGERPRINT_SAMPLE_BYTES, FileMetadata};
use rstest::rstest;
use serde_json::{Value, json};

use crate::common::{
    constants::{TEST_EMPTY_FINGERPRINT, TEST_LINE, TEST_LINE_FINGERPRINT},
    fixtures::sandbox,
    funcs::digest,
    guards::set_pre_epoch_mtime,
    macros::{asserts::assert_err_is, skip::skip},
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
fn rejects_a_directory(sandbox: Sandbox) {
    let path = sandbox.dir_at("folder");
    let error = FileMetadata::try_from(path.as_path()).unwrap_err();

    assert!(matches!(error, Error::NotAFile(ref found) if *found == path));

    assert_eq!(error.to_string(), format!("not a file: {}", path.display()));
}

#[rstest]
fn rejects_a_missing_file(sandbox: Sandbox) {
    let path = sandbox.path().join("missing.bin");
    let error = FileMetadata::try_from(path.as_path()).unwrap_err();

    assert!(matches!(error, Error::Io(error) if error.kind() == std::io::ErrorKind::NotFound));
}

#[rstest]
fn rejects_a_pre_epoch_mtime(sandbox: Sandbox) {
    let path = sandbox.line_file();

    if !set_pre_epoch_mtime(&path) {
        skip!("a pre-epoch mtime cannot be set here");
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
