use std::path::PathBuf;
#[cfg(unix)]
use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

use chrono::DateTime;
use preader::{ChecksumBody, FileMetadata, StateData, Timestamps};
use rstest::{fixture, rstest};
use sha2::{Digest, Sha256};

use crate::common::constants::{TEST_FILE_PATH, TEST_FINGERPRINT, TEST_STATE_NAME};

const PRECOMPUTED_DIGEST: &str = "16065da22c1c78d8f8495f3c0db2d285a411d7f0eba6fc29bc14c2244097216b";
const OTHER_FINGERPRINT: &str = "fcde2b2edba56bf408601fb721fe9b5c338d10ee429ea04fae5511b68fbf8fb9";

const POSITION: u64 = 7;

#[fixture]
fn file() -> FileMetadata {
    FileMetadata {
        path: PathBuf::from(TEST_FILE_PATH),
        size: 4,
        mtime: DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
        fingerprint: TEST_FINGERPRINT.to_string(),
    }
}

#[fixture]
fn timestamps() -> Timestamps {
    Timestamps {
        created_at: DateTime::from_timestamp(1_700_000_001, 0).unwrap(),
        updated_at: DateTime::from_timestamp(1_700_000_002, 0).unwrap(),
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
fn compute_matches_the_precomputed_digest(file: FileMetadata, timestamps: Timestamps) {
    assert_eq!(
        body(&file, &timestamps).compute().unwrap(),
        PRECOMPUTED_DIGEST
    );
}

#[rstest]
fn compute_hashes_the_compact_serialization(file: FileMetadata, timestamps: Timestamps) {
    let body = body(&file, &timestamps);
    let expected = hex::encode(Sha256::digest(serde_json::to_string(&body).unwrap()));

    assert_eq!(body.compute().unwrap(), expected);
}

#[rstest]
#[case::name(|body: &mut ChecksumBody| body.name = "job-2")]
#[case::position(|body: &mut ChecksumBody| body.position = 8)]
fn compute_includes_every_body_field(
    file: FileMetadata,
    timestamps: Timestamps,
    #[case] change: fn(&mut ChecksumBody),
) {
    let mut body = body(&file, &timestamps);

    change(&mut body);

    assert_ne!(body.compute().unwrap(), PRECOMPUTED_DIGEST);
}

#[rstest]
#[case::path(|file: &mut FileMetadata| file.path = PathBuf::from("/tmp/other.bin"))]
#[case::size(|file: &mut FileMetadata| file.size = 5)]
#[case::mtime(|file: &mut FileMetadata| file.mtime = DateTime::from_timestamp(1, 0).unwrap())]
#[case::fingerprint(|file: &mut FileMetadata| file.fingerprint = OTHER_FINGERPRINT.to_string())]
fn compute_includes_every_file_field(
    mut file: FileMetadata,
    timestamps: Timestamps,
    #[case] change: fn(&mut FileMetadata),
) {
    change(&mut file);

    assert_ne!(
        body(&file, &timestamps).compute().unwrap(),
        PRECOMPUTED_DIGEST
    );
}

#[rstest]
#[case::created_at(|timestamps: &mut Timestamps| timestamps.created_at = DateTime::from_timestamp(1, 0).unwrap())]
#[case::updated_at(|timestamps: &mut Timestamps| timestamps.updated_at = DateTime::from_timestamp(1, 0).unwrap())]
fn compute_includes_every_timestamp_field(
    file: FileMetadata,
    mut timestamps: Timestamps,
    #[case] change: fn(&mut Timestamps),
) {
    change(&mut timestamps);

    assert_ne!(
        body(&file, &timestamps).compute().unwrap(),
        PRECOMPUTED_DIGEST
    );
}

#[rstest]
fn compute_ignores_sub_second_precision(file: FileMetadata, mut timestamps: Timestamps) {
    timestamps.updated_at = DateTime::from_timestamp(1_700_000_002, 500_000_000).unwrap();

    assert_eq!(
        body(&file, &timestamps).compute().unwrap(),
        PRECOMPUTED_DIGEST
    );
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
        name: TEST_STATE_NAME.to_string(),
        file,
        position: POSITION,
        timestamps,
        checksum: checksum.to_string(),
    };
    assert_eq!(
        ChecksumBody::from(&data).compute().unwrap(),
        PRECOMPUTED_DIGEST
    );
}

#[cfg(unix)]
#[rstest]
fn compute_fails_when_the_path_is_not_utf8(mut file: FileMetadata, timestamps: Timestamps) {
    file.path = PathBuf::from(OsStr::from_bytes(b"/tmp/data-\xff\xfe.bin"));

    let error = body(&file, &timestamps).compute().unwrap_err();

    assert!(error.to_string().contains("invalid UTF-8"));
}
