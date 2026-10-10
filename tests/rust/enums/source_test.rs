use preader::{Error, IteratorBuild, IteratorRead, Mismatch, PathError, State, StateSource};
use rstest::rstest;
use rstest_reuse::apply;

use crate::common::{
    constants::{TEST_LINE, TEST_OTHER_STATE_NAME, TEST_STATE_NAME},
    fixtures::sandbox,
    funcs::canonical,
    macros::asserts::assert_err_is,
    rule::Rule,
    sandbox::Sandbox,
    templates::name::{device_names, invalid_names, unportable_characters, windows_invalid_names},
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
fn build_fails_when_the_existing_state_is_unsaved(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let mut bytes = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    bytes.read().unwrap();

    let state = bytes.state().clone();

    drop(bytes);

    let recorded = state.checksum.clone();

    assert_err_is!(
        sandbox.reader().bytes(&path).state(state).build(),
        Error::Mismatch(Mismatch::Checksum { saved, .. }) if *saved == recorded
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
fn name_is_used_verbatim(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let source = StateSource::from("sub/job-1.state.json");

    assert_eq!(
        sandbox.reader().bytes(&path).state(source).build().unwrap().state().name,
        "sub/job-1.state.json"
    );
}

#[rstest]
fn auto_load_resumes_only_a_matching_file(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let other = sandbox.write("other.bin", TEST_LINE);
    let reader = sandbox.resuming();
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    bytes.read().unwrap();
    bytes.state().save().unwrap();

    drop(bytes);

    assert_eq!(
        reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap().state().position,
        1
    );

    assert_err_is!(
        reader.bytes(&other).state(TEST_STATE_NAME).build(),
        Error::Mismatch(Mismatch::Path { .. })
    );
}

#[rstest]
fn auto_load_starts_fresh_without_a_saved_state(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let mut bytes = sandbox.resuming().bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    assert_eq!(bytes.state().position, 0);
}

#[apply(invalid_names)]
fn build_fails_when_the_name_is_invalid(sandbox: Sandbox, #[case] name: &str, #[case] rule: Rule) {
    let path = sandbox.line_file();

    assert_err_is!(
        sandbox.reader().bytes(&path).state(name).build(),
        Error::Path(error) if rule.matches(error, name)
    );
}

#[apply(windows_invalid_names)]
fn build_fails_when_the_name_is_not_portable(sandbox: Sandbox, #[case] name: &str) {
    let path = sandbox.line_file();

    assert_err_is!(
        sandbox.reader().bytes(&path).state(name).build(),
        Error::Path(PathError::Invalid(found)) if found == name
    );
}

#[apply(unportable_characters)]
fn build_fails_when_a_character_is_not_portable(
    sandbox: Sandbox,
    #[case] character: char,
    #[values("job{}1", "sub{}/job-1")] shape: &str,
) {
    let path = sandbox.line_file();
    let name = shape.replace("{}", &character.to_string());

    assert_err_is!(
        sandbox.reader().bytes(&path).state(name.as_str()).build(),
        Error::Path(PathError::Invalid(found)) if *found == name
    );
}

#[rstest]
fn build_fails_when_the_existing_state_tracks_another_file(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let other = sandbox.write("other.bin", TEST_LINE);
    let state: State = sandbox.state(&path);

    let recorded = state.file.path.clone();

    assert_err_is!(
        sandbox.reader().bytes(&other).state(state).build(),
        Error::Mismatch(Mismatch::Path { saved, current })
            if *saved == recorded && *current == canonical(&other)
    );
}

#[rstest]
fn build_fails_when_the_existing_state_tracks_another_file_without_verification(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let other = sandbox.write("other.bin", TEST_LINE);
    let state = sandbox.state(&path);

    assert_err_is!(
        sandbox.lenient().bytes(&other).state(state).build(),
        Error::Mismatch(Mismatch::Path { .. })
    );
}

#[rstest]
fn missing_name_starts_a_fresh_state(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let state = sandbox.named_state(&path, TEST_OTHER_STATE_NAME);

    assert_eq!(state.name, TEST_OTHER_STATE_NAME);
    assert_eq!(state.position, 0);
}

#[rstest]
fn auto_load_fails_when_the_payload_names_another_state(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let state = sandbox.saved(&path, TEST_STATE_NAME);
    let payload = state.path().unwrap();
    let patched = std::fs::read_to_string(&payload).unwrap().replace(
        &format!("\"name\": \"{TEST_STATE_NAME}\""),
        &format!("\"name\": \"{TEST_OTHER_STATE_NAME}\""),
    );

    std::fs::write(&payload, patched).unwrap();

    assert_err_is!(
        sandbox.resuming().bytes(&path).state(TEST_STATE_NAME).build(),
        Error::Mismatch(Mismatch::Name { .. })
    );
}

#[apply(device_names)]
fn build_fails_when_the_name_is_a_device(sandbox: Sandbox, #[case] name: &str) {
    let path = sandbox.line_file();

    assert_err_is!(
        sandbox.reader().bytes(&path).state(name).build(),
        Error::Path(PathError::Invalid(found)) if found == name
    );
}
