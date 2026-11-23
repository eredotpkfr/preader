#[cfg(unix)]
use std::os::unix::fs::symlink;
#[cfg(unix)]
use std::{ffi::OsStr, os::unix::ffi::OsStrExt};
use std::{
    fs,
    io::ErrorKind,
    path::{Path as FilePath, PathBuf},
};

use preader::{
    Config, Error, Mismatch, NameError, PReader, State, StateRegistry, TMP_FILE_EXTENSION, path,
};
use rstest::rstest;
use rstest_reuse::apply;
use serde_json::{Value, json};
use time::UtcDateTime;

use self::{
    Lookup::{Delete, Load, Path},
    Probe::{Exists, Find},
    Walk::{All, Clear, Count, Names, Search},
};
#[cfg(unix)]
use crate::common::constants::TEST_OTHER_STATE_FILE;
#[cfg(unix)]
use crate::common::guards::Blocked;
use crate::common::{
    constants::{
        TEST_DEEP_STATE_NAME, TEST_EVERY_DEPTH, TEST_FILE_NAME, TEST_FILE_PATH, TEST_LINE,
        TEST_LINE_FINGERPRINT, TEST_MISSING_STATE_NAME, TEST_NESTED_STATE_NAME,
        TEST_OTHER_STATE_NAME, TEST_STAMP, TEST_STATE_FILE, TEST_STATE_FILE_EXTENSION,
        TEST_STATE_NAME, TEST_TRACKED_COPIES, TEST_TRACKED_NAME,
    },
    fixtures::sandbox,
    funcs::{canonical, digest, items, names, state_data, state_file},
    macros::asserts::assert_err_is,
    sandbox::Sandbox,
    templates::{
        name::{alias_names, invalid_names, valid_names},
        payload::malformed_payloads,
    },
};

const PATH_DIGEST: &str = "07cb9e47c6d8681a47020d0bb04776e06ada8b6d7aabcf4838a127f98e9f4fe2";

#[derive(Clone, Copy, Debug)]
enum Lookup {
    Load,
    Delete,
    Path,
}

impl Lookup {
    fn call(self, registry: &StateRegistry, name: &str) -> preader::Result<()> {
        match self {
            Self::Load => registry.load(name).map(drop),
            Self::Delete => registry.delete(name),
            Self::Path => registry.path(name).map(drop),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Probe {
    Find,
    Exists,
}

impl Probe {
    fn call(self, registry: &StateRegistry, name: &str) -> bool {
        match self {
            Self::Find => registry.find(name).is_some(),
            Self::Exists => registry.exists(name),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Walk {
    Names,
    Count,
    All,
    Search,
    Clear,
}

impl Walk {
    fn call(self, registry: &StateRegistry) -> preader::Result<()> {
        match self {
            Self::Names => registry.names()?.try_for_each(|name| name.map(drop)),
            Self::Count => registry.count().map(drop),
            Self::All => registry.all().map(drop),
            Self::Search => registry.search(TEST_STATE_NAME)?.try_for_each(|name| name.map(drop)),
            Self::Clear => registry.clear(),
        }
    }
}

fn tmp_suffix() -> String {
    format!("{TEST_STATE_FILE_EXTENSION}.{TMP_FILE_EXTENSION}")
}

fn leftovers(sandbox: &Sandbox) -> Vec<PathBuf> {
    fs::read_dir(sandbox.state_dir())
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == TMP_FILE_EXTENSION))
        .collect()
}

fn unverifiable(name: &str) -> String {
    json!({
        "name": name,
        "file": {
            "path": TEST_FILE_PATH,
            "size": 4,
            "mtime": TEST_STAMP * 1_000_000,
            "fingerprint": TEST_LINE_FINGERPRINT,
        },
        "position": 7,
        "timestamps": {
            "created_at": TEST_STAMP + 1,
            "updated_at": TEST_STAMP + 2,
        },
        "_checksum": "not-a-real-checksum",
    })
    .to_string()
}

fn read_state(path: &FilePath, state: &State) {
    let mut payload: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    let checksum = payload.as_object_mut().unwrap().shift_remove("_checksum").unwrap();
    let (metadata, stamps) = (&state.file, &state.timestamps);

    assert_eq!(
        payload,
        json!({
            "name": state.name,
            "file": {
                "path": metadata.path,
                "size": metadata.size,
                "mtime": i64::try_from(metadata.mtime.unix_timestamp_nanos() / 1_000).unwrap(),
                "fingerprint": metadata.fingerprint,
            },
            "position": state.position,
            "timestamps": {
                "created_at": stamps.created_at.unix_timestamp(),
                "updated_at": stamps.updated_at.unix_timestamp(),
            },
        })
    );
    assert_eq!(checksum, digest(payload.to_string().as_bytes()));
}

#[rstest]
fn state_registry_fields(sandbox: Sandbox) {
    assert_eq!(sandbox.states().state_dir(), sandbox.config().state_dir);
}

#[rstest]
fn find_returns_none_for_an_unknown_state(sandbox: Sandbox) {
    assert!(sandbox.states().find(TEST_MISSING_STATE_NAME).is_none());
}

#[rstest]
fn autoname_matches_a_known_digest(sandbox: Sandbox) {
    assert_eq!(
        sandbox.states().autoname(TEST_FILE_PATH.as_ref()),
        PATH_DIGEST
    );
}

#[rstest]
fn lookup_answers_when_the_state_dir_is_a_file(
    sandbox: Sandbox,
    #[values(Find, Exists)] probe: Probe,
) {
    sandbox.block_states();

    assert!(!probe.call(&sandbox.states(), TEST_STATE_NAME));
}

#[rstest]
fn lookup_answers_when_the_state_is_missing(
    sandbox: Sandbox,
    #[values(Find, Exists)] probe: Probe,
) {
    sandbox.save(TEST_STATE_NAME);

    assert!(!probe.call(&sandbox.states(), TEST_MISSING_STATE_NAME));
}

#[apply(valid_names)]
fn lookup_finds_a_state_saved_under_a_valid_name(
    sandbox: Sandbox,
    #[case] name: String,
    #[values(Find, Exists)] probe: Probe,
) {
    sandbox.save(&name);

    assert!(probe.call(&sandbox.states(), &name));
}

#[rstest]
fn load_fails_when_no_state_is_saved(sandbox: Sandbox) {
    assert_err_is!(
        sandbox.states().load(TEST_STATE_NAME),
        Error::NotFound(found) if found == TEST_STATE_NAME
    );
}

#[rstest]
#[case::plain("job-1", "job-1.state.json")]
#[case::dotted("job.1", "job.1.state.json")]
#[case::only_looks_suffixed("mystate.json", "mystate.json.state.json")]
#[case::suffixed("job-1.state.json", "job-1.state.json.state.json")]
fn path_appends_the_extension_to_the_name(
    sandbox: Sandbox,
    #[case] name: &str,
    #[case] expected: &str,
) {
    assert_eq!(
        sandbox.states().path(name).unwrap().file_name().unwrap(),
        expected
    );
}

#[apply(valid_names)]
fn path_stays_inside_the_state_dir(sandbox: Sandbox, #[case] name: String) {
    assert!(sandbox.states().path(&name).unwrap().starts_with(sandbox.state_dir()));
}

#[apply(invalid_names)]
fn tmp_fails_when_the_name_is_invalid(
    sandbox: Sandbox,
    #[case] name: String,
    #[case] expected: NameError,
) {
    assert_err_is!(sandbox.states().tmp_path(&name), Error::Name(error) if *error == expected);
}

#[rstest]
fn path_joins_the_state_dir(sandbox: Sandbox) {
    assert_eq!(
        sandbox.states().path(TEST_STATE_NAME).unwrap(),
        sandbox.state_dir().join(TEST_STATE_FILE)
    );
}

#[rstest]
fn lookup_fails_when_the_state_is_missing(
    sandbox: Sandbox,
    #[values(Load, Delete)] lookup: Lookup,
) {
    assert_err_is!(
        lookup.call(&sandbox.states(), TEST_MISSING_STATE_NAME),
        Error::NotFound(name) if name == TEST_MISSING_STATE_NAME
    );
}

#[rstest]
fn clear_succeeds_on_an_empty_registry(sandbox: Sandbox) {
    sandbox.states().clear().unwrap();

    assert!(names(&sandbox.states()).is_empty());
}

#[rstest]
fn find_returns_none_for_a_corrupt_state(sandbox: Sandbox) {
    sandbox.write_state(TEST_STATE_NAME, "not valid json");

    assert!(sandbox.states().find(TEST_STATE_NAME).is_none());
}

#[rstest]
#[case::current_dir_component(TEST_FILE_PATH, "/tmp/./data.bin")]
#[case::trailing_slash("/tmp/data", "/tmp/data/")]
#[case::relative_and_absolute(TEST_FILE_NAME, TEST_FILE_PATH)]
fn autoname_does_not_normalize(sandbox: Sandbox, #[case] left: &str, #[case] right: &str) {
    assert_ne!(
        sandbox.states().autoname(left.as_ref()),
        sandbox.states().autoname(right.as_ref())
    );
}

#[rstest]
fn tmp_differs_between_calls(sandbox: Sandbox) {
    assert_ne!(
        sandbox.states().tmp_path(TEST_STATE_NAME).unwrap(),
        sandbox.states().tmp_path(TEST_STATE_NAME).unwrap()
    );
}

#[rstest]
fn tmp_never_collides_with_the_state_file(sandbox: Sandbox) {
    assert_ne!(
        sandbox.states().tmp_path(TEST_STATE_NAME).unwrap(),
        sandbox.states().path(TEST_STATE_NAME).unwrap()
    );
}

#[rstest]
fn autoname_is_stable_for_one_path(sandbox: Sandbox) {
    let path = TEST_FILE_PATH.as_ref();

    assert_eq!(
        sandbox.states().autoname(path),
        sandbox.states().autoname(path)
    );
}

#[rstest]
#[case::names(|registry: &StateRegistry| items(registry.names().unwrap()).len())]
#[case::count(|registry: &StateRegistry| registry.count().unwrap())]
#[case::all(|registry: &StateRegistry| registry.all().unwrap().len())]
#[case::search(|registry: &StateRegistry| items(registry.search(TEST_STATE_NAME).unwrap()).len())]
#[case::clear(|registry: &StateRegistry| registry.clear().map(|()| 0).unwrap())]
fn walk_yields_nothing_when_the_state_dir_is_missing(
    sandbox: Sandbox,
    #[case] walk: fn(&StateRegistry) -> usize,
) {
    let walked = walk(&sandbox.states());

    assert!(!sandbox.state_dir().exists());

    assert_eq!(walked, 0);
}

#[rstest]
fn path_joins_a_state_dir_that_is_a_file(sandbox: Sandbox) {
    sandbox.block_states();

    assert_eq!(
        sandbox.states().path(TEST_STATE_NAME).unwrap(),
        sandbox.state_dir().join(TEST_STATE_FILE)
    );
}

#[apply(invalid_names)]
fn resync_fails_when_the_name_is_invalid(
    sandbox: Sandbox,
    #[case] name: String,
    #[case] expected: NameError,
) {
    assert_err_is!(
        sandbox.states().resync(&name, sandbox.line_file()),
        Error::Name(error) if *error == expected
    );
}

#[rstest]
fn delete_is_seen_by_a_registry_sharing_the_state_dir(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);
    sandbox.reader().states().delete(TEST_STATE_NAME).unwrap();

    assert!(!sandbox.states().exists(TEST_STATE_NAME));
}

#[rstest]
fn walk_fails_when_the_state_dir_is_a_file(
    sandbox: Sandbox,
    #[values(Names, Count, All, Search, Clear)] walk: Walk,
) {
    sandbox.block_states();

    assert_err_is!(
        walk.call(&sandbox.states()),
        Error::Io(error) if error.kind() == ErrorKind::NotADirectory
    );
}

#[rstest]
fn load_verifies_by_default(sandbox: Sandbox) {
    sandbox.write_state(TEST_STATE_NAME, unverifiable(TEST_STATE_NAME));

    assert_err_is!(
        sandbox.states().load(TEST_STATE_NAME),
        Error::Mismatch {
            kind: Mismatch::Checksum,
            ..
        }
    );
}

#[rstest]
fn lookup_answers_when_the_state_is_a_directory(
    sandbox: Sandbox,
    #[values(Find, Exists)] probe: Probe,
) {
    fs::create_dir_all(sandbox.states().path(TEST_STATE_NAME).unwrap()).unwrap();

    assert!(!probe.call(&sandbox.states(), TEST_STATE_NAME));
}

#[rstest]
fn load_fails_when_the_name_is_suffixed(sandbox: Sandbox) {
    sandbox.checkpoint(&sandbox.line_file(), TEST_STATE_NAME, 2);

    assert_err_is!(
        sandbox.states().load(TEST_STATE_FILE),
        Error::NotFound(found) if found == TEST_STATE_FILE
    );
}

#[apply(valid_names)]
fn resync_accepts_a_valid_name(sandbox: Sandbox, #[case] name: String) {
    let path = sandbox.line_file();

    sandbox.saved(&path, &name);

    assert_eq!(sandbox.states().resync(&name, &path).unwrap().name, name);
}

#[rstest]
fn delete_fails_when_the_state_is_a_directory(sandbox: Sandbox) {
    fs::create_dir_all(sandbox.states().path(TEST_STATE_NAME).unwrap()).unwrap();

    assert_err_is!(sandbox.states().delete(TEST_STATE_NAME), Error::Io(_));
}

#[rstest]
fn resync_fails_when_the_state_is_unsaved(sandbox: Sandbox) {
    let path = sandbox.line_file();

    sandbox.state(&path);

    assert_err_is!(
        sandbox.states().resync(TEST_STATE_NAME, &path),
        Error::NotFound(name) if name == TEST_STATE_NAME
    );
}

#[rstest]
fn lookup_fails_when_the_name_is_suffixed(
    sandbox: Sandbox,
    #[values(Load, Delete)] lookup: Lookup,
) {
    let saved = sandbox.save(TEST_STATE_NAME);

    assert_err_is!(
        lookup.call(&sandbox.states(), TEST_STATE_FILE),
        Error::NotFound(name) if name == TEST_STATE_FILE
    );

    assert!(saved.exists());
}

#[rstest]
fn path_nests_under_a_subdirectory(sandbox: Sandbox) {
    let path = sandbox.states().path("sub/job-1").unwrap();

    assert_eq!(path.parent().unwrap(), sandbox.state_dir().join("sub"));
}

#[rstest]
fn tmp_nests_under_a_subdirectory(sandbox: Sandbox) {
    let tmp = sandbox.states().tmp_path("sub/job-1").unwrap();

    assert_eq!(tmp.parent().unwrap(), sandbox.state_dir().join("sub"));
}

#[apply(malformed_payloads)]
fn load_fails_when_the_payload_is_malformed(
    sandbox: Sandbox,
    #[case] payload: &str,
    #[case] message: &str,
) {
    sandbox.write_state(TEST_STATE_NAME, payload);

    assert_err_is!(
        sandbox.states().load(TEST_STATE_NAME),
        Error::Corrupt { name, error }
            if name == TEST_STATE_NAME && error.to_string().contains(message)
    );
}

#[rstest]
fn delete_removes_the_saved_state(sandbox: Sandbox) {
    let path = sandbox.save(TEST_STATE_NAME);

    sandbox.states().delete(TEST_STATE_NAME).unwrap();

    assert!(!path.exists());
    assert!(!sandbox.states().exists(TEST_STATE_NAME));
}

#[rstest]
fn autoname_differs_between_paths(sandbox: Sandbox) {
    assert_ne!(
        sandbox.states().autoname("/tmp/data-1.bin".as_ref()),
        sandbox.states().autoname("/tmp/data-2.bin".as_ref())
    );
}

#[rstest]
fn load_fails_when_the_state_is_a_directory(sandbox: Sandbox) {
    fs::create_dir_all(sandbox.states().path(TEST_STATE_NAME).unwrap()).unwrap();

    assert_err_is!(
        sandbox.states().load(TEST_STATE_NAME),
        Error::NotFound(found) if found == TEST_STATE_NAME
    );
}

#[rstest]
fn exists_reports_only_saved_states(sandbox: Sandbox) {
    assert!(!sandbox.states().exists(TEST_STATE_NAME));

    sandbox.save(TEST_STATE_NAME);

    assert!(sandbox.states().exists(TEST_STATE_NAME));
    assert!(!sandbox.states().exists(TEST_MISSING_STATE_NAME));
}

#[rstest]
fn load_returns_a_verified_state(sandbox: Sandbox) {
    sandbox.checkpoint(&sandbox.line_file(), TEST_STATE_NAME, 2);

    let state = sandbox.states().load(TEST_STATE_NAME).unwrap();

    assert_eq!(state.name, TEST_STATE_NAME);
    assert_eq!(state.position, 2);
}

#[rstest]
#[case::absolute(TEST_FILE_PATH)]
#[case::relative(TEST_FILE_NAME)]
#[case::empty("")]
#[case::directory("/tmp/")]
fn autoname_is_lowercase_hex(sandbox: Sandbox, #[case] file: &str) {
    let name = sandbox.states().autoname(file.as_ref());

    assert!(name.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()));

    assert_eq!(name.len(), 64);
}

#[cfg(unix)]
#[rstest]
fn lookup_fails_when_the_state_is_a_symlink(
    sandbox: Sandbox,
    #[values(Load, Delete, Path)] lookup: Lookup,
) {
    let real = sandbox.save(TEST_STATE_NAME);

    symlink(&real, sandbox.state_dir().join(TEST_OTHER_STATE_FILE)).unwrap();

    assert_err_is!(
        lookup.call(&sandbox.states(), TEST_OTHER_STATE_NAME),
        Error::Alias { name } if name == TEST_OTHER_STATE_NAME
    );
}

#[rstest]
fn save_fails_when_the_state_dir_is_a_file(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let mut bytes = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    sandbox.block_states();

    assert_err_is!(bytes.save(), Error::Io(_));
}

#[rstest]
fn find_returns_none_when_the_state_is_corrupt(sandbox: Sandbox) {
    let saved = sandbox.save(TEST_STATE_NAME);

    fs::write(&saved, "not valid json").unwrap();

    assert!(sandbox.states().exists(TEST_STATE_NAME));
    assert!(sandbox.states().find(TEST_STATE_NAME).is_none());
}

#[rstest]
fn path_is_relative_without_a_state_dir() {
    let reader = PReader::from(Config {
        state_dir: PathBuf::new(),
        ..Config::default()
    });

    assert_eq!(
        reader.states().path(TEST_STATE_NAME).unwrap(),
        PathBuf::from(".").join(TEST_STATE_FILE)
    );
}

#[cfg(unix)]
#[rstest]
fn lookup_answers_when_the_state_is_a_symlink(
    sandbox: Sandbox,
    #[values(Find, Exists)] probe: Probe,
) {
    let real = sandbox.save(TEST_STATE_NAME);

    sandbox.write_state(TEST_STATE_NAME, unverifiable(TEST_OTHER_STATE_NAME));

    symlink(&real, sandbox.state_dir().join(TEST_OTHER_STATE_FILE)).unwrap();

    assert!(!probe.call(sandbox.lenient().states(), TEST_OTHER_STATE_NAME));
}

#[rstest]
fn path_uses_the_native_separator(sandbox: Sandbox) {
    let path = sandbox.states().path(TEST_NESTED_STATE_NAME).unwrap();
    let expected = sandbox.state_dir().join("sub-1").join("sub-2").join(TEST_STATE_FILE);

    assert_eq!(path.as_os_str(), expected.as_os_str());
}

#[cfg(unix)]
#[rstest]
fn autoname_hashes_a_non_utf8_path(sandbox: Sandbox) {
    let path = OsStr::from_bytes(b"/tmp/data-\xff\xfe.bin").as_ref();
    let name = sandbox.states().autoname(path);

    assert_eq!(name, digest(b"/tmp/data-\xff\xfe.bin"));
}

#[rstest]
fn name_keeps_a_state_suffix(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let state = sandbox.named_state(&path, TEST_STATE_FILE);

    assert_eq!(state.name, TEST_STATE_FILE);
    assert_eq!(
        sandbox.states().path(TEST_STATE_FILE).unwrap().file_name().unwrap(),
        state_file(TEST_STATE_FILE).as_str()
    );
}

#[rstest]
fn count_includes_states_that_all_rejects(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);

    sandbox.write_state(TEST_OTHER_STATE_NAME, "not valid json");

    assert_eq!(sandbox.states().count().unwrap(), 2);

    assert_err_is!(
        sandbox.states().all(),
        Error::Corrupt { name, .. } if name == TEST_OTHER_STATE_NAME
    );
}

#[rstest]
fn load_fails_when_the_content_is_not_utf8(sandbox: Sandbox) {
    sandbox.write_state(TEST_STATE_NAME, b"{\"name\": \"\xff\"}");

    assert_err_is!(
        sandbox.states().load(TEST_STATE_NAME),
        Error::Corrupt { name, error }
            if name == TEST_STATE_NAME && error.to_string().contains("invalid unicode")
    );
}

#[rstest]
fn find_returns_none_when_the_state_does_not_verify(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);
    sandbox.append(&sandbox.path().join(TEST_FILE_NAME), b"more");

    assert!(sandbox.lenient().states().find(TEST_STATE_NAME).is_some());
    assert!(sandbox.states().find(TEST_STATE_NAME).is_none());
}

#[apply(alias_names)]
fn lookup_answers_when_the_name_is_a_case_or_unicode_alias(
    sandbox: Sandbox,
    #[case] name: &str,
    #[case] alias: &str,
    #[values(Find, Exists)] probe: Probe,
) {
    sandbox.save(name);
    sandbox.write_state(name, unverifiable(alias));

    if !sandbox.state_dir().join(format!("{alias}{TEST_STATE_FILE_EXTENSION}")).exists() {
        return;
    }

    assert!(!probe.call(sandbox.lenient().states(), alias));
}

#[rstest]
fn resync_fails_when_the_path_is_missing(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);

    assert_err_is!(
        sandbox.states().resync(TEST_STATE_NAME, sandbox.path().join("missing.bin")),
        Error::Io(error)
            if error.kind() == ErrorKind::NotFound && error.to_string().contains("os error")
    );
}

#[rstest]
fn load_ignores_unknown_fields_without_verification(sandbox: Sandbox) {
    let payload =
        unverifiable(TEST_STATE_NAME).replace(r#""_checksum""#, r#""foo": 42, "_checksum""#);

    sandbox.write_state(TEST_STATE_NAME, &payload);

    assert_eq!(
        sandbox.lenient().states().load(TEST_STATE_NAME).unwrap().position,
        7
    );
}

#[rstest]
fn find_returns_none_when_the_payload_names_another_state(sandbox: Sandbox) {
    let saved = sandbox.save(TEST_STATE_NAME);

    fs::copy(
        &saved,
        sandbox.states().path(TEST_OTHER_STATE_NAME).unwrap(),
    )
    .unwrap();

    assert!(sandbox.states().exists(TEST_OTHER_STATE_NAME));
    assert!(sandbox.states().find(TEST_OTHER_STATE_NAME).is_none());
}

#[rstest]
#[case::load(|registry: &StateRegistry, name: &str| registry.load(name).unwrap())]
#[case::find(|registry: &StateRegistry, name: &str| registry.find(name).unwrap())]
fn lookup_returns_the_saved_state(
    sandbox: Sandbox,
    #[case] lookup: fn(&StateRegistry, &str) -> State,
) {
    let path = sandbox.line_file();
    let saved = sandbox.checkpoint(&path, TEST_STATE_NAME, 2);

    let state = lookup(&sandbox.states(), TEST_STATE_NAME);

    assert_eq!(state, saved);
    assert_eq!(state.name, TEST_STATE_NAME);
    assert_eq!(state.file.path, canonical(&path));
}

#[rstest]
fn path_does_not_need_the_state_dir(sandbox: Sandbox) {
    let state_dir = sandbox.path().join("elsewhere");
    let path = sandbox
        .reader_in("elsewhere", sandbox.config())
        .states()
        .path(TEST_STATE_NAME)
        .unwrap();

    assert!(!state_dir.exists());

    assert_eq!(path.parent().unwrap(), state_dir);
}

#[rstest]
fn tmp_does_not_need_the_state_dir(sandbox: Sandbox) {
    let state_dir = sandbox.path().join("elsewhere");
    let tmp = sandbox
        .reader_in("elsewhere", sandbox.config())
        .states()
        .tmp_path(TEST_STATE_NAME)
        .unwrap();

    assert!(!state_dir.exists());

    assert_eq!(tmp.parent().unwrap(), state_dir);
}

#[rstest]
fn default_agrees_with_the_default_config() {
    let derived = StateRegistry::default().path(TEST_STATE_NAME).unwrap();
    let configured = StateRegistry::from(&Config::default()).path(TEST_STATE_NAME).unwrap();

    assert_eq!(derived, configured);
    assert_eq!(derived.parent().unwrap(), path::default_state_dir());
}

#[rstest]
fn clear_removes_a_corrupt_state(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);

    sandbox.write_state(TEST_OTHER_STATE_NAME, "not valid json");

    assert_eq!(sandbox.states().count().unwrap(), 2);

    sandbox.states().clear().unwrap();

    assert_eq!(sandbox.states().count().unwrap(), 0);
}

#[cfg(unix)]
#[rstest]
fn names_ignores_a_directory_alias(sandbox: Sandbox) {
    let real = format!("real/{TEST_STATE_NAME}");
    let saved = sandbox.save(&real);

    symlink(saved.parent().unwrap(), sandbox.state_dir().join("alias")).unwrap();

    assert_eq!(names(&sandbox.states()), [real]);
}

#[rstest]
fn save_leaves_no_temporary_file(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let mut bytes = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    bytes.save().unwrap();

    let leftovers = leftovers(&sandbox);

    assert!(leftovers.is_empty(), "{leftovers:?}");
}

#[rstest]
fn load_fails_when_a_field_has_the_wrong_type(sandbox: Sandbox) {
    sandbox.save(TEST_STATE_NAME);
    sandbox.patch(TEST_STATE_NAME, |payload| {
        payload["position"] = json!("abc")
    });

    assert_err_is!(
        sandbox.states().load(TEST_STATE_NAME),
        Error::Corrupt { name, error }
            if name == TEST_STATE_NAME && error.to_string().contains("invalid type")
    );
}

#[rstest]
#[case::plain("job-1")]
#[case::dotted("job.1")]
#[case::only_looks_suffixed("mystate.json")]
#[case::suffixed("job-1.state.json")]
fn tmp_keeps_the_name_before_the_stamp(sandbox: Sandbox, #[case] name: &str) {
    let tmp = sandbox.states().tmp_path(name).unwrap();
    let file_name = tmp.file_name().unwrap().to_str().unwrap();
    let (stem, _stamp) = file_name.strip_suffix(&tmp_suffix()).unwrap().rsplit_once('.').unwrap();

    assert_eq!(stem, name);
}

#[rstest]
fn resync_saves_the_resynced_state(sandbox: Sandbox) {
    let path = sandbox.write(TEST_TRACKED_NAME, b"foo");

    sandbox.saved(&path, TEST_STATE_NAME);
    sandbox.append(&path, b"more");

    let resynced = sandbox.states().resync(TEST_STATE_NAME, &path).unwrap();

    assert_eq!(sandbox.states().load(TEST_STATE_NAME).unwrap(), resynced);
}

#[rstest]
#[case::first(TEST_STATE_NAME)]
#[case::middle(TEST_OTHER_STATE_NAME)]
#[case::last("job-3")]
fn all_fails_when_any_state_is_corrupt(sandbox: Sandbox, #[case] corrupted: &str) {
    for name in [TEST_STATE_NAME, TEST_OTHER_STATE_NAME, "job-3"] {
        sandbox.save(name);
    }

    sandbox.write_state(corrupted, "not valid json");

    assert_err_is!(
        sandbox.states().all(),
        Error::Corrupt { name, error }
            if name == corrupted && error.to_string().contains("expected ident")
    );
}

#[cfg(unix)]
#[rstest]
fn resync_fails_when_the_state_is_a_symlink(sandbox: Sandbox) {
    let real = sandbox.save(TEST_STATE_NAME);
    let alias = sandbox.state_dir().join(TEST_OTHER_STATE_FILE);

    symlink(&real, &alias).unwrap();

    assert_err_is!(
        sandbox.states().resync(TEST_OTHER_STATE_NAME, sandbox.line_file()),
        Error::Alias { name } if name == TEST_OTHER_STATE_NAME
    );

    assert!(alias.is_symlink());
}

#[rstest]
fn save_fails_when_the_state_path_is_a_directory(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let mut bytes = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    fs::create_dir_all(sandbox.states().path(TEST_STATE_NAME).unwrap()).unwrap();

    assert_err_is!(bytes.save(), Error::Io(_));
}

#[cfg(unix)]
#[rstest]
fn path_fails_when_the_state_file_is_a_symlink(sandbox: Sandbox) {
    let outside = sandbox.write("outside.json", b"{}");

    fs::create_dir_all(sandbox.state_dir()).unwrap();

    symlink(&outside, sandbox.state_dir().join(TEST_STATE_FILE)).unwrap();

    assert_err_is!(
        sandbox.states().path(TEST_STATE_NAME),
        Error::Alias { name } if name == TEST_STATE_NAME
    );
}

#[rstest]
fn resync_saves_the_state_when_the_file_is_unchanged(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let mut state = sandbox.state(&path);

    state.timestamps = state_data(canonical(&path)).timestamps;

    sandbox.persist(&state);

    let resynced = sandbox.states().resync(TEST_STATE_NAME, &path).unwrap();

    assert_eq!(sandbox.states().load(TEST_STATE_NAME).unwrap(), resynced);
}

#[apply(invalid_names)]
fn every_lookup_rejects_an_invalid_name(
    sandbox: Sandbox,
    #[case] name: String,
    #[case] expected: NameError,
) {
    let registry = sandbox.states();

    assert_err_is!(registry.load(&name), Error::Name(error) if *error == expected);
    assert_err_is!(registry.delete(&name), Error::Name(error) if *error == expected);
    assert_err_is!(registry.path(&name), Error::Name(error) if *error == expected);
}

#[rstest]
fn load_fails_when_the_payload_names_another_state(sandbox: Sandbox) {
    let path = sandbox.line_file();

    sandbox.saved(&path, TEST_STATE_NAME);
    sandbox.patch(TEST_STATE_NAME, |payload| {
        payload["name"] = json!(TEST_OTHER_STATE_NAME)
    });

    assert_err_is!(
        sandbox.states().load(TEST_STATE_NAME),
        Error::Mismatch { name, kind: Mismatch::Name { saved } }
            if name == TEST_STATE_NAME && saved == TEST_OTHER_STATE_NAME
    );
}

#[cfg(unix)]
#[rstest]
fn load_fails_when_the_state_is_unreadable(sandbox: Sandbox) {
    let mut blocked = Blocked::default();

    if !Blocked::enforced(&sandbox.path().join("probe")) {
        return;
    }

    let saved = sandbox.save(TEST_STATE_NAME);

    blocked.block(&saved);

    assert_err_is!(
        sandbox.states().load(TEST_STATE_NAME),
        Error::Io(error) if error.kind() == ErrorKind::PermissionDenied
    );
}

#[rstest]
fn save_fails_when_a_directory_blocks_the_state_file(sandbox: Sandbox) {
    let nested = sandbox.save(&format!("{TEST_STATE_FILE}/{TEST_OTHER_STATE_NAME}"));
    let path = sandbox.line_file();
    let mut bytes = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    assert_err_is!(bytes.save(), Error::Io(_));

    assert!(nested.exists());
}

#[cfg(unix)]
#[rstest]
fn walk_fails_when_the_state_dir_is_unreadable(
    sandbox: Sandbox,
    #[values(Names, Count, All, Search, Clear)] walk: Walk,
) {
    let mut blocked = Blocked::default();

    if !Blocked::enforced(&sandbox.path().join("probe")) {
        return;
    }

    sandbox.save(TEST_STATE_NAME);
    blocked.block(sandbox.state_dir());

    assert_err_is!(
        walk.call(&sandbox.states()),
        Error::Io(error) if error.kind() == ErrorKind::PermissionDenied
    );
}

#[rstest]
fn tmp_carries_both_suffixes(sandbox: Sandbox) {
    let tmp = sandbox.states().tmp_path(TEST_STATE_NAME).unwrap();
    let name = tmp.file_name().unwrap().to_str().unwrap();

    assert!(name.starts_with(&format!("{TEST_STATE_NAME}.")));
    assert!(name.ends_with(&tmp_suffix()));

    assert_eq!(tmp.parent().unwrap(), sandbox.state_dir());
}

#[apply(alias_names)]
fn lookup_fails_when_the_name_is_a_case_or_unicode_alias(
    sandbox: Sandbox,
    #[case] name: &str,
    #[case] alias: &str,
    #[values(Load, Delete, Path)] lookup: Lookup,
) {
    sandbox.save(name);

    if !sandbox.state_dir().join(format!("{alias}{TEST_STATE_FILE_EXTENSION}")).exists() {
        return;
    }

    assert_err_is!(
        lookup.call(&sandbox.states(), alias),
        Error::Alias { name: reported } if reported == alias
    );

    assert!(names(&sandbox.states()).contains(&name.to_owned()));
}

#[rstest]
fn clear_removes_every_saved_state(sandbox: Sandbox) {
    let first = sandbox.save(TEST_STATE_NAME);
    let second = sandbox.save(TEST_OTHER_STATE_NAME);

    assert_eq!(sandbox.states().count().unwrap(), 2);

    sandbox.states().clear().unwrap();

    assert!(!first.exists());
    assert!(!second.exists());

    assert_eq!(sandbox.states().count().unwrap(), 0);
}

#[cfg(unix)]
#[rstest]
fn lookup_fails_when_the_state_is_a_dangling_symlink(
    sandbox: Sandbox,
    #[values(Load, Delete, Path)] lookup: Lookup,
) {
    let link = sandbox.state_dir().join(TEST_STATE_FILE);

    fs::create_dir_all(sandbox.state_dir()).unwrap();

    symlink(sandbox.path().join("missing.json"), &link).unwrap();

    assert_err_is!(
        lookup.call(&sandbox.states(), TEST_STATE_NAME),
        Error::Alias { name } if name == TEST_STATE_NAME
    );

    assert!(link.is_symlink());
}

#[apply(alias_names)]
fn resync_fails_when_the_name_is_a_case_or_unicode_alias(
    sandbox: Sandbox,
    #[case] name: &str,
    #[case] alias: &str,
) {
    sandbox.save(name);

    if !sandbox.state_dir().join(format!("{alias}{TEST_STATE_FILE_EXTENSION}")).exists() {
        return;
    }

    assert_err_is!(
        sandbox.states().resync(alias, sandbox.line_file()),
        Error::Alias { name: reported } if reported == alias
    );

    assert!(names(&sandbox.states()).contains(&name.to_owned()));
}

#[cfg(unix)]
#[rstest]
fn resync_fails_when_the_state_is_a_dangling_symlink(sandbox: Sandbox) {
    let link = sandbox.state_dir().join(TEST_STATE_FILE);

    fs::create_dir_all(sandbox.state_dir()).unwrap();

    symlink(sandbox.path().join("missing.json"), &link).unwrap();

    assert_err_is!(
        sandbox.states().resync(TEST_STATE_NAME, sandbox.line_file()),
        Error::Alias { name } if name == TEST_STATE_NAME
    );

    assert!(link.is_symlink());
}

#[rstest]
fn load_ignores_an_unknown_field(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let saved = sandbox.checkpoint(&path, TEST_STATE_NAME, 2);

    sandbox.patch(TEST_STATE_NAME, |payload| payload["foo"] = json!(42));

    let state = sandbox.states().load(TEST_STATE_NAME).unwrap();

    assert_eq!(state, saved);
    assert_eq!(state.name, TEST_STATE_NAME);
    assert_eq!(state.file.path, canonical(&path));
}

#[rstest]
fn clear_removes_states_at_every_depth(sandbox: Sandbox) {
    for name in TEST_EVERY_DEPTH {
        sandbox.save(name);
    }

    sandbox.states().clear().unwrap();

    let remaining = walkdir::WalkDir::new(sandbox.state_dir())
        .into_iter()
        .flatten()
        .filter(|entry| entry.file_type().is_file())
        .count();

    assert_eq!(sandbox.states().count().unwrap(), 0);
    assert_eq!(remaining, 0);
}

#[cfg(unix)]
#[rstest]
fn clear_fails_when_a_state_cannot_be_deleted(sandbox: Sandbox) {
    let mut blocked = Blocked::default();

    if !Blocked::enforced(&sandbox.path().join("probe")) {
        return;
    }

    let saved = sandbox.save(TEST_STATE_NAME);

    blocked.read_only(sandbox.state_dir());

    assert_err_is!(
        sandbox.states().clear(),
        Error::Io(error) if error.kind() == ErrorKind::PermissionDenied
    );

    assert!(saved.exists());
}

#[rstest]
fn save_leaves_no_temporary_file_when_it_fails(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let mut bytes = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    fs::create_dir_all(sandbox.states().path(TEST_STATE_NAME).unwrap()).unwrap();

    assert!(bytes.save().is_err());

    let leftovers = leftovers(&sandbox);

    assert!(leftovers.is_empty(), "{leftovers:?}");
}

#[rstest]
#[case::flat(TEST_STATE_NAME)]
#[case::nested(TEST_NESTED_STATE_NAME)]
#[case::deep(TEST_DEEP_STATE_NAME)]
fn save_creates_the_state_file_under_its_name(sandbox: Sandbox, #[case] state: &str) {
    let path = sandbox.line_file();
    let saved = sandbox
        .reader()
        .bytes(&path)
        .state(state)
        .build()
        .unwrap()
        .save()
        .unwrap()
        .clone();
    let file = sandbox.states().path(state).unwrap();

    assert!(file.is_file());

    assert_eq!(file, sandbox.state_dir().join(state_file(state)));

    read_state(&file, &saved);
}

#[cfg(unix)]
#[rstest]
fn lookup_answers_when_the_name_is_a_directory_alias(
    sandbox: Sandbox,
    #[values(Find, Exists)] probe: Probe,
) {
    let real = format!("real/{TEST_STATE_NAME}");
    let alias = format!("alias/{TEST_STATE_NAME}");
    let saved = sandbox.save(&real);

    sandbox.write_state(&real, unverifiable(&alias));

    symlink(saved.parent().unwrap(), sandbox.state_dir().join("alias")).unwrap();

    assert!(!probe.call(sandbox.lenient().states(), &alias));
}

#[rstest]
fn resync_canonicalizes_an_unnormalized_path(sandbox: Sandbox) {
    let target = sandbox.write("real.bin", b"foo");

    sandbox.saved(&target, TEST_STATE_NAME);
    sandbox.dir_at("folder");

    let detour = sandbox.path().join("folder").join("..").join("real.bin");

    assert_eq!(
        sandbox.states().resync(TEST_STATE_NAME, &detour).unwrap().file.path,
        canonical(&target)
    );

    assert_ne!(detour, target);
}

#[cfg(unix)]
#[rstest]
fn lookup_answers_when_the_name_escapes_through_a_symlink(
    sandbox: Sandbox,
    #[values(Find, Exists)] probe: Probe,
) {
    let name = format!("link/{TEST_STATE_NAME}");
    let outside = sandbox.dir_at("outside");

    fs::create_dir_all(sandbox.state_dir()).unwrap();

    symlink(&outside, sandbox.state_dir().join("link")).unwrap();

    fs::write(outside.join(TEST_STATE_FILE), unverifiable(&name)).unwrap();

    assert!(!probe.call(sandbox.lenient().states(), &name));
}

#[rstest]
fn save_fails_when_a_state_file_blocks_the_directory(sandbox: Sandbox) {
    let saved = sandbox.save(TEST_STATE_NAME);
    let before = fs::read(&saved).unwrap();
    let path = sandbox.line_file();
    let mut bytes = sandbox
        .reader()
        .bytes(&path)
        .state(format!("{TEST_STATE_FILE}/{TEST_OTHER_STATE_NAME}"))
        .build()
        .unwrap();

    assert_err_is!(bytes.save(), Error::Io(_));

    assert_eq!(fs::read(&saved).unwrap(), before);
}

#[cfg(unix)]
#[rstest]
fn path_fails_when_the_name_escapes_through_a_symlink(sandbox: Sandbox) {
    let outside = sandbox.dir_at("outside");

    fs::create_dir_all(sandbox.state_dir()).unwrap();

    symlink(&outside, sandbox.state_dir().join("link")).unwrap();

    let name = format!("link/{TEST_STATE_NAME}");

    assert_err_is!(sandbox.states().path(&name), Error::Alias { .. });
    assert_err_is!(sandbox.states().tmp_path(&name), Error::Alias { .. });
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

#[apply(alias_names)]
fn delete_never_touches_the_state_behind_an_alias(
    sandbox: Sandbox,
    #[case] name: &str,
    #[case] alias: &str,
) {
    let saved = sandbox.save(name);
    let before = fs::read(&saved).unwrap();
    let registry = sandbox.states();

    registry.delete(alias).ok();

    sandbox
        .reader()
        .bytes(sandbox.line_file())
        .state(alias)
        .build()
        .map(|mut bytes| bytes.save().is_ok())
        .ok();

    assert!(names(&registry).contains(&name.to_owned()));

    assert_eq!(fs::read(&saved).unwrap(), before);
}

#[cfg(unix)]
#[rstest]
#[case::a_symlink(true)]
#[case::an_unnormalized_path(false)]
fn resync_canonicalizes_a_symlink(sandbox: Sandbox, #[case] linked: bool) {
    let content = TEST_LINE.repeat(TEST_TRACKED_COPIES);

    sandbox.recorded(&content, 5);

    let target = sandbox.path().join(TEST_TRACKED_NAME);
    let detour = if linked {
        let alias = sandbox.path().join("link.bin");

        symlink(&target, &alias).unwrap();

        alias
    } else {
        sandbox.path().join(".").join(TEST_TRACKED_NAME)
    };

    assert_eq!(
        sandbox.states().resync(TEST_STATE_NAME, &detour).unwrap().file.path.as_os_str(),
        canonical(&target).as_os_str()
    );
}

#[cfg(unix)]
#[rstest]
fn lookup_fails_when_the_name_is_a_directory_alias(
    sandbox: Sandbox,
    #[values(Load, Delete, Path)] lookup: Lookup,
) {
    let saved = sandbox.save(&format!("real/{TEST_STATE_NAME}"));
    let before = fs::read(&saved).unwrap();
    let alias = format!("alias/{TEST_STATE_NAME}");

    symlink(saved.parent().unwrap(), sandbox.state_dir().join("alias")).unwrap();

    assert_err_is!(
        lookup.call(&sandbox.states(), &alias),
        Error::Alias { name } if *name == alias
    );

    assert_eq!(fs::read(&saved).unwrap(), before);
}

#[rstest]
fn tmp_uses_a_nanosecond_stamp(sandbox: Sandbox) {
    let before = UtcDateTime::now().unix_timestamp_nanos();
    let tmp = sandbox.states().tmp_path(TEST_STATE_NAME).unwrap();
    let after = UtcDateTime::now().unix_timestamp_nanos();
    let name = tmp.file_name().unwrap().to_str().unwrap();
    let stamp = name
        .strip_prefix(&format!("{TEST_STATE_NAME}."))
        .unwrap()
        .strip_suffix(&tmp_suffix())
        .unwrap();

    assert!((before..=after).contains(&stamp.parse::<i128>().unwrap()));
}

#[cfg(unix)]
#[rstest]
fn resync_fails_when_the_name_is_a_directory_alias(sandbox: Sandbox) {
    let saved = sandbox.save(&format!("real/{TEST_STATE_NAME}"));
    let before = fs::read(&saved).unwrap();
    let alias = format!("alias/{TEST_STATE_NAME}");

    symlink(saved.parent().unwrap(), sandbox.state_dir().join("alias")).unwrap();

    assert_err_is!(
        sandbox.states().resync(&alias, sandbox.line_file()),
        Error::Alias { name } if *name == alias
    );

    assert_eq!(fs::read(&saved).unwrap(), before);
}

#[cfg(unix)]
#[rstest]
fn lookup_fails_when_the_name_escapes_through_a_symlink(
    sandbox: Sandbox,
    #[values(Load, Delete, Path)] lookup: Lookup,
) {
    let outside = sandbox.dir_at("outside");
    let victim = outside.join(TEST_STATE_FILE);
    let escaping = format!("link/{TEST_STATE_NAME}");

    fs::create_dir_all(sandbox.state_dir()).unwrap();

    symlink(&outside, sandbox.state_dir().join("link")).unwrap();

    fs::write(&victim, "{}").unwrap();

    assert_err_is!(
        lookup.call(&sandbox.states(), &escaping),
        Error::Alias { name } if *name == escaping
    );

    assert!(victim.is_file());
}

#[cfg(unix)]
#[rstest]
fn walk_fails_when_a_subdirectory_is_unreadable(
    sandbox: Sandbox,
    #[values(Names, Count, All, Search, Clear)] walk: Walk,
) {
    let mut blocked = Blocked::default();

    if !Blocked::enforced(&sandbox.path().join("probe")) {
        return;
    }

    sandbox.save(TEST_STATE_NAME);

    let nested = sandbox.state_dir().join("sub");

    fs::create_dir_all(&nested).unwrap();
    fs::write(nested.join(TEST_OTHER_STATE_FILE), "{}").unwrap();

    blocked.block(&nested);

    assert_err_is!(
        walk.call(&sandbox.states()),
        Error::Io(error) if error.kind() == ErrorKind::PermissionDenied
    );
}

#[cfg(unix)]
#[rstest]
fn walk_is_empty_when_the_state_dir_cannot_be_created(sandbox: Sandbox) {
    let mut blocked = Blocked::default();

    if !Blocked::enforced(&sandbox.path().join("probe")) {
        return;
    }

    blocked.read_only(sandbox.path());

    let registry = sandbox.states();

    assert!(registry.clear().is_ok());
    assert!(!sandbox.state_dir().exists());

    assert_eq!(registry.names().unwrap().count(), 0);
    assert_eq!(registry.count().unwrap(), 0);
    assert_eq!(registry.all().unwrap().len(), 0);
}

#[apply(valid_names)]
fn tmp_keeps_the_name_beside_the_state_file(sandbox: Sandbox, #[case] name: String) {
    let tmp = sandbox.states().tmp_path(&name).unwrap();
    let path = sandbox.states().path(&name).unwrap();
    let file_name = path.file_name().unwrap().to_str().unwrap();
    let tmp_name = tmp.file_name().unwrap().to_str().unwrap();
    let stem = file_name.strip_suffix(TEST_STATE_FILE_EXTENSION).unwrap();

    assert!(tmp_name.starts_with(&format!("{stem}.")));
    assert!(tmp_name.ends_with(&tmp_suffix()));

    assert_eq!(tmp.parent(), path.parent());
}

#[cfg(unix)]
#[rstest]
#[case::missing(false, "No such file or directory")]
#[case::a_symlink_loop(true, "Too many levels of symbolic links")]
fn resync_fails_when_the_path_cannot_be_resolved(
    sandbox: Sandbox,
    #[case] looped: bool,
    #[case] message: &str,
) {
    sandbox.saved(&sandbox.line_file(), TEST_STATE_NAME);

    let target = if looped {
        let (first, second) = (
            sandbox.path().join("link-1.bin"),
            sandbox.path().join("link-2.bin"),
        );

        symlink(&second, &first).unwrap();
        symlink(&first, &second).unwrap();

        first
    } else {
        sandbox.path().join("missing.bin")
    };

    assert_err_is!(
        sandbox.states().resync(TEST_STATE_NAME, &target),
        Error::Io(error) if error.to_string().contains(message)
    );
}

#[cfg(unix)]
#[rstest]
fn resync_fails_when_the_name_escapes_through_a_symlink(sandbox: Sandbox) {
    let outside = sandbox.dir_at("outside");
    let victim = outside.join(TEST_STATE_FILE);
    let escaping = format!("link/{TEST_STATE_NAME}");

    fs::create_dir_all(sandbox.state_dir()).unwrap();

    symlink(&outside, sandbox.state_dir().join("link")).unwrap();

    fs::write(&victim, "{}").unwrap();

    assert_err_is!(
        sandbox.states().resync(&escaping, sandbox.line_file()),
        Error::Alias { name } if *name == escaping
    );

    assert_eq!(fs::read_to_string(&victim).unwrap(), "{}");
}

#[apply(invalid_names)]
fn lookup_answers_when_the_name_is_invalid(
    sandbox: Sandbox,
    #[case] name: String,
    #[case] _expected: NameError,
    #[values(Find, Exists)] probe: Probe,
) {
    let reader = sandbox.reader_in(
        "nested/preader",
        Config {
            verify_state: false,
            ..sandbox.config()
        },
    );
    let registry = reader.states();
    let mut stray = registry.state_dir().join(name.split('/').collect::<PathBuf>());

    stray.add_extension(TEST_STATE_FILE_EXTENSION.trim_start_matches('.'));

    if stray.starts_with(sandbox.path())
        && let Some(parent) = stray.parent()
        && fs::create_dir_all(parent).is_ok()
    {
        fs::write(&stray, unverifiable(&name)).ok();
    }

    assert!(!probe.call(registry, &name));
}

#[cfg(unix)]
#[rstest]
fn resync_fails_when_the_state_cannot_be_saved(sandbox: Sandbox) {
    let mut blocked = Blocked::default();

    if !Blocked::enforced(&sandbox.path().join("probe")) {
        return;
    }

    let path = sandbox.write(TEST_TRACKED_NAME, b"foo");
    let saved = sandbox.saved(&path, TEST_STATE_NAME);
    let before = fs::read(&saved).unwrap();

    sandbox.append(&path, b"more");
    blocked.read_only(sandbox.state_dir());

    assert_err_is!(
        sandbox.states().resync(TEST_STATE_NAME, &path),
        Error::Io(error) if error.kind() == ErrorKind::PermissionDenied
    );

    assert_eq!(fs::read(&saved).unwrap(), before);
}

#[rstest]
fn load_skips_verification_when_it_is_off(sandbox: Sandbox) {
    sandbox.write_state(TEST_STATE_NAME, unverifiable(TEST_STATE_NAME));

    let state = sandbox.lenient().states().load(TEST_STATE_NAME).unwrap();

    assert_eq!(state.name, TEST_STATE_NAME);
    assert_eq!(state.position, 7);
    assert_eq!(state.checksum, "not-a-real-checksum");
    assert_eq!(state.file.path.as_os_str(), TEST_FILE_PATH);
    assert_eq!(state.file.size, 4);
    assert_eq!(state.file.mtime.unix_timestamp(), TEST_STAMP);
    assert_eq!(state.file.fingerprint, TEST_LINE_FINGERPRINT);
    assert_eq!(state.timestamps.created_at.unix_timestamp(), TEST_STAMP + 1);
    assert_eq!(state.timestamps.updated_at.unix_timestamp(), TEST_STAMP + 2);
}

#[rstest]
fn save_replaces_a_hard_link_without_touching_its_target(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let mut bytes = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).build().unwrap();
    let outside = sandbox.write("outside.txt", b"foo");
    let link = sandbox.states().path(TEST_STATE_NAME).unwrap();

    fs::create_dir_all(sandbox.state_dir()).unwrap();
    fs::hard_link(&outside, &link).unwrap();

    let saved = bytes.save().unwrap().clone();

    assert_eq!(fs::read(&outside).unwrap(), b"foo");

    assert_ne!(fs::read(&link).unwrap(), b"foo");

    read_state(&link, &saved);
}

#[cfg(unix)]
#[rstest]
fn names_uses_a_symlinked_state_dir_as_the_root(sandbox: Sandbox) {
    let real = sandbox.dir_at("real");
    let link = sandbox.path().join("linked");

    symlink(&real, &link).unwrap();

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
        .save()
        .unwrap();

    assert!(real.join(TEST_STATE_FILE).is_file());

    assert_eq!(names(&registry), [TEST_STATE_NAME]);
    assert_eq!(
        registry.load(TEST_STATE_NAME).unwrap().name,
        TEST_STATE_NAME
    );

    registry.delete(TEST_STATE_NAME).unwrap();

    assert_eq!(fs::read_dir(&real).unwrap().count(), 0);
}

#[rstest]
fn resync_accepts_every_path_shape(sandbox: Sandbox) {
    let content = TEST_LINE.repeat(TEST_TRACKED_COPIES);

    sandbox.recorded(&content, 5);

    let path = sandbox.path().join(TEST_TRACKED_NAME);
    let text = path.to_str().unwrap().to_owned();
    let resolved = canonical(&path);

    for resynced in [
        sandbox.states().resync(TEST_STATE_NAME, text.as_str()).unwrap(),
        sandbox.states().resync(TEST_STATE_NAME, &text).unwrap(),
        sandbox.states().resync(TEST_STATE_NAME, text.clone()).unwrap(),
        sandbox.states().resync(TEST_STATE_NAME, path.as_path()).unwrap(),
        sandbox.states().resync(TEST_STATE_NAME, &path).unwrap(),
        sandbox.states().resync(TEST_STATE_NAME, path.clone()).unwrap(),
    ] {
        assert_eq!(resynced.file.path, resolved);
    }
}

#[rstest]
fn all_returns_every_saved_state(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let first = sandbox.checkpoint(&path, TEST_STATE_NAME, 2);
    let second = sandbox.checkpoint(&path, TEST_OTHER_STATE_NAME, 3);

    let mut states: Vec<(String, u64, PathBuf)> = sandbox
        .states()
        .all()
        .unwrap()
        .into_iter()
        .map(|state| (state.name, state.position, state.file.path))
        .collect();
    let mut loaded = sandbox.states().all().unwrap();

    states.sort();
    loaded.sort_by(|left, right| left.name.cmp(&right.name));

    assert_eq!(
        states,
        [
            (TEST_STATE_NAME.to_owned(), first.position, canonical(&path)),
            (
                TEST_OTHER_STATE_NAME.to_owned(),
                second.position,
                canonical(&path)
            ),
        ]
    );
    assert_eq!(loaded, [first, second]);
}

#[cfg(unix)]
#[rstest]
fn every_lookup_rejects_a_directory_alias(sandbox: Sandbox) {
    let real = format!("real/{TEST_STATE_NAME}");
    let alias = format!("alias/{TEST_STATE_NAME}");
    let saved = sandbox.save(&real);

    sandbox.write_state(&real, unverifiable(&alias));

    let before = fs::read(&saved).unwrap();
    let registry = sandbox.states();

    symlink(saved.parent().unwrap(), sandbox.state_dir().join("alias")).unwrap();

    let path = sandbox.line_file();
    let built = sandbox.reader().bytes(&path).state(alias.as_str()).build();

    assert!(sandbox.lenient().states().find(&alias).is_none());
    assert!(!registry.exists(&alias));

    assert_eq!(names(&registry), [real]);
    assert_eq!(fs::read(&saved).unwrap(), before);

    assert_err_is!(registry.load(&alias), Error::Alias { name } if *name == alias);
    assert_err_is!(registry.delete(&alias), Error::Alias { .. });
    assert_err_is!(built, Error::Alias { .. });
}

#[apply(valid_names)]
fn saved_name_round_trips_through_names(sandbox: Sandbox, #[case] name: String) {
    let path = sandbox.line_file();
    let state = sandbox
        .reader()
        .bytes(&path)
        .state(&name)
        .build()
        .unwrap()
        .save()
        .unwrap()
        .clone();
    let saved = sandbox.states().path(&name).unwrap();
    let registry = sandbox.states();
    let file = sandbox.state_dir().join(state_file(&name));
    let payload: Value = serde_json::from_str(&fs::read_to_string(&saved).unwrap()).unwrap();
    let pattern = format!("^{}$", regex::escape(&name));

    assert!(file.is_file());

    assert_eq!(saved, file);
    assert_eq!(payload["name"], name);
    assert_eq!(names(&registry), [name.as_str()]);
    assert_eq!(registry.count().unwrap(), 1);
    assert_eq!(items(registry.search(&pattern).unwrap()), [name.as_str()]);
    assert_eq!(registry.load(&name).unwrap(), state);

    read_state(&file, &state);

    registry.delete(&name).unwrap();

    assert!(!file.exists());

    sandbox.save(&name);
    registry.clear().unwrap();

    assert!(!file.exists());
}
