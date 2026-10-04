use preader::IteratorBuild;
use rstest::rstest;

use crate::common::{
    constants::{TEST_BLANK_SEGMENT_CONTENT, TEST_DELIMITER, TEST_SEGMENT_CONTENT, TEST_SEGMENTS},
    fixtures::sandbox,
    funcs::{items, texts},
    sandbox::Sandbox,
};

#[rstest]
fn the_default_character_comes_from_the_constant(sandbox: Sandbox) {
    let path = sandbox.file(TEST_SEGMENT_CONTENT);
    let read = items(sandbox.reader().delimiter(&path).build().unwrap());

    assert_eq!(texts(&read), TEST_SEGMENTS);
}

#[rstest]
#[case::newline(b'\n')]
#[case::null(b'\0')]
#[case::letter(b'x')]
fn character_sets_the_boundary(sandbox: Sandbox, #[case] character: u8) {
    let content = [b"foo", [character].as_slice(), b"bar"].concat();
    let path = sandbox.file(&content);
    let read = items(sandbox.reader().delimiter(&path).character(character).build().unwrap());

    assert_eq!(read, [b"foo", b"bar"]);
}

#[rstest]
#[case::stripped(false, "seg-0")]
#[case::kept(true, "seg-0,")]
fn keep_decides_about_the_delimiter(sandbox: Sandbox, #[case] keep: bool, #[case] expected: &str) {
    let path = sandbox.file(TEST_SEGMENT_CONTENT);
    let read = items(sandbox.reader().delimiter(&path).keep(keep).limit(1).build().unwrap());

    assert_eq!(texts(&read), [expected]);
}

#[rstest]
#[case::kept(false, 3)]
#[case::skipped(true, 2)]
fn skip_empty_decides_about_blank_segments(
    sandbox: Sandbox,
    #[case] skip_empty: bool,
    #[case] expected: usize,
) {
    let path = sandbox.file(TEST_BLANK_SEGMENT_CONTENT);
    let read = items(sandbox.reader().delimiter(&path).skip_empty(skip_empty).build().unwrap());

    assert_eq!(read.len(), expected);
}

#[rstest]
#[case::unaligned(false, "eg-0")]
#[case::aligned(true, "seg-1")]
fn align_drops_a_partial_segment(sandbox: Sandbox, #[case] align: bool, #[case] expected: &str) {
    let path = sandbox.file(TEST_SEGMENT_CONTENT);
    let read = items(
        sandbox
            .reader()
            .delimiter(&path)
            .align(align)
            .start(1)
            .limit(1)
            .build()
            .unwrap(),
    );

    assert_eq!(texts(&read), [expected]);
}

#[rstest]
fn the_last_character_wins(sandbox: Sandbox) {
    let path = sandbox.file(TEST_SEGMENT_CONTENT);
    let read = items(
        sandbox
            .reader()
            .delimiter(&path)
            .character(b'x')
            .character(TEST_DELIMITER)
            .limit(1)
            .build()
            .unwrap(),
    );

    assert_eq!(texts(&read), ["seg-0"]);
}
