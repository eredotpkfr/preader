use std::time::Duration;

pub const TEST_STAMP: i64 = 1_700_000_000;
pub const TEST_TIMEOUT: Duration = Duration::from_secs(60);
pub const TEST_STATE_NAME: &str = "job-1";
pub const TEST_OTHER_STATE_NAME: &str = "job-2";
pub const TEST_SUB_STATE_NAME: &str = "sub";
pub const TEST_MISSING_STATE_NAME: &str = "job-missing";
pub const TEST_NESTED_STATE_NAME: &str = "sub-1/sub-2/job-1";
pub const TEST_DEEP_STATE_NAME: &str = "sub-1/sub-2/sub-3/sub-4/job-1";
pub const TEST_EVERY_DEPTH: [&str; 3] = [
    TEST_STATE_NAME,
    TEST_NESTED_STATE_NAME,
    TEST_DEEP_STATE_NAME,
];
pub const TEST_FILE_NAME: &str = "data.bin";
pub const TEST_TRACKED_NAME: &str = "tracked.bin";
pub const TEST_FILE_PATH: &str = "/tmp/data.bin";
pub const TEST_LINE: &[u8] = b"foo\n";
pub const TEST_LINE_FINGERPRINT: &str =
    "b5bb9d8014a0f9b1d61e21e796d78dccdf1352f23cd32812f4850b878ae4944c";
pub const TEST_EMPTY_FINGERPRINT: &str =
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
pub const TEST_LARGE_COPIES: usize = 2560;
pub const TEST_ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
pub const TEST_LINE_CONTENT: &[u8] = b"line-0\nline-1\nline-2\nline-3\nline-4\nline-5";
pub const TEST_BLANK_LINE_CONTENT: &[u8] = b"line-0\n\nline-2\n";
pub const TEST_SEGMENT_CONTENT: &[u8] =
    b"seg-0,seg-1,seg-2,seg-3,seg-4,seg-5,seg-6,seg-7,seg-8,seg-9";
pub const TEST_BLANK_SEGMENT_CONTENT: &[u8] = b"seg-0,,seg-2,";
pub const TEST_INVALID_UTF8: &[u8] = b"\xff";

#[cfg(unix)]
pub const TEST_NON_UTF8_NAME: &[u8] = b"data-\xff.bin";
pub const TEST_WINDOW: usize = preader::FINGERPRINT_SAMPLE_BYTES as usize;
#[cfg(unix)]
pub const TEST_READ_FROM: u64 = 10;
#[cfg(unix)]
pub const TEST_REWOUND_TO: u64 = 2;
pub const TEST_UNSEEKABLE_POSITION: u64 = i64::MAX as u64 + 1;
pub const TEST_UNICODE_TEXT: &str = "café Ünicode 日本語 🦀";
