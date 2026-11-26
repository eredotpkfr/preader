use std::{assert_matches, fs};

use preader::{Error, Mismatch, StateSource};
use rstest::rstest;
use serde_json::json;

use crate::common::{
    constants::{TEST_LINE, TEST_OTHER_STATE_NAME, TEST_STATE_NAME, TEST_UNTRACKED_NAME},
    fixtures::sandbox,
    funcs::consume,
    macros::asserts::assert_err_is,
    sandbox::Sandbox,
};

#[rstest]
fn default_is_auto() {
    assert_matches!(StateSource::default(), StateSource::Auto);
}

#[rstest]
fn none_is_auto() {
    assert_matches!(StateSource::from(Option::<&str>::None), StateSource::Auto);
}

#[rstest]
fn missing_name_starts_a_fresh_state(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let state = sandbox.named_state(&path, TEST_OTHER_STATE_NAME);

    assert_eq!(state.name, TEST_OTHER_STATE_NAME);
    assert_eq!(state.position, 0);
}

#[rstest]
fn auto_load_starts_fresh_without_a_saved_state(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let bytes = sandbox.resuming().bytes(&path).build().unwrap();

    assert_eq!(bytes.state().position, 0);
}

#[rstest]
fn existing_state_accepts_an_unboxed_state(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let state = sandbox.state(&path);

    assert_matches!(
        StateSource::from(state.clone()),
        StateSource::Existing(boxed) if *boxed == state
    );
}

#[rstest]
#[case::borrowed_str(StateSource::from(TEST_STATE_NAME))]
#[case::borrowed_string(StateSource::from(&TEST_STATE_NAME.to_owned()))]
#[case::owned_string(StateSource::from(TEST_STATE_NAME.to_owned()))]
#[case::some(StateSource::from(Some(TEST_STATE_NAME)))]
fn name_accepts_every_string_shape(sandbox: Sandbox, #[case] source: StateSource) {
    let path = sandbox.line_file();
    let bytes = sandbox.reader().bytes(&path).state(source).build().unwrap();

    assert_eq!(bytes.state().name, TEST_STATE_NAME);
}

#[rstest]
fn existing_state_accepts_a_box(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let state = sandbox.state(&path);
    let source = StateSource::from(Box::new(state.clone()));

    assert_matches!(source, StateSource::Existing(boxed) if *boxed == state);
}

#[rstest]
fn auto_is_stable_for_the_same_file(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let first = sandbox.reader().bytes(&path).build().unwrap().state().name.clone();

    assert_eq!(
        first,
        sandbox.reader().bytes(&path).build().unwrap().state().name
    );
}

#[rstest]
fn build_fails_when_the_existing_state_tracks_another_file_without_verification(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let other = sandbox.write(TEST_UNTRACKED_NAME, b"bar");
    let state = sandbox.state(&path);

    assert_err_is!(
        sandbox.lenient().bytes(&other).state(state).build(),
        Error::Mismatch {
            kind: Mismatch::Path,
            ..
        }
    );
}

#[rstest]
fn name_is_used_verbatim(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let source = StateSource::from("sub/job-1.state.json");

    assert_eq!(
        sandbox.reader().bytes(&path).state(source).build().unwrap().state().name,
        "sub/job-1.state.json"
    );
}

#[rstest]
fn named_state_fails_when_the_payload_names_another_state(sandbox: Sandbox) {
    let path = sandbox.line_file();

    sandbox.saved(&path, TEST_STATE_NAME);
    sandbox.patch(TEST_STATE_NAME, |payload| {
        payload["name"] = json!(TEST_OTHER_STATE_NAME)
    });

    assert_err_is!(
        sandbox.reader().bytes(&path).state(TEST_STATE_NAME).build(),
        Error::Mismatch {
            kind: Mismatch::Name { .. },
            ..
        }
    );
}

#[rstest]
fn auto_derives_the_name_from_the_file(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let other = sandbox.write(TEST_UNTRACKED_NAME, TEST_LINE);
    let name = sandbox.reader().bytes(&path).build().unwrap().state().name.clone();

    assert_eq!(name.len(), 64);

    assert_ne!(
        name,
        sandbox.reader().bytes(&other).build().unwrap().state().name
    );
}

#[rstest]
fn build_fails_when_the_existing_state_is_unsaved(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let mut bytes = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    consume(&mut bytes, 2);

    let state = bytes.state().clone();

    assert_err_is!(
        sandbox.reader().bytes(&path).state(state).build(),
        Error::Mismatch {
            kind: Mismatch::Unsealed,
            ..
        }
    );
}

#[rstest]
fn existing_state_is_carried_through(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let mut bytes = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    consume(&mut bytes, 2);

    let state = bytes.save().unwrap().clone();

    let resumed = sandbox.reader().bytes(&path).state(state).build().unwrap();

    assert_eq!(resumed.state().position, 1);
}

#[rstest]
fn build_fails_when_an_unsaved_state_tracks_a_moved_file(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let moved = sandbox.path().join("moved.bin");
    let mut bytes = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    consume(&mut bytes, 2);

    let state = bytes.state().clone();

    fs::rename(&path, &moved).unwrap();

    assert_err_is!(
        sandbox.reader().bytes(&moved).state(state).build(),
        Error::Mismatch {
            kind: Mismatch::Unsealed,
            ..
        }
    );
}

#[rstest]
fn named_state_resumes_only_a_matching_file(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let other = sandbox.write(TEST_UNTRACKED_NAME, TEST_LINE);
    let reader = sandbox.reader();
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    consume(&mut bytes, 2);

    bytes.save().unwrap();

    assert_eq!(
        reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap().state().position,
        1
    );

    assert_err_is!(
        reader.bytes(&other).state(TEST_STATE_NAME).build(),
        Error::Mismatch {
            kind: Mismatch::Path,
            ..
        }
    );
}
