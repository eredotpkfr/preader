use std::path::PathBuf;

use crate::{
    ByteBuilder, ChunkBuilder, Config, LineBuilder, SegmentBuilder, StateRegistry,
    bases::builder::PReaderIteratorBuilder,
};

/// Entry point that reads files as resumable iterators
///
/// A reader owns a [`Config`] and the [`StateRegistry`] for its state directory, and reads a
/// file as bytes, chunks, lines or segments
#[cfg_attr(feature = "python", pyo3::pyclass(module = "preader"))]
#[derive(Debug)]
pub struct PReader {
    /// Configuration every iterator of this reader uses
    config: Config,
    /// Registry for the state directory of [`config`](PReader::config)
    registry: StateRegistry,
}

impl Default for PReader {
    /// Creates a reader with [`Config::default`]
    fn default() -> Self {
        Self::from(Config::default())
    }
}

impl From<Config> for PReader {
    /// Creates a reader that uses `config` and keeps its states in [`Config::state_dir`]
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
    ///
    /// let config = Config {
    ///     verify_state: false,
    ///     ..Config::default()
    /// };
    /// let reader = PReader::from(config);
    ///
    /// assert!(!reader.config().verify_state);
    /// ```
    fn from(config: Config) -> Self {
        Self {
            registry: (&config).into(),
            config,
        }
    }
}

impl PReader {
    /// Creates a reader with the default configuration
    ///
    /// Creating a reader never touches the file system
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
    ///
    /// let reader = PReader::new();
    ///
    /// assert_eq!(reader.config(), &Config::default());
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the configuration the reader was created with
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
    ///
    /// let config = Config {
    ///     buffer_capacity: 4096,
    ///     ..Config::default()
    /// };
    /// let reader = PReader::from(config);
    ///
    /// assert_eq!(reader.config().buffer_capacity, 4096);
    /// ```
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Returns the registry of the states saved in this reader's state directory
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
    ///
    /// let dir = tempfile::tempdir()?;
    ///
    /// let config = Config {
    ///     state_dir: dir.path().join("states"),
    ///     ..Config::default()
    /// };
    /// let reader = PReader::from(config);
    ///
    /// assert_eq!(reader.states().state_dir(), dir.path().join("states"));
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn states(&self) -> &StateRegistry {
        &self.registry
    }

    /// Returns a builder for an iterator that yields `file` one byte at a time
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
    ///
    /// let dir = tempfile::tempdir()?;
    /// let file = dir.path().join("data.bin");
    ///
    /// let config = Config {
    ///     state_dir: dir.path().join("states"),
    ///     ..Config::default()
    /// };
    /// let reader = PReader::from(config);
    ///
    /// std::fs::write(&file, b"foo")?;
    ///
    /// let bytes = reader.bytes(&file).build()?;
    /// let read = bytes.collect::<preader::Result<Vec<_>>>()?;
    ///
    /// assert_eq!(read, b"foo");
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn bytes(&self, file: impl Into<PathBuf>) -> ByteBuilder<'_> {
        self.builder(file)
    }

    /// Returns a builder for an iterator that yields `file` in fixed-size chunks
    ///
    /// The size defaults to [`DEFAULT_CHUNK_SIZE`](crate::DEFAULT_CHUNK_SIZE) and is set with
    /// [`ChunkBuilder::size`]
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
    ///
    /// let dir = tempfile::tempdir()?;
    /// let file = dir.path().join("data.bin");
    ///
    /// let config = Config {
    ///     state_dir: dir.path().join("states"),
    ///     ..Config::default()
    /// };
    /// let reader = PReader::from(config);
    ///
    /// std::fs::write(&file, b"foobar")?;
    ///
    /// let chunks = reader.chunks(&file).size(3).build()?;
    /// let read = chunks.collect::<preader::Result<Vec<_>>>()?;
    ///
    /// assert_eq!(read, [b"foo", b"bar"]);
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn chunks(&self, file: impl Into<PathBuf>) -> ChunkBuilder<'_> {
        self.builder(file)
    }

    /// Returns a builder for an iterator that yields the UTF-8 lines of `file`
    ///
    /// Lines end at `\n` and are yielded without their terminator unless [`LineBuilder::keepends`]
    /// is set. A line that is not valid UTF-8 is yielded as an [`Error::Utf8`](crate::Error::Utf8)
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
    /// let lines = reader.lines(&file).build()?;
    /// let read = lines.collect::<preader::Result<Vec<_>>>()?;
    ///
    /// assert_eq!(read, ["foo", "bar"]);
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn lines(&self, file: impl Into<PathBuf>) -> LineBuilder<'_> {
        self.builder(file)
    }

    /// Returns a builder for an iterator that splits `file` on a delimiter byte
    ///
    /// The delimiter defaults to [`DEFAULT_DELIMITER`](crate::DEFAULT_DELIMITER) and is set
    /// with [`SegmentBuilder::delimiter`]. Segments are yielded without their trailing delimiter
    /// unless [`SegmentBuilder::keep`] is set
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
    /// let segments = reader.segments(&file).build()?;
    /// let read = segments.collect::<preader::Result<Vec<_>>>()?;
    ///
    /// assert_eq!(read, [b"foo", b"bar"]);
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn segments(&self, file: impl Into<PathBuf>) -> SegmentBuilder<'_> {
        self.builder(file)
    }

    /// Creates a builder for `file` that shares this reader's configuration and registry
    fn builder<I: Default>(&self, file: impl Into<PathBuf>) -> PReaderIteratorBuilder<'_, I> {
        PReaderIteratorBuilder::new(&self.config, &self.registry, file)
    }
}
