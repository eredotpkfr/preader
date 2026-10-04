use preader::{Error, IteratorBuild, IteratorRead};
use rstest::rstest;

use crate::common::{
    constants::{TEST_ALPHABET, TEST_INVALID_UTF8, TEST_READ_FROM, TEST_REWOUND_TO},
    fixtures::sandbox,
    funcs::items,
    macros::asserts::assert_err_is,
    sandbox::Sandbox,
};

#[rstest]
fn read_keeps_a_short_final_chunk(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let read = items(sandbox.reader().chunks(&path).size(10).build().unwrap());

    assert_eq!(read.last().unwrap(), b"uvwxyz");
    assert_eq!(read.concat(), TEST_ALPHABET);
}

#[rstest]
fn drop_partial_keeps_exactly_divisible_tail(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let read = items(sandbox.reader().chunks(&path).size(13).drop_partial(true).build().unwrap());

    assert_eq!(read.len(), 2);
    assert_eq!(read.concat(), TEST_ALPHABET);
}

#[rstest]
fn drop_partial_discards_chunk_cut_short_by_end(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let mut chunks = sandbox
        .reader()
        .chunks(&path)
        .size(10)
        .end(15)
        .drop_partial(true)
        .build()
        .unwrap();

    assert_eq!(chunks.read().unwrap(), Some(&TEST_ALPHABET[..10]));
    assert_eq!(chunks.read().unwrap(), None);
    assert_eq!(chunks.state().position, 10);
}

#[rstest]
fn drop_partial_discards_a_truncated_chunk(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.capped(1);
    let mut chunks = reader.chunks(&path).size(10).drop_partial(true).build().unwrap();

    chunks.read().unwrap();
    sandbox.truncate(&path, 14);

    assert_eq!(chunks.read().unwrap(), None);
    assert_eq!(chunks.state().position, 14);
}

#[rstest]
fn zero_size_yields_nothing(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);

    assert!(items(sandbox.reader().chunks(&path).size(0).build().unwrap()).is_empty());
}

#[rstest]
fn size_larger_than_the_file_yields_one_chunk(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let read = items(sandbox.reader().chunks(&path).size(1024).build().unwrap());

    assert_eq!(read, [TEST_ALPHABET]);
}

#[rstest]
fn read_keeps_invalid_bytes_verbatim(sandbox: Sandbox) {
    let content = [TEST_ALPHABET, TEST_INVALID_UTF8].concat();
    let path = sandbox.file(&content);
    let read = items(sandbox.reader().chunks(&path).size(4).build().unwrap());

    assert_eq!(read.concat(), content);
}

#[rstest]
fn read_fills_a_chunk_across_a_buffer_refill(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.capped(1);
    let read = items(reader.chunks(&path).size(10).build().unwrap());

    assert_eq!(read[0], &TEST_ALPHABET[..10]);
    assert_eq!(read.concat(), TEST_ALPHABET);
}

#[rstest]
fn position_counts_bytes_not_chunks(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let mut chunks = sandbox.reader().chunks(&path).size(4).build().unwrap();

    chunks.read().unwrap();

    assert_eq!(chunks.state().position, 4);
}

#[rstest]
fn iterator_yields_owned_chunks(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let mut collected: Vec<Vec<u8>> = Vec::new();

    for chunk in sandbox.reader().chunks(&path).size(13).build().unwrap() {
        collected.push(chunk.unwrap());
    }

    assert_eq!(collected.concat(), TEST_ALPHABET);
}

#[cfg(unix)]
#[rstest]
fn read_reports_a_mid_chunk_io_error(sandbox: Sandbox) {
    let directory = sandbox.dir_at("folder");
    let opened = sandbox.state_at(&directory, TEST_READ_FROM);
    let mut chunks = sandbox.lenient().chunks(&directory).state(opened).build().unwrap();

    *chunks.state() = sandbox.state_at(&directory, TEST_REWOUND_TO);

    let read = chunks.read().map(|chunk| chunk.map(<[u8]>::to_vec));

    assert_eq!(chunks.state().position, TEST_READ_FROM);

    assert_err_is!(read, Error::Io(_));
}
