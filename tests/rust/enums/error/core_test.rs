use std::{fs, io::ErrorKind};

use preader::{Error, IteratorBuild, IteratorRead, Mismatch, NameError};
use rstest::rstest;

use crate::common::{
    constants::{TEST_INVALID_UTF8, TEST_MISSING_STATE_NAME, TEST_STATE_NAME},
    fixtures::sandbox,
    funcs::state_data,
    macros::asserts::{assert_err_eq, assert_err_is},
    sandbox::Sandbox,
};

#[rstest]
fn io_errors_are_transparent(sandbox: Sandbox) {
    let missing = sandbox.path().join("missing.bin");

    assert_err_is!(
        sandbox.reader().bytes(&missing).build(),
        Error::Io(io) if io.kind() == ErrorKind::NotFound
    );
}

#[rstest]
fn corrupt_state_names_the_state(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let state = sandbox.saved(&path, TEST_STATE_NAME);

    fs::write(state.path().unwrap(), b"{ not json").unwrap();

    assert_err_eq!(
        sandbox.states().load(TEST_STATE_NAME),
        "state 'job-1' is corrupt: key must be a string at line 1 column 3"
    );
    assert_err_is!(
        sandbox.states().load(TEST_STATE_NAME),
        Error::Corrupt { name, .. } if name == TEST_STATE_NAME
    );
}

#[rstest]
fn regex_errors_are_transparent(sandbox: Sandbox) {
    assert_err_eq!(
        sandbox.states().search("["),
        "regex parse error:\n    [\n    ^\nerror: unclosed character class"
    );

    assert_err_is!(sandbox.states().search("["), Error::Regex(_));
}

#[rstest]
fn utf8_errors_are_transparent(sandbox: Sandbox) {
    let path = sandbox.file(TEST_INVALID_UTF8);
    let mut lines = sandbox.reader().lines(&path).build().unwrap();
    let error = lines.read().unwrap_err();

    assert!(matches!(error, Error::Utf8(_)), "{error}");
    assert!(error.to_string().contains("invalid utf-8"), "{error}");
}

#[rstest]
fn name_errors_are_transparent(sandbox: Sandbox) {
    let mut data = state_data(sandbox.line_file());

    data.name = "../escape".to_owned();

    let mut state = sandbox.manager().state(data);
    let error = state.save().unwrap_err();

    assert!(matches!(error, Error::Name(NameError::Escapes)), "{error}");

    assert_eq!(error.to_string(), "state name escapes the state directory");
}

#[rstest]
fn mismatch_errors_are_transparent(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let state = sandbox.stored(&path, TEST_STATE_NAME);

    sandbox.append(&path, b"more");

    assert_err_is!(state.verify(), Error::Mismatch(Mismatch::Size));
}

#[rstest]
fn missing_state_names_itself(sandbox: Sandbox) {
    assert_err_eq!(
        sandbox.states().load(TEST_MISSING_STATE_NAME),
        format!("state '{TEST_MISSING_STATE_NAME}' not found")
    );

    assert_err_is!(
        sandbox.states().load(TEST_MISSING_STATE_NAME),
        Error::NotFound(name) if name == TEST_MISSING_STATE_NAME
    );
}

#[rstest]
fn inverted_range_describes_itself(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let reader = sandbox.reader();

    assert_err_eq!(
        reader.bytes(&path).start(9).end(4).build(),
        "start must not be greater than end"
    );

    assert_err_is!(
        reader.bytes(&path).start(9).end(4).build(),
        Error::InvalidRange
    );
}

#[rstest]
fn a_directory_is_reported_as_one(sandbox: Sandbox) {
    let directory = sandbox.dir_at("folder");

    assert_err_eq!(sandbox.reader().bytes(&directory).build(), "is a directory");
}

#[rstest]
fn a_file_state_dir_is_reported_as_not_a_directory(sandbox: Sandbox) {
    sandbox.block_states();

    assert_err_eq!(sandbox.states().names(), "not a directory");
}

#[rstest]
fn transparent_variant_borrows_its_message(sandbox: Sandbox) {
    let missing = sandbox.path().join("missing.bin");
    let error = sandbox.reader().bytes(&missing).build().unwrap_err();
    let Error::Io(inner) = &error else {
        panic!("{error}");
    };

    assert_eq!(error.to_string(), inner.to_string());
}
