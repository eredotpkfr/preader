use rstest::rstest;

use crate::common::{
    constants::{TEST_BLANK_SEGMENT_CONTENT, TEST_DEFAULT_DELIMITER, TEST_SEGMENT_CONTENT},
    fixtures::sandbox,
    funcs::{items, texts},
    sandbox::Sandbox,
};

#[rstest]
#[case::kept(false, &["seg-0", "", "seg-2"])]
#[case::skipped(true, &["seg-0", "seg-2"])]
fn skip_empty_decides_about_blank_segments(
    sandbox: Sandbox,
    #[case] skip_empty: bool,
    #[case] expected: &[&str],
) {
    let path = sandbox.file(TEST_BLANK_SEGMENT_CONTENT);
    let read = items(sandbox.reader().segments(&path).skip_empty(skip_empty).build().unwrap());

    assert_eq!(texts(&read), expected);
}

#[rstest]
#[case::stripped(false, "seg-0")]
#[case::kept(true, "seg-0,")]
fn keep_decides_about_the_delimiter(sandbox: Sandbox, #[case] keep: bool, #[case] expected: &str) {
    let path = sandbox.file(TEST_SEGMENT_CONTENT);
    let read = items(sandbox.reader().segments(&path).keep(keep).limit(1).build().unwrap());

    assert_eq!(texts(&read), [expected]);
}

#[rstest]
#[case::unaligned(false, "eg-0")]
#[case::aligned(true, "seg-1")]
fn align_drops_a_partial_segment(sandbox: Sandbox, #[case] align: bool, #[case] expected: &str) {
    let path = sandbox.file(TEST_SEGMENT_CONTENT);
    let read =
        items(sandbox.reader().segments(&path).align(align).start(1).limit(1).build().unwrap());

    assert_eq!(texts(&read), [expected]);
}

#[rstest]
fn last_delimiter_wins(sandbox: Sandbox) {
    let path = sandbox.file(b"seg-0,seg-1|seg-2");
    let read = items(
        sandbox
            .reader()
            .segments(&path)
            .delimiter(TEST_DEFAULT_DELIMITER)
            .delimiter(b'|')
            .limit(1)
            .build()
            .unwrap(),
    );

    assert_eq!(texts(&read), ["seg-0,seg-1"]);
}
