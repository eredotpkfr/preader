#[cfg(unix)]
use std::{ffi::OsStr, os::unix::ffi::OsStrExt, os::unix::fs::symlink};
use std::{
    fs,
    path::{Path, PathBuf},
};

use preader::{
    Config, DEFAULT_VERIFY_STATE, FileMetadata, STATE_FILE_SUFFIX, State, StateData, StateManager,
    TMP_STATE_FILE_SUFFIX, Timestamps, default_state_dir,
};
use rstest::{fixture, rstest};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

use crate::common::{
    constants::{TEST_FILE_PATH, TEST_FINGERPRINT, TEST_STATE_NAME},
    fixtures::tmp_dir,
};

const FILE_PATH_DIGEST: &str = "07cb9e47c6d8681a47020d0bb04776e06ada8b6d7aabcf4838a127f98e9f4fe2";

struct Sandbox {
    tmp_dir: TempDir,
    manager: StateManager,
}

impl Sandbox {
    fn state_dir(&self) -> &Path {
        self.tmp_dir.path()
    }

    fn load_error(&self, name: &str) -> String {
        self.manager.load(name).err().unwrap().to_string()
    }

    fn write_state(&self, name: &str, payload: impl AsRef<[u8]>) {
        fs::write(self.manager.path(name).unwrap(), payload).unwrap();
    }

    fn save_verifiable_state(&self, name: &str, position: u64) -> PathBuf {
        let file = self.state_dir().join("data.bin");

        fs::write(&file, b"foo\n").unwrap();

        let data = StateData {
            name: name.to_string(),
            file: FileMetadata::try_from(file.as_path()).unwrap(),
            position,
            timestamps: Timestamps::now(),
            checksum: String::new(),
        };

        State::from((data, self.manager.clone())).save().unwrap()
    }
}

#[fixture]
fn sandbox(#[default(true)] verify_state: bool, tmp_dir: TempDir) -> Sandbox {
    let config = Config {
        state_dir: tmp_dir.path().to_path_buf(),
        verify_state,
        ..Config::default()
    };
    let manager = StateManager::from(&config);

    Sandbox { tmp_dir, manager }
}

fn manager_in(state_dir: &Path) -> StateManager {
    StateManager::from(&Config {
        state_dir: state_dir.to_path_buf(),
        ..Config::default()
    })
}

fn unverifiable_payload(name: &str) -> String {
    format!(
        r#"{{
            "name": "{name}",
            "file": {{
                "path": "{TEST_FILE_PATH}",
                "size": 4,
                "mtime": 1700000000,
                "fingerprint": "{TEST_FINGERPRINT}"
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
fn name_matches_the_precomputed_digest(sandbox: Sandbox) {
    assert_eq!(
        sandbox.manager.name(Path::new(TEST_FILE_PATH)),
        FILE_PATH_DIGEST
    );
}

#[rstest]
fn name_hashes_the_path_bytes(sandbox: Sandbox) {
    let expected = hex::encode(Sha256::digest(TEST_FILE_PATH.as_bytes()));

    assert_eq!(sandbox.manager.name(Path::new(TEST_FILE_PATH)), expected);
}

#[rstest]
fn name_is_stable_for_the_same_path(sandbox: Sandbox) {
    let path = Path::new(TEST_FILE_PATH);

    assert_eq!(sandbox.manager.name(path), sandbox.manager.name(path));
}

#[rstest]
fn name_differs_between_paths(sandbox: Sandbox) {
    assert_ne!(
        sandbox.manager.name(Path::new("/tmp/data-1.bin")),
        sandbox.manager.name(Path::new("/tmp/data-2.bin"))
    );
}

#[rstest]
#[case::current_dir_component(TEST_FILE_PATH, "/tmp/./data.bin")]
#[case::trailing_slash("/tmp/data", "/tmp/data/")]
#[case::relative_and_absolute("data.bin", "/tmp/data.bin")]
fn name_does_not_normalize_the_path(sandbox: Sandbox, #[case] left: &str, #[case] right: &str) {
    assert_ne!(
        sandbox.manager.name(Path::new(left)),
        sandbox.manager.name(Path::new(right))
    );
}

#[rstest]
#[case::absolute("/tmp/data.bin")]
#[case::relative("data.bin")]
#[case::empty("")]
#[case::directory("/tmp/")]
fn name_is_a_lowercase_hex_digest(sandbox: Sandbox, #[case] file: &str) {
    let name = sandbox.manager.name(Path::new(file));

    assert_eq!(name.len(), 64);
    assert!(name.chars().all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
}

#[cfg(unix)]
#[rstest]
fn name_hashes_non_utf8_paths(sandbox: Sandbox) {
    let path = Path::new(OsStr::from_bytes(b"/tmp/data-\xff\xfe.bin"));
    let name = sandbox.manager.name(path);

    assert_eq!(name.len(), 64);
    assert_ne!(name, sandbox.manager.name(Path::new("/tmp/data.bin")));
}

#[rstest]
fn name_ignores_the_config(#[with(false)] sandbox: Sandbox, #[from(sandbox)] other: Sandbox) {
    let path = Path::new(TEST_FILE_PATH);

    assert_eq!(sandbox.manager.name(path), other.manager.name(path));
}

#[rstest]
fn path_appends_the_suffix_inside_the_state_dir(sandbox: Sandbox) {
    let path = sandbox.manager.path(TEST_STATE_NAME).unwrap();

    assert_eq!(path.parent().unwrap(), sandbox.state_dir());
    assert_eq!(
        path.file_name().unwrap(),
        format!("{TEST_STATE_NAME}.{STATE_FILE_SUFFIX}").as_str()
    );
}

#[rstest]
#[case::traversal("../../etc/passwd", "path escapes root")]
#[case::absolute("/tmp", "path escapes root")]
#[case::empty("", "path must not be empty")]
#[case::current_dir(".", "path must not be empty")]
fn path_fails_when_the_name_is_unsafe(sandbox: Sandbox, #[case] name: &str, #[case] message: &str) {
    let error = sandbox.manager.path(name).unwrap_err();

    assert!(error.to_string().contains(message));
}

#[cfg(windows)]
#[rstest]
#[case::drive_absolute("C:\\job-1")]
#[case::drive_relative("C:job-1")]
#[case::root_relative("\\job-1")]
#[case::unc_share("\\\\server\\share\\job-1")]
#[case::verbatim_drive("\\\\?\\C:\\job-1")]
#[case::backslash_traversal("..\\..\\etc\\passwd")]
fn path_fails_when_a_windows_name_is_unsafe(sandbox: Sandbox, #[case] name: &str) {
    assert!(sandbox.manager.path(name).is_err());
    assert!(sandbox.manager.tmp(name).is_err());
}

#[rstest]
#[case::plain("job-1")]
#[case::nested("sub/job-1")]
#[case::unnormalized("./job-1")]
#[case::already_suffixed("job-1.state.json")]
fn path_stays_inside_the_state_dir(sandbox: Sandbox, #[case] name: &str) {
    let path = sandbox.manager.path(name).unwrap();

    assert!(path.starts_with(sandbox.state_dir()));
}

#[rstest]
fn path_nests_under_a_subdirectory_name(sandbox: Sandbox) {
    let path = sandbox.manager.path("sub/job-1").unwrap();

    assert_eq!(path.parent().unwrap(), sandbox.state_dir().join("sub"));
}

#[rstest]
#[case::plain("job-1", "job-1.state.json")]
#[case::with_a_dot("job.1", "job.1.state.json")]
#[case::only_looks_suffixed("mystate.json", "mystate.json.state.json")]
#[case::already_suffixed("job-1.state.json", "job-1.state.json")]
fn path_normalizes_the_name(sandbox: Sandbox, #[case] name: &str, #[case] expected: &str) {
    let path = sandbox.manager.path(name).unwrap();

    assert_eq!(path.file_name().unwrap(), expected);
}

#[rstest]
fn path_ignores_a_trailing_slash(sandbox: Sandbox) {
    assert_eq!(
        sandbox.manager.path("job-1/").unwrap(),
        sandbox.manager.path("job-1").unwrap()
    );
}

#[rstest]
fn path_does_not_need_the_state_dir(tmp_dir: TempDir) {
    let state_dir = tmp_dir.path().join("not-created-yet");
    let path = manager_in(&state_dir).path(TEST_STATE_NAME).unwrap();

    assert!(!state_dir.exists());
    assert_eq!(path.parent().unwrap(), state_dir);
}

#[rstest]
fn tmp_carries_both_suffixes_inside_the_state_dir(sandbox: Sandbox) {
    let tmp = sandbox.manager.tmp(TEST_STATE_NAME).unwrap();
    let file_name = tmp.file_name().unwrap().to_str().unwrap().to_string();

    assert_eq!(tmp.parent().unwrap(), sandbox.state_dir());
    assert!(file_name.starts_with(&format!("{TEST_STATE_NAME}.")));
    assert!(file_name.ends_with(&format!(".{STATE_FILE_SUFFIX}.{TMP_STATE_FILE_SUFFIX}")));
}

#[rstest]
fn tmp_differs_between_calls(sandbox: Sandbox) {
    assert_ne!(
        sandbox.manager.tmp(TEST_STATE_NAME).unwrap(),
        sandbox.manager.tmp(TEST_STATE_NAME).unwrap()
    );
}

#[rstest]
#[case::traversal("../../etc/passwd", "path escapes root")]
#[case::absolute("/tmp", "path escapes root")]
#[case::empty("", "path must not be empty")]
#[case::current_dir(".", "path must not be empty")]
fn tmp_fails_when_the_name_is_unsafe(sandbox: Sandbox, #[case] name: &str, #[case] message: &str) {
    let error = sandbox.manager.tmp(name).unwrap_err();

    assert!(error.to_string().contains(message));
}

#[rstest]
#[case::plain("job-1", "job-1")]
#[case::with_a_dot("job.1", "job.1")]
#[case::only_looks_suffixed("mystate.json", "mystate.json")]
#[case::already_suffixed("job-1.state.json", "job-1")]
fn tmp_normalizes_the_name(sandbox: Sandbox, #[case] name: &str, #[case] expected: &str) {
    let tmp = sandbox.manager.tmp(name).unwrap();
    let file_name = tmp.file_name().unwrap().to_str().unwrap();
    let tail = format!(".{STATE_FILE_SUFFIX}.{TMP_STATE_FILE_SUFFIX}");
    let (stem, _stamp) = file_name.strip_suffix(&tail).unwrap().rsplit_once('.').unwrap();

    assert_eq!(stem, expected);
}

#[rstest]
fn tmp_uses_a_nanosecond_timestamp(sandbox: Sandbox) {
    let tmp = sandbox.manager.tmp(TEST_STATE_NAME).unwrap();
    let file_name = tmp.file_name().unwrap().to_str().unwrap();
    let stamp = file_name
        .strip_prefix(&format!("{TEST_STATE_NAME}."))
        .unwrap()
        .strip_suffix(&format!(".{STATE_FILE_SUFFIX}.{TMP_STATE_FILE_SUFFIX}"))
        .unwrap();

    assert!(stamp.parse::<i64>().unwrap() > 0);
}

#[rstest]
fn tmp_never_collides_with_the_state_file(sandbox: Sandbox) {
    assert_ne!(
        sandbox.manager.tmp(TEST_STATE_NAME).unwrap(),
        sandbox.manager.path(TEST_STATE_NAME).unwrap()
    );
}

#[rstest]
fn tmp_does_not_need_the_state_dir(tmp_dir: TempDir) {
    let state_dir = tmp_dir.path().join("not-created-yet");
    let tmp = manager_in(&state_dir).tmp(TEST_STATE_NAME).unwrap();

    assert!(!state_dir.exists());
    assert_eq!(tmp.parent().unwrap(), state_dir);
}

#[rstest]
fn tmp_nests_under_a_subdirectory_name(sandbox: Sandbox) {
    let tmp = sandbox.manager.tmp("sub/job-1").unwrap();

    assert_eq!(tmp.parent().unwrap(), sandbox.state_dir().join("sub"));
}

#[rstest]
#[case::one_level("sub/job-1")]
#[case::four_levels("sub-1/sub-2/sub-3/sub-4/job-1")]
fn load_reads_back_a_nested_state(sandbox: Sandbox, #[case] name: &str) {
    let path = sandbox.save_verifiable_state(name, 3);

    assert!(path.is_file());
    assert_eq!(path, sandbox.manager.path(name).unwrap());
    assert_eq!(sandbox.manager.load(name).unwrap().position, 3);
}

#[cfg(unix)]
#[rstest]
fn tmp_fails_when_the_name_escapes_through_a_symlink(sandbox: Sandbox) {
    let outside = sandbox.tmp_dir.path().join("outside");

    fs::create_dir_all(&outside).unwrap();
    symlink(&outside, sandbox.state_dir().join("link")).unwrap();

    assert!(sandbox.manager.tmp("link/job-1").is_err());
}

#[rstest]
fn load_fails_when_the_state_is_missing(sandbox: Sandbox) {
    assert!(sandbox.load_error(TEST_STATE_NAME).contains("state not found"));
}

#[rstest]
#[case::unparsable("not valid json", "expected ident")]
#[case::missing_fields("{}", "missing field `name`")]
#[case::empty("", "EOF while parsing")]
fn load_fails_when_the_payload_is_malformed(
    sandbox: Sandbox,
    #[case] payload: &str,
    #[case] message: &str,
) {
    sandbox.write_state(TEST_STATE_NAME, payload);

    assert!(sandbox.load_error(TEST_STATE_NAME).contains(message));
}

#[rstest]
#[case::traversal("../../etc/passwd", "path escapes root")]
#[case::absolute("/tmp", "path escapes root")]
#[case::empty("", "path must not be empty")]
#[case::current_dir(".", "path must not be empty")]
fn load_fails_when_the_name_is_unsafe(sandbox: Sandbox, #[case] name: &str, #[case] message: &str) {
    assert!(sandbox.load_error(name).contains(message));
}

#[rstest]
fn load_deserializes_the_payload_without_verification(#[with(false)] sandbox: Sandbox) {
    sandbox.write_state(TEST_STATE_NAME, unverifiable_payload(TEST_STATE_NAME));

    let state = sandbox.manager.load(TEST_STATE_NAME).unwrap();

    assert_eq!(state.name, TEST_STATE_NAME);
    assert_eq!(state.position, 7);
    assert_eq!(state.checksum, "not-a-real-checksum");

    assert_eq!(state.file.path, Path::new(TEST_FILE_PATH));
    assert_eq!(state.file.size, 4);
    assert_eq!(state.file.mtime.timestamp(), 1_700_000_000);
    assert_eq!(state.file.fingerprint, TEST_FINGERPRINT);

    assert_eq!(state.timestamps.created_at.timestamp(), 1_700_000_001);
    assert_eq!(state.timestamps.updated_at.timestamp(), 1_700_000_002);
}

#[rstest]
fn load_verifies_by_default(sandbox: Sandbox) {
    sandbox.write_state(TEST_STATE_NAME, unverifiable_payload(TEST_STATE_NAME));

    assert!(sandbox.load_error(TEST_STATE_NAME).contains("state checksum mismatch"));
}

#[rstest]
fn load_accepts_an_already_suffixed_name(sandbox: Sandbox) {
    let path = sandbox.save_verifiable_state(TEST_STATE_NAME, 2);
    let suffixed = path.file_name().unwrap().to_str().unwrap();
    let state = sandbox.manager.load(suffixed).unwrap();

    assert_eq!(state.position, 2);
}

#[rstest]
fn load_fails_when_the_path_is_a_directory(sandbox: Sandbox) {
    fs::create_dir_all(sandbox.manager.path(TEST_STATE_NAME).unwrap()).unwrap();

    assert!(sandbox.load_error(TEST_STATE_NAME).contains("state not found"));
}

#[rstest]
fn load_fails_when_the_content_is_not_utf8(sandbox: Sandbox) {
    sandbox.write_state(TEST_STATE_NAME, b"{\"name\": \"\xff\"}");

    assert!(sandbox.load_error(TEST_STATE_NAME).contains("valid UTF-8"));
}

#[rstest]
fn load_returns_a_verified_state(sandbox: Sandbox) {
    sandbox.save_verifiable_state(TEST_STATE_NAME, 2);

    let state = sandbox.manager.load(TEST_STATE_NAME).unwrap();

    assert_eq!(state.name, TEST_STATE_NAME);
    assert_eq!(state.position, 2);
}

#[rstest]
fn load_carries_the_managers_saved_position(#[with(false)] mut sandbox: Sandbox) {
    sandbox.manager.last_saved_position = 512;
    sandbox.write_state(TEST_STATE_NAME, unverifiable_payload(TEST_STATE_NAME));

    let state = sandbox.manager.load(TEST_STATE_NAME).unwrap();

    assert_eq!(state.manager.last_saved_position, 512);
}

#[rstest]
fn load_ignores_unknown_fields(#[with(false)] sandbox: Sandbox) {
    let payload = unverifiable_payload(TEST_STATE_NAME)
        .replace(r#""_checksum""#, r#""foo": 42, "_checksum""#);

    sandbox.write_state(TEST_STATE_NAME, &payload);

    assert_eq!(sandbox.manager.load(TEST_STATE_NAME).unwrap().position, 7);
}

#[rstest]
fn from_config_starts_with_no_saved_position(sandbox: Sandbox) {
    assert_eq!(sandbox.manager.last_saved_position, 0);
}

#[rstest]
fn from_config_carries_the_state_dir(sandbox: Sandbox) {
    assert_eq!(sandbox.manager.config.state_dir, sandbox.state_dir());
}

#[rstest]
#[case::verifying(true)]
#[case::lenient(false)]
fn from_config_carries_the_verify_flag(
    #[case] verify_state: bool,
    #[with(verify_state)] sandbox: Sandbox,
) {
    assert_eq!(sandbox.manager.config.verify_state, verify_state);
}

#[rstest]
fn default_uses_the_crate_defaults() {
    let manager = StateManager::default();

    assert_eq!(manager.last_saved_position, 0);
    assert_eq!(manager.config.state_dir, default_state_dir());
    assert_eq!(manager.config.verify_state, DEFAULT_VERIFY_STATE);
}

#[rstest]
fn clone_does_not_share_state(sandbox: Sandbox) {
    let mut clone = sandbox.manager.clone();

    clone.last_saved_position = 99;
    clone.config.verify_state = false;

    assert_eq!(sandbox.manager.last_saved_position, 0);
    assert!(sandbox.manager.config.verify_state);
}
