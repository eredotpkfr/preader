use std::{
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
};

use pyo3::{prelude::*, types::PyBytes};

use crate::{
    iterators::reader::PReaderIterator,
    types::{PReaderItem, PReaderState, config::PReaderIteratorConfig},
};

#[pyclass]
pub struct PReaderChunkIterator {
    config: PReaderIteratorConfig,
    state: PReaderState,
    reader: BufReader<File>,
    buffer: Vec<u8>,
}

impl PReaderIterator for PReaderChunkIterator {
    fn config(&self) -> &PReaderIteratorConfig {
        &self.config
    }

    fn state(&mut self) -> &mut PReaderState {
        &mut self.state
    }
}

impl PReaderChunkIterator {
    pub(crate) fn new(
        config: PReaderIteratorConfig,
        mut state: PReaderState,
        chunk_size: usize,
    ) -> PyResult<Self> {
        let file = File::open(state.file.path.clone())?;
        let mut reader = BufReader::with_capacity(config.buffer_capacity, file);
        let buffer = vec![0u8; chunk_size];

        if state.position > 0 {
            reader.seek(SeekFrom::Start(state.position))?;
        }

        state.manager.last_saved_position = state.position;

        Ok(Self {
            config,
            state,
            reader,
            buffer,
        })
    }

    fn read_chunk(&mut self) -> std::io::Result<Option<&[u8]>> {
        let read_count = self.reader.read(&mut self.buffer)?;

        Ok((read_count > 0).then_some(&self.buffer[..read_count]))
    }
}

#[pymethods]
impl PReaderChunkIterator {
    fn state(&self) -> PReaderState {
        self.state.clone()
    }

    fn percent(&self) -> f64 {
        self.state.percent()
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> PyResult<Option<PReaderItem>> {
        let py = slf.py();
        let saver = slf.saver();

        let Some(chunk) = slf.read_chunk()? else {
            (saver)(&mut slf.state, 0)?;

            return Ok(None);
        };

        let consumed = chunk.len() as u64;
        let value = PyBytes::new(py, chunk).unbind().into_any();
        let threshold = slf.config.auto_save_state_bytes;

        slf.state.advance(consumed);
        (saver)(&mut slf.state, threshold)?;

        Ok(Some(value))
    }
}

impl Drop for PReaderChunkIterator {
    fn drop(&mut self) {
        Python::try_attach(|_| {
            (self.saver())(&mut self.state, 0).unwrap();
        });
    }
}
