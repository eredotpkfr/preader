use preader::indent_lines;
use rstest::rstest;

#[rstest]
#[case::without_newline("foo", 2, "foo")]
#[case::multiple_lines("foo\nbar\nbaz", 2, "foo\n  bar\n  baz")]
#[case::zero_width("foo\nbar", 0, "foo\nbar")]
#[case::after_a_trailing_newline("foo\n", 3, "foo\n   ")]
#[case::consecutive_newlines("foo\n\nbar", 1, "foo\n \n bar")]
#[case::empty_text("", 4, "")]
fn indent_lines_pads_after_every_newline(
    #[case] text: &str,
    #[case] width: usize,
    #[case] expected: &str,
) {
    assert_eq!(indent_lines(text, width), expected);
}
