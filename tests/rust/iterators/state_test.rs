use std::fs;

#[cfg(unix)]
use preader::{Config, PReader};
use preader::{Error, IteratorBuild, STATE_FILE_EXTENSION};
use rstest::rstest;
use rstest_reuse::apply;

use crate::common::{
    constants::{
        TEST_EVERY_DEPTH, TEST_LINE, TEST_NESTED_STATE_NAME, TEST_OTHER_STATE_NAME, TEST_STATE_NAME,
    },
    fixtures::sandbox,
    funcs::{names, native},
    macros::asserts::assert_err_is,
    sandbox::Sandbox,
    templates::non_state_files,
};

fn every_depth() -> [String; 3] {
    TEST_EVERY_DEPTH.map(native)
}

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
    let mut expected = every_depth();

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
    fs::create_dir_all(
        sandbox
            .state_dir()
            .join(format!("{TEST_OTHER_STATE_NAME}{STATE_FILE_EXTENSION}")),
    )
    .unwrap();

    assert_eq!(names(&sandbox.states()), [TEST_STATE_NAME]);
}

#[rstest]
fn clear_keeps_a_state_shaped_directory(sandbox: Sandbox) {
    let shaped = sandbox
        .state_dir()
        .join(format!("{TEST_OTHER_STATE_NAME}{STATE_FILE_EXTENSION}"));

    sandbox.save(TEST_STATE_NAME);
    fs::create_dir_all(&shaped).unwrap();
    sandbox.states().clear().unwrap();

    assert!(shaped.is_dir());
}

#[rstest]
fn state_shaped_parent_does_not_hide_states(sandbox: Sandbox) {
    let saved = sandbox.save(&native("archive.state.json/job-1")).path().unwrap();

    assert_eq!(
        names(&sandbox.states()),
        [native("archive.state.json/job-1")]
    );
    assert_eq!(sandbox.states().count().unwrap(), 1);

    sandbox.states().clear().unwrap();

    assert!(!saved.exists());
}

#[rstest]
fn suffix_shaped_directory_stays_in_the_name(sandbox: Sandbox) {
    let name = native("sub-1/.state.json/sub-2/job-1");
    let path = sandbox.line_file();
    let mut state = sandbox
        .reader()
        .bytes(&path)
        .state(format!("{name}{STATE_FILE_EXTENSION}"))
        .build()
        .unwrap();
    let written = state.state().save().unwrap();

    assert_eq!(
        written,
        sandbox.state_dir().join(format!("{name}{STATE_FILE_EXTENSION}"))
    );
    assert_eq!(names(&sandbox.states()), [name.as_str()]);
    assert_eq!(sandbox.states().load(&name).unwrap().name, name);
}

#[rstest]
fn forward_slashes_resolve_to_the_native_name(sandbox: Sandbox) {
    let saved = sandbox.save("sub-1/sub-2/job-1");

    assert_eq!(saved.name, native(TEST_NESTED_STATE_NAME));
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
    assert!(!sandbox.states().exists(&native("link/job-1")));
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
    for name in every_depth() {
        sandbox.save(&name);
    }

    assert_eq!(sandbox.states().search("job-1").unwrap().count(), 3);
}

#[rstest]
fn search_fails_on_an_invalid_pattern(sandbox: Sandbox) {
    assert_err_is!(sandbox.states().search("["), Error::Regex(_));
}

#[rstest]
fn bare_suffix_file_yields_an_empty_name(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);
    fs::write(sandbox.state_dir().join(STATE_FILE_EXTENSION), "{}").unwrap();

    assert_eq!(names(&sandbox.states()), ["", TEST_STATE_NAME]);

    assert_err_is!(sandbox.states().all(), Error::Path(_));
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
