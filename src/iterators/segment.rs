use std::ops::Range;

use crate::{
    Result, SegmentIterator,
    constants::DEFAULT_DELIMITER,
    enums::skip::SkipPlan,
    interfaces::{iterator::IteratorRead, segmented::Segmented, skip::Skip},
};

#[derive(Debug)]
pub struct Segment {
    pub(crate) buffer: Vec<u8>,
    pub(crate) delimiter: u8,
    pub(crate) keep: bool,
    pub(crate) align: bool,
    pub(crate) skip_empty: bool,
}

impl Default for Segment {
    fn default() -> Self {
        Self {
            delimiter: DEFAULT_DELIMITER,
            keep: false,
            skip_empty: false,
            align: false,
            buffer: Vec::new(),
        }
    }
}

impl Segmented for Segment {
    fn boundary(&self) -> u8 {
        self.delimiter
    }

    fn buffer(&mut self) -> &mut Vec<u8> {
        &mut self.buffer
    }

    fn body(&self) -> Option<Range<usize>> {
        let full = self.buffer.len();
        let trimmed = self.buffer.strip_suffix(&[self.boundary()]).unwrap_or(&self.buffer).len();

        if self.skip_empty && trimmed == 0 {
            return None;
        }

        Some(0..if self.keep { full } else { trimmed })
    }
}

impl IteratorRead for SegmentIterator {
    type Borrowed<'a> = &'a [u8];
    type Owned = Vec<u8>;

    fn read(&mut self) -> Result<Option<&[u8]>> {
        Ok(self.segment()?.map(|body| &self.inner.buffer[body]))
    }
}

impl Skip for Segment {
    fn skip(&self, count: u64) -> SkipPlan {
        SkipPlan::Items {
            count,
            boundary: self.align.then_some(self.boundary()),
        }
    }
}
