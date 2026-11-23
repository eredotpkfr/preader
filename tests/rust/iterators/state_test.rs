use std::fs;
#[cfg(unix)]
use std::{
    ffi::OsStr,
    os::unix::{ffi::OsStrExt, fs::symlink},
    path::PathBuf,
};

use preader::Error;
use rstest::rstest;
use rstest_reuse::apply;

#[cfg(unix)]
use crate::common::{constants::TEST_STATE_FILE, templates::file::unrepresentable_files};
use crate::common::{
    constants::{
        TEST_EVERY_DEPTH, TEST_LINE, TEST_MISSING_STATE_NAME, TEST_NESTED_STATE_NAME,
        TEST_OTHER_STATE_FILE, TEST_OTHER_STATE_NAME, TEST_STATE_NAME, TEST_SUB_STATE_NAME,
    },
    fixtures::sandbox,
    funcs::{items, names, state_file},
    macros::asserts::assert_err_is,
    sandbox::Sandbox,
    templates::file::{foreign_state_files, non_state_files, unaddressable_files},
};

#[cfg(unix)]
fn outside_state(sandbox: &Sandbox) -> PathBuf {
    let path = sandbox.line_file();
    let elsewhere = sandbox.reader_in("outside", sandbox.config());
    let mut state = elsewhere.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    elsewhere.states().path(&state.save().unwrap().name).unwrap()
}

#[rstest]
fn names_is_empty_without_a_state_dir(sandbox: Sandbox) {
    assert!(names(&sandbox.states()).is_empty());
}

#[rstest]
fn names_keeps_the_slashes_of_a_nested_name(sandbox: Sandbox) {
    sandbox.save(TEST_NESTED_STATE_NAME);

    assert_eq!(names(&sandbox.states()), [TEST_NESTED_STATE_NAME]);
}

#[rstest]
fn search_fails_when_the_pattern_is_invalid(sandbox: Sandbox) {
    assert_err_is!(sandbox.states().search("["), Error::Regex(_));
}

#[rstest]
fn names_does_not_create_the_state_dir(sandbox: Sandbox) {
    sandbox.states().names().unwrap();

    assert!(!sandbox.state_dir().exists());
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
fn search_returns_nothing_without_a_match(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);

    let matched: Vec<String> = items(sandbox.states().search(TEST_MISSING_STATE_NAME).unwrap());

    assert!(matched.is_empty());
}

#[rstest]
fn names_yields_each_state_once(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);
    sandbox.append(&sandbox.line_file(), TEST_LINE);

    assert_eq!(sandbox.states().names().unwrap().count(), 1);
}

#[rstest]
#[case::everything("", &[TEST_STATE_NAME, TEST_OTHER_STATE_NAME])]
#[case::anywhere("ob-", &[TEST_STATE_NAME, TEST_OTHER_STATE_NAME])]
#[case::wildcard("job.1", &[TEST_STATE_NAME])]
#[case::character_class("^job-[12]$", &[TEST_STATE_NAME, TEST_OTHER_STATE_NAME])]
#[case::anchored("^job-1$", &[TEST_STATE_NAME])]
fn search_filters_the_saved_states_by_regex(
    sandbox: Sandbox,
    #[case] pattern: &str,
    #[case] expected: &[&str],
) {
    sandbox.save(TEST_STATE_NAME);
    sandbox.save(TEST_OTHER_STATE_NAME);

    let mut matched = items(sandbox.states().search(pattern).unwrap());

    matched.sort();

    assert_eq!(matched, expected);
}

#[rstest]
fn names_restarts_for_each_walk(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);

    let first = names(&sandbox.states());
    let second = names(&sandbox.states());

    assert_eq!(first, [TEST_STATE_NAME]);
    assert_eq!(first, second);
}

#[rstest]
fn search_matches_states_at_every_depth(sandbox: Sandbox) {
    for name in TEST_EVERY_DEPTH {
        sandbox.save(name);
    }

    let mut matched = items(sandbox.states().search(TEST_STATE_NAME).unwrap());
    let mut expected = TEST_EVERY_DEPTH;

    matched.sort();
    expected.sort();

    assert_eq!(matched, expected);
}

#[rstest]
#[case::flat("job-1/.state.json")]
#[case::repeated("job-1/.state.json/.state.json")]
fn names_lists_a_suffix_shaped_component(sandbox: Sandbox, #[case] state: &str) {
    sandbox.save(state);

    let path = sandbox.states().path(state).unwrap();

    assert_eq!(path, sandbox.state_dir().join(state_file(state)));
    assert_eq!(names(&sandbox.states()), [state]);
}

#[rstest]
fn search_matches_anywhere_in_the_name(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);
    sandbox.save("myjob-2");

    let mut matched = items(sandbox.states().search("ob").unwrap());

    matched.sort();

    assert_eq!(matched, [TEST_STATE_NAME, "myjob-2"]);
}

#[rstest]
fn names_lists_states_at_every_depth(sandbox: Sandbox) {
    let mut expected = TEST_EVERY_DEPTH;

    for name in &expected {
        sandbox.save(name);
    }

    expected.sort();

    assert_eq!(names(&sandbox.states()), expected);
    assert_eq!(sandbox.states().count().unwrap(), expected.len());
}

#[rstest]
fn search_matches_everything_with_an_empty_pattern(sandbox: Sandbox) {
    sandbox.save("foo-1");
    sandbox.save("bar-1");

    let mut matched = items(sandbox.states().search("").unwrap());

    matched.sort();

    assert_eq!(matched, ["bar-1", "foo-1"]);
}

#[cfg(unix)]
#[rstest]
fn names_ignores_a_symlink_that_leaves_the_state_dir(sandbox: Sandbox) {
    let target = outside_state(&sandbox);

    sandbox.save(TEST_OTHER_STATE_NAME);

    symlink(&target, sandbox.state_dir().join(TEST_STATE_FILE)).unwrap();

    assert!(!sandbox.states().exists(TEST_STATE_NAME));

    assert_eq!(names(&sandbox.states()), [TEST_OTHER_STATE_NAME]);
}

#[apply(non_state_files)]
fn walk_ignores_a_non_state_file(sandbox: Sandbox, #[case] name: &str) {
    sandbox.save(TEST_STATE_NAME);

    fs::write(sandbox.state_dir().join(name), "not a state").unwrap();

    assert_eq!(names(&sandbox.states()), [TEST_STATE_NAME]);
    assert_eq!(sandbox.states().count().unwrap(), 1);
}

#[cfg(unix)]
#[rstest]
fn names_ignores_a_symlinked_state(sandbox: Sandbox) {
    let target = sandbox.save(TEST_STATE_NAME);

    symlink(&target, sandbox.state_dir().join("alias.state.json")).unwrap();

    assert!(!sandbox.states().exists("alias"));

    assert_eq!(names(&sandbox.states()), [TEST_STATE_NAME]);
}

#[cfg(unix)]
#[rstest]
fn names_ignores_a_non_utf8_state(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);

    let unrepresentable = OsStr::from_bytes(b"job-\xff.state.json");

    if fs::write(sandbox.state_dir().join(unrepresentable), "{}").is_err() {
        return;
    }

    assert_eq!(names(&sandbox.states()), [TEST_STATE_NAME]);
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
fn names_ignores_a_broken_symlink(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);

    symlink(
        sandbox.path().join("missing.state.json"),
        sandbox.state_dir().join(TEST_OTHER_STATE_FILE),
    )
    .unwrap();

    assert!(!sandbox.states().exists(TEST_OTHER_STATE_NAME));

    assert_eq!(names(&sandbox.states()), [TEST_STATE_NAME]);
}

#[rstest]
fn clear_keeps_a_state_shaped_directory(sandbox: Sandbox) {
    let shaped = sandbox.state_dir().join(TEST_OTHER_STATE_FILE);

    sandbox.save(TEST_STATE_NAME);

    fs::create_dir_all(&shaped).unwrap();

    sandbox.states().clear().unwrap();

    assert!(shaped.is_dir());

    assert_eq!(sandbox.states().count().unwrap(), 0);
}

#[rstest]
fn search_filters_by_pattern(sandbox: Sandbox) {
    sandbox.save("foo-1");
    sandbox.save("foo-2");
    sandbox.save("bar-1");

    let mut matched = items(sandbox.states().search("^foo").unwrap());

    matched.sort();

    assert_eq!(matched, ["foo-1", "foo-2"]);
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

#[rstest]
fn search_treats_the_pattern_as_a_regex(sandbox: Sandbox) {
    for name in ["foo-1", "foo.1", "foox1"] {
        sandbox.save(name);
    }

    let mut matched = items(sandbox.states().search("foo.1").unwrap());

    matched.sort();

    assert_eq!(matched, ["foo-1", "foo.1", "foox1"]);
}

#[rstest]
fn names_lists_a_state_under_a_state_shaped_parent(sandbox: Sandbox) {
    let saved = sandbox.save("archive.state.json/job-1");

    assert_eq!(names(&sandbox.states()), ["archive.state.json/job-1"]);
    assert_eq!(sandbox.states().count().unwrap(), 1);

    sandbox.states().clear().unwrap();

    assert!(!saved.exists());
}

#[rstest]
fn names_keeps_a_state_suffix_at_every_depth(sandbox: Sandbox) {
    let mut expected = TEST_EVERY_DEPTH.map(state_file);

    for name in &expected {
        sandbox.save(name);
    }

    let registry = sandbox.states();
    let mut loaded: Vec<String> =
        names(&registry).iter().map(|name| registry.load(name).unwrap().name).collect();

    loaded.sort();
    expected.sort();

    assert_eq!(loaded, expected);
}

#[cfg(unix)]
#[rstest]
fn clear_keeps_a_non_utf8_state(sandbox: Sandbox) {
    let saved = sandbox.save(TEST_STATE_NAME);
    let unrepresentable = sandbox.state_dir().join(OsStr::from_bytes(b"job-\xff.state.json"));

    if fs::write(&unrepresentable, "{}").is_err() {
        return;
    }

    sandbox.states().clear().unwrap();

    assert!(!saved.exists());
    assert!(unrepresentable.exists());
}

#[apply(unaddressable_files)]
fn clear_keeps_a_file_it_cannot_address(sandbox: Sandbox, #[case] file: &str) {
    let saved = sandbox.save(TEST_STATE_NAME);
    let stray = sandbox.state_dir().join(file);

    fs::create_dir_all(stray.parent().unwrap()).unwrap();
    fs::write(&stray, "{}").unwrap();

    sandbox.states().clear().unwrap();

    assert!(!saved.exists());
    assert!(stray.exists());
}

#[cfg(unix)]
#[apply(unrepresentable_files)]
fn clear_keeps_a_file_it_cannot_represent(sandbox: Sandbox, #[case] file: String) {
    let saved = sandbox.save(TEST_STATE_NAME);
    let stray = sandbox.state_dir().join(file);

    fs::create_dir_all(stray.parent().unwrap()).unwrap();
    fs::write(&stray, "{}").unwrap();

    sandbox.states().clear().unwrap();

    assert!(!saved.exists());
    assert!(stray.exists());
}

#[cfg(unix)]
#[rstest]
fn clear_keeps_a_symlinked_state(sandbox: Sandbox) {
    let target = outside_state(&sandbox);
    let saved = sandbox.save(TEST_STATE_NAME);
    let alias = sandbox.state_dir().join("alias.state.json");

    symlink(&target, &alias).unwrap();

    sandbox.states().clear().unwrap();

    assert!(!saved.exists());
    assert!(target.exists());
    assert!(alias.symlink_metadata().is_ok());
}

#[rstest]
fn names_survives_deleting_the_yielded_state(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);
    sandbox.save(TEST_OTHER_STATE_NAME);

    let mut iterator = sandbox.states().names().unwrap();
    let first = iterator.next().unwrap().unwrap();

    sandbox.states().delete(&first).unwrap();

    let mut seen: Vec<String> = [vec![first], items(iterator)].concat();

    seen.sort();

    assert_eq!(seen, [TEST_STATE_NAME, TEST_OTHER_STATE_NAME]);
}

#[rstest]
fn walk_ignores_a_state_shaped_directory(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);

    fs::create_dir_all(sandbox.state_dir().join(TEST_OTHER_STATE_FILE)).unwrap();

    let all: Vec<String> =
        sandbox.states().all().unwrap().into_iter().map(|state| state.name).collect();

    assert_eq!(names(&sandbox.states()), [TEST_STATE_NAME]);
    assert_eq!(sandbox.states().count().unwrap(), 1);
    assert_eq!(all, [TEST_STATE_NAME]);
}

#[rstest]
fn names_survives_a_midway_delete(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);
    sandbox.save(TEST_OTHER_STATE_NAME);

    let mut iterator = sandbox.states().names().unwrap();
    let first = iterator.next().unwrap().unwrap();
    let deleted = if first == TEST_STATE_NAME {
        TEST_OTHER_STATE_NAME
    } else {
        TEST_STATE_NAME
    };

    sandbox.states().delete(deleted).unwrap();

    let rest = items(iterator);

    assert!(rest.iter().all(|name| name == deleted));

    assert_eq!(names(&sandbox.states()), [first]);
}

#[cfg(unix)]
#[rstest]
fn clear_does_not_descend_into_a_symlinked_directory(sandbox: Sandbox) {
    let target = outside_state(&sandbox);
    let saved = sandbox.save(TEST_STATE_NAME);

    symlink(
        sandbox.path().join("outside").join("missing.bin"),
        sandbox.path().join("outside").join("link.bin"),
    )
    .unwrap();
    symlink(
        sandbox.path().join("outside"),
        sandbox.state_dir().join("link"),
    )
    .unwrap();

    sandbox.states().clear().unwrap();

    assert!(!saved.exists());
    assert!(target.exists());
}

#[cfg(unix)]
#[apply(unrepresentable_files)]
fn walk_skips_a_file_it_cannot_represent(sandbox: Sandbox, #[case] file: String) {
    sandbox.save(TEST_STATE_NAME);

    let stray = sandbox.state_dir().join(file);

    fs::create_dir_all(stray.parent().unwrap()).unwrap();
    fs::write(&stray, "{}").unwrap();

    let registry = sandbox.states();
    let loaded: Vec<String> = registry.all().unwrap().into_iter().map(|state| state.name).collect();

    assert_eq!(names(&registry), [TEST_STATE_NAME]);
    assert_eq!(registry.count().unwrap(), 1);
    assert_eq!(loaded, [TEST_STATE_NAME]);
}

#[cfg(unix)]
#[rstest]
fn names_does_not_descend_into_a_symlinked_directory(sandbox: Sandbox) {
    outside_state(&sandbox);

    symlink(
        sandbox.path().join("outside").join("missing.bin"),
        sandbox.path().join("outside").join("link.bin"),
    )
    .unwrap();
    fs::create_dir_all(sandbox.state_dir()).unwrap();

    symlink(
        sandbox.path().join("outside"),
        sandbox.state_dir().join("link"),
    )
    .unwrap();

    assert!(names(&sandbox.states()).is_empty());
    assert!(!sandbox.states().exists("link/job-1"));
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

#[apply(foreign_state_files)]
fn names_lists_a_foreign_state_file_under_its_own_name(
    sandbox: Sandbox,
    #[case] file: &str,
    #[case] name: &str,
) {
    let saved = sandbox.save(TEST_STATE_NAME);
    let sub = sandbox.save(TEST_SUB_STATE_NAME);
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
