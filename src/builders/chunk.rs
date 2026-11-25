use crate::ChunkBuilder;

impl ChunkBuilder<'_> {
    /// Sets the size of each chunk in bytes
    ///
    /// Defaults to [`DEFAULT_CHUNK_SIZE`](crate::DEFAULT_CHUNK_SIZE). The last chunk is shorter
    /// when the read range does not divide evenly, and a size of `0` yields nothing
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
    /// std::fs::write(&file, b"abcdef")?;
    ///
    /// let chunks = reader.chunks(&file).size(4).build()?;
    /// let read = chunks.collect::<preader::Result<Vec<_>>>()?;
    ///
    /// assert_eq!(read, [&b"abcd"[..], b"ef"]);
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn size(mut self, size: usize) -> Self {
        self.inner.size = size;
        self
    }

    /// Sets whether a last chunk shorter than the size is dropped
    ///
    /// This includes a chunk cut short by [`end`](crate::PReaderIteratorBuilder::end). The dropped
    /// bytes are not finished, so a saved state stays before them
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
    /// std::fs::write(&file, b"foobarb")?;
    ///
    /// let chunks = reader.chunks(&file).size(3).drop_partial(true).build()?;
    /// let read = chunks.collect::<preader::Result<Vec<_>>>()?;
    ///
    /// assert_eq!(read, [b"foo", b"bar"]);
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn drop_partial(mut self, drop_partial: bool) -> Self {
        self.inner.drop_partial = drop_partial;
        self
    }
}
