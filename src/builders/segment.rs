use crate::SegmentBuilder;

impl SegmentBuilder<'_> {
    /// Sets the byte that segments are split on
    ///
    /// Defaults to [`DEFAULT_DELIMITER`](crate::DEFAULT_DELIMITER)
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
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
    /// std::fs::write(&file, "foo;bar")?;
    ///
    /// let segments = reader.segments(&file).delimiter(b';').build()?;
    /// let read = segments.collect::<preader::Result<Vec<_>>>()?;
    ///
    /// assert_eq!(read, [b"foo", b"bar"]);
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn delimiter(mut self, delimiter: u8) -> Self {
        self.inner.delimiter = delimiter;
        self
    }

    /// Sets whether each segment keeps its trailing delimiter
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
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
    /// std::fs::write(&file, "foo,bar,")?;
    ///
    /// let segments = reader.segments(&file).keep(true).build()?;
    /// let read = segments.collect::<preader::Result<Vec<_>>>()?;
    ///
    /// assert_eq!(read, [b"foo,", b"bar,"]);
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn keep(mut self, keep: bool) -> Self {
        self.inner.keep = keep;
        self
    }

    /// Sets whether segments that are empty without their delimiter are skipped
    ///
    /// An empty segment still counts toward [`skip`](crate::PReaderIteratorBuilder::skip) but not
    /// toward [`limit`](crate::PReaderIteratorBuilder::limit)
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
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
    /// std::fs::write(&file, "foo,,bar")?;
    ///
    /// let segments = reader.segments(&file).skip_empty(true).build()?;
    /// let read = segments.collect::<preader::Result<Vec<_>>>()?;
    ///
    /// assert_eq!(read, [b"foo", b"bar"]);
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn skip_empty(mut self, skip_empty: bool) -> Self {
        self.inner.skip_empty = skip_empty;
        self
    }

    /// Sets whether a [`start`] inside a segment skips to the next segment
    ///
    /// A resumed state already past [`start`] is not moved
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
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
    /// let segments = reader.segments(&file).start(1).align(true).build()?;
    /// let read = segments.collect::<preader::Result<Vec<_>>>()?;
    ///
    /// assert_eq!(read, [b"bar"]);
    /// # Ok::<(), preader::Error>(())
    /// ```
    ///
    /// [`start`]: crate::PReaderIteratorBuilder::start
    pub fn align(mut self, align: bool) -> Self {
        self.inner.align = align;
        self
    }
}
