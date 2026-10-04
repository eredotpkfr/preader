#[cfg(unix)]
use std::os::unix::fs::symlink;
use std::{
    fs::File,
    io::{ErrorKind, Seek},
};

use preader::{FINGERPRINT_SAMPLE_BYTES, fingerprint, starts_mid_item};
use rstest::rstest;

use crate::common::{
    constants::{
        TEST_BLANK_LINE_CONTENT, TEST_EMPTY_FINGERPRINT, TEST_FOO_FINGERPRINT,
        TEST_UNSEEKABLE_POSITION, TEST_WINDOW_FINGERPRINT,
    },
    fixtures::sandbox,
    sandbox::Sandbox,
};

const WINDOW: usize = FINGERPRINT_SAMPLE_BYTES as usize;
#[rstest]
#[case::plain_content(b"foo".to_vec(), TEST_FOO_FINGERPRINT)]
#[case::empty_file(Vec::new(), TEST_EMPTY_FINGERPRINT)]
#[case::exactly_the_window(vec![b'a'; WINDOW], TEST_WINDOW_FINGERPRINT)]
#[case::just_past_the_window(vec![b'a'; WINDOW + 1], TEST_WINDOW_FINGERPRINT)]
fn fingerprint_digests_the_first_window(
    sandbox: Sandbox,
    #[case] content: Vec<u8>,
    #[case] expected: &str,
) {
    let path = sandbox.file(&content);

    assert_eq!(
        fingerprint(&path, FINGERPRINT_SAMPLE_BYTES).unwrap(),
        expected
    );
}

#[rstest]
fn fingerprint_ignores_content_past_window(sandbox: Sandbox) {
    let window = vec![b'a'; WINDOW];
    let shorter = sandbox.write("shorter.bin", &[&window, &b"x"[..]].concat());
    let longer = sandbox.write("longer.bin", &[&window, &b"y"[..]].concat());

    assert_eq!(
        fingerprint(&shorter, FINGERPRINT_SAMPLE_BYTES).unwrap(),
        fingerprint(&longer, FINGERPRINT_SAMPLE_BYTES).unwrap()
    );
}

#[rstest]
#[case::whole_file(3, TEST_FOO_FINGERPRINT)]
#[case::no_bytes(0, TEST_EMPTY_FINGERPRINT)]
fn fingerprint_honours_the_window(sandbox: Sandbox, #[case] window: u64, #[case] expected: &str) {
    let path = sandbox.file(b"foo");

    assert_eq!(fingerprint(&path, window).unwrap(), expected);
}

#[cfg(unix)]
#[rstest]
fn fingerprint_follows_a_symlink(sandbox: Sandbox) {
    let target = sandbox.file(b"foo");
    let link = sandbox.path().join("link.bin");

    symlink(&target, &link).unwrap();

    assert_eq!(
        fingerprint(&link, FINGERPRINT_SAMPLE_BYTES).unwrap(),
        TEST_FOO_FINGERPRINT
    );
}

#[rstest]
fn fingerprint_fails_when_the_file_is_missing(sandbox: Sandbox) {
    let missing = sandbox.path().join("missing.bin");
    let error = fingerprint(&missing, FINGERPRINT_SAMPLE_BYTES).unwrap_err();

    assert_eq!(error.kind(), ErrorKind::NotFound);
}

#[rstest]
fn fingerprint_fails_when_path_is_directory(sandbox: Sandbox) {
    let error = fingerprint(sandbox.path(), FINGERPRINT_SAMPLE_BYTES).unwrap_err();

    assert!(matches!(
        error.kind(),
        ErrorKind::IsADirectory | ErrorKind::PermissionDenied
    ));
}

#[rstest]
#[case::at_the_start_of_the_file(0, false)]
#[case::on_a_boundary(8, false)]
#[case::inside_an_item(9, true)]
#[case::at_the_end_of_the_file(15, false)]
#[case::past_the_end_of_the_file(16, false)]
fn starts_mid_item_detects_unaligned_position(
    sandbox: Sandbox,
    #[case] position: u64,
    #[case] expected: bool,
) {
    let path = sandbox.file(TEST_BLANK_LINE_CONTENT);
    let file = File::open(path).unwrap();

    assert_eq!(starts_mid_item(&file, position, b'\n').unwrap(), expected);
}

#[rstest]
#[case::inside_the_file(2)]
#[case::at_the_file_end(3)]
#[case::past_the_file_end(9)]
fn starts_mid_item_restores_the_cursor(sandbox: Sandbox, #[case] position: u64) {
    let mut file = File::open(sandbox.file(b"foo")).unwrap();

    starts_mid_item(&file, position, b'\n').unwrap();

    assert_eq!(file.stream_position().unwrap(), position);
}

#[rstest]
#[case::restoring_the_cursor(TEST_UNSEEKABLE_POSITION)]
#[case::reading_the_previous_byte(TEST_UNSEEKABLE_POSITION + 1)]
fn starts_mid_item_fails_when_position_is_unseekable(sandbox: Sandbox, #[case] position: u64) {
    let file = File::open(sandbox.file(b"foo")).unwrap();
    let error = starts_mid_item(&file, position, b'\n').unwrap_err();

    assert!(error.raw_os_error().is_some());
}

#[cfg(unix)]
#[rstest]
fn starts_mid_item_fails_when_file_is_directory(sandbox: Sandbox) {
    let directory = File::open(sandbox.path()).unwrap();
    let error = starts_mid_item(&directory, 1, b'\n').unwrap_err();

    assert_eq!(error.kind(), ErrorKind::IsADirectory);
}
