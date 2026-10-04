use std::path::PathBuf;
#[cfg(unix)]
use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

use chrono::DateTime;
use preader::{ChecksumBody, FileMetadata, StateData, Timestamps};
use rstest::{fixture, rstest};
use sha2::{Digest, Sha256};

use crate::common::constants::{
    TEST_EMPTY_FINGERPRINT, TEST_FILE_PATH, TEST_LINE, TEST_LINE_FINGERPRINT,
    TEST_OTHER_STATE_NAME, TEST_STAMP, TEST_STATE_NAME,
};
#[cfg(unix)]
use crate::common::macros::asserts::assert_err;

const DIGEST: &str = "f74d122580787555a6f2d245be2066bbe5af3e83fe5e39092c479e0e1b41225a";
const POSITION: u64 = 7;

#[fixture]
fn file() -> FileMetadata {
    FileMetadata {
        path: PathBuf::from(TEST_FILE_PATH),
        size: TEST_LINE.len() as u64,
        mtime: DateTime::from_timestamp(TEST_STAMP, 0).unwrap(),
        fingerprint: TEST_LINE_FINGERPRINT.to_owned(),
    }
}

#[fixture]
fn timestamps() -> Timestamps {
    Timestamps {
        created_at: DateTime::from_timestamp(TEST_STAMP + 1, 0).unwrap(),
        updated_at: DateTime::from_timestamp(TEST_STAMP + 2, 0).unwrap(),
    }
}

fn body<'a>(file: &'a FileMetadata, timestamps: &'a Timestamps) -> ChecksumBody<'a> {
    ChecksumBody {
        name: TEST_STATE_NAME,
        file,
        position: POSITION,
        timestamps,
    }
}

#[rstest]
fn compute_matches_a_known_digest(file: FileMetadata, timestamps: Timestamps) {
    assert_eq!(body(&file, &timestamps).compute().unwrap(), DIGEST);
}

#[rstest]
fn compute_hashes_the_compact_json(file: FileMetadata, timestamps: Timestamps) {
    let body = body(&file, &timestamps);
    let expected = hex::encode(Sha256::digest(serde_json::to_string(&body).unwrap()));

    assert_eq!(body.compute().unwrap(), expected);
}

#[rstest]
#[case::name(|body: &mut ChecksumBody| body.name = TEST_OTHER_STATE_NAME)]
#[case::position(|body: &mut ChecksumBody| body.position = POSITION + 1)]
fn compute_covers_every_body_field(
    file: FileMetadata,
    timestamps: Timestamps,
    #[case] change: fn(&mut ChecksumBody),
) {
    let mut body = body(&file, &timestamps);

    change(&mut body);

    assert_ne!(body.compute().unwrap(), DIGEST);
}

#[rstest]
#[case::path(|file: &mut FileMetadata| file.path = PathBuf::from("/tmp/other.bin"))]
#[case::size(|file: &mut FileMetadata| file.size += 1)]
#[case::mtime(|file: &mut FileMetadata| file.mtime = DateTime::from_timestamp(1, 0).unwrap())]
#[case::fingerprint(|file: &mut FileMetadata| file.fingerprint = TEST_EMPTY_FINGERPRINT.to_owned())]
fn compute_covers_every_file_field(
    mut file: FileMetadata,
    timestamps: Timestamps,
    #[case] change: fn(&mut FileMetadata),
) {
    change(&mut file);

    assert_ne!(body(&file, &timestamps).compute().unwrap(), DIGEST);
}

#[rstest]
#[case::created_at(|stamps: &mut Timestamps| stamps.created_at = DateTime::from_timestamp(1, 0).unwrap())]
#[case::updated_at(|stamps: &mut Timestamps| stamps.updated_at = DateTime::from_timestamp(1, 0).unwrap())]
fn compute_covers_every_timestamp(
    file: FileMetadata,
    mut timestamps: Timestamps,
    #[case] change: fn(&mut Timestamps),
) {
    change(&mut timestamps);

    assert_ne!(body(&file, &timestamps).compute().unwrap(), DIGEST);
}

#[rstest]
fn compute_ignores_sub_second_precision(file: FileMetadata, mut timestamps: Timestamps) {
    timestamps.updated_at = DateTime::from_timestamp(TEST_STAMP + 2, 500_000_000).unwrap();

    assert_eq!(body(&file, &timestamps).compute().unwrap(), DIGEST);
}

#[rstest]
#[case::empty("")]
#[case::stale("not-a-real-checksum")]
fn compute_ignores_the_stored_checksum(
    file: FileMetadata,
    timestamps: Timestamps,
    #[case] checksum: &str,
) {
    let data = StateData {
        name: TEST_STATE_NAME.to_owned(),
        file,
        position: POSITION,
        timestamps,
        checksum: checksum.to_owned(),
    };

    assert_eq!(ChecksumBody::from(&data).compute().unwrap(), DIGEST);
}

#[cfg(unix)]
#[rstest]
fn compute_fails_when_the_path_is_not_utf8(mut file: FileMetadata, timestamps: Timestamps) {
    file.path = PathBuf::from(OsStr::from_bytes(b"/tmp/data-\xff\xfe.bin"));

    assert_err!(body(&file, &timestamps).compute(), "invalid UTF-8");
}
