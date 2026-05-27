use std::{
    fs::File,
    io::{BufRead, BufReader, Seek, SeekFrom},
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

    fn read_segment(&mut self) -> std::io::Result<Option<&[u8]>> {
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

        let Some(segment) = slf.read_segment()? else {
            return Ok(None);
        };

        let consumed = segment.len() as u64;
        let value = PyBytes::new(py, segment).unbind().into_any();

        slf.state.advance(consumed);

        if slf.config.auto_save_state {
            let delta = slf.state.position - slf.state.manager.last_saved_position;

            if delta >= slf.config.auto_save_state_bytes {
                slf.state.save()?;
            }
        }

        Ok(Some(value))
    }
}

impl Drop for PReaderDelimiterIterator {
    fn drop(&mut self) {
        if !self.config.auto_save_state {
            return;
        }

        if self.state.position <= self.state.manager.last_saved_position {
            return;
        }

        Python::try_attach(|_| {
            self.state.save().ok();
        });
    }
}
