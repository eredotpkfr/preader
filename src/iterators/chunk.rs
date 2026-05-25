use std::{
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
};

use pyo3::{prelude::*, types::PyBytes};

use crate::types::{PReaderItem, PReaderState, config::PReaderIteratorConfig};

#[pyclass]
pub struct PReaderChunkIterator {
    config: PReaderIteratorConfig,
    state: PReaderState,
    reader: BufReader<File>,
    buffer: Vec<u8>,
}

impl PReaderChunkIterator {
    pub(crate) fn new(
        config: PReaderIteratorConfig,
        state: PReaderState,
        chunk_size: usize,
    ) -> PyResult<Self> {
        let file = File::open(state.file.path.clone())?;
        let mut reader = BufReader::with_capacity(config.buffer_capacity, file);

        if state.position > 0 {
            reader.seek(SeekFrom::Start(state.position))?;
        }

        Ok(Self {
            config,
            state,
            reader,
            buffer: vec![0u8; chunk_size],
        })
    }

    fn read_chunk(&mut self) -> std::io::Result<Option<&[u8]>> {
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

        let consumed = chunk.len() as u64;
        let value = PyBytes::new(py, chunk).unbind().into_any();

        slf.state.advance(consumed);

        Ok(Some(value))
    }
}
