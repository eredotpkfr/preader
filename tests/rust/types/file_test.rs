use std::{fs, io::ErrorKind, path::Path, time::Duration};
#[cfg(unix)]
use std::{os::unix::net::UnixListener, process::Command};

use preader::{Error, FileMetadata};
use rstest::rstest;
use serde_json::{Value, error::Category, json};
use time::UtcDateTime;

use crate::common::{
    constants::{
        TEST_EMPTY_FINGERPRINT, TEST_LINE, TEST_LINE_FINGERPRINT, TEST_PRE_EPOCH, TEST_WINDOW,
    },
    fixtures::sandbox,
    funcs::{digest, items},
    guards::{mtime, set_pre_epoch_mtime},
    macros::asserts::assert_err_is,
    sandbox::Sandbox,
};
#[cfg(unix)]
use crate::common::{guards::Blocked, macros::asserts::assert_err_eq};

#[rstest]
fn fields(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let metadata = FileMetadata::try_from(&path).unwrap();

    assert_eq!(metadata.path, path);
    assert_eq!(metadata.size, TEST_LINE.len() as u64);
    assert_eq!(metadata.fingerprint, TEST_LINE_FINGERPRINT);
    assert_eq!(
        metadata.mtime,
        UtcDateTime::from(mtime(&path)).truncate_to_microsecond()
    );
}

#[rstest]
fn compares_by_value(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let metadata = FileMetadata::try_from(&path).unwrap();

    assert_eq!(metadata, FileMetadata::try_from(&path).unwrap());

    assert_ne!(
        metadata,
        FileMetadata::try_from(&sandbox.large_file()).unwrap()
    );
}

#[rstest]
fn fingerprints_an_empty_file(sandbox: Sandbox) {
    let metadata = FileMetadata::try_from(&sandbox.empty_file()).unwrap();

    assert_eq!(metadata.size, 0);
    assert_eq!(metadata.fingerprint, TEST_EMPTY_FINGERPRINT);
}

#[rstest]
fn fails_when_the_path_is_a_directory(sandbox: Sandbox) {
    let path = sandbox.dir_at("folder");

    assert_err_is!(
        FileMetadata::try_from(&path),
        Error::Io(error) if error.kind() == ErrorKind::IsADirectory
    );
}

#[rstest]
fn fails_when_the_file_is_missing(sandbox: Sandbox) {
    let path = sandbox.path().join("missing.bin");

    assert_err_is!(
        FileMetadata::try_from(&path),
        Error::Io(error) if error.kind() == ErrorKind::NotFound
    );
}

#[rstest]
#[case::whole_seconds(Duration::from_secs(TEST_PRE_EPOCH.unsigned_abs()), TEST_PRE_EPOCH, 0)]
#[case::part_of_a_second(Duration::from_millis(500), -1, 500_000)]
fn reads_an_mtime_before_the_epoch(
    sandbox: Sandbox,
    #[case] before: Duration,
    #[case] seconds: i64,
    #[case] micros: u32,
) {
    let path = sandbox.line_file();

    if !set_pre_epoch_mtime(&path, before) {
        return;
    }

    let metadata = FileMetadata::try_from(&path).unwrap();

    assert_eq!(metadata.mtime.unix_timestamp(), seconds);
    assert_eq!(metadata.mtime.microsecond(), micros);
}

#[rstest]
fn fingerprint_ignores_bytes_past_the_window(sandbox: Sandbox) {
    let path = sandbox.large_file();
    let metadata = FileMetadata::try_from(&path).unwrap();

    assert!(metadata.size > TEST_WINDOW as u64);

    assert_eq!(
        metadata.fingerprint,
        digest(&fs::read(&path).unwrap()[..TEST_WINDOW])
    );
}

#[rstest]
fn reads_a_relative_path() {
    let metadata = FileMetadata::try_from(Path::new("Cargo.toml")).unwrap();

    assert_eq!(metadata.path, Path::new("Cargo.toml"));
    assert_eq!(metadata.size, fs::metadata("Cargo.toml").unwrap().len());
}

#[cfg(unix)]
#[rstest]
fn fails_when_the_path_is_a_socket(sandbox: Sandbox) {
    let socket = sandbox.path().join("a-socket");
    let _listener = UnixListener::bind(&socket).unwrap();

    assert_err_is!(
        sandbox.reader().bytes(&socket).build(),
        Error::Io(error) if error.kind() == ErrorKind::InvalidInput
    );
}

#[rstest]
fn deserialization_fails_when_the_mtime_is_not_an_integer() {
    let payload = json!({
        "path": "foo.bin",
        "size": 4,
        "mtime": "foo",
        "fingerprint": TEST_LINE_FINGERPRINT,
    });
    let error = serde_json::from_value::<FileMetadata>(payload).unwrap_err();

    assert_eq!(error.classify(), Category::Data);
}

#[rstest]
fn records_an_mtime_before_the_epoch(sandbox: Sandbox) {
    let path = sandbox.line_file();

    if !set_pre_epoch_mtime(&path, Duration::from_secs(TEST_PRE_EPOCH.unsigned_abs())) {
        return;
    }

    let bytes = sandbox.reader().bytes(&path).build().unwrap();

    assert_eq!(bytes.state().file.mtime.unix_timestamp(), TEST_PRE_EPOCH);
    assert_eq!(items(bytes), TEST_LINE);
}

#[cfg(unix)]
#[rstest]
fn fails_when_the_file_is_unreadable(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let mut blocked = Blocked::default();

    if !Blocked::enforced(&sandbox.path().join("probe")) {
        return;
    }

    blocked.block(&path);

    assert_err_is!(
        sandbox.reader().bytes(&path).build(),
        Error::Io(error) if error.kind() == ErrorKind::PermissionDenied
    );
}

#[cfg(unix)]
#[rstest]
fn fails_when_the_path_is_a_fifo(sandbox: Sandbox) {
    let fifo = sandbox.path().join("a-fifo");

    if !Command::new("mkfifo").arg(&fifo).status().is_ok_and(|status| status.success()) {
        return;
    }

    assert_err_eq!(sandbox.reader().bytes(&fifo).build(), "not a regular file");

    assert_err_is!(
        sandbox.reader().bytes(&fifo).build(),
        Error::Io(error) if error.kind() == ErrorKind::InvalidInput
    );
}

#[rstest]
fn serializes_the_mtime_as_unix_microseconds(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let metadata = FileMetadata::try_from(&path).unwrap();
    let payload: Value = serde_json::to_value(&metadata).unwrap();

    assert_eq!(
        payload,
        json!({
            "path": path.to_str().unwrap(),
            "size": metadata.size,
            "mtime": i64::try_from(metadata.mtime.unix_timestamp_nanos() / 1_000).unwrap(),
            "fingerprint": TEST_LINE_FINGERPRINT,
        })
    );
    assert_eq!(
        serde_json::from_value::<FileMetadata>(payload).unwrap(),
        metadata
    );
}
