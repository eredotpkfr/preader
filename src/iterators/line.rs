use std::{ops::Range, str};

use crate::{
    LineIterator, Result,
    enums::skip::SkipPlan,
    interfaces::{iterator::IteratorRead, segmented::Segmented, skip::Skip},
};

// Byte a line iterator always splits on
const LINE_BOUNDARY: u8 = b'\n';
// Byte a line iterator strips alongside the boundary
const CARRIAGE_RETURN: u8 = b'\r';

#[derive(Debug, Default)]
pub struct Line {
    pub(crate) buffer: Vec<u8>,
    pub(crate) keepends: bool,
    pub(crate) align: bool,
    pub(crate) skip_empty: bool,
}

impl Segmented for Line {
    fn boundary(&self) -> u8 {
        LINE_BOUNDARY
    }

    fn buffer(&mut self) -> &mut Vec<u8> {
        &mut self.buffer
    }

    fn body(&self) -> Option<Range<usize>> {
        let full = self.buffer.len();
        let ending = self
            .buffer
            .iter()
            .rev()
            .take_while(|byte| matches!(**byte, LINE_BOUNDARY | CARRIAGE_RETURN))
            .count();
        let trimmed = full - ending;

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
        let Some(body) = self.segment()? else {
            return Ok(None);
        };

        Ok(Some(str::from_utf8(&self.inner.buffer[body])?))
    }
}

impl Skip for Line {
    fn skip(&self, count: u64) -> SkipPlan {
        SkipPlan::Items {
            count,
            boundary: self.align.then_some(self.boundary()),
        }
    }
}
