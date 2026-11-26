use std::{
    fs,
    time::{Duration, UNIX_EPOCH},
};

use rstest::rstest;
use time::UtcDateTime;

use crate::common::{
    constants::{TEST_PRE_EPOCH, TEST_STAMP},
    fixtures::sandbox,
    guards::{set_mtime, set_pre_epoch_mtime},
    sandbox::Sandbox,
};

#[rstest]
fn now_is_truncated_to_whole_seconds() {
    let before = UtcDateTime::now().truncate_to_second();
    let now = preader::time::now();
    let after = UtcDateTime::now();

    assert!(before <= now);
    assert!(now <= after);

    assert_eq!(now.nanosecond(), 0);
}

#[rstest]
#[case::whole_seconds(Duration::from_secs(TEST_PRE_EPOCH.unsigned_abs()), TEST_PRE_EPOCH, 0)]
#[case::part_of_a_second(Duration::from_millis(500), -1, 500_000)]
fn mtime_reads_a_time_before_the_epoch(
    sandbox: Sandbox,
    #[case] before: Duration,
    #[case] seconds: i64,
    #[case] micros: u32,
) {
    let path = sandbox.line_file();

    if !set_pre_epoch_mtime(&path, before) {
        return;
    }

    let read = preader::time::mtime(&fs::metadata(&path).unwrap()).unwrap();

    assert_eq!(read.unix_timestamp(), seconds);
    assert_eq!(read.microsecond(), micros);
}

#[rstest]
fn mtime_is_truncated_to_microseconds(sandbox: Sandbox) {
    let path = sandbox.line_file();

    set_mtime(
        &path,
        UNIX_EPOCH + Duration::new(TEST_STAMP.cast_unsigned(), 123_456_789),
    );

    let read = preader::time::mtime(&fs::metadata(&path).unwrap()).unwrap();

    assert_eq!(read.unix_timestamp(), TEST_STAMP);
    assert_eq!(read.nanosecond(), 123_456_000);
}
