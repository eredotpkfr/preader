use std::ops::Range;

use crate::{
    Result, SegmentIterator,
    constants::DEFAULT_DELIMITER,
    enums::skip::SkipPlan,
    interfaces::{iterator::IteratorRead, segmented::Segmented, skip::Skip},
};

/// Strategy behind [`SegmentIterator`], which yields the parts between delimiters as `Vec<u8>`
///
/// The settings are made through [`SegmentBuilder`](crate::SegmentBuilder), returned by
/// [`PReader::segments`](crate::PReader::segments)
#[derive(Debug)]
pub struct Segment {
    /// Buffer the current segment is read into
    pub(crate) buffer: Vec<u8>,
    /// Byte that ends a segment
    pub(crate) delimiter: u8,
    /// Whether the trailing delimiter is kept
    pub(crate) keep: bool,
    /// Whether a partial segment at a mid-segment [`start`] is skipped
    ///
    /// [`start`]: crate::PReaderIteratorBuilder::start
    pub(crate) align: bool,
    /// Whether segments that are empty without their delimiter are skipped, even under [`keep`]
    ///
    /// [`keep`]: Self::keep
    pub(crate) skip_empty: bool,
}

impl Default for Segment {
    /// Creates a strategy that splits on [`DEFAULT_DELIMITER`] and strips it
    fn default() -> Self {
        Self {
            delimiter: DEFAULT_DELIMITER,
            keep: false,
            skip_empty: false,
            align: false,
            buffer: Vec::new(),
        }
    }
}

/// Splits on the delimiter and strips one trailing delimiter unless [`keep`](Self::keep) is set
impl Segmented for Segment {
    /// Returns the delimiter
    fn boundary(&self) -> u8 {
        self.delimiter
    }

    /// Returns the segment buffer
    fn buffer(&mut self) -> &mut Vec<u8> {
        &mut self.buffer
    }

    /// Returns the segment without its trailing delimiter, or with it under [`keep`](Self::keep)
    ///
    /// Returns `None` for a segment that is empty after stripping when [`skip_empty`] is set
    ///
    /// [`skip_empty`]: Self::skip_empty
    fn body(&self) -> Option<Range<usize>> {
        let full = self.buffer.len();
        let trimmed = self.buffer.strip_suffix(&[self.boundary()]).unwrap_or(&self.buffer).len();

        if self.skip_empty && trimmed == 0 {
            return None;
        }

        Some(0..if self.keep { full } else { trimmed })
    }
}

/// Reads segments borrowed from the iterator's buffer and yields them as `Vec<u8>`
impl IteratorRead for SegmentIterator {
    /// Segment borrowed from the iterator's buffer
    type Borrowed<'a> = &'a [u8];
    /// Segment copied out of the buffer
    type Owned = Vec<u8>;

    /// Reads the next segment, or returns `None` when the read is done
    ///
    /// A last segment without the delimiter is not yielded when the file has grown past its
    /// recorded size, since its end is not known yet
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
    /// let file = dir.path().join("data.csv");
    ///
    /// let config = Config {
    ///     state_dir: dir.path().join("states"),
    ///     ..Config::default()
    /// };
    /// let reader = PReader::from(config);
    ///
    /// std::fs::write(&file, "foo,bar")?;
    ///
    /// let mut segments = reader.segments(&file).build()?;
    /// let first = segments.read()?;
    ///
    /// assert_eq!(first, Some(&b"foo"[..]));
    /// # Ok::<(), preader::Error>(())
    /// ```
    fn read(&mut self) -> Result<Option<&[u8]>> {
        Ok(self.segment()?.map(|body| &self.inner.buffer[body]))
    }
}

/// Skips `count` segments
impl Skip for Segment {
    /// Returns [`SkipPlan::Items`] of `count` segments, aligned on the delimiter under [`align`]
    ///
    /// [`align`]: Self::align
    fn skip(&self, count: u64) -> SkipPlan {
        SkipPlan::Items {
            count,
            boundary: self.align.then_some(self.boundary()),
        }
    }
}
