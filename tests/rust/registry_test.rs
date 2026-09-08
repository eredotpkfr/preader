use std::{fs, path::PathBuf};

use preader::{Config, Error, IteratorBuild, PReader, State};
use rstest::{fixture, rstest};
use tempfile::TempDir;

use crate::common::{
    constants::TEST_STATE_NAME,
    fixtures::tmp_dir,
    funcs::{reader, write},
};

const OTHER_STATE_NAME: &str = "job-2";
const UNSAFE_NAMES: [&str; 4] = ["", "../escape", "/absolute", "."];

struct Sandbox {
    reader: PReader,
    file: PathBuf,
    tmp_dir: TempDir,
}

impl Sandbox {
    fn save(&self, name: &str) -> State {
        let build = self.reader.bytes(&self.file).state(name).build();
        let mut state = build.unwrap().state().clone();

        state.save().unwrap();

        state
    }

    fn load_error(&self, name: &str) -> Error {
        self.reader.states().load(name).err().unwrap()
    }
}

#[fixture]
fn sandbox(tmp_dir: TempDir) -> Sandbox {
    let reader = reader(&tmp_dir, Config::default());
    let file = write(&tmp_dir, "data.bin", b"abcdef");

    Sandbox {
        reader,
        file,
        tmp_dir,
    }
}

#[rstest]
fn state_dir_points_at_the_configured_directory(sandbox: Sandbox) {
    assert_eq!(
        sandbox.reader.states().state_dir(),
        sandbox.reader.config().state_dir
    );
}

#[rstest]
fn names_lists_every_saved_state(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);
    sandbox.save(OTHER_STATE_NAME);

    let mut names =
        sandbox.reader.states().names().unwrap().collect::<Result<Vec<_>, _>>().unwrap();

    names.sort();

    assert_eq!(names, [TEST_STATE_NAME, OTHER_STATE_NAME]);
}

#[rstest]
fn names_is_empty_when_the_directory_is_missing(sandbox: Sandbox) {
    assert_eq!(sandbox.reader.states().names().unwrap().count(), 0);
}

#[rstest]
fn names_creates_the_state_directory(sandbox: Sandbox) {
    let directory = sandbox.reader.config().state_dir.clone();

    assert!(!directory.exists());

    sandbox.reader.states().names().unwrap();

    assert!(directory.is_dir());
}

#[rstest]
fn search_filters_by_the_pattern(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);
    sandbox.save(OTHER_STATE_NAME);

    let found = sandbox
        .reader
        .states()
        .search("-1$")
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(found, [TEST_STATE_NAME]);
}

#[rstest]
fn search_fails_on_an_invalid_pattern(sandbox: Sandbox) {
    let error = sandbox.reader.states().search("[").err().unwrap();

    assert!(
        matches!(error, Error::Regex(_)),
        "unexpected error: {error}"
    );
}

#[rstest]
fn count_counts_the_saved_states(sandbox: Sandbox) {
    assert_eq!(sandbox.reader.states().count().unwrap(), 0);

    sandbox.save(TEST_STATE_NAME);

    assert_eq!(sandbox.reader.states().count().unwrap(), 1);

    sandbox.save(OTHER_STATE_NAME);

    assert_eq!(sandbox.reader.states().count().unwrap(), 2);
}

#[rstest]
fn all_loads_every_saved_state(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);
    sandbox.save(OTHER_STATE_NAME);

    let mut names = sandbox
        .reader
        .states()
        .all()
        .unwrap()
        .iter()
        .map(|state| state.name.clone())
        .collect::<Vec<_>>();

    names.sort();

    assert_eq!(names, [TEST_STATE_NAME, OTHER_STATE_NAME]);
}

#[rstest]
fn exists_reports_only_saved_states(sandbox: Sandbox) {
    assert!(!sandbox.reader.states().exists(TEST_STATE_NAME));

    sandbox.save(TEST_STATE_NAME);

    assert!(sandbox.reader.states().exists(TEST_STATE_NAME));
}

#[rstest]
#[case("")]
#[case("../escape")]
#[case("/absolute")]
#[case(".")]
fn exists_is_false_for_an_unsafe_name(sandbox: Sandbox, #[case] name: &str) {
    assert!(!sandbox.reader.states().exists(name));
}

#[rstest]
fn find_returns_none_for_an_unknown_state(sandbox: Sandbox) {
    assert!(sandbox.reader.states().find(TEST_STATE_NAME).is_none());
}

#[rstest]
fn find_returns_the_saved_state(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);

    let found = sandbox.reader.states().find(TEST_STATE_NAME).unwrap();

    assert_eq!(found.name, TEST_STATE_NAME);
}

#[rstest]
fn path_reports_the_state_file_location(sandbox: Sandbox) {
    let path = sandbox.reader.states().path(TEST_STATE_NAME).unwrap();

    assert_eq!(
        path,
        sandbox.reader.config().state_dir.join("job-1.state.json")
    );
}

#[rstest]
fn load_fails_with_missing_for_an_unknown_state(sandbox: Sandbox) {
    let error = sandbox.load_error(TEST_STATE_NAME);

    assert!(
        matches!(&error, Error::NotFound(name) if name == TEST_STATE_NAME),
        "unexpected error: {error}"
    );
}

#[rstest]
#[case::traversal("../escape", "path escapes root")]
#[case::absolute("/absolute", "path escapes root")]
#[case::empty("", "path must not be empty")]
#[case::current_dir(".", "path must not be empty")]
fn load_fails_when_the_name_is_unsafe(sandbox: Sandbox, #[case] name: &str, #[case] message: &str) {
    let error = sandbox.load_error(name);

    assert!(
        error.to_string().contains(message),
        "name={name:?} produced {error}"
    );
}

#[rstest]
fn load_fails_with_missing_when_the_state_is_a_directory(sandbox: Sandbox) {
    let path = sandbox.reader.states().path(TEST_STATE_NAME).unwrap();

    fs::create_dir_all(&path).unwrap();

    let error = sandbox.load_error(TEST_STATE_NAME);

    assert!(
        matches!(&error, Error::NotFound(_)),
        "unexpected error: {error}"
    );
    assert!(error.to_string().contains("state not found"));
}

#[rstest]
fn delete_removes_the_state_file(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);

    sandbox.reader.states().delete(TEST_STATE_NAME).unwrap();

    assert!(!sandbox.reader.states().exists(TEST_STATE_NAME));
}

#[rstest]
#[case::traversal("../escape", "path escapes root")]
#[case::absolute("/absolute", "path escapes root")]
#[case::empty("", "path must not be empty")]
#[case::current_dir(".", "path must not be empty")]
fn delete_fails_when_the_name_is_unsafe(
    sandbox: Sandbox,
    #[case] name: &str,
    #[case] message: &str,
) {
    let error = sandbox.reader.states().delete(name).unwrap_err();

    assert!(
        error.to_string().contains(message),
        "name={name:?} produced {error}"
    );
}

#[rstest]
fn delete_fails_for_an_unknown_state(sandbox: Sandbox) {
    let error = sandbox.reader.states().delete(TEST_STATE_NAME).err().unwrap();

    assert!(
        matches!(&error, Error::NotFound(_)),
        "unexpected error: {error}"
    );
}

#[rstest]
fn clear_removes_every_state(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);
    sandbox.save(OTHER_STATE_NAME);

    sandbox.reader.states().clear().unwrap();

    assert_eq!(sandbox.reader.states().count().unwrap(), 0);
}

#[rstest]
fn clear_succeeds_on_an_empty_registry(sandbox: Sandbox) {
    sandbox.reader.states().clear().unwrap();

    assert_eq!(sandbox.reader.states().count().unwrap(), 0);
}

#[rstest]
fn an_unsafe_name_is_rejected_by_every_lookup(sandbox: Sandbox) {
    let registry = sandbox.reader.states();

    for name in UNSAFE_NAMES {
        assert!(!registry.exists(name), "exists({name:?})");
        assert!(registry.path(name).is_err(), "path({name:?})");
        assert!(registry.load(name).is_err(), "load({name:?})");
        assert!(registry.find(name).is_none(), "find({name:?})");
        assert!(registry.delete(name).is_err(), "delete({name:?})");
    }
}

#[rstest]
fn find_returns_none_for_a_corrupt_state(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);

    let path = sandbox.reader.states().path(TEST_STATE_NAME).unwrap();

    fs::write(&path, "not valid json").unwrap();

    assert!(sandbox.reader.states().exists(TEST_STATE_NAME));
    assert!(sandbox.reader.states().find(TEST_STATE_NAME).is_none());
    assert!(matches!(
        sandbox.load_error(TEST_STATE_NAME),
        Error::Serde(_)
    ));
}

fn nested(parts: &[&str]) -> String {
    parts.iter().collect::<std::path::PathBuf>().to_string_lossy().into_owned()
}

fn every_depth() -> [String; 3] {
    [
        TEST_STATE_NAME.to_owned(),
        nested(&["sub-1", "sub-2", "job-1"]),
        nested(&["sub-1", "sub-2", "sub-3", "sub-4", "job-1"]),
    ]
}

fn names_of(sandbox: &Sandbox) -> Vec<String> {
    let mut names =
        sandbox.reader.states().names().unwrap().collect::<Result<Vec<_>, _>>().unwrap();

    names.sort();
    names
}

#[rstest]
fn names_survives_a_midway_delete(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);
    sandbox.save(OTHER_STATE_NAME);

    let mut iterator = sandbox.reader.states().names().unwrap();
    let first = iterator.next().unwrap().unwrap();
    let deleted = if first == TEST_STATE_NAME {
        OTHER_STATE_NAME
    } else {
        TEST_STATE_NAME
    };

    sandbox.reader.states().delete(deleted).unwrap();

    let rest: Vec<String> = iterator.map(Result::unwrap).collect();

    assert!(rest.iter().all(|name| name == deleted), "{rest:?}");
    assert_eq!(names_of(&sandbox), [first]);
}

#[rstest]
#[case::plain("notes.txt")]
#[case::suffix_shaped("job-1.state.json.bak")]
#[case::partial("job-1.state")]
#[case::a_temporary_file("job-1.state.json.tmp")]
fn names_ignores_a_non_state_file(sandbox: Sandbox, #[case] file: &str) {
    sandbox.save(TEST_STATE_NAME);
    fs::write(
        sandbox.reader.states().state_dir().join(file),
        "not a state",
    )
    .unwrap();

    assert_eq!(names_of(&sandbox), [TEST_STATE_NAME]);
    assert_eq!(sandbox.reader.states().count().unwrap(), 1);
}

#[cfg(unix)]
#[rstest]
fn names_ignores_a_non_utf8_state(sandbox: Sandbox) {
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

    sandbox.save(TEST_STATE_NAME);

    let ghost = OsStr::from_bytes(b"ghost-\xff.state.json");

    if fs::write(sandbox.reader.states().state_dir().join(ghost), "{}").is_err() {
        return;
    }

    assert_eq!(names_of(&sandbox), [TEST_STATE_NAME]);
}

#[rstest]
#[case::unrelated("README.md")]
#[case::a_temporary_file("job-1.state.json.tmp")]
fn clear_keeps_a_non_state_file(sandbox: Sandbox, #[case] name: &str) {
    sandbox.save(TEST_STATE_NAME);

    let unrelated = sandbox.reader.states().state_dir().join(name);

    fs::write(&unrelated, "not a state").unwrap();
    sandbox.reader.states().clear().unwrap();

    assert_eq!(sandbox.reader.states().count().unwrap(), 0);
    assert!(unrelated.exists());
}

#[rstest]
fn forward_slashes_resolve_to_the_native_name(sandbox: Sandbox) {
    let saved = sandbox.save("sub-1/sub-2/job-1");

    assert_eq!(saved.name, nested(&["sub-1", "sub-2", "job-1"]));
    assert_eq!(names_of(&sandbox), [saved.name.as_str()]);
}

#[rstest]
fn a_suffix_shaped_directory_stays_in_the_name(sandbox: Sandbox) {
    let name = nested(&["sub-1", ".state.json", "sub-2", "job-1"]);
    let path = write(&sandbox.tmp_dir, "data.bin", b"abcdef");
    let mut state = sandbox
        .reader
        .bytes(&path)
        .state(format!("{name}.state.json"))
        .build()
        .unwrap()
        .state()
        .clone();
    let written = state.save().unwrap();

    assert_eq!(
        written,
        sandbox.reader.states().state_dir().join(format!("{name}.state.json"))
    );
    assert_eq!(names_of(&sandbox), [name.as_str()]);
    assert_eq!(sandbox.reader.states().load(&name).unwrap().name, name);
}

#[cfg(unix)]
#[rstest]
fn names_ignores_a_symlinked_state(sandbox: Sandbox) {
    use std::os::unix::fs::symlink;

    let real = sandbox.save(TEST_STATE_NAME).path().unwrap();
    let alias = sandbox
        .reader
        .states()
        .state_dir()
        .join(format!("{OTHER_STATE_NAME}.state.json"));

    symlink(&real, &alias).unwrap();

    assert_eq!(names_of(&sandbox), [TEST_STATE_NAME]);
    assert!(sandbox.reader.states().load(OTHER_STATE_NAME).is_err());
}

#[cfg(unix)]
#[rstest]
fn names_ignores_a_broken_symlink(sandbox: Sandbox) {
    use std::os::unix::fs::symlink;

    sandbox.save(TEST_STATE_NAME);

    let state_dir = sandbox.reader.states().state_dir().to_path_buf();

    symlink(
        state_dir.join("gone.state.json"),
        state_dir.join("broken.state.json"),
    )
    .unwrap();

    assert_eq!(names_of(&sandbox), [TEST_STATE_NAME]);
    assert!(!sandbox.reader.states().exists("broken"));
}

#[cfg(unix)]
#[rstest]
fn names_does_not_descend_into_a_symlinked_directory(sandbox: Sandbox) {
    use std::os::unix::fs::symlink;

    let outside = sandbox.tmp_dir.path().join("outside");
    let elsewhere = PReader::from(Config {
        state_dir: outside.clone(),
        ..Config::default()
    });
    let path = write(&sandbox.tmp_dir, "data.bin", b"abcdef");

    elsewhere
        .bytes(&path)
        .state(TEST_STATE_NAME)
        .build()
        .unwrap()
        .state()
        .save()
        .unwrap();

    let state_dir = sandbox.reader.states().state_dir().to_path_buf();

    fs::create_dir_all(&state_dir).unwrap();
    symlink(&outside, state_dir.join("link")).unwrap();

    assert_eq!(names_of(&sandbox), Vec::<String>::new());
    assert!(!sandbox.reader.states().exists(&nested(&["link", TEST_STATE_NAME])));
}

#[cfg(unix)]
#[rstest]
fn names_ignores_a_symlink_that_leaves_the_state_dir(sandbox: Sandbox) {
    use std::os::unix::fs::symlink;

    let outside = sandbox.tmp_dir.path().join("outside");
    let elsewhere = PReader::from(Config {
        state_dir: outside.clone(),
        ..Config::default()
    });
    let path = write(&sandbox.tmp_dir, "data.bin", b"abcdef");
    let target = elsewhere
        .bytes(&path)
        .state(TEST_STATE_NAME)
        .build()
        .unwrap()
        .state()
        .save()
        .unwrap();

    sandbox.save(OTHER_STATE_NAME);
    symlink(
        &target,
        sandbox.reader.states().state_dir().join("evil.state.json"),
    )
    .unwrap();

    assert_eq!(names_of(&sandbox), [OTHER_STATE_NAME]);
    assert!(!sandbox.reader.states().exists("evil"));
}

#[rstest]
fn names_lists_states_at_every_depth(sandbox: Sandbox) {
    let mut expected = every_depth();

    for name in &expected {
        sandbox.save(name);
    }

    expected.sort();

    assert_eq!(names_of(&sandbox), expected);
    assert_eq!(sandbox.reader.states().count().unwrap(), expected.len());
}

#[rstest]
fn search_matches_states_at_every_depth(sandbox: Sandbox) {
    let mut expected = every_depth();

    for name in &expected {
        sandbox.save(name);
    }

    expected.sort();

    let mut found = sandbox
        .reader
        .states()
        .search(TEST_STATE_NAME)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    found.sort();

    assert_eq!(found, expected);
}

#[rstest]
fn clear_removes_states_at_every_depth(sandbox: Sandbox) {
    for name in &every_depth() {
        sandbox.save(name);
    }

    sandbox.reader.states().clear().unwrap();

    assert_eq!(sandbox.reader.states().count().unwrap(), 0);
}

#[rstest]
fn clear_removes_a_corrupt_state(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);
    fs::write(
        sandbox.reader.states().path(OTHER_STATE_NAME).unwrap(),
        "not valid json",
    )
    .unwrap();

    sandbox.reader.states().clear().unwrap();

    assert_eq!(sandbox.reader.states().count().unwrap(), 0);
}

#[rstest]
fn names_ignores_a_state_shaped_directory(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);
    fs::create_dir(sandbox.reader.states().state_dir().join("archive.state.json")).unwrap();

    assert_eq!(names_of(&sandbox), [TEST_STATE_NAME]);
    assert_eq!(sandbox.reader.states().count().unwrap(), 1);

    let loaded: Vec<String> = sandbox
        .reader
        .states()
        .all()
        .unwrap()
        .into_iter()
        .map(|state| state.name.clone())
        .collect();

    assert_eq!(loaded, [TEST_STATE_NAME]);
}

#[rstest]
fn clear_keeps_a_state_shaped_directory(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);

    let directory = sandbox.reader.states().state_dir().join("archive.state.json");

    fs::create_dir(&directory).unwrap();
    sandbox.reader.states().clear().unwrap();

    assert_eq!(sandbox.reader.states().count().unwrap(), 0);
    assert!(directory.is_dir());
}

#[rstest]
fn a_state_shaped_parent_does_not_hide_its_states(sandbox: Sandbox) {
    let saved = sandbox.save("archive.state.json/job-1").path().unwrap();

    assert_eq!(
        names_of(&sandbox),
        [nested(&["archive.state.json", "job-1"])]
    );
    assert_eq!(sandbox.reader.states().count().unwrap(), 1);

    sandbox.reader.states().clear().unwrap();

    assert!(!saved.exists());
}

#[rstest]
fn registries_sharing_a_state_dir_see_each_other(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);

    let other = reader(&sandbox.tmp_dir, Config::default());

    other.states().delete(TEST_STATE_NAME).unwrap();

    assert!(!sandbox.reader.states().exists(TEST_STATE_NAME));
}

#[rstest]
fn load_returns_the_payload_name(tmp_dir: TempDir) {
    let reader = reader(
        &tmp_dir,
        Config {
            verify_state: false,
            ..Config::default()
        },
    );
    let path = write(&tmp_dir, "data.bin", b"abcdef");
    let mut state = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap().state().clone();
    let payload = state.save().unwrap();
    let patched = fs::read_to_string(&payload).unwrap().replace(
        &format!("\"name\": \"{TEST_STATE_NAME}\""),
        "\"name\": \"a-different-name\"",
    );

    fs::write(&payload, patched).unwrap();

    assert_eq!(
        reader.states().load(TEST_STATE_NAME).unwrap().name,
        "a-different-name"
    );
}

#[rstest]
fn count_counts_states_that_all_rejects(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);
    fs::write(
        sandbox.reader.states().state_dir().join("broken.state.json"),
        "not valid json",
    )
    .unwrap();

    assert_eq!(sandbox.reader.states().count().unwrap(), 2);

    let error = sandbox.reader.states().all().err().unwrap();

    assert!(matches!(error, Error::Serde(_)), "got {error}");
}

#[rstest]
#[case::everything("", ["bar-1", "foo-1"])]
#[case::anywhere("o", ["foo-1", "foo-2"])]
#[case::regex("foo.1", ["foo-1", "foo.1"])]
fn search_treats_the_pattern_as_a_regex(
    sandbox: Sandbox,
    #[case] pattern: &str,
    #[case] expected: [&str; 2],
) {
    for name in ["foo-1", "foo-2", "bar-1", "foo.1"] {
        sandbox.save(name);
    }

    let mut found = sandbox
        .reader
        .states()
        .search(pattern)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    found.sort();
    found.retain(|name| expected.contains(&name.as_str()));

    assert_eq!(found, expected);
}

#[rstest]
fn all_fails_when_any_state_is_corrupt(sandbox: Sandbox) {
    for name in ["foo-1", "foo-2", "foo-3"] {
        sandbox.save(name);
    }

    fs::write(
        sandbox.reader.states().state_dir().join("foo-2.state.json"),
        "not valid json",
    )
    .unwrap();

    assert!(matches!(
        sandbox.reader.states().all(),
        Err(Error::Serde(_))
    ));
}

#[rstest]
fn names_fails_when_the_state_dir_is_a_file(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());

    fs::write(reader.config().state_dir.as_path(), "not a directory").unwrap();

    assert!(matches!(reader.states().names().err(), Some(Error::Io(_))));
    assert!(matches!(reader.states().count().err(), Some(Error::Io(_))));
    assert!(matches!(reader.states().all().err(), Some(Error::Io(_))));
    assert!(matches!(reader.states().clear().err(), Some(Error::Io(_))));
}

#[rstest]
fn a_blocked_state_dir_still_answers_the_lookups(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());

    fs::write(reader.config().state_dir.as_path(), "not a directory").unwrap();

    assert!(!reader.states().exists(TEST_STATE_NAME));
    assert!(reader.states().find(TEST_STATE_NAME).is_none());
    assert!(reader.states().path(TEST_STATE_NAME).is_ok());
}

#[rstest]
fn delete_fails_when_the_state_is_a_directory(sandbox: Sandbox) {
    let state_dir = sandbox.reader.states().state_dir().to_path_buf();

    fs::create_dir_all(state_dir.join(format!("{TEST_STATE_NAME}.state.json"))).unwrap();

    let error = sandbox.reader.states().delete(TEST_STATE_NAME).err().unwrap();

    assert!(matches!(error, Error::Io(_)), "got {error}");
}
