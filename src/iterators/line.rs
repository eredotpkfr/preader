use std::{ops::Range, str};

use crate::{
    LineIterator, Result,
    enums::skip::SkipPlan,
    interfaces::{iterator::IteratorRead, segmented::Segmented, skip::Skip},
};

/// Byte a line iterator always splits on
const LINE_BOUNDARY: u8 = b'\n';
/// Byte a line iterator strips alongside the boundary
const CARRIAGE_RETURN: u8 = b'\r';

/// Strategy behind [`LineIterator`], which yields each line of the file as a `String`
///
/// Only `\n` ends a line, and the whole trailing run of `\n` and `\r` bytes is stripped unless
/// [`keepends`](crate::LineBuilder::keepends) is set. The settings are made through
/// [`LineBuilder`](crate::LineBuilder), returned by [`PReader::lines`](crate::PReader::lines)
#[derive(Debug, Default)]
pub struct Line {
    /// Buffer the current line is read into
    pub(crate) buffer: Vec<u8>,
    /// Whether the trailing `\n` and `\r` bytes are kept
    pub(crate) keepends: bool,
    /// Whether a partial line at a mid-line [`start`] is skipped
    ///
    /// [`start`]: crate::PReaderIteratorBuilder::start
    pub(crate) align: bool,
    /// Whether lines that are empty without their terminator are skipped, even under [`keepends`]
    ///
    /// [`keepends`]: Self::keepends
    pub(crate) skip_empty: bool,
}

/// Splits on `\n` and strips the trailing terminator unless [`keepends`](Self::keepends) is set
impl Segmented for Line {
    /// Returns `\n`, the only byte that ends a line
    fn boundary(&self) -> u8 {
        LINE_BOUNDARY
    }

    /// Returns the line buffer
    fn buffer(&mut self) -> &mut Vec<u8> {
        &mut self.buffer
    }

    /// Returns the line without its trailing `\n` and `\r` run, or with it under [`keepends`]
    ///
    /// Returns `None` for a line that is empty after stripping when [`skip_empty`] is set
    ///
    /// [`keepends`]: Self::keepends
    /// [`skip_empty`]: Self::skip_empty
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

/// Reads lines borrowed from the iterator's buffer and yields them as `String`
impl IteratorRead for LineIterator {
    /// Line borrowed from the iterator's buffer
    type Borrowed<'a> = &'a str;
    /// Line copied out of the buffer
    type Owned = String;

    /// Reads the next line, or returns `None` when the read is done
    ///
    /// A line that is not valid UTF-8 is an error for that line only, and the next read continues
    /// with the following line. The failed line counts toward
    /// [`limit`](crate::PReaderIteratorBuilder::limit). A last line without `\n` is not yielded
    /// when the file has grown past its recorded size, since its end is not known yet
    ///
    /// # Errors
    ///
    /// Returns [`Error::Utf8`](crate::Error::Utf8) when the line is not valid UTF-8,
    /// [`Error::Io`](crate::Error::Io) when the file cannot be read, and the errors of
    /// [`PReaderIterator::save`](crate::PReaderIterator::save) when an autosave fails
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, IteratorRead, PReader};
    ///
    /// let dir = tempfile::tempdir()?;
    /// let file = dir.path().join("data.txt");
    ///
    /// let config = Config {
    ///     state_dir: dir.path().join("states"),
    ///     ..Config::default()
    /// };
    /// let reader = PReader::from(config);
    ///
    /// std::fs::write(&file, "foo\nbar\n")?;
    ///
    /// let mut lines = reader.lines(&file).build()?;
    /// let first = lines.read()?;
    ///
    /// assert_eq!(first, Some("foo"));
    /// # Ok::<(), preader::Error>(())
    /// ```
    fn read(&mut self) -> Result<Option<&str>> {
        let Some(body) = self.segment()? else {
            return Ok(None);
        };

        Ok(Some(str::from_utf8(&self.inner.buffer[body])?))
    }
}

/// Skips `count` lines
impl Skip for Line {
    /// Returns [`SkipPlan::Items`] of `count` lines, aligned on `\n` when [`align`] is set
    ///
    /// [`align`]: Self::align
    fn skip(&self, count: u64) -> SkipPlan {
        SkipPlan::Items {
            count,
            boundary: self.align.then_some(self.boundary()),
        }
    }
}
