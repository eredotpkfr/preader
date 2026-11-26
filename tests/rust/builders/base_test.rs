use preader::IteratorOptions;
use rstest::rstest;

use crate::common::{
    constants::{TEST_ALPHABET, TEST_OTHER_STATE_NAME, TEST_STATE_NAME},
    fixtures::sandbox,
    funcs::items,
    sandbox::Sandbox,
};

#[rstest]
fn defaults_read_the_whole_file(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let read = items(sandbox.reader().bytes(&path).build().unwrap());

    assert_eq!(read, TEST_ALPHABET);
}

#[rstest]
fn end_stops_the_read_at_the_offset(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let read = items(sandbox.reader().bytes(&path).end(3).build().unwrap());

    assert_eq!(read, b"abc");
}

#[rstest]
fn last_limit_wins(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let read = items(sandbox.reader().bytes(&path).limit(1).limit(5).build().unwrap());

    assert_eq!(read, b"abcde");
}

#[rstest]
fn state_replaces_the_source(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let name = sandbox
        .reader()
        .bytes(&path)
        .state(TEST_STATE_NAME)
        .state(TEST_OTHER_STATE_NAME)
        .build()
        .unwrap()
        .state()
        .name
        .clone();

    assert_eq!(name, TEST_OTHER_STATE_NAME);
}

#[rstest]
fn each_setter_patches_one_option(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let read =
        items(sandbox.reader().bytes(&path).start(2).end(6).skip(1).limit(2).build().unwrap());

    assert_eq!(read, b"de");
}

#[rstest]
fn setter_after_options_patches_it(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let options = IteratorOptions {
        start: 1,
        ..IteratorOptions::default()
    };
    let read = items(sandbox.reader().bytes(&path).options(options).limit(1).build().unwrap());

    assert_eq!(read, b"b");
}

#[rstest]
fn options_replaces_every_field(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let options = IteratorOptions {
        start: 1,
        ..IteratorOptions::default()
    };
    let read = items(sandbox.reader().bytes(&path).limit(1).options(options).build().unwrap());

    assert_eq!(read, &TEST_ALPHABET[1..]);
}

#[rstest]
fn setters_can_be_chained_in_any_order(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.reader();
    let forward = items(reader.bytes(&path).start(2).end(6).skip(1).limit(2).build().unwrap());
    let reverse = items(reader.bytes(&path).limit(2).skip(1).end(6).start(2).build().unwrap());

    assert_eq!(forward, b"de");
    assert_eq!(forward, reverse);
}
