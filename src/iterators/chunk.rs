use std::io::{ErrorKind, Read};

use crate::{ChunkIterator, Result, interfaces::iterator::IteratorRead, types::core::Reader};

pub const DEFAULT_CHUNK_SIZE: usize = 1024;

#[derive(Debug)]
pub struct Chunk {
    pub(crate) buffer: Vec<u8>,
    pub(crate) size: usize,
    pub(crate) drop_partial: bool,
}

impl Default for Chunk {
    fn default() -> Self {
        Self {
            size: DEFAULT_CHUNK_SIZE,
            drop_partial: false,
            buffer: vec![0; DEFAULT_CHUNK_SIZE],
        }
    }
}

impl Chunk {
    fn fill(&mut self, reader: &mut Reader, max: usize) -> Result<usize> {
        let mut filled = 0;

        while filled < max {
            match reader.read(&mut self.buffer[filled..max]) {
                Ok(0) => break,
                Ok(count) => filled += count,
                Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                Err(error) => return Err(error.into()),
            }
        }

        Ok(filled)
    }
}

impl IteratorRead for ChunkIterator {
    type Borrowed<'a> = &'a [u8];
    type Owned = Vec<u8>;

    fn read(&mut self) -> Result<Option<&[u8]>> {
        if self.done() {
            return self.stop();
        }

        let size = self.fields.size;
        let max = self.progress.remaining(self.state.position).min(size as u64) as usize;

        if self.fields.drop_partial && max < size {
            return self.stop();
        }

        let filled = self.fields.fill(&mut self.reader, max)?;

        if filled == 0 || (self.fields.drop_partial && filled < size) {
            self.advance(filled)?;

            return self.stop();
        }

        self.progress.count();
        self.advance(filled)?;

        Ok(Some(&self.fields.buffer[..filled]))
    }
}
