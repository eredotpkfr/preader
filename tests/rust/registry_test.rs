use std::fs;

use preader::{Error, STATE_FILE_EXTENSION};
use rstest::rstest;

use crate::common::{
    constants::{
        TEST_EVERY_DEPTH, TEST_MISSING_STATE_NAME, TEST_OTHER_STATE_NAME, TEST_STATE_NAME,
        TEST_UNSAFE_NAMES,
    },
    fixtures::sandbox,
    funcs::{names, native},
    guards::Blocked,
    macros::asserts::{assert_err, assert_err_is},
    sandbox::Sandbox,
};

fn corrupt(sandbox: &Sandbox, name: &str) {
    let path = sandbox.states().path(name).unwrap();

    path.parent().map(fs::create_dir_all).transpose().unwrap();
    fs::write(path, "not valid json").unwrap();
}

#[rstest]
fn state_dir_points_at_configured_directory(sandbox: Sandbox) {
    assert_eq!(sandbox.states().state_dir(), sandbox.config().state_dir);
}

#[rstest]
fn path_reports_the_state_file_location(sandbox: Sandbox) {
    let path = sandbox.states().path(TEST_STATE_NAME).unwrap();

    assert_eq!(path, sandbox.save(TEST_STATE_NAME).path().unwrap());
    assert_eq!(
        path.file_name().unwrap(),
        format!("{TEST_STATE_NAME}{STATE_FILE_EXTENSION}").as_str()
    );
}

#[rstest]
fn load_returns_the_saved_state(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);

    assert_eq!(
        sandbox.states().load(TEST_STATE_NAME).unwrap().name,
        TEST_STATE_NAME
    );
}

#[rstest]
fn load_reports_a_missing_state(sandbox: Sandbox) {
    assert_err_is!(sandbox.states().load(TEST_MISSING_STATE_NAME), Error::NotFound(name) if name == TEST_MISSING_STATE_NAME);
}

#[rstest]
fn load_reports_state_shaped_directory_as_missing(sandbox: Sandbox) {
    fs::create_dir_all(sandbox.states().path(TEST_STATE_NAME).unwrap()).unwrap();

    assert_err_is!(sandbox.states().load(TEST_STATE_NAME), Error::NotFound(_));
}

#[rstest]
fn load_returns_the_name_from_the_payload(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let state = sandbox.stored(&path, TEST_STATE_NAME);
    let payload = state.path().unwrap();
    let patched = fs::read_to_string(&payload).unwrap().replace(
        &format!("\"name\": \"{TEST_STATE_NAME}\""),
        "\"name\": \"a-different-name\"",
    );

    fs::write(&payload, patched).unwrap();

    assert_eq!(
        sandbox.lenient().states().load(TEST_STATE_NAME).unwrap().name,
        "a-different-name"
    );
}

#[rstest]
#[case::traversal(TEST_UNSAFE_NAMES[0])]
#[case::absolute(TEST_UNSAFE_NAMES[1])]
#[case::empty(TEST_UNSAFE_NAMES[2])]
#[case::current_dir(TEST_UNSAFE_NAMES[3])]
fn every_lookup_rejects_an_unsafe_name(sandbox: Sandbox, #[case] unsafe_name: (&str, &str)) {
    let (name, message) = unsafe_name;
    let registry = sandbox.states();

    assert!(registry.find(name).is_none());
    assert!(!registry.exists(name));

    assert_err!(registry.load(name), message);
    assert_err!(registry.delete(name), message);
    assert_err!(registry.path(name), message);
}

#[rstest]
fn find_returns_the_saved_state(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);

    assert_eq!(
        sandbox.states().find(TEST_STATE_NAME).unwrap().name,
        TEST_STATE_NAME
    );
}

#[rstest]
fn find_returns_none_for_an_unknown_state(sandbox: Sandbox) {
    assert!(sandbox.states().find(TEST_MISSING_STATE_NAME).is_none());
}

#[rstest]
fn find_returns_none_for_a_corrupt_state(sandbox: Sandbox) {
    corrupt(&sandbox, TEST_STATE_NAME);

    assert!(sandbox.states().find(TEST_STATE_NAME).is_none());
}

#[rstest]
fn exists_reports_only_saved_states(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);

    assert!(sandbox.states().exists(TEST_STATE_NAME));
    assert!(!sandbox.states().exists(TEST_MISSING_STATE_NAME));
}

#[rstest]
fn exists_is_false_for_state_shaped_directory(sandbox: Sandbox) {
    fs::create_dir_all(sandbox.states().path(TEST_STATE_NAME).unwrap()).unwrap();

    assert!(!sandbox.states().exists(TEST_STATE_NAME));
}

#[rstest]
fn count_counts_the_saved_states(sandbox: Sandbox) {
    assert_eq!(sandbox.states().count().unwrap(), 0);

    sandbox.save(TEST_STATE_NAME);
    sandbox.save(TEST_OTHER_STATE_NAME);

    assert_eq!(sandbox.states().count().unwrap(), 2);
}

#[rstest]
fn count_includes_states_that_all_rejects(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);
    corrupt(&sandbox, "broken");

    assert_eq!(sandbox.states().count().unwrap(), 2);

    assert_err_is!(sandbox.states().all(), Error::Serde(_));
}

#[rstest]
fn all_loads_every_saved_state(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);
    sandbox.save(TEST_OTHER_STATE_NAME);

    let mut loaded: Vec<String> = sandbox
        .states()
        .all()
        .unwrap()
        .into_iter()
        .map(|state| state.name.clone())
        .collect();

    loaded.sort();

    assert_eq!(loaded, [TEST_STATE_NAME, TEST_OTHER_STATE_NAME]);
}

#[rstest]
fn delete_removes_the_state_file(sandbox: Sandbox) {
    let path = sandbox.save(TEST_STATE_NAME).path().unwrap();

    sandbox.states().delete(TEST_STATE_NAME).unwrap();

    assert!(!path.exists());
    assert!(!sandbox.states().exists(TEST_STATE_NAME));
}

#[rstest]
fn delete_fails_for_an_unknown_state(sandbox: Sandbox) {
    assert_err_is!(
        sandbox.states().delete(TEST_MISSING_STATE_NAME),
        Error::NotFound(_)
    );
}

#[rstest]
fn delete_fails_for_a_state_shaped_directory(sandbox: Sandbox) {
    fs::create_dir_all(sandbox.states().path(TEST_STATE_NAME).unwrap()).unwrap();

    assert_err_is!(sandbox.states().delete(TEST_STATE_NAME), Error::Io(_));
}

#[rstest]
fn clear_removes_every_state(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);
    sandbox.save(TEST_OTHER_STATE_NAME);
    sandbox.states().clear().unwrap();

    assert_eq!(sandbox.states().count().unwrap(), 0);
}

#[rstest]
fn clear_succeeds_on_an_empty_registry(sandbox: Sandbox) {
    sandbox.states().clear().unwrap();

    assert!(names(&sandbox.states()).is_empty());
}

#[rstest]
fn clear_removes_a_corrupt_state(sandbox: Sandbox) {
    corrupt(&sandbox, TEST_STATE_NAME);
    sandbox.states().clear().unwrap();

    assert_eq!(sandbox.states().count().unwrap(), 0);
}

#[rstest]
fn clear_removes_states_at_every_depth(sandbox: Sandbox) {
    for name in TEST_EVERY_DEPTH {
        sandbox.save(&native(name));
    }

    sandbox.states().clear().unwrap();

    assert_eq!(sandbox.states().count().unwrap(), 0);
}

#[rstest]
fn registries_sharing_state_dir_see_each_other(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);
    sandbox.reader().states().delete(TEST_STATE_NAME).unwrap();

    assert!(!sandbox.states().exists(TEST_STATE_NAME));
}

#[rstest]
fn state_dir_that_is_a_file_fails_every_walk(sandbox: Sandbox) {
    sandbox.block_states();

    let registry = sandbox.states();

    assert_err_is!(registry.names(), Error::Io(_));
    assert_err_is!(registry.count(), Error::Io(_));
    assert_err_is!(registry.all(), Error::Io(_));
    assert_err_is!(registry.clear(), Error::Io(_));
    assert_err_is!(registry.search("job"), Error::Io(_));
}

#[rstest]
fn state_dir_that_is_file_still_answers_lookups(sandbox: Sandbox) {
    sandbox.block_states();

    let registry = sandbox.states();

    assert!(!registry.exists(TEST_STATE_NAME));
    assert!(registry.find(TEST_STATE_NAME).is_none());
    assert!(registry.path(TEST_STATE_NAME).is_ok());
}

#[cfg(unix)]
#[rstest]
fn unreadable_subdirectory_fails_every_walk(sandbox: Sandbox) {
    let mut blocked = Blocked::default();

    if !Blocked::enforced(&sandbox.path().join("probe")) {
        return;
    }

    sandbox.save(TEST_STATE_NAME);

    let nested = sandbox.state_dir().join("sub");

    fs::create_dir_all(&nested).unwrap();
    fs::write(
        nested.join(format!("{TEST_OTHER_STATE_NAME}{STATE_FILE_EXTENSION}")),
        "{}",
    )
    .unwrap();
    blocked.block(&nested);

    let registry = sandbox.states();

    assert_err_is!(registry.count(), Error::Io(_));
    assert_err_is!(registry.all(), Error::Io(_));
    assert_err_is!(registry.clear(), Error::Io(_));
}
