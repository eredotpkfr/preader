use rstest_reuse::template;

#[template]
#[rstest]
#[case::newline(b'\n')]
#[case::null(b'\0')]
#[case::letter(b'x')]
#[case::high_byte(0xE9)]
fn delimiter_characters(#[case] character: u8) {}
