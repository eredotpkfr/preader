use std::{fs, io::ErrorKind};

use preader::{
    Config, DEFAULT_AUTO_SAVE_STATE_BYTES, DEFAULT_BUFFER_CAPACITY, Error, IteratorRead,
};
use rstest::rstest;

#[cfg(unix)]
use crate::common::constants::{TEST_READ_FROM, TEST_REWOUND_TO};
use crate::common::{
    constants::{
        TEST_BLANK_LINE_CONTENT, TEST_INVALID_UTF8, TEST_LINE, TEST_LINE_CONTENT, TEST_STATE_NAME,
        TEST_UNICODE_TEXT,
    },
    fixtures::sandbox,
    funcs::{consume, drain, items, try_items},
    macros::asserts::assert_err_is,
    matrix::contents::CRLF_LINES,
    sandbox::Sandbox,
};

const LINES: [&str; 6] = ["line-0", "line-1", "line-2", "line-3", "line-4", "line-5"];

fn invalid_lines() -> Vec<u8> {
    [b"line-0\n", TEST_INVALID_UTF8, b"\xfe\nline-2\n"].concat()
}

fn lines(sandbox: &Sandbox, content: &[u8]) -> Vec<String> {
    let path = sandbox.file(content);

    items(sandbox.reader().lines(&path).build().unwrap())
}

#[rstest]
fn iterator_defaults(sandbox: Sandbox) {
    let path = sandbox.file(&[TEST_LINE, b"\n"].concat());
    let reader = sandbox.reader();
    let defaults = items(reader.lines(&path).build().unwrap());
    let explicit =
        items(reader.lines(&path).keepends(false).skip_empty(false).skip(0).build().unwrap());

    assert_eq!(defaults, ["foo", ""]);
    assert_eq!(defaults, explicit);
}

#[rstest]
fn splits_on_newline(sandbox: Sandbox) {
    assert_eq!(lines(&sandbox, TEST_LINE_CONTENT), LINES);
}

#[rstest]
#[case::inside_the_line(b"foo\rbar\rbaz", &["foo\rbar\rbaz"])]
#[case::at_the_end(b"foo\rbar\r", &["foo\rbar"])]
fn lone_carriage_return_is_not_a_line_separator(
    sandbox: Sandbox,
    #[case] content: &[u8],
    #[case] expected: &[&str],
) {
    assert_eq!(lines(&sandbox, content), expected);
}

#[rstest]
fn crlf_strips_both_carriage_return_and_newline(sandbox: Sandbox) {
    assert_eq!(lines(&sandbox, CRLF_LINES), ["line-0", "line-1"]);
}

#[rstest]
fn read_keeps_a_blank_line(sandbox: Sandbox) {
    assert_eq!(
        lines(&sandbox, TEST_BLANK_LINE_CONTENT),
        ["line-0", "", "line-2"]
    );
}

#[rstest]
fn mixed_line_endings_are_both_stripped(sandbox: Sandbox) {
    assert_eq!(lines(&sandbox, b"foo\nbar\r\nbaz\n"), ["foo", "bar", "baz"]);
}

#[rstest]
fn reads_multi_byte_characters_within_one_line(sandbox: Sandbox) {
    let path = sandbox.file(TEST_UNICODE_TEXT.as_bytes());
    let read = items(sandbox.reader().lines(&path).build().unwrap());

    assert_eq!(read, [TEST_UNICODE_TEXT]);
}

#[rstest]
fn skipped_content_is_not_validated(sandbox: Sandbox) {
    let path = sandbox.file(&invalid_lines());
    let read = items(sandbox.reader().lines(&path).skip(2).build().unwrap());

    assert_eq!(read, ["line-2"]);
}

#[rstest]
fn fails_when_a_line_is_not_utf8(sandbox: Sandbox) {
    let path = sandbox.file(b"\xff\xfe\n");
    let mut lines = sandbox.reader().lines(&path).build().unwrap();

    assert_err_is!(lines.read(), Error::Utf8(_));
}

#[rstest]
#[case::neither(false, false, &["", "", ""])]
#[case::skip_empty(false, true, &[])]
#[case::keepends(true, false, &["\n", "\n", "\n"])]
#[case::both(true, true, &[])]
fn blank_only_file_follows_keepends_and_skip_empty(
    sandbox: Sandbox,
    #[case] keepends: bool,
    #[case] skip_empty: bool,
    #[case] expected: &[&str],
) {
    let path = sandbox.file(b"\n\n\n");
    let read = items(
        sandbox
            .reader()
            .lines(&path)
            .keepends(keepends)
            .skip_empty(skip_empty)
            .build()
            .unwrap(),
    );

    assert_eq!(read, expected);
}

#[rstest]
#[case::neither(false, false, &["foo", "", "bar"])]
#[case::skip_empty(false, true, &["foo", "bar"])]
#[case::keepends(true, false, &["foo\n", "\n", "bar\n"])]
#[case::both(true, true, &["foo\n", "bar\n"])]
fn blank_lines_follow_keepends_and_skip_empty(
    sandbox: Sandbox,
    #[case] keepends: bool,
    #[case] skip_empty: bool,
    #[case] expected: &[&str],
) {
    let path = sandbox.file(b"foo\n\nbar\n");
    let read = items(
        sandbox
            .reader()
            .lines(&path)
            .keepends(keepends)
            .skip_empty(skip_empty)
            .build()
            .unwrap(),
    );

    assert_eq!(read, expected);
}

#[rstest]
fn keepends_leaves_the_last_line_unterminated(sandbox: Sandbox) {
    let path = sandbox.file(b"foo\nbar");
    let read = items(sandbox.reader().lines(&path).keepends(true).build().unwrap());

    assert_eq!(read, ["foo\n", "bar"]);
}

#[rstest]
fn skip_empty_skips_an_unterminated_carriage_return(sandbox: Sandbox) {
    let path = sandbox.file(b"line-0\n\r");
    let read = items(sandbox.reader().lines(&path).skip_empty(true).build().unwrap());

    assert_eq!(read, ["line-0"]);
}

#[rstest]
fn align_is_a_no_op_on_a_line_boundary(sandbox: Sandbox) {
    let path = sandbox.file(TEST_BLANK_LINE_CONTENT);
    let mut aligned = sandbox.reader().lines(&path).start(7).align(true).build().unwrap();

    assert_eq!(aligned.read().unwrap(), Some(""));
}

#[rstest]
fn align_saturates_a_maximum_skip(sandbox: Sandbox) {
    let path = sandbox.file(TEST_LINE_CONTENT);
    let read = items(
        sandbox
            .reader()
            .lines(&path)
            .start(2)
            .align(true)
            .skip(u64::MAX)
            .build()
            .unwrap(),
    );

    assert!(read.is_empty());
}

#[rstest]
#[case::stripped(false, &["foo", "bar"])]
#[case::kept(true, &["foo\r\r\n", "bar\r\r\r\n"])]
fn repeated_carriage_returns_belong_to_the_terminator(
    sandbox: Sandbox,
    #[case] keepends: bool,
    #[case] expected: &[&str],
) {
    let path = sandbox.file(b"foo\r\r\nbar\r\r\r\n");
    let read = items(sandbox.reader().lines(&path).keepends(keepends).build().unwrap());

    assert_eq!(read, expected);
}

#[rstest]
fn skipped_blank_line_does_not_consume_the_limit(sandbox: Sandbox) {
    let path = sandbox.file(TEST_BLANK_LINE_CONTENT);
    let read = items(sandbox.reader().lines(&path).skip_empty(true).limit(2).build().unwrap());

    assert_eq!(read, ["line-0", "line-2"]);
}

#[rstest]
fn position_counts_bytes_not_characters(sandbox: Sandbox) {
    let path = sandbox.file("café\nfoo\n".as_bytes());
    let mut lines = sandbox.reader().lines(&path).build().unwrap();

    consume(&mut lines, 2);

    assert_eq!(lines.state().position, 6);
}

#[rstest]
#[case::aligned_both(true, true, true, &["line-2\n"])]
#[case::aligned_keepends(true, true, false, &["\n", "line-2\n"])]
#[case::aligned_skip_empty(true, false, true, &["line-2"])]
#[case::aligned_neither(true, false, false, &["", "line-2"])]
#[case::unaligned_both(false, true, true, &["e-0\n", "line-2\n"])]
#[case::unaligned_keepends(false, true, false, &["e-0\n", "\n", "line-2\n"])]
#[case::unaligned_skip_empty(false, false, true, &["e-0", "line-2"])]
#[case::unaligned_neither(false, false, false, &["e-0", "", "line-2"])]
fn flag_combinations_narrow_the_output(
    sandbox: Sandbox,
    #[case] align: bool,
    #[case] keepends: bool,
    #[case] skip_empty: bool,
    #[case] expected: &[&str],
) {
    let path = sandbox.file(TEST_BLANK_LINE_CONTENT);
    let read = items(
        sandbox
            .reader()
            .lines(&path)
            .start(3)
            .keepends(keepends)
            .align(align)
            .skip_empty(skip_empty)
            .build()
            .unwrap(),
    );

    assert_eq!(read, expected);
}

#[rstest]
fn crlf_keepends_preserves_the_full_terminator(sandbox: Sandbox) {
    let path = sandbox.file(CRLF_LINES);
    let read = items(sandbox.reader().lines(&path).keepends(true).build().unwrap());

    assert_eq!(read, ["line-0\r\n", "line-1\r\n"]);
}

#[rstest]
fn reads_multi_byte_characters(sandbox: Sandbox) {
    let words: Vec<&str> = TEST_UNICODE_TEXT.split_whitespace().collect();
    let content: String = words.iter().map(|word| format!("{word}\n")).collect();

    assert_eq!(lines(&sandbox, content.as_bytes()), words);
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
    let path = sandbox.file(TEST_LINE_CONTENT);
    let mut lines = sandbox
        .reader()
        .lines(&path)
        .skip(skip)
        .start(start)
        .align(align)
        .build()
        .unwrap();

    consume(&mut lines, 1);

    assert_eq!(lines.state().position, 7);
}

#[rstest]
fn align_and_skip_count_a_blank_line(sandbox: Sandbox) {
    let path = sandbox.file(TEST_BLANK_LINE_CONTENT);
    let mut lines = sandbox.reader().lines(&path).start(2).align(true).skip(1).build().unwrap();

    assert_eq!(lines.read().unwrap(), Some("line-2"));
}

#[rstest]
fn skip_empty_skips_crlf_only_lines(sandbox: Sandbox) {
    let path = sandbox.file(b"line-0\n\r\n\r\nline-1\n");
    let read = items(sandbox.reader().lines(&path).skip_empty(true).build().unwrap());

    assert_eq!(read, ["line-0", "line-1"]);
}

#[rstest]
fn skipping_holds_back_an_unterminated_line(sandbox: Sandbox) {
    let path = sandbox.file(b"foo\nbar");
    let mut lines = sandbox.reader().lines(&path).skip(2).build().unwrap();

    sandbox.append(&path, b"more");

    assert!(items(&mut lines).is_empty());

    assert_eq!(lines.state().position, 4);
}

#[rstest]
fn skipping_past_a_terminated_line_ignores_growth(sandbox: Sandbox) {
    let path = sandbox.file(b"foo\nbar\n");
    let mut lines = sandbox.reader().lines(&path).skip(2).build().unwrap();

    sandbox.append(&path, b"more");

    assert!(items(&mut lines).is_empty());

    assert_eq!(lines.state().position, 8);
}

#[rstest]
fn end_yields_a_crossing_line_whole(sandbox: Sandbox) {
    let path = sandbox.file(TEST_BLANK_LINE_CONTENT);
    let mut lines = sandbox.reader().lines(&path).end(10).build().unwrap();
    let read = items(&mut lines);

    assert_eq!(read, ["line-0", "", "line-2"]);
    assert_eq!(lines.state().position, TEST_BLANK_LINE_CONTENT.len() as u64);
}

#[rstest]
fn fails_when_a_save_fails_on_a_skipped_item(sandbox: Sandbox) {
    let path = sandbox.file(TEST_LINE_CONTENT);

    sandbox.block_states();

    let reader = sandbox.autosaving(1);
    let mut lines = reader.lines(&path).state(TEST_STATE_NAME).skip(2).build().unwrap();

    assert_err_is!(lines.read(), Error::Io(error) if error.kind() == ErrorKind::AlreadyExists);

    assert_eq!(lines.state().position, 7);
}

#[rstest]
fn position_counts_finished_items(sandbox: Sandbox) {
    let path = sandbox.file(TEST_LINE_CONTENT);
    let mut lines = sandbox.reader().lines(&path).build().unwrap();

    consume(&mut lines, 1);

    assert_eq!(lines.state().position, 0);
    assert_eq!(lines.percent(), 0.0);

    consume(&mut lines, 1);

    assert_eq!(lines.state().position, 7);
}

#[rstest]
#[case::file_start(0, 0)]
#[case::item_boundary(7, 1)]
fn align_skips_nothing_when_already_aligned(
    sandbox: Sandbox,
    #[case] start: u64,
    #[case] index: usize,
) {
    let path = sandbox.file(TEST_LINE_CONTENT);
    let reader = sandbox.reader();
    let aligned = items(reader.lines(&path).start(start).align(true).build().unwrap());
    let unaligned = items(reader.lines(&path).start(start).build().unwrap());

    assert_eq!(aligned, unaligned);
    assert_eq!(aligned, LINES[index..]);
}

#[rstest]
fn read_fails_when_a_line_is_not_utf8_before_any_save(sandbox: Sandbox) {
    let path = sandbox.file(b"foo\n\xff\xfe\n");
    let reader = sandbox.autosaving(DEFAULT_AUTO_SAVE_STATE_BYTES.get());
    let mut lines = reader.lines(&path).state(TEST_STATE_NAME).build().unwrap();

    sandbox.block_states();

    assert_eq!(lines.read().unwrap(), Some("foo"));

    assert_err_is!(lines.read(), Error::Utf8(_));
}

#[rstest]
fn fails_when_a_save_fails_on_a_filtered_blank(sandbox: Sandbox) {
    let path = sandbox.file(b"\n\nfoo\n");

    sandbox.block_states();

    let reader = sandbox.autosaving(1);
    let mut lines = reader.lines(&path).state(TEST_STATE_NAME).skip_empty(true).build().unwrap();

    assert_err_is!(lines.read(), Error::Io(error) if error.kind() == ErrorKind::AlreadyExists);

    assert_eq!(lines.state().position, 1);
}

#[rstest]
fn resumed_read_ignores_align(sandbox: Sandbox) {
    let path = sandbox.file(TEST_BLANK_LINE_CONTENT);
    let state = sandbox.checkpoint(&path, TEST_STATE_NAME, 3);
    let reader = sandbox.reader();

    let resumed = items(reader.lines(&path).state(state).start(2).align(true).build().unwrap());

    assert_eq!(resumed, ["e-0", "", "line-2"]);
}

#[rstest]
#[case::refilled(1)]
#[case::default(DEFAULT_BUFFER_CAPACITY)]
fn growth_holds_back_an_unterminated_line(sandbox: Sandbox, #[case] capacity: usize) {
    let path = sandbox.file(b"foo\nbar");
    let mut lines = sandbox.capped(capacity).lines(&path).build().unwrap();

    consume(&mut lines, 1);

    sandbox.append(&path, b"more");

    assert!(items(&mut lines).is_empty());

    assert_eq!(lines.state().position, 4);
    assert_eq!(lines.percent(), 400.0 / 7.0);
}

#[rstest]
fn yields_every_ascii_byte_value(sandbox: Sandbox) {
    let content: Vec<u8> = (0..128).collect();
    let path = sandbox.file(&content);
    let read = items(sandbox.reader().lines(&path).keepends(true).build().unwrap());

    assert_eq!(
        read,
        [
            String::from_utf8(content[..11].to_vec()).unwrap(),
            String::from_utf8(content[11..].to_vec()).unwrap(),
        ]
    );
}

#[rstest]
fn file_growth_during_iteration_is_ignored(sandbox: Sandbox) {
    let path = sandbox.file(b"foo\nbar\n");
    let mut lines = sandbox.reader().lines(&path).build().unwrap();

    consume(&mut lines, 1);

    sandbox.append(&path, b"more");

    assert_eq!(items(&mut lines), ["bar"]);
    assert_eq!(lines.state().position, 8);
}

#[rstest]
fn fails_when_the_aligned_file_is_replaced_by_a_directory_without_verification(sandbox: Sandbox) {
    let path = sandbox.file(TEST_LINE_CONTENT);
    let reader = sandbox.lenient();
    let state = reader.lines(&path).state(TEST_STATE_NAME).build().unwrap().state().clone();

    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();

    assert_err_is!(
        reader.lines(&path).state(state).start(5).align(true).build(),
        Error::Io(_)
    );
}

#[rstest]
fn align_skips_a_partial_line(sandbox: Sandbox) {
    let path = sandbox.file(TEST_BLANK_LINE_CONTENT);
    let reader = sandbox.reader();
    let mut aligned = reader.lines(&path).start(2).align(true).build().unwrap();
    let mut partial = reader.lines(&path).start(2).build().unwrap();

    assert_eq!(aligned.read().unwrap(), Some(""));
    assert_eq!(partial.read().unwrap(), Some("ne-0"));
}

#[rstest]
fn keepends_does_not_change_the_position(sandbox: Sandbox) {
    let path = sandbox.file(TEST_LINE_CONTENT);
    let reader = sandbox.reader();
    let mut stripped = reader.lines(&path).build().unwrap();
    let mut kept = reader.lines(&path).keepends(true).build().unwrap();

    drain(&mut stripped);
    drain(&mut kept);

    assert_eq!(stripped.state().position, kept.state().position);
    assert_eq!(kept.state().position, TEST_LINE_CONTENT.len() as u64);
}

#[rstest]
fn skip_stops_at_a_truncation(sandbox: Sandbox) {
    let path = sandbox.file(TEST_LINE_CONTENT);
    let reader = sandbox.lenient();

    sandbox.saved(&path, TEST_STATE_NAME);
    sandbox.truncate(&path, 8);

    let mut resumed = reader.lines(&path).state(TEST_STATE_NAME).skip(3).build().unwrap();

    assert!(items(&mut resumed).is_empty());

    assert_eq!(resumed.state().position, 8);
    assert_eq!(resumed.state().file.size, TEST_LINE_CONTENT.len() as u64);
}

#[rstest]
fn align_and_skip_combine(sandbox: Sandbox) {
    let path = sandbox.file(TEST_LINE_CONTENT);
    let reader = sandbox.reader();
    let aligned = items(reader.lines(&path).start(9).skip(1).align(true).build().unwrap());
    let unaligned = items(reader.lines(&path).start(9).skip(1).build().unwrap());

    assert_eq!(aligned, LINES[3..]);
    assert_eq!(unaligned, LINES[2..]);
}

#[rstest]
fn fails_when_a_save_fails_while_skipping(sandbox: Sandbox) {
    let path = sandbox.file(TEST_LINE_CONTENT);

    sandbox.block_states();

    let reader = sandbox.reader_with(Config {
        auto_save_state: true,
        ..sandbox.config()
    });
    let mut lines = reader.lines(&path).state(TEST_STATE_NAME).skip(3).build().unwrap();

    sandbox.truncate(&path, 1);

    assert_err_is!(lines.read(), Error::Io(error) if error.kind() == ErrorKind::AlreadyExists);

    assert_eq!(lines.state().position, 1);
}

#[rstest]
fn resume_applies_a_different_keepends(sandbox: Sandbox) {
    let path = sandbox.file(TEST_LINE_CONTENT);
    let reader = sandbox.reader();
    let mut lines = reader.lines(&path).state(TEST_STATE_NAME).limit(1).build().unwrap();

    drain(&mut lines);

    let state = lines.save().unwrap().clone();

    let read = items(reader.lines(&path).state(state).keepends(true).limit(1).build().unwrap());

    assert_eq!(read, ["line-1\n"]);
}

#[rstest]
fn skip_counts_blank_items_before_skip_empty(sandbox: Sandbox) {
    let path = sandbox.file(b"foo\n\nbar\n\nbaz\n");
    let reader = sandbox.reader();
    let kept = items(reader.lines(&path).skip(2).build().unwrap());
    let filtered = items(reader.lines(&path).skip(2).skip_empty(true).build().unwrap());

    assert_eq!(kept, ["bar", "", "baz"]);
    assert_eq!(filtered, ["bar", "baz"]);
}

#[cfg(unix)]
#[rstest]
fn read_fails_when_io_fails_mid_line(sandbox: Sandbox) {
    let directory = sandbox.dir_at("folder");
    let opened = sandbox.state_at(&directory, TEST_READ_FROM);
    let mut lines = sandbox.lenient().lines(&directory).state(opened).build().unwrap();

    *lines.state_mut() = sandbox.state_at(&directory, TEST_REWOUND_TO);

    assert_err_is!(lines.read(), Error::Io(_));

    assert_eq!(lines.state().position, TEST_REWOUND_TO);

    assert_err_is!(lines.read(), Error::Io(_));

    assert_eq!(lines.state().position, TEST_READ_FROM);
}

#[rstest]
fn resume_ignores_align_and_skip(sandbox: Sandbox) {
    let path = sandbox.file(TEST_LINE_CONTENT);
    let reader = sandbox.reader();
    let mut first =
        reader.lines(&path).state(TEST_STATE_NAME).start(9).align(true).build().unwrap();

    consume(&mut first, 2);

    let state = first.save().unwrap().clone();

    let resumed =
        items(reader.lines(&path).state(state).start(0).skip(5).align(true).build().unwrap());

    assert_eq!(resumed, LINES[3..]);
}

#[rstest]
fn resume_rereads_an_invalid_line(sandbox: Sandbox) {
    let path = sandbox.file(&[TEST_LINE, TEST_INVALID_UTF8, b"\nbar\n"].concat());
    let reader = sandbox.autosaving(DEFAULT_AUTO_SAVE_STATE_BYTES.get());

    assert_err_is!(
        try_items(reader.lines(&path).state(TEST_STATE_NAME).build().unwrap()),
        Error::Utf8(_)
    );

    let mut resumed = reader.lines(&path).state(TEST_STATE_NAME).build().unwrap();

    assert_err_is!(resumed.read(), Error::Utf8(_));

    assert_eq!(resumed.read().unwrap(), Some("bar"));
}

#[rstest]
fn growth_resumes_an_unterminated_line_whole(sandbox: Sandbox) {
    let path = sandbox.file(b"foo\nbar");
    let reader = sandbox.reader();
    let mut lines = reader.lines(&path).state(TEST_STATE_NAME).build().unwrap();

    consume(&mut lines, 1);

    sandbox.append(&path, b"more");

    drain(&mut lines);

    lines.save().unwrap();

    let resynced = sandbox.states().resync(TEST_STATE_NAME, &path).unwrap();
    let resumed = items(reader.lines(&path).state(resynced).build().unwrap());

    assert_eq!(resumed, ["barmore"]);
}

#[rstest]
fn growth_keeps_a_new_line_apart(sandbox: Sandbox) {
    let path = sandbox.file(b"foo\nbar");
    let reader = sandbox.reader();
    let mut lines = reader.lines(&path).state(TEST_STATE_NAME).build().unwrap();

    consume(&mut lines, 1);

    sandbox.append(&path, b"\nmore");

    drain(&mut lines);

    lines.save().unwrap();

    let resynced = sandbox.states().resync(TEST_STATE_NAME, &path).unwrap();
    let resumed = items(reader.lines(&path).state(resynced).build().unwrap());

    assert_eq!(resumed, ["bar", "more"]);
}

#[rstest]
fn threshold_saves_during_iteration(sandbox: Sandbox) {
    let path = sandbox.file(TEST_LINE_CONTENT);
    let reader = sandbox.autosaving(10);
    let mut lines = reader.lines(&path).state(TEST_STATE_NAME).build().unwrap();
    let mut positions = Vec::new();

    while lines.read().unwrap().is_some() {
        if let Some(state) = sandbox.states().find(TEST_STATE_NAME)
            && positions.last() != Some(&state.position)
        {
            positions.push(state.position);
        }
    }

    assert_eq!(positions, [14, 28]);
    assert_eq!(
        sandbox.states().load(TEST_STATE_NAME).unwrap().position,
        TEST_LINE_CONTENT.len() as u64
    );
}

#[rstest]
fn resume_inside_an_item_ignores_align_and_skip(sandbox: Sandbox) {
    let path = sandbox.file(TEST_LINE_CONTENT);
    let reader = sandbox.reader();
    let state = reader
        .lines(&path)
        .state(TEST_STATE_NAME)
        .start(9)
        .build()
        .unwrap()
        .save()
        .unwrap()
        .clone();

    let resumed =
        items(reader.lines(&path).state(state).start(0).skip(5).align(true).build().unwrap());

    assert_eq!(resumed, ["ne-1", "line-2", "line-3", "line-4", "line-5"]);
}

#[rstest]
fn resume_rereads_the_invalid_line_after_a_manual_save(sandbox: Sandbox) {
    let path = sandbox.file(&[TEST_LINE, TEST_INVALID_UTF8, b"\nbar\n"].concat());
    let reader = sandbox.reader();
    let mut lines = reader.lines(&path).state(TEST_STATE_NAME).build().unwrap();

    lines.read().unwrap();

    assert_err_is!(lines.read(), Error::Utf8(_));

    lines.save().unwrap();

    let mut resumed = reader.lines(&path).state(TEST_STATE_NAME).build().unwrap();

    assert_err_is!(resumed.read(), Error::Utf8(_));

    assert_eq!(resumed.read().unwrap(), Some("bar"));
}

#[rstest]
fn resumed_read_floors_the_last_saved_position(sandbox: Sandbox) {
    let path = sandbox.file(TEST_LINE_CONTENT);
    let mut first = sandbox.reader().lines(&path).state(TEST_STATE_NAME).build().unwrap();

    consume(&mut first, 3);

    let state = first.save().unwrap().clone();

    let saving = sandbox.autosaving(10);
    let mut resumed = saving.lines(&path).state(state).build().unwrap();

    consume(&mut resumed, 2);

    assert_eq!(sandbox.states().load(TEST_STATE_NAME).unwrap().position, 21);
}

#[rstest]
fn resume_after_an_invalid_line_stays_aligned(sandbox: Sandbox) {
    let path = sandbox.file(&[TEST_LINE, TEST_INVALID_UTF8, b"\nbar\nbaz\n"].concat());
    let reader = sandbox.autosaving(1);
    let mut lines = reader.lines(&path).state(TEST_STATE_NAME).build().unwrap();

    assert_eq!(lines.read().unwrap(), Some("foo"));

    assert_err_is!(lines.read(), Error::Utf8(_));

    assert_eq!(items(&mut lines), ["bar", "baz"]);
    assert_eq!(sandbox.states().load(TEST_STATE_NAME).unwrap().position, 14);

    assert!(items(reader.lines(&path).state(TEST_STATE_NAME).build().unwrap()).is_empty());
}

#[rstest]
fn read_counts_the_invalid_line(sandbox: Sandbox) {
    let path = sandbox.file(&invalid_lines());
    let mut lines = sandbox.reader().lines(&path).build().unwrap();

    assert_eq!(lines.read().unwrap(), Some("line-0"));

    assert_err_is!(lines.read(), Error::Utf8(_));

    assert_eq!(lines.state().position, 7);
    assert_eq!(lines.read().unwrap(), Some("line-2"));
    assert_eq!(lines.state().position, 10);
    assert_eq!(lines.read().unwrap(), None);
    assert_eq!(lines.state().position, invalid_lines().len() as u64);
    assert_eq!(lines.percent(), 100.0);
}
