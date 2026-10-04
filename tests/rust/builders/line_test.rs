use preader::IteratorBuild;
use rstest::rstest;

use crate::common::{
    constants::{TEST_BLANK_LINE_CONTENT, TEST_LINE_CONTENT, TEST_LINES},
    fixtures::sandbox,
    funcs::{items, texts},
    sandbox::Sandbox,
};

#[rstest]
fn defaults_strip_the_terminator(sandbox: Sandbox) {
    let path = sandbox.file(TEST_LINE_CONTENT);
    let read = items(sandbox.reader().lines(&path).build().unwrap());

    assert_eq!(texts(&read), TEST_LINES);
}

#[rstest]
#[case::stripped(false, "line-0")]
#[case::kept(true, "line-0\n")]
fn keepends_decides_about_the_terminator(
    sandbox: Sandbox,
    #[case] keepends: bool,
    #[case] expected: &str,
) {
    let path = sandbox.file(TEST_LINE_CONTENT);
    let read = items(sandbox.reader().lines(&path).keepends(keepends).limit(1).build().unwrap());

    assert_eq!(texts(&read), [expected]);
}

#[rstest]
#[case::kept(false, 3)]
#[case::skipped(true, 2)]
fn skip_empty_decides_about_blank_lines(
    sandbox: Sandbox,
    #[case] skip_empty: bool,
    #[case] expected: usize,
) {
    let path = sandbox.file(TEST_BLANK_LINE_CONTENT);
    let read = items(sandbox.reader().lines(&path).skip_empty(skip_empty).build().unwrap());

    assert_eq!(read.len(), expected);
}

#[rstest]
#[case::unaligned(false, "ne-0")]
#[case::aligned(true, "line-1")]
fn align_drops_a_partial_line(sandbox: Sandbox, #[case] align: bool, #[case] expected: &str) {
    let path = sandbox.file(TEST_LINE_CONTENT);
    let read = items(sandbox.reader().lines(&path).align(align).start(2).limit(1).build().unwrap());

    assert_eq!(texts(&read), [expected]);
}

#[rstest]
fn last_flag_wins(sandbox: Sandbox) {
    let path = sandbox.file(TEST_LINE_CONTENT);
    let read = items(
        sandbox
            .reader()
            .lines(&path)
            .keepends(true)
            .keepends(false)
            .limit(1)
            .build()
            .unwrap(),
    );

    assert_eq!(texts(&read), ["line-0"]);
}
