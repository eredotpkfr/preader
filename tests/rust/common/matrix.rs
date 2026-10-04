use preader::IteratorOptions;

const STARTS: [u64; 4] = [0, 1, 3, 7];
const ENDS: [u64; 3] = [2, 5, u64::MAX];
const SKIPS: [u64; 3] = [0, 1, 3];
const LIMITS: [u64; 3] = [1, 2, u64::MAX];
const SIZES: [usize; 4] = [0, 1, 3, 64];
const FLAGS: [bool; 2] = [false, true];

pub const EMPTY: &[u8] = b"";
pub const LINES: &[u8] = b"line-0\n\nline-1\nline-2\n";
pub const UNTERMINATED_LINES: &[u8] = b"line-0\n\nline-1\nline-2";
pub const CRLF_LINES: &[u8] = b"line-0\r\nline-1\r\n";
pub const BLANK_LINES: &[u8] = b"\n\n\n";

pub const SEGMENTS: &[u8] = b"seg-0,seg-1,,seg-2,";
pub const UNTERMINATED_SEGMENTS: &[u8] = b"seg-0,seg-1,,seg-2";
pub const BLANK_SEGMENTS: &[u8] = b",,,";
pub const WHOLE_SEGMENT: &[u8] = b"no-delimiter-here";

#[derive(Clone, Copy, Debug)]
pub struct Shape {
    pub keep: bool,
    pub skip_empty: bool,
    pub align: bool,
}

pub fn windows() -> impl Iterator<Item = IteratorOptions> {
    STARTS.into_iter().flat_map(|start| {
        ENDS.into_iter().flat_map(move |end| {
            SKIPS.into_iter().flat_map(move |skip| {
                LIMITS.into_iter().map(move |limit| IteratorOptions {
                    start,
                    end,
                    skip,
                    limit,
                })
            })
        })
    })
}

pub fn shapes() -> impl Iterator<Item = Shape> {
    FLAGS.into_iter().flat_map(|keep| {
        FLAGS.into_iter().flat_map(move |skip_empty| {
            FLAGS.into_iter().map(move |align| Shape {
                keep,
                skip_empty,
                align,
            })
        })
    })
}

pub fn sizings() -> impl Iterator<Item = (usize, bool)> {
    SIZES
        .into_iter()
        .flat_map(|size| FLAGS.into_iter().map(move |drop_partial| (size, drop_partial)))
}
