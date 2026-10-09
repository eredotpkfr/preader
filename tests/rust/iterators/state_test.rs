use std::fs;

use preader::Error;
#[cfg(unix)]
use preader::{Config, IteratorBuild, PReader};
use rstest::rstest;
use rstest_reuse::apply;

#[cfg(unix)]
use crate::common::templates::file::unrepresentable_files;
use crate::common::{
    constants::{
        TEST_EVERY_DEPTH, TEST_LINE, TEST_NESTED_STATE_NAME, TEST_OTHER_STATE_NAME,
        TEST_STATE_NAME, TEST_SUB_STATE_NAME,
    },
    fixtures::sandbox,
    funcs::{names, state_file},
    macros::asserts::assert_err_is,
    sandbox::Sandbox,
    templates::{
        file::{foreign_state_files, non_state_files, unaddressable_files},
        name::valid_names,
    },
};

#[cfg(unix)]
fn outside_state(sandbox: &Sandbox) -> std::path::PathBuf {
    let path = sandbox.line_file();
    let elsewhere = PReader::from(Config {
        state_dir: sandbox.path().join("outside"),
        ..sandbox.config()
    });
    let mut state = elsewhere.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    state.state().save().unwrap()
}

#[rstest]
fn names_lists_every_saved_state(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);
    sandbox.save(TEST_OTHER_STATE_NAME);

    assert_eq!(
        names(&sandbox.states()),
        [TEST_STATE_NAME, TEST_OTHER_STATE_NAME]
    );
}

#[rstest]
fn names_is_empty_without_a_state_dir(sandbox: Sandbox) {
    assert!(names(&sandbox.states()).is_empty());
}

#[rstest]
fn names_creates_the_state_dir(sandbox: Sandbox) {
    assert!(!sandbox.state_dir().exists());

    sandbox.states().names().unwrap();

    assert!(sandbox.state_dir().is_dir());
}

#[rstest]
fn names_lists_states_at_every_depth(sandbox: Sandbox) {
    let mut expected = TEST_EVERY_DEPTH;

    for name in &expected {
        sandbox.save(name);
    }

    expected.sort();

    assert_eq!(names(&sandbox.states()), expected);
}

#[rstest]
fn names_survives_a_midway_delete(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);
    sandbox.save(TEST_OTHER_STATE_NAME);

    let mut iterator = sandbox.states().names().unwrap();
    let first = iterator.next().unwrap().unwrap();

    sandbox.states().delete(&first).unwrap();

    let rest: Vec<String> = iterator.map(Result::unwrap).collect();

    assert_eq!(rest.len(), 1);

    assert_ne!(rest[0], first);
}

#[apply(non_state_files)]
fn names_ignores_a_non_state_file(sandbox: Sandbox, #[case] name: &str) {
    sandbox.save(TEST_STATE_NAME);
    fs::write(sandbox.state_dir().join(name), "not a state").unwrap();

    assert_eq!(names(&sandbox.states()), [TEST_STATE_NAME]);
}

#[apply(non_state_files)]
fn clear_keeps_a_non_state_file(sandbox: Sandbox, #[case] name: &str) {
    sandbox.save(TEST_STATE_NAME);

    let unrelated = sandbox.state_dir().join(name);

    fs::write(&unrelated, "not a state").unwrap();
    sandbox.states().clear().unwrap();

    assert!(unrelated.exists());

    assert_eq!(sandbox.states().count().unwrap(), 0);
}

#[cfg(unix)]
#[rstest]
fn names_ignores_a_non_utf8_state(sandbox: Sandbox) {
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

    sandbox.save(TEST_STATE_NAME);

    let ghost = OsStr::from_bytes(b"ghost-\xff.state.json");

    if fs::write(sandbox.state_dir().join(ghost), "{}").is_err() {
        return;
    }

    assert_eq!(names(&sandbox.states()), [TEST_STATE_NAME]);
}

#[rstest]
fn names_ignores_a_state_shaped_directory(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);

    fs::create_dir_all(sandbox.state_dir().join(state_file(TEST_OTHER_STATE_NAME))).unwrap();

    assert_eq!(names(&sandbox.states()), [TEST_STATE_NAME]);
}

#[rstest]
fn clear_keeps_a_state_shaped_directory(sandbox: Sandbox) {
    let shaped = sandbox.state_dir().join(state_file(TEST_OTHER_STATE_NAME));

    sandbox.save(TEST_STATE_NAME);

    fs::create_dir_all(&shaped).unwrap();

    sandbox.states().clear().unwrap();

    assert!(shaped.is_dir());
}

#[rstest]
fn state_shaped_parent_does_not_hide_states(sandbox: Sandbox) {
    let saved = sandbox.save("archive.state.json/job-1").path().unwrap();

    assert_eq!(names(&sandbox.states()), ["archive.state.json/job-1"]);
    assert_eq!(sandbox.states().count().unwrap(), 1);

    sandbox.states().clear().unwrap();

    assert!(!saved.exists());
}

#[rstest]
fn nested_name_keeps_its_forward_slashes(sandbox: Sandbox) {
    let saved = sandbox.save("sub-1/sub-2/job-1");

    assert_eq!(saved.name, TEST_NESTED_STATE_NAME);
    assert_eq!(names(&sandbox.states()), [saved.name.as_str()]);
}

#[cfg(unix)]
#[rstest]
fn names_ignores_a_symlinked_state(sandbox: Sandbox) {
    use std::os::unix::fs::symlink;

    let target = sandbox.save(TEST_STATE_NAME).path().unwrap();

    symlink(&target, sandbox.state_dir().join("alias.state.json")).unwrap();

    assert!(!sandbox.states().exists("alias"));

    assert_eq!(names(&sandbox.states()), [TEST_STATE_NAME]);
}

#[cfg(unix)]
#[rstest]
fn names_ignores_a_broken_symlink(sandbox: Sandbox) {
    use std::os::unix::fs::symlink;

    sandbox.save(TEST_STATE_NAME);
    symlink(
        sandbox.path().join("missing.state.json"),
        sandbox.state_dir().join("broken.state.json"),
    )
    .unwrap();

    assert_eq!(names(&sandbox.states()), [TEST_STATE_NAME]);
}

#[cfg(unix)]
#[rstest]
fn names_does_not_descend_into_symlinked_directory(sandbox: Sandbox) {
    use std::os::unix::fs::symlink;

    outside_state(&sandbox);

    fs::create_dir_all(sandbox.state_dir()).unwrap();
    symlink(
        sandbox.path().join("outside"),
        sandbox.state_dir().join("link"),
    )
    .unwrap();

    assert!(names(&sandbox.states()).is_empty());
    assert!(!sandbox.states().exists("link/job-1"));
}

#[cfg(unix)]
#[rstest]
fn names_ignores_symlink_that_leaves_state_dir(sandbox: Sandbox) {
    use std::os::unix::fs::symlink;

    let target = outside_state(&sandbox);

    sandbox.save(TEST_OTHER_STATE_NAME);
    symlink(&target, sandbox.state_dir().join("evil.state.json")).unwrap();

    assert!(!sandbox.states().exists("evil"));

    assert_eq!(names(&sandbox.states()), [TEST_OTHER_STATE_NAME]);
}

#[rstest]
fn search_filters_by_the_pattern(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);
    sandbox.save(TEST_OTHER_STATE_NAME);

    let matched: Vec<String> =
        sandbox.states().search("job-2").unwrap().map(Result::unwrap).collect();

    assert_eq!(matched, [TEST_OTHER_STATE_NAME]);
}

#[rstest]
fn search_returns_nothing_without_a_match(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);

    let matched: Vec<String> =
        sandbox.states().search("nothing").unwrap().map(Result::unwrap).collect();

    assert!(matched.is_empty());
}

#[rstest]
#[case::everything("", 2)]
#[case::anywhere("ob-", 2)]
#[case::regex("^job-[12]$", 2)]
#[case::anchored("^job-1$", 1)]
fn search_treats_the_pattern_as_a_regex(
    sandbox: Sandbox,
    #[case] pattern: &str,
    #[case] expected: usize,
) {
    sandbox.save(TEST_STATE_NAME);
    sandbox.save(TEST_OTHER_STATE_NAME);

    let matched = sandbox.states().search(pattern).unwrap().count();

    assert_eq!(matched, expected);
}

#[rstest]
fn search_matches_states_at_every_depth(sandbox: Sandbox) {
    for name in TEST_EVERY_DEPTH {
        sandbox.save(name);
    }

    assert_eq!(sandbox.states().search("job-1").unwrap().count(), 3);
}

#[rstest]
fn search_fails_when_the_pattern_is_invalid(sandbox: Sandbox) {
    assert_err_is!(sandbox.states().search("["), Error::Regex(_));
}

#[rstest]
fn state_iterator_can_be_reused_for_each_walk(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);

    let first = names(&sandbox.states());
    let second = names(&sandbox.states());

    assert_eq!(first, second);
}

#[rstest]
fn item_is_yielded_once_per_state(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);
    sandbox.append(&sandbox.line_file(), TEST_LINE);

    assert_eq!(sandbox.states().names().unwrap().count(), 1);
}

#[apply(valid_names)]
fn saved_name_round_trips_through_the_listing(sandbox: Sandbox, #[case] name: &str) {
    let saved = sandbox.save(name).path().unwrap();
    let registry = sandbox.states();
    let file = sandbox.state_dir().join(state_file(name));
    let payload: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&saved).unwrap()).unwrap();
    let pattern = format!("^{}$", regex::escape(name));

    assert!(file.is_file());

    assert_eq!(saved, file);
    assert_eq!(payload["name"], name);
    assert_eq!(names(&registry), [name]);
    assert_eq!(
        registry.search(&pattern).unwrap().map(Result::unwrap).collect::<Vec<_>>(),
        [name]
    );
    assert_eq!(registry.load(name).unwrap().name, name);
    assert_eq!(registry.path(name).unwrap(), file);

    registry.delete(name).unwrap();

    assert!(!file.exists());

    sandbox.save(name);
    registry.clear().unwrap();

    assert!(!file.exists());
}

#[apply(foreign_state_files)]
fn foreign_state_file_is_listed_under_its_own_name(
    sandbox: Sandbox,
    #[case] file: &str,
    #[case] name: &str,
) {
    let saved = sandbox.save(TEST_STATE_NAME).path().unwrap();
    let sub = sandbox.save(TEST_SUB_STATE_NAME).path().unwrap();
    let foreign = sandbox.state_dir().join(file);

    fs::create_dir_all(foreign.parent().unwrap()).unwrap();
    fs::write(&foreign, "{}").unwrap();

    let registry = sandbox.states();
    let mut expected = [TEST_STATE_NAME, TEST_SUB_STATE_NAME, name];

    expected.sort_unstable();

    assert_eq!(names(&registry), expected);
    assert_eq!(registry.path(name).unwrap(), foreign);

    registry.delete(name).unwrap();

    assert!(!foreign.exists());
    assert!(saved.exists());
    assert!(sub.exists());
}

#[apply(unaddressable_files)]
fn walk_skips_a_file_it_cannot_address(sandbox: Sandbox, #[case] file: &str) {
    sandbox.save(TEST_STATE_NAME);
    sandbox.save(TEST_SUB_STATE_NAME);

    let stray = sandbox.state_dir().join(file);

    fs::create_dir_all(stray.parent().unwrap()).unwrap();
    fs::write(&stray, "{}").unwrap();

    let registry = sandbox.states();

    assert_eq!(names(&registry), [TEST_STATE_NAME, TEST_SUB_STATE_NAME]);
    assert_eq!(registry.count().unwrap(), 2);
    assert_eq!(registry.all().unwrap().len(), 2);
    assert_eq!(registry.search("").unwrap().count(), 2);
}

#[apply(unaddressable_files)]
fn clear_keeps_a_file_it_cannot_address(sandbox: Sandbox, #[case] file: &str) {
    let saved = sandbox.save(TEST_STATE_NAME).path().unwrap();
    let stray = sandbox.state_dir().join(file);

    fs::create_dir_all(stray.parent().unwrap()).unwrap();
    fs::write(&stray, "{}").unwrap();
    sandbox.states().clear().unwrap();

    assert!(!saved.exists());
    assert!(stray.exists());
}

#[cfg(unix)]
#[apply(unrepresentable_files)]
fn walk_skips_a_file_it_cannot_represent(sandbox: Sandbox, #[case] file: &str) {
    sandbox.save(TEST_STATE_NAME);

    let stray = sandbox.state_dir().join(file);

    fs::create_dir_all(stray.parent().unwrap()).unwrap();
    fs::write(&stray, "{}").unwrap();

    let registry = sandbox.states();

    assert_eq!(names(&registry), [TEST_STATE_NAME]);
    assert_eq!(registry.count().unwrap(), 1);
}

#[cfg(unix)]
#[apply(unrepresentable_files)]
fn clear_keeps_a_file_it_cannot_represent(sandbox: Sandbox, #[case] file: &str) {
    let saved = sandbox.save(TEST_STATE_NAME).path().unwrap();
    let stray = sandbox.state_dir().join(file);

    fs::create_dir_all(stray.parent().unwrap()).unwrap();
    fs::write(&stray, "{}").unwrap();

    sandbox.states().clear().unwrap();

    assert!(!saved.exists());
    assert!(stray.exists());
}

#[apply(foreign_state_files)]
fn clear_removes_a_foreign_state_file(sandbox: Sandbox, #[case] file: &str, #[case] _name: &str) {
    let foreign = sandbox.state_dir().join(file);

    fs::create_dir_all(foreign.parent().unwrap()).unwrap();
    fs::write(&foreign, "{}").unwrap();
    sandbox.states().clear().unwrap();

    assert!(!foreign.exists());
}

#[cfg(unix)]
#[rstest]
fn clear_keeps_a_non_utf8_state_file(sandbox: Sandbox) {
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

    let ghost = sandbox.state_dir().join(OsStr::from_bytes(b"ghost-\xff.state.json"));

    fs::create_dir_all(sandbox.state_dir()).unwrap();

    if fs::write(&ghost, "{}").is_err() {
        return;
    }

    sandbox.states().clear().unwrap();

    assert!(ghost.exists());
}

#[cfg(unix)]
#[rstest]
fn clear_keeps_a_symlinked_state(sandbox: Sandbox) {
    use std::os::unix::fs::symlink;

    let target = outside_state(&sandbox);
    let alias = sandbox.state_dir().join("alias.state.json");

    fs::create_dir_all(sandbox.state_dir()).unwrap();
    symlink(&target, &alias).unwrap();
    sandbox.states().clear().unwrap();

    assert!(target.exists());
    assert!(alias.symlink_metadata().is_ok());
}

#[cfg(unix)]
#[rstest]
fn clear_does_not_descend_into_a_symlinked_directory(sandbox: Sandbox) {
    use std::os::unix::fs::symlink;

    let target = outside_state(&sandbox);

    fs::create_dir_all(sandbox.state_dir()).unwrap();
    symlink(
        sandbox.path().join("outside"),
        sandbox.state_dir().join("link"),
    )
    .unwrap();
    sandbox.states().clear().unwrap();

    assert!(target.exists());
}
