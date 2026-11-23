#[cfg(unix)]
use std::os::unix::fs::symlink;
use std::{assert_matches, fs, io::ErrorKind, path::Path, time::Duration};

use preader::{Error, IteratorRead, Mismatch, NameError};
use rstest::rstest;

use crate::common::{
    constants::{
        TEST_INVALID_UTF8, TEST_LINE, TEST_MISSING_STATE_NAME, TEST_OTHER_STATE_NAME,
        TEST_STATE_NAME, TEST_TRACKED_NAME, TEST_UNTRACKED_NAME,
    },
    fixtures::sandbox,
    funcs::consume,
    guards::{mtime, set_mtime},
    macros::asserts::{assert_err_eq, assert_err_is},
    sandbox::Sandbox,
};

type Cause = fn(&Sandbox, &Path) -> Error;

fn moved(sandbox: &Sandbox, path: &Path) -> Error {
    let moved = sandbox.path().join("moved.bin");

    fs::rename(path, &moved).unwrap();

    sandbox.reader().bytes(&moved).state(TEST_STATE_NAME).build().unwrap_err()
}

fn grown(sandbox: &Sandbox, path: &Path) -> Error {
    sandbox.append(path, b"more");

    sandbox.states().load(TEST_STATE_NAME).unwrap_err()
}

fn touched(sandbox: &Sandbox, path: &Path) -> Error {
    set_mtime(path, mtime(path) + Duration::from_secs(3600));

    sandbox.states().load(TEST_STATE_NAME).unwrap_err()
}

fn deleted(sandbox: &Sandbox, path: &Path) -> Error {
    fs::remove_file(path).unwrap();

    sandbox.states().load(TEST_STATE_NAME).unwrap_err()
}

fn replaced(sandbox: &Sandbox, _path: &Path) -> Error {
    let untracked = sandbox.write(TEST_UNTRACKED_NAME, b"bar\n");

    sandbox.states().resync(TEST_STATE_NAME, &untracked).unwrap_err()
}

fn unsaved(sandbox: &Sandbox, path: &Path) -> Error {
    let reader = sandbox.reader();
    let mut bytes = reader.bytes(path).state(TEST_STATE_NAME).build().unwrap();

    consume(&mut bytes, 2);

    let state = bytes.state().clone();

    reader.bytes(path).state(state).build().unwrap_err()
}

fn unparsable(sandbox: &Sandbox, _path: &Path) -> Error {
    sandbox.write_state(TEST_STATE_NAME, "not valid json");

    sandbox.states().load(TEST_STATE_NAME).unwrap_err()
}

fn tampered(sandbox: &Sandbox, _path: &Path) -> Error {
    sandbox.tamper(TEST_STATE_NAME);

    sandbox.states().load(TEST_STATE_NAME).unwrap_err()
}

fn misplaced(sandbox: &Sandbox, _path: &Path) -> Error {
    let registry = sandbox.states();

    fs::copy(
        registry.path(TEST_STATE_NAME).unwrap(),
        registry.path(TEST_OTHER_STATE_NAME).unwrap(),
    )
    .unwrap();

    registry.load(TEST_OTHER_STATE_NAME).unwrap_err()
}

fn overtaken(sandbox: &Sandbox, path: &Path) -> Error {
    let reader = sandbox.reader();
    let mut first = reader.bytes(path).state(TEST_STATE_NAME).build().unwrap();
    let mut second = reader.bytes(path).state(TEST_STATE_NAME).build().unwrap();

    consume(&mut first, 2);

    first.save().unwrap();
    second.save().unwrap_err()
}

#[rstest]
fn invalid_mtime_describes_itself() {
    assert_eq!(
        Error::InvalidMtime.to_string(),
        "file mtime is out of range"
    );
}

#[rstest]
fn a_file_state_dir_is_reported_as_not_a_directory(sandbox: Sandbox) {
    sandbox.block_states();

    assert_err_eq!(sandbox.states().names(), "not a directory");
}

#[rstest]
fn a_directory_is_reported_as_one(sandbox: Sandbox) {
    let directory = sandbox.dir_at("folder");

    assert_err_eq!(sandbox.reader().bytes(&directory).build(), "is a directory");
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
fn regex_errors_are_transparent(sandbox: Sandbox) {
    assert_err_eq!(
        sandbox.states().search("["),
        "regex parse error:\n    [\n    ^\nerror: unclosed character class"
    );

    assert_err_is!(sandbox.states().search("["), Error::Regex(_));
}

#[rstest]
fn serde_errors_are_transparent() {
    let inner = serde_json::from_str::<u8>("x").unwrap_err();
    let message = inner.to_string();
    let error = Error::from(inner);

    assert_eq!(error.to_string(), message);

    assert_matches!(error, Error::Serde(_));
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
fn utf8_errors_are_transparent(sandbox: Sandbox) {
    let path = sandbox.file(TEST_INVALID_UTF8);
    let mut lines = sandbox.reader().lines(&path).build().unwrap();
    let error = lines.read().unwrap_err();
    let Error::Utf8(inner) = &error else {
        panic!("{error}");
    };

    assert_eq!(error.to_string(), inner.to_string());
}

#[rstest]
fn corrupt_state_names_the_state(sandbox: Sandbox) {
    sandbox.write_state(TEST_STATE_NAME, b"{ not json");

    assert_err_eq!(
        sandbox.states().load(TEST_STATE_NAME),
        format!(
            "state '{TEST_STATE_NAME}': state file is not valid (key must be a string at line 1 column 3)"
        )
    );

    assert_err_is!(
        sandbox.states().load(TEST_STATE_NAME),
        Error::Corrupt { name, .. } if name == TEST_STATE_NAME
    );
}

#[rstest]
#[case::moved(moved, |error: &Error| matches!(error, Error::Mismatch { kind: Mismatch::Path, .. }), "file path changed", TEST_STATE_NAME)]
#[case::grown(grown, |error: &Error| matches!(error, Error::Mismatch { kind: Mismatch::Size, .. }), "file size changed", TEST_STATE_NAME)]
#[case::touched(touched, |error: &Error| matches!(error, Error::Mismatch { kind: Mismatch::Mtime, .. }), "file mtime changed", TEST_STATE_NAME)]
#[case::deleted(deleted, |error: &Error| matches!(error, Error::Mismatch { kind: Mismatch::Missing, .. }), "file is missing", TEST_STATE_NAME)]
#[case::replaced(replaced, |error: &Error| matches!(error, Error::Mismatch { kind: Mismatch::Identity, .. }), "file is not the tracked file", TEST_STATE_NAME)]
#[case::unsaved(unsaved, |error: &Error| matches!(error, Error::Mismatch { kind: Mismatch::Unsealed, .. }), "has unsaved progress", TEST_STATE_NAME)]
#[case::unparsable(unparsable, |error: &Error| matches!(error, Error::Corrupt { .. }), "state file is not valid", TEST_STATE_NAME)]
#[case::tampered(tampered, |error: &Error| matches!(error, Error::Mismatch { kind: Mismatch::Checksum, .. }), "modified outside preader", TEST_STATE_NAME)]
#[case::misplaced(misplaced, |error: &Error| matches!(error, Error::Mismatch { kind: Mismatch::Name { .. }, .. }), &format!("belongs to '{TEST_STATE_NAME}'"), TEST_OTHER_STATE_NAME)]
#[case::overtaken(overtaken, |error: &Error| matches!(error, Error::Mismatch { kind: Mismatch::Stale, .. }), "saved elsewhere", TEST_STATE_NAME)]
fn causes_map_to_their_state_error(
    sandbox: Sandbox,
    #[case] cause: Cause,
    #[case] variant: fn(&Error) -> bool,
    #[case] message: &str,
    #[case] name: &str,
) {
    let path = sandbox.line_file();

    sandbox.saved(&path, TEST_STATE_NAME);

    let error = cause(&sandbox, &path);
    let text = error.to_string();

    assert!(variant(&error), "{error:?}");
    assert!(text.contains(message));
    assert!(text.starts_with(&format!("state '{name}': ")));
}

#[rstest]
fn name_errors_carry_the_bare_message(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let reader = sandbox.reader();

    assert_err_eq!(
        reader.bytes(&path).state("../job-1").build(),
        "state name escapes the state directory"
    );

    assert_err_is!(
        reader.bytes(&path).state("../job-1").build(),
        Error::Name(NameError::Escapes)
    );
}

#[rstest]
fn all_names_the_failing_state(sandbox: Sandbox) {
    let tracked = sandbox.write(TEST_TRACKED_NAME, TEST_LINE);
    let grown = sandbox.write("grown.bin", TEST_LINE);

    sandbox.saved(&tracked, TEST_STATE_NAME);
    sandbox.saved(&grown, TEST_OTHER_STATE_NAME);
    sandbox.append(&grown, b"more");

    assert_err_is!(
        sandbox.states().all(),
        Error::Mismatch {
            name,
            kind: Mismatch::Size,
        } if name == TEST_OTHER_STATE_NAME
    );
}

#[rstest]
fn io_errors_are_transparent(sandbox: Sandbox) {
    let missing = sandbox.path().join("missing.bin");
    let error = sandbox.reader().bytes(&missing).build().unwrap_err();
    let Error::Io(inner) = &error else {
        panic!("{error}");
    };

    assert_eq!(error.to_string(), inner.to_string());
    assert_eq!(inner.kind(), ErrorKind::NotFound);
}

#[rstest]
fn mismatch_errors_name_the_state(sandbox: Sandbox) {
    let path = sandbox.line_file();

    sandbox.saved(&path, TEST_STATE_NAME);
    sandbox.append(&path, b"more");

    assert_err_eq!(
        sandbox.states().load(TEST_STATE_NAME),
        format!(
            "state '{TEST_STATE_NAME}': file size changed (resync the state if this is expected)"
        )
    );

    assert_err_is!(
        sandbox.states().load(TEST_STATE_NAME),
        Error::Mismatch {
            kind: Mismatch::Size,
            ..
        }
    );
}

#[cfg(unix)]
#[rstest]
fn aliases_map_to_state_error(sandbox: Sandbox) {
    let name = format!("link/{TEST_STATE_NAME}");
    let outside = sandbox.dir_at("outside");

    fs::create_dir_all(sandbox.state_dir()).unwrap();
    symlink(&outside, sandbox.state_dir().join("link")).unwrap();

    assert_err_eq!(
        sandbox.states().path(&name),
        format!("state '{name}': name is an alias of another entry (symlink, case or Unicode)")
    );

    assert_err_is!(
        sandbox.states().path(&name),
        Error::Alias { name: alias } if *alias == name
    );
}

#[rstest]
fn resync_accepts_the_error_name_of_an_auto_named_state(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let reader = sandbox.resuming();
    let mut bytes = reader.bytes(&path).build().unwrap();

    consume(&mut bytes, 2);

    let saved = bytes.save().unwrap().clone();

    sandbox.append(&path, b"more");

    let error = reader.bytes(&path).build().unwrap_err();
    let Error::Mismatch {
        name,
        kind: Mismatch::Size,
    } = &error
    else {
        panic!("{error}");
    };

    reader.states().resync(name, &path).unwrap();

    assert_eq!(
        reader.bytes(&path).build().unwrap().state().position,
        saved.position
    );
}
