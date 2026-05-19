use std::{fs::File, path::PathBuf};

use pyo3::{exceptions::PyValueError, prelude::*};

use crate::{
    config::PReaderConfig,
    iterators::{
        PReaderByteIterator, PReaderChunkIterator, PReaderDelimiterIterator, PReaderLineIterator,
    },
};

const DEFAULT_CHUNK_SIZE: usize = 1024;

#[pyclass]
pub struct PReader {
    #[pyo3(get)]
    pub config: PReaderConfig,
}

#[pymethods]
impl PReader {
    #[new]
    #[pyo3(signature = (config=PReaderConfig::default()))]
    fn new(config: PReaderConfig) -> Self {
        Self { config }
    }

    fn bytes(&self, py: Python<'_>, path: PathBuf) -> PyResult<Py<PReaderByteIterator>> {
        Py::new(
            py,
            PReaderByteIterator::new(File::open(&path)?, self.config.buffer_capacity)?,
        )
    }

    #[pyo3(signature = (path, chunk_size = DEFAULT_CHUNK_SIZE))]
    fn chunks(
        &self,
        py: Python<'_>,
        path: PathBuf,
        chunk_size: usize,
    ) -> PyResult<Py<PReaderChunkIterator>> {
        Py::new(
            py,
            PReaderChunkIterator::new(File::open(&path)?, self.config.buffer_capacity, chunk_size)?,
        )
    }

    fn lines(&self, py: Python<'_>, path: PathBuf) -> PyResult<Py<PReaderLineIterator>> {
        Py::new(
            py,
            PReaderLineIterator::new(File::open(&path)?, self.config.buffer_capacity)?,
        )
    }

    fn delimiter(
        &self,
        py: Python<'_>,
        path: PathBuf,
        delimiter: char,
    ) -> PyResult<Py<PReaderDelimiterIterator>> {
        let Ok(delimiter_byte) = u8::try_from(delimiter) else {
            return Err(PyValueError::new_err(
                "delimiter must fit in a single byte (0-255)",
            ));
        };

        Py::new(
            py,
            PReaderDelimiterIterator::new(
                File::open(&path)?,
                self.config.buffer_capacity,
                delimiter_byte,
            )?,
        )
    }
}
