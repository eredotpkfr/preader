#[cfg(unix)]
use std::os::unix::fs::symlink;
#[cfg(unix)]
use std::{ffi::OsStr, os::unix::ffi::OsStrExt};
use std::{fs, path::Path};

use preader::{
    Config, FileMetadata, STATE_FILE_EXTENSION, State, StateData, StateManager, TMP_FILE_EXTENSION,
    Timestamps, default_state_dir,
};
#[cfg(unix)]
use preader::{Error, PathError};
use rstest::rstest;
use rstest_reuse::apply;
use sha2::{Digest, Sha256};

#[cfg(unix)]
use crate::common::macros::asserts::assert_err_is;
use crate::common::{
    constants::{TEST_FILE_PATH, TEST_LINE, TEST_LINE_FINGERPRINT, TEST_STATE_NAME},
    fixtures::sandbox,
    funcs::native,
    macros::asserts::{assert_err, assert_err_eq},
    sandbox::Sandbox,
    templates::{malformed_payloads, unsafe_names, windows_unsafe_names},
};

const PATH_DIGEST: &str = "07cb9e47c6d8681a47020d0bb04776e06ada8b6d7aabcf4838a127f98e9f4fe2";

fn lenient(sandbox: &Sandbox) -> StateManager {
    StateManager::from(&Config {
        verify_state: false,
        ..sandbox.config()
    })
}

fn manager_in(sandbox: &Sandbox, name: &str) -> StateManager {
    StateManager::from(&Config {
        state_dir: sandbox.path().join(name),
        ..sandbox.config()
    })
}

fn write_state(sandbox: &Sandbox, name: &str, payload: impl AsRef<[u8]>) {
    let path = sandbox.manager().path(name).unwrap();

    path.parent().map(fs::create_dir_all).transpose().unwrap();
    fs::write(path, payload).unwrap();
}

fn verifiable(sandbox: &Sandbox, name: &str, position: u64) -> State {
    let file = sandbox.file(TEST_LINE);
    let data = StateData {
        name: name.to_owned(),
        file: FileMetadata::try_from(file.as_path()).unwrap(),
        position,
        timestamps: Timestamps::now(),
        checksum: String::new(),
    };
    let mut state = sandbox.manager().state(data);

    state.save().unwrap();

    state
}

fn unverifiable(name: &str) -> String {
    format!(
        r#"{{
            "name": "{name}",
            "file": {{
                "path": "{TEST_FILE_PATH}",
                "size": 4,
                "mtime": 1700000000,
                "fingerprint": "{TEST_LINE_FINGERPRINT}"
            }},
            "position": 7,
            "timestamps": {{
                "created_at": 1700000001,
                "updated_at": 1700000002
            }},
            "_checksum": "not-a-real-checksum"
        }}"#
    )
}

#[rstest]
fn autoname_matches_a_known_digest(sandbox: Sandbox) {
    assert_eq!(
        sandbox.manager().autoname(Path::new(TEST_FILE_PATH)),
        PATH_DIGEST
    );
}

#[rstest]
fn autoname_hashes_the_path_bytes(sandbox: Sandbox) {
    let expected = hex::encode(Sha256::digest(TEST_FILE_PATH.as_bytes()));

    assert_eq!(
        sandbox.manager().autoname(Path::new(TEST_FILE_PATH)),
        expected
    );
}

#[rstest]
fn autoname_is_stable_for_one_path(sandbox: Sandbox) {
    let path = Path::new(TEST_FILE_PATH);

    assert_eq!(
        sandbox.manager().autoname(path),
        sandbox.manager().autoname(path)
    );
}

#[rstest]
fn autoname_differs_between_paths(sandbox: Sandbox) {
    assert_ne!(
        sandbox.manager().autoname(Path::new("/tmp/data-1.bin")),
        sandbox.manager().autoname(Path::new("/tmp/data-2.bin"))
    );
}

#[rstest]
#[case::current_dir_component(TEST_FILE_PATH, "/tmp/./data.bin")]
#[case::trailing_slash("/tmp/data", "/tmp/data/")]
#[case::relative_and_absolute("data.bin", "/tmp/data.bin")]
fn autoname_does_not_normalize(sandbox: Sandbox, #[case] left: &str, #[case] right: &str) {
    assert_ne!(
        sandbox.manager().autoname(Path::new(left)),
        sandbox.manager().autoname(Path::new(right))
    );
}

#[rstest]
#[case::absolute("/tmp/data.bin")]
#[case::relative("data.bin")]
#[case::empty("")]
#[case::directory("/tmp/")]
fn autoname_is_lowercase_hex(sandbox: Sandbox, #[case] file: &str) {
    let name = sandbox.manager().autoname(Path::new(file));

    assert!(name.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()));

    assert_eq!(name.len(), 64);
}

#[cfg(unix)]
#[rstest]
fn autoname_hashes_a_non_utf8_path(sandbox: Sandbox) {
    let path = Path::new(OsStr::from_bytes(b"/tmp/data-\xff\xfe.bin"));
    let name = sandbox.manager().autoname(path);

    assert_eq!(name.len(), 64);

    assert_ne!(name, sandbox.manager().autoname(Path::new("/tmp/data.bin")));
}

#[rstest]
fn path_appends_the_suffix(sandbox: Sandbox) {
    let path = sandbox.manager().path(TEST_STATE_NAME).unwrap();

    assert_eq!(path.parent().unwrap(), sandbox.state_dir());
    assert_eq!(
        path.file_name().unwrap(),
        format!("{TEST_STATE_NAME}{STATE_FILE_EXTENSION}").as_str()
    );
}

#[rstest]
#[case::plain("job-1", "job-1.state.json")]
#[case::with_a_dot("job.1", "job.1.state.json")]
#[case::only_looks_suffixed("mystate.json", "mystate.json.state.json")]
#[case::already_suffixed("job-1.state.json", "job-1.state.json")]
fn path_normalizes_the_name(sandbox: Sandbox, #[case] name: &str, #[case] expected: &str) {
    assert_eq!(
        sandbox.manager().path(name).unwrap().file_name().unwrap(),
        expected
    );
}

#[rstest]
#[case::plain("job-1")]
#[case::nested("sub/job-1")]
#[case::unnormalized("./job-1")]
#[case::already_suffixed("job-1.state.json")]
fn path_stays_inside_the_state_dir(sandbox: Sandbox, #[case] name: &str) {
    assert!(sandbox.manager().path(name).unwrap().starts_with(sandbox.state_dir()));
}

#[rstest]
fn path_nests_under_a_subdirectory(sandbox: Sandbox) {
    let path = sandbox.manager().path("sub/job-1").unwrap();

    assert_eq!(path.parent().unwrap(), sandbox.state_dir().join("sub"));
}

#[rstest]
fn path_ignores_a_trailing_slash(sandbox: Sandbox) {
    assert_eq!(
        sandbox.manager().path("job-1/").unwrap(),
        sandbox.manager().path("job-1").unwrap()
    );
}

#[rstest]
fn path_does_not_need_the_state_dir(sandbox: Sandbox) {
    let state_dir = sandbox.path().join("not-created-yet");
    let path = manager_in(&sandbox, "not-created-yet").path(TEST_STATE_NAME).unwrap();

    assert!(!state_dir.exists());

    assert_eq!(path.parent().unwrap(), state_dir);
}

#[apply(unsafe_names)]
fn path_fails_when_the_name_is_unsafe(
    sandbox: Sandbox,
    #[case] name: &str,
    #[case] message: String,
) {
    assert_err_eq!(sandbox.manager().path(name), message);
}

#[cfg(windows)]
#[apply(windows_unsafe_names)]
fn path_fails_when_a_windows_name_is_unsafe(
    sandbox: Sandbox,
    #[case] name: &str,
    #[case] message: &str,
) {
    assert_err_eq!(sandbox.manager().path(name), message);
}

#[cfg(unix)]
#[apply(windows_unsafe_names)]
fn path_accepts_a_windows_name_on_unix(
    sandbox: Sandbox,
    #[case] name: &str,
    #[case] _message: &str,
) {
    assert_eq!(
        sandbox.manager().path(name).unwrap(),
        sandbox.state_dir().join(format!("{name}{STATE_FILE_EXTENSION}"))
    );
}

#[cfg(unix)]
#[rstest]
fn path_fails_when_name_escapes_through_symlink(sandbox: Sandbox) {
    let outside = sandbox.dir_at("outside");

    fs::create_dir_all(sandbox.state_dir()).unwrap();
    symlink(&outside, sandbox.state_dir().join("link")).unwrap();

    assert_err_is!(
        sandbox.manager().path("link/job-1"),
        Error::Path(PathError::Symlink(_))
    );
    assert_err_is!(
        sandbox.manager().tmp("link/job-1"),
        Error::Path(PathError::Symlink(_))
    );
}

#[rstest]
fn tmp_carries_both_suffixes(sandbox: Sandbox) {
    let tmp = sandbox.manager().tmp(TEST_STATE_NAME).unwrap();
    let name = tmp.file_name().unwrap().to_str().unwrap();

    assert!(name.starts_with(&format!("{TEST_STATE_NAME}.")));
    assert!(name.ends_with(&format!("{STATE_FILE_EXTENSION}.{TMP_FILE_EXTENSION}")));

    assert_eq!(tmp.parent().unwrap(), sandbox.state_dir());
}

#[rstest]
fn tmp_differs_between_calls(sandbox: Sandbox) {
    assert_ne!(
        sandbox.manager().tmp(TEST_STATE_NAME).unwrap(),
        sandbox.manager().tmp(TEST_STATE_NAME).unwrap()
    );
}

#[rstest]
fn tmp_never_collides_with_the_state_file(sandbox: Sandbox) {
    assert_ne!(
        sandbox.manager().tmp(TEST_STATE_NAME).unwrap(),
        sandbox.manager().path(TEST_STATE_NAME).unwrap()
    );
}

#[rstest]
fn tmp_uses_a_nanosecond_stamp(sandbox: Sandbox) {
    let tmp = sandbox.manager().tmp(TEST_STATE_NAME).unwrap();
    let name = tmp.file_name().unwrap().to_str().unwrap();
    let stamp = name
        .strip_prefix(&format!("{TEST_STATE_NAME}."))
        .unwrap()
        .strip_suffix(&format!("{STATE_FILE_EXTENSION}.{TMP_FILE_EXTENSION}"))
        .unwrap();

    assert!(stamp.parse::<i64>().unwrap() > 0);
}

#[rstest]
#[case::plain("job-1", "job-1")]
#[case::with_a_dot("job.1", "job.1")]
#[case::only_looks_suffixed("mystate.json", "mystate.json")]
#[case::already_suffixed("job-1.state.json", "job-1")]
fn tmp_normalizes_the_name(sandbox: Sandbox, #[case] name: &str, #[case] expected: &str) {
    let tmp = sandbox.manager().tmp(name).unwrap();
    let file_name = tmp.file_name().unwrap().to_str().unwrap();
    let tail = format!("{STATE_FILE_EXTENSION}.{TMP_FILE_EXTENSION}");
    let (stem, _stamp) = file_name.strip_suffix(&tail).unwrap().rsplit_once('.').unwrap();

    assert_eq!(stem, expected);
}

#[rstest]
fn tmp_nests_under_a_subdirectory(sandbox: Sandbox) {
    let tmp = sandbox.manager().tmp("sub/job-1").unwrap();

    assert_eq!(tmp.parent().unwrap(), sandbox.state_dir().join("sub"));
}

#[rstest]
fn tmp_does_not_need_the_state_dir(sandbox: Sandbox) {
    let state_dir = sandbox.path().join("not-created-yet");
    let tmp = manager_in(&sandbox, "not-created-yet").tmp(TEST_STATE_NAME).unwrap();

    assert!(!state_dir.exists());

    assert_eq!(tmp.parent().unwrap(), state_dir);
}

#[apply(unsafe_names)]
fn tmp_fails_when_the_name_is_unsafe(
    sandbox: Sandbox,
    #[case] name: &str,
    #[case] message: String,
) {
    assert_err_eq!(sandbox.manager().tmp(name), message);
}

#[cfg(windows)]
#[apply(windows_unsafe_names)]
fn tmp_fails_when_a_windows_name_is_unsafe(
    sandbox: Sandbox,
    #[case] name: &str,
    #[case] message: &str,
) {
    assert_err_eq!(sandbox.manager().tmp(name), message);
}

#[rstest]
fn state_wraps_the_data_with_the_manager(sandbox: Sandbox) {
    let state = verifiable(&sandbox, TEST_STATE_NAME, 2);

    assert_eq!(state.name, TEST_STATE_NAME);
    assert_eq!(state.position, 2);
    assert_eq!(
        state.path().unwrap(),
        sandbox.manager().path(TEST_STATE_NAME).unwrap()
    );
}

#[rstest]
fn load_returns_a_verified_state(sandbox: Sandbox) {
    verifiable(&sandbox, TEST_STATE_NAME, 2);

    let state = sandbox.manager().load(TEST_STATE_NAME).unwrap();

    assert_eq!(state.name, TEST_STATE_NAME);
    assert_eq!(state.position, 2);
}

#[rstest]
fn load_accepts_an_already_suffixed_name(sandbox: Sandbox) {
    verifiable(&sandbox, TEST_STATE_NAME, 2);

    let suffixed = format!("{TEST_STATE_NAME}{STATE_FILE_EXTENSION}");

    assert_eq!(sandbox.manager().load(&suffixed).unwrap().position, 2);
}

#[rstest]
fn load_fails_when_the_state_is_missing(sandbox: Sandbox) {
    assert_err!(sandbox.manager().load(TEST_STATE_NAME), "state not found");
}

#[rstest]
fn load_fails_when_the_path_is_a_directory(sandbox: Sandbox) {
    fs::create_dir_all(sandbox.manager().path(TEST_STATE_NAME).unwrap()).unwrap();

    assert_err!(sandbox.manager().load(TEST_STATE_NAME), "state not found");
}

#[apply(malformed_payloads)]
fn load_fails_when_the_payload_is_malformed(
    sandbox: Sandbox,
    #[case] payload: &str,
    #[case] message: &str,
) {
    write_state(&sandbox, TEST_STATE_NAME, payload);

    assert_err!(sandbox.manager().load(TEST_STATE_NAME), message);
}

#[rstest]
fn load_fails_when_the_content_is_not_utf8(sandbox: Sandbox) {
    write_state(&sandbox, TEST_STATE_NAME, b"{\"name\": \"\xff\"}");

    assert_err!(sandbox.manager().load(TEST_STATE_NAME), "valid UTF-8");
}

#[apply(unsafe_names)]
fn load_fails_when_the_name_is_unsafe(
    sandbox: Sandbox,
    #[case] name: &str,
    #[case] message: String,
) {
    assert_err_eq!(sandbox.manager().load(name), message);
}

#[cfg(windows)]
#[apply(windows_unsafe_names)]
fn load_fails_when_a_windows_name_is_unsafe(
    sandbox: Sandbox,
    #[case] name: &str,
    #[case] message: &str,
) {
    assert_err_eq!(sandbox.manager().load(name), message);
}

#[rstest]
fn load_verifies_by_default(sandbox: Sandbox) {
    write_state(&sandbox, TEST_STATE_NAME, unverifiable(TEST_STATE_NAME));

    assert_err!(
        sandbox.manager().load(TEST_STATE_NAME),
        "state checksum mismatch"
    );
}

#[rstest]
fn load_skips_verification_when_it_is_off(sandbox: Sandbox) {
    write_state(&sandbox, TEST_STATE_NAME, unverifiable(TEST_STATE_NAME));

    let state = lenient(&sandbox).load(TEST_STATE_NAME).unwrap();

    assert_eq!(state.name, TEST_STATE_NAME);
    assert_eq!(state.position, 7);
    assert_eq!(state.checksum, "not-a-real-checksum");
    assert_eq!(state.file.path, Path::new(TEST_FILE_PATH));
    assert_eq!(state.file.size, 4);
    assert_eq!(state.file.mtime.timestamp(), 1_700_000_000);
    assert_eq!(state.file.fingerprint, TEST_LINE_FINGERPRINT);
    assert_eq!(state.timestamps.created_at.timestamp(), 1_700_000_001);
    assert_eq!(state.timestamps.updated_at.timestamp(), 1_700_000_002);
}

#[rstest]
fn load_ignores_unknown_fields(sandbox: Sandbox) {
    let payload =
        unverifiable(TEST_STATE_NAME).replace(r#""_checksum""#, r#""foo": 42, "_checksum""#);

    write_state(&sandbox, TEST_STATE_NAME, &payload);

    assert_eq!(lenient(&sandbox).load(TEST_STATE_NAME).unwrap().position, 7);
}

#[rstest]
fn from_config_carries_the_state_dir(sandbox: Sandbox) {
    let path = sandbox.manager().path(TEST_STATE_NAME).unwrap();

    assert_eq!(path.parent().unwrap(), sandbox.state_dir());
}

#[rstest]
fn default_agrees_with_the_default_config() {
    let derived = StateManager::default().path(TEST_STATE_NAME).unwrap();
    let configured = StateManager::from(&Config::default()).path(TEST_STATE_NAME).unwrap();

    assert_eq!(derived, configured);
    assert_eq!(derived.parent().unwrap(), default_state_dir());
}
