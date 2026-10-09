use std::fs;

use preader::{Error, IteratorBuild, Mismatch, PathError};
use rstest::rstest;
use rstest_reuse::apply;

#[cfg(unix)]
use crate::common::guards::Blocked;
use crate::common::{
    constants::{
        TEST_EVERY_DEPTH, TEST_MISSING_STATE_NAME, TEST_OTHER_STATE_NAME, TEST_STATE_NAME,
    },
    fixtures::sandbox,
    funcs::{names, state_file},
    macros::asserts::assert_err_is,
    rule::Rule,
    sandbox::Sandbox,
    templates::name::{device_names, invalid_names, unportable_characters, windows_invalid_names},
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
        state_file(TEST_STATE_NAME).as_str()
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
fn load_fails_when_the_payload_names_another_state(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let state = sandbox.stored(&path, TEST_STATE_NAME);
    let payload = state.path().unwrap();
    let patched = fs::read_to_string(&payload).unwrap().replace(
        &format!("\"name\": \"{TEST_STATE_NAME}\""),
        "\"name\": \"a-different-name\"",
    );

    fs::write(&payload, patched).unwrap();

    assert_err_is!(
        sandbox.lenient().states().load(TEST_STATE_NAME),
        Error::Mismatch(Mismatch::Name { saved, current })
            if saved == "a-different-name" && current == TEST_STATE_NAME
    );
}

#[apply(invalid_names)]
fn every_lookup_rejects_an_invalid_name(sandbox: Sandbox, #[case] name: &str, #[case] rule: Rule) {
    let registry = sandbox.states();

    assert!(registry.find(name).is_none());
    assert!(!registry.exists(name));

    assert_err_is!(registry.load(name), Error::Path(error) if rule.matches(error, name));
    assert_err_is!(registry.delete(name), Error::Path(error) if rule.matches(error, name));
    assert_err_is!(registry.path(name), Error::Path(error) if rule.matches(error, name));
}

#[apply(windows_invalid_names)]
fn every_lookup_rejects_an_unportable_name(sandbox: Sandbox, #[case] name: &str) {
    let registry = sandbox.states();
    assert!(registry.find(name).is_none());
    assert!(!registry.exists(name));

    assert_err_is!(registry.load(name), Error::Path(PathError::Invalid(found)) if found == name);
    assert_err_is!(registry.delete(name), Error::Path(PathError::Invalid(found)) if found == name);
    assert_err_is!(registry.path(name), Error::Path(PathError::Invalid(found)) if found == name);
}

#[apply(unportable_characters)]
fn every_lookup_rejects_an_unportable_character(
    sandbox: Sandbox,
    #[case] character: char,
    #[values("job{}1", "sub{}/job-1")] shape: &str,
) {
    let registry = sandbox.states();
    let name = shape.replace("{}", &character.to_string());

    assert!(registry.find(&name).is_none());
    assert!(!registry.exists(&name));

    assert_err_is!(registry.load(&name), Error::Path(PathError::Invalid(found)) if *found == name);
    assert_err_is!(registry.delete(&name), Error::Path(PathError::Invalid(found)) if *found == name);
    assert_err_is!(registry.path(&name), Error::Path(PathError::Invalid(found)) if *found == name);
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
fn delete_fails_when_the_state_is_unknown(sandbox: Sandbox) {
    assert_err_is!(
        sandbox.states().delete(TEST_MISSING_STATE_NAME),
        Error::NotFound(_)
    );
}

#[rstest]
fn delete_fails_when_the_state_is_a_directory(sandbox: Sandbox) {
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
        sandbox.save(name);
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
fn walk_fails_when_the_state_dir_is_a_file(sandbox: Sandbox) {
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
fn walk_fails_when_a_subdirectory_is_unreadable(sandbox: Sandbox) {
    let mut blocked = Blocked::default();

    if !Blocked::enforced(&sandbox.path().join("probe")) {
        return;
    }

    sandbox.save(TEST_STATE_NAME);

    let nested = sandbox.state_dir().join("sub");

    fs::create_dir_all(&nested).unwrap();
    fs::write(nested.join(state_file(TEST_OTHER_STATE_NAME)), "{}").unwrap();
    blocked.block(&nested);

    let registry = sandbox.states();

    assert_err_is!(registry.count(), Error::Io(_));
    assert_err_is!(registry.all(), Error::Io(_));
    assert_err_is!(registry.clear(), Error::Io(_));
}

#[rstest]
#[case::uppercase("job-1", "JOB-1")]
#[case::decomposed("café", "cafe\u{301}")]
#[case::uppercase_directory("sub/job-1", "SUB/job-1")]
fn alias_never_touches_the_saved_state(sandbox: Sandbox, #[case] name: &str, #[case] alias: &str) {
    let saved = sandbox.save(name).path().unwrap();
    let before = fs::read(&saved).unwrap();
    let registry = sandbox.states();

    registry.delete(alias).ok();
    sandbox
        .reader()
        .bytes(sandbox.line_file())
        .state(alias)
        .build()
        .map(|mut bytes| bytes.state().save())
        .ok();

    assert!(names(&registry).contains(&name.to_owned()));

    assert_eq!(fs::read(&saved).unwrap(), before);
}

#[rstest]
fn path_uses_the_native_separator(sandbox: Sandbox) {
    let path = sandbox.states().path("sub-1/sub-2/job-1").unwrap();
    let expected = sandbox.state_dir().join("sub-1").join("sub-2").join(state_file("job-1"));

    assert_eq!(path.as_os_str(), expected.as_os_str());
}

#[rstest]
fn delete_removes_only_the_hard_link(sandbox: Sandbox) {
    let outside = sandbox.write("outside.txt", b"foo");
    let link = sandbox.states().path(TEST_STATE_NAME).unwrap();

    fs::create_dir_all(sandbox.state_dir()).unwrap();
    fs::hard_link(&outside, &link).unwrap();

    sandbox.states().delete(TEST_STATE_NAME).unwrap();

    assert!(!link.exists());

    assert_eq!(fs::read(&outside).unwrap(), b"foo");
}

#[cfg(unix)]
#[rstest]
fn symlinked_state_dir_is_used_as_the_root(sandbox: Sandbox) {
    use preader::{Config, PReader, StateRegistry};

    let real = sandbox.dir_at("real");
    let link = sandbox.path().join("linked");

    std::os::unix::fs::symlink(&real, &link).unwrap();

    let config = Config {
        state_dir: link,
        ..sandbox.config()
    };
    let registry = StateRegistry::from(&config);

    PReader::from(config)
        .bytes(sandbox.line_file())
        .state(TEST_STATE_NAME)
        .build()
        .unwrap()
        .state()
        .save()
        .unwrap();

    assert!(real.join(state_file(TEST_STATE_NAME)).is_file());

    assert_eq!(names(&registry), [TEST_STATE_NAME]);
    assert_eq!(
        registry.load(TEST_STATE_NAME).unwrap().name,
        TEST_STATE_NAME
    );

    registry.delete(TEST_STATE_NAME).unwrap();

    assert_eq!(fs::read_dir(&real).unwrap().count(), 0);
}

#[rstest]
fn delete_treats_a_suffixed_name_as_another_state(sandbox: Sandbox) {
    let saved = sandbox.save(TEST_STATE_NAME).path().unwrap();
    let outcome = sandbox.states().delete(&state_file(TEST_STATE_NAME));

    assert!(saved.exists());

    assert_err_is!(outcome, Error::NotFound(_));
}

#[cfg(unix)]
#[rstest]
fn every_lookup_rejects_a_directory_alias(sandbox: Sandbox) {
    let saved = sandbox.save("real/job-1").path().unwrap();
    let before = fs::read(&saved).unwrap();
    let registry = sandbox.states();

    std::os::unix::fs::symlink(saved.parent().unwrap(), sandbox.state_dir().join("alias")).unwrap();

    let path = sandbox.line_file();
    let built = sandbox.reader().bytes(&path).state("alias/job-1").build();

    assert!(registry.find("alias/job-1").is_none());
    assert!(!registry.exists("alias/job-1"));

    assert_eq!(names(&registry), ["real/job-1"]);
    assert_eq!(fs::read(&saved).unwrap(), before);

    assert_err_is!(
        registry.load("alias/job-1"),
        Error::Path(PathError::Alias(found)) if found == "alias/job-1"
    );
    assert_err_is!(
        registry.delete("alias/job-1"),
        Error::Path(PathError::Alias(found)) if found == "alias/job-1"
    );
    assert_err_is!(built, Error::Path(PathError::Alias(found)) if found == "alias/job-1");
}

#[apply(device_names)]
fn every_lookup_rejects_a_device_name(sandbox: Sandbox, #[case] name: &str) {
    let registry = sandbox.states();

    assert!(registry.find(name).is_none());
    assert!(!registry.exists(name));

    assert_err_is!(registry.load(name), Error::Path(PathError::Invalid(found)) if found == name);
    assert_err_is!(registry.delete(name), Error::Path(PathError::Invalid(found)) if found == name);
    assert_err_is!(registry.path(name), Error::Path(PathError::Invalid(found)) if found == name);
}
