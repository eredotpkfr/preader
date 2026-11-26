use rstest_reuse::template;

#[template]
#[rstest]
#[case::comma(crate::common::constants::TEST_DEFAULT_DELIMITER)]
#[case::newline(b'\n')]
#[case::null(b'\0')]
#[case::letter(b'x')]
#[case::last_ascii(0x7F)]
#[case::first_non_ascii(0x80)]
#[case::high_byte(0xE9)]
#[case::last(0xFF)]
fn delimiters(#[case] delimiter: u8) {}
