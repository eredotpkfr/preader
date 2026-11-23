#[cfg(unix)]
use std::{
    ffi::OsStr,
    os::unix::{ffi::OsStrExt, fs::symlink},
};
use std::{
    fs,
    io::ErrorKind,
    path::PathBuf,
    time::{Duration, UNIX_EPOCH},
};

use preader::{Error, FileMetadata, Mismatch, NameError, State, Timestamps, file};
use rstest::{fixture, rstest};
use rstest_reuse::apply;
use serde_json::Value;
use time::UtcDateTime;

#[cfg(unix)]
use crate::common::{constants::TEST_NON_UTF8_NAME, guards::Blocked};
use crate::common::{
    constants::{
        TEST_EMPTY_FINGERPRINT, TEST_FILE_PATH, TEST_LARGE_COPIES, TEST_LINE,
        TEST_LINE_FINGERPRINT, TEST_OTHER_STATE_NAME, TEST_PRE_EPOCH, TEST_STAMP, TEST_STATE_NAME,
        TEST_TRACKED_COPIES, TEST_TRACKED_NAME, TEST_WINDOW,
    },
    fixtures::sandbox,
    funcs::{canonical, consume, digest, drain, items, state_data},
    guards::{mtime, set_mtime, set_pre_epoch_mtime},
    macros::asserts::assert_err_is,
    sandbox::Sandbox,
    templates::content::lost_prefixes,
};

const PAST_WINDOW: usize = TEST_WINDOW + 404;
const DIGEST: &str = "0881b148c8b6071356b5cb714b11e46cd2aecc19a316b6498588a736812745dd";
const CHECKSUM_KEY: &str = "_checksum";

#[fixture]
fn digested() -> State {
    State {
        name: TEST_STATE_NAME.to_owned(),
        file: FileMetadata {
            path: PathBuf::from(TEST_FILE_PATH),
            size: TEST_LINE.len() as u64,
            mtime: UtcDateTime::from_unix_timestamp(TEST_STAMP).unwrap(),
            fingerprint: TEST_LINE_FINGERPRINT.to_owned(),
        },
        position: 7,
        timestamps: Timestamps {
            created_at: UtcDateTime::from_unix_timestamp(TEST_STAMP + 1).unwrap(),
            updated_at: UtcDateTime::from_unix_timestamp(TEST_STAMP + 2).unwrap(),
        },
        checksum: DIGEST.to_owned(),
    }
}

fn leaves(value: &Value, pointer: String) -> Vec<String> {
    match value.as_object() {
        Some(fields) => fields
            .iter()
            .flat_map(|(key, field)| leaves(field, format!("{pointer}/{key}")))
            .collect(),
        None => vec![pointer],
    }
}

fn bump(leaf: &mut Value, pointer: &str) {
    *leaf = match leaf {
        Value::Number(number) => (number.as_i64().unwrap() + 1).into(),
        Value::String(text) => format!("{text}-changed").into(),
        other => panic!("unexpected leaf {other} at {pointer}"),
    };
}

#[rstest]
fn fields(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let state = sandbox.state(&path);

    assert_eq!(state.name, TEST_STATE_NAME);
    assert_eq!(state.position, 0);
    assert_eq!(state.file.path, canonical(&path));
    assert_eq!(state.file.size, TEST_LINE.len() as u64);
    assert_eq!(state.timestamps.created_at, state.timestamps.updated_at);
}

#[rstest]
fn compares_by_value() {
    let one = state_data(PathBuf::from(TEST_FILE_PATH));
    let mut changed = state_data(PathBuf::from(TEST_FILE_PATH));

    changed.position += 1;

    assert_eq!(one, state_data(PathBuf::from(TEST_FILE_PATH)));

    assert_ne!(one, changed);
}

#[rstest]
fn checksum_matches_a_known_digest(digested: State) {
    assert_eq!(digested.checksum().unwrap(), DIGEST);
}

#[rstest]
#[case::empty("")]
#[case::stale("not-a-real-checksum")]
fn checksum_ignores_the_stored_checksum(mut digested: State, #[case] checksum: &str) {
    digested.checksum = checksum.to_owned();

    assert_eq!(digested.checksum().unwrap(), DIGEST);
}

#[rstest]
#[case::name(|state: &mut State| state.name = TEST_OTHER_STATE_NAME.to_owned())]
#[case::position(|state: &mut State| state.position += 1)]
fn checksum_covers_every_top_level_field(mut digested: State, #[case] change: fn(&mut State)) {
    let before = digested.checksum().unwrap();

    change(&mut digested);

    assert_ne!(digested.checksum().unwrap(), before);
}

#[rstest]
#[case::path(|file: &mut FileMetadata| file.path = PathBuf::from("/tmp/other.bin"))]
#[case::size(|file: &mut FileMetadata| file.size += 1)]
#[case::mtime(|file: &mut FileMetadata| file.mtime = UtcDateTime::from_unix_timestamp(1).unwrap())]
#[case::fingerprint(|file: &mut FileMetadata| file.fingerprint = TEST_EMPTY_FINGERPRINT.to_owned())]
fn checksum_covers_every_file_field(mut digested: State, #[case] change: fn(&mut FileMetadata)) {
    let before = digested.checksum().unwrap();

    change(&mut digested.file);

    assert_ne!(digested.checksum().unwrap(), before);
}

#[rstest]
#[case::created_at(|stamps: &mut Timestamps| stamps.created_at = UtcDateTime::from_unix_timestamp(1).unwrap())]
#[case::updated_at(|stamps: &mut Timestamps| stamps.updated_at = UtcDateTime::from_unix_timestamp(1).unwrap())]
fn checksum_covers_every_timestamp(mut digested: State, #[case] change: fn(&mut Timestamps)) {
    let before = digested.checksum().unwrap();

    change(&mut digested.timestamps);

    assert_ne!(digested.checksum().unwrap(), before);
}

#[rstest]
fn verify_passes_when_the_file_is_untouched(sandbox: Sandbox) {
    let path = sandbox.large_file();

    sandbox.saved(&path, TEST_STATE_NAME);

    assert_eq!(
        sandbox.states().load(TEST_STATE_NAME).unwrap().name,
        TEST_STATE_NAME
    );
}

#[rstest]
fn checksum_ignores_sub_second_precision(mut digested: State) {
    digested.timestamps.updated_at = UtcDateTime::from_unix_timestamp_nanos(
        i128::from(TEST_STAMP + 2) * 1_000_000_000 + 500_000_000,
    )
    .unwrap();

    assert_eq!(digested.checksum().unwrap(), DIGEST);
}

#[cfg(unix)]
#[rstest]
fn checksum_fails_when_the_path_is_not_utf8(mut digested: State) {
    digested.file.path = PathBuf::from(OsStr::from_bytes(b"/tmp/data-\xff\xfe.bin"));

    assert_err_is!(digested.checksum(), Error::Serde(_));
}

#[apply(lost_prefixes)]
fn verify_fails_when_the_prefix_is_gone(
    sandbox: Sandbox,
    #[case] before: Vec<u8>,
    #[case] after: Vec<u8>,
) {
    sandbox.recorded(&before, 5);
    sandbox.write(TEST_TRACKED_NAME, &after);

    assert_err_is!(
        sandbox.states().load(TEST_STATE_NAME),
        Error::Mismatch {
            kind: Mismatch::Identity,
            ..
        }
    );
}

#[apply(lost_prefixes)]
fn resync_fails_when_the_prefix_is_gone(
    sandbox: Sandbox,
    #[case] before: Vec<u8>,
    #[case] after: Vec<u8>,
) {
    sandbox.recorded(&before, 5);

    let path = sandbox.write(TEST_TRACKED_NAME, &after);

    assert_err_is!(
        sandbox.states().resync(TEST_STATE_NAME, &path),
        Error::Mismatch {
            kind: Mismatch::Identity,
            ..
        }
    );
}

#[rstest]
fn checksum_hashes_the_compact_json_without_itself(digested: State) {
    let mut payload = serde_json::to_value(&digested).unwrap();

    payload.as_object_mut().unwrap().shift_remove(CHECKSUM_KEY);

    assert_eq!(
        digested.checksum().unwrap(),
        digest(payload.to_string().as_bytes())
    );
}

#[rstest]
fn verify_fails_when_the_file_is_deleted(sandbox: Sandbox) {
    let path = sandbox.large_file();

    sandbox.saved(&path, TEST_STATE_NAME);

    fs::remove_file(&path).unwrap();

    assert_err_is!(
        sandbox.states().load(TEST_STATE_NAME),
        Error::Mismatch {
            kind: Mismatch::Missing,
            ..
        }
    );
}

#[rstest]
fn verify_fails_when_the_file_grew(sandbox: Sandbox) {
    let path = sandbox.large_file();

    sandbox.saved(&path, TEST_STATE_NAME);
    sandbox.append(&path, b"more");

    assert_err_is!(
        sandbox.states().load(TEST_STATE_NAME),
        Error::Mismatch {
            kind: Mismatch::Size,
            ..
        }
    );
}

#[rstest]
fn resync_fails_when_the_path_is_a_directory(sandbox: Sandbox) {
    sandbox.saved(&sandbox.line_file(), TEST_STATE_NAME);

    let directory = sandbox.dir_at("elsewhere");

    assert_err_is!(
        sandbox.states().resync(TEST_STATE_NAME, &directory),
        Error::Io(error) if error.kind() == ErrorKind::IsADirectory
    );
}

#[cfg(unix)]
#[rstest]
fn bytes_fails_when_the_path_is_not_utf8(sandbox: Sandbox) {
    let path = sandbox.path().join(OsStr::from_bytes(TEST_NON_UTF8_NAME));

    if fs::write(&path, TEST_LINE).is_err() {
        return;
    }

    assert_err_is!(sandbox.reader().bytes(&path).build(), Error::Serde(_));
}

#[rstest]
fn resync_accepts_anything_for_an_empty_file(sandbox: Sandbox) {
    sandbox.recorded(b"", 0);

    let path = sandbox.write(TEST_TRACKED_NAME, b"foo");
    let resynced = sandbox.states().resync(TEST_STATE_NAME, &path).unwrap();

    assert_eq!(resynced.position, 0);
    assert_eq!(resynced.file.size, 3);
}

#[rstest]
fn verify_reports_the_checksum_first(sandbox: Sandbox) {
    let path = sandbox.large_file();

    sandbox.saved(&path, TEST_STATE_NAME);
    sandbox.tamper(TEST_STATE_NAME);

    fs::remove_file(&path).unwrap();

    assert_err_is!(
        sandbox.states().load(TEST_STATE_NAME),
        Error::Mismatch {
            kind: Mismatch::Checksum,
            ..
        }
    );
}

#[rstest]
fn verify_fails_when_the_mtime_changed(sandbox: Sandbox) {
    let path = sandbox.large_file();

    sandbox.saved(&path, TEST_STATE_NAME);

    set_mtime(&path, mtime(&path) + Duration::from_secs(3600));

    assert_err_is!(
        sandbox.states().load(TEST_STATE_NAME),
        Error::Mismatch {
            kind: Mismatch::Mtime,
            ..
        }
    );
}

#[rstest]
fn resync_fails_when_the_state_is_tampered(sandbox: Sandbox) {
    let path = sandbox.file(&b"a".repeat(20));

    sandbox.saved(&path, TEST_STATE_NAME);
    sandbox.tamper(TEST_STATE_NAME);

    assert_err_is!(
        sandbox.states().resync(TEST_STATE_NAME, &path),
        Error::Mismatch {
            kind: Mismatch::Checksum,
            ..
        }
    );
}

#[rstest]
fn verify_fails_when_the_mtime_moves_before_the_epoch(sandbox: Sandbox) {
    let path = sandbox.large_file();

    sandbox.saved(&path, TEST_STATE_NAME);

    if !set_pre_epoch_mtime(&path, Duration::from_secs(TEST_PRE_EPOCH.unsigned_abs())) {
        return;
    }

    assert_err_is!(
        sandbox.states().load(TEST_STATE_NAME),
        Error::Mismatch {
            kind: Mismatch::Mtime,
            ..
        }
    );
}

#[rstest]
fn verify_ignores_a_change_past_the_fingerprint_window(sandbox: Sandbox) {
    let path = sandbox.large_file();

    sandbox.saved(&path, TEST_STATE_NAME);

    let stamp = mtime(&path);

    sandbox.overwrite(&path, PAST_WINDOW as u64, b"\xff");

    set_mtime(&path, stamp);

    assert!(sandbox.states().load(TEST_STATE_NAME).is_ok());
}

#[rstest]
fn verify_fails_when_the_size_changed(sandbox: Sandbox) {
    let path = sandbox.large_file();

    sandbox.saved(&path, TEST_STATE_NAME);

    let stamp = mtime(&path);

    sandbox.truncate(&path, PAST_WINDOW as u64);

    set_mtime(&path, stamp);

    assert_err_is!(
        sandbox.states().load(TEST_STATE_NAME),
        Error::Mismatch {
            kind: Mismatch::Size,
            ..
        }
    );
}

#[rstest]
fn checksum_is_a_hex_digest_of_the_data(sandbox: Sandbox) {
    let state = sandbox.state(&sandbox.line_file());

    assert!(
        state
            .checksum
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    );

    assert_eq!(state.checksum.len(), 64);
    assert_eq!(state.checksum, state.checksum().unwrap());
}

#[rstest]
fn verify_fails_when_the_path_is_a_directory(sandbox: Sandbox) {
    let path = sandbox.line_file();

    sandbox.saved(&path, TEST_STATE_NAME);

    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();

    assert_err_is!(
        sandbox.states().load(TEST_STATE_NAME),
        Error::Io(error) if error.kind() == ErrorKind::IsADirectory
    );
}

#[rstest]
fn verify_fails_when_the_fingerprint_window_changed(sandbox: Sandbox) {
    let path = sandbox.large_file();

    sandbox.saved(&path, TEST_STATE_NAME);

    let stamp = mtime(&path);

    sandbox.overwrite(&path, 10, b"\xff");

    set_mtime(&path, stamp);

    assert_err_is!(
        sandbox.states().load(TEST_STATE_NAME),
        Error::Mismatch {
            kind: Mismatch::Identity,
            ..
        }
    );
}

#[rstest]
fn load_restores_the_saved_state_to_the_second(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let mut bytes = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    consume(&mut bytes, 2);

    let saved = bytes.save().unwrap().clone();

    assert_eq!(sandbox.states().load(TEST_STATE_NAME).unwrap(), saved);
}

#[rstest]
fn verify_fails_when_a_tampered_state_tracks_a_changed_file(sandbox: Sandbox) {
    let path = sandbox.large_file();

    sandbox.saved(&path, TEST_STATE_NAME);
    sandbox.tamper(TEST_STATE_NAME);

    let stamp = mtime(&path);

    sandbox.truncate(&path, PAST_WINDOW as u64);

    set_mtime(&path, stamp);

    assert_err_is!(
        sandbox.states().load(TEST_STATE_NAME),
        Error::Mismatch {
            kind: Mismatch::Checksum,
            ..
        }
    );
}

#[rstest]
fn resync_fails_when_a_tampered_state_tracks_a_changed_file(sandbox: Sandbox) {
    let path = sandbox.file(&b"a".repeat(20));

    sandbox.saved(&path, TEST_STATE_NAME);
    sandbox.tamper(TEST_STATE_NAME);
    sandbox.file(&b"b".repeat(20));

    assert_err_is!(
        sandbox.states().resync(TEST_STATE_NAME, &path),
        Error::Mismatch {
            kind: Mismatch::Checksum,
            ..
        }
    );
}

#[cfg(unix)]
#[rstest]
fn resync_fails_when_the_path_is_not_utf8(sandbox: Sandbox) {
    let content = TEST_LINE.repeat(TEST_TRACKED_COPIES);

    sandbox.recorded(&content, 5);

    let odd = sandbox.path().join(OsStr::from_bytes(TEST_NON_UTF8_NAME));

    if fs::write(&odd, &content).is_err() {
        return;
    }

    assert_err_is!(
        sandbox.states().resync(TEST_STATE_NAME, &odd),
        Error::Serde(_)
    );
}

#[rstest]
fn verify_fails_when_an_mtime_before_the_epoch_changed(sandbox: Sandbox) {
    let path = sandbox.large_file();

    if !set_pre_epoch_mtime(&path, Duration::from_secs(TEST_PRE_EPOCH.unsigned_abs())) {
        return;
    }

    sandbox.saved(&path, TEST_STATE_NAME);

    set_pre_epoch_mtime(&path, Duration::from_secs(172_800));

    assert_err_is!(
        sandbox.states().load(TEST_STATE_NAME),
        Error::Mismatch {
            kind: Mismatch::Mtime,
            ..
        }
    );
}

#[rstest]
fn resync_keeps_a_position_that_equals_the_size(sandbox: Sandbox) {
    let content = TEST_LINE.repeat(TEST_TRACKED_COPIES);
    let state = sandbox.recorded(&content, 100);

    let path = sandbox.write(TEST_TRACKED_NAME, &content);
    let resynced = sandbox.states().resync(TEST_STATE_NAME, &path).unwrap();

    assert_eq!(state.position, 100);
    assert_eq!(resynced.position, 100);
    assert_eq!(resynced.file.size, 100);
}

#[rstest]
fn verify_fails_when_the_mtime_changed_within_a_second(sandbox: Sandbox) {
    let path = sandbox.large_file();
    let stamp = UNIX_EPOCH + Duration::from_secs(TEST_STAMP.cast_unsigned());

    set_mtime(&path, stamp);

    sandbox.saved(&path, TEST_STATE_NAME);

    set_mtime(&path, stamp + Duration::from_micros(1));

    assert_err_is!(
        sandbox.states().load(TEST_STATE_NAME),
        Error::Mismatch {
            kind: Mismatch::Mtime,
            ..
        }
    );
}

#[rstest]
fn load_fails_when_the_file_is_deleted_before_the_save(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let mut bytes = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    fs::remove_file(&path).unwrap();

    bytes.save().unwrap();

    assert_err_is!(
        sandbox.states().load(TEST_STATE_NAME),
        Error::Mismatch {
            kind: Mismatch::Missing,
            ..
        }
    );
}

#[cfg(unix)]
#[rstest]
fn resync_fails_when_the_path_is_a_symlinked_directory(sandbox: Sandbox) {
    sandbox.saved(&sandbox.line_file(), TEST_STATE_NAME);

    let directory = sandbox.dir_at("elsewhere");
    let link = sandbox.path().join("link");

    symlink(&directory, &link).unwrap();

    assert_err_is!(
        sandbox.states().resync(TEST_STATE_NAME, &link),
        Error::Io(error) if error.kind() == ErrorKind::IsADirectory
    );
}

#[rstest]
fn save_refreshes_the_checksum(sandbox: Sandbox) {
    let path = sandbox.large_file();
    let mut bytes = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).build().unwrap();
    let before = bytes.save().unwrap().checksum.clone();

    consume(&mut bytes, 5);

    bytes.save().unwrap();

    assert_ne!(
        sandbox.states().load(TEST_STATE_NAME).unwrap().checksum,
        before
    );
}

#[rstest]
fn resync_accepts_an_mtime_before_the_epoch(sandbox: Sandbox) {
    let content = TEST_LINE.repeat(TEST_TRACKED_COPIES);

    sandbox.recorded(&content, 5);

    let path = sandbox.path().join(TEST_TRACKED_NAME);

    if !set_pre_epoch_mtime(&path, Duration::from_secs(TEST_PRE_EPOCH.unsigned_abs())) {
        return;
    }

    let resynced = sandbox.states().resync(TEST_STATE_NAME, &path).unwrap();

    assert_eq!(resynced.position, 5);
    assert_eq!(resynced.file.mtime.unix_timestamp(), TEST_PRE_EPOCH);
}

#[rstest]
fn save_updates_only_the_updated_at(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let state = state_data(canonical(&path));
    let mut bytes = sandbox.lenient().bytes(&path).state(state).build().unwrap();
    let created_at = bytes.state().timestamps.created_at;

    bytes.save().unwrap();

    assert!(bytes.state().timestamps.updated_at > created_at);

    assert_eq!(bytes.state().timestamps.created_at, created_at);
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
    sandbox.recorded(&b"a".repeat(before), read);

    let path = sandbox.write(TEST_TRACKED_NAME, &b"a".repeat(after));
    let resynced = sandbox.states().resync(TEST_STATE_NAME, &path).unwrap();

    assert!(resynced.position <= resynced.file.size);

    assert_eq!(resynced.position, position);
    assert_eq!(resynced.file.size, after as u64);
    assert_eq!(resynced.percent(), percent);
}

#[rstest]
fn resync_saves_the_moved_path(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let mut bytes = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    bytes.save().unwrap();

    let moved = sandbox.write("moved.bin", TEST_LINE);

    sandbox.states().resync(TEST_STATE_NAME, &moved).unwrap();

    assert_eq!(
        sandbox.states().load(TEST_STATE_NAME).unwrap().file.path,
        canonical(&moved)
    );
}

#[rstest]
fn percent_tracks_the_position(sandbox: Sandbox) {
    let path = sandbox.file(b"foo");
    let mut bytes = sandbox.reader().bytes(&path).build().unwrap();

    assert_eq!(bytes.state().percent(), 0.0);

    consume(&mut bytes, 2);

    assert_eq!(bytes.state().percent(), 100.0 / 3.0);

    drain(&mut bytes);

    assert_eq!(bytes.state().percent(), 100.0);
}

#[cfg(unix)]
#[rstest]
fn resync_fails_when_the_file_is_unreadable(sandbox: Sandbox) {
    let content = TEST_LINE.repeat(TEST_TRACKED_COPIES);

    sandbox.recorded(&content, 5);

    let path = sandbox.path().join(TEST_TRACKED_NAME);
    let mut blocked = Blocked::default();

    if !Blocked::enforced(&sandbox.path().join("probe")) {
        return;
    }

    blocked.block(&path);

    assert_err_is!(
        sandbox.states().resync(TEST_STATE_NAME, &path),
        Error::Io(error) if error.kind() == ErrorKind::PermissionDenied
    );
}

#[rstest]
#[case::name("\"name\"")]
#[case::file("\"file\"")]
#[case::position("\"position\"")]
#[case::timestamps("\"timestamps\"")]
fn payload_keeps_the_checksum_last(sandbox: Sandbox, #[case] key: &str) {
    let path = sandbox.line_file();
    let mut bytes = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    bytes.save().unwrap();

    let text = fs::read_to_string(sandbox.states().path(TEST_STATE_NAME).unwrap()).unwrap();
    let at = |key: &str| text.find(key).unwrap();

    assert!(at("\"_checksum\"") > at(key));
}

#[rstest]
#[case::missing("missing.bin", false, ErrorKind::NotFound)]
#[case::a_directory("elsewhere", true, ErrorKind::IsADirectory)]
fn resync_fails_when_the_path_is_not_a_file_without_verification(
    sandbox: Sandbox,
    #[case] name: &str,
    #[case] directory: bool,
    #[case] kind: ErrorKind,
) {
    let path = sandbox.write(TEST_TRACKED_NAME, &TEST_LINE.repeat(TEST_TRACKED_COPIES));
    let reader = sandbox.lenient();

    reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap().save().unwrap();

    let target = if directory {
        sandbox.dir_at(name)
    } else {
        sandbox.path().join(name)
    };

    assert_err_is!(
        reader.states().resync(TEST_STATE_NAME, &target),
        Error::Io(error) if error.kind() == kind
    );
}

#[rstest]
fn resync_moves_the_identity_forward(sandbox: Sandbox) {
    sandbox.recorded(&TEST_LINE.repeat(TEST_TRACKED_COPIES), 5);

    let original = sandbox.path().join(TEST_TRACKED_NAME);
    let grown = sandbox.write("grown.bin", &TEST_LINE.repeat(30));
    let once = sandbox.states().resync(TEST_STATE_NAME, &grown).unwrap();

    assert_eq!(once.file.size, 120);

    assert_err_is!(
        sandbox.states().resync(TEST_STATE_NAME, &original),
        Error::Mismatch {
            kind: Mismatch::Identity,
            ..
        }
    );
}

#[rstest]
#[case::position_past_the_size("/position", 999, 33300.0)]
#[case::zero_size("/file/size", 0, 300.0)]
fn percent_reads_the_stored_numbers(
    sandbox: Sandbox,
    #[case] pointer: &str,
    #[case] value: u64,
    #[case] expected: f64,
) {
    let reader = sandbox.lenient();
    let path = sandbox.file(b"foo");
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    drain(&mut bytes);

    bytes.save().unwrap();
    sandbox.patch(TEST_STATE_NAME, |payload| {
        *payload.pointer_mut(pointer).unwrap() = value.into()
    });

    assert_eq!(
        reader.states().load(TEST_STATE_NAME).unwrap().percent(),
        expected
    );
}

#[rstest]
fn resync_reseals_a_tampered_state(sandbox: Sandbox) {
    let path = sandbox.file(&b"a".repeat(20));

    sandbox.saved(&path, TEST_STATE_NAME);
    sandbox.tamper(TEST_STATE_NAME);

    assert_err_is!(
        sandbox.states().load(TEST_STATE_NAME),
        Error::Mismatch {
            kind: Mismatch::Checksum,
            ..
        }
    );

    let resynced = sandbox.lenient().states().resync(TEST_STATE_NAME, &path).unwrap();

    assert_eq!(resynced.position, 20);
    assert_eq!(sandbox.states().load(TEST_STATE_NAME).unwrap(), resynced);
}

#[rstest]
#[case::path_rejected("../../etc/passwd", |failure: &Error| matches!(failure, Error::Name(NameError::Escapes)))]
#[case::commit_failed(TEST_STATE_NAME, |failure: &Error| matches!(failure, Error::Io(error) if error.kind() == ErrorKind::AlreadyExists))]
fn save_leaves_the_state_untouched_when_it_fails(
    sandbox: Sandbox,
    #[case] name: &str,
    #[case] expected: fn(&Error) -> bool,
) {
    let path = sandbox.line_file();
    let mut data = state_data(canonical(&path));

    data.name = name.to_owned();

    let mut bytes = sandbox.lenient().bytes(&path).state(data).build().unwrap();
    let before = bytes.state().clone();

    sandbox.block_states();

    let error = bytes.save().unwrap_err();

    assert!(expected(&error), "{error:?}");

    assert_eq!(*bytes.state(), before);
}

#[rstest]
fn resync_ignores_a_change_past_the_fingerprint_window(sandbox: Sandbox) {
    let mut after = TEST_LINE.repeat(TEST_LARGE_COPIES);

    after[PAST_WINDOW] = b'\xff';

    sandbox.recorded(&TEST_LINE.repeat(TEST_LARGE_COPIES), 6000);

    let path = sandbox.write(TEST_TRACKED_NAME, &after);
    let resynced = sandbox.states().resync(TEST_STATE_NAME, &path).unwrap();
    let resumed = sandbox.reader().bytes(&path).state(resynced.clone()).build().unwrap();

    assert_eq!(resynced.position, 6000);
    assert_eq!(resumed.state().position, 6000);
}

#[rstest]
fn resync_keeps_name_position_and_created_at(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let state = state_data(canonical(&path));

    sandbox.persist(&state);

    let moved = sandbox.write("moved.bin", &TEST_LINE.repeat(2));
    let resynced = sandbox.states().resync(TEST_STATE_NAME, &moved).unwrap();

    assert!(resynced.timestamps.updated_at > state.timestamps.updated_at);

    assert_eq!(resynced.name, state.name);
    assert_eq!(resynced.position, state.position);
    assert_eq!(resynced.timestamps.created_at, state.timestamps.created_at);
}

#[rstest]
fn resync_updates_the_file_metadata(sandbox: Sandbox) {
    sandbox.saved(&sandbox.line_file(), TEST_STATE_NAME);

    let moved = sandbox.write("moved.bin", b"foo\nbar");
    let resynced = sandbox.states().resync(TEST_STATE_NAME, &moved).unwrap();

    assert_eq!(resynced.file.path, canonical(&moved));
    assert_eq!(resynced.file.size, 7);
    assert_eq!(
        resynced.file.mtime,
        UtcDateTime::from(mtime(&moved)).truncate_to_microsecond()
    );
    assert_eq!(
        resynced.file.fingerprint,
        file::fingerprint(&moved, TEST_WINDOW as u64).unwrap()
    );
}

#[rstest]
fn resync_is_stable_when_repeated(sandbox: Sandbox) {
    let content = TEST_LINE.repeat(TEST_TRACKED_COPIES);

    sandbox.recorded(&content, 5);

    let path = sandbox.write(TEST_TRACKED_NAME, &content);
    let once = sandbox.states().resync(TEST_STATE_NAME, &path).unwrap();
    let twice = sandbox.states().resync(TEST_STATE_NAME, &path).unwrap();
    let resumed = sandbox.reader().bytes(&path).state(twice.clone()).build().unwrap();

    assert_eq!(twice.position, 5);
    assert_eq!(twice.file, once.file);
    assert_eq!(resumed.state().position, 5);
}

#[rstest]
#[case::appended(25, b"more")]
#[case::unchanged(25, b"")]
#[case::grown_past_a_wide_window(TEST_LARGE_COPIES, b"more")]
fn resync_accepts_a_kept_prefix(sandbox: Sandbox, #[case] copies: usize, #[case] extra: &[u8]) {
    let before = TEST_LINE.repeat(copies);
    let after = [&before[..], extra].concat();

    sandbox.recorded(&before, 5);

    let path = sandbox.write(TEST_TRACKED_NAME, &after);
    let resynced = sandbox.states().resync(TEST_STATE_NAME, &path).unwrap();
    let resumed = sandbox.reader().bytes(&path).state(resynced.clone()).build().unwrap();

    assert_eq!(resynced.position, 5);
    assert_eq!(resynced.file.size, after.len() as u64);
    assert_eq!(resumed.state().position, 5);
}

#[rstest]
fn resync_clamps_the_position_to_the_new_size(sandbox: Sandbox) {
    let content = TEST_LINE.repeat(TEST_LARGE_COPIES);
    let saved = sandbox.recorded(&content, 6000);
    let path = sandbox.write(TEST_TRACKED_NAME, &content);

    sandbox.truncate(&path, PAST_WINDOW as u64);

    let resynced = sandbox.states().resync(TEST_STATE_NAME, &path).unwrap();
    let resumed = sandbox.reader().bytes(&path).state(resynced.clone()).build().unwrap();

    assert_eq!(saved.position, 6000);
    assert_eq!(resynced.position, PAST_WINDOW as u64);
    assert_eq!(resynced.percent(), 100.0);
    assert_eq!(resumed.state().position, PAST_WINDOW as u64);
}

#[rstest]
#[case::replaced(b"bar\n", TEST_TRACKED_COPIES, 50, 50.0)]
#[case::shrunk(TEST_LINE, 2, 8, 100.0)]
#[case::emptied(b"", 0, 0, 0.0)]
fn resync_trusts_the_path_without_verification(
    sandbox: Sandbox,
    #[case] unit: &[u8],
    #[case] copies: usize,
    #[case] position: u64,
    #[case] percent: f64,
) {
    let path = sandbox.write(TEST_TRACKED_NAME, &TEST_LINE.repeat(TEST_TRACKED_COPIES));
    let mut bytes =
        sandbox.lenient().bytes(&path).state(TEST_STATE_NAME).limit(50).build().unwrap();

    drain(&mut bytes);

    bytes.save().unwrap();

    sandbox.write(TEST_TRACKED_NAME, &unit.repeat(copies));

    let resynced = sandbox.lenient().states().resync(TEST_STATE_NAME, &path).unwrap();

    assert_eq!(resynced.position, position);
    assert_eq!(resynced.file.size, (unit.len() * copies) as u64);
    assert_eq!(resynced.percent(), percent);
}

#[rstest]
fn checksum_covers_every_serialized_field(digested: State) {
    let before = digested.checksum().unwrap();
    let payload = serde_json::to_value(&digested).unwrap();
    let pointers: Vec<String> = leaves(&payload, String::new())
        .into_iter()
        .filter(|pointer| pointer.strip_prefix('/') != Some(CHECKSUM_KEY))
        .collect();

    assert!(!pointers.is_empty());

    for pointer in pointers {
        let mut changed = payload.clone();
        let leaf = changed.pointer_mut(&pointer).unwrap();

        bump(leaf, &pointer);

        let state: State = serde_json::from_value(changed).unwrap();

        assert_ne!(state.checksum().unwrap(), before, "{pointer}");
    }
}

#[rstest]
fn resync_lets_a_grown_file_resume(sandbox: Sandbox) {
    let path = sandbox.line_file();

    sandbox.checkpoint(&path, TEST_STATE_NAME, 2);
    sandbox.append(&path, b"more");

    assert_err_is!(
        sandbox.reader().bytes(&path).state(TEST_STATE_NAME).build(),
        Error::Mismatch {
            kind: Mismatch::Size,
            ..
        }
    );

    let resynced = sandbox.states().resync(TEST_STATE_NAME, &path).unwrap();
    let size = resynced.file.size;
    let bytes = sandbox.reader().bytes(&path).state(resynced).build().unwrap();

    assert_eq!(size, fs::metadata(&path).unwrap().len());
    assert_eq!(items(bytes), b"o\nmore");
}

#[rstest]
fn payload_carries_every_field(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let mut bytes = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    bytes.save().unwrap();

    let payload = sandbox.payload(bytes.state());
    let keys = |value: &Value| {
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
fn resync_lets_a_moved_file_resume(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let moved = sandbox.path().join("moved.bin");
    let state = sandbox.checkpoint(&path, TEST_STATE_NAME, 2);

    fs::rename(&path, &moved).unwrap();

    assert_err_is!(
        sandbox.reader().bytes(&moved).state(TEST_STATE_NAME).build(),
        Error::Mismatch {
            kind: Mismatch::Path,
            ..
        }
    );
    assert_err_is!(
        sandbox.reader().bytes(&moved).state(state).build(),
        Error::Mismatch {
            kind: Mismatch::Path,
            ..
        }
    );

    let resynced = sandbox.states().resync(TEST_STATE_NAME, &moved).unwrap();
    let bytes = sandbox.reader().bytes(&moved).state(resynced).build().unwrap();

    assert_eq!(bytes.state().file.path, canonical(&moved));
    assert_eq!(items(bytes), b"o\n");
}

#[rstest]
fn verify_fails_when_any_saved_field_is_tampered(sandbox: Sandbox) {
    let state_path = sandbox.saved(&sandbox.large_file(), TEST_STATE_NAME);
    let saved: Value = serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    let pointers: Vec<String> = leaves(&saved, String::new())
        .into_iter()
        .filter(|pointer| pointer.strip_prefix('/') != Some(CHECKSUM_KEY))
        .collect();

    assert!(!pointers.is_empty());

    for pointer in pointers {
        let mut payload = saved.clone();
        let leaf = payload.pointer_mut(&pointer).unwrap();

        bump(leaf, &pointer);

        fs::write(&state_path, serde_json::to_string(&payload).unwrap()).unwrap();

        let expected = match pointer.as_str() {
            "/name" => Mismatch::Name {
                saved: format!("{TEST_STATE_NAME}-changed"),
            },
            _ => Mismatch::Checksum,
        };

        assert_err_is!(
            sandbox.states().load(TEST_STATE_NAME),
            Error::Mismatch { kind, .. } if kind.to_string() == expected.to_string(),
            "{pointer}"
        );
    }
}
