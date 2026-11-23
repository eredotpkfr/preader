use std::{assert_matches, fs, io::ErrorKind, iter, ops::Range, time::Duration};
#[cfg(unix)]
use std::{
    env,
    os::unix::fs::{MetadataExt, symlink},
    path::{Component, PathBuf},
};

use preader::{
    Config, DEFAULT_AUTO_SAVE_STATE_BYTES, DEFAULT_BUFFER_CAPACITY, Error, IteratorOptions,
    IteratorRead, Mismatch, NameError, State,
};
use rstest::rstest;
use rstest_reuse::apply;
use serde_json::{Value, json};

use crate::common::{
    constants::{
        TEST_ALPHABET, TEST_FILE_NAME, TEST_LINE, TEST_LINE_CONTENT, TEST_OTHER_STATE_NAME,
        TEST_STATE_FILE, TEST_STATE_FILE_EXTENSION, TEST_STATE_NAME, TEST_TRACKED_NAME,
        TEST_UNTRACKED_NAME,
    },
    fixtures::sandbox,
    funcs::{canonical, consume, digest, drain, items, names, range, state_data, take, try_items},
    guards::{mtime, set_mtime},
    iterators::{
        IteratorKind::{self, Bytes, Chunks, Lines, Segments},
        Plan,
    },
    macros::asserts::assert_err_is,
    sandbox::Sandbox,
    templates::{
        name::{alias_names, invalid_names, longest_names, valid_names},
        payload::malformed_payloads,
    },
};
#[cfg(unix)]
use crate::common::{
    constants::{TEST_READ_FROM, TEST_REWOUND_TO},
    funcs::state_file,
    guards::Blocked,
};

const CONTENT_SIZE: u64 = TEST_ALPHABET.len() as u64;

fn shared_content() -> Vec<u8> {
    (0..4096).flat_map(|index| format!("{index},\n").into_bytes()).collect()
}

fn named(name: &str) -> Plan {
    Plan {
        state: name.into(),
        ..Plan::default()
    }
}

fn resumed(state: State) -> Plan {
    Plan {
        state: state.into(),
        ..Plan::default()
    }
}

#[apply(invalid_names)]
fn fails_when_the_state_name_is_invalid(
    sandbox: Sandbox,
    #[case] name: String,
    #[case] expected: NameError,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    assert_err_is!(
        iterator.open(&sandbox.reader(), &sandbox.line_file(), named(&name)),
        Error::Name(error) if *error == expected
    );
}

#[rstest]
#[case::small(range(10, 5))]
#[case::maximum(range(u64::MAX, 0))]
#[case::one_apart(range(1, 0))]
fn fails_when_the_start_exceeds_the_end(
    sandbox: Sandbox,
    #[case] options: IteratorOptions,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let plan = Plan {
        options,
        ..Plan::default()
    };

    assert_err_is!(
        iterator.open(&sandbox.reader(), &sandbox.line_file(), plan),
        Error::InvalidRange
    );
}

#[rstest]
#[case::empty_file(b"")]
#[case::unread_file(b"foo")]
fn percent_is_zero_before_reading(
    sandbox: Sandbox,
    #[case] content: &[u8],
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let opened = iterator
        .open(&sandbox.reader(), &sandbox.file(content), Plan::default())
        .unwrap();

    assert_eq!(opened.percent(), 0.0);
}

#[rstest]
fn build_fails_when_the_name_has_a_nul_byte(sandbox: Sandbox) {
    let path = sandbox.line_file();

    assert_err_is!(
        sandbox.reader().bytes(&path).state("job\0").build(),
        Error::Name(NameError::Invalid)
    );
}

#[rstest]
fn yields_one_item_for_a_single_byte_file(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let opened = iterator.open(&sandbox.reader(), &sandbox.file(b"a"), Plan::default()).unwrap();

    assert_eq!(items(opened), [b"a"]);
}

#[rstest]
#[case::start_past_the_end(IteratorOptions { start: 1000, ..IteratorOptions::default() })]
#[case::end_zero(IteratorOptions { end: 0, ..IteratorOptions::default() })]
#[case::limit_zero(IteratorOptions { limit: 0, ..IteratorOptions::default() })]
#[case::skip_past_the_end(IteratorOptions { skip: 1000, ..IteratorOptions::default() })]
#[case::maximum_start_and_end(range(u64::MAX, u64::MAX))]
fn boundary_options_yield_nothing(
    sandbox: Sandbox,
    #[case] options: IteratorOptions,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let plan = Plan {
        options,
        ..Plan::default()
    };
    let opened = iterator.open(&sandbox.reader(), &sandbox.line_file(), plan).unwrap();

    assert!(items(opened).is_empty());
}

#[rstest]
fn second_iteration_yields_nothing(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let mut opened = iterator
        .open(
            &sandbox.reader(),
            &sandbox.file(&shared_content()),
            Plan::default(),
        )
        .unwrap();

    drain(&mut opened);

    assert!(items(opened).is_empty());
}

#[apply(valid_names)]
fn saves_a_state_under_a_valid_name(
    sandbox: Sandbox,
    #[case] name: String,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let mut opened = iterator.open(&sandbox.reader(), &sandbox.line_file(), named(&name)).unwrap();

    opened.save().unwrap();

    assert_eq!(names(&sandbox.states()), [name]);
}

#[rstest]
fn draining_writes_nothing_by_default(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let mut bytes = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    drain(&mut bytes);

    assert!(!sandbox.states().exists(TEST_STATE_NAME));
}

#[rstest]
fn fails_when_the_file_is_missing(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let missing = sandbox.path().join("missing.bin");

    assert_err_is!(
        iterator.open(&sandbox.reader(), &missing, Plan::default()),
        Error::Io(error) if error.kind() == ErrorKind::NotFound
    );
}

#[rstest]
fn yields_nothing_for_an_empty_file(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let mut opened = iterator
        .open(&sandbox.reader(), &sandbox.empty_file(), Plan::default())
        .unwrap();

    assert!(items(opened.by_ref()).is_empty());

    assert_eq!(opened.state().position, 0);
}

#[rstest]
fn drop_does_not_save_without_progress(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let reader = sandbox.autosaving(DEFAULT_AUTO_SAVE_STATE_BYTES.get());

    drop(iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap());

    assert!(!sandbox.states().exists(TEST_STATE_NAME));
}

#[rstest]
#[case::unicode("café-日本語.bin")]
#[case::special_characters("foo bar; $baz & `qux`.bin")]
fn reads_an_unusual_path(
    sandbox: Sandbox,
    #[case] name: &str,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.write(name, b"foo");
    let mut opened = iterator.open(&sandbox.reader(), &path, Plan::default()).unwrap();

    drain(&mut opened);

    assert_eq!(opened.state().position, 3);
}

#[rstest]
fn drop_does_not_save_by_default(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let mut opened = iterator.open(&sandbox.reader(), &path, named(TEST_STATE_NAME)).unwrap();

    consume(&mut opened, 2);
    drop(opened);

    assert!(!sandbox.states().exists(TEST_STATE_NAME));
}

#[rstest]
#[case::spaces("a b.bin")]
#[case::quote("a'b.bin")]
#[case::dollar("a$b.bin")]
#[case::accents("café.bin")]
#[case::ideographs("日本語.bin")]
#[case::semicolon("a;b.bin")]
fn unusual_path_is_read_verbatim(sandbox: Sandbox, #[case] name: &str) {
    let path = sandbox.write(name, TEST_LINE);
    let mut bytes = sandbox.reader().bytes(&path).build().unwrap();

    assert_eq!(items(bytes.by_ref()), TEST_LINE);
    assert_eq!(bytes.state().position, TEST_LINE.len() as u64);
}

#[rstest]
fn resumed_read_continues_where_it_stopped(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.reader();

    sandbox.checkpoint(&path, TEST_STATE_NAME, 4);

    let resumed = items(reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap());

    assert_eq!(resumed, &TEST_ALPHABET[4..]);
}

#[rstest]
fn recorded_size_never_refreshes(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let mut bytes = sandbox.reader().bytes(&path).build().unwrap();

    sandbox.append(&path, b"more");
    bytes.read().unwrap();

    assert_eq!(bytes.state().file.size, CONTENT_SIZE);
}

#[rstest]
fn resumed_read_applies_the_limit_again(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.reader();

    sandbox.checkpoint(&path, TEST_STATE_NAME, 3);

    let second = items(reader.bytes(&path).state(TEST_STATE_NAME).limit(3).build().unwrap());

    assert_eq!(second, b"def");
}

#[apply(malformed_payloads)]
fn fails_when_a_named_state_is_corrupt(
    sandbox: Sandbox,
    #[case] payload: &str,
    #[case] message: &str,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.line_file();
    let reader = sandbox.reader();

    sandbox.write_state(TEST_STATE_NAME, payload);

    assert_err_is!(
        iterator.open(&reader, &path, named(TEST_STATE_NAME)),
        Error::Corrupt { name, error }
            if name == TEST_STATE_NAME && error.to_string().contains(message)
    );
}

#[rstest]
fn manual_save_without_progress_records_the_start(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let mut bytes = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).start(5).build().unwrap();

    bytes.save().unwrap();

    assert_eq!(sandbox.states().load(TEST_STATE_NAME).unwrap().position, 5);
}

#[rstest]
#[case::short("foo.bin".to_owned())]
#[case::long(format!("{}.bin", "f".repeat(180)))]
fn names_the_state_by_the_path_digest(
    sandbox: Sandbox,
    #[case] name: String,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.write(&name, b"foo");
    let expected = digest(canonical(&path).as_os_str().as_encoded_bytes());
    let opened = iterator.open(&sandbox.reader(), &path, Plan::default()).unwrap();

    assert_eq!(opened.state().name, expected);
}

#[rstest]
fn resumed_read_ignores_start_it_is_already_past(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.reader();

    sandbox.checkpoint(&path, TEST_STATE_NAME, 5);

    let second = take(
        &mut reader.bytes(&path).state(TEST_STATE_NAME).start(2).build().unwrap(),
        1,
    );

    assert_eq!(second, b"f");
}

#[rstest]
fn manual_save_records_the_finished_position(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let mut bytes = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    consume(&mut bytes, 4);

    bytes.save().unwrap();

    assert_eq!(sandbox.states().load(TEST_STATE_NAME).unwrap().position, 3);
}

#[rstest]
fn fully_consumed_file_yields_nothing_on_resume(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);

    sandbox.checkpoint(&path, TEST_STATE_NAME, CONTENT_SIZE);

    let mut second = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    assert!(second.read().unwrap().is_none());

    assert_eq!(second.state().position, CONTENT_SIZE);
}

#[rstest]
fn save_returns_the_saved_state(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let mut opened = iterator.open(&sandbox.reader(), &path, named(TEST_STATE_NAME)).unwrap();

    consume(&mut opened, 2);

    let saved = opened.save().unwrap();

    assert_eq!(saved, &sandbox.states().load(TEST_STATE_NAME).unwrap());
}

#[rstest]
fn dropping_reports_failed_save_without_panicking(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.autosaving(DEFAULT_AUTO_SAVE_STATE_BYTES.get());
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    consume(&mut bytes, 3);

    sandbox.block_states();

    drop(bytes);

    assert!(!sandbox.states().exists(TEST_STATE_NAME));
}

#[rstest]
fn drop_does_not_panic_when_saving_fails(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());

    sandbox.block_states();

    let reader = sandbox.autosaving(DEFAULT_AUTO_SAVE_STATE_BYTES.get());
    let mut opened = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();

    consume(&mut opened, 2);
    drop(opened);

    assert!(!sandbox.states().exists(TEST_STATE_NAME));
}

#[rstest]
fn unnormalized_path_resolves_to_the_same_name(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let detoured = sandbox.path().join(".").join(TEST_FILE_NAME);
    let reader = sandbox.reader();

    assert_eq!(
        reader.bytes(&path).build().unwrap().state().name,
        reader.bytes(&detoured).build().unwrap().state().name
    );
}

#[rstest]
fn starts_fresh_after_a_corrupt_state_is_deleted(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let reader = sandbox.reader();

    sandbox.write_state(TEST_STATE_NAME, "not valid json");

    sandbox.states().delete(TEST_STATE_NAME).unwrap();

    let fresh = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();

    assert_eq!(fresh.state().position, 0);
}

#[cfg(unix)]
#[rstest]
fn follows_a_symlink(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let target = sandbox.line_file();
    let link = sandbox.path().join("link.bin");

    symlink(&target, &link).unwrap();

    let opened = iterator.open(&sandbox.reader(), &link, Plan::default()).unwrap();

    assert_eq!(opened.state().file.path, canonical(&target));
}

#[rstest]
fn counts_invalid_bytes(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let content = b"foo\n\xff\xfe\nbar\n";
    let path = sandbox.file(content);
    let mut opened = iterator.open(&sandbox.reader(), &path, Plan::default()).unwrap();

    opened.by_ref().for_each(drop);

    assert_eq!(opened.state().position, content.len() as u64);
}

#[rstest]
#[case::single_byte(1)]
#[case::tiny(2)]
#[case::uneven(3)]
#[case::large(DEFAULT_BUFFER_CAPACITY)]
fn buffer_capacity_does_not_change_the_lossless_output(
    sandbox: Sandbox,
    #[case] capacity: usize,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(TEST_LINE_CONTENT);
    let capped = sandbox.capped(capacity);
    let plain = sandbox.reader();
    let expected = iterator.read(&plain, &path, Plan::default()).unwrap().0;
    let found = iterator.read(&capped, &path, Plan::default()).unwrap().0;

    assert_eq!(found, expected);
}

#[rstest]
fn end_below_position_does_not_rewind_state(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.reader();

    sandbox.checkpoint(&path, TEST_STATE_NAME, 10);

    let mut second = reader.bytes(&path).state(TEST_STATE_NAME).end(4).build().unwrap();

    assert!(second.read().unwrap().is_none());

    assert_eq!(second.state().position, 10);
}

#[rstest]
fn existing_state_name_is_checked_only_on_save(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.line_file();
    let mut data = state_data(canonical(&path));

    data.name = "../job-1".to_owned();

    let plan = resumed(data);
    let mut opened = iterator.open(&sandbox.lenient(), &path, plan).unwrap();

    assert_err_is!(opened.save(), Error::Name(NameError::Escapes));
}

#[rstest]
fn fails_when_a_named_state_was_tampered(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.line_file();
    let reader = sandbox.reader();

    iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap().save().unwrap();
    sandbox.tamper(TEST_STATE_NAME);

    assert_err_is!(
        iterator.open(&reader, &path, named(TEST_STATE_NAME)),
        Error::Mismatch {
            name,
            kind: Mismatch::Checksum,
        } if name == TEST_STATE_NAME
    );
}

#[rstest]
fn auto_load_state_ignores_an_existing_state_by_default(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.line_file();
    let reader = sandbox.reader();
    let mut opened = iterator.open(&reader, &path, Plan::default()).unwrap();

    drain(&mut opened);

    opened.save().unwrap();

    assert_eq!(
        iterator.open(&reader, &path, Plan::default()).unwrap().state().position,
        0
    );
}

#[rstest]
fn failed_save_of_a_named_state_keeps_reading(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let mut bytes = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    sandbox.block_states();

    assert_err_is!(bytes.save(), Error::Io(error) if error.kind() == ErrorKind::AlreadyExists);

    assert_eq!(bytes.read().unwrap(), Some(TEST_ALPHABET[0]));
}

#[rstest]
fn save_succeeds_after_the_file_is_deleted(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.line_file();
    let mut opened = iterator.open(&sandbox.reader(), &path, named(TEST_STATE_NAME)).unwrap();

    fs::remove_file(&path).unwrap();

    let saved = opened.save().unwrap().clone();

    assert!(sandbox.states().path(TEST_STATE_NAME).unwrap().is_file());

    sandbox.payload(&saved);
}

#[apply(longest_names)]
fn save_round_trips_the_longest_portable_name(sandbox: Sandbox, #[case] name: String) {
    let path = sandbox.line_file();
    let mut bytes = sandbox.reader().bytes(&path).state(name.as_str()).build().unwrap();

    bytes.save().unwrap();

    assert_eq!(names(&sandbox.states()), [name.as_str()]);
    assert_eq!(sandbox.states().load(&name).unwrap().name, name);
}

#[rstest]
fn late_final_save_flushes_the_tail(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.autosaving(1000);
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    consume(&mut bytes, 3);

    assert!(!sandbox.states().exists(TEST_STATE_NAME));

    drain(&mut bytes);

    assert_eq!(
        sandbox.states().load(TEST_STATE_NAME).unwrap().position,
        CONTENT_SIZE
    );
}

#[rstest]
fn fails_when_a_named_state_tracks_a_grown_file(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.line_file();
    let reader = sandbox.reader();

    iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap().save().unwrap();
    sandbox.append(&path, b"more");

    assert_err_is!(
        iterator.open(&reader, &path, named(TEST_STATE_NAME)),
        Error::Mismatch {
            name,
            kind: Mismatch::Size,
        } if name == TEST_STATE_NAME
    );
}

#[rstest]
fn default_threshold_saves_only_at_the_end(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.autosaving(DEFAULT_AUTO_SAVE_STATE_BYTES.get());
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    consume(&mut bytes, 5);

    assert!(!sandbox.states().exists(TEST_STATE_NAME));

    drain(&mut bytes);

    assert_eq!(
        sandbox.states().load(TEST_STATE_NAME).unwrap().position,
        CONTENT_SIZE
    );
}

#[cfg(unix)]
#[rstest]
fn fails_when_the_named_state_is_a_dangling_symlink(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let link = sandbox.state_dir().join(TEST_STATE_FILE);

    fs::create_dir_all(sandbox.state_dir()).unwrap();

    symlink(sandbox.path().join("missing.json"), &link).unwrap();

    assert_err_is!(
        iterator.open(&sandbox.reader(), &sandbox.line_file(), named(TEST_STATE_NAME)),
        Error::Alias { name } if name == TEST_STATE_NAME
    );

    assert!(link.is_symlink());
}

#[rstest]
fn fails_when_a_named_state_tracks_a_touched_file(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.line_file();
    let reader = sandbox.reader();

    iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap().save().unwrap();

    set_mtime(&path, mtime(&path) + Duration::from_secs(3600));

    assert_err_is!(
        iterator.open(&reader, &path, named(TEST_STATE_NAME)),
        Error::Mismatch {
            name,
            kind: Mismatch::Mtime,
        } if name == TEST_STATE_NAME
    );
}

#[rstest]
fn save_persists_across_readers(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.line_file();
    let mut opened = iterator.open(&sandbox.reader(), &path, named(TEST_STATE_NAME)).unwrap();

    drain(&mut opened);

    let saved = opened.save().unwrap().clone();

    let loaded = sandbox.reader().states().load(TEST_STATE_NAME).unwrap();

    assert_eq!(loaded, saved);
    assert_eq!(loaded.position, TEST_LINE.len() as u64);
}

#[rstest]
fn fails_when_a_named_state_tracks_a_file_with_other_content(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let tracked = sandbox.write(TEST_TRACKED_NAME, b"foo");
    let untracked = sandbox.write(TEST_UNTRACKED_NAME, b"bar");
    let reader = sandbox.reader();

    iterator
        .open(&reader, &tracked, named(TEST_STATE_NAME))
        .unwrap()
        .save()
        .unwrap();

    assert_err_is!(
        iterator.open(&reader, &untracked, named(TEST_STATE_NAME)),
        Error::Mismatch {
            name,
            kind: Mismatch::Identity,
        } if name == TEST_STATE_NAME
    );
}

#[rstest]
fn changing_state_dir_creates_a_fresh_state(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let mut bytes = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    consume(&mut bytes, 2);

    bytes.save().unwrap();

    let elsewhere = sandbox.reader_in("elsewhere", sandbox.config());

    assert_eq!(
        elsewhere.bytes(&path).state(TEST_STATE_NAME).build().unwrap().state().position,
        0
    );
}

#[apply(malformed_payloads)]
fn auto_load_state_ignores_a_corrupt_state_by_default(
    sandbox: Sandbox,
    #[case] payload: &str,
    #[case] _message: &str,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let reader = sandbox.reader();
    let name = iterator
        .open(&reader, &path, Plan::default())
        .unwrap()
        .save()
        .unwrap()
        .name
        .clone();

    sandbox.write_state(&name, payload);

    let opened = iterator.open(&reader, &path, Plan::default()).unwrap();

    assert_eq!(opened.state().position, 0);
}

#[rstest]
fn resumes_a_named_state_by_default(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let reader = sandbox.reader();
    let mut opened = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();

    consume(&mut opened, 3);

    let saved = opened.save().unwrap().clone();
    let resumed = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();

    assert_eq!(resumed.state().position, saved.position);
}

#[rstest]
fn fails_when_the_state_tracks_a_file_with_other_content(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let tracked = sandbox.write(TEST_TRACKED_NAME, b"foo");
    let untracked = sandbox.write(TEST_UNTRACKED_NAME, b"bar");
    let reader = sandbox.reader();
    let state = iterator
        .open(&reader, &tracked, named(TEST_STATE_NAME))
        .unwrap()
        .state()
        .clone();

    assert_err_is!(
        iterator.open(&reader, &untracked, resumed(state)),
        Error::Mismatch {
            name,
            kind: Mismatch::Identity,
        } if name == TEST_STATE_NAME
    );
}

#[rstest]
fn fails_when_the_tracked_file_is_deleted(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let reader = sandbox.reader();
    let state = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap().state().clone();

    fs::remove_file(&path).unwrap();

    let plan = resumed(state);

    assert_err_is!(
        iterator.open(&reader, &path, plan),
        Error::Io(error) if error.kind() == ErrorKind::NotFound
    );
}

#[rstest]
fn save_refreshes_the_checksum(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let mut opened = iterator.open(&sandbox.reader(), &path, named(TEST_STATE_NAME)).unwrap();
    let before = sandbox.payload(opened.save().unwrap())["_checksum"].clone();

    consume(&mut opened, 5);

    let after = opened.save().unwrap().clone();

    assert_ne!(sandbox.payload(&after)["_checksum"], before);
}

#[rstest]
fn clearing_registry_does_not_affect_a_live_iterator(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.autosaving(1);
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    consume(&mut bytes, 2);

    sandbox.states().clear().unwrap();

    assert_eq!(bytes.read().unwrap(), Some(TEST_ALPHABET[2]));
    assert_eq!(sandbox.states().load(TEST_STATE_NAME).unwrap().position, 2);
}

#[rstest]
fn auto_load_state_resumes_previous_position(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let reader = sandbox.resuming();
    let mut opened = iterator.open(&reader, &path, Plan::default()).unwrap();

    consume(&mut opened, 2);

    let saved = opened.save().unwrap().clone();
    let resumed = iterator.open(&reader, &path, Plan::default()).unwrap();

    assert_eq!(resumed.state().position, saved.position);
}

#[rstest]
fn default_threshold_saves_only_on_drop(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let reader = sandbox.autosaving(DEFAULT_AUTO_SAVE_STATE_BYTES.get());
    let mut opened = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();

    consume(&mut opened, 3);

    let position = opened.state().position;

    assert!(!sandbox.states().exists(TEST_STATE_NAME));

    drop(opened);

    assert_eq!(
        sandbox.states().load(TEST_STATE_NAME).unwrap().position,
        position
    );
}

#[rstest]
fn treats_an_explicit_none_state_as_auto(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.line_file();
    let reader = sandbox.reader();
    let none = Plan {
        state: Option::<&str>::None.into(),
        ..Plan::default()
    };
    let explicit = iterator.open(&reader, &path, none).unwrap();
    let implicit = iterator.open(&reader, &path, Plan::default()).unwrap();

    assert_eq!(explicit.state().name, implicit.state().name);
}

#[rstest]
fn state_dir_change_creates_a_fresh_state(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let mut opened = iterator.open(&sandbox.reader(), &path, named(TEST_STATE_NAME)).unwrap();

    consume(&mut opened, 2);

    opened.save().unwrap();

    let other = sandbox.reader_in("elsewhere", sandbox.config());
    let fresh = iterator.open(&other, &path, named(TEST_STATE_NAME)).unwrap();

    assert_eq!(fresh.state().position, 0);
}

#[rstest]
fn fails_when_the_state_tracks_another_file(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let tracked = sandbox.write(TEST_TRACKED_NAME, b"foo");
    let untracked = sandbox.write(TEST_UNTRACKED_NAME, b"foo");
    let reader = sandbox.reader();
    let state = iterator
        .open(&reader, &tracked, named(TEST_STATE_NAME))
        .unwrap()
        .state()
        .clone();
    let plan = resumed(state);

    assert_err_is!(
        iterator.open(&reader, &untracked, plan),
        Error::Mismatch {
            name,
            kind: Mismatch::Path,
        } if name == TEST_STATE_NAME
    );
}

#[rstest]
fn fails_when_the_state_tracks_another_file_without_verification(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let reader = sandbox.lenient();
    let tracked = sandbox.write(TEST_TRACKED_NAME, b"foo");
    let untracked = sandbox.write(TEST_UNTRACKED_NAME, b"bar");
    let state = iterator
        .open(&reader, &tracked, named(TEST_STATE_NAME))
        .unwrap()
        .state()
        .clone();
    let plan = resumed(state);

    assert_err_is!(
        iterator.open(&reader, &untracked, plan),
        Error::Mismatch {
            name,
            kind: Mismatch::Path,
        } if name == TEST_STATE_NAME
    );
}

#[rstest]
fn fails_when_the_state_was_loaded_from_a_tampered_file(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.line_file();
    let reader = sandbox.reader();

    iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap().save().unwrap();
    sandbox.tamper(TEST_STATE_NAME);

    let plan = resumed(sandbox.lenient().states().load(TEST_STATE_NAME).unwrap());

    assert_err_is!(
        iterator.open(&reader, &path, plan),
        Error::Mismatch {
            name,
            kind: Mismatch::Unsealed,
        } if name == TEST_STATE_NAME
    );
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

    consume(&mut bytes, 2);

    bytes.save().unwrap();

    sandbox.append(&path, b"more");

    assert_eq!(reader.bytes(&path).build().unwrap().state().position, 1);
}

#[rstest]
fn auto_load_state_fails_when_the_state_tracks_a_grown_file(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let reader = sandbox.resuming();
    let saved = iterator.open(&reader, &path, Plan::default()).unwrap().save().unwrap().clone();

    sandbox.append(&path, b"more");

    assert_err_is!(
        iterator.open(&reader, &path, Plan::default()),
        Error::Mismatch {
            name,
            kind: Mismatch::Size,
        } if *name == saved.name
    );
}

#[rstest]
fn accepts_an_unsaved_state_without_verification(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let reader = sandbox.lenient();
    let mut opened = iterator.open(&reader, &path, Plan::default()).unwrap();

    consume(&mut opened, 3);

    let state = opened.state().clone();
    let plan = resumed(state.clone());
    let resumed = iterator.open(&reader, &path, plan).unwrap();

    assert_eq!(resumed.state().position, state.position);
}

#[rstest]
fn manual_save_does_not_reset_autosave_baseline(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.autosaving(10);
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    consume(&mut bytes, 5);

    bytes.save().unwrap();

    assert_eq!(sandbox.states().load(TEST_STATE_NAME).unwrap().position, 4);

    consume(&mut bytes, 6);

    assert_eq!(sandbox.states().load(TEST_STATE_NAME).unwrap().position, 10);
}

#[rstest]
fn fails_when_a_named_state_tracks_a_moved_file(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.line_file();
    let moved = sandbox.path().join("moved.bin");
    let reader = sandbox.reader();

    iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap().save().unwrap();

    fs::rename(&path, &moved).unwrap();

    assert_err_is!(
        iterator.open(&reader, &moved, named(TEST_STATE_NAME)),
        Error::Mismatch {
            name,
            kind: Mismatch::Path,
        } if name == TEST_STATE_NAME
    );
}

#[rstest]
fn save_recreates_a_state_deleted_meanwhile(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let mut opened = iterator.open(&sandbox.reader(), &path, named(TEST_STATE_NAME)).unwrap();

    consume(&mut opened, 3);

    opened.save().unwrap();
    sandbox.states().delete(TEST_STATE_NAME).unwrap();

    consume(&mut opened, 2);

    let saved = opened.save().unwrap().clone();

    assert_eq!(sandbox.states().load(TEST_STATE_NAME).unwrap(), saved);
}

#[rstest]
fn fails_when_the_payload_names_another_state(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.line_file();
    let reader = sandbox.reader();

    iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap().save().unwrap();

    sandbox.patch(TEST_STATE_NAME, |payload| {
        payload["name"] = Value::from(TEST_OTHER_STATE_NAME)
    });

    assert_err_is!(
        iterator.open(&reader, &path, named(TEST_STATE_NAME)),
        Error::Mismatch {
            name,
            kind: Mismatch::Name { saved },
        } if name == TEST_STATE_NAME && saved == TEST_OTHER_STATE_NAME
    );
}

#[apply(malformed_payloads)]
fn auto_load_state_fails_when_the_state_is_corrupt(
    sandbox: Sandbox,
    #[case] payload: &str,
    #[case] message: &str,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let reader = sandbox.resuming();
    let saved = iterator.open(&reader, &path, Plan::default()).unwrap().save().unwrap().clone();

    sandbox.write_state(&saved.name, payload);

    assert_err_is!(
        iterator.open(&reader, &path, Plan::default()),
        Error::Corrupt { name, error } if *name == saved.name && error.to_string().contains(message)
    );
}

#[rstest]
fn resumes_a_stale_named_state_without_verification(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let reader = sandbox.lenient();
    let mut opened = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();

    consume(&mut opened, 3);

    let saved = opened.save().unwrap().clone();

    sandbox.append(&path, b"more");

    let resumed = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();

    assert_eq!(resumed.state().position, saved.position);
}

#[rstest]
fn fails_when_the_state_tracks_a_grown_file(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let reader = sandbox.reader();
    let mut opened = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();

    drain(&mut opened);

    let state = opened.save().unwrap().clone();

    sandbox.append(&path, b"more");

    assert_err_is!(
        iterator.open(&reader, &path, resumed(state)),
        Error::Mismatch {
            name,
            kind: Mismatch::Size,
        } if name == TEST_STATE_NAME
    );
}

#[rstest]
fn fails_when_the_resumed_file_is_replaced_by_a_directory(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let reader = sandbox.reader();
    let state = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap().state().clone();

    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();

    let plan = resumed(state);

    assert_err_is!(
        iterator.open(&reader, &path, plan),
        Error::Io(error) if error.kind() == ErrorKind::IsADirectory
    );
}

#[rstest]
fn save_syncs_the_state_object_with_the_file(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let mut opened = iterator.open(&sandbox.reader(), &path, named(TEST_STATE_NAME)).unwrap();

    consume(&mut opened, 13);

    let position = opened.state().position;

    opened.save().unwrap();

    let state = opened.state();
    let payload = sandbox.payload(state);

    assert_eq!(payload["position"], position);
    assert_eq!(&sandbox.states().load(TEST_STATE_NAME).unwrap(), state);
}

#[rstest]
fn fails_when_a_tampered_state_tracks_a_moved_file(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.line_file();
    let moved = sandbox.path().join("moved.bin");
    let reader = sandbox.reader();

    iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap().save().unwrap();
    sandbox.tamper(TEST_STATE_NAME);

    fs::rename(&path, &moved).unwrap();

    assert_err_is!(
        iterator.open(&reader, &moved, named(TEST_STATE_NAME)),
        Error::Mismatch {
            name,
            kind: Mismatch::Checksum,
        } if name == TEST_STATE_NAME
    );
}

#[rstest]
fn resume_skips_again_when_position_equals_start(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.reader();
    let mut first = reader.bytes(&path).state(TEST_STATE_NAME).start(2).limit(1).build().unwrap();

    drain(&mut first);

    first.save().unwrap();

    let mut resumed = reader.bytes(&path).state(TEST_STATE_NAME).start(3).skip(1).build().unwrap();

    assert_eq!(resumed.read().unwrap(), Some(TEST_ALPHABET[4]));
}

#[rstest]
fn fails_when_the_state_is_unsaved(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let reader = sandbox.reader();
    let mut opened = iterator.open(&reader, &path, Plan::default()).unwrap();

    consume(&mut opened, 3);

    let state = opened.state().clone();
    let expected = state.name.clone();
    let plan = resumed(state);

    assert_err_is!(
        iterator.open(&reader, &path, plan),
        Error::Mismatch {
            name,
            kind: Mismatch::Unsealed,
        } if *name == expected
    );
}

#[rstest]
fn fails_when_a_resynced_state_tracks_a_file_with_other_content(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let first = sandbox.write(TEST_TRACKED_NAME, b"foo");
    let second = sandbox.write(TEST_UNTRACKED_NAME, b"bar");
    let reader = sandbox.reader();

    iterator.open(&reader, &first, named(TEST_STATE_NAME)).unwrap().save().unwrap();

    let resynced = reader.states().resync(TEST_STATE_NAME, &first).unwrap();

    assert_err_is!(
        iterator.open(&reader, &second, resumed(resynced)),
        Error::Mismatch {
            name,
            kind: Mismatch::Identity,
        } if name == TEST_STATE_NAME
    );
}

#[rstest]
fn state_name_change_creates_an_orphaned_state(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let reader = sandbox.reader();
    let mut opened = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();

    consume(&mut opened, 2);

    let saved = opened.save().unwrap().clone();
    let renamed = iterator.open(&reader, &path, named(TEST_OTHER_STATE_NAME)).unwrap();

    assert_eq!(renamed.state().position, 0);
    assert_eq!(sandbox.states().load(TEST_STATE_NAME).unwrap(), saved);
}

#[rstest]
#[case::zero(0)]
#[case::tiny(2)]
fn buffer_capacity_does_not_change_the_output(
    sandbox: Sandbox,
    #[case] capacity: usize,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let content = shared_content();
    let path = sandbox.file(&content);
    let full = items(iterator.open(&sandbox.reader(), &path, Plan::default()).unwrap());
    let mut small = iterator.open(&sandbox.capped(capacity), &path, Plan::default()).unwrap();
    let read = items(small.by_ref());

    assert!(!read.is_empty());

    assert_eq!(read, full);
    assert_eq!(small.state().position, content.len() as u64);
}

#[cfg(unix)]
#[rstest]
fn fails_when_the_state_name_escapes_through_a_symlink(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let outside = sandbox.dir_at("outside");
    let escaping = format!("link/{TEST_STATE_NAME}");

    fs::create_dir_all(sandbox.state_dir()).unwrap();

    symlink(&outside, sandbox.state_dir().join("link")).unwrap();

    assert_err_is!(
        iterator.open(&sandbox.reader(), &sandbox.line_file(), named(&escaping)),
        Error::Alias { name } if *name == escaping
    );

    assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
}

#[rstest]
fn extra_next_after_exhaustion_does_not_resave(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let content = shared_content();
    let reader = sandbox.autosaving(DEFAULT_AUTO_SAVE_STATE_BYTES.get());
    let mut opened =
        iterator.open(&reader, &sandbox.file(&content), named(TEST_STATE_NAME)).unwrap();

    drain(&mut opened);

    assert_eq!(
        sandbox.states().load(TEST_STATE_NAME).unwrap().position,
        content.len() as u64
    );

    sandbox.states().delete(TEST_STATE_NAME).unwrap();

    assert!(opened.next().is_none());
    assert!(!sandbox.states().exists(TEST_STATE_NAME));
}

#[rstest]
fn resumes_a_stale_state_without_verification(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let mut opened = iterator.open(&sandbox.reader(), &path, named(TEST_STATE_NAME)).unwrap();

    consume(&mut opened, 2);

    let state = opened.save().unwrap().clone();

    sandbox.append(&path, b"more");

    let plan = resumed(state.clone());
    let resumed = iterator.open(&sandbox.lenient(), &path, plan).unwrap();

    assert_eq!(resumed.state().position, state.position);
}

#[rstest]
fn auto_load_state_starts_fresh_when_the_file_moves(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let moved = sandbox.path().join("moved.bin");
    let reader = sandbox.resuming();
    let mut opened = iterator.open(&reader, &path, Plan::default()).unwrap();

    consume(&mut opened, 2);

    opened.save().unwrap();

    fs::rename(&path, &moved).unwrap();

    assert_eq!(
        iterator.open(&reader, &moved, Plan::default()).unwrap().state().position,
        0
    );
}

#[apply(alias_names)]
fn fails_when_the_state_name_is_a_case_or_unicode_alias(
    sandbox: Sandbox,
    #[case] name: &str,
    #[case] alias: &str,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.line_file();
    let reader = sandbox.reader();

    iterator.open(&reader, &path, named(name)).unwrap().save().unwrap();

    if !sandbox.state_dir().join(format!("{alias}{TEST_STATE_FILE_EXTENSION}")).exists() {
        return;
    }

    assert_err_is!(
        iterator.open(&reader, &path, named(alias)),
        Error::Alias { name: reported } if reported == alias
    );

    assert!(names(&sandbox.states()).contains(&name.to_owned()));
}

#[rstest]
fn save_lets_the_last_writer_win_for_a_new_name(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let reader = sandbox.reader();
    let mut first = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();
    let mut second = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();

    consume(&mut first, 4);

    first.save().unwrap();

    consume(&mut second, 2);

    let saved = second.save().unwrap().clone();

    assert_eq!(sandbox.states().load(TEST_STATE_NAME).unwrap(), saved);
}

#[cfg(unix)]
#[rstest]
fn drop_does_not_rewrite_a_state_saved_without_progress(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let reader = sandbox.autosaving(DEFAULT_AUTO_SAVE_STATE_BYTES.get());
    let saved = reader.states().path(TEST_STATE_NAME).unwrap();
    let mut opened = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();

    consume(&mut opened, 3);

    opened.save().unwrap();

    let inode = fs::metadata(&saved).unwrap().ino();

    drop(opened);

    assert_eq!(fs::metadata(&saved).unwrap().ino(), inode);
}

#[rstest]
fn failed_manual_save_keeps_reading(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.line_file();

    sandbox.block_states();

    let mut opened = iterator.open(&sandbox.reader(), &path, Plan::default()).unwrap();
    let mut fresh = iterator.open(&sandbox.reader(), &path, Plan::default()).unwrap();

    assert_err_is!(
        opened.save(),
        Error::Io(error) if error.kind() == ErrorKind::AlreadyExists
    );

    assert_eq!(
        opened.next().unwrap().unwrap(),
        fresh.next().unwrap().unwrap()
    );
}

#[rstest]
fn position_counts_finished_items(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let mut opened = iterator.open(&sandbox.reader(), &path, Plan::default()).unwrap();

    opened.next().unwrap().unwrap();

    assert_eq!(opened.state().position, 0);
    assert_eq!(opened.percent(), 0.0);

    opened.next().unwrap().unwrap();

    let finished = match iterator {
        Bytes => 1,
        Chunks => 1024,
        Lines => 3,
        Segments => 2,
    };

    assert_eq!(opened.state().position, finished);
}

#[rstest]
fn auto_load_state_resumes_a_stale_state_without_verification(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let reader = sandbox.reader_with(Config {
        auto_load_state: true,
        verify_state: false,
        ..sandbox.config()
    });
    let mut opened = iterator.open(&reader, &path, Plan::default()).unwrap();

    consume(&mut opened, 2);

    let saved = opened.save().unwrap().clone();

    sandbox.append(&path, b"more");

    assert_eq!(
        iterator.open(&reader, &path, Plan::default()).unwrap().state().position,
        saved.position
    );
}

#[rstest]
fn resume_on_a_fully_consumed_file_yields_nothing(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.line_file();
    let reader = sandbox.reader();
    let mut opened = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();

    drain(&mut opened);

    let state = opened.save().unwrap().clone();
    let mut resumed = iterator.open(&reader, &path, resumed(state)).unwrap();

    assert!(items(resumed.by_ref()).is_empty());

    assert_eq!(resumed.state().position, TEST_LINE.len() as u64);
    assert_eq!(resumed.percent(), 100.0);
}

#[rstest]
fn fails_when_the_final_save_fails(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let full = items(iterator.open(&sandbox.reader(), &path, Plan::default()).unwrap());

    sandbox.block_states();

    let saving = sandbox.autosaving(DEFAULT_AUTO_SAVE_STATE_BYTES.get());
    let mut opened = iterator.open(&saving, &path, named(TEST_STATE_NAME)).unwrap();

    assert_eq!(take(&mut opened, full.len()), full);

    assert_err_is!(
        opened.next().unwrap(),
        Error::Io(error) if error.kind() == ErrorKind::AlreadyExists
    );
}

#[rstest]
fn save_overwrites_a_state_it_did_not_load(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let reader = sandbox.reader();
    let mut first = iterator.open(&reader, &path, Plan::default()).unwrap();

    consume(&mut first, 3);

    first.save().unwrap();

    let mut second = iterator.open(&reader, &path, Plan::default()).unwrap();

    consume(&mut second, 2);

    let saved = second.save().unwrap().clone();

    assert_eq!(sandbox.states().load(&saved.name).unwrap(), saved);
}

#[rstest]
fn save_does_not_disturb_the_iterator(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let reader = sandbox.reader();
    let full = items(iterator.open(&reader, &path, Plan::default()).unwrap());
    let mut opened = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();

    consume(&mut opened, 6);

    let position = opened.state().position;

    opened.save().unwrap();

    assert_eq!(
        sandbox.states().load(TEST_STATE_NAME).unwrap().position,
        position
    );
    assert_eq!(items(opened), full[6..]);
}

#[rstest]
fn error_ignoring_loop_still_terminates(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.autosaving(1);
    let mut polls = 0;
    let mut errors = 0;

    sandbox.block_states();

    for item in reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap() {
        polls += 1;

        assert!(
            polls <= 2,
            "the iterator kept yielding after the failed save"
        );

        if let Err(error) = item {
            errors += 1;

            assert_matches!(error, Error::Io(error) if error.kind() == ErrorKind::AlreadyExists);
        }
    }

    assert_eq!(polls, 2);
    assert_eq!(errors, 1);
}

#[cfg(unix)]
#[rstest]
fn resolves_a_relative_path_to_the_same_name(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.line_file();
    let reader = sandbox.reader();
    let relative: PathBuf = canonical(&env::current_dir().unwrap())
        .components()
        .skip(1)
        .map(|_| Component::ParentDir)
        .chain(path.components().skip(1))
        .collect();

    let from_relative = iterator.open(&reader, &relative, Plan::default()).unwrap();
    let from_absolute = iterator.open(&reader, &path, Plan::default()).unwrap();

    assert_eq!(from_relative.state().name, from_absolute.state().name);
}

#[cfg(unix)]
#[rstest]
fn auto_load_state_fails_when_the_state_is_unreadable(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.resuming();
    let mut bytes = reader.bytes(&path).build().unwrap();
    let mut blocked = Blocked::default();

    bytes.read().unwrap();

    let state_path = reader.states().path(&bytes.save().unwrap().name).unwrap();

    if !Blocked::enforced(&sandbox.path().join("probe")) {
        return;
    }

    blocked.block(&state_path);

    assert_err_is!(reader.bytes(&path).build(), Error::Io(_));
}

#[rstest]
fn drop_saves_a_state_object_from_another_reader(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let mut opened = iterator.open(&sandbox.reader(), &path, named(TEST_STATE_NAME)).unwrap();

    consume(&mut opened, 2);

    let state = opened.save().unwrap().clone();
    let elsewhere = sandbox.reader_in(
        "elsewhere",
        Config {
            auto_save_state: true,
            ..sandbox.config()
        },
    );

    drop(iterator.open(&elsewhere, &path, resumed(state.clone())).unwrap());

    assert_eq!(
        elsewhere.states().load(TEST_STATE_NAME).unwrap().position,
        state.position
    );
}

#[rstest]
fn save_excludes_the_unfinished_item(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let reader = sandbox.reader();
    let mut opened = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();

    opened.next().unwrap().unwrap();

    let unfinished = opened.next().unwrap().unwrap();

    opened.save().unwrap();

    let saved = sandbox.states().load(TEST_STATE_NAME).unwrap();
    let mut resumed = iterator.open(&reader, &path, resumed(saved)).unwrap();

    assert_eq!(resumed.next().unwrap().unwrap(), unfinished);
}

#[rstest]
fn auto_load_state_fails_when_the_payload_is_incomplete(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let reader = sandbox.resuming();
    let saved = iterator.open(&reader, &path, Plan::default()).unwrap().save().unwrap().clone();

    sandbox.patch(&saved.name, |payload| {
        payload.as_object_mut().unwrap().remove("timestamps");
    });

    assert_err_is!(
        iterator.open(&reader, &path, Plan::default()),
        Error::Corrupt { name, error }
            if *name == saved.name && error.to_string().contains("missing field `timestamps`")
    );
}

#[rstest]
fn skip_beyond_the_item_count_yields_nothing(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let content = shared_content();
    let path = sandbox.file(&content);
    let reader = sandbox.reader();
    let count = items(iterator.open(&reader, &path, Plan::default()).unwrap()).len() as u64;
    let plan = Plan {
        options: IteratorOptions {
            skip: count + 1,
            ..IteratorOptions::default()
        },
        ..Plan::default()
    };
    let mut opened = iterator.open(&reader, &path, plan).unwrap();

    assert!(items(opened.by_ref()).is_empty());

    assert_eq!(opened.state().position, content.len() as u64);
}

#[rstest]
#[case::start(IteratorOptions {
    start: 1,
    ..IteratorOptions::default()
})]
#[case::start_and_skip(IteratorOptions {
    start: 1,
    skip: 1,
    ..IteratorOptions::default()
})]
fn resume_ignores_options_when_already_past_start(
    sandbox: Sandbox,
    #[case] options: IteratorOptions,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let reader = sandbox.reader();
    let full = items(iterator.open(&reader, &path, Plan::default()).unwrap());
    let mut opened = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();

    consume(&mut opened, 4);

    let state = opened.save().unwrap().clone();
    let resumed = iterator
        .open(
            &reader,
            &path,
            Plan {
                state: state.into(),
                options,
                ..Plan::default()
            },
        )
        .unwrap();

    assert_eq!(items(resumed), full[3..]);
}

#[cfg(unix)]
#[rstest]
fn next_fails_when_io_fails_mid_item(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let directory = sandbox.dir_at("folder");
    let plan = Plan {
        state: sandbox.state_at(&directory, TEST_READ_FROM).into(),
        ..Plan::default()
    };
    let mut opened = iterator.open(&sandbox.lenient(), &directory, plan).unwrap();

    *opened.state_mut() = sandbox.state_at(&directory, TEST_REWOUND_TO);

    assert_err_is!(opened.next().unwrap(), Error::Io(_));

    assert_eq!(opened.state().position, TEST_REWOUND_TO);

    assert_err_is!(opened.next().unwrap(), Error::Io(_));

    assert_eq!(opened.state().position, TEST_READ_FROM);
}

#[rstest]
fn threshold_autosave_triggers_mid_iteration(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let mut probe = iterator.open(&sandbox.reader(), &path, Plan::default()).unwrap();

    consume(&mut probe, 4);

    let threshold = probe.state().position;
    let saving = sandbox.autosaving(threshold);
    let mut opened = iterator.open(&saving, &path, named(TEST_STATE_NAME)).unwrap();

    consume(&mut opened, 3);

    assert!(!sandbox.states().exists(TEST_STATE_NAME));

    opened.next().unwrap().unwrap();

    assert_eq!(
        sandbox.states().load(TEST_STATE_NAME).unwrap().position,
        threshold
    );
}

#[cfg(unix)]
#[rstest]
fn drop_does_not_rewrite_a_loaded_state_without_progress(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let reader = sandbox.autosaving(DEFAULT_AUTO_SAVE_STATE_BYTES.get());
    let saved = reader.states().path(TEST_STATE_NAME).unwrap();
    let mut first = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();

    consume(&mut first, 3);

    first.save().unwrap();

    let inode = fs::metadata(&saved).unwrap().ino();
    let opened = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();

    drop(opened);

    assert_eq!(fs::metadata(&saved).unwrap().ino(), inode);
}

#[rstest]
#[case::end_is_tighter(2, 100)]
#[case::limit_is_tighter(100, 2)]
fn tighter_of_end_and_limit_wins(
    sandbox: Sandbox,
    #[case] finished: usize,
    #[case] limit: u64,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let reader = sandbox.reader();
    let full = items(iterator.open(&reader, &path, Plan::default()).unwrap());
    let mut ended = iterator.open(&reader, &path, Plan::default()).unwrap();

    consume(&mut ended, finished + 1);

    let plan = Plan {
        options: IteratorOptions {
            end: ended.state().position,
            limit,
            ..IteratorOptions::default()
        },
        ..Plan::default()
    };

    assert_eq!(
        items(iterator.open(&reader, &path, plan).unwrap()),
        full[..2]
    );
}

#[rstest]
fn read_fails_when_the_final_save_fails(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.autosaving(DEFAULT_AUTO_SAVE_STATE_BYTES.get());
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();
    let mut yielded = Vec::new();

    sandbox.block_states();

    let error = loop {
        match bytes.read() {
            Ok(Some(byte)) => yielded.push(byte),
            Ok(None) => panic!("the read finished without reporting the failed save"),
            Err(error) => break error,
        }
    };

    assert_eq!(yielded, TEST_ALPHABET);

    assert_matches!(error, Error::Io(error) if error.kind() == ErrorKind::AlreadyExists);
}

#[rstest]
fn two_iterators_with_the_same_name_advance_independently(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let reader = sandbox.reader();
    let mut first = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();
    let mut second = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();

    let taken = take(&mut first, 2);
    let finished = match iterator {
        Bytes => 1,
        Chunks => 1024,
        Lines => 3,
        Segments => 2,
    };

    assert_eq!(first.state().position, finished);
    assert_eq!(second.state().position, 0);
    assert_eq!(second.next().unwrap().unwrap(), taken[0]);
}

#[rstest]
fn fails_when_an_unsaved_state_tracks_a_moved_file(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let moved = sandbox.path().join("moved.bin");
    let reader = sandbox.reader();
    let mut opened = iterator.open(&reader, &path, Plan::default()).unwrap();

    consume(&mut opened, 3);

    let state = opened.state().clone();
    let expected = state.name.clone();
    let plan = resumed(state);

    fs::rename(&path, &moved).unwrap();

    assert_err_is!(
        iterator.open(&reader, &moved, plan),
        Error::Mismatch {
            name,
            kind: Mismatch::Unsealed,
        } if *name == expected
    );
}

#[rstest]
fn iteration_survives_the_file_being_deleted(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let content = shared_content();
    let path = sandbox.file(&content);
    let reader = sandbox.capped(1);
    let expected = items(iterator.open(&reader, &path, Plan::default()).unwrap());
    let mut opened = iterator.open(&reader, &path, Plan::default()).unwrap();

    let first = opened.next().unwrap().unwrap();

    fs::remove_file(&path).unwrap();

    let rest = items(opened.by_ref());

    assert_eq!([vec![first], rest].concat(), expected);
    assert_eq!(opened.state().position, content.len() as u64);
}

#[cfg(unix)]
#[rstest]
fn symlinks_to_one_target_share_the_autoname(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let real = sandbox.write("real.bin", b"foo");
    let first_link = sandbox.path().join("link-1.bin");
    let second_link = sandbox.path().join("link-2.bin");
    let reader = sandbox.reader();

    symlink(&real, &first_link).unwrap();
    symlink(&real, &second_link).unwrap();

    let first = iterator.open(&reader, &first_link, Plan::default()).unwrap();
    let second = iterator.open(&reader, &second_link, Plan::default()).unwrap();

    assert_eq!(first.state().name, second.state().name);
}

#[cfg(unix)]
#[rstest]
fn save_fails_when_the_state_name_became_an_alias(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let outside = sandbox.dir_at("outside");
    let escaping = format!("link/{TEST_STATE_NAME}");
    let mut opened = iterator.open(&sandbox.reader(), &path, named(&escaping)).unwrap();

    consume(&mut opened, 2);

    fs::create_dir_all(sandbox.state_dir()).unwrap();

    symlink(&outside, sandbox.state_dir().join("link")).unwrap();

    assert_err_is!(opened.save(), Error::Alias { name } if *name == escaping);

    assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
}

#[rstest]
fn state_object_saves_only_into_the_readers_state_dir(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let owner = sandbox
        .autosaving(DEFAULT_AUTO_SAVE_STATE_BYTES.get())
        .bytes(&path)
        .state(TEST_STATE_NAME)
        .build()
        .unwrap();
    let borrowed = owner.state().clone();
    let borrower = sandbox.reader_in(
        "elsewhere",
        Config {
            auto_save_state: true,
            verify_state: false,
            ..sandbox.config()
        },
    );

    drop(owner);

    drain(&mut borrower.bytes(&path).state(borrowed).build().unwrap());

    assert!(sandbox.path().join("elsewhere").join(TEST_STATE_FILE).is_file());
    assert!(!sandbox.state_dir().join(TEST_STATE_FILE).exists());
}

#[rstest]
fn autosave_fails_when_the_state_was_resynced_meanwhile(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let moved = sandbox.path().join("moved.bin");
    let reader = sandbox.autosaving(4);
    let mut opened = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();

    consume(&mut opened, 10);

    fs::rename(&path, &moved).unwrap();

    let resynced = reader.states().resync(TEST_STATE_NAME, &moved).unwrap();

    assert_err_is!(
        try_items(opened.by_ref()),
        Error::Mismatch {
            name,
            kind: Mismatch::Stale,
        } if name == TEST_STATE_NAME
    );

    assert_eq!(reader.states().load(TEST_STATE_NAME).unwrap(), resynced);
}

#[cfg(unix)]
#[rstest]
fn fails_when_the_auto_named_state_is_a_symlink(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.line_file();
    let name = sandbox.reader().bytes(&path).build().unwrap().state().name.clone();
    let outside = sandbox.write("outside.json", b"{}");

    fs::create_dir_all(sandbox.state_dir()).unwrap();

    symlink(&outside, sandbox.state_dir().join(state_file(&name))).unwrap();

    assert_err_is!(
        iterator.open(&sandbox.reader(), &path, Plan::default()),
        Error::Alias { name: alias } if *alias == name
    );

    assert_eq!(fs::read(&outside).unwrap(), b"{}");
}

#[rstest]
fn clear_does_not_affect_a_live_iterator(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let reader = sandbox.reader();
    let mut opened = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();

    consume(&mut opened, 2);

    let saved = opened.save().unwrap().clone();

    sandbox.states().clear().unwrap();

    assert!(!sandbox.states().exists(TEST_STATE_NAME));

    opened.next().unwrap().unwrap();

    let fresh = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();

    assert!(opened.state().position > saved.position);

    assert_eq!(fresh.state().position, 0);
}

#[rstest]
#[case::reading(IteratorOptions::default())]
#[case::skipping(IteratorOptions { skip: 1, ..IteratorOptions::default() })]
fn fails_when_the_resumed_file_is_replaced_by_a_directory_without_verification(
    sandbox: Sandbox,
    #[case] options: IteratorOptions,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let reader = sandbox.lenient();
    let state = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap().state().clone();

    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();

    let plan = Plan {
        state: state.into(),
        options,
        ..Plan::default()
    };

    assert_err_is!(
        iterator.open(&reader, &path, plan).and_then(try_items),
        Error::Io(error)
            if matches!(error.kind(), ErrorKind::IsADirectory | ErrorKind::PermissionDenied)
            && error.to_string().contains("os error")
    );
}

#[rstest]
fn save_updates_only_the_updated_at(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.line_file();
    let reader = sandbox.lenient();

    iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap().save().unwrap();

    sandbox.patch(TEST_STATE_NAME, |payload| {
        payload["timestamps"] = json!({"created_at": 0, "updated_at": 0});
    });

    let mut opened = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();
    let created_at = opened.state().timestamps.created_at;

    let saved = opened.save().unwrap().clone();

    assert!(saved.timestamps.updated_at > created_at);

    assert_eq!(saved.timestamps.created_at, created_at);

    sandbox.payload(&saved);
}

#[rstest]
fn resume_applies_the_limit_again(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let full = items(iterator.open(&sandbox.reader(), &path, Plan::default()).unwrap());
    let resuming = sandbox.resuming();
    let options = IteratorOptions {
        limit: 2,
        ..IteratorOptions::default()
    };
    let mut opened = iterator
        .open(
            &resuming,
            &path,
            Plan {
                options,
                ..Plan::default()
            },
        )
        .unwrap();

    drain(&mut opened);

    opened.save().unwrap();

    let resumed = iterator
        .open(
            &resuming,
            &path,
            Plan {
                options,
                ..Plan::default()
            },
        )
        .unwrap();

    assert_eq!(items(resumed), full[2..4]);
}

#[rstest]
fn fails_when_a_save_fails_after_a_truncation(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let first = iterator
        .open(&sandbox.reader(), &path, Plan::default())
        .unwrap()
        .next()
        .unwrap()
        .unwrap();

    sandbox.block_states();

    let saving = sandbox.reader_with(Config {
        auto_save_state: true,
        buffer_capacity: 1,
        ..sandbox.config()
    });
    let mut opened = iterator.open(&saving, &path, named(TEST_STATE_NAME)).unwrap();

    assert_eq!(opened.next().unwrap().unwrap(), first);

    sandbox.truncate(&path, 1);

    assert_err_is!(
        opened.next().unwrap(),
        Error::Io(error) if error.kind() == ErrorKind::AlreadyExists
    );
}

#[rstest]
fn failed_autosave_exhausts_the_iterator(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let mut fresh = iterator.open(&sandbox.reader(), &path, Plan::default()).unwrap();

    sandbox.block_states();

    let reader = sandbox.autosaving(1);
    let mut opened = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();

    assert_eq!(
        opened.next().unwrap().unwrap(),
        fresh.next().unwrap().unwrap()
    );

    assert_err_is!(
        opened.next().unwrap(),
        Error::Io(error) if error.kind() == ErrorKind::AlreadyExists
    );

    assert!((0..3).all(|_| opened.next().is_none()));
}

#[rstest]
fn iteration_stops_at_a_truncation(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let content = shared_content();
    let path = sandbox.file(&content);
    let truncated = sandbox.write("truncated.bin", &content[..4096]);
    let reader = sandbox.capped(1);
    let expected = items(iterator.open(&reader, &truncated, Plan::default()).unwrap());
    let mut opened = iterator.open(&reader, &path, Plan::default()).unwrap();

    let first = opened.next().unwrap().unwrap();

    sandbox.truncate(&path, 4096);

    let rest = items(opened.by_ref());

    assert_eq!([vec![first], rest].concat(), expected);
    assert_eq!(opened.state().position, 4096);
}

#[rstest]
fn end_below_the_position_does_not_rewind_the_state(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let content = shared_content();
    let path = sandbox.file(&content);
    let reader = sandbox.autosaving(DEFAULT_AUTO_SAVE_STATE_BYTES.get());

    drain(&mut iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap());

    let saved = sandbox.states().load(TEST_STATE_NAME).unwrap().position;

    assert_eq!(saved, content.len() as u64);

    let plan = Plan {
        state: TEST_STATE_NAME.into(),
        options: IteratorOptions {
            end: 5,
            ..IteratorOptions::default()
        },
        ..Plan::default()
    };

    drain(&mut iterator.open(&reader, &path, plan).unwrap());

    assert_eq!(
        sandbox.states().load(TEST_STATE_NAME).unwrap().position,
        saved
    );
}

#[rstest]
fn replacing_state_with_earlier_one_keeps_reading(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.autosaving(5);
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    consume(&mut bytes, 10);
    drop(bytes);

    let rewound = sandbox.named_state(&path, TEST_OTHER_STATE_NAME);
    let mut resumed = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    assert_eq!(resumed.state().position, 9);

    *resumed.state_mut() = rewound;

    assert_eq!(resumed.read().unwrap(), Some(TEST_ALPHABET[9]));
    assert_eq!(resumed.read().unwrap(), Some(TEST_ALPHABET[10]));
    assert_eq!(resumed.state().position, 1);
}

#[rstest]
fn save_rewinds_to_an_explicit_state_object(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let reader = sandbox.reader();
    let mut opened = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();

    consume(&mut opened, 3);

    let earlier = opened.save().unwrap().clone();

    consume(&mut opened, 5);

    opened.save().unwrap();

    let mut second = iterator.open(&reader, &path, resumed(earlier.clone())).unwrap();

    consume(&mut second, 1);

    let rewound = second.save().unwrap().clone();

    assert_eq!(sandbox.states().load(TEST_STATE_NAME).unwrap(), rewound);
    assert_eq!(rewound.position, earlier.position);
}

#[rstest]
fn resumed_read_floors_the_last_saved_position(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let mut first = iterator.open(&sandbox.reader(), &path, named(TEST_STATE_NAME)).unwrap();

    consume(&mut first, 3);

    let state = first.save().unwrap().clone();

    let (threshold, floored) = match iterator {
        Bytes => (3, 3),
        Chunks => (1500, 3072),
        Lines => (4, 9),
        Segments => (4, 8),
    };
    let plan = resumed(state);
    let mut resumed = iterator.open(&sandbox.autosaving(threshold), &path, plan).unwrap();

    consume(&mut resumed, 2);

    assert_eq!(
        sandbox.states().load(TEST_STATE_NAME).unwrap().position,
        floored
    );
}

#[rstest]
fn resume_applies_skip_when_the_position_equals_the_start(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let reader = sandbox.reader();
    let full = items(iterator.open(&reader, &path, Plan::default()).unwrap());
    let mut opened = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();

    consume(&mut opened, 3);

    let saved = opened.save().unwrap().clone();
    let plan = Plan {
        options: IteratorOptions {
            start: saved.position,
            skip: 1,
            ..IteratorOptions::default()
        },
        state: saved.into(),
        ..Plan::default()
    };
    let resumed = items(iterator.open(&reader, &path, plan).unwrap());

    assert_eq!(resumed, full[3..]);
}

#[rstest]
fn state_object_saves_into_the_readers_state_dir(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let content = shared_content();
    let path = sandbox.file(&content);
    let mut opened = iterator.open(&sandbox.reader(), &path, named(TEST_STATE_NAME)).unwrap();

    consume(&mut opened, 2);

    let state = opened.save().unwrap().clone();
    let elsewhere = sandbox.reader_in(
        "elsewhere",
        Config {
            auto_save_state: true,
            ..sandbox.config()
        },
    );
    let plan = resumed(state.clone());

    drain(&mut iterator.open(&elsewhere, &path, plan).unwrap());

    assert_eq!(sandbox.states().load(TEST_STATE_NAME).unwrap(), state);
    assert_eq!(
        elsewhere.states().load(TEST_STATE_NAME).unwrap().position,
        content.len() as u64
    );
}

#[rstest]
fn save_fails_when_another_iterator_saved_meanwhile(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let reader = sandbox.reader();

    iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap().save().unwrap();

    let mut first = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();
    let mut second = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();

    consume(&mut first, 4);

    let saved = first.save().unwrap().clone();

    consume(&mut second, 2);

    assert_err_is!(
        second.save(),
        Error::Mismatch {
            name,
            kind: Mismatch::Stale,
        } if name == TEST_STATE_NAME
    );

    assert_eq!(sandbox.states().load(TEST_STATE_NAME).unwrap(), saved);
}

#[rstest]
fn percent_tracks_the_position(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let content = shared_content();
    let size = content.len() as u64;
    let path = sandbox.file(&content);
    let mut opened = iterator.open(&sandbox.reader(), &path, Plan::default()).unwrap();

    assert_eq!(opened.percent(), 0.0);

    while opened.next().transpose().unwrap().is_some() {
        assert_eq!(
            opened.percent(),
            opened.state().position as f64 * 100.0 / size as f64
        );
        assert_eq!(opened.state().percent(), opened.percent());
    }

    assert_eq!(opened.state().position, size);
    assert_eq!(opened.percent(), 100.0);
    assert_eq!(opened.state().percent(), 100.0);
}

#[rstest]
#[case::with_verification(true)]
#[case::without_verification(false)]
fn save_fails_when_the_state_was_resynced_meanwhile(
    sandbox: Sandbox,
    #[case] verify_state: bool,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let moved = sandbox.path().join("moved.bin");
    let reader = sandbox.reader_with(Config {
        verify_state,
        ..sandbox.config()
    });
    let mut opened = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();

    consume(&mut opened, 3);

    opened.save().unwrap();

    fs::rename(&path, &moved).unwrap();

    reader.states().resync(TEST_STATE_NAME, &moved).unwrap();

    assert_err_is!(
        opened.save(),
        Error::Mismatch {
            name,
            kind: Mismatch::Stale,
        } if name == TEST_STATE_NAME
    );

    assert_eq!(
        reader.states().load(TEST_STATE_NAME).unwrap().file.path,
        canonical(&moved)
    );
}

#[rstest]
fn fails_when_a_threshold_save_fails(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let full = items(iterator.open(&sandbox.reader(), &path, Plan::default()).unwrap());
    let mut probe = iterator.open(&sandbox.reader(), &path, Plan::default()).unwrap();

    consume(&mut probe, 3);

    let threshold = probe.state().position;

    sandbox.block_states();

    let saving = sandbox.autosaving(threshold);
    let mut opened = iterator.open(&saving, &path, named(TEST_STATE_NAME)).unwrap();

    assert_eq!(take(&mut opened, 2), full[..2]);

    assert_err_is!(
        opened.next().unwrap(),
        Error::Io(error) if error.kind() == ErrorKind::AlreadyExists
    );
}

#[rstest]
fn next_keeps_the_saved_timestamps(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let reader = sandbox.lenient();

    iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap().save().unwrap();

    sandbox.patch(TEST_STATE_NAME, |payload| {
        payload["timestamps"] = json!({"created_at": 0, "updated_at": 0});
    });

    let mut opened = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();
    let saved = opened.state().clone();

    consume(&mut opened, 3);

    let finished = match iterator {
        Bytes => 2,
        Chunks => 2048,
        Lines => 6,
        Segments => 5,
    };

    assert_eq!(opened.state().position, finished);
    assert_eq!(opened.state().timestamps, saved.timestamps);
}

#[rstest]
fn recorded_file_size_never_refreshes_after_the_file_grows(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let content = shared_content();
    let size = content.len() as u64;
    let path = sandbox.file(&content);
    let reader = sandbox.lenient();
    let mut opened = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();

    drain(&mut opened);

    let state = opened.state().clone();

    sandbox.append(&path, b"more");

    let plan = Plan {
        state: state.into(),
        options: IteratorOptions {
            end: 2 * size,
            ..IteratorOptions::default()
        },
        ..Plan::default()
    };
    let mut resumed = iterator.open(&reader, &path, plan).unwrap();

    assert!(items(resumed.by_ref()).is_empty());

    assert_eq!(resumed.state().file.size, size);
}

#[cfg(unix)]
#[rstest]
fn save_fails_when_a_saved_state_name_became_an_alias(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let outside = sandbox.dir_at("outside");
    let escaping = format!("link/{TEST_STATE_NAME}");
    let mut opened = iterator.open(&sandbox.reader(), &path, named(&escaping)).unwrap();

    consume(&mut opened, 2);

    opened.save().unwrap();

    fs::rename(
        sandbox.state_dir().join("link"),
        sandbox.path().join("moved"),
    )
    .unwrap();

    symlink(&outside, sandbox.state_dir().join("link")).unwrap();

    assert_err_is!(opened.save(), Error::Alias { name } if *name == escaping);

    assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
}

#[cfg(unix)]
#[rstest]
fn fails_when_the_state_name_is_a_directory_alias(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.line_file();
    let reader = sandbox.reader();
    let real = format!("real/{TEST_STATE_NAME}");
    let alias = format!("alias/{TEST_STATE_NAME}");

    iterator.open(&reader, &path, named(&real)).unwrap().save().unwrap();

    let saved = sandbox.states().path(&real).unwrap();
    let before = fs::read(&saved).unwrap();

    symlink(saved.parent().unwrap(), sandbox.state_dir().join("alias")).unwrap();

    assert_err_is!(
        iterator.open(&reader, &path, named(&alias)),
        Error::Alias { name } if *name == alias
    );

    assert_eq!(fs::read(&saved).unwrap(), before);
}

#[rstest]
fn save_leaves_the_state_untouched_when_it_fails(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.line_file();
    let reader = sandbox.lenient();

    iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap().save().unwrap();

    sandbox.patch(TEST_STATE_NAME, |payload| {
        payload["timestamps"] = json!({"created_at": 0, "updated_at": 0});
    });

    let mut opened = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();
    let before = opened.state().clone();
    let payload = sandbox.states().path(TEST_STATE_NAME).unwrap();

    fs::remove_file(&payload).unwrap();
    fs::create_dir(&payload).unwrap();

    assert_err_is!(opened.save(), Error::Io(_));

    assert_eq!(*opened.state(), before);
}

#[rstest]
fn drop_keeps_a_state_resynced_meanwhile(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let moved = sandbox.path().join("moved.bin");
    let reader = sandbox.autosaving(DEFAULT_AUTO_SAVE_STATE_BYTES.get());
    let mut opened = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();

    consume(&mut opened, 3);

    let saved = opened.save().unwrap().clone();

    fs::rename(&path, &moved).unwrap();

    let resynced = reader.states().resync(TEST_STATE_NAME, &moved).unwrap();

    opened.next().unwrap().unwrap();

    drop(opened);

    let resumed = iterator.open(&reader, &moved, named(TEST_STATE_NAME)).unwrap();

    assert_eq!(reader.states().load(TEST_STATE_NAME).unwrap(), resynced);
    assert_eq!(resumed.state().position, saved.position);
}

#[rstest]
fn file_growth_during_iteration_is_ignored(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let content = shared_content();
    let size = content.len() as u64;
    let path = sandbox.file(&content);
    let full = items(iterator.open(&sandbox.reader(), &path, Plan::default()).unwrap());
    let mut opened = iterator.open(&sandbox.reader(), &path, Plan::default()).unwrap();

    opened.next().unwrap().unwrap();
    sandbox.append(&path, b"more");

    let (kept, finished) = match iterator {
        Bytes | Chunks | Lines => (full.len(), size),
        Segments => (full.len() - 1, size - 1),
    };

    assert_eq!(items(opened.by_ref()), full[1..kept]);
    assert_eq!(opened.state().position, finished);
    assert_eq!(opened.state().file.size, size);
}

#[rstest]
fn threshold_saves_during_iteration(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let content = shared_content();
    let path = sandbox.file(&content);
    let reader = sandbox.autosaving(8192);
    let mut opened = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();
    let state_file = sandbox.states().path(TEST_STATE_NAME).unwrap();
    let mut payload = Vec::new();
    let mut positions = Vec::new();

    while opened.next().transpose().unwrap().is_some() {
        if let Ok(current) = fs::read(&state_file)
            && current != payload
        {
            positions.push(sandbox.states().load(TEST_STATE_NAME).unwrap().position);
            payload = current;
        }
    }

    let saved = match iterator {
        Bytes | Chunks => [8192, 16384],
        Lines => [8196, 16392],
        Segments => [8195, 16391],
    };

    assert_eq!(positions, saved);
    assert_eq!(
        sandbox.states().load(TEST_STATE_NAME).unwrap().position,
        content.len() as u64
    );
}

#[rstest]
#[case::start(5, None, 0, u64::MAX, 5..usize::MAX)]
#[case::end(0, Some(5), 0, u64::MAX, 0..5)]
#[case::skip(0, None, 3, u64::MAX, 3..usize::MAX)]
#[case::start_and_skip(5, None, 3, u64::MAX, 8..usize::MAX)]
#[case::start_and_limit(10, None, 0, 3, 10..13)]
#[case::start_and_end(5, Some(10), 0, u64::MAX, 5..10)]
#[case::start_skip_and_limit(5, None, 3, 4, 8..12)]
#[case::skip_and_limit(0, None, 1, 2, 1..3)]
fn options_narrow_the_output(
    sandbox: Sandbox,
    #[case] start: usize,
    #[case] end: Option<usize>,
    #[case] skip: u64,
    #[case] limit: u64,
    #[case] expected: Range<usize>,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let reader = sandbox.reader();
    let mut plain = iterator.open(&reader, &path, Plan::default()).unwrap();
    let starts: Vec<u64> =
        iter::from_fn(|| plain.next().map(|item| item.map(|_| plain.state().position).unwrap()))
            .collect();
    let full = items(iterator.open(&reader, &path, Plan::default()).unwrap());
    let plan = Plan {
        options: IteratorOptions {
            start: starts[start],
            end: end.map_or(u64::MAX, |index| starts[index]),
            skip,
            limit,
        },
        ..Plan::default()
    };

    assert_eq!(
        items(iterator.open(&reader, &path, plan).unwrap()),
        full.into_iter().take(expected.end).skip(expected.start).collect::<Vec<_>>()
    );
}

#[rstest]
fn save_keeps_failing_after_the_state_was_resynced(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let moved = sandbox.path().join("moved.bin");
    let reader = sandbox.reader();
    let mut opened = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();

    consume(&mut opened, 3);

    let saved = opened.save().unwrap().clone();

    fs::rename(&path, &moved).unwrap();

    reader.states().resync(TEST_STATE_NAME, &moved).unwrap();

    assert_err_is!(
        opened.save(),
        Error::Mismatch {
            name,
            kind: Mismatch::Stale,
        } if name == TEST_STATE_NAME
    );

    opened.next().unwrap().unwrap();

    assert_err_is!(
        opened.save(),
        Error::Mismatch {
            name,
            kind: Mismatch::Stale,
        } if name == TEST_STATE_NAME
    );

    let resumed = iterator.open(&reader, &moved, named(TEST_STATE_NAME)).unwrap();

    assert_eq!(resumed.state().position, saved.position);
}

#[rstest]
fn resync_lets_a_grown_file_resume(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let content = shared_content();
    let path = sandbox.file(&content);
    let reader = sandbox.reader();
    let mut opened = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();

    drain(&mut opened);

    opened.save().unwrap();
    sandbox.append(&path, b"more");

    assert_err_is!(
        iterator.open(&reader, &path, named(TEST_STATE_NAME)),
        Error::Mismatch {
            name,
            kind: Mismatch::Size,
        } if name == TEST_STATE_NAME
    );

    let options = IteratorOptions {
        start: content.len() as u64,
        ..IteratorOptions::default()
    };
    let appended = items(
        iterator
            .open(
                &reader,
                &path,
                Plan {
                    options,
                    ..Plan::default()
                },
            )
            .unwrap(),
    );
    let resynced = reader.states().resync(TEST_STATE_NAME, &path).unwrap();
    let resumed = iterator.open(&reader, &path, resumed(resynced.clone())).unwrap();

    assert_eq!(resynced.file.size, (content.len() + b"more".len()) as u64);
    assert_eq!(items(resumed), appended);
}

#[rstest]
fn resync_lets_a_moved_file_resume(
    sandbox: Sandbox,
    #[values(Bytes, Chunks, Lines, Segments)] iterator: IteratorKind,
) {
    let path = sandbox.file(&shared_content());
    let moved = sandbox.path().join("moved.bin");
    let reader = sandbox.reader();
    let full = items(iterator.open(&reader, &path, Plan::default()).unwrap());
    let mut opened = iterator.open(&reader, &path, named(TEST_STATE_NAME)).unwrap();

    consume(&mut opened, 2);

    let state = opened.save().unwrap().clone();

    fs::rename(&path, &moved).unwrap();

    assert_err_is!(
        iterator.open(&reader, &moved, resumed(state)),
        Error::Mismatch {
            name,
            kind: Mismatch::Path,
        } if name == TEST_STATE_NAME
    );
    assert_err_is!(
        iterator.open(&reader, &moved, named(TEST_STATE_NAME)),
        Error::Mismatch {
            name,
            kind: Mismatch::Path,
        } if name == TEST_STATE_NAME
    );

    let resynced = reader.states().resync(TEST_STATE_NAME, &moved).unwrap();
    let resumed = iterator.open(&reader, &moved, resumed(resynced)).unwrap();

    assert_eq!(resumed.state().file.path, canonical(&moved));
    assert_eq!(items(resumed), full[1..]);
}

#[cfg(unix)]
#[rstest]
fn resumed_read_covers_what_failed_save_left_behind(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.autosaving(5);
    let mut blocked = Blocked::default();
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();
    let first = take(&mut bytes, 10);

    drop(bytes);

    let checkpoint = sandbox.states().load(TEST_STATE_NAME).unwrap().position;

    blocked.read_only(sandbox.state_dir());

    if fs::write(sandbox.state_dir().join("probe"), b"x").is_ok() {
        return;
    }

    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();
    let mut second = Vec::new();

    let error = loop {
        match bytes.read() {
            Ok(Some(byte)) => second.push(byte),
            Ok(None) => panic!("the read finished without reporting the failed save"),
            Err(error) => break error,
        }
    };

    drop(bytes);
    drop(blocked);

    assert_eq!(
        sandbox.states().load(TEST_STATE_NAME).unwrap().position,
        checkpoint
    );

    assert_matches!(error, Error::Io(_));

    let third = items(reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap());

    assert_eq!(second, &third[..second.len()]);
    assert_eq!(
        [&first[..first.len() - 1], third.as_slice()].concat(),
        TEST_ALPHABET
    );
}
