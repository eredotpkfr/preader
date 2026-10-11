pub const EMPTY: &[u8] = b"";
pub const LINES: &[u8] = b"line-0\n\nline-1\nline-2\n";
pub const UNTERMINATED_LINES: &[u8] = b"line-0\n\nline-1\nline-2";
pub const CRLF_LINES: &[u8] = b"line-0\r\nline-1\r\n";
pub const BLANK_LINES: &[u8] = b"\n\n\n";
pub const SEGMENTS: &[u8] = b"seg-0,seg-1,,seg-2,";
pub const UNTERMINATED_SEGMENTS: &[u8] = b"seg-0,seg-1,,seg-2";
pub const BLANK_SEGMENTS: &[u8] = b",,,";
pub const WHOLE_SEGMENT: &[u8] = b"seg-0";
pub const UTF8_SEGMENTS: &[u8] = "café,日本語,,🦀,鉄".as_bytes();
pub const ALL_BYTES: &[u8] = &{
    let mut bytes = [0; 256];
    let mut index = 0;

    while index < bytes.len() {
        bytes[index] = index as u8;
        index += 1;
    }

    bytes
};
