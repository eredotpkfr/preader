use std::{
    fs::File,
    io::{BufRead, BufReader, Result, Seek, SeekFrom},
};

use pyo3::{prelude::*, types::PyString};

use crate::{
    iterators::reader::PReaderIterator,
    types::{PReaderItem, PReaderState, config::PReaderIteratorConfig},
};

#[pyclass]
pub struct PReaderLineIterator {
    config: PReaderIteratorConfig,
    state: PReaderState,
    reader: BufReader<File>,
    buffer: String,
}

impl PReaderIterator for PReaderLineIterator {
    fn config(&self) -> &PReaderIteratorConfig {
        &self.config
    }

    fn state(&mut self) -> &mut PReaderState {
        &mut self.state
    }
}

impl PReaderLineIterator {
    pub(crate) fn new(config: PReaderIteratorConfig, mut state: PReaderState) -> PyResult<Self> {
        let file = File::open(&state.file.path)?;
        let mut reader = BufReader::with_capacity(config.buffer_capacity, file);
        let buffer = String::new();

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

    fn read_line(&mut self) -> Result<Option<(String, u64)>> {
        self.buffer.clear();

        let read_count = self.reader.read_line(&mut self.buffer)?;

        if read_count == 0 {
            return Ok(None);
        }

        let trimmed = self.buffer.trim_end_matches('\n').trim_end_matches('\r').to_owned();

        Ok(Some((trimmed, read_count as u64)))
    }
}

#[pymethods]
impl PReaderLineIterator {
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
        let saver = slf.config().saver();

        let Some((line, read_count)) = slf.read_line()? else {
            (saver)(&mut slf.state, 0)?;

            return Ok(None);
        };

        let value = PyString::new(py, &line).unbind().into_any();
        let threshold = slf.config.auto_save_state_bytes;

        slf.state.advance(read_count);
        (saver)(&mut slf.state, threshold)?;

        Ok(Some(value))
    }
}

impl Drop for PReaderLineIterator {
    fn drop(&mut self) {
        Python::try_attach(|_| {
            if let Err(e) = (self.config().saver())(&mut self.state, 0) {
                eprintln!("preader: save failed: {e}");
            }
        });
    }
}
