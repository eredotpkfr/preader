use std::io::ErrorKind;

use preader::{Config, DEFAULT_AUTO_SAVE_STATE_BYTES, Error, IteratorOptions, IteratorRead};
use rstest::rstest;

#[cfg(unix)]
use crate::common::constants::{TEST_READ_FROM, TEST_REWOUND_TO};
use crate::common::{
    constants::{TEST_ALPHABET, TEST_CHUNK_SIZE, TEST_INVALID_UTF8, TEST_STATE_NAME},
    fixtures::sandbox,
    funcs::{consume, drain, items, try_items},
    macros::asserts::assert_err_is,
    matrix::contents::ALL_BYTES,
    sandbox::Sandbox,
};

fn chunked(content: &[u8], size: usize) -> Vec<Vec<u8>> {
    content.chunks(size).map(<[u8]>::to_vec).collect()
}

#[rstest]
fn iterator_defaults(sandbox: Sandbox) {
    let content = TEST_ALPHABET.repeat(40);
    let path = sandbox.file(&content);
    let read = items(sandbox.reader().chunks(&path).build().unwrap());

    assert_eq!(read, [&content[..1024], &content[1024..]]);
}

#[rstest]
fn drop_partial_yields_nothing_for_an_empty_file(sandbox: Sandbox) {
    let path = sandbox.empty_file();

    assert!(items(sandbox.reader().chunks(&path).drop_partial(true).build().unwrap()).is_empty());
}

#[rstest]
fn zero_size_yields_nothing(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);

    assert!(items(sandbox.reader().chunks(&path).size(0).build().unwrap()).is_empty());
}

#[rstest]
fn drop_partial_discards_a_single_byte_file(sandbox: Sandbox) {
    let path = sandbox.file(b"a");

    assert!(items(sandbox.reader().chunks(&path).drop_partial(true).build().unwrap()).is_empty());
}

#[rstest]
fn yields_one_chunk_for_a_large_size(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let read = items(sandbox.reader().chunks(&path).size(1024).build().unwrap());

    assert_eq!(read, [TEST_ALPHABET]);
}

#[rstest]
fn splits_into_fixed_size_pieces(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let read = items(sandbox.reader().chunks(&path).size(3).build().unwrap());

    assert_eq!(read, chunked(TEST_ALPHABET, 3));
}

#[rstest]
fn drop_partial_yields_nothing_for_a_large_size(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let read = items(sandbox.reader().chunks(&path).size(1024).drop_partial(true).build().unwrap());

    assert!(read.is_empty());
}

#[rstest]
fn whole_file_size_yields_one_chunk(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let read = items(sandbox.reader().chunks(&path).size(TEST_ALPHABET.len()).build().unwrap());

    assert_eq!(read, [TEST_ALPHABET]);
}

#[rstest]
#[case::maximum_skip(4, u64::MAX)]
#[case::overflowing_product(1 << 32, 1 << 32)]
fn skip_saturates_when_multiplied_by_the_size(
    sandbox: Sandbox,
    #[case] size: usize,
    #[case] skip: u64,
) {
    let path = sandbox.file(TEST_ALPHABET);
    let read = items(sandbox.reader().chunks(&path).size(size).skip(skip).build().unwrap());

    assert!(read.is_empty());
}

#[rstest]
fn read_fills_a_chunk_across_a_buffer_refill(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.capped(16);
    let read = items(reader.chunks(&path).size(10).build().unwrap());

    assert_eq!(read, chunked(TEST_ALPHABET, 10));
}

#[rstest]
fn limit_counts_chunks_from_an_unaligned_start(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let read = items(sandbox.reader().chunks(&path).size(4).start(1).limit(1).build().unwrap());

    assert_eq!(read, [b"bcde"]);
}

#[rstest]
fn start_is_not_realigned_to_the_chunk_grid(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let read = items(sandbox.reader().chunks(&path).start(7).size(3).build().unwrap());

    assert_eq!(read, chunked(&TEST_ALPHABET[7..], 3));
}

#[rstest]
fn read_keeps_invalid_bytes_verbatim(sandbox: Sandbox) {
    let content = [TEST_ALPHABET, TEST_INVALID_UTF8].concat();
    let path = sandbox.file(&content);
    let read = items(sandbox.reader().chunks(&path).size(4).build().unwrap());

    assert_eq!(read.concat(), content);
}

#[rstest]
fn drop_partial_keeps_an_exact_chunk(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let read = items(sandbox.reader().chunks(&path).size(13).drop_partial(true).build().unwrap());

    assert_eq!(read.len(), 2);
    assert_eq!(read.concat(), TEST_ALPHABET);
}

#[rstest]
#[case::start(IteratorOptions { start: 6, ..IteratorOptions::default() }, &TEST_ALPHABET[6..])]
#[case::end(IteratorOptions { end: 10, ..IteratorOptions::default() }, &TEST_ALPHABET[..10])]
#[case::skip(IteratorOptions { skip: 4, ..IteratorOptions::default() }, &TEST_ALPHABET[12..])]
#[case::start_and_limit(
    IteratorOptions { start: 6, limit: 2, ..IteratorOptions::default() },
    &TEST_ALPHABET[6..12],
)]
fn options_narrow_the_output(
    sandbox: Sandbox,
    #[case] options: IteratorOptions,
    #[case] expected: &[u8],
) {
    let path = sandbox.file(TEST_ALPHABET);
    let read = items(sandbox.reader().chunks(&path).options(options).size(3).build().unwrap());

    assert_eq!(read, expected.chunks(3).collect::<Vec<_>>());
}

#[rstest]
fn read_keeps_a_short_final_chunk(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let read = items(sandbox.reader().chunks(&path).size(10).build().unwrap());

    assert_eq!(read.last().unwrap(), b"uvwxyz");
    assert_eq!(read.concat(), TEST_ALPHABET);
}

#[rstest]
fn position_counts_bytes_not_chunks(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let mut chunks = sandbox.reader().chunks(&path).size(4).build().unwrap();

    chunks.read().unwrap();
    chunks.read().unwrap();

    assert_eq!(chunks.state().position, 4);
}

#[rstest]
fn keeps_a_partial_chunk_by_default(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let read = items(sandbox.reader().chunks(&path).end(12).size(5).build().unwrap());

    assert_eq!(
        read,
        [
            &TEST_ALPHABET[0..5],
            &TEST_ALPHABET[5..10],
            &TEST_ALPHABET[10..12]
        ]
    );
}

#[rstest]
fn yields_every_byte_value(sandbox: Sandbox) {
    let path = sandbox.file(ALL_BYTES);
    let expected: Vec<Vec<u8>> = (0..=u8::MAX).map(|value| vec![value]).collect();

    assert_eq!(
        items(sandbox.reader().chunks(&path).size(1).build().unwrap()),
        expected
    );
}

#[rstest]
fn iterator_yields_owned_chunks(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let mut collected: Vec<Vec<u8>> = Vec::new();

    for chunk in sandbox.reader().chunks(&path).size(13).build().unwrap() {
        collected.push(chunk.unwrap());
    }

    assert_eq!(collected, chunked(TEST_ALPHABET, 13));
}

#[rstest]
fn drop_partial_reads_past_a_buffer_refill(sandbox: Sandbox) {
    let content = TEST_ALPHABET.repeat(20);
    let path = sandbox.file(&content);
    let reader = sandbox.capped(64);
    let read =
        items(reader.chunks(&path).size(TEST_CHUNK_SIZE).drop_partial(true).build().unwrap());

    assert_eq!(
        read.concat(),
        content[..content.len() - content.len() % TEST_CHUNK_SIZE]
    );
}

#[rstest]
fn position_counts_finished_items(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let mut chunks = sandbox.reader().chunks(&path).size(3).build().unwrap();

    chunks.read().unwrap();

    assert_eq!(chunks.state().position, 0);
    assert_eq!(chunks.percent(), 0.0);

    chunks.read().unwrap();

    assert_eq!(chunks.state().position, 3);
}

#[rstest]
fn drop_partial_discards_a_chunk_cut_short_by_end(sandbox: Sandbox) {
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
#[case::ten_byte_chunks(TEST_ALPHABET, 10, 14, &TEST_ALPHABET[..10])]
#[case::three_byte_chunks(b"foobarbaz", 3, 5, b"foo")]
fn drop_partial_discards_a_truncated_chunk(
    sandbox: Sandbox,
    #[case] content: &[u8],
    #[case] size: usize,
    #[case] truncated: u64,
    #[case] first: &[u8],
) {
    let path = sandbox.file(content);
    let reader = sandbox.capped(1);
    let mut chunks = reader.chunks(&path).size(size).drop_partial(true).build().unwrap();

    assert_eq!(chunks.read().unwrap(), Some(first));

    sandbox.truncate(&path, truncated);

    assert!(items(chunks.by_ref()).is_empty());

    assert_eq!(chunks.state().position, truncated);
}

#[rstest]
fn file_growth_during_iteration_is_ignored(sandbox: Sandbox) {
    let path = sandbox.file(&TEST_ALPHABET[..6]);
    let mut chunks = sandbox.reader().chunks(&path).size(3).build().unwrap();

    chunks.read().unwrap();
    sandbox.append(&path, b"more");

    assert_eq!(items(chunks.by_ref()), [&TEST_ALPHABET[3..6]]);
    assert_eq!(chunks.state().position, 6);
}

#[rstest]
fn size_of_one_yields_single_bytes(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.reader();
    let read = items(reader.chunks(&path).size(1).build().unwrap());
    let bytes: Vec<Vec<u8>> = items(reader.bytes(&path).build().unwrap())
        .into_iter()
        .map(|byte| vec![byte])
        .collect();

    assert_eq!(read, chunked(TEST_ALPHABET, 1));
    assert_eq!(read, bytes);
}

#[rstest]
fn read_yields_invalid_bytes(sandbox: Sandbox) {
    let invalid = [b"foo", TEST_INVALID_UTF8, b"bar"].concat();
    let path = sandbox.file(&invalid);
    let mut chunks = sandbox.reader().chunks(&path).size(4).build().unwrap();

    assert_eq!(items(chunks.by_ref()), [&invalid[..4], &invalid[4..]]);
    assert_eq!(chunks.state().position, 7);
    assert_eq!(chunks.state().percent(), 100.0);
}

#[rstest]
fn resume_applies_a_different_size(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.reader();
    let mut chunks = reader.chunks(&path).state(TEST_STATE_NAME).size(4).build().unwrap();

    consume(&mut chunks, 2);

    let state = chunks.save().unwrap().clone();

    let resumed = items(reader.chunks(&path).state(state).size(3).build().unwrap());

    assert_eq!(resumed, chunked(&TEST_ALPHABET[4..], 3));
}

#[rstest]
fn resume_applies_a_different_size_and_limit(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.reader();
    let mut chunks = reader.chunks(&path).state(TEST_STATE_NAME).size(4).limit(1).build().unwrap();

    drain(&mut chunks);

    let state = chunks.save().unwrap().clone();

    let read = items(reader.chunks(&path).state(state).size(2).limit(2).build().unwrap());

    assert_eq!(read, [b"ef", b"gh"]);
}

#[cfg(unix)]
#[rstest]
fn read_fails_when_io_fails_mid_chunk(sandbox: Sandbox) {
    let directory = sandbox.dir_at("folder");
    let opened = sandbox.state_at(&directory, TEST_READ_FROM);
    let mut chunks = sandbox.lenient().chunks(&directory).state(opened).build().unwrap();

    *chunks.state_mut() = sandbox.state_at(&directory, TEST_REWOUND_TO);

    assert_err_is!(chunks.read(), Error::Io(_));

    assert_eq!(chunks.state().position, TEST_REWOUND_TO);

    assert_err_is!(chunks.read(), Error::Io(_));

    assert_eq!(chunks.state().position, TEST_READ_FROM);
}

#[rstest]
fn resume_applies_a_skip_past_the_saved_position(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.reader();
    let mut chunks = reader.chunks(&path).state(TEST_STATE_NAME).size(2).build().unwrap();

    consume(&mut chunks, 2);

    let state = chunks.save().unwrap().clone();

    let resumed =
        items(reader.chunks(&path).state(state).size(2).start(1).skip(5).build().unwrap());

    assert_eq!(resumed, chunked(&TEST_ALPHABET[11..], 2));
}

#[rstest]
fn fails_when_a_save_fails_on_a_chunk_dropped_by_end(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.autosaving(DEFAULT_AUTO_SAVE_STATE_BYTES.get());
    let mut chunks = reader
        .chunks(&path)
        .state(TEST_STATE_NAME)
        .size(8)
        .drop_partial(true)
        .end(12)
        .build()
        .unwrap();

    sandbox.block_states();

    assert_eq!(chunks.read().unwrap(), Some(&TEST_ALPHABET[..8]));

    assert_err_is!(chunks.read(), Error::Io(error) if error.kind() == ErrorKind::AlreadyExists);

    assert_eq!(chunks.state().position, 8);
}

#[rstest]
fn threshold_saves_during_iteration(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.autosaving(10);
    let mut chunks = reader.chunks(&path).state(TEST_STATE_NAME).size(3).build().unwrap();
    let mut positions = Vec::new();

    while chunks.read().unwrap().is_some() {
        if let Some(state) = sandbox.states().find(TEST_STATE_NAME)
            && positions.last() != Some(&state.position)
        {
            positions.push(state.position);
        }
    }

    assert_eq!(positions, [12, 24]);
    assert_eq!(
        sandbox.states().load(TEST_STATE_NAME).unwrap().position,
        TEST_ALPHABET.len() as u64
    );
}

#[rstest]
fn fails_when_a_save_fails_on_a_dropped_short_chunk(sandbox: Sandbox) {
    sandbox.block_states();

    let reader = sandbox.reader_with(Config {
        auto_save_state: true,
        buffer_capacity: 1,
        ..sandbox.config()
    });
    let path = sandbox.file(b"foobarbaz");
    let mut chunks = reader
        .chunks(&path)
        .state(TEST_STATE_NAME)
        .size(3)
        .drop_partial(true)
        .build()
        .unwrap();

    assert_eq!(chunks.read().unwrap(), Some(&b"foo"[..]));

    sandbox.truncate(&path, 5);

    assert_err_is!(
        try_items(chunks.by_ref()),
        Error::Io(error) if error.kind() == ErrorKind::AlreadyExists
    );

    assert_eq!(chunks.state().position, 5);
}

#[rstest]
fn resumed_read_floors_the_last_saved_position(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let mut chunks = sandbox.reader().chunks(&path).state(TEST_STATE_NAME).size(2).build().unwrap();

    consume(&mut chunks, 4);

    let state = chunks.save().unwrap().clone();

    let saving = sandbox.autosaving(4);
    let mut resumed = saving.chunks(&path).state(state).size(2).build().unwrap();

    consume(&mut resumed, 2);

    assert_eq!(sandbox.states().load(TEST_STATE_NAME).unwrap().position, 8);
}
