use rstest_reuse::template;

#[template]
#[rstest]
#[case::comma(preader::DEFAULT_DELIMITER)]
#[case::newline(b'\n')]
#[case::null(b'\0')]
#[case::letter(b'x')]
#[case::high_byte(0xE9)]
#[case::last(0xFF)]
fn delimiter_characters(#[case] character: u8) {}
