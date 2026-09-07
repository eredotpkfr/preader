use std::io::{ErrorKind, Read};

use crate::{
    ChunkIterator, Result, Skip,
    constants::DEFAULT_CHUNK_SIZE,
    interfaces::{iterator::IteratorRead, skippable::Skippable},
    types::core::Reader,
};

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

        let size = self.inner.size;
        let max = self.progress.remaining(self.state.position).min(size as u64) as usize;

        if self.inner.drop_partial && max < size {
            return self.stop();
        }

        let filled = self.inner.fill(&mut self.reader, max)?;

        if filled == 0 || (self.inner.drop_partial && filled < size) {
            self.advance(filled)?;

            return self.stop();
        }

        self.progress.count();
        self.advance(filled)?;

        Ok(Some(&self.inner.buffer[..filled]))
    }
}

impl Skippable for Chunk {
    fn skip(&self, count: u64) -> Skip {
        Skip::Bytes(count.saturating_mul(self.size as u64))
    }
}
