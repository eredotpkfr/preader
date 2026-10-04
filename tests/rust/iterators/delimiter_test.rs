use preader::{DEFAULT_DELIMITER, Error, IteratorBuild, IteratorRead};
use rstest::rstest;

use crate::common::{
    constants::{
        TEST_BLANK_SEGMENT_CONTENT, TEST_INVALID_UTF8, TEST_READ_FROM, TEST_REWOUND_TO,
        TEST_SEGMENT_CONTENT,
    },
    fixtures::sandbox,
    funcs::{drain, items, texts},
    macros::asserts::assert_err_is,
    sandbox::Sandbox,
};

const SEGMENTS: [&str; 10] = [
    "seg-0", "seg-1", "seg-2", "seg-3", "seg-4", "seg-5", "seg-6", "seg-7", "seg-8", "seg-9",
];

fn segments(sandbox: &Sandbox, content: &[u8]) -> Vec<String> {
    let path = sandbox.file(content);

    texts(&items(sandbox.reader().delimiter(&path).build().unwrap()))
}

#[rstest]
fn read_splits_on_the_delimiter(sandbox: Sandbox) {
    assert_eq!(segments(&sandbox, TEST_SEGMENT_CONTENT), SEGMENTS);
}

#[rstest]
fn read_keeps_a_blank_segment(sandbox: Sandbox) {
    assert_eq!(
        segments(&sandbox, TEST_BLANK_SEGMENT_CONTENT),
        ["seg-0", "", "seg-2"]
    );
}

#[rstest]
fn delimiter_only_file_yields_blank_segments(sandbox: Sandbox) {
    assert_eq!(segments(&sandbox, b",,,"), ["", "", ""]);
}

#[rstest]
fn file_without_delimiter_yields_one_segment(sandbox: Sandbox) {
    assert_eq!(
        segments(&sandbox, b"no-delimiter-here"),
        ["no-delimiter-here"]
    );
}

#[rstest]
fn trailing_delimiter_yields_no_extra_segment(sandbox: Sandbox) {
    assert_eq!(segments(&sandbox, b"seg-0,seg-1,"), ["seg-0", "seg-1"]);
}

#[rstest]
#[case::null(b'\0')]
#[case::newline(b'\n')]
#[case::high_byte(0xE9)]
fn character_selects_the_separator(sandbox: Sandbox, #[case] character: u8) {
    let content = [b"seg-0", [character].as_slice(), b"seg-1"].concat();
    let path = sandbox.file(&content);
    let read = items(sandbox.reader().delimiter(&path).character(character).build().unwrap());

    assert_eq!(texts(&read), ["seg-0", "seg-1"]);
}

#[rstest]
fn keep_does_not_change_the_position(sandbox: Sandbox) {
    let path = sandbox.file(TEST_SEGMENT_CONTENT);
    let reader = sandbox.reader();
    let mut stripped = reader.delimiter(&path).build().unwrap();
    let mut kept = reader.delimiter(&path).keep(true).build().unwrap();

    drain(&mut stripped);
    drain(&mut kept);

    assert_eq!(stripped.state().position, kept.state().position);
}

#[rstest]
fn skip_empty_judges_blankness_without_delimiter(sandbox: Sandbox) {
    let path = sandbox.file(TEST_BLANK_SEGMENT_CONTENT);
    let read =
        items(sandbox.reader().delimiter(&path).keep(true).skip_empty(true).build().unwrap());

    assert_eq!(texts(&read), ["seg-0,", "seg-2,"]);
}

#[rstest]
fn a_skipped_blank_segment_does_not_consume_the_limit(sandbox: Sandbox) {
    let path = sandbox.file(TEST_BLANK_SEGMENT_CONTENT);
    let read = items(sandbox.reader().delimiter(&path).skip_empty(true).limit(2).build().unwrap());

    assert_eq!(texts(&read), ["seg-0", "seg-2"]);
}

#[rstest]
fn read_splits_invalid_bytes(sandbox: Sandbox) {
    let content = [TEST_INVALID_UTF8, b",", TEST_INVALID_UTF8].concat();
    let path = sandbox.file(&content);
    let read = items(sandbox.reader().delimiter(&path).build().unwrap());

    assert_eq!(read, [TEST_INVALID_UTF8, TEST_INVALID_UTF8]);
}

#[rstest]
fn read_fills_segment_across_buffer_refill(sandbox: Sandbox) {
    let path = sandbox.file(TEST_SEGMENT_CONTENT);
    let reader = sandbox.capped(1);
    let read = items(reader.delimiter(&path).build().unwrap());

    assert_eq!(texts(&read), SEGMENTS);
}

#[rstest]
fn align_skips_segment_window_starts_inside(sandbox: Sandbox) {
    let path = sandbox.file(TEST_SEGMENT_CONTENT);
    let reader = sandbox.reader();
    let mut aligned = reader.delimiter(&path).start(2).align(true).build().unwrap();
    let mut partial = reader.delimiter(&path).start(2).build().unwrap();

    assert_eq!(aligned.read().unwrap(), Some(b"seg-1".as_slice()));
    assert_eq!(partial.read().unwrap(), Some(b"g-0".as_slice()));
}

#[rstest]
fn align_is_a_no_op_on_a_boundary(sandbox: Sandbox) {
    let path = sandbox.file(TEST_SEGMENT_CONTENT);
    let mut aligned = sandbox.reader().delimiter(&path).start(6).align(true).build().unwrap();

    assert_eq!(aligned.read().unwrap(), Some(b"seg-1".as_slice()));
}

#[rstest]
fn resumed_read_ignores_align(sandbox: Sandbox) {
    let path = sandbox.file(TEST_SEGMENT_CONTENT);
    let reader = sandbox.reader();
    let mut first = reader.bytes(&path).limit(3).build().unwrap();

    drain(&mut first);

    first.state().save().unwrap();

    let state = first.state().clone();

    drop(first);

    let mut resumed = reader.delimiter(&path).state(state).start(2).align(true).build().unwrap();

    assert_eq!(resumed.read().unwrap(), Some(b"-0".as_slice()));
}

#[rstest]
fn segment_crossing_the_end_is_yielded_whole(sandbox: Sandbox) {
    let path = sandbox.file(TEST_SEGMENT_CONTENT);
    let mut segments = sandbox.reader().delimiter(&path).end(8).build().unwrap();
    let read = items(&mut segments);

    assert_eq!(texts(&read), ["seg-0", "seg-1"]);
    assert_eq!(segments.state().position, 12);
}

#[rstest]
fn iterator_yields_owned_segments(sandbox: Sandbox) {
    let path = sandbox.file(TEST_SEGMENT_CONTENT);
    let mut collected: Vec<Vec<u8>> = Vec::new();

    for segment in sandbox.reader().delimiter(&path).character(DEFAULT_DELIMITER).build().unwrap() {
        collected.push(segment.unwrap());
    }

    assert_eq!(texts(&collected), SEGMENTS);
}

#[cfg(unix)]
#[rstest]
fn read_reports_a_mid_segment_io_error(sandbox: Sandbox) {
    let directory = sandbox.dir_at("folder");
    let opened = sandbox.state_at(&directory, TEST_READ_FROM);
    let mut segments = sandbox.lenient().delimiter(&directory).state(opened).build().unwrap();

    *segments.state() = sandbox.state_at(&directory, TEST_REWOUND_TO);

    assert_err_is!(segments.read(), Error::Io(_));

    assert_eq!(segments.state().position, TEST_READ_FROM);
}
