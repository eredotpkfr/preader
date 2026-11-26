use preader::hash;
use rstest::rstest;

use crate::common::constants::{TEST_EMPTY_FINGERPRINT, TEST_LINE, TEST_LINE_FINGERPRINT};

#[rstest]
#[case::empty(b"", TEST_EMPTY_FINGERPRINT)]
#[case::line(TEST_LINE, TEST_LINE_FINGERPRINT)]
fn sha256_returns_the_hex_digest(#[case] data: &[u8], #[case] expected: &str) {
    assert_eq!(hash::sha256(data), expected);
}

#[rstest]
fn sha256_accepts_every_byte_shape() {
    let text = String::from_utf8(TEST_LINE.to_vec()).unwrap();

    let digests = [
        hash::sha256(TEST_LINE),
        hash::sha256(text.as_str()),
        hash::sha256(text),
    ];

    assert_eq!(digests, [TEST_LINE_FINGERPRINT; 3]);
}
