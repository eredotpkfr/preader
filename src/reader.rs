use std::path::PathBuf;

use pyo3::{exceptions::PyValueError, prelude::*};

use crate::{
    PReaderState, PReaderStateError,
    iterators::{
        PReaderByteIterator, PReaderChunkIterator, PReaderDelimiterIterator, PReaderLineIterator,
    },
    types::{PReaderStateManager, config::PReaderConfig},
};

pub const DEFAULT_CHUNK_SIZE: usize = 1024;

#[pyclass]
pub struct PReader {
    #[pyo3(get)]
    pub config: PReaderConfig,
    #[pyo3(get)]
    pub file: PathBuf,
}

#[pymethods]
impl PReader {
    #[new]
    #[pyo3(signature = (file, *, config=PReaderConfig::default()))]
    fn new(file: PathBuf, config: PReaderConfig) -> Self {
        Self {
            config: config.clone(),
            file: file.canonicalize().unwrap(),
        }
    }

    #[pyo3(signature = (*, state=None))]
    fn bytes(
        &self,
        py: Python<'_>,
        state: Option<PReaderState>,
    ) -> PyResult<Py<PReaderByteIterator>> {
        let state = self.resolve_state(state)?;

        Py::new(py, PReaderByteIterator::new((&self.config).into(), state)?)
    }

    #[pyo3(signature = (*, state=None, chunk_size=DEFAULT_CHUNK_SIZE))]
    fn chunks(
        &self,
        py: Python<'_>,
        state: Option<PReaderState>,
        chunk_size: usize,
    ) -> PyResult<Py<PReaderChunkIterator>> {
        let state = self.resolve_state(state)?;
        let config = (&self.config).into();

        Py::new(py, PReaderChunkIterator::new(config, state, chunk_size)?)
    }

    #[pyo3(signature = (*, state=None))]
    fn lines(
        &self,
        py: Python<'_>,
        state: Option<PReaderState>,
    ) -> PyResult<Py<PReaderLineIterator>> {
        let state = self.resolve_state(state)?;

        Py::new(py, PReaderLineIterator::new((&self.config).into(), state)?)
    }

    #[pyo3(signature = (*, state=None, delimiter))]
    fn delimiter(
        &self,
        py: Python<'_>,
        state: Option<PReaderState>,
        delimiter: char,
    ) -> PyResult<Py<PReaderDelimiterIterator>> {
        let Ok(delimiter) = u8::try_from(delimiter) else {
            return Err(PyValueError::new_err(
                "delimiter must fit in a single byte (0-255)",
            ));
        };
        let state = self.resolve_state(state)?;
        let config = (&self.config).into();

        Py::new(py, PReaderDelimiterIterator::new(config, state, delimiter)?)
    }
}

impl PReader {
    fn resolve_state(&self, explicit: Option<PReaderState>) -> PyResult<PReaderState> {
        if let Some(state) = explicit {
            return Ok(state);
        }

        if self.config.auto_load_state {
            if let Ok(state) = PReaderStateManager::load(&self.config.clone().into(), &self.file) {
                return Ok(state);
            }
        }

        let state = PReaderState::try_from((self.config.clone(), &self.file));

        Ok(state.map_err(PReaderStateError::from_anyhow)?)
    }
}
