use std::{io::BufRead, ops::Range};

use crate::{
    LineIterator, Result,
    interfaces::{iterator::IteratorRead, segmented::Segmented},
    types::core::Reader,
};

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
        let trimmed = self.buffer.trim_end_matches('\n').trim_end_matches('\r').len();

        if self.skip_empty && trimmed == 0 {
            return None;
        }

        Some(
            0..if self.keepends {
                self.buffer.len()
            } else {
                trimmed
            },
        )
    }
}

impl IteratorRead for LineIterator {
    type Borrowed<'a> = &'a str;
    type Owned = String;

    fn read(&mut self) -> Result<Option<&str>> {
        let Some(body) = self.segment()? else {
            return Ok(None);
        };

        Ok(Some(&self.fields.buffer[body]))
    }
}
