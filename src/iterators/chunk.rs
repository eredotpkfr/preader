use std::{
    fs::File,
    io::{BufReader, Read, Result},
};

use pyo3::{prelude::*, types::PyBytes};

use crate::types::{PReaderItem, progress::ProgressState};

#[pyclass]
pub struct PReaderChunkIterator {
    progress: ProgressState,
    reader: BufReader<File>,
    buffer: Vec<u8>,
}

impl PReaderChunkIterator {
    pub fn new(reader: BufReader<File>, chunk_size: usize) -> Result<Self> {
        Ok(Self {
            progress: ProgressState::try_from(reader.get_ref())?,
            reader,
            buffer: vec![0u8; chunk_size],
        })
    }

    fn read_chunk(&mut self) -> Result<Option<&[u8]>> {
        let read_count = self.reader.read(&mut self.buffer)?;

        Ok((read_count > 0).then_some(&self.buffer[..read_count]))
    }
}

#[pymethods]
impl PReaderChunkIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> PyResult<Option<PReaderItem>> {
        let py = slf.py();

        let Some(chunk) = slf.read_chunk()? else {
            return Ok(None);
        };

        let value = PyBytes::new(py, chunk).unbind();
        let consumed = chunk.len();

        Ok(Some(slf.progress.yield_item(value, consumed)))
    }
}
