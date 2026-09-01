#[cfg(unix)]
use std::os::unix::fs::symlink;
use std::{
    fs::File,
    io::{ErrorKind, Seek},
};

use preader::{fingerprint, starts_mid_item};
use rstest::rstest;
use tempfile::TempDir;

use crate::common::{fixtures::tmp_dir, funcs::write};

const EMPTY_DIGEST: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
const FOO_DIGEST: &str = "2c26b46b68ffc68ff99b453c1d30413413422d706483bfa0f98a5e886266e7ae";
const FULL_WINDOW_DIGEST: &str = "c93eee2d0db02f10acc7460d9576e122dcf8cd53c4bf8dfcae1b3e74ebcfff5a";

const FINGERPRINT_WINDOW: usize = 4096;
const UNSEEKABLE_POSITION: u64 = i64::MAX as u64 + 1;

#[rstest]
#[case::plain_content(b"foo".to_vec(), FOO_DIGEST)]
#[case::empty_file(Vec::new(), EMPTY_DIGEST)]
#[case::exactly_the_window(vec![b'a'; FINGERPRINT_WINDOW], FULL_WINDOW_DIGEST)]
#[case::just_past_the_window(vec![b'a'; FINGERPRINT_WINDOW + 1], FULL_WINDOW_DIGEST)]
fn fingerprint_digests_the_first_window(
    tmp_dir: TempDir,
    #[case] content: Vec<u8>,
    #[case] expected: &str,
) {
    let path = write(&tmp_dir, "data.bin", &content);

    assert_eq!(fingerprint(&path).unwrap(), expected);
}

#[rstest]
fn fingerprint_ignores_content_past_the_window(tmp_dir: TempDir) {
    let window = vec![b'a'; FINGERPRINT_WINDOW];
    let shorter = write(&tmp_dir, "shorter.bin", &[&window, &b"x"[..]].concat());
    let longer = write(&tmp_dir, "longer.bin", &[&window, &b"y"[..]].concat());

    assert_eq!(
        fingerprint(&shorter).unwrap(),
        fingerprint(&longer).unwrap()
    );
}

#[cfg(unix)]
#[rstest]
fn fingerprint_follows_a_symlink(tmp_dir: TempDir) {
    let target = write(&tmp_dir, "target.bin", b"foo");
    let link = tmp_dir.path().join("link.bin");

    symlink(&target, &link).unwrap();

    assert_eq!(fingerprint(&link).unwrap(), FOO_DIGEST);
}

#[rstest]
fn fingerprint_fails_when_the_file_is_missing(tmp_dir: TempDir) {
    let error = fingerprint(&tmp_dir.path().join("missing.bin")).unwrap_err();

    assert_eq!(error.kind(), ErrorKind::NotFound);
}

#[rstest]
fn fingerprint_fails_when_the_path_is_a_directory(tmp_dir: TempDir) {
    let error = fingerprint(tmp_dir.path()).unwrap_err();

    assert!(matches!(
        error.kind(),
        ErrorKind::IsADirectory | ErrorKind::PermissionDenied
    ));
}

#[rstest]
#[case::at_the_start_of_the_file(0, false)]
#[case::on_a_boundary(7, false)]
#[case::inside_an_item(9, true)]
#[case::at_the_end_of_the_file(21, false)]
#[case::past_the_end_of_the_file(22, false)]
fn starts_mid_item_detects_an_unaligned_position(
    tmp_dir: TempDir,
    #[case] position: u64,
    #[case] expected: bool,
) {
    let path = write(&tmp_dir, "data.bin", b"line-0\nline-1\nline-2\n");
    let file = File::open(path).unwrap();

    assert_eq!(starts_mid_item(&file, position, b'\n').unwrap(), expected);
}

#[rstest]
#[case::inside_the_file(2)]
#[case::at_the_file_end(3)]
#[case::past_the_file_end(9)]
fn starts_mid_item_leaves_the_cursor_at_the_position(tmp_dir: TempDir, #[case] position: u64) {
    let mut file = File::open(write(&tmp_dir, "data.bin", b"foo")).unwrap();

    starts_mid_item(&file, position, b'\n').unwrap();

    assert_eq!(file.stream_position().unwrap(), position);
}

#[rstest]
#[case::restoring_the_cursor(UNSEEKABLE_POSITION)]
#[case::reading_the_previous_byte(UNSEEKABLE_POSITION + 1)]
fn starts_mid_item_fails_when_the_position_is_too_large(tmp_dir: TempDir, #[case] position: u64) {
    let file = File::open(write(&tmp_dir, "data.bin", b"foo")).unwrap();
    let error = starts_mid_item(&file, position, b'\n').unwrap_err();

    assert_eq!(error.kind(), ErrorKind::InvalidInput);
}

#[cfg(unix)]
#[rstest]
fn starts_mid_item_fails_when_the_file_is_a_directory(tmp_dir: TempDir) {
    let directory = File::open(tmp_dir.path()).unwrap();
    let error = starts_mid_item(&directory, 1, b'\n').unwrap_err();

    assert_eq!(error.kind(), ErrorKind::IsADirectory);
}
