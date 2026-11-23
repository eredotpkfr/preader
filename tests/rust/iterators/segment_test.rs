use std::{
    fs,
    io::{BufRead, ErrorKind},
};

use preader::{Config, DEFAULT_BUFFER_CAPACITY, Error, IteratorRead};
use rstest::rstest;
use rstest_reuse::apply;

#[cfg(unix)]
use crate::common::constants::{TEST_READ_FROM, TEST_REWOUND_TO};
use crate::common::{
    constants::{
        TEST_BLANK_SEGMENT_CONTENT, TEST_DEFAULT_DELIMITER, TEST_INVALID_UTF8,
        TEST_SEGMENT_CONTENT, TEST_STATE_NAME, TEST_UNICODE_TEXT,
    },
    fixtures::sandbox,
    funcs::{consume, drain, items, texts},
    macros::asserts::assert_err_is,
    matrix::contents::{ALL_BYTES, EMPTY, UTF8_SEGMENTS},
    sandbox::Sandbox,
    templates::delimiter::delimiters,
};

const SEGMENTS: [&str; 10] = [
    "seg-0", "seg-1", "seg-2", "seg-3", "seg-4", "seg-5", "seg-6", "seg-7", "seg-8", "seg-9",
];

fn segments(sandbox: &Sandbox, content: &[u8]) -> Vec<String> {
    let path = sandbox.file(content);

    texts(&items(sandbox.reader().segments(&path).build().unwrap()))
}

#[rstest]
fn iterator_defaults(sandbox: Sandbox) {
    let path = sandbox.file(TEST_BLANK_SEGMENT_CONTENT);
    let reader = sandbox.reader();
    let defaults = items(reader.segments(&path).build().unwrap());
    let explicit = items(
        reader
            .segments(&path)
            .delimiter(TEST_DEFAULT_DELIMITER)
            .keep(false)
            .skip_empty(false)
            .skip(0)
            .build()
            .unwrap(),
    );

    assert_eq!(texts(&defaults), ["seg-0", "", "seg-2"]);
    assert_eq!(defaults, explicit);
}

#[rstest]
fn splits_on_the_delimiter(sandbox: Sandbox) {
    assert_eq!(segments(&sandbox, TEST_SEGMENT_CONTENT), SEGMENTS);
}

#[rstest]
fn content_without_the_delimiter_yields_one_segment(sandbox: Sandbox) {
    assert_eq!(segments(&sandbox, b"seg-0"), ["seg-0"]);
}

#[rstest]
fn trailing_delimiter_yields_no_extra_segment(sandbox: Sandbox) {
    assert_eq!(segments(&sandbox, b"seg-0,seg-1,"), ["seg-0", "seg-1"]);
}

#[rstest]
#[case::neither(false, false, &["", "", ""])]
#[case::skip_empty(false, true, &[])]
#[case::keep_delimiter(true, false, &[",", ",", ","])]
#[case::both(true, true, &[])]
fn delimiter_only_file_follows_keep_and_skip_empty(
    sandbox: Sandbox,
    #[case] keep: bool,
    #[case] skip_empty: bool,
    #[case] expected: &[&str],
) {
    let path = sandbox.file(&[TEST_DEFAULT_DELIMITER; 3]);
    let read = items(
        sandbox
            .reader()
            .segments(&path)
            .keep(keep)
            .skip_empty(skip_empty)
            .build()
            .unwrap(),
    );

    assert_eq!(texts(&read), expected);
}

#[rstest]
fn align_saturates_a_maximum_skip(sandbox: Sandbox) {
    let path = sandbox.file(TEST_SEGMENT_CONTENT);
    let read = items(
        sandbox
            .reader()
            .segments(&path)
            .start(2)
            .align(true)
            .skip(u64::MAX)
            .build()
            .unwrap(),
    );

    assert!(read.is_empty());
}

#[rstest]
#[case::neither(false, false, &["foo", "", "bar"])]
#[case::skip_empty(false, true, &["foo", "bar"])]
#[case::keep_delimiter(true, false, &["foo,", ",", "bar,"])]
#[case::both(true, true, &["foo,", "bar,"])]
fn blank_segments_follow_keep_and_skip_empty(
    sandbox: Sandbox,
    #[case] keep: bool,
    #[case] skip_empty: bool,
    #[case] expected: &[&str],
) {
    let path = sandbox.file(b"foo,,bar,");
    let read = items(
        sandbox
            .reader()
            .segments(&path)
            .keep(keep)
            .skip_empty(skip_empty)
            .build()
            .unwrap(),
    );

    assert_eq!(texts(&read), expected);
}

#[rstest]
fn keep_leaves_the_last_segment_unterminated(sandbox: Sandbox) {
    let path = sandbox.file(b"foo,bar");
    let read = items(sandbox.reader().segments(&path).keep(true).build().unwrap());

    assert_eq!(texts(&read), ["foo,", "bar"]);
}

#[rstest]
fn read_splits_invalid_bytes(sandbox: Sandbox) {
    let content = [
        TEST_INVALID_UTF8,
        &[TEST_DEFAULT_DELIMITER],
        TEST_INVALID_UTF8,
    ]
    .concat();
    let path = sandbox.file(&content);
    let read = items(sandbox.reader().segments(&path).build().unwrap());

    assert_eq!(read, [TEST_INVALID_UTF8, TEST_INVALID_UTF8]);
}

#[rstest]
#[case::aligned_both(true, true, true, &["seg-2,"])]
#[case::aligned_keep_delimiter(true, true, false, &[",", "seg-2,"])]
#[case::aligned_skip_empty(true, false, true, &["seg-2"])]
#[case::aligned_neither(true, false, false, &["", "seg-2"])]
#[case::unaligned_both(false, true, true, &["-0,", "seg-2,"])]
#[case::unaligned_keep_delimiter(false, true, false, &["-0,", ",", "seg-2,"])]
#[case::unaligned_skip_empty(false, false, true, &["-0", "seg-2"])]
#[case::unaligned_neither(false, false, false, &["-0", "", "seg-2"])]
fn flag_combinations_narrow_the_output(
    sandbox: Sandbox,
    #[case] align: bool,
    #[case] keep: bool,
    #[case] skip_empty: bool,
    #[case] expected: &[&str],
) {
    let path = sandbox.file(TEST_BLANK_SEGMENT_CONTENT);
    let read = items(
        sandbox
            .reader()
            .segments(&path)
            .start(3)
            .keep(keep)
            .align(align)
            .skip_empty(skip_empty)
            .build()
            .unwrap(),
    );

    assert_eq!(texts(&read), expected);
}

#[rstest]
fn skipped_blank_segment_does_not_consume_the_limit(sandbox: Sandbox) {
    let path = sandbox.file(TEST_BLANK_SEGMENT_CONTENT);
    let read = items(sandbox.reader().segments(&path).skip_empty(true).limit(2).build().unwrap());

    assert_eq!(texts(&read), ["seg-0", "seg-2"]);
}

#[rstest]
#[case::skip(1, 0, false)]
#[case::align_start(0, 1, true)]
fn skipped_items_count_as_finished(
    sandbox: Sandbox,
    #[case] skip: u64,
    #[case] start: u64,
    #[case] align: bool,
) {
    let path = sandbox.file(TEST_SEGMENT_CONTENT);
    let mut segments = sandbox
        .reader()
        .segments(&path)
        .skip(skip)
        .start(start)
        .align(align)
        .build()
        .unwrap();

    consume(&mut segments, 1);

    assert_eq!(segments.state().position, 6);
}

#[rstest]
fn skip_empty_judges_blankness_without_delimiter(sandbox: Sandbox) {
    let path = sandbox.file(TEST_BLANK_SEGMENT_CONTENT);
    let read = items(sandbox.reader().segments(&path).keep(true).skip_empty(true).build().unwrap());

    assert_eq!(texts(&read), ["seg-0,", "seg-2,"]);
}

#[rstest]
fn align_is_a_no_op_on_a_segment_boundary(sandbox: Sandbox) {
    let path = sandbox.file(TEST_SEGMENT_CONTENT);
    let mut aligned = sandbox.reader().segments(&path).start(6).align(true).build().unwrap();

    assert_eq!(aligned.read().unwrap(), Some(b"seg-1".as_slice()));
}

#[apply(delimiters)]
fn delimiter_accepts_any_byte(sandbox: Sandbox, #[case] delimiter: u8) {
    let segment = delimiter.wrapping_add(1);
    let path = sandbox.file(&[segment, delimiter, segment]);
    let read = items(sandbox.reader().segments(&path).delimiter(delimiter).build().unwrap());

    assert_eq!(read, [[segment], [segment]]);
}

#[rstest]
fn align_and_skip_count_a_blank_segment(sandbox: Sandbox) {
    let path = sandbox.file(TEST_BLANK_SEGMENT_CONTENT);
    let mut segments =
        sandbox.reader().segments(&path).start(2).align(true).skip(1).build().unwrap();

    assert_eq!(segments.read().unwrap(), Some(b"seg-2".as_slice()));
}

#[rstest]
fn newline_delimiter_keeps_the_carriage_return(sandbox: Sandbox) {
    let path = sandbox.file(b"foo\r\nbar\n");
    let read = items(sandbox.reader().segments(&path).delimiter(b'\n').build().unwrap());

    assert_eq!(read, [b"foo\r".as_slice(), b"bar"]);
}

#[rstest]
fn yields_every_byte_value(sandbox: Sandbox) {
    let path = sandbox.file(ALL_BYTES);
    let read = items(sandbox.reader().segments(&path).keep(true).build().unwrap());

    assert_eq!(read, [&ALL_BYTES[..45], &ALL_BYTES[45..]]);
}

#[rstest]
fn skipping_holds_back_an_unterminated_segment(sandbox: Sandbox) {
    let path = sandbox.file(b"foo,bar");
    let mut segments = sandbox.reader().segments(&path).skip(2).build().unwrap();

    sandbox.append(&path, b"more");

    assert!(items(&mut segments).is_empty());

    assert_eq!(segments.state().position, 4);
}

#[rstest]
fn end_yields_a_crossing_segment_whole(sandbox: Sandbox) {
    let path = sandbox.file(TEST_SEGMENT_CONTENT);
    let mut segments = sandbox.reader().segments(&path).end(8).build().unwrap();
    let read = items(&mut segments);

    assert_eq!(texts(&read), ["seg-0", "seg-1"]);
    assert_eq!(segments.state().position, 12);
}

#[rstest]
fn skipping_past_a_terminated_segment_ignores_growth(sandbox: Sandbox) {
    let path = sandbox.file(b"foo,bar,");
    let mut segments = sandbox.reader().segments(&path).skip(2).build().unwrap();

    sandbox.append(&path, b"more");

    assert!(items(&mut segments).is_empty());

    assert_eq!(segments.state().position, 8);
}

#[apply(delimiters)]
fn delimiter_selects_the_separator(sandbox: Sandbox, #[case] delimiter: u8) {
    let content = [b"seg-0", [delimiter].as_slice(), b"seg-1"].concat();
    let path = sandbox.file(&content);
    let read = items(sandbox.reader().segments(&path).delimiter(delimiter).build().unwrap());

    assert_eq!(texts(&read), ["seg-0", "seg-1"]);
}

#[rstest]
fn fails_when_a_save_fails_on_a_skipped_item(sandbox: Sandbox) {
    let path = sandbox.file(TEST_SEGMENT_CONTENT);

    sandbox.block_states();

    let reader = sandbox.autosaving(1);
    let mut segments = reader.segments(&path).state(TEST_STATE_NAME).skip(2).build().unwrap();

    assert_err_is!(segments.read(), Error::Io(error) if error.kind() == ErrorKind::AlreadyExists);

    assert_eq!(segments.state().position, 6);
}

#[rstest]
fn position_counts_finished_items(sandbox: Sandbox) {
    let path = sandbox.file(TEST_SEGMENT_CONTENT);
    let mut segments = sandbox.reader().segments(&path).build().unwrap();

    segments.read().unwrap();

    assert_eq!(segments.state().position, 0);
    assert_eq!(segments.percent(), 0.0);

    segments.read().unwrap();

    assert_eq!(segments.state().position, 6);
}

#[rstest]
#[case::file_start(0, 0)]
#[case::item_boundary(12, 2)]
fn align_skips_nothing_when_already_aligned(
    sandbox: Sandbox,
    #[case] start: u64,
    #[case] index: usize,
) {
    let path = sandbox.file(TEST_SEGMENT_CONTENT);
    let reader = sandbox.reader();
    let aligned = items(reader.segments(&path).start(start).align(true).build().unwrap());
    let unaligned = items(reader.segments(&path).start(start).build().unwrap());

    assert_eq!(aligned, unaligned);
    assert_eq!(texts(&aligned), SEGMENTS[index..]);
}

#[rstest]
#[case::comma(b',')]
#[case::semicolon(b';')]
#[case::pipe(b'|')]
#[case::tab(b'\t')]
#[case::newline(b'\n')]
#[case::null(b'\0')]
fn splits_utf8_text_between_characters(sandbox: Sandbox, #[case] delimiter: u8) {
    let words: Vec<&str> = TEST_UNICODE_TEXT.split_whitespace().chain(["鉄"]).collect();
    let path = sandbox.file(&words.join(char::from(delimiter).to_string().as_str()).into_bytes());
    let read = items(sandbox.reader().segments(&path).delimiter(delimiter).build().unwrap());

    assert_eq!(texts(&read), words);
}

#[rstest]
fn fails_when_a_save_fails_on_a_filtered_blank(sandbox: Sandbox) {
    let path = sandbox.file(b",,foo");

    sandbox.block_states();

    let reader = sandbox.autosaving(1);
    let mut segments =
        reader.segments(&path).state(TEST_STATE_NAME).skip_empty(true).build().unwrap();

    assert_err_is!(segments.read(), Error::Io(error) if error.kind() == ErrorKind::AlreadyExists);

    assert_eq!(segments.state().position, 1);
}

#[rstest]
fn resumed_read_ignores_align(sandbox: Sandbox) {
    let path = sandbox.file(TEST_SEGMENT_CONTENT);
    let state = sandbox.checkpoint(&path, TEST_STATE_NAME, 3);
    let reader = sandbox.reader();

    let resumed = items(reader.segments(&path).state(state).start(2).align(true).build().unwrap());

    assert_eq!(
        resumed,
        TEST_SEGMENT_CONTENT[3..]
            .split(|byte| *byte == TEST_DEFAULT_DELIMITER)
            .collect::<Vec<_>>()
    );
}

#[rstest]
#[case::refilled(1)]
#[case::default(DEFAULT_BUFFER_CAPACITY)]
fn growth_holds_back_an_unterminated_segment(sandbox: Sandbox, #[case] capacity: usize) {
    let path = sandbox.file(b"foo,bar");
    let mut segments = sandbox.capped(capacity).segments(&path).build().unwrap();

    consume(&mut segments, 1);

    sandbox.append(&path, b"more");

    assert!(items(&mut segments).is_empty());

    assert_eq!(segments.state().position, 4);
    assert_eq!(segments.percent(), 400.0 / 7.0);
}

#[rstest]
fn file_growth_during_iteration_is_ignored(sandbox: Sandbox) {
    let path = sandbox.file(b"foo,bar,");
    let mut segments = sandbox.reader().segments(&path).build().unwrap();

    consume(&mut segments, 1);

    sandbox.append(&path, b"more");

    assert_eq!(texts(&items(&mut segments)), ["bar"]);
    assert_eq!(segments.state().position, 8);
}

#[rstest]
fn fails_when_the_aligned_file_is_replaced_by_a_directory_without_verification(sandbox: Sandbox) {
    let path = sandbox.file(TEST_SEGMENT_CONTENT);
    let reader = sandbox.lenient();
    let state = reader.segments(&path).state(TEST_STATE_NAME).build().unwrap().state().clone();

    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();

    assert_err_is!(
        reader.segments(&path).state(state).start(5).align(true).build(),
        Error::Io(_)
    );
}

#[rstest]
fn keep_does_not_change_the_position(sandbox: Sandbox) {
    let path = sandbox.file(TEST_SEGMENT_CONTENT);
    let reader = sandbox.reader();
    let mut stripped = reader.segments(&path).build().unwrap();
    let mut kept = reader.segments(&path).keep(true).build().unwrap();

    drain(&mut stripped);
    drain(&mut kept);

    assert_eq!(stripped.state().position, kept.state().position);
    assert_eq!(kept.state().position, TEST_SEGMENT_CONTENT.len() as u64);
}

#[rstest]
fn skip_stops_at_a_truncation(sandbox: Sandbox) {
    let path = sandbox.file(TEST_SEGMENT_CONTENT);
    let reader = sandbox.lenient();

    sandbox.saved(&path, TEST_STATE_NAME);
    sandbox.truncate(&path, 8);

    let mut resumed = reader.segments(&path).state(TEST_STATE_NAME).skip(3).build().unwrap();

    assert!(items(&mut resumed).is_empty());

    assert_eq!(resumed.state().position, 8);
    assert_eq!(resumed.state().file.size, TEST_SEGMENT_CONTENT.len() as u64);
}

#[rstest]
fn align_and_skip_combine(sandbox: Sandbox) {
    let path = sandbox.file(TEST_SEGMENT_CONTENT);
    let reader = sandbox.reader();
    let aligned = items(reader.segments(&path).start(3).skip(1).align(true).build().unwrap());
    let unaligned = items(reader.segments(&path).start(3).skip(1).build().unwrap());

    assert_eq!(texts(&aligned), SEGMENTS[2..]);
    assert_eq!(texts(&unaligned), SEGMENTS[1..]);
}

#[rstest]
fn fails_when_a_save_fails_while_skipping(sandbox: Sandbox) {
    let path = sandbox.file(TEST_SEGMENT_CONTENT);

    sandbox.block_states();

    let reader = sandbox.reader_with(Config {
        auto_save_state: true,
        ..sandbox.config()
    });
    let mut segments = reader.segments(&path).state(TEST_STATE_NAME).skip(3).build().unwrap();

    sandbox.truncate(&path, 1);

    assert_err_is!(segments.read(), Error::Io(error) if error.kind() == ErrorKind::AlreadyExists);

    assert_eq!(segments.state().position, 1);
}

#[rstest]
fn read_yields_invalid_bytes(sandbox: Sandbox) {
    let content = [b"foo,", TEST_INVALID_UTF8, b",bar"].concat();
    let path = sandbox.file(&content);
    let mut segments = sandbox.reader().segments(&path).build().unwrap();
    let read = items(&mut segments);

    assert_eq!(read, [b"foo".as_slice(), TEST_INVALID_UTF8, b"bar"]);
    assert_eq!(segments.state().position, 9);
    assert_eq!(segments.state().percent(), 100.0);
}

#[rstest]
fn skip_counts_blank_items_before_skip_empty(sandbox: Sandbox) {
    let path = sandbox.file(b"foo,,bar,,baz");
    let reader = sandbox.reader();
    let kept = items(reader.segments(&path).skip(2).build().unwrap());
    let filtered = items(reader.segments(&path).skip(2).skip_empty(true).build().unwrap());

    assert_eq!(texts(&kept), ["bar", "", "baz"]);
    assert_eq!(texts(&filtered), ["bar", "baz"]);
}

#[rstest]
fn align_skips_a_partial_segment(sandbox: Sandbox) {
    let path = sandbox.file(TEST_SEGMENT_CONTENT);
    let reader = sandbox.reader();
    let mut aligned = reader.segments(&path).start(2).align(true).build().unwrap();
    let mut partial = reader.segments(&path).start(2).build().unwrap();

    assert_eq!(aligned.read().unwrap(), Some(b"seg-1".as_slice()));
    assert_eq!(partial.read().unwrap(), Some(b"g-0".as_slice()));
}

#[cfg(unix)]
#[rstest]
fn read_fails_when_io_fails_mid_segment(sandbox: Sandbox) {
    let directory = sandbox.dir_at("folder");
    let opened = sandbox.state_at(&directory, TEST_READ_FROM);
    let mut segments = sandbox.lenient().segments(&directory).state(opened).build().unwrap();

    *segments.state_mut() = sandbox.state_at(&directory, TEST_REWOUND_TO);

    assert_err_is!(segments.read(), Error::Io(_));

    assert_eq!(segments.state().position, TEST_REWOUND_TO);

    assert_err_is!(segments.read(), Error::Io(_));

    assert_eq!(segments.state().position, TEST_READ_FROM);
}

#[rstest]
fn resume_ignores_align_and_skip(sandbox: Sandbox) {
    let path = sandbox.file(TEST_SEGMENT_CONTENT);
    let reader = sandbox.reader();
    let mut first = reader
        .segments(&path)
        .state(TEST_STATE_NAME)
        .start(14)
        .align(true)
        .build()
        .unwrap();

    consume(&mut first, 2);

    let state = first.save().unwrap().clone();

    let resumed = items(
        reader
            .segments(&path)
            .state(state)
            .start(0)
            .skip(5)
            .align(true)
            .build()
            .unwrap(),
    );

    assert_eq!(texts(&resumed), SEGMENTS[4..]);
}

#[apply(delimiters)]
fn read_matches_std_split(
    sandbox: Sandbox,
    #[values(
        EMPTY,
        TEST_SEGMENT_CONTENT,
        TEST_BLANK_SEGMENT_CONTENT,
        UTF8_SEGMENTS,
        ALL_BYTES
    )]
    content: &[u8],
    #[case] delimiter: u8,
    #[values(1, 2, 64, 8192)] capacity: usize,
) {
    let path = sandbox.file(content);
    let reader = sandbox.capped(capacity);
    let builder = || reader.segments(&path).delimiter(delimiter);
    let stripped: Vec<Vec<u8>> = BufRead::split(content, delimiter).map(Result::unwrap).collect();
    let kept: Vec<&[u8]> = content.split_inclusive(|byte| *byte == delimiter).collect();

    assert_eq!(items(builder().build().unwrap()), stripped);
    assert_eq!(items(builder().keep(true).build().unwrap()), kept);
}

#[rstest]
fn growth_resumes_an_unterminated_segment_whole(sandbox: Sandbox) {
    let path = sandbox.file(b"foo,bar");
    let reader = sandbox.reader();
    let mut segments = reader.segments(&path).state(TEST_STATE_NAME).build().unwrap();

    consume(&mut segments, 1);

    sandbox.append(&path, b"more");

    drain(&mut segments);

    segments.save().unwrap();

    let resynced = sandbox.states().resync(TEST_STATE_NAME, &path).unwrap();
    let resumed = items(reader.segments(&path).state(resynced).build().unwrap());

    assert_eq!(texts(&resumed), ["barmore"]);
}

#[rstest]
fn growth_keeps_a_new_segment_apart(sandbox: Sandbox) {
    let path = sandbox.file(b"foo,bar");
    let reader = sandbox.reader();
    let mut segments = reader.segments(&path).state(TEST_STATE_NAME).build().unwrap();

    consume(&mut segments, 1);

    sandbox.append(&path, b",more");

    drain(&mut segments);

    segments.save().unwrap();

    let resynced = sandbox.states().resync(TEST_STATE_NAME, &path).unwrap();
    let resumed = items(reader.segments(&path).state(resynced).build().unwrap());

    assert_eq!(texts(&resumed), ["bar", "more"]);
}

#[rstest]
fn resume_applies_a_different_delimiter(sandbox: Sandbox) {
    let path = sandbox.file(b"seg-0,seg-1;seg-2,seg-3");
    let reader = sandbox.reader();
    let mut segments = reader
        .segments(&path)
        .state(TEST_STATE_NAME)
        .delimiter(TEST_DEFAULT_DELIMITER)
        .limit(1)
        .build()
        .unwrap();

    drain(&mut segments);

    let state = segments.save().unwrap().clone();

    let read = items(reader.segments(&path).state(state).delimiter(b';').build().unwrap());

    assert_eq!(texts(&read), ["seg-1", "seg-2,seg-3"]);
}

#[rstest]
fn threshold_saves_during_iteration(sandbox: Sandbox) {
    let path = sandbox.file(TEST_SEGMENT_CONTENT);
    let reader = sandbox.autosaving(10);
    let mut segments = reader.segments(&path).state(TEST_STATE_NAME).build().unwrap();
    let mut positions = Vec::new();

    while segments.read().unwrap().is_some() {
        if let Some(state) = sandbox.states().find(TEST_STATE_NAME)
            && positions.last() != Some(&state.position)
        {
            positions.push(state.position);
        }
    }

    assert_eq!(positions, [12, 24, 36, 48]);
    assert_eq!(
        sandbox.states().load(TEST_STATE_NAME).unwrap().position,
        TEST_SEGMENT_CONTENT.len() as u64
    );
}

#[rstest]
fn resume_inside_an_item_ignores_align_and_skip(sandbox: Sandbox) {
    let path = sandbox.file(TEST_SEGMENT_CONTENT);
    let reader = sandbox.reader();
    let state = reader
        .segments(&path)
        .state(TEST_STATE_NAME)
        .start(14)
        .build()
        .unwrap()
        .save()
        .unwrap()
        .clone();

    let resumed = items(
        reader
            .segments(&path)
            .state(state)
            .start(0)
            .skip(5)
            .align(true)
            .build()
            .unwrap(),
    );

    assert_eq!(
        texts(&resumed),
        [
            "g-2", "seg-3", "seg-4", "seg-5", "seg-6", "seg-7", "seg-8", "seg-9"
        ]
    );
}

#[rstest]
fn resumed_read_floors_the_last_saved_position(sandbox: Sandbox) {
    let path = sandbox.file(TEST_SEGMENT_CONTENT);
    let mut first = sandbox.reader().segments(&path).state(TEST_STATE_NAME).build().unwrap();

    consume(&mut first, 3);

    let state = first.save().unwrap().clone();

    let saving = sandbox.autosaving(8);
    let mut resumed = saving.segments(&path).state(state).build().unwrap();

    consume(&mut resumed, 2);

    assert_eq!(sandbox.states().load(TEST_STATE_NAME).unwrap().position, 18);
}
