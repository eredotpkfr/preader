use std::{io::BufRead, ops::Range};

use crate::{
    DelimiterIterator, Result, Skip,
    constants::DEFAULT_DELIMITER,
    interfaces::{iterator::IteratorRead, segmented::Segmented, skippable::Skippable},
    types::core::Reader,
};

#[derive(Debug)]
pub struct Delimiter {
    pub(crate) buffer: Vec<u8>,
    pub(crate) character: u8,
    pub(crate) keep: bool,
    pub(crate) align: bool,
    pub(crate) skip_empty: bool,
}

impl Default for Delimiter {
    fn default() -> Self {
        Self {
            character: DEFAULT_DELIMITER,
            keep: false,
            skip_empty: false,
            align: false,
            buffer: Vec::new(),
        }
    }
}

impl Segmented for Delimiter {
    fn fill(&mut self, reader: &mut Reader) -> Result<usize> {
        self.buffer.clear();

        Ok(reader.read_until(self.character, &mut self.buffer)?)
    }

    fn body(&self) -> Option<Range<usize>> {
        let full = self.buffer.len();
        let trimmed = self.buffer.strip_suffix(&[self.character]).unwrap_or(&self.buffer).len();

        if self.skip_empty && trimmed == 0 {
            return None;
        }

        Some(0..if self.keep { full } else { trimmed })
    }
}

impl IteratorRead for DelimiterIterator {
    type Borrowed<'a> = &'a [u8];
    type Owned = Vec<u8>;

    fn read(&mut self) -> Result<Option<&[u8]>> {
        Ok(self.segment()?.map(|body| &self.inner.buffer[body]))
    }
}

impl Skippable for Delimiter {
    fn skip(&self, count: u64) -> Skip {
        Skip::Items {
            count,
            boundary: self.align.then_some(self.character),
        }
    }
}
