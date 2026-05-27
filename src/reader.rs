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
}

#[pymethods]
impl PReader {
    #[new]
    #[pyo3(signature = (*, config=PReaderConfig::default()))]
    fn new(config: PReaderConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    #[pyo3(signature = (file, *, state=None, state_name=None))]
    fn bytes(
        &self,
        py: Python<'_>,
        file: PathBuf,
        state: Option<PReaderState>,
        state_name: Option<String>,
    ) -> PyResult<Py<PReaderByteIterator>> {
        let file = file.canonicalize()?;
        let state = self.resolve_state(&file, state, state_name)?;

        Py::new(py, PReaderByteIterator::new((&self.config).into(), state)?)
    }

    #[pyo3(signature = (file, *, state=None, state_name=None, chunk_size=DEFAULT_CHUNK_SIZE))]
    fn chunks(
        &self,
        py: Python<'_>,
        file: PathBuf,
        state: Option<PReaderState>,
        state_name: Option<String>,
        chunk_size: usize,
    ) -> PyResult<Py<PReaderChunkIterator>> {
        let file = file.canonicalize()?;
        let state = self.resolve_state(&file, state, state_name)?;
        let config = (&self.config).into();

        Py::new(py, PReaderChunkIterator::new(config, state, chunk_size)?)
    }

    #[pyo3(signature = (file, *, state=None, state_name=None))]
    fn lines(
        &self,
        py: Python<'_>,
        file: PathBuf,
        state: Option<PReaderState>,
        state_name: Option<String>,
    ) -> PyResult<Py<PReaderLineIterator>> {
        let file = file.canonicalize()?;
        let state = self.resolve_state(&file, state, state_name)?;

        Py::new(py, PReaderLineIterator::new((&self.config).into(), state)?)
    }

    #[pyo3(signature = (file, *, state=None, state_name=None, delimiter))]
    fn delimiter(
        &self,
        py: Python<'_>,
        file: PathBuf,
        state: Option<PReaderState>,
        state_name: Option<String>,
        delimiter: char,
    ) -> PyResult<Py<PReaderDelimiterIterator>> {
        let Ok(delimiter) = u8::try_from(delimiter) else {
            return Err(PyValueError::new_err(
                "delimiter must fit in a single byte (0-255)",
            ));
        };
        let file = file.canonicalize()?;
        let state = self.resolve_state(&file, state, state_name)?;
        let config = (&self.config).into();

        Py::new(py, PReaderDelimiterIterator::new(config, state, delimiter)?)
    }
}

impl PReader {
    fn resolve_state(
        &self,
        file: &PathBuf,
        explicit: Option<PReaderState>,
        state_name: Option<String>,
    ) -> PyResult<PReaderState> {
        if let Some(state) = explicit {
            if self.config.verify_state {
                state.verify()?;
            }
            return Ok(state);
        }

        let manager = PReaderStateManager::from(self.config.clone());
        let name = state_name.unwrap_or(manager.name(file));

        if self.config.auto_load_state {
            if let Ok(state) = manager.load(&name) {
                return Ok(state);
            }
        }

        let state = PReaderState::new(self.config.clone(), file, Some(name));

        state.map_err(PReaderStateError::from_anyhow)
    }
}
