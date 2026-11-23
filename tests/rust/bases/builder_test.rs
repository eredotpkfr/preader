use preader::Error;
use rstest::rstest;

#[cfg(unix)]
use crate::common::constants::TEST_READ_FROM;
use crate::common::{
    constants::{TEST_ALPHABET, TEST_FILE_NAME, TEST_LINE_CONTENT, TEST_STATE_NAME},
    fixtures::sandbox,
    funcs::{canonical, items},
    iterators::{IteratorKind, Plan},
    macros::asserts::assert_err_is,
    sandbox::Sandbox,
};

#[rstest]
fn skip_past_the_end_yields_nothing(sandbox: Sandbox) {
    let path = sandbox.file(TEST_LINE_CONTENT);

    assert!(items(sandbox.reader().lines(&path).skip(99).build().unwrap()).is_empty());
}

#[rstest]
fn build_fails_when_the_range_is_inverted_before_opening_the_file(sandbox: Sandbox) {
    let missing = sandbox.path().join("missing.bin");

    assert_err_is!(
        sandbox.reader().bytes(&missing).start(9).end(4).build(),
        Error::InvalidRange
    );
}

#[rstest]
fn largest_chunk_size_still_reads_the_file(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let read = items(sandbox.reader().chunks(&path).size(usize::MAX).build().unwrap());

    assert_eq!(read, [TEST_ALPHABET]);
}

#[rstest]
fn skip_counts_bytes_for_a_byte_iterator(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let read = items(sandbox.reader().bytes(&path).skip(3).limit(1).build().unwrap());

    assert_eq!(read, b"d");
}

#[rstest]
fn skip_counts_items_for_a_segmented_iterator(sandbox: Sandbox) {
    let path = sandbox.file(TEST_LINE_CONTENT);
    let lines = items(sandbox.reader().lines(&path).skip(2).limit(1).build().unwrap());

    assert_eq!(lines, ["line-2"]);
}

#[rstest]
fn skip_multiplies_by_the_chunk_size(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let read = items(sandbox.reader().chunks(&path).size(4).skip(2).limit(1).build().unwrap());

    assert_eq!(read, [b"ijkl"]);
}

#[cfg(unix)]
#[rstest]
fn build_fails_when_the_alignment_read_fails(sandbox: Sandbox) {
    let directory = sandbox.dir_at("folder");
    let state = sandbox.state_at(&directory, TEST_READ_FROM);
    let built = sandbox
        .lenient()
        .lines(&directory)
        .state(state)
        .start(TEST_READ_FROM)
        .align(true)
        .build();

    assert_err_is!(built, Error::Io(_));
}

#[rstest]
fn file_is_canonicalized(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let detoured = sandbox.path().join(".").join(TEST_FILE_NAME);
    let state = sandbox.reader().bytes(&detoured).build().unwrap().state().clone();

    assert_eq!(state.file.path.as_os_str(), canonical(&path).as_os_str());
}

#[rstest]
fn resumed_state_starts_at_its_position(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let state = sandbox.checkpoint(&path, TEST_STATE_NAME, 3);

    let (read, resumed) = IteratorKind::Bytes
        .read(
            &sandbox.reader(),
            &path,
            Plan {
                state: state.into(),
                take: Some(1),
                ..Plan::default()
            },
        )
        .unwrap();

    assert_eq!(read, [b"d"]);
    assert_eq!(resumed.position, 3);
}
