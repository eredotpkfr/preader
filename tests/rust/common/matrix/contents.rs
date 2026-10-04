pub const EMPTY: &[u8] = b"";
pub const LINES: &[u8] = b"line-0\n\nline-1\nline-2\n";
pub const UNTERMINATED_LINES: &[u8] = b"line-0\n\nline-1\nline-2";
pub const CRLF_LINES: &[u8] = b"line-0\r\nline-1\r\n";
pub const BLANK_LINES: &[u8] = b"\n\n\n";

pub const SEGMENTS: &[u8] = b"seg-0,seg-1,,seg-2,";
pub const UNTERMINATED_SEGMENTS: &[u8] = b"seg-0,seg-1,,seg-2";
pub const BLANK_SEGMENTS: &[u8] = b",,,";
pub const WHOLE_SEGMENT: &[u8] = b"no-delimiter-here";
