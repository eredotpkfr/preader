use crate::{IteratorOptions, PReaderIteratorBuilder, StateSource};

impl<I> PReaderIteratorBuilder<'_, I> {
    /// Sets the state the iterator resumes from and saves to
    ///
    /// Accepts a name, a [`State`](crate::State), an [`Option`] of either, or a [`StateSource`],
    /// which describes the state each one resumes
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
    ///
    /// let dir = tempfile::tempdir()?;
    /// let file = dir.path().join("data.txt");
    ///
    /// let reader = PReader::from(Config {
    ///     state_dir: dir.path().join("states"),
    ///     ..Config::default()
    /// });
    ///
    /// std::fs::write(&file, "foo\nbar\n")?;
    ///
    /// let mut lines = reader.lines(&file).state("job-1").build()?;
    ///
    /// lines.next().transpose()?;
    /// lines.next().transpose()?;
    /// lines.save()?;
    ///
    /// let resumed = reader.lines(&file).state("job-1").build()?;
    /// let read = resumed.collect::<preader::Result<Vec<_>>>()?;
    ///
    /// assert_eq!(read, ["bar"]);
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn state(mut self, state: impl Into<StateSource>) -> Self {
        self.state = state.into();
        self
    }

    /// Replaces [`start`], [`end`], [`skip`] and [`limit`] with the values in `options`
    ///
    /// An earlier call to one of them is overwritten, and a later one overrides its value
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, IteratorOptions, PReader};
    ///
    /// let dir = tempfile::tempdir()?;
    /// let file = dir.path().join("data.txt");
    ///
    /// let reader = PReader::from(Config {
    ///     state_dir: dir.path().join("states"),
    ///     ..Config::default()
    /// });
    ///
    /// std::fs::write(&file, "foo\nbar\nbaz\n")?;
    ///
    /// let options = IteratorOptions {
    ///     start: 4,
    ///     limit: 1,
    ///     ..IteratorOptions::default()
    /// };
    /// let lines = reader.lines(&file).options(options).build()?;
    /// let read = lines.collect::<preader::Result<Vec<_>>>()?;
    ///
    /// assert_eq!(read, ["bar"]);
    /// # Ok::<(), preader::Error>(())
    /// ```
    ///
    /// [`start`]: Self::start
    /// [`end`]: Self::end
    /// [`skip`]: Self::skip
    /// [`limit`]: Self::limit
    pub fn options(mut self, options: IteratorOptions) -> Self {
        self.options = options;
        self
    }

    /// Sets the byte offset where reading begins
    ///
    /// A `start` beyond the position of a resumed state moves the state forward. A `start` past
    /// the recorded file size is clamped to it, so nothing is read. For lines and segments it may
    /// land inside an item, which is yielded from there unless [`align`] is set
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
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
    /// let bytes = reader.bytes(&file).start(3).build()?;
    /// let read = bytes.collect::<preader::Result<Vec<_>>>()?;
    ///
    /// assert_eq!(read, b"bar");
    /// # Ok::<(), preader::Error>(())
    /// ```
    ///
    /// [`align`]: crate::LineBuilder::align
    pub fn start(mut self, start: u64) -> Self {
        self.options.start = start;
        self
    }

    /// Sets the byte offset at which no new item starts
    ///
    /// It is clamped to the recorded file size. A byte iterator stops at `end` and a chunk
    /// iterator cuts its last chunk there, while a line or segment that starts before `end` is
    /// yielded whole
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
    ///
    /// let dir = tempfile::tempdir()?;
    /// let file = dir.path().join("data.txt");
    ///
    /// let reader = PReader::from(Config {
    ///     state_dir: dir.path().join("states"),
    ///     ..Config::default()
    /// });
    ///
    /// std::fs::write(&file, "foo\nbar\n")?;
    ///
    /// let lines = reader.lines(&file).end(2).build()?;
    /// let read = lines.collect::<preader::Result<Vec<_>>>()?;
    ///
    /// assert_eq!(read, ["foo"]);
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn end(mut self, end: u64) -> Self {
        self.options.end = end;
        self
    }

    /// Sets the number of items skipped after [`start`](Self::start)
    ///
    /// A byte iterator skips bytes and a chunk iterator skips `skip` times its size in bytes,
    /// both folded into [`start`](Self::start). Lines and segments are skipped only when the read
    /// begins at or before `start`, so a resumed state already past it ignores the count
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
    ///
    /// let dir = tempfile::tempdir()?;
    /// let file = dir.path().join("data.txt");
    ///
    /// let reader = PReader::from(Config {
    ///     state_dir: dir.path().join("states"),
    ///     ..Config::default()
    /// });
    ///
    /// std::fs::write(&file, "foo\nbar\nbaz\n")?;
    ///
    /// let lines = reader.lines(&file).skip(1).build()?;
    /// let read = lines.collect::<preader::Result<Vec<_>>>()?;
    ///
    /// assert_eq!(read, ["bar", "baz"]);
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn skip(mut self, skip: u64) -> Self {
        self.options.skip = skip;
        self
    }

    /// Sets the maximum number of items the iterator yields
    ///
    /// The limit applies to this iterator only, not to the total across resumes. Skipped items
    /// do not count toward it, and `0` yields nothing
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
    ///
    /// let dir = tempfile::tempdir()?;
    /// let file = dir.path().join("data.txt");
    ///
    /// let reader = PReader::from(Config {
    ///     state_dir: dir.path().join("states"),
    ///     ..Config::default()
    /// });
    ///
    /// std::fs::write(&file, "foo\nbar\nbaz\n")?;
    ///
    /// let lines = reader.lines(&file).limit(2).build()?;
    /// let read = lines.collect::<preader::Result<Vec<_>>>()?;
    ///
    /// assert_eq!(read, ["foo", "bar"]);
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn limit(mut self, limit: u64) -> Self {
        self.options.limit = limit;
        self
    }
}
