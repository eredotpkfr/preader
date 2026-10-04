use preader::{Error, IteratorBuild, IteratorRead};
use rstest::rstest;

use crate::common::{
    constants::{
        TEST_BLANK_LINE_CONTENT, TEST_CRLF_CONTENT, TEST_INVALID_LINES, TEST_LINE_CONTENT,
        TEST_LINES, TEST_UNICODE_TEXT,
    },
    fixtures::sandbox,
    funcs::{items, texts},
    sandbox::Sandbox,
};

fn lines(sandbox: &Sandbox, content: &[u8]) -> Vec<String> {
    let path = sandbox.file(content);

    texts(&items(sandbox.reader().lines(&path).build().unwrap()))
}

#[rstest]
fn read_strips_the_line_ending(sandbox: Sandbox) {
    assert_eq!(lines(&sandbox, TEST_LINE_CONTENT), TEST_LINES);
}

#[rstest]
fn read_keeps_a_blank_line(sandbox: Sandbox) {
    assert_eq!(
        lines(&sandbox, TEST_BLANK_LINE_CONTENT),
        ["line-0", "", "line-2"]
    );
}

#[rstest]
fn read_yields_blank_only_file_as_blank_lines(sandbox: Sandbox) {
    assert_eq!(lines(&sandbox, b"\n\n\n"), ["", "", ""]);
}

#[rstest]
fn read_strips_both_crlf_characters(sandbox: Sandbox) {
    assert_eq!(lines(&sandbox, TEST_CRLF_CONTENT), ["line-0", "line-1"]);
}

#[rstest]
fn read_strips_repeated_carriage_returns(sandbox: Sandbox) {
    assert_eq!(
        lines(&sandbox, b"line-0\r\r\nline-1\r\r\r\n"),
        ["line-0", "line-1"]
    );
}

#[rstest]
fn lone_carriage_return_is_not_a_separator(sandbox: Sandbox) {
    assert_eq!(lines(&sandbox, b"line-0\rline-1"), ["line-0\rline-1"]);
}

#[rstest]
fn keepends_preserves_the_full_crlf(sandbox: Sandbox) {
    let path = sandbox.file(TEST_CRLF_CONTENT);
    let read = items(sandbox.reader().lines(&path).keepends(true).build().unwrap());

    assert_eq!(texts(&read), ["line-0\r\n", "line-1\r\n"]);
}

#[rstest]
fn keepends_leaves_the_last_line_unterminated(sandbox: Sandbox) {
    let path = sandbox.file(TEST_LINE_CONTENT);
    let read = items(sandbox.reader().lines(&path).keepends(true).build().unwrap());

    assert_eq!(texts(&read).last().unwrap(), "line-5");
}

#[rstest]
fn keepends_does_not_change_the_position(sandbox: Sandbox) {
    let path = sandbox.file(TEST_LINE_CONTENT);
    let reader = sandbox.reader();
    let mut stripped = reader.lines(&path).build().unwrap();
    let mut kept = reader.lines(&path).keepends(true).build().unwrap();

    while stripped.read().unwrap().is_some() {}
    while kept.read().unwrap().is_some() {}

    assert_eq!(stripped.state().position, kept.state().position);
}

#[rstest]
fn skip_empty_judges_blankness_after_trimming(sandbox: Sandbox) {
    let path = sandbox.file(b"line-0\n\r\n\r\nline-1\n");
    let read = items(sandbox.reader().lines(&path).skip_empty(true).build().unwrap());

    assert_eq!(texts(&read), ["line-0", "line-1"]);
}

#[rstest]
fn skip_empty_drops_a_blank_only_file(sandbox: Sandbox) {
    let path = sandbox.file(b"\n\n\n");

    assert!(items(sandbox.reader().lines(&path).skip_empty(true).build().unwrap()).is_empty());
}

#[rstest]
fn read_yields_multi_byte_characters(sandbox: Sandbox) {
    let path = sandbox.file(TEST_UNICODE_TEXT.as_bytes());
    let read = items(sandbox.reader().lines(&path).build().unwrap());

    assert_eq!(texts(&read), [TEST_UNICODE_TEXT]);
}

#[rstest]
fn position_counts_bytes_not_characters(sandbox: Sandbox) {
    let path = sandbox.file(TEST_UNICODE_TEXT.as_bytes());
    let mut lines = sandbox.reader().lines(&path).build().unwrap();

    lines.read().unwrap();

    assert_eq!(lines.state().position, TEST_UNICODE_TEXT.len() as u64);
}

#[rstest]
fn read_fails_on_invalid_utf8(sandbox: Sandbox) {
    let path = sandbox.file(b"\xff\xfe\n");
    let mut lines = sandbox.reader().lines(&path).build().unwrap();

    assert!(matches!(lines.read().unwrap_err(), Error::Utf8(_)));
}

#[rstest]
fn read_counts_the_invalid_line(sandbox: Sandbox) {
    let path = sandbox.file(TEST_INVALID_LINES);
    let mut lines = sandbox.reader().lines(&path).build().unwrap();

    assert_eq!(lines.read().unwrap(), Some("line-0"));
    assert_eq!(lines.state().position, 7);
    assert!(lines.read().is_err());
    assert_eq!(lines.state().position, 10);
    assert_eq!(lines.read().unwrap(), Some("line-2"));
    assert_eq!(lines.read().unwrap(), None);
    assert_eq!(lines.state().position, TEST_INVALID_LINES.len() as u64);
    assert_eq!(lines.state().percent(), 100.0);
}

#[rstest]
fn resume_after_an_invalid_line_stays_aligned(sandbox: Sandbox) {
    let path = sandbox.file(TEST_INVALID_LINES);
    let reader = sandbox.resuming();
    let mut lines = reader.lines(&path).state("job-1").build().unwrap();

    lines.read().unwrap();
    lines.read().unwrap_err();
    lines.state().save().unwrap();
    drop(lines);

    let resumed = items(reader.lines(&path).state("job-1").build().unwrap());

    assert_eq!(texts(&resumed), ["line-2"]);
}

#[rstest]
fn skipped_content_is_not_validated(sandbox: Sandbox) {
    let path = sandbox.file(TEST_INVALID_LINES);
    let read = items(sandbox.reader().lines(&path).skip(2).build().unwrap());

    assert_eq!(texts(&read), ["line-2"]);
}

#[rstest]
fn align_skips_line_window_starts_inside(sandbox: Sandbox) {
    let path = sandbox.file(TEST_BLANK_LINE_CONTENT);
    let reader = sandbox.reader();
    let mut aligned = reader.lines(&path).start(2).align(true).build().unwrap();
    let mut partial = reader.lines(&path).start(2).build().unwrap();

    assert_eq!(aligned.read().unwrap(), Some(""));
    assert_eq!(partial.read().unwrap(), Some("ne-0"));
}

#[rstest]
fn align_is_a_no_op_on_a_line_boundary(sandbox: Sandbox) {
    let path = sandbox.file(TEST_BLANK_LINE_CONTENT);
    let mut aligned = sandbox.reader().lines(&path).start(7).align(true).build().unwrap();

    assert_eq!(aligned.read().unwrap(), Some(""));
}

#[rstest]
fn align_and_skip_combine(sandbox: Sandbox) {
    let path = sandbox.file(TEST_BLANK_LINE_CONTENT);
    let mut lines = sandbox.reader().lines(&path).start(2).align(true).skip(1).build().unwrap();

    assert_eq!(lines.read().unwrap(), Some("line-2"));
}

#[rstest]
fn resumed_read_ignores_align(sandbox: Sandbox) {
    let path = sandbox.file(TEST_BLANK_LINE_CONTENT);
    let reader = sandbox.reader();
    let mut first = reader.bytes(&path).limit(3).build().unwrap();

    while first.read().unwrap().is_some() {}

    first.state().save().unwrap();

    let state = first.state().clone();

    drop(first);

    let resumed = items(reader.lines(&path).state(state).start(2).align(true).build().unwrap());

    assert_eq!(texts(&resumed), ["e-0", "", "line-2"]);
}

#[rstest]
fn line_crossing_the_end_is_yielded_whole(sandbox: Sandbox) {
    let path = sandbox.file(TEST_BLANK_LINE_CONTENT);
    let mut lines = sandbox.reader().lines(&path).end(10).build().unwrap();
    let read = texts(&items(&mut lines));

    assert_eq!(read, ["line-0", "", "line-2"]);
    assert_eq!(lines.state().position, TEST_BLANK_LINE_CONTENT.len() as u64);
}

#[rstest]
fn iterator_yields_owned_lines(sandbox: Sandbox) {
    let path = sandbox.file(TEST_LINE_CONTENT);
    let mut collected: Vec<String> = Vec::new();

    for line in sandbox.reader().lines(&path).build().unwrap() {
        collected.push(line.unwrap());
    }

    assert_eq!(collected, TEST_LINES);
}
