use std::io::Read;

use crate::{
    ByteIterator, Result,
    enums::skip::SkipPlan,
    interfaces::{iterator::IteratorRead, skip::Skip},
};

/// Strategy behind [`ByteIterator`], which yields each byte of the file as a `u8`
///
/// It has no settings. The iterator is built with [`PReader::bytes`](crate::PReader::bytes)
#[derive(Debug, Default)]
pub struct Byte;

/// Reads single bytes; the borrowed and owned items are both `u8`
impl IteratorRead for ByteIterator {
    /// The byte itself, since a `u8` needs no buffer
    type Borrowed<'a> = u8;
    /// The byte itself
    type Owned = u8;

    /// Reads the next byte, or returns `None` when the read is done
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
    /// std::fs::write(&file, b"foo")?;
    ///
    /// let mut bytes = reader.bytes(&file).build()?;
    /// let first = bytes.read()?;
    ///
    /// assert_eq!(first, Some(b'f'));
    /// # Ok::<(), preader::Error>(())
    /// ```
    fn read(&mut self) -> Result<Option<u8>> {
        if self.done() {
            return self.finalize();
        }

        let (read, byte) = match (&mut self.reader).bytes().next().transpose() {
            Ok(byte) => (Ok(usize::from(byte.is_some())), byte),
            Err(error) => (Err(error.into()), None),
        };

        if self.advance(read)? == 0 {
            return self.finalize();
        }

        self.progress.tick();

        Ok(byte)
    }
}

/// Skips `count` bytes
impl Skip for Byte {
    /// Returns [`SkipPlan::Bytes`] of `count` bytes
    fn skip(&self, count: u64) -> SkipPlan {
        SkipPlan::Bytes(count)
    }
}
