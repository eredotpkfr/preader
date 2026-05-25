use std::path::PathBuf;

use pyo3::{exceptions::PyValueError, prelude::*};

use crate::{
    PReaderState,
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
    pub manager: PReaderStateManager,
    pub file: PathBuf,
}

#[pymethods]
impl PReader {
    #[new]
    #[pyo3(signature = (file, *, config=PReaderConfig::default()))]
    fn new(file: PathBuf, config: PReaderConfig) -> Self {
        Self {
            config: config.clone(),
            manager: config.into(),
            file: file.canonicalize().unwrap(),
        }
    }

    #[pyo3(signature = (*, state=None))]
    fn bytes(
        &self,
        py: Python<'_>,
        state: Option<PReaderState>,
    ) -> PyResult<Py<PReaderByteIterator>> {
        let state = self.resolve_state(state, &self.file)?;

        Py::new(py, PReaderByteIterator::new((&self.config).into(), state)?)
    }

    #[pyo3(signature = (*, state=None, chunk_size=DEFAULT_CHUNK_SIZE))]
    fn chunks(
        &self,
        py: Python<'_>,
        state: Option<PReaderState>,
        chunk_size: usize,
    ) -> PyResult<Py<PReaderChunkIterator>> {
        let state = self.resolve_state(state, &self.file)?;

        Py::new(
            py,
            PReaderChunkIterator::new((&self.config).into(), state, chunk_size)?,
        )
    }

    #[pyo3(signature = (*, state=None))]
    fn lines(
        &self,
        py: Python<'_>,
        state: Option<PReaderState>,
    ) -> PyResult<Py<PReaderLineIterator>> {
        let state = self.resolve_state(state, &self.file)?;

        Py::new(py, PReaderLineIterator::new((&self.config).into(), state)?)
    }

    #[pyo3(signature = (*, state=None, delimiter))]
    fn delimiter(
        &self,
        py: Python<'_>,
        state: Option<PReaderState>,
        delimiter: char,
    ) -> PyResult<Py<PReaderDelimiterIterator>> {
        let Ok(delimiter_byte) = u8::try_from(delimiter) else {
            return Err(PyValueError::new_err(
                "delimiter must fit in a single byte (0-255)",
            ));
        };
        let state = self.resolve_state(state, &self.file)?;

        Py::new(
            py,
            PReaderDelimiterIterator::new((&self.config).into(), state, delimiter_byte)?,
        )
    }
}

impl PReader {
    fn resolve_state(
        &self,
        explicit: Option<PReaderState>,
        path: &PathBuf,
    ) -> PyResult<PReaderState> {
        if let Some(state) = explicit {
            return Ok(state);
        }

        if self.config.auto_load_state {
            if let Ok(state) = self.manager.load(self.manager.name(path)) {
                return Ok(state);
            }
        }

        Ok(PReaderState::try_from(path)?)
    }
}
