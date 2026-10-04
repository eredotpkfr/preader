use std::{fs, io::ErrorKind};

use preader::{Error, IteratorBuild, IteratorRead, Mismatch, PathError};
use rstest::rstest;

use crate::common::{
    constants::{TEST_INVALID_UTF8, TEST_MISSING_STATE_NAME, TEST_STATE_NAME},
    fixtures::sandbox,
    guards::set_pre_epoch_mtime,
    macros::{asserts::assert_err_is, skip::skip},
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
fn serde_errors_are_transparent(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let state = sandbox.saved(&path, TEST_STATE_NAME);

    fs::write(state.path().unwrap(), b"{ not json").unwrap();

    let error = sandbox.states().load(TEST_STATE_NAME).unwrap_err();

    assert!(matches!(error, Error::Serde(_)), "{error}");
    assert!(error.to_string().contains("line 1 column"), "{error}");
}

#[rstest]
fn regex_errors_are_transparent(sandbox: Sandbox) {
    let error = sandbox.states().search("[").unwrap_err();

    assert!(matches!(error, Error::Regex(_)), "{error}");
    assert!(error.to_string().contains("regex parse error"), "{error}");
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
fn time_errors_are_transparent(sandbox: Sandbox) {
    let path = sandbox.line_file();

    if !set_pre_epoch_mtime(&path) {
        skip!("a pre-epoch mtime cannot be set here");
    }

    let error = sandbox.reader().bytes(&path).build().unwrap_err();

    assert!(matches!(error, Error::Time(_)), "{error}");
}

#[rstest]
fn path_errors_are_transparent(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let mut state = sandbox.named_state(&path, "../escape");
    let error = state.save().unwrap_err();

    assert!(
        matches!(error, Error::Path(PathError::Escapes(_))),
        "{error}"
    );

    assert_eq!(error.to_string(), "path escapes root: ../escape");
}

#[rstest]
fn mismatch_errors_are_transparent(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let state = sandbox.stored(&path, TEST_STATE_NAME);

    sandbox.append(&path, b"more");

    let error = state.verify().unwrap_err();

    assert!(
        matches!(error, Error::Mismatch(Mismatch::Size { .. })),
        "{error}"
    );
}

#[rstest]
fn missing_state_names_itself(sandbox: Sandbox) {
    let error = sandbox.states().load(TEST_MISSING_STATE_NAME).unwrap_err();

    assert!(matches!(&error, Error::NotFound(name) if name == TEST_MISSING_STATE_NAME));

    assert_eq!(
        error.to_string(),
        format!("state not found: {TEST_MISSING_STATE_NAME}")
    );
}

#[rstest]
fn inverted_range_reports_both_bounds(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let error = sandbox.reader().bytes(&path).start(9).end(4).build().unwrap_err();

    assert!(matches!(error, Error::InvalidRange { start: 9, end: 4 }));

    assert_eq!(error.to_string(), "start (9) must be <= end (4)");
}

#[rstest]
fn directory_is_not_a_file(sandbox: Sandbox) {
    let directory = sandbox.dir_at("folder").canonicalize().unwrap();
    let error = sandbox.reader().bytes(&directory).build().unwrap_err();

    assert!(
        matches!(&error, Error::NotAFile(found) if *found == directory),
        "{error}"
    );

    assert_eq!(
        error.to_string(),
        format!("not a file: {}", directory.display())
    );
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
