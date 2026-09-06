use std::ops::Range;

use crate::{Result, types::core::Reader};

pub trait Segmented {
    fn fill(&mut self, reader: &mut Reader) -> Result<usize>;
    fn body(&self) -> Option<Range<usize>>;
}
