use std::ops::Range;

pub trait Segmented {
    fn boundary(&self) -> u8;
    fn buffer(&mut self) -> &mut Vec<u8>;
    fn body(&self) -> Option<Range<usize>>;
}
