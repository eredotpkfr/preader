use std::io::Read;

use crate::{
    ChunkIterator, Result,
    constants::DEFAULT_CHUNK_SIZE,
    enums::skip::SkipPlan,
    interfaces::{iterator::IteratorRead, skip::Skip},
    types::core::FileReader,
};

/// Strategy behind [`ChunkIterator`], which yields the file in fixed-size `Vec<u8>` chunks
///
/// The settings are made through [`ChunkBuilder`](crate::ChunkBuilder), returned by
/// [`PReader::chunks`](crate::PReader::chunks)
#[derive(Debug)]
pub struct Chunk {
    /// Buffer the current chunk is read into
    pub(crate) buffer: Vec<u8>,
    /// Size of a chunk in bytes; `0` yields no chunks
    pub(crate) size: usize,
    /// Whether a last chunk shorter than [`size`](Self::size) is dropped instead of yielded
    pub(crate) drop_partial: bool,
}

impl Default for Chunk {
    /// Creates a strategy with [`DEFAULT_CHUNK_SIZE`]-byte chunks that keeps a shorter last chunk
    fn default() -> Self {
        Self {
            size: DEFAULT_CHUNK_SIZE,
            drop_partial: false,
            buffer: Vec::new(),
        }
    }
}

impl Chunk {
    /// Reads up to `max` bytes into the buffer and returns how many were read
    ///
    /// It stops early only at the end of the file and retries reads that were interrupted
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`](crate::Error::Io) when the file cannot be read
    fn fill(&mut self, reader: &mut FileReader, max: usize) -> Result<usize> {
        self.buffer.clear();

        Ok(reader.take(max as u64).read_to_end(&mut self.buffer)?)
    }
}

/// Reads chunks borrowed from the iterator's buffer and yields them as `Vec<u8>`
impl IteratorRead for ChunkIterator {
    /// Chunk borrowed from the iterator's buffer
    type Borrowed<'a> = &'a [u8];
    /// Chunk copied out of the buffer
    type Owned = Vec<u8>;

    /// Reads the next chunk, or returns `None` when the read is done
    ///
    /// The last chunk is shorter when the file or [`end`](crate::PReaderIteratorBuilder::end)
    /// cuts it, unless it is dropped with [`drop_partial`](crate::ChunkBuilder::drop_partial).
    /// The file ends at its recorded size, so a file that grew is still cut there
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`](crate::Error::Io) when the file cannot be read, and the errors of
    /// [`PReaderIterator::save`](crate::PReaderIterator::save) when an autosave fails
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, IteratorRead, PReader};
    ///
    /// let dir = tempfile::tempdir()?;
    /// let file = dir.path().join("data.bin");
    ///
    /// let reader = PReader::from(Config {
    ///     state_dir: dir.path().join("states"),
    ///     ..Config::default()
    /// });
    ///
    /// std::fs::write(&file, b"foobar")?;
    ///
    /// let mut chunks = reader.chunks(&file).size(3).build()?;
    /// let first = chunks.read()?;
    ///
    /// assert_eq!(first, Some(&b"foo"[..]));
    /// # Ok::<(), preader::Error>(())
    /// ```
    fn read(&mut self) -> Result<Option<&[u8]>> {
        if self.done() {
            return self.finalize();
        }

        let size = self.inner.size;
        let max = self.progress.remaining(self.state.position).min(size as u64) as usize;

        if self.inner.drop_partial && max < size {
            return self.finalize();
        }

        let read = self.inner.fill(&mut self.reader, max);
        let filled = self.advance(read)?;

        if filled == 0 || (self.inner.drop_partial && filled < size) {
            return self.finalize();
        }

        self.progress.tick();

        Ok(Some(&self.inner.buffer[..filled]))
    }
}

/// Skips `count` whole chunks
impl Skip for Chunk {
    /// Returns [`SkipPlan::Bytes`] of `count` times the chunk size, saturating at `u64::MAX`
    fn skip(&self, count: u64) -> SkipPlan {
        SkipPlan::Bytes(count.saturating_mul(self.size as u64))
    }
}
