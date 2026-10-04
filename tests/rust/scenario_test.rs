use std::fs;

use preader::{
    Config, DEFAULT_BUFFER_CAPACITY, DEFAULT_DELIMITER, Error, IteratorBuild, IteratorRead,
    PReader, STATE_FILE_EXTENSION,
};
use rstest::rstest;

use crate::common::{
    constants::{
        TEST_ALPHABET, TEST_BLANK_LINE_CONTENT, TEST_LINE_CONTENT, TEST_OTHER_STATE_NAME,
        TEST_SEGMENT_CONTENT, TEST_STATE_NAME, TEST_UNSAFE_NAME,
    },
    fixtures::sandbox,
    funcs::{items, texts},
    iterators::{ITERATORS, LOSSLESS_ITERATORS, Plan},
    sandbox::Sandbox,
};

#[rstest]
#[case::tiny(1)]
#[case::uneven(3)]
#[case::large(DEFAULT_BUFFER_CAPACITY)]
fn buffer_capacity_does_not_change_the_output(sandbox: Sandbox, #[case] capacity: usize) {
    let path = sandbox.file(TEST_LINE_CONTENT);
    let capped = sandbox.capped(capacity);
    let plain = sandbox.reader();

    for iterator in ITERATORS {
        let expected = iterator.read(&plain, &path, Plan::default()).unwrap().0;
        let found = iterator.read(&capped, &path, Plan::default()).unwrap().0;

        assert_eq!(found, expected, "{iterator:?}");
    }
}

#[rstest]
fn resume_skips_again_when_position_equals_start(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.resuming();
    let mut first = reader.bytes(&path).state(TEST_STATE_NAME).start(2).limit(1).build().unwrap();

    while first.read().unwrap().is_some() {}

    first.state().save().unwrap();
    drop(first);

    let mut resumed = reader.bytes(&path).state(TEST_STATE_NAME).start(3).skip(1).build().unwrap();

    assert_eq!(resumed.read().unwrap(), Some(TEST_ALPHABET[4]));
}

#[rstest]
fn chunk_start_is_not_realigned_to_the_grid(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let read = items(sandbox.reader().chunks(&path).size(4).start(1).limit(1).build().unwrap());

    assert_eq!(read, [b"bcde"]);
}

#[rstest]
fn resume_honours_a_different_chunk_size(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.reader();
    let mut chunks = reader.chunks(&path).state(TEST_STATE_NAME).size(4).limit(1).build().unwrap();

    while chunks.read().unwrap().is_some() {}

    chunks.state().save().unwrap();

    let state = chunks.state().clone();

    drop(chunks);

    let read = items(reader.chunks(&path).state(state).size(2).limit(2).build().unwrap());

    assert_eq!(read, [b"ef", b"gh"]);
}

#[rstest]
fn resume_honours_a_different_delimiter(sandbox: Sandbox) {
    let path = sandbox.file(b"a,b;c,d");
    let reader = sandbox.reader();
    let mut segments = reader
        .delimiter(&path)
        .state(TEST_STATE_NAME)
        .character(DEFAULT_DELIMITER)
        .limit(1)
        .build()
        .unwrap();

    while segments.read().unwrap().is_some() {}

    segments.state().save().unwrap();

    let state = segments.state().clone();

    drop(segments);

    let read = items(reader.delimiter(&path).state(state).character(b';').build().unwrap());

    assert_eq!(texts(&read), ["b", "c,d"]);
}

#[rstest]
fn resume_honours_a_different_keepends(sandbox: Sandbox) {
    let path = sandbox.file(TEST_LINE_CONTENT);
    let reader = sandbox.reader();
    let mut lines = reader.lines(&path).state(TEST_STATE_NAME).limit(1).build().unwrap();

    while lines.read().unwrap().is_some() {}

    lines.state().save().unwrap();

    let state = lines.state().clone();

    drop(lines);

    let read = items(reader.lines(&path).state(state).keepends(true).limit(1).build().unwrap());

    assert_eq!(read, ["line-1\n"]);
}

#[rstest]
fn skip_counts_blank_items_before_skip_empty(sandbox: Sandbox) {
    let path = sandbox.file(TEST_BLANK_LINE_CONTENT);
    let reader = sandbox.reader();
    let kept = items(reader.lines(&path).skip(1).build().unwrap());
    let filtered = items(reader.lines(&path).skip(1).skip_empty(true).build().unwrap());

    assert_eq!(kept, ["", "line-2"]);
    assert_eq!(filtered, ["line-2"]);
}

#[rstest]
fn skip_stops_at_a_truncation(sandbox: Sandbox) {
    let path = sandbox.file(TEST_SEGMENT_CONTENT);
    let reader = sandbox.reader_with(Config {
        verify_state: false,
        auto_load_state: true,
        ..sandbox.config()
    });

    sandbox.saved(&path, TEST_STATE_NAME);
    sandbox.truncate(&path, 8);

    let mut segments = reader.delimiter(&path).state(TEST_STATE_NAME).skip(3).build().unwrap();

    assert!(segments.read().unwrap().is_none());
}

#[rstest]
fn every_byte_value_survives_a_round_trip(sandbox: Sandbox) {
    let content: Vec<u8> = (0..=255).collect();
    let path = sandbox.file(&content);

    for iterator in LOSSLESS_ITERATORS {
        let (read, state) = iterator.read(&sandbox.reader(), &path, Plan::default()).unwrap();

        assert_eq!(read.concat(), content, "{iterator:?}");
        assert_eq!(state.position, content.len() as u64, "{iterator:?}");
    }
}

#[rstest]
#[case::corrupt("not valid json")]
#[case::incomplete("{}")]
#[case::empty("")]
fn auto_load_ignores_an_unreadable_payload(sandbox: Sandbox, #[case] payload: &str) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.resuming();
    let mut bytes = reader.bytes(&path).build().unwrap();

    bytes.read().unwrap();

    let state_path = bytes.state().save().unwrap();

    drop(bytes);
    fs::write(&state_path, payload).unwrap();

    assert_eq!(reader.bytes(&path).build().unwrap().state().position, 0);
}

#[rstest]
fn auto_load_ignores_an_unverifiable_state(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.resuming();
    let mut bytes = reader.bytes(&path).build().unwrap();

    bytes.read().unwrap();
    bytes.state().save().unwrap();
    drop(bytes);

    sandbox.append(&path, b"more");

    assert_eq!(reader.bytes(&path).build().unwrap().state().position, 0);
}

#[rstest]
fn auto_load_resumes_stale_state_without_verification(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.reader_with(Config {
        auto_load_state: true,
        verify_state: false,
        ..sandbox.config()
    });
    let mut bytes = reader.bytes(&path).build().unwrap();

    bytes.read().unwrap();
    bytes.state().save().unwrap();
    drop(bytes);

    sandbox.append(&path, b"more");

    assert_eq!(reader.bytes(&path).build().unwrap().state().position, 1);
}

#[rstest]
fn state_object_keeps_its_own_state_dir(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let mut owner = sandbox.autosaving(0).bytes(&path).state(TEST_STATE_NAME).build().unwrap();
    let borrowed = owner.state().clone();
    let elsewhere = sandbox.path().join("other-states");
    let borrower = PReader::from(Config {
        state_dir: elsewhere.clone(),
        auto_save_state: true,
        verify_state: false,
        ..sandbox.config()
    });

    drop(owner);

    for byte in borrower.bytes(&path).state(borrowed).build().unwrap() {
        byte.unwrap();
    }

    let name = format!("{TEST_STATE_NAME}{STATE_FILE_EXTENSION}");

    assert!(sandbox.state_dir().join(&name).is_file());
    assert!(!elsewhere.join(&name).exists());
}

#[rstest]
fn changing_state_name_leaves_old_state_in_place(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);

    sandbox.saved(&path, TEST_STATE_NAME);

    let mut fresh = sandbox.reader().bytes(&path).state(TEST_OTHER_STATE_NAME).build().unwrap();

    assert_eq!(fresh.state().position, 0);
    assert!(sandbox.states().exists(TEST_STATE_NAME));
}

#[rstest]
fn changing_state_dir_creates_fresh_state(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let mut bytes = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    bytes.read().unwrap();
    bytes.state().save().unwrap();
    drop(bytes);

    let elsewhere = sandbox.reader_in(
        "other-states",
        Config {
            auto_load_state: true,
            ..sandbox.config()
        },
    );

    assert_eq!(
        elsewhere.bytes(&path).state(TEST_STATE_NAME).build().unwrap().state().position,
        0
    );
}

#[rstest]
fn autoname_changes_when_the_file_moves(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.resuming();
    let mut bytes = reader.bytes(&path).build().unwrap();

    bytes.read().unwrap();
    bytes.state().save().unwrap();
    drop(bytes);

    let moved = sandbox.path().join("moved.bin");

    fs::rename(&path, &moved).unwrap();

    assert_eq!(reader.bytes(&moved).build().unwrap().state().position, 0);
}

#[cfg(unix)]
#[rstest]
fn symlinks_to_one_target_share_the_autoname(sandbox: Sandbox) {
    use std::os::unix::fs::symlink;

    let real = sandbox.write("real.bin", TEST_ALPHABET);
    let (first, second) = (
        sandbox.path().join("link-1.bin"),
        sandbox.path().join("link-2.bin"),
    );

    symlink(&real, &first).unwrap();
    symlink(&real, &second).unwrap();

    let reader = sandbox.reader();

    assert_eq!(
        reader.bytes(&first).build().unwrap().state().name,
        reader.bytes(&second).build().unwrap().state().name
    );
}

#[rstest]
fn deleted_tracked_file_fails_the_build(sandbox: Sandbox) {
    let tracked = sandbox.write("tracked.bin", TEST_ALPHABET);
    let untracked = sandbox.write("untracked.bin", TEST_ALPHABET);
    let reader = sandbox.lenient();
    let state = sandbox.named_state(&tracked, TEST_STATE_NAME);

    fs::remove_file(&tracked).unwrap();

    let error = reader.bytes(&untracked).state(state).build().unwrap_err();

    assert!(
        matches!(&error, Error::Io(io) if io.kind() == std::io::ErrorKind::NotFound),
        "{error}"
    );
}

#[rstest]
fn file_replaced_by_directory_fails_read(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.lenient();
    let state = sandbox.named_state(&path, TEST_STATE_NAME);

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
fn save_error_stops_read_after_truncation(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.reader_with(Config {
        auto_save_state: true,
        buffer_capacity: 1,
        ..sandbox.config()
    });
    let mut bytes = reader.bytes(&path).state(TEST_UNSAFE_NAME).build().unwrap();

    bytes.read().unwrap();
    sandbox.truncate(&path, 1);

    let error = bytes.find_map(Result::err).unwrap();

    assert!(error.to_string().contains("path escapes root"), "{error}");
}

#[rstest]
fn save_error_stops_read_on_skipped_item(sandbox: Sandbox) {
    let path = sandbox.file(TEST_LINE_CONTENT);
    let reader = sandbox.autosaving(1);
    let mut lines = reader.lines(&path).state(TEST_UNSAFE_NAME).skip(2).build().unwrap();
    let error = lines.read().unwrap_err();

    assert!(error.to_string().contains("path escapes root"), "{error}");
    assert_eq!(lines.state().position, 7);
}

#[rstest]
fn save_error_stops_read_on_filtered_blank(sandbox: Sandbox) {
    let path = sandbox.file(b"\nfoo\n");
    let reader = sandbox.autosaving(1);
    let mut lines = reader.lines(&path).state(TEST_UNSAFE_NAME).skip_empty(true).build().unwrap();
    let error = lines.read().unwrap_err();

    assert!(error.to_string().contains("path escapes root"), "{error}");
    assert_eq!(lines.state().position, 1);
}

#[rstest]
fn save_error_stops_read_when_end_drops_chunk(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.autosaving(0);
    let mut chunks = reader
        .chunks(&path)
        .state(TEST_UNSAFE_NAME)
        .size(8)
        .drop_partial(true)
        .end(12)
        .build()
        .unwrap();
    let error = chunks.find_map(Result::err).unwrap();

    assert!(error.to_string().contains("path escapes root"), "{error}");
}
