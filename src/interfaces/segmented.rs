use std::ops::Range;

use crate::{Result, types::core::FileReader};

pub trait Segmented {
    fn fill(&mut self, reader: &mut FileReader) -> Result<usize>;
    fn body(&self) -> Option<Range<usize>>;
}
