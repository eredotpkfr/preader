use std::path::PathBuf;

use pyo3::{exceptions::PyValueError, prelude::*};

use crate::{
    StateRegistry,
    enums::StateInput,
    iterators::{ByteIterator, ChunkIterator, DelimiterIterator, LineIterator},
    types::{IteratorOptions, config::Config},
};

pub const DEFAULT_CHUNK_SIZE: usize = 1024;

#[pyclass]
pub struct PReader {
    #[pyo3(get)]
    pub config: Config,
}

#[pymethods]
impl PReader {
    #[new]
    #[pyo3(signature = (*, config=Config::default()))]
    fn new(config: Config) -> Self {
        Self { config }
    }

    fn states(&self) -> StateRegistry {
        (&self.config).into()
    }

    #[pyo3(signature = (file, *, state=StateInput::Default, options=IteratorOptions::default()))]
    fn bytes(
        &self,
        py: Python<'_>,
        file: PathBuf,
        state: StateInput,
        options: IteratorOptions,
    ) -> PyResult<Py<ByteIterator>> {
        let file = file.canonicalize()?;
        let state = state.resolve(&self.config, &file)?;
        let iterator = ByteIterator::new((&self.config).into(), state, options)?;

        Py::new(py, iterator)
    }

    #[pyo3(signature = (
        file,
        *,
        state = StateInput::Default,
        options = IteratorOptions::default(),
        chunk_size = DEFAULT_CHUNK_SIZE,
        drop_partial = false,
    ))]
    fn chunks(
        &self,
        py: Python<'_>,
        file: PathBuf,
        state: StateInput,
        options: IteratorOptions,
        chunk_size: usize,
        drop_partial: bool,
    ) -> PyResult<Py<ChunkIterator>> {
        let file = file.canonicalize()?;
        let state = state.resolve(&self.config, &file)?;
        let iterator = ChunkIterator::new(
            (&self.config).into(),
            state,
            chunk_size,
            options,
            drop_partial,
        )?;

        Py::new(py, iterator)
    }

    #[pyo3(signature = (
        file,
        *,
        state = StateInput::Default,
        options = IteratorOptions::default(),
        keepends = false,
        align_start = false,
        skip_empty = false,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn lines(
        &self,
        py: Python<'_>,
        file: PathBuf,
        state: StateInput,
        options: IteratorOptions,
        keepends: bool,
        align_start: bool,
        skip_empty: bool,
    ) -> PyResult<Py<LineIterator>> {
        let file = file.canonicalize()?;
        let state = state.resolve(&self.config, &file)?;
        let iterator = LineIterator::new(
            (&self.config).into(),
            state,
            keepends,
            options,
            align_start,
            skip_empty,
        )?;

        Py::new(py, iterator)
    }

    #[pyo3(signature = (
        file,
        *,
        state = StateInput::Default,
        options = IteratorOptions::default(),
        delimiter,
        keep_delimiter = false,
        align_start = false,
        skip_empty = false,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn delimiter(
        &self,
        py: Python<'_>,
        file: PathBuf,
        state: StateInput,
        options: IteratorOptions,
        delimiter: char,
        keep_delimiter: bool,
        align_start: bool,
        skip_empty: bool,
    ) -> PyResult<Py<DelimiterIterator>> {
        let delimiter = u8::try_from(delimiter)
            .map_err(|_| PyValueError::new_err("delimiter must fit in a single byte"))?;
        let file = file.canonicalize()?;
        let state = state.resolve(&self.config, &file)?;
        let iterator = DelimiterIterator::new(
            (&self.config).into(),
            state,
            delimiter,
            keep_delimiter,
            options,
            align_start,
            skip_empty,
        )?;

        Py::new(py, iterator)
    }
}
