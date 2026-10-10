#[cfg(unix)]
use std::{ffi::OsStr, os::unix::ffi::OsStrExt};
use std::{fs, io::ErrorKind, path::PathBuf, time::Duration};

use chrono::{DateTime, Timelike, Utc};
use preader::{
    Config, Error, FINGERPRINT_SAMPLE_BYTES, IteratorBuild, IteratorRead, Mismatch, NameError,
    PReader, StateManager, TMP_FILE_EXTENSION, fingerprint,
};
use rstest::rstest;
use rstest_reuse::apply;

#[cfg(unix)]
use crate::common::{constants::TEST_NON_UTF8_NAME, guards::Blocked};
use crate::common::{
    constants::{
        TEST_FILE_PATH, TEST_LARGE_COPIES, TEST_LINE, TEST_STATE_NAME, TEST_TRACKED_NAME,
        TEST_WINDOW,
    },
    fixtures::sandbox,
    funcs::{canonical, consume, drain, names, state_data, state_file, tamper},
    guards::{mtime, set_mtime, set_pre_epoch_mtime},
    macros::asserts::assert_err_is,
    sandbox::Sandbox,
    templates::name::{longest_names, too_long_names},
};

const PAST_WINDOW: usize = TEST_WINDOW + 404;

#[rstest]
fn fields_describe_the_tracked_file(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let state = sandbox.state(&path);

    assert_eq!(state.name, TEST_STATE_NAME);
    assert_eq!(state.position, 0);
    assert_eq!(state.file.path, canonical(&path));
    assert_eq!(state.file.size, TEST_LINE.len() as u64);
    assert_eq!(state.timestamps.created_at, state.timestamps.updated_at);
}

#[rstest]
fn eq_compares_the_data() {
    let one = StateManager::default().state(state_data(PathBuf::from(TEST_FILE_PATH)));
    let mut changed = state_data(PathBuf::from(TEST_FILE_PATH));

    changed.position += 1;

    assert_eq!(
        one,
        StateManager::default().state(state_data(PathBuf::from(TEST_FILE_PATH)))
    );

    assert_ne!(one, StateManager::default().state(changed));
}

#[rstest]
fn eq_ignores_the_manager() {
    let elsewhere = Config {
        state_dir: PathBuf::from("/tmp/preader-elsewhere"),
        ..Config::default()
    };
    let one = StateManager::from(&elsewhere).state(state_data(PathBuf::from(TEST_FILE_PATH)));
    let other = StateManager::default().state(state_data(PathBuf::from(TEST_FILE_PATH)));

    assert_eq!(one, other);

    assert_ne!(one.path().unwrap(), other.path().unwrap());
}

#[rstest]
fn checksum_is_a_stable_hex_digest(sandbox: Sandbox) {
    let state = sandbox.state(&sandbox.line_file());
    let digest = state.checksum().unwrap();

    assert!(
        digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    );

    assert_eq!(digest.len(), 64);
    assert_eq!(state.checksum().unwrap(), digest);
}

#[rstest]
fn name_keeps_a_state_suffix(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let name = state_file(TEST_STATE_NAME);
    let state = sandbox.named_state(&path, &name);

    assert_eq!(state.name, name);
    assert_eq!(
        state.path().unwrap().file_name().unwrap(),
        state_file(&name).as_str()
    );
}

#[rstest]
fn path_joins_the_state_dir(sandbox: Sandbox) {
    let state = sandbox.state(&sandbox.line_file());

    assert_eq!(
        state.path().unwrap(),
        sandbox.state_dir().join(state_file(TEST_STATE_NAME))
    );
}

#[rstest]
fn path_is_relative_without_a_state_dir(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let reader = PReader::from(Config {
        state_dir: PathBuf::new(),
        ..Config::default()
    });
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    assert_eq!(
        bytes.state().path().unwrap(),
        PathBuf::from(state_file(TEST_STATE_NAME))
    );
}

#[rstest]
#[case::position_past_the_size("\"position\": 3", "\"position\": 999", 33300.0)]
#[case::zero_size("\"size\": 3", "\"size\": 0", 300.0)]
fn percent_reads_the_stored_numbers(
    sandbox: Sandbox,
    #[case] from: &str,
    #[case] to: &str,
    #[case] expected: f64,
) {
    let reader = sandbox.lenient();
    let path = sandbox.file(b"foo");
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    drain(&mut bytes);

    let payload = bytes.state().save().unwrap();

    drop(bytes);

    let patched = fs::read_to_string(&payload).unwrap().replace(from, to);

    assert!(patched.contains(to));
    fs::write(&payload, patched).unwrap();

    assert_eq!(
        reader.states().load(TEST_STATE_NAME).unwrap().percent(),
        expected
    );
}

#[rstest]
fn percent_tracks_the_position(sandbox: Sandbox) {
    let path = sandbox.file(&b"a".repeat(4));
    let mut bytes = sandbox.reader().bytes(&path).build().unwrap();

    assert_eq!(bytes.state().percent(), 0.0);

    bytes.read().unwrap();
    bytes.read().unwrap();

    assert_eq!(bytes.state().percent(), 50.0);

    drain(&mut bytes);

    assert_eq!(bytes.state().percent(), 100.0);
}

#[rstest]
fn verify_passes_when_the_file_is_untouched(sandbox: Sandbox) {
    let path = sandbox.large_file();

    sandbox.stored(&path, TEST_STATE_NAME).verify().unwrap();
}

#[rstest]
fn verify_fails_when_the_payload_is_tampered(sandbox: Sandbox) {
    let state = sandbox.stored(&sandbox.large_file(), TEST_STATE_NAME);

    tamper(&state);

    let loaded = sandbox.lenient().states().load(TEST_STATE_NAME).unwrap();

    assert_err_is!(loaded.verify(), Error::Mismatch(Mismatch::Checksum));
}

#[rstest]
fn verify_reports_the_checksum_first(sandbox: Sandbox) {
    let path = sandbox.large_file();
    let state = sandbox.stored(&path, TEST_STATE_NAME);

    tamper(&state);

    let loaded = sandbox.lenient().states().load(TEST_STATE_NAME).unwrap();

    fs::remove_file(&path).unwrap();

    assert_err_is!(loaded.verify(), Error::Mismatch(Mismatch::Checksum));
}

#[rstest]
fn verify_fails_when_the_size_changed(sandbox: Sandbox) {
    let path = sandbox.large_file();
    let state = sandbox.stored(&path, TEST_STATE_NAME);
    let stamp = mtime(&path);

    sandbox.truncate(&path, 4500);
    set_mtime(&path, stamp);

    assert_err_is!(state.verify(), Error::Mismatch(Mismatch::Size));
}

#[rstest]
fn verify_fails_when_the_mtime_changed(sandbox: Sandbox) {
    let path = sandbox.large_file();
    let state = sandbox.stored(&path, TEST_STATE_NAME);

    set_mtime(&path, mtime(&path) + Duration::from_secs(3600));

    assert_err_is!(state.verify(), Error::Mismatch(Mismatch::Mtime));
}

#[rstest]
fn verify_fails_when_an_mtime_before_the_epoch_changed(sandbox: Sandbox) {
    let path = sandbox.large_file();

    if !set_pre_epoch_mtime(&path, Duration::from_secs(86_400)) {
        return;
    }

    let state = sandbox.stored(&path, TEST_STATE_NAME);

    set_pre_epoch_mtime(&path, Duration::from_secs(172_800));

    assert_err_is!(state.verify(), Error::Mismatch(Mismatch::Mtime));
}

#[rstest]
fn verify_fails_when_the_fingerprint_window_changed(sandbox: Sandbox) {
    let path = sandbox.large_file();
    let state = sandbox.stored(&path, TEST_STATE_NAME);
    let stamp = mtime(&path);

    sandbox.overwrite(&path, 10, b"\xff");
    set_mtime(&path, stamp);

    assert_err_is!(state.verify(), Error::Mismatch(Mismatch::Fingerprint));
}

#[rstest]
fn verify_ignores_a_change_past_the_fingerprint_window(sandbox: Sandbox) {
    let path = sandbox.large_file();
    let state = sandbox.stored(&path, TEST_STATE_NAME);
    let stamp = mtime(&path);

    sandbox.overwrite(&path, PAST_WINDOW as u64, b"\xff");
    set_mtime(&path, stamp);

    assert!(state.verify().is_ok());
}

#[rstest]
fn verify_fails_when_the_file_is_deleted(sandbox: Sandbox) {
    let path = sandbox.large_file();
    let state = sandbox.stored(&path, TEST_STATE_NAME);

    fs::remove_file(&path).unwrap();

    let error = state.verify().unwrap_err();

    assert!(matches!(error, Error::Io(_)), "{error}");
    assert!(!error.to_string().contains("mismatch"), "{error}");
}

#[rstest]
fn verify_fails_when_the_path_is_a_directory(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let state = sandbox.state(&path);

    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();

    assert_err_is!(state.verify(), Error::Io(error) if error.kind() == ErrorKind::IsADirectory);
}

#[rstest]
fn verify_fails_when_the_file_grew(sandbox: Sandbox) {
    let path = sandbox.large_file();
    let state = sandbox.stored(&path, TEST_STATE_NAME);

    sandbox.append(&path, b"more");

    assert_err_is!(state.verify(), Error::Mismatch(Mismatch::Size));
}

#[cfg(unix)]
#[rstest]
fn verify_fails_when_the_path_is_not_utf8(sandbox: Sandbox) {
    let path = PathBuf::from(OsStr::from_bytes(TEST_NON_UTF8_NAME));
    let state = StateManager::from(&sandbox.config()).state(state_data(path));

    assert_err_is!(state.verify(), Error::Serde(_));
}

#[rstest]
fn resync_updates_the_file_metadata(sandbox: Sandbox) {
    let state = sandbox.state(&sandbox.line_file());
    let moved = sandbox.write("moved.bin", b"foo\nbar");
    let resynced = state.resync(&moved).unwrap();

    assert_eq!(resynced.file.path, canonical(&moved));
    assert_eq!(resynced.file.size, 7);
    assert_eq!(
        resynced.file.fingerprint,
        fingerprint(&moved, FINGERPRINT_SAMPLE_BYTES).unwrap()
    );
}

#[rstest]
fn resync_keeps_name_position_and_created_at(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let mut bytes = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    bytes.read().unwrap();

    let state = bytes.state().clone();
    let moved = sandbox.write("moved.bin", TEST_LINE);
    let resynced = state.resync(&moved).unwrap();

    assert!(resynced.timestamps.updated_at > state.timestamps.updated_at);

    assert_eq!(resynced.name, state.name);
    assert_eq!(resynced.position, state.position);
    assert_eq!(resynced.timestamps.created_at, state.timestamps.created_at);
}

#[rstest]
fn resync_does_not_mutate_the_original(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let state = sandbox.state(&path);
    let moved = sandbox.write("moved.bin", TEST_LINE);

    state.resync(&moved).unwrap();

    assert_eq!(state.file.path, canonical(&path));
}

#[rstest]
#[case::truncated_inside_the_window(TEST_LINE, 25, TEST_LINE, 2)]
#[case::truncated_inside_a_wide_window(TEST_LINE, TEST_LARGE_COPIES, TEST_LINE, 250)]
#[case::truncated_with_a_changed_prefix(TEST_LINE, TEST_LARGE_COPIES, b"bar\n", 1250)]
#[case::grown_with_a_changed_prefix(TEST_LINE, 25, b"bar\n", 27)]
#[case::replaced_at_the_same_size(TEST_LINE, 25, b"bar\n", 25)]
#[case::emptied(TEST_LINE, 25, b"", 0)]
#[case::truncated_at_the_window(b"a", TEST_WINDOW, b"a", TEST_WINDOW - 1)]
fn resync_fails_when_the_prefix_is_gone(
    sandbox: Sandbox,
    #[case] before: &[u8],
    #[case] copies: usize,
    #[case] after: &[u8],
    #[case] remaining: usize,
) {
    let saved = sandbox.recorded(&before.repeat(copies), 5);
    let path = sandbox.write(TEST_TRACKED_NAME, &after.repeat(remaining));

    assert_err_is!(saved.resync(&path), Error::Mismatch(Mismatch::Identity));
}

#[rstest]
#[case::appended(25, b"more")]
#[case::unchanged(25, b"")]
#[case::grown_past_a_wide_window(TEST_LARGE_COPIES, b"more")]
fn resync_accepts_a_kept_prefix(sandbox: Sandbox, #[case] copies: usize, #[case] extra: &[u8]) {
    let before = TEST_LINE.repeat(copies);
    let after = [&before[..], extra].concat();
    let saved = sandbox.recorded(&before, 5);
    let path = sandbox.write(TEST_TRACKED_NAME, &after);
    let resynced = saved.resync(&path).unwrap();

    assert_eq!(resynced.position, 5);
    assert_eq!(resynced.file.size, after.len() as u64);
    resynced.verify().unwrap();
}

#[rstest]
fn resync_clamps_the_position_to_the_new_size(sandbox: Sandbox) {
    let content = TEST_LINE.repeat(TEST_LARGE_COPIES);
    let saved = sandbox.recorded(&content, 6000);
    let path = sandbox.write(TEST_TRACKED_NAME, &content);

    sandbox.truncate(&path, 4500);

    let resynced = saved.resync(&path).unwrap();

    assert_eq!(saved.position, 6000);
    assert_eq!(resynced.position, 4500);
    assert_eq!(resynced.percent(), 100.0);

    resynced.verify().unwrap();
}

#[rstest]
fn resync_ignores_a_change_past_the_fingerprint_window(sandbox: Sandbox) {
    let mut after = TEST_LINE.repeat(TEST_LARGE_COPIES);

    after[PAST_WINDOW] = b'\xff';

    let saved = sandbox.recorded(&TEST_LINE.repeat(TEST_LARGE_COPIES), 6000);
    let path = sandbox.write(TEST_TRACKED_NAME, &after);
    let resynced = saved.resync(&path).unwrap();

    assert_eq!(resynced.position, 6000);

    resynced.verify().unwrap();
}

#[rstest]
fn resync_accepts_anything_for_an_empty_file(sandbox: Sandbox) {
    let saved = sandbox.recorded(b"", 0);
    let path = sandbox.write(TEST_TRACKED_NAME, b"foo");
    let resynced = saved.resync(&path).unwrap();

    assert_eq!(resynced.position, 0);
    assert_eq!(resynced.file.size, 3);
}

#[rstest]
#[case::at_the_window(TEST_WINDOW, TEST_WINDOW, 100, 100, 100.0 * 100.0 / TEST_WINDOW as f64)]
#[case::past_the_window(TEST_WINDOW + 1, TEST_WINDOW, TEST_WINDOW as u64 + 1, TEST_WINDOW as u64, 100.0)]
fn resync_accepts_at_the_fingerprint_window(
    sandbox: Sandbox,
    #[case] before: usize,
    #[case] after: usize,
    #[case] read: u64,
    #[case] position: u64,
    #[case] percent: f64,
) {
    let saved = sandbox.recorded(&b"a".repeat(before), read);
    let path = sandbox.write(TEST_TRACKED_NAME, &b"a".repeat(after));
    let resynced = saved.resync(&path).unwrap();

    assert_eq!(resynced.position, position);
    assert_eq!(resynced.percent(), percent);
}

#[rstest]
#[case::replaced(b"bar\n", 25, 50.0)]
#[case::shrunk(TEST_LINE, 2, 625.0)]
#[case::emptied(b"", 0, 5000.0)]
fn resync_trusts_the_path_without_verification(
    sandbox: Sandbox,
    #[case] unit: &[u8],
    #[case] copies: usize,
    #[case] percent: f64,
) {
    let path = sandbox.write(TEST_TRACKED_NAME, &TEST_LINE.repeat(25));
    let mut bytes =
        sandbox.lenient().bytes(&path).state(TEST_STATE_NAME).limit(50).build().unwrap();

    drain(&mut bytes);

    let saved = bytes.state().clone();

    drop(bytes);

    sandbox.write(TEST_TRACKED_NAME, &unit.repeat(copies));

    let resynced = saved.resync(&path).unwrap();

    assert_eq!(resynced.position, 50);
    assert_eq!(resynced.file.size, (unit.len() * copies) as u64);
    assert_eq!(resynced.percent(), percent);
}

#[rstest]
#[case::missing("missing.bin", false)]
#[case::a_directory("elsewhere", true)]
fn resync_fails_when_the_path_is_not_a_file_without_verification(
    sandbox: Sandbox,
    #[case] name: &str,
    #[case] directory: bool,
) {
    let path = sandbox.write(TEST_TRACKED_NAME, &TEST_LINE.repeat(25));
    let mut iterator = sandbox.lenient().bytes(&path).state(TEST_STATE_NAME).build().unwrap();
    let saved = iterator.state().clone();
    let target = if directory {
        sandbox.dir_at(name)
    } else {
        sandbox.path().join(name)
    };

    assert_err_is!(saved.resync(&target), Error::Io(_));
}

#[rstest]
fn resync_keeps_a_position_that_equals_the_size(sandbox: Sandbox) {
    let content = TEST_LINE.repeat(25);
    let saved = sandbox.recorded(&content, 100);
    let path = sandbox.write(TEST_TRACKED_NAME, &content);
    let resynced = saved.resync(&path).unwrap();

    assert_eq!(resynced.position, 100);
    assert_eq!(resynced.file.size, 100);
}

#[rstest]
fn resync_is_stable_when_repeated(sandbox: Sandbox) {
    let content = TEST_LINE.repeat(25);
    let saved = sandbox.recorded(&content, 5);
    let path = sandbox.write(TEST_TRACKED_NAME, &content);
    let once = saved.resync(&path).unwrap();
    let twice = once.resync(&path).unwrap();

    assert_eq!(twice.position, 5);
    assert_eq!(twice.file, once.file);
    twice.verify().unwrap();
}

#[rstest]
fn resync_moves_the_identity_forward(sandbox: Sandbox) {
    let saved = sandbox.recorded(&TEST_LINE.repeat(25), 5);
    let original = sandbox.path().join(TEST_TRACKED_NAME);
    let grown = sandbox.write("grown.bin", &TEST_LINE.repeat(30));
    let once = saved.resync(&grown).unwrap();

    assert_eq!(once.file.size, 120);

    assert_err_is!(once.resync(&original), Error::Mismatch(Mismatch::Identity));
}

#[rstest]
fn resync_reseals_a_tampered_state(sandbox: Sandbox) {
    let path = sandbox.file(&b"a".repeat(20));
    let loaded = sandbox.stored(&path, TEST_STATE_NAME);

    tamper(&loaded);

    let reloaded = sandbox.lenient().states().load(TEST_STATE_NAME).unwrap();

    assert_err_is!(reloaded.verify(), Error::Mismatch(Mismatch::Checksum));

    let resynced = reloaded.resync(&path).unwrap();

    resynced.verify().unwrap();

    assert_eq!(resynced.position, 999);
}

#[rstest]
fn resync_keeps_an_unsafe_name_for_the_save(sandbox: Sandbox) {
    let path = sandbox.write(TEST_TRACKED_NAME, &TEST_LINE.repeat(25));
    let mut data = state_data(canonical(&path));

    data.name = "../../etc/passwd".to_owned();

    let saved = sandbox.manager().state(data);
    let mut resynced = saved.resync(&path).unwrap();

    assert_eq!(resynced.name, "../../etc/passwd");

    assert_err_is!(resynced.save(), Error::Name(NameError::Escapes));
}

#[rstest]
fn resync_keeps_the_state_dir_for_the_save(sandbox: Sandbox) {
    let path = sandbox.write(TEST_TRACKED_NAME, &TEST_LINE.repeat(25));
    let saved = sandbox.state(&path);
    let written = saved.resync(&path).unwrap().save().unwrap();

    assert!(written.starts_with(sandbox.state_dir()));
    assert!(written.is_file());

    assert_eq!(
        sandbox.states().load(TEST_STATE_NAME).unwrap().name,
        TEST_STATE_NAME
    );
}

#[rstest]
fn resync_accepts_an_mtime_before_the_epoch(sandbox: Sandbox) {
    let content = TEST_LINE.repeat(25);
    let saved = sandbox.recorded(&content, 5);
    let path = sandbox.path().join(TEST_TRACKED_NAME);

    if !set_pre_epoch_mtime(&path, Duration::from_secs(86_400)) {
        return;
    }

    let resynced = saved.resync(&path).unwrap();

    assert_eq!(resynced.position, 5);
    assert_eq!(resynced.file.mtime.timestamp(), -86_400);
}

#[rstest]
fn resync_accepts_every_path_shape(sandbox: Sandbox) {
    let content = TEST_LINE.repeat(25);
    let saved = sandbox.recorded(&content, 5);
    let path = sandbox.path().join(TEST_TRACKED_NAME);
    let text = path.to_str().unwrap().to_owned();
    let resolved = canonical(&path);

    for resynced in [
        saved.resync(text.as_str()).unwrap(),
        saved.resync(&text).unwrap(),
        saved.resync(text.clone()).unwrap(),
        saved.resync(path.as_path()).unwrap(),
        saved.resync(&path).unwrap(),
        saved.resync(path.clone()).unwrap(),
    ] {
        assert_eq!(resynced.file.path, resolved);
    }
}

#[rstest]
fn build_fails_when_a_resynced_state_tracks_another_file(sandbox: Sandbox) {
    let first = sandbox.write("a.bin", &b"a".repeat(10));
    let second = sandbox.write("b.bin", &b"b".repeat(10));
    let resynced = sandbox.state(&first).resync(&first).unwrap();

    assert_err_is!(
        sandbox.reader().bytes(&second).state(resynced).build(),
        Error::Mismatch(Mismatch::Path)
    );
}

#[rstest]
#[case::directly(false)]
#[case::through_a_symlink(true)]
#[cfg(unix)]
fn resync_fails_when_the_path_is_a_directory(sandbox: Sandbox, #[case] linked: bool) {
    use std::os::unix::fs::symlink;

    let state = sandbox.state(&sandbox.line_file());
    let directory = sandbox.dir_at("elsewhere");
    let target = if linked {
        let alias = sandbox.path().join("alias");

        symlink(&directory, &alias).unwrap();

        alias
    } else {
        directory
    };

    assert_err_is!(state.resync(&target), Error::Io(error) if error.kind() == ErrorKind::IsADirectory);
}

#[rstest]
#[case::missing(false)]
#[case::a_symlink_loop(true)]
#[cfg(unix)]
fn resync_fails_when_the_path_cannot_be_resolved(sandbox: Sandbox, #[case] looped: bool) {
    use std::os::unix::fs::symlink;

    let state = sandbox.state(&sandbox.line_file());
    let target = if looped {
        let (first, second) = (sandbox.path().join("a-link"), sandbox.path().join("b-link"));

        symlink(&second, &first).unwrap();
        symlink(&first, &second).unwrap();

        first
    } else {
        sandbox.path().join("missing.bin")
    };

    assert_err_is!(state.resync(&target), Error::Io(_));
}

#[rstest]
#[case::a_symlink(true)]
#[case::an_unnormalized_path(false)]
#[cfg(unix)]
fn resync_canonicalizes_the_path(sandbox: Sandbox, #[case] linked: bool) {
    use std::os::unix::fs::symlink;

    let content = TEST_LINE.repeat(25);
    let saved = sandbox.recorded(&content, 5);
    let target = sandbox.path().join(TEST_TRACKED_NAME);
    let detour = if linked {
        let alias = sandbox.path().join("alias.bin");

        symlink(&target, &alias).unwrap();

        alias
    } else {
        sandbox.path().join(".").join(TEST_TRACKED_NAME)
    };

    assert_eq!(saved.resync(&detour).unwrap().file.path, canonical(&target));
}

#[cfg(unix)]
#[rstest]
fn resync_fails_when_the_path_is_not_utf8(sandbox: Sandbox) {
    let content = TEST_LINE.repeat(25);
    let saved = sandbox.recorded(&content, 5);
    let odd = sandbox.path().join(OsStr::from_bytes(TEST_NON_UTF8_NAME));

    if fs::write(&odd, &content).is_err() {
        return;
    }

    assert_err_is!(saved.resync(&odd), Error::Serde(_));
}

#[cfg(unix)]
#[rstest]
fn resync_fails_when_the_file_is_unreadable(sandbox: Sandbox) {
    let content = TEST_LINE.repeat(25);
    let saved = sandbox.recorded(&content, 5);
    let path = sandbox.path().join(TEST_TRACKED_NAME);
    let mut blocked = Blocked::default();

    if !Blocked::enforced(&sandbox.path().join("probe")) {
        return;
    }

    blocked.block(&path);

    assert_err_is!(
        saved.resync(&path),
        Error::Io(io) if io.kind() == ErrorKind::PermissionDenied
    );
}

#[rstest]
fn save_returns_the_created_path(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let mut state = sandbox.state(&path);

    assert!(!state.path().unwrap().exists());

    let written = state.save().unwrap();

    assert!(written.is_file());

    assert_eq!(written, state.path().unwrap());
}

#[rstest]
fn save_updates_only_the_updated_at(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let mut state = sandbox.state(&path);
    let created_at = state.timestamps.created_at;

    state.save().unwrap();
    state.save().unwrap();

    assert!(state.timestamps.updated_at > created_at);

    assert_eq!(state.timestamps.created_at, created_at);
}

#[rstest]
fn save_persists_across_readers(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let mut bytes = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    drain(&mut bytes);

    bytes.state().save().unwrap();

    drop(bytes);

    assert_eq!(
        sandbox.reader().states().load(TEST_STATE_NAME).unwrap().position,
        TEST_LINE.len() as u64
    );
}

#[rstest]
fn save_refreshes_the_checksum(sandbox: Sandbox) {
    let path = sandbox.large_file();
    let mut bytes = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).build().unwrap();
    let before = bytes.state().checksum().unwrap();

    consume(&mut bytes, 5);

    bytes.state().save().unwrap();

    drop(bytes);

    assert_ne!(
        sandbox.states().load(TEST_STATE_NAME).unwrap().checksum,
        before
    );
}

#[rstest]
fn save_does_not_disturb_the_iterator(sandbox: Sandbox) {
    let path = sandbox.large_file();
    let mut bytes = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    consume(&mut bytes, 5);

    bytes.state().save().unwrap();

    assert_eq!(sandbox.states().load(TEST_STATE_NAME).unwrap().position, 5);

    let mut remaining = 0;

    while bytes.read().unwrap().is_some() {
        remaining += 1;
    }

    assert_eq!(remaining, TEST_LARGE_COPIES * TEST_LINE.len() - 5);
}

#[rstest]
fn save_succeeds_after_the_file_is_deleted(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let mut state = sandbox.state(&path);

    fs::remove_file(&path).unwrap();

    assert!(state.save().unwrap().is_file());
}

#[rstest]
fn save_fails_when_the_state_dir_is_a_file(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let mut state = sandbox.state(&path);

    sandbox.block_states();

    assert_err_is!(state.save(), Error::Io(_));
}

#[rstest]
fn save_fails_when_the_state_path_is_a_directory(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let mut state = sandbox.state(&path);

    fs::create_dir_all(state.path().unwrap()).unwrap();

    assert_err_is!(state.save(), Error::Io(_));
}

#[rstest]
fn build_fails_when_the_name_has_a_nul_byte(sandbox: Sandbox) {
    let path = sandbox.line_file();

    assert_err_is!(
        sandbox.reader().bytes(&path).state("job\0").build(),
        Error::Name(NameError::Invalid)
    );
}

#[apply(too_long_names)]
fn build_fails_when_the_name_is_too_long(sandbox: Sandbox, #[case] name: String) {
    let path = sandbox.line_file();

    assert_err_is!(
        sandbox.reader().bytes(&path).state(name.as_str()).build(),
        Error::Name(NameError::Invalid)
    );
}

#[apply(longest_names)]
fn save_round_trips_the_longest_portable_name(sandbox: Sandbox, #[case] name: String) {
    let path = sandbox.line_file();
    let mut state = sandbox.named_state(&path, &name);

    state.save().unwrap();

    assert_eq!(names(&sandbox.states()), [name.as_str()]);
    assert_eq!(sandbox.states().load(&name).unwrap().name, name);
}

#[cfg(unix)]
#[rstest]
fn save_fails_when_the_path_is_not_utf8(sandbox: Sandbox) {
    let path = PathBuf::from(OsStr::from_bytes(TEST_NON_UTF8_NAME));
    let mut state = StateManager::from(&sandbox.config()).state(state_data(path));

    assert_err_is!(state.save(), Error::Serde(_));
}

#[rstest]
#[case::path_rejected("../../etc/passwd")]
#[case::commit_failed(TEST_STATE_NAME)]
fn save_leaves_the_state_untouched_when_it_fails(sandbox: Sandbox, #[case] name: &str) {
    let mut data = state_data(sandbox.line_file());

    data.name = name.to_owned();

    let mut state = sandbox.manager().state(data);
    let before = state.timestamps.updated_at;

    sandbox.block_states();

    assert_err_is!(state.save(), Error::Io(_) | Error::Name(_));

    assert_eq!(state.timestamps.updated_at, before);
}

#[rstest]
#[case::when_it_succeeds(false)]
#[case::when_it_fails(true)]
fn save_leaves_no_temporary_file(sandbox: Sandbox, #[case] blocked: bool) {
    let path = sandbox.line_file();
    let mut state = sandbox.state(&path);

    if blocked {
        fs::create_dir_all(state.path().unwrap()).unwrap();
    }

    assert_eq!(state.save().is_err(), blocked);

    let leftovers: Vec<PathBuf> = fs::read_dir(sandbox.state_dir())
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == TMP_FILE_EXTENSION))
        .collect();

    assert!(leftovers.is_empty(), "{leftovers:?}");
}

#[rstest]
fn load_fails_when_the_file_is_deleted(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let mut state = sandbox.state(&path);

    fs::remove_file(&path).unwrap();
    state.save().unwrap();

    assert_err_is!(
        sandbox.states().load(TEST_STATE_NAME),
        Error::Io(io) if io.kind() == ErrorKind::NotFound
    );
}

#[rstest]
fn load_restores_the_saved_state_to_the_second(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let mut bytes = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    bytes.read().unwrap();
    bytes.state().save().unwrap();

    let saved = bytes.state().clone();

    drop(bytes);

    let reloaded = sandbox.states().load(TEST_STATE_NAME).unwrap();
    let second = |stamp: DateTime<Utc>| stamp.with_nanosecond(0).unwrap();

    assert_eq!(reloaded.name, saved.name);
    assert_eq!(reloaded.position, saved.position);
    assert_eq!(reloaded.file, saved.file);
    assert_eq!(reloaded.checksum, saved.checksum);
    assert_eq!(
        reloaded.timestamps.created_at,
        second(saved.timestamps.created_at)
    );
    assert_eq!(
        reloaded.timestamps.updated_at,
        second(saved.timestamps.updated_at)
    );
}

#[rstest]
fn payload_carries_every_field(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let mut state = sandbox.state(&path);

    state.save().unwrap();

    let payload = sandbox.payload(&state);
    let keys = |value: &serde_json::Value| {
        let mut keys: Vec<String> = value.as_object().unwrap().keys().cloned().collect();

        keys.sort();
        keys
    };

    assert_eq!(
        keys(&payload),
        ["_checksum", "file", "name", "position", "timestamps"]
    );
    assert_eq!(
        keys(&payload["file"]),
        ["fingerprint", "mtime", "path", "size"]
    );
    assert_eq!(keys(&payload["timestamps"]), ["created_at", "updated_at"]);
}

#[rstest]
fn payload_keeps_the_checksum_last(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let mut state = sandbox.state(&path);
    let text = fs::read_to_string(state.save().unwrap()).unwrap();
    let at = |key: &str| text.find(key).unwrap();

    for key in ["\"name\"", "\"file\"", "\"position\"", "\"timestamps\""] {
        assert!(at("\"_checksum\"") > at(key), "{key}");
    }
}

#[rstest]
fn path_uses_the_native_separator(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let state = sandbox.named_state(&path, "sub-1/sub-2/job-1");
    let expected = sandbox.state_dir().join("sub-1").join("sub-2").join(state_file("job-1"));

    assert_eq!(state.path().unwrap().as_os_str(), expected.as_os_str());
}

#[rstest]
fn save_replaces_a_hard_link_without_touching_its_target(sandbox: Sandbox) {
    let outside = sandbox.write("outside.txt", b"foo");
    let link = sandbox.states().path(TEST_STATE_NAME).unwrap();

    fs::create_dir_all(sandbox.state_dir()).unwrap();
    fs::hard_link(&outside, &link).unwrap();

    sandbox.save(TEST_STATE_NAME);

    assert_eq!(fs::read(&outside).unwrap(), b"foo");

    assert_ne!(fs::read(&link).unwrap(), b"foo");
}

#[rstest]
fn save_fails_when_a_state_file_blocks_the_directory(sandbox: Sandbox) {
    let saved = sandbox.save(TEST_STATE_NAME).path().unwrap();
    let before = fs::read(&saved).unwrap();
    let path = sandbox.line_file();
    let mut state = sandbox.named_state(&path, "job-1.state.json/job-2");
    let outcome = state.save();

    assert_eq!(fs::read(&saved).unwrap(), before);

    assert_err_is!(outcome, Error::Io(_));
}

#[rstest]
fn save_fails_when_a_directory_blocks_the_state_file(sandbox: Sandbox) {
    let nested = sandbox.save("job-1.state.json/job-2").path().unwrap();
    let path = sandbox.line_file();
    let mut state = sandbox.named_state(&path, TEST_STATE_NAME);
    let outcome = state.save();

    assert!(nested.exists());

    assert_err_is!(outcome, Error::Io(_));
}
