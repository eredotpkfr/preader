use std::fs;

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
    tmp_dir: TempDir,
}

impl Sandbox {
    fn save(&self, name: &str) -> State {
        let path = write(&self.tmp_dir, "data.bin", b"abcdef");
        let mut state = self.reader.bytes(&path).state(name).build().unwrap().state().clone();

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

    Sandbox { reader, tmp_dir }
}

#[rstest]
fn state_dir_points_at_the_configured_directory(sandbox: Sandbox) {
    assert_eq!(
        sandbox.reader.states().state_dir(),
        sandbox.reader.config.state_dir
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
    let directory = sandbox.reader.config.state_dir.clone();

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
    assert!(sandbox.reader.states().find(TEST_STATE_NAME).unwrap().is_none());
}

#[rstest]
fn find_returns_the_saved_state(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);

    let found = sandbox.reader.states().find(TEST_STATE_NAME).unwrap().unwrap();

    assert_eq!(found.name, TEST_STATE_NAME);
}

#[rstest]
fn path_reports_the_state_file_location(sandbox: Sandbox) {
    let path = sandbox.reader.states().path(TEST_STATE_NAME).unwrap();

    assert_eq!(
        path,
        sandbox.reader.config.state_dir.join("job-1.state.json")
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
#[case("")]
#[case("../escape")]
#[case("/absolute")]
#[case(".")]
fn load_fails_with_missing_for_an_unsafe_name(sandbox: Sandbox, #[case] name: &str) {
    let error = sandbox.load_error(name);

    assert!(
        matches!(&error, Error::NotFound(reported) if reported == name),
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
#[case("")]
#[case("../escape")]
#[case("/absolute")]
#[case(".")]
fn delete_fails_with_missing_for_an_unsafe_name(sandbox: Sandbox, #[case] name: &str) {
    let error = sandbox.reader.states().delete(name).err().unwrap();

    assert!(
        matches!(&error, Error::NotFound(reported) if reported == name),
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
        assert!(registry.find(name).unwrap().is_none(), "find({name:?})");
        assert!(registry.delete(name).is_err(), "delete({name:?})");
    }
}
