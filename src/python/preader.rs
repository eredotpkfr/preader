use std::path::PathBuf;

use pyo3::{exceptions::PyValueError, prelude::*};

use crate::{
    Config, DEFAULT_CHUNK_SIZE, IteratorBuild, IteratorOptions, PReader, StateRegistry,
    StateSource,
    python::{
        bases::iterator::IteratorBase,
        iterators::{
            byte::ByteIterator, chunk::ChunkIterator, delimiter::DelimiterIterator,
            line::LineIterator,
        },
        macros::repr::pyrepr,
    },
};

#[pymethods]
impl PReader {
    #[new]
    #[pyo3(signature = (*, config = Config::default()))]
    fn py_new(config: Config) -> Self {
        Self::from(config)
    }

    #[getter]
    #[pyo3(name = "config")]
    fn py_config(&self) -> Config {
        self.config().clone()
    }

    #[getter]
    #[pyo3(name = "states")]
    fn py_states(&self) -> StateRegistry {
        self.states().clone()
    }

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

    #[pyo3(name = "delimiter", signature = (
        file, *,
        state = StateSource::Auto,
        options = IteratorOptions::default(),
        delimiter,
        keep_delimiter = false,
        align_start = false,
        skip_empty = false
    ))]
    #[expect(clippy::too_many_arguments)]
    fn py_delimiter(
        &self,
        py: Python<'_>,
        file: PathBuf,
        state: StateSource,
        options: IteratorOptions,
        delimiter: char,
        keep_delimiter: bool,
        align_start: bool,
        skip_empty: bool,
    ) -> PyResult<Py<DelimiterIterator>> {
        let character = u8::try_from(delimiter)
            .map_err(|_| PyValueError::new_err("delimiter must fit in a single byte"))?;
        let builder = self
            .delimiter(file)
            .character(character)
            .keep(keep_delimiter)
            .align(align_start)
            .skip_empty(skip_empty);
        let iterator = builder.state(state).options(options).build()?;

        Py::new(py, (DelimiterIterator(iterator), IteratorBase))
    }

    fn __repr__(&self) -> String {
        pyrepr!("PReader" {
            config = self.config().__repr__(),
        })
    }
}
