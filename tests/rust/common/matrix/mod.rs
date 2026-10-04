pub mod asserts;
pub mod contents;
pub mod expected;
pub mod windows;

pub use asserts::assert_bounds;
pub use contents::{
    BLANK_LINES, BLANK_SEGMENTS, CRLF_LINES, EMPTY, LINES, SEGMENTS, UNTERMINATED_LINES,
    UNTERMINATED_SEGMENTS, WHOLE_SEGMENT,
};
pub use windows::{Shape, shapes, sizings, windows};
