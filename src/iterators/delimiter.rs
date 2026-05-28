use std::{
    fs::File,
    io::{BufRead, BufReader, Result, Seek, SeekFrom},
};

use pyo3::{prelude::*, types::PyBytes};

use crate::{
    iterators::reader::PReaderIterator,
    types::{PReaderItem, PReaderState, config::PReaderIteratorConfig},
};

#[pyclass]
pub struct PReaderDelimiterIterator {
    config: PReaderIteratorConfig,
    state: PReaderState,
    reader: BufReader<File>,
    delimiter: u8,
    buffer: Vec<u8>,
}

impl PReaderIterator for PReaderDelimiterIterator {
    fn config(&self) -> &PReaderIteratorConfig {
        &self.config
    }

    fn state(&mut self) -> &mut PReaderState {
        &mut self.state
    }
}

impl PReaderDelimiterIterator {
    pub(crate) fn new(
        config: PReaderIteratorConfig,
        mut state: PReaderState,
        delimiter: u8,
    ) -> PyResult<Self> {
        let file = File::open(state.file.path.clone())?;
        let mut reader = BufReader::with_capacity(config.buffer_capacity, file);
        let buffer = Vec::new();

        if state.position > 0 {
            reader.seek(SeekFrom::Start(state.position))?;
        }

        state.manager.last_saved_position = state.position;

        Ok(Self {
            config,
            state,
            reader,
            delimiter,
            buffer,
        })
    }

    fn read_segment(&mut self) -> Result<Option<&[u8]>> {
        self.buffer.clear();
        self.reader.read_until(self.delimiter, &mut self.buffer)?;

        Ok((!self.buffer.is_empty()).then_some(self.buffer.as_slice()))
    }
}

#[pymethods]
impl PReaderDelimiterIterator {
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

        let Some(segment) = slf.read_segment()? else {
            (saver)(&mut slf.state, 0)?;

            return Ok(None);
        };

        let consumed = segment.len() as u64;
        let value = PyBytes::new(py, segment).unbind().into_any();
        let threshold = slf.config.auto_save_state_bytes;

        slf.state.advance(consumed);
        (saver)(&mut slf.state, threshold)?;

        Ok(Some(value))
    }
}

impl Drop for PReaderDelimiterIterator {
    fn drop(&mut self) {
        Python::try_attach(|_| {
            (self.saver())(&mut self.state, 0).unwrap();
        });
    }
}
