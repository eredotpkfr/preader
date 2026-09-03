use std::io::{ErrorKind, Read};

use preader::{IteratorOptions, Window};
use rstest::rstest;
use tempfile::TempDir;

use crate::common::{fixtures::tmp_dir, funcs::write};

const FILE_SIZE: u64 = 100;
const CONTENT: &[u8] = b"foo\nbar\n";
const CONTENT_SIZE: u64 = CONTENT.len() as u64;
const UNSEEKABLE_POSITION: u64 = i64::MAX as u64 + 1;

#[rstest]
#[case::unread_file(IteratorOptions::default(), 0, 0, true)]
#[case::start_moves_the_position(IteratorOptions { start: 10, ..Default::default() }, 0, 10, true)]
#[case::resume_at_the_start(IteratorOptions { start: 10, ..Default::default() }, 10, 10, true)]
#[case::resume_past_the_start(IteratorOptions { start: 10, ..Default::default() }, 40, 40, false)]
#[case::resume_before_the_start(IteratorOptions { start: 40, ..Default::default() }, 10, 40, true)]
#[case::fully_consumed(IteratorOptions::default(), FILE_SIZE, FILE_SIZE, false)]
#[case::resume_past_the_end(IteratorOptions { end: 50, ..Default::default() }, 80, 80, false)]
#[case::start_past_the_file(IteratorOptions { start: 150, ..Default::default() }, 0, FILE_SIZE, true)]
#[case::start_at_the_file_end(
    IteratorOptions { start: FILE_SIZE, ..Default::default() },
    FILE_SIZE,
    FILE_SIZE,
    false
)]
fn window_resolves_the_position(
    #[case] options: IteratorOptions,
    #[case] position: u64,
    #[case] expected_position: u64,
    #[case] expected_from_start: bool,
) {
    let window = options.window(position, FILE_SIZE, 0);

    assert_eq!(window.position, expected_position);
    assert_eq!(window.from_start, expected_from_start);
}

#[rstest]
fn window_resolves_an_empty_file() {
    let window = IteratorOptions::default().window(0, 0, 0);

    assert_eq!(window.position, 0);
    assert_eq!(window.end, 0);
    assert!(!window.from_start);
}

#[rstest]
#[case::unbounded(u64::MAX, FILE_SIZE)]
#[case::inside_the_file(50, 50)]
#[case::at_the_file_size(FILE_SIZE, FILE_SIZE)]
#[case::past_the_file_size(FILE_SIZE + 1, FILE_SIZE)]
fn window_clamps_the_end_to_the_file_size(#[case] end: u64, #[case] expected: u64) {
    let options = IteratorOptions {
        end,
        ..Default::default()
    };

    assert_eq!(options.window(0, FILE_SIZE, 0).end, expected);
}

#[rstest]
#[case::no_skip(0, 10)]
#[case::skip_moves_the_position(15, 25)]
#[case::skip_at_the_end(90, FILE_SIZE)]
#[case::skip_past_the_end(FILE_SIZE, FILE_SIZE)]
#[case::skip_saturates(u64::MAX, FILE_SIZE)]
fn window_folds_skip_bytes_into_the_start(#[case] skip_bytes: u64, #[case] expected: u64) {
    let options = IteratorOptions {
        start: 10,
        ..Default::default()
    };

    assert_eq!(options.window(0, FILE_SIZE, skip_bytes).position, expected);
}

#[rstest]
#[case::start_of_the_file(0, CONTENT)]
#[case::mid_file(4, b"bar\n")]
#[case::end_of_the_file(CONTENT_SIZE, b"")]
fn open_reads_from_the_position(tmp_dir: TempDir, #[case] position: u64, #[case] expected: &[u8]) {
    let path = write(&tmp_dir, "data.bin", CONTENT);
    let window = IteratorOptions::default().window(position, CONTENT_SIZE, 0);

    let mut content = Vec::new();

    window.open(&path, 64).unwrap().read_to_end(&mut content).unwrap();

    assert_eq!(content, expected);
}

#[rstest]
#[case::zero_is_raised_to_one(0, 1)]
#[case::single_byte(1, 1)]
#[case::larger_than_the_file(64, 64)]
fn open_applies_the_buffer_capacity(
    tmp_dir: TempDir,
    #[case] capacity: usize,
    #[case] expected: usize,
) {
    let path = write(&tmp_dir, "data.bin", CONTENT);
    let window = IteratorOptions::default().window(0, CONTENT_SIZE, 0);
    let mut reader = window.open(&path, capacity).unwrap();

    assert_eq!(reader.capacity(), expected);

    let mut content = Vec::new();

    reader.read_to_end(&mut content).unwrap();

    assert_eq!(content, CONTENT);
}

#[rstest]
fn open_fails_when_the_file_is_missing(tmp_dir: TempDir) {
    let window = IteratorOptions::default().window(0, 0, 0);
    let error = window.open(&tmp_dir.path().join("missing.bin"), 64).unwrap_err();

    assert_eq!(error.kind(), ErrorKind::NotFound);
}

#[rstest]
fn open_fails_when_the_position_is_too_large(tmp_dir: TempDir) {
    let path = write(&tmp_dir, "data.bin", CONTENT);
    let window = Window {
        position: UNSEEKABLE_POSITION,
        end: 0,
        from_start: false,
    };
    let error = window.open(&path, 64).unwrap_err();

    assert!(error.raw_os_error().is_some());
}
