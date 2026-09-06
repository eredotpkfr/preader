use std::{io::BufRead, ops::Range};

use crate::{
    DelimiterIterator, Result,
    interfaces::{iterator::IteratorRead, segmented::Segmented},
    types::core::Reader,
};

pub const DEFAULT_DELIMITER: u8 = b',';

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
        let trimmed = self.buffer.strip_suffix(&[self.character]).unwrap_or(&self.buffer).len();

        if self.skip_empty && trimmed == 0 {
            return None;
        }

        Some(
            0..if self.keep {
                self.buffer.len()
            } else {
                trimmed
            },
        )
    }
}

impl IteratorRead for DelimiterIterator {
    type Borrowed<'a> = &'a [u8];
    type Owned = Vec<u8>;

    fn read(&mut self) -> Result<Option<&[u8]>> {
        let Some(body) = self.segment()? else {
            return Ok(None);
        };

        Ok(Some(&self.fields.buffer[body]))
    }
}
