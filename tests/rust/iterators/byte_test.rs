use preader::IteratorRead;
use rstest::rstest;

use crate::common::{
    constants::{TEST_ALPHABET, TEST_INVALID_UTF8, TEST_STATE_NAME},
    fixtures::sandbox,
    funcs::{consume, items},
    matrix::contents::ALL_BYTES,
    sandbox::Sandbox,
};

#[rstest]
fn read_yields_every_byte_in_order(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let read = items(sandbox.reader().bytes(&path).build().unwrap());

    assert_eq!(read, TEST_ALPHABET);
}

#[rstest]
fn read_advances_one_byte_at_a_time(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let mut bytes = sandbox.reader().bytes(&path).build().unwrap();

    for position in 0..3 {
        bytes.read().unwrap();

        assert_eq!(bytes.state().position, position);
    }
}

#[rstest]
fn iterator_composes_with_adapters(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let collected: Vec<u8> = sandbox
        .reader()
        .bytes(&path)
        .build()
        .unwrap()
        .map(Result::unwrap)
        .filter(u8::is_ascii_lowercase)
        .take(3)
        .collect();

    assert_eq!(collected, b"abc");
}

#[rstest]
fn yields_every_byte_value(sandbox: Sandbox) {
    let path = sandbox.file(ALL_BYTES);

    assert_eq!(
        items(sandbox.reader().bytes(&path).build().unwrap()),
        ALL_BYTES
    );
}

#[rstest]
fn read_stays_exhausted_after_the_end(sandbox: Sandbox) {
    let path = sandbox.file(b"a");
    let mut bytes = sandbox.reader().bytes(&path).build().unwrap();

    assert_eq!(bytes.read().unwrap(), Some(b'a'));

    for _ in 0..3 {
        assert_eq!(bytes.read().unwrap(), None);
    }
}

#[rstest]
fn read_yields_an_invalid_byte_verbatim(sandbox: Sandbox) {
    let path = sandbox.file(TEST_INVALID_UTF8);
    let mut bytes = sandbox.reader().bytes(&path).build().unwrap();

    assert_eq!(bytes.read().unwrap(), Some(TEST_INVALID_UTF8[0]));
    assert_eq!(bytes.read().unwrap(), None);
    assert_eq!(bytes.state().position, 1);
}

#[rstest]
fn position_counts_finished_items(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let mut bytes = sandbox.reader().bytes(&path).build().unwrap();

    bytes.read().unwrap();

    assert_eq!(bytes.state().position, 0);
    assert_eq!(bytes.percent(), 0.0);

    bytes.read().unwrap();

    assert_eq!(bytes.state().position, 1);
}

#[rstest]
#[case::whole_alphabet(TEST_ALPHABET, &TEST_ALPHABET[1..])]
#[case::three_bytes(&TEST_ALPHABET[..3], b"bc")]
fn file_growth_during_iteration_is_ignored(
    sandbox: Sandbox,
    #[case] content: &[u8],
    #[case] expected: &[u8],
) {
    let size = content.len() as u64;
    let path = sandbox.file(content);
    let mut bytes = sandbox.reader().bytes(&path).build().unwrap();

    bytes.read().unwrap();
    sandbox.append(&path, b"more");

    assert_eq!(items(bytes.by_ref()), expected);
    assert_eq!(bytes.state().position, size);
    assert_eq!(bytes.state().file.size, size);
}

#[rstest]
fn read_yields_invalid_bytes(sandbox: Sandbox) {
    let invalid = [b"foo", TEST_INVALID_UTF8, b"bar"].concat();
    let path = sandbox.file(&invalid);
    let mut bytes = sandbox.reader().bytes(&path).build().unwrap();

    assert_eq!(items(bytes.by_ref()), invalid);
    assert_eq!(bytes.state().position, 7);
    assert_eq!(bytes.state().percent(), 100.0);
}

#[rstest]
fn threshold_records_every_item_boundary(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.autosaving(1);
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();
    let mut positions = Vec::new();

    bytes.read().unwrap();

    while bytes.read().unwrap().is_some() {
        positions.push(sandbox.states().load(TEST_STATE_NAME).unwrap().position);
    }

    assert_eq!(
        positions,
        (1..TEST_ALPHABET.len() as u64).collect::<Vec<u64>>()
    );
}

#[rstest]
fn resume_applies_a_skip_past_the_saved_position(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.reader();
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    consume(&mut bytes, 4);

    let state = bytes.save().unwrap().clone();

    let resumed = items(reader.bytes(&path).state(state).start(1).skip(10).build().unwrap());

    assert_eq!(resumed, TEST_ALPHABET[11..]);
}

#[rstest]
fn threshold_saves_during_iteration(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.autosaving(10);
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();
    let mut positions = Vec::new();

    while bytes.read().unwrap().is_some() {
        if let Some(state) = sandbox.states().find(TEST_STATE_NAME)
            && positions.last() != Some(&state.position)
        {
            positions.push(state.position);
        }
    }

    assert_eq!(positions, [10, 20]);
    assert_eq!(
        sandbox.states().load(TEST_STATE_NAME).unwrap().position,
        TEST_ALPHABET.len() as u64
    );
}

#[rstest]
fn resumed_read_floors_the_last_saved_position(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);

    sandbox.checkpoint(&path, TEST_STATE_NAME, 7);

    let resumed = sandbox.autosaving(3);
    let mut bytes = resumed.bytes(&path).state(TEST_STATE_NAME).build().unwrap();
    let mut positions = vec![7];

    while bytes.read().unwrap().is_some() {
        let position = sandbox.states().load(TEST_STATE_NAME).unwrap().position;

        if positions.last() != Some(&position) {
            positions.push(position);
        }
    }

    assert_eq!(positions, [7, 9, 12, 15, 18, 21, 24]);
    assert_eq!(
        sandbox.states().load(TEST_STATE_NAME).unwrap().position,
        TEST_ALPHABET.len() as u64
    );
}
