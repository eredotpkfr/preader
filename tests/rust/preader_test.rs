use std::{ffi::OsString, path::PathBuf};

use preader::{Config, IteratorRead, PReader};
use rstest::rstest;

use crate::common::{
    constants::{TEST_ALPHABET, TEST_BLANK_SEGMENT_CONTENT, TEST_LINE_CONTENT, TEST_STATE_NAME},
    fixtures::sandbox,
    funcs::{canonical, items},
    sandbox::Sandbox,
};

#[rstest]
fn defaults() {
    assert_eq!(*PReader::new().config(), Config::default());
    assert_eq!(*PReader::default().config(), Config::default());
}

#[rstest]
fn fields(sandbox: Sandbox) {
    let config = sandbox.config();

    assert_eq!(*PReader::from(config.clone()).config(), config);
}

#[rstest]
fn bytes_reads_the_file(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let bytes = sandbox.reader().bytes(&path).build().unwrap();

    assert_eq!(bytes.state().file.path, canonical(&path));
    assert_eq!(items(bytes), TEST_ALPHABET);
}

#[rstest]
fn chunks_reads_the_file(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let chunks = sandbox.reader().chunks(&path).build().unwrap();

    assert_eq!(chunks.state().file.path, canonical(&path));
    assert_eq!(items(chunks), [TEST_ALPHABET]);
}

#[rstest]
fn states_share_the_configured_state_dir(sandbox: Sandbox) {
    let reader = sandbox.reader();

    assert_eq!(reader.states().state_dir(), reader.config().state_dir);
    assert_eq!(
        reader.states().path(TEST_STATE_NAME).unwrap().parent(),
        Some(reader.config().state_dir.as_path())
    );
}

#[rstest]
fn segments_reads_the_file(sandbox: Sandbox) {
    let path = sandbox.file(TEST_BLANK_SEGMENT_CONTENT);
    let segments = sandbox.reader().segments(&path).build().unwrap();

    assert_eq!(segments.state().file.path, canonical(&path));
    assert_eq!(items(segments), [b"seg-0".as_slice(), b"", b"seg-2"]);
}

#[rstest]
fn lines_reads_the_file(sandbox: Sandbox) {
    let path = sandbox.file(TEST_LINE_CONTENT);
    let lines = sandbox.reader().lines(&path).build().unwrap();

    assert_eq!(lines.state().file.path, canonical(&path));
    assert_eq!(
        items(lines),
        ["line-0", "line-1", "line-2", "line-3", "line-4", "line-5"]
    );
}

#[rstest]
fn state_argument_accepts_every_name_shape(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let name = TEST_STATE_NAME.to_owned();
    let reader = sandbox.reader();

    let named = [
        reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap().state().name.clone(),
        reader.bytes(&path).state(&name).build().unwrap().state().name.clone(),
        reader.bytes(&path).state(name.clone()).build().unwrap().state().name.clone(),
        reader
            .bytes(&path)
            .state(Some(TEST_STATE_NAME))
            .build()
            .unwrap()
            .state()
            .name
            .clone(),
    ];

    assert_eq!(named, [TEST_STATE_NAME; 4]);
}

#[rstest]
fn file_argument_accepts_every_ownership_shape(sandbox: Sandbox) {
    let owned = sandbox.line_file();
    let text = owned.to_str().unwrap().to_owned();
    let os = OsString::from(&text);
    let reader = sandbox.reader();
    let resolved = canonical(&owned);

    let shapes: [PathBuf; 6] = [
        reader.bytes(text.as_str()).build().unwrap().state().file.path.clone(),
        reader.bytes(&text).build().unwrap().state().file.path.clone(),
        reader.bytes(text.clone()).build().unwrap().state().file.path.clone(),
        reader.bytes(owned.as_path()).build().unwrap().state().file.path.clone(),
        reader.bytes(&owned).build().unwrap().state().file.path.clone(),
        reader.bytes(os).build().unwrap().state().file.path.clone(),
    ];

    assert_eq!(shapes.to_vec(), vec![resolved; 6]);

    let mut moved = reader.bytes(owned).build().unwrap();

    assert_eq!(moved.read().unwrap(), Some(b'f'));
}
