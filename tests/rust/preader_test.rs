#[cfg(unix)]
use std::fs;
use std::{ffi::OsString, path::PathBuf};

use preader::{Config, IteratorBuild, IteratorRead, PReader};
use rstest::rstest;

#[cfg(unix)]
use crate::common::{constants::TEST_NON_UTF8_NAME, macros::asserts::assert_err};
use crate::common::{
    constants::{TEST_FILE_NAME, TEST_LINE, TEST_STATE_NAME},
    fixtures::sandbox,
    funcs::{canonical, drain, items},
    sandbox::Sandbox,
};

#[rstest]
fn new_matches_the_default_config() {
    assert_eq!(*PReader::new().config(), Config::default());
    assert_eq!(*PReader::default().config(), Config::default());
}

#[rstest]
fn config_converts_into_a_reader(sandbox: Sandbox) {
    let config = sandbox.config();

    assert_eq!(*PReader::from(config.clone()).config(), config);
}

#[rstest]
fn states_point_at_the_configured_directory(sandbox: Sandbox) {
    let reader = sandbox.reader();

    assert_eq!(reader.states().state_dir(), reader.config().state_dir);
    assert_eq!(
        reader.states().path(TEST_STATE_NAME).unwrap().parent(),
        Some(reader.config().state_dir.as_path())
    );
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

    assert_eq!(named.to_vec(), vec![name; 4]);
}

#[rstest]
fn every_iterator_reads_a_single_byte_file(sandbox: Sandbox) {
    let path = sandbox.file(b"a");
    let reader = sandbox.reader();

    assert_eq!(items(reader.bytes(&path).build().unwrap()), b"a");
    assert_eq!(items(reader.chunks(&path).build().unwrap()), [b"a"]);
    assert_eq!(items(reader.lines(&path).build().unwrap()), ["a"]);
    assert_eq!(items(reader.delimiter(&path).build().unwrap()), [b"a"]);
}

#[rstest]
fn every_iterator_counts_invalid_bytes(sandbox: Sandbox) {
    let content = b"foo\n\xff\xfe\nbar\n";
    let path = sandbox.file(content);
    let reader = sandbox.reader();
    let size = content.len() as u64;

    let mut bytes = reader.bytes(&path).build().unwrap();
    let mut chunks = reader.chunks(&path).size(4).build().unwrap();
    let mut segments = reader.delimiter(&path).character(b'\n').build().unwrap();
    let mut lines = reader.lines(&path).build().unwrap();

    drain(&mut bytes);
    drain(&mut chunks);
    drain(&mut segments);

    lines.by_ref().for_each(drop);

    assert_eq!(bytes.state().position, size, "bytes");
    assert_eq!(chunks.state().position, size, "chunks");
    assert_eq!(segments.state().position, size, "delimiter");
    assert_eq!(lines.state().position, size, "lines");
}

#[rstest]
fn unnormalized_path_resolves_to_same_name(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let detoured = sandbox.path().join(".").join(TEST_FILE_NAME);
    let reader = sandbox.reader();

    assert_eq!(
        reader.bytes(&path).build().unwrap().state().name,
        reader.bytes(&detoured).build().unwrap().state().name
    );
}

#[rstest]
fn long_path_still_produces_a_digest_name(sandbox: Sandbox) {
    let path = sandbox.write(&format!("{}.bin", "y".repeat(180)), TEST_LINE);

    assert_eq!(
        sandbox.reader().bytes(&path).build().unwrap().state().name.len(),
        64
    );
}

#[rstest]
#[case::spaces("a b.bin")]
#[case::quote("a'b.bin")]
#[case::dollar("a$b.bin")]
#[case::accents("café.bin")]
#[case::ideographs("日本語.bin")]
#[case::semicolon("a;b.bin")]
fn unusual_path_is_read_verbatim(sandbox: Sandbox, #[case] name: &str) {
    let path = sandbox.write(name, TEST_LINE);

    assert_eq!(
        items(sandbox.reader().bytes(&path).build().unwrap()).len(),
        TEST_LINE.len()
    );
}

#[cfg(unix)]
#[rstest]
fn symlink_is_resolved_to_its_target(sandbox: Sandbox) {
    use std::os::unix::fs::symlink;

    let target = sandbox.line_file();
    let link = sandbox.path().join("link.bin");

    symlink(&target, &link).unwrap();

    assert_eq!(
        sandbox.reader().bytes(&link).build().unwrap().state().file.path,
        canonical(&target)
    );
}

#[cfg(unix)]
#[rstest]
fn socket_is_rejected(sandbox: Sandbox) {
    use std::os::unix::net::UnixListener;

    let socket = sandbox.path().join("a-socket");
    let _listener = UnixListener::bind(&socket).unwrap();

    assert_err!(sandbox.reader().bytes(&socket).build(), "not a file");
}

#[cfg(unix)]
#[rstest]
fn non_utf8_path_is_rejected(sandbox: Sandbox) {
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

    let path = sandbox.path().join(OsStr::from_bytes(TEST_NON_UTF8_NAME));

    if fs::write(&path, TEST_LINE).is_err() {
        return;
    }

    assert_err!(sandbox.reader().bytes(&path).build(), "invalid UTF-8");
}
