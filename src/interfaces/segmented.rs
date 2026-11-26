use std::ops::Range;

/// Strategy that splits a file into items ending at a boundary byte
///
/// Implemented by [`Line`](crate::Line) and [`Segment`](crate::Segment) so that
/// [`PReaderIterator::segment`](crate::PReaderIterator::segment) can read both
pub trait Segmented {
    /// Returns the byte that ends an item
    fn boundary(&self) -> u8;
    /// Returns the buffer the next item is read into
    fn buffer(&mut self) -> &mut Vec<u8>;
    /// Returns the range of the buffered item to yield, or `None` to skip the item
    fn body(&self) -> Option<Range<usize>>;
}
