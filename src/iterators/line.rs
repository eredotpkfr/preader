use std::{io::BufRead, ops::Range};

use crate::{
    LineIterator, Result, Skip,
    interfaces::{iterator::IteratorRead, segmented::Segmented, skippable::Skippable},
    types::core::Reader,
};

// Byte a line iterator always splits on
const LINE_BOUNDARY: u8 = b'\n';

#[derive(Debug, Default)]
pub struct Line {
    pub(crate) buffer: String,
    pub(crate) keepends: bool,
    pub(crate) align: bool,
    pub(crate) skip_empty: bool,
}

impl Segmented for Line {
    fn fill(&mut self, reader: &mut Reader) -> Result<usize> {
        self.buffer.clear();

        Ok(reader.read_line(&mut self.buffer)?)
    }

    fn body(&self) -> Option<Range<usize>> {
        let full = self.buffer.len();
        let trimmed = self.buffer.trim_end_matches('\n').trim_end_matches('\r').len();

        if self.skip_empty && trimmed == 0 {
            return None;
        }

        Some(0..if self.keepends { full } else { trimmed })
    }
}

impl IteratorRead for LineIterator {
    type Borrowed<'a> = &'a str;
    type Owned = String;

    fn read(&mut self) -> Result<Option<&str>> {
        Ok(self.segment()?.map(|body| &self.inner.buffer[body]))
    }
}

impl Skippable for Line {
    fn skip(&self, count: u64) -> Skip {
        Skip::Items {
            count,
            boundary: self.align.then_some(LINE_BOUNDARY),
        }
    }
}
