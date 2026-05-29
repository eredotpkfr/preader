use std::{
    fs::File,
    io::{BufReader, Bytes, Read, Result, Seek, SeekFrom},
};

use pyo3::{prelude::*, types::PyBytes};

use crate::{
    PReaderState,
    iterators::reader::PReaderIterator,
    types::{PReaderItem, config::PReaderIteratorConfig},
};

#[pyclass]
pub struct PReaderByteIterator {
    config: PReaderIteratorConfig,
    state: PReaderState,
    bytes: Bytes<BufReader<File>>,
}

impl PReaderIterator for PReaderByteIterator {
    fn config(&self) -> &PReaderIteratorConfig {
        &self.config
    }

    fn state(&mut self) -> &mut PReaderState {
        &mut self.state
    }
}

impl PReaderByteIterator {
    pub(crate) fn new(config: PReaderIteratorConfig, mut state: PReaderState) -> PyResult<Self> {
        let file = File::open(&state.file.path)?;
        let mut reader = BufReader::with_capacity(config.buffer_capacity, file);

        if state.position > 0 {
            reader.seek(SeekFrom::Start(state.position))?;
        }

        state.manager.last_saved_position = state.position;

        Ok(Self {
            config,
            state,
            bytes: reader.bytes(),
        })
    }

    fn read_byte(&mut self) -> Result<Option<u8>> {
        self.bytes.next().transpose()
    }
}

#[pymethods]
impl PReaderByteIterator {
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

        let Some(byte) = slf.read_byte()? else {
            (saver)(&mut slf.state, 0)?;

            return Ok(None);
        };

        let value = PyBytes::new(py, &[byte]).unbind();
        let threshold = slf.config.auto_save_state_bytes;

        slf.state.advance(1);
        (saver)(&mut slf.state, threshold)?;

        Ok(Some(value.into_any()))
    }
}

impl Drop for PReaderByteIterator {
    fn drop(&mut self) {
        Python::try_attach(|_| {
            if let Err(e) = (self.config().saver())(&mut self.state, 0) {
                eprintln!("preader: save failed: {e}");
            }
        });
    }
}
