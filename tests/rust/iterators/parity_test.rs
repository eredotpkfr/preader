use std::{
    fs::{self, OpenOptions},
    os::unix::fs::symlink,
    path::{Path, PathBuf},
};

use preader::{Config, Error, IteratorBuild, IteratorRead, PReader, State};
use rstest::rstest;
use tempfile::TempDir;

use crate::common::{
    constants::TEST_STATE_NAME,
    fixtures::tmp_dir,
    funcs::{reader, write},
};

const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
const LINES: &[u8] = b"line-0\nline-1\nline-2\nline-3\nline-4\nline-5";
const UNSAFE_NAMES: [(&str, &str); 4] = [
    ("../../etc/passwd", "path escapes root"),
    ("/tmp", "path escapes root"),
    ("", "path must not be empty"),
    (".", "path must not be empty"),
];

fn append(path: &Path, content: &[u8]) {
    use std::io::Write;

    OpenOptions::new().append(true).open(path).unwrap().write_all(content).unwrap();
}

fn truncate(path: &Path, size: u64) {
    OpenOptions::new().write(true).open(path).unwrap().set_len(size).unwrap();
}

fn real(path: &Path) -> PathBuf {
    path.canonicalize().unwrap()
}

fn saved_state(reader: &PReader, path: &Path) -> State {
    let mut state = reader.bytes(path).state(TEST_STATE_NAME).build().unwrap().state().clone();

    state.save().unwrap();

    state
}

#[rstest]
fn auto_load_disabled_ignores_a_saved_state(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", ALPHABET);

    saved_state(&reader, &path);

    assert_eq!(
        reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap().state().position,
        0
    );
}

#[rstest]
fn auto_load_resumes_the_previous_position(tmp_dir: TempDir) {
    let config = Config {
        auto_load_state: true,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.bin", ALPHABET);
    let mut bytes = reader.bytes(&path).build().unwrap();

    bytes.read().unwrap();
    bytes.state().save().unwrap();
    drop(bytes);

    assert_eq!(reader.bytes(&path).build().unwrap().state().position, 1);
}

#[rstest]
fn auto_load_ignores_an_unverifiable_state(tmp_dir: TempDir) {
    let config = Config {
        auto_load_state: true,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.bin", ALPHABET);
    let mut bytes = reader.bytes(&path).build().unwrap();

    bytes.read().unwrap();
    bytes.state().save().unwrap();
    drop(bytes);

    append(&path, b"tampered");

    assert_eq!(reader.bytes(&path).build().unwrap().state().position, 0);
}

#[rstest]
fn auto_load_resumes_a_stale_state_without_verification(tmp_dir: TempDir) {
    let config = Config {
        auto_load_state: true,
        verify_state: false,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.bin", ALPHABET);
    let mut bytes = reader.bytes(&path).build().unwrap();

    bytes.read().unwrap();
    bytes.state().save().unwrap();
    drop(bytes);

    append(&path, b"tampered");

    assert_eq!(reader.bytes(&path).build().unwrap().state().position, 1);
}

#[rstest]
#[case::corrupt("not valid json")]
#[case::incomplete(r#"{"name": "job-1", "position": 3}"#)]
fn auto_load_ignores_an_unreadable_payload(tmp_dir: TempDir, #[case] payload: &str) {
    let config = Config {
        auto_load_state: true,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.bin", ALPHABET);
    let mut bytes = reader.bytes(&path).build().unwrap();

    bytes.read().unwrap();

    let state_path = bytes.state().save().unwrap();

    drop(bytes);
    fs::write(&state_path, payload).unwrap();

    assert_eq!(reader.bytes(&path).build().unwrap().state().position, 0);
}

#[rstest]
fn auto_load_starts_fresh_when_the_name_tracks_another_file(tmp_dir: TempDir) {
    let config = Config {
        auto_load_state: true,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let first = write(&tmp_dir, "data-1.bin", b"foo");
    let second = write(&tmp_dir, "data-2.bin", b"bar");

    reader.bytes(&first).state("shared").build().unwrap().state().save().unwrap();

    let mut reused = reader.bytes(&second).state("shared").build().unwrap();

    assert_eq!(reused.state().file.path, real(&second));
    assert_eq!(reused.read().unwrap(), Some(b'b'));

    reused.state().save().unwrap();
    drop(reused);

    assert_eq!(
        reader.states().load("shared").unwrap().file.path,
        real(&second)
    );
}

#[rstest]
fn verification_disabled_accepts_a_stale_state_object(tmp_dir: TempDir) {
    let config = Config {
        verify_state: false,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.bin", ALPHABET);
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    bytes.read().unwrap();

    let state = bytes.state().clone();

    drop(bytes);
    append(&path, b"more");

    assert_eq!(
        reader.bytes(&path).state(state).build().unwrap().state().position,
        1
    );
}

#[rstest]
fn verification_disabled_reads_the_tracked_file_not_the_argument(tmp_dir: TempDir) {
    let config = Config {
        verify_state: false,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let tracked = write(&tmp_dir, "tracked.bin", b"foo");
    let untracked = write(&tmp_dir, "untracked.bin", b"bar");
    let state = reader.bytes(&tracked).state(TEST_STATE_NAME).build().unwrap().state().clone();
    let mut resumed = reader.bytes(&untracked).state(state).build().unwrap();

    assert_eq!(resumed.state().file.path, real(&tracked));

    let collected: Vec<u8> = resumed.map(|byte| byte.unwrap()).collect();

    assert_eq!(collected, b"foo");
}

#[rstest]
fn resync_allows_resuming_a_moved_file(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", ALPHABET);
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    bytes.read().unwrap();
    bytes.state().save().unwrap();

    let state = bytes.state().clone();

    drop(bytes);

    let moved = tmp_dir.path().join("moved.bin");

    fs::rename(&path, &moved).unwrap();

    let error = reader.bytes(&moved).state(state.clone()).build().unwrap_err();

    assert!(error.to_string().contains("resync"), "{error}");

    let resynced = state.resync(&moved).unwrap();
    let mut resumed = reader.bytes(&moved).state(resynced).build().unwrap();

    assert_eq!(resumed.state().file.path, real(&moved));

    let collected: Vec<u8> = resumed.map(|byte| byte.unwrap()).collect();

    assert_eq!(collected, &ALPHABET[1..]);
}

#[rstest]
fn resync_allows_resuming_a_grown_file(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", ALPHABET);
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    while bytes.read().unwrap().is_some() {}

    bytes.state().save().unwrap();

    let state = bytes.state().clone();

    drop(bytes);
    append(&path, b"more");

    let error = reader.bytes(&path).state(state.clone()).build().unwrap_err();

    assert!(error.to_string().contains("file size mismatch"), "{error}");

    let resynced = state.resync(&path).unwrap();

    assert_eq!(resynced.file.size, ALPHABET.len() as u64 + 4);

    let resumed = reader.bytes(&path).state(resynced).build().unwrap();
    let collected: Vec<u8> = resumed.map(|byte| byte.unwrap()).collect();

    assert_eq!(collected, b"more");
}

#[rstest]
fn a_state_object_keeps_its_own_state_dir(tmp_dir: TempDir) {
    let config = Config {
        auto_save_state: true,
        ..Config::default()
    };
    let owner = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.bin", ALPHABET);
    let state = owner.bytes(&path).state(TEST_STATE_NAME).build().unwrap().state().clone();

    let elsewhere = tmp_dir.path().join("other-states");
    let borrower = PReader::from(Config {
        state_dir: elsewhere.clone(),
        auto_save_state: true,
        verify_state: false,
        ..Config::default()
    });

    for byte in borrower.bytes(&path).state(state).build().unwrap() {
        byte.unwrap();
    }

    let name = format!("{TEST_STATE_NAME}.state.json");

    assert!(owner.config().state_dir.join(&name).is_file());
    assert!(!elsewhere.join(&name).exists());
}

#[rstest]
fn changing_the_state_name_leaves_the_old_state_in_place(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", ALPHABET);

    reader.bytes(&path).state("job-old").build().unwrap().state().save().unwrap();

    let mut fresh = reader.bytes(&path).state("job-new").build().unwrap();

    assert_eq!(fresh.state().position, 0);
    assert!(reader.states().exists("job-old"));
}

#[rstest]
fn changing_the_state_dir_creates_a_fresh_state(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", ALPHABET);
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    bytes.read().unwrap();
    bytes.state().save().unwrap();
    drop(bytes);

    let elsewhere = PReader::from(Config {
        state_dir: tmp_dir.path().join("other-states"),
        auto_load_state: true,
        ..Config::default()
    });

    assert_eq!(
        elsewhere.bytes(&path).state(TEST_STATE_NAME).build().unwrap().state().position,
        0
    );
}

#[rstest]
fn an_unsafe_name_defers_its_rejection_to_the_save(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", ALPHABET);

    for (name, message) in UNSAFE_NAMES {
        let mut bytes = reader.bytes(&path).state(name).build().unwrap();

        assert_eq!(bytes.state().position, 0, "{name}");

        let error = bytes.state().save().unwrap_err();

        assert!(error.to_string().contains(message), "{name}: {error}");
    }
}

#[rstest]
fn the_autoname_changes_when_the_file_moves(tmp_dir: TempDir) {
    let config = Config {
        auto_load_state: true,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.bin", ALPHABET);
    let mut bytes = reader.bytes(&path).build().unwrap();

    bytes.read().unwrap();
    bytes.state().save().unwrap();
    drop(bytes);

    let moved = tmp_dir.path().join("moved.bin");

    fs::rename(&path, &moved).unwrap();

    assert_eq!(reader.bytes(&moved).build().unwrap().state().position, 0);
}

#[rstest]
fn two_symlinks_to_the_same_target_share_the_autoname(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let real = write(&tmp_dir, "real.bin", ALPHABET);
    let (first, second) = (
        tmp_dir.path().join("link-1.bin"),
        tmp_dir.path().join("link-2.bin"),
    );

    symlink(&real, &first).unwrap();
    symlink(&real, &second).unwrap();

    assert_eq!(
        reader.bytes(&first).build().unwrap().state().name,
        reader.bytes(&second).build().unwrap().state().name
    );
}

#[rstest]
fn a_deleted_tracked_file_fails_the_build(tmp_dir: TempDir) {
    let config = Config {
        verify_state: false,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let tracked = write(&tmp_dir, "tracked.bin", b"foo");
    let untracked = write(&tmp_dir, "untracked.bin", b"foo");
    let state = reader.bytes(&tracked).state(TEST_STATE_NAME).build().unwrap().state().clone();

    fs::remove_file(&tracked).unwrap();

    let error = reader.bytes(&untracked).state(state).build().unwrap_err();

    assert!(
        matches!(&error, Error::Io(io) if io.kind() == std::io::ErrorKind::NotFound),
        "{error}"
    );
}

#[rstest]
fn reading_a_file_replaced_by_a_directory_fails(tmp_dir: TempDir) {
    let config = Config {
        verify_state: false,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.bin", ALPHABET);
    let state = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap().state().clone();

    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();

    let outcome = reader
        .bytes(&path)
        .state(state)
        .build()
        .and_then(|mut bytes| bytes.try_for_each(|byte| byte.map(drop)));

    assert!(matches!(outcome, Err(Error::Io(_))), "{outcome:?}");
}

#[rstest]
fn a_error_surfaces_after_a_truncation(tmp_dir: TempDir) {
    let config = Config {
        auto_save_state: true,
        buffer_capacity: 1,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.bin", ALPHABET);
    let mut bytes = reader.bytes(&path).state("../../escape").build().unwrap();

    bytes.read().unwrap();
    truncate(&path, 1);

    assert!(
        bytes.all(|item| item.is_ok()),
        "a save failure must not reach the item stream"
    );

    let error = bytes.error().unwrap();

    assert!(error.to_string().contains("path escapes root"), "{error}");
}

#[rstest]
fn a_error_surfaces_on_a_skipped_item(tmp_dir: TempDir) {
    let config = Config {
        auto_save_state: true,
        auto_save_state_bytes: 1,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.txt", LINES);
    let mut lines = reader.lines(&path).state("../../escape").skip(2).build().unwrap();

    assert_eq!(lines.read().unwrap(), Some("line-2"));

    let error = lines.error().unwrap();

    assert!(error.to_string().contains("path escapes root"), "{error}");
    assert_eq!(lines.state().position, 21);
}

#[rstest]
fn a_error_surfaces_on_a_filtered_blank(tmp_dir: TempDir) {
    let config = Config {
        auto_save_state: true,
        auto_save_state_bytes: 1,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.txt", b"\nfoo\n");
    let mut lines = reader.lines(&path).state("../../escape").skip_empty(true).build().unwrap();

    assert_eq!(lines.read().unwrap(), Some("foo"));

    let error = lines.error().unwrap();

    assert!(error.to_string().contains("path escapes root"), "{error}");
    assert_eq!(lines.state().position, 5);
}

#[rstest]
fn a_error_surfaces_when_end_drops_a_chunk(tmp_dir: TempDir) {
    let config = Config {
        auto_save_state: true,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.bin", ALPHABET);
    let mut chunks = reader
        .chunks(&path)
        .state("../../escape")
        .size(8)
        .drop_partial(true)
        .end(12)
        .build()
        .unwrap();

    assert!(
        chunks.all(|item| item.is_ok()),
        "a save failure must not reach the item stream"
    );

    let error = chunks.error().unwrap();

    assert!(error.to_string().contains("path escapes root"), "{error}");
}

#[rstest]
fn dropping_an_iterator_without_progress_saves_nothing(tmp_dir: TempDir) {
    let config = Config {
        auto_save_state: true,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.bin", ALPHABET);

    drop(reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap());

    assert!(!reader.states().exists(TEST_STATE_NAME));
}

#[rstest]
fn clearing_the_registry_does_not_disturb_a_live_iterator(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", ALPHABET);
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    bytes.read().unwrap();
    bytes.state().save().unwrap();
    reader.states().clear().unwrap();

    assert!(!reader.states().exists(TEST_STATE_NAME));

    bytes.read().unwrap();

    assert_eq!(bytes.state().position, 2);
    assert_eq!(
        reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap().state().position,
        0
    );
}

#[rstest]
fn percent_tracks_the_position_during_iteration(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", ALPHABET);
    let size = ALPHABET.len() as f64;
    let mut bytes = reader.bytes(&path).build().unwrap();

    assert_eq!(bytes.state().percent(), 0.0);

    while bytes.read().unwrap().is_some() {
        let state = bytes.state();

        assert!((state.percent() - state.position as f64 / size * 100.0).abs() < 1e-9);
    }

    assert_eq!(bytes.state().percent(), 100.0);
}

#[rstest]
fn the_recorded_file_size_never_refreshes(tmp_dir: TempDir) {
    let config = Config {
        verify_state: false,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.bin", ALPHABET);
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    while bytes.read().unwrap().is_some() {}

    let state = bytes.state().clone();

    drop(bytes);
    append(&path, b"more");

    let mut resumed = reader.bytes(&path).state(state).end(100).build().unwrap();

    assert!(resumed.read().unwrap().is_none());
    assert_eq!(resumed.state().file.size, ALPHABET.len() as u64);
}

#[rstest]
#[case::stripped(false, ["foo", "bar"])]
#[case::kept(true, ["foo\r\r\n", "bar\r\r\r\n"])]
fn repeated_carriage_returns_are_stripped_together(
    tmp_dir: TempDir,
    #[case] keepends: bool,
    #[case] expected: [&str; 2],
) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.txt", b"foo\r\r\nbar\r\r\r\n");
    let mut lines = reader.lines(&path).keepends(keepends).build().unwrap();
    let mut collected = Vec::new();

    while let Some(line) = lines.read().unwrap() {
        collected.push(line.to_owned());
    }

    assert_eq!(collected, expected);
}

#[rstest]
fn skip_counts_blank_items_before_skip_empty(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.txt", b"foo\n\nbar\nbaz\n");

    let kept: Vec<String> =
        reader.lines(&path).skip(1).build().unwrap().map(|line| line.unwrap()).collect();
    let filtered: Vec<String> = reader
        .lines(&path)
        .skip(1)
        .skip_empty(true)
        .build()
        .unwrap()
        .map(|line| line.unwrap())
        .collect();

    assert_eq!(kept, ["", "bar", "baz"]);
    assert_eq!(filtered, ["bar", "baz"]);
}

#[rstest]
fn skip_stops_at_a_truncation(tmp_dir: TempDir) {
    let config = Config {
        verify_state: false,
        auto_load_state: true,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.bin", ALPHABET);

    saved_state(&reader, &path);
    truncate(&path, 8);

    let mut lines = reader.lines(&path).state(TEST_STATE_NAME).skip(3).build().unwrap();

    assert!(lines.read().unwrap().is_none());
}

#[rstest]
fn drop_partial_counts_a_discarded_truncated_chunk(tmp_dir: TempDir) {
    let config = Config {
        buffer_capacity: 1,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.bin", b"foobarbaz");
    let mut chunks = reader.chunks(&path).size(3).drop_partial(true).build().unwrap();

    assert_eq!(chunks.read().unwrap(), Some(b"foo".as_slice()));

    truncate(&path, 5);

    assert!(chunks.read().unwrap().is_none());
    assert_eq!(chunks.state().position, 5);
}

#[rstest]
fn a_resume_honours_a_different_keepends(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.txt", b"foo\nbar\nbaz\n");
    let mut lines = reader.lines(&path).state(TEST_STATE_NAME).build().unwrap();

    assert_eq!(lines.read().unwrap(), Some("foo"));

    lines.state().save().unwrap();

    let state = lines.state().clone();

    drop(lines);

    let resumed = reader.lines(&path).state(state).keepends(true).build().unwrap();
    let collected: Vec<String> = resumed.map(|line| line.unwrap()).collect();

    assert_eq!(collected, ["bar\n", "baz\n"]);
}
