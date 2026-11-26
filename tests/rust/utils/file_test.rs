#[cfg(unix)]
use std::os::unix::fs::symlink;
use std::{
    fs::{self, File},
    io::{ErrorKind, Seek},
    path::PathBuf,
};

use preader::{TMP_FILE_EXTENSION, file};
use rstest::rstest;

use crate::common::{
    constants::{
        TEST_BLANK_LINE_CONTENT, TEST_EMPTY_FINGERPRINT, TEST_STATE_FILE, TEST_UNSEEKABLE_POSITION,
        TEST_WINDOW,
    },
    fixtures::sandbox,
    macros::asserts::assert_err_is,
    sandbox::Sandbox,
};

const WINDOW_FINGERPRINT: &str = "c93eee2d0db02f10acc7460d9576e122dcf8cd53c4bf8dfcae1b3e74ebcfff5a";
const FOO_FINGERPRINT: &str = "2c26b46b68ffc68ff99b453c1d30413413422d706483bfa0f98a5e886266e7ae";

fn tmp(sandbox: &Sandbox) -> PathBuf {
    sandbox.path().join(format!("{TEST_STATE_FILE}.{TMP_FILE_EXTENSION}"))
}

#[rstest]
#[case::plain_content(b"foo".to_vec(), FOO_FINGERPRINT)]
#[case::empty_file(Vec::new(), TEST_EMPTY_FINGERPRINT)]
#[case::exactly_the_window(vec![b'a'; TEST_WINDOW], WINDOW_FINGERPRINT)]
#[case::just_past_the_window(vec![b'a'; TEST_WINDOW + 1], WINDOW_FINGERPRINT)]
fn fingerprint_digests_the_first_window(
    sandbox: Sandbox,
    #[case] content: Vec<u8>,
    #[case] expected: &str,
) {
    let path = sandbox.file(&content);

    assert_eq!(
        file::fingerprint(&path, TEST_WINDOW as u64).unwrap(),
        expected
    );
}

#[rstest]
#[case::whole_file(3, FOO_FINGERPRINT)]
#[case::no_bytes(0, TEST_EMPTY_FINGERPRINT)]
fn fingerprint_honours_the_window(sandbox: Sandbox, #[case] window: u64, #[case] expected: &str) {
    let path = sandbox.file(b"foo");

    assert_eq!(file::fingerprint(&path, window).unwrap(), expected);
}

#[rstest]
fn fingerprint_fails_when_the_path_is_a_directory(sandbox: Sandbox) {
    assert_err_is!(
        file::fingerprint(sandbox.path(), TEST_WINDOW as u64),
        error if matches!(error.kind(), ErrorKind::IsADirectory | ErrorKind::PermissionDenied)
    );
}

#[rstest]
fn fingerprint_fails_when_the_file_is_missing(sandbox: Sandbox) {
    let missing = sandbox.path().join("missing.bin");

    assert_err_is!(
        file::fingerprint(&missing, TEST_WINDOW as u64),
        error if error.kind() == ErrorKind::NotFound
    );
}

#[rstest]
#[case::at_the_start_of_the_file(0, false)]
#[case::on_a_boundary(8, false)]
#[case::inside_an_item(9, true)]
#[case::at_the_end_of_the_file(15, false)]
#[case::past_the_end_of_the_file(16, false)]
fn starts_mid_item_detects_an_unaligned_position(
    sandbox: Sandbox,
    #[case] position: u64,
    #[case] expected: bool,
) {
    let path = sandbox.file(TEST_BLANK_LINE_CONTENT);
    let file = File::open(path).unwrap();

    assert_eq!(
        file::starts_mid_item(&file, position, b'\n').unwrap(),
        expected
    );
}

#[cfg(unix)]
#[rstest]
fn starts_mid_item_fails_when_the_file_is_a_directory(sandbox: Sandbox) {
    let directory = File::open(sandbox.path()).unwrap();

    assert_err_is!(
        file::starts_mid_item(&directory, 1, b'\n'),
        error if error.kind() == ErrorKind::IsADirectory
    );
}

#[rstest]
#[case::restoring_the_cursor(TEST_UNSEEKABLE_POSITION)]
#[case::seeking_the_previous_byte(TEST_UNSEEKABLE_POSITION + 1)]
fn starts_mid_item_fails_when_the_position_is_unseekable(sandbox: Sandbox, #[case] position: u64) {
    let file = File::open(sandbox.file(b"foo")).unwrap();

    assert_err_is!(
        file::starts_mid_item(&file, position, b'\n'),
        error if error.raw_os_error().is_some()
    );
}

#[rstest]
#[case::inside_the_file(2)]
#[case::at_the_file_end(3)]
#[case::past_the_file_end(9)]
fn starts_mid_item_restores_the_cursor(sandbox: Sandbox, #[case] position: u64) {
    let mut file = File::open(sandbox.file(b"foo")).unwrap();

    file::starts_mid_item(&file, position, b'\n').unwrap();

    assert_eq!(file.stream_position().unwrap(), position);
}

#[cfg(unix)]
#[rstest]
fn fingerprint_follows_a_symlink(sandbox: Sandbox) {
    let target = sandbox.file(b"foo");
    let link = sandbox.path().join("link.bin");

    symlink(&target, &link).unwrap();

    assert_eq!(
        file::fingerprint(&link, TEST_WINDOW as u64).unwrap(),
        FOO_FINGERPRINT
    );
}

#[rstest]
fn write_atomic_creates_the_parent_and_removes_the_tmp(sandbox: Sandbox) {
    let path = sandbox.path().join("sub").join(TEST_STATE_FILE);
    let tmp = tmp(&sandbox);

    file::write_atomic(&path, &tmp, b"foo").unwrap();

    assert!(!tmp.exists());

    assert_eq!(fs::read(&path).unwrap(), b"foo");
}

#[rstest]
fn write_atomic_removes_the_tmp_when_the_rename_fails(sandbox: Sandbox) {
    let path = sandbox.path().join(TEST_STATE_FILE);
    let tmp = tmp(&sandbox);

    fs::create_dir_all(path.join("inner")).unwrap();

    assert!(file::write_atomic(&path, &tmp, b"foo").is_err());
    assert!(!tmp.exists());
    assert!(path.is_dir());
}

#[rstest]
fn write_atomic_replaces_an_existing_file(sandbox: Sandbox) {
    let path = sandbox.path().join(TEST_STATE_FILE);
    let tmp = tmp(&sandbox);

    fs::write(&path, b"foo").unwrap();
    file::write_atomic(&path, &tmp, b"bar").unwrap();

    assert!(!tmp.exists());

    assert_eq!(fs::read(&path).unwrap(), b"bar");
}

#[rstest]
fn write_atomic_fails_when_the_tmp_exists(sandbox: Sandbox) {
    let path = sandbox.path().join(TEST_STATE_FILE);
    let tmp = tmp(&sandbox);

    fs::write(&tmp, b"foo").unwrap();

    assert_err_is!(
        file::write_atomic(&path, &tmp, b"bar"),
        error if error.kind() == ErrorKind::AlreadyExists
    );

    assert!(!path.exists());

    assert_eq!(fs::read(&tmp).unwrap(), b"foo");
}

#[rstest]
fn fingerprint_ignores_content_past_the_window(sandbox: Sandbox) {
    let window = vec![b'a'; TEST_WINDOW];
    let first = sandbox.write("a.bin", &[&window, &b"x"[..]].concat());
    let second = sandbox.write("b.bin", &[&window, &b"y"[..]].concat());

    assert_eq!(
        file::fingerprint(&first, TEST_WINDOW as u64).unwrap(),
        file::fingerprint(&second, TEST_WINDOW as u64).unwrap()
    );
}
