use crate::LineBuilder;

impl LineBuilder<'_> {
    /// Sets whether each line keeps its terminator
    ///
    /// The terminator is the whole trailing run of `\n` and `\r` bytes
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
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
    /// let lines = reader.lines(&file).keepends(true).build()?;
    /// let read = lines.collect::<preader::Result<Vec<_>>>()?;
    ///
    /// assert_eq!(read, ["foo\n", "bar\n"]);
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn keepends(mut self, keepends: bool) -> Self {
        self.inner.keepends = keepends;
        self
    }

    /// Sets whether lines that are empty without their terminator are skipped
    ///
    /// An empty line still counts toward [`skip`](crate::PReaderIteratorBuilder::skip) but not
    /// toward [`limit`](crate::PReaderIteratorBuilder::limit)
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
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
    /// std::fs::write(&file, "foo\n\nbar\n")?;
    ///
    /// let lines = reader.lines(&file).skip_empty(true).build()?;
    /// let read = lines.collect::<preader::Result<Vec<_>>>()?;
    ///
    /// assert_eq!(read, ["foo", "bar"]);
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn skip_empty(mut self, skip_empty: bool) -> Self {
        self.inner.skip_empty = skip_empty;
        self
    }

    /// Sets whether a [`start`] inside a line skips to the next line
    ///
    /// A resumed state already past [`start`] is not moved
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
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
    /// let lines = reader.lines(&file).start(1).align(true).build()?;
    /// let read = lines.collect::<preader::Result<Vec<_>>>()?;
    ///
    /// assert_eq!(read, ["bar"]);
    /// # Ok::<(), preader::Error>(())
    /// ```
    ///
    /// [`start`]: crate::PReaderIteratorBuilder::start
    pub fn align(mut self, align: bool) -> Self {
        self.inner.align = align;
        self
    }
}
