use preader::{Config, IteratorBuild, IteratorRead, State, StateSource};
use rstest::rstest;

use crate::common::{
    constants::{TEST_LINE, TEST_OTHER_STATE_NAME, TEST_STATE_NAME},
    fixtures::sandbox,
    funcs::canonical,
    macros::asserts::assert_err,
    sandbox::Sandbox,
};

#[rstest]
fn default_is_auto() {
    assert!(matches!(StateSource::default(), StateSource::Auto));
}

#[rstest]
fn auto_derives_the_name_from_the_file(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let other = sandbox.write("other.bin", TEST_LINE);
    let name = sandbox.reader().bytes(&path).build().unwrap().state().name.clone();

    assert_eq!(name.len(), 64);

    assert_ne!(
        name,
        sandbox.reader().bytes(&other).build().unwrap().state().name
    );
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
fn name_accepts_every_string_shape(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let owned = TEST_STATE_NAME.to_owned();
    let reader = sandbox.reader();

    for source in [
        StateSource::from(TEST_STATE_NAME),
        StateSource::from(&owned),
        StateSource::from(owned.clone()),
        StateSource::from(Some(TEST_STATE_NAME)),
    ] {
        assert_eq!(
            reader.bytes(&path).state(source).build().unwrap().state().name,
            TEST_STATE_NAME
        );
    }
}

#[rstest]
fn none_falls_back_to_auto(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let reader = sandbox.reader();
    let source: StateSource = Option::<&str>::None.into();

    assert_eq!(
        reader.bytes(&path).state(source).build().unwrap().state().name.len(),
        64
    );
}

#[rstest]
fn existing_state_is_carried_through(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let mut bytes = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    bytes.read().unwrap();
    bytes.state().save().unwrap();

    let state = bytes.state().clone();

    drop(bytes);

    let mut resumed = sandbox.reader().bytes(&path).state(state).build().unwrap();

    assert_eq!(resumed.state().position, 1);
}

#[rstest]
fn advanced_state_needs_save_before_it_is_reused(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let mut bytes = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    bytes.read().unwrap();

    let state = bytes.state().clone();

    drop(bytes);

    let error = sandbox.reader().bytes(&path).state(state).build().unwrap_err();

    assert!(
        error.to_string().contains("state checksum mismatch"),
        "{error}"
    );
}

#[rstest]
fn existing_state_accepts_a_box(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let state = sandbox.state(&path);
    let source = StateSource::from(Box::new(state));

    assert!(matches!(source, StateSource::Existing(_)));
}

#[rstest]
fn name_is_normalized_before_it_is_used(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let source = StateSource::from(format!("{TEST_STATE_NAME}.state.json"));

    assert_eq!(
        sandbox.reader().bytes(&path).state(source).build().unwrap().state().name,
        TEST_STATE_NAME
    );
}

#[rstest]
fn auto_load_resumes_only_a_matching_file(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let other = sandbox.write("other.bin", TEST_LINE);
    let reader = sandbox.reader_with(Config {
        auto_load_state: true,
        ..sandbox.config()
    });
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    bytes.read().unwrap();
    bytes.state().save().unwrap();

    drop(bytes);

    assert_eq!(
        reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap().state().position,
        1
    );
    assert_eq!(
        reader.bytes(&other).state(TEST_STATE_NAME).build().unwrap().state().position,
        0
    );
}

#[rstest]
fn existing_state_is_rejected_for_another_file(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let other = sandbox.write("other.bin", TEST_LINE);
    let state: State = sandbox.state(&path);

    assert_err!(
        sandbox.reader().bytes(&other).state(state).build(),
        "file path mismatch",
    );
}

#[rstest]
fn existing_state_skips_check_without_verification(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let other = sandbox.write("other.bin", TEST_LINE);
    let state = sandbox.state(&path);
    let mut resumed = sandbox.lenient().bytes(&other).state(state).build().unwrap();

    assert_eq!(resumed.state().file.path, canonical(&path));
}

#[rstest]
fn missing_name_starts_a_fresh_state(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let state = sandbox.named_state(&path, TEST_OTHER_STATE_NAME);

    assert_eq!(state.name, TEST_OTHER_STATE_NAME);
    assert_eq!(state.position, 0);
}
