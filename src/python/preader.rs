use std::path::PathBuf;

use pyo3::prelude::*;

use crate::{
    Config, DEFAULT_CHUNK_SIZE, DEFAULT_DELIMITER, IteratorOptions, PReader, StateRegistry,
    StateSource,
    python::{
        bases::iterator::IteratorBase,
        iterators::{
            byte::ByteIterator, chunk::ChunkIterator, line::LineIterator, segment::SegmentIterator,
        },
        macros::repr::pyrepr,
    },
};

#[pymethods]
impl PReader {
    /// Creates a reader that uses `config`, or a default `Config()` when it is omitted
    #[new]
    #[pyo3(signature = (*, config = Config::default()))]
    fn py_new(config: Config) -> Self {
        Self::from(config)
    }

    /// Copy of the configuration the reader was created with
    #[getter]
    #[pyo3(name = "config")]
    fn py_config(&self) -> Config {
        self.config().clone()
    }

    /// Registry of the states saved in this reader's state directory
    #[getter]
    #[pyo3(name = "states")]
    fn py_states(&self) -> StateRegistry {
        self.states().clone()
    }

    /// Returns a `ByteIterator` that reads `file` one byte at a time
    ///
    /// A state name resumes the state saved under it, or starts fresh when none is saved, and a
    /// `State` resumes from itself. `state=None` uses the auto-named state of the file, which
    /// resumes only with `auto_load_state`. An iterator that does not start from the saved state
    /// overwrites it on its first save. Raises `OSError` when the file or the saved state cannot be
    /// read, `ValueError` for an invalid state name or `options.start` greater than `options.end`,
    /// `StateError` for an aliased state name, and one of its subclasses when the saved or passed
    /// state is corrupt or cannot resume on the file
    #[pyo3(name = "bytes", signature = (
        file, *, state = StateSource::Auto, options = IteratorOptions::default()
    ))]
    fn py_bytes(
        &self,
        py: Python<'_>,
        file: PathBuf,
        state: StateSource,
        options: IteratorOptions,
    ) -> PyResult<Py<ByteIterator>> {
        let iterator = self.bytes(file).state(state).options(options).build()?;

        Py::new(py, (ByteIterator(iterator), IteratorBase))
    }

    /// Returns a `ChunkIterator` that reads `file` in chunks of `chunk_size` bytes, 1024 by default
    ///
    /// A state name resumes the state saved under it, or starts fresh when none is saved, and a
    /// `State` resumes from itself. `state=None` uses the auto-named state of the file, which
    /// resumes only with `auto_load_state`. An iterator that does not start from the saved state
    /// overwrites it on its first save. Raises `OSError` when the file or the saved state cannot be
    /// read, `ValueError` for an invalid state name or `options.start` greater than `options.end`,
    /// `StateError` for an aliased state name, and one of its subclasses when the saved or passed
    /// state is corrupt or cannot resume on the file
    #[pyo3(name = "chunks", signature = (
        file, *,
        state = StateSource::Auto,
        options = IteratorOptions::default(),
        chunk_size = DEFAULT_CHUNK_SIZE,
        drop_partial = false
    ))]
    fn py_chunks(
        &self,
        py: Python<'_>,
        file: PathBuf,
        state: StateSource,
        options: IteratorOptions,
        chunk_size: usize,
        drop_partial: bool,
    ) -> PyResult<Py<ChunkIterator>> {
        let builder = self.chunks(file).size(chunk_size).drop_partial(drop_partial);
        let iterator = builder.state(state).options(options).build()?;

        Py::new(py, (ChunkIterator(iterator), IteratorBase))
    }

    /// Returns a `LineIterator` that reads `file` as UTF-8 lines
    ///
    /// With `align_start`, a partial line at `options.start` is skipped unless a resumed state is
    /// already past it
    ///
    /// A state name resumes the state saved under it, or starts fresh when none is saved, and a
    /// `State` resumes from itself. `state=None` uses the auto-named state of the file, which
    /// resumes only with `auto_load_state`. An iterator that does not start from the saved state
    /// overwrites it on its first save. Raises `OSError` when the file or the saved state cannot be
    /// read, `ValueError` for an invalid state name or `options.start` greater than `options.end`,
    /// `StateError` for an aliased state name, and one of its subclasses when the saved or passed
    /// state is corrupt or cannot resume on the file
    #[pyo3(name = "lines", signature = (
        file, *,
        state = StateSource::Auto,
        options = IteratorOptions::default(),
        keepends = false,
        align_start = false,
        skip_empty = false
    ))]
    #[expect(clippy::too_many_arguments)]
    fn py_lines(
        &self,
        py: Python<'_>,
        file: PathBuf,
        state: StateSource,
        options: IteratorOptions,
        keepends: bool,
        align_start: bool,
        skip_empty: bool,
    ) -> PyResult<Py<LineIterator>> {
        let builder = self.lines(file).keepends(keepends).align(align_start).skip_empty(skip_empty);
        let iterator = builder.state(state).options(options).build()?;

        Py::new(py, (LineIterator(iterator), IteratorBase))
    }

    /// Returns a `SegmentIterator` that reads `file` as segments ended by `delimiter`
    ///
    /// `delimiter` must be a one-byte `bytes` and defaults to `b","`; any other length raises
    /// `ValueError` and a one-character `str` raises `TypeError`. With `align_start`, a partial
    /// segment at `options.start` is skipped unless a resumed state is already past it
    ///
    /// A state name resumes the state saved under it, or starts fresh when none is saved, and a
    /// `State` resumes from itself. `state=None` uses the auto-named state of the file, which
    /// resumes only with `auto_load_state`. An iterator that does not start from the saved state
    /// overwrites it on its first save. Raises `OSError` when the file or the saved state cannot be
    /// read, `ValueError` for an invalid state name or `options.start` greater than `options.end`,
    /// `StateError` for an aliased state name, and one of its subclasses when the saved or passed
    /// state is corrupt or cannot resume on the file
    #[pyo3(name = "segments", signature = (
        file, *,
        state = StateSource::Auto,
        options = IteratorOptions::default(),
        delimiter = [DEFAULT_DELIMITER],
        keep_delimiter = false,
        align_start = false,
        skip_empty = false
    ))]
    #[expect(clippy::too_many_arguments)]
    fn py_segments(
        &self,
        py: Python<'_>,
        file: PathBuf,
        state: StateSource,
        options: IteratorOptions,
        delimiter: [u8; 1],
        keep_delimiter: bool,
        align_start: bool,
        skip_empty: bool,
    ) -> PyResult<Py<SegmentIterator>> {
        let builder = self
            .segments(file)
            .delimiter(delimiter[0])
            .keep(keep_delimiter)
            .align(align_start)
            .skip_empty(skip_empty);
        let iterator = builder.state(state).options(options).build()?;

        Py::new(py, (SegmentIterator(iterator), IteratorBase))
    }

    /// Returns the class name with the configuration
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        pyrepr!(slf, config)
    }
}
