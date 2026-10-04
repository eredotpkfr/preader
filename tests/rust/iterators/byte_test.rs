use preader::{IteratorBuild, IteratorRead};
use rstest::rstest;

use crate::common::{
    constants::{TEST_ALPHABET, TEST_INVALID_UTF8},
    fixtures::sandbox,
    funcs::items,
    sandbox::Sandbox,
};

#[rstest]
fn read_yields_every_byte_in_order(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let read = items(sandbox.reader().bytes(&path).build().unwrap());

    assert_eq!(read.concat(), TEST_ALPHABET);
}

#[rstest]
fn read_yields_every_byte_value(sandbox: Sandbox) {
    let content: Vec<u8> = (0..=255).collect();
    let path = sandbox.file(&content);
    let read = items(sandbox.reader().bytes(&path).build().unwrap());

    assert_eq!(read.concat(), content);
}

#[rstest]
fn read_yields_an_invalid_byte_verbatim(sandbox: Sandbox) {
    let path = sandbox.file(TEST_INVALID_UTF8);
    let mut bytes = sandbox.reader().bytes(&path).build().unwrap();

    assert_eq!(bytes.read().unwrap(), Some(TEST_INVALID_UTF8[0]));
    assert_eq!(bytes.state().position, 1);
}

#[rstest]
fn read_advances_one_byte_at_a_time(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let mut bytes = sandbox.reader().bytes(&path).build().unwrap();

    for position in 1..=3 {
        bytes.read().unwrap();

        assert_eq!(bytes.state().position, position);
    }
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
fn the_iterator_yields_owned_bytes(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let mut collected = Vec::new();

    for byte in sandbox.reader().bytes(&path).build().unwrap() {
        collected.push(byte.unwrap());
    }

    assert_eq!(collected, TEST_ALPHABET);
}

#[rstest]
fn the_iterator_composes_with_adapters(sandbox: Sandbox) {
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
