use std::io::ErrorKind;

use preader::{Error, IteratorBuild};
use rstest::rstest;

use crate::common::{
    constants::{TEST_ALPHABET, TEST_FILE_NAME, TEST_LINE_CONTENT, TEST_STATE_NAME},
    fixtures::sandbox,
    funcs::{drain, items},
    iterators::{ITERATORS, IteratorKind, Plan},
    sandbox::Sandbox,
};

#[rstest]
fn options_are_validated_before_file_is_opened(sandbox: Sandbox) {
    let missing = sandbox.path().join("missing.bin");
    let error = sandbox.reader().bytes(&missing).start(9).end(4).build().unwrap_err();

    assert!(matches!(error, Error::InvalidRange { .. }), "{error}");
}

#[rstest]
fn missing_file_fails_to_build(sandbox: Sandbox) {
    let missing = sandbox.path().join("missing.bin");
    let error = sandbox.reader().bytes(&missing).build().unwrap_err();

    assert!(matches!(&error, Error::Io(io) if io.kind() == ErrorKind::NotFound));
}

#[rstest]
fn file_is_canonicalized(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let detoured = sandbox.path().join(".").join(TEST_FILE_NAME);
    let state = sandbox.reader().bytes(&detoured).build().unwrap().state().clone();

    assert_eq!(state.file.path, path.canonicalize().unwrap());
}

#[rstest]
fn empty_file_builds_and_yields_nothing(sandbox: Sandbox) {
    let path = sandbox.empty_file();

    for iterator in ITERATORS {
        let (read, state) = iterator.read(&sandbox.reader(), &path, Plan::default()).unwrap();

        assert!(read.is_empty(), "{iterator:?}");

        assert_eq!(state.position, 0, "{iterator:?}");
    }
}

#[rstest]
fn zero_buffer_capacity_still_reads_every_item(sandbox: Sandbox) {
    let path = sandbox.file(TEST_LINE_CONTENT);
    let reader = sandbox.capped(0);

    for iterator in ITERATORS {
        let (read, state) = iterator.read(&reader, &path, Plan::default()).unwrap();

        assert!(!read.is_empty(), "{iterator:?}");

        assert_eq!(
            state.position,
            TEST_LINE_CONTENT.len() as u64,
            "{iterator:?}"
        );
    }
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
fn skip_multiplies_by_the_chunk_size(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let read = items(sandbox.reader().chunks(&path).size(4).skip(2).limit(1).build().unwrap());

    assert_eq!(read, [b"ijkl"]);
}

#[rstest]
fn skip_saturates_for_a_chunk_iterator(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let read = items(sandbox.reader().chunks(&path).size(4).skip(u64::MAX).build().unwrap());

    assert!(read.is_empty());
}

#[rstest]
fn skip_counts_items_for_a_segmented_iterator(sandbox: Sandbox) {
    let path = sandbox.file(TEST_LINE_CONTENT);
    let lines = items(sandbox.reader().lines(&path).skip(2).limit(1).build().unwrap());

    assert_eq!(lines, ["line-2"]);
}

#[rstest]
fn skip_past_the_end_yields_nothing(sandbox: Sandbox) {
    let path = sandbox.file(TEST_LINE_CONTENT);

    assert!(items(sandbox.reader().lines(&path).skip(99).build().unwrap()).is_empty());
}

#[rstest]
fn resumed_state_starts_at_its_position(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let mut bytes = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).limit(3).build().unwrap();

    drain(&mut bytes);

    bytes.state().save().unwrap();

    let state = bytes.state().clone();

    drop(bytes);

    let (read, resumed) = IteratorKind::Bytes
        .read(
            &sandbox.reader(),
            &path,
            Plan {
                take: Some(1),
                ..Plan::resuming(state)
            },
        )
        .unwrap();

    assert_eq!(read, [b"d"]);
    assert_eq!(resumed.position, 4);
}

#[rstest]
fn maximum_start_and_end_yield_nothing(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let read = items(sandbox.reader().bytes(&path).start(u64::MAX).end(u64::MAX).build().unwrap());

    assert!(read.is_empty());
}
