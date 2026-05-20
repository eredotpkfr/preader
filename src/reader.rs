use std::{fs::File, io::BufReader, path::PathBuf};

use pyo3::{exceptions::PyValueError, prelude::*};

use crate::{
    iterators::{
        PReaderByteIterator, PReaderChunkIterator, PReaderDelimiterIterator, PReaderLineIterator,
    },
    types::PReaderConfig,
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
        let file = File::open(path)?;
        let reader = BufReader::with_capacity(self.config.buffer_capacity, file);
        let iterator = PReaderByteIterator::new(reader)?;

        Py::new(py, iterator)
    }

    #[pyo3(signature = (path, chunk_size = DEFAULT_CHUNK_SIZE))]
    fn chunks(
        &self,
        py: Python<'_>,
        path: PathBuf,
        chunk_size: usize,
    ) -> PyResult<Py<PReaderChunkIterator>> {
        let file = File::open(path)?;
        let reader = BufReader::with_capacity(self.config.buffer_capacity, file);
        let iterator = PReaderChunkIterator::new(reader, chunk_size)?;

        Py::new(py, iterator)
    }

    fn lines(&self, py: Python<'_>, path: PathBuf) -> PyResult<Py<PReaderLineIterator>> {
        let file = File::open(path)?;
        let reader = BufReader::with_capacity(self.config.buffer_capacity, file);
        let iterator = PReaderLineIterator::new(reader)?;

        Py::new(py, iterator)
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

        let file = File::open(path)?;
        let reader = BufReader::with_capacity(self.config.buffer_capacity, file);
        let iterator = PReaderDelimiterIterator::new(reader, delimiter_byte)?;

        Py::new(py, iterator)
    }
}
