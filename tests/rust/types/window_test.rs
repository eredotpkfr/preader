use std::io::{ErrorKind, Read};

use preader::{Error, IteratorOptions, Skip, Window};
use rstest::rstest;

use crate::common::{constants::TEST_LINE_CONTENT, fixtures::sandbox, sandbox::Sandbox};

const CONTENT_SIZE: u64 = TEST_LINE_CONTENT.len() as u64;
const UNSEEKABLE_POSITION: u64 = i64::MAX as u64 + 1;

fn window(position: u64) -> Window {
    IteratorOptions::default().window(position, CONTENT_SIZE, Skip::Bytes(0))
}

#[rstest]
#[case::start_of_the_file(0, TEST_LINE_CONTENT)]
#[case::mid_file(7, b"line-1\nline-2\nline-3\nline-4\nline-5")]
#[case::end_of_the_file(CONTENT_SIZE, b"")]
fn open_reads_from_the_position(sandbox: Sandbox, #[case] position: u64, #[case] expected: &[u8]) {
    let path = sandbox.file(TEST_LINE_CONTENT);
    let mut content = Vec::new();

    window(position).open(&path, 64).unwrap().read_to_end(&mut content).unwrap();

    assert_eq!(content, expected);
}

#[rstest]
#[case::zero_is_raised_to_one(0, 1)]
#[case::single_byte(1, 1)]
#[case::larger_than_the_file(64, 64)]
fn open_applies_the_buffer_capacity(
    sandbox: Sandbox,
    #[case] capacity: usize,
    #[case] expected: usize,
) {
    let path = sandbox.file(TEST_LINE_CONTENT);
    let mut reader = window(0).open(&path, capacity).unwrap();
    let mut content = Vec::new();

    assert_eq!(reader.capacity(), expected);

    reader.read_to_end(&mut content).unwrap();

    assert_eq!(content, TEST_LINE_CONTENT);
}

#[rstest]
fn open_fails_when_the_file_is_missing(sandbox: Sandbox) {
    let error = window(0).open(&sandbox.path().join("missing.bin"), 64).unwrap_err();

    assert!(matches!(error, Error::Io(error) if error.kind() == ErrorKind::NotFound));
}

#[rstest]
fn open_fails_when_the_position_is_unseekable(sandbox: Sandbox) {
    let path = sandbox.file(TEST_LINE_CONTENT);
    let window = Window {
        position: UNSEEKABLE_POSITION,
        end: 0,
        skipping: None,
    };
    let error = window.open(&path, 64).unwrap_err();

    assert!(matches!(error, Error::Io(error) if error.raw_os_error().is_some()));
}

#[rstest]
fn open_fails_when_the_path_is_a_directory(sandbox: Sandbox) {
    let path = sandbox.dir_at("folder");
    let mut content = Vec::new();
    let opened = window(0).open(&path, 64);

    let Ok(mut reader) = opened else {
        return;
    };

    assert!(matches!(
        reader.read_to_end(&mut content).unwrap_err().kind(),
        ErrorKind::IsADirectory | ErrorKind::PermissionDenied
    ));
}
