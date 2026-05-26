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
        let file = File::open(state.file.path.clone())?;
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
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> PyResult<Option<PReaderItem>> {
        let py = slf.py();

        let Some(byte) = slf.read_byte()? else {
            return Ok(None);
        };

        let value = PyBytes::new(py, &[byte]).unbind();

        slf.state().advance(1);

        if slf.config.auto_save_state {
            let delta = slf.state.position - slf.state.manager.last_saved_position;

            if delta >= slf.config.auto_save_state_bytes {
                slf.state().save()?;
            }
        }

        Ok(Some(value.into_any()))
    }
}

impl Drop for PReaderByteIterator {
    fn drop(&mut self) {
        if !self.config.auto_save_state {
            return;
        }

        if self.state.position <= self.state.manager.last_saved_position {
            return;
        }

        Python::try_attach(|_| self.state.save().unwrap());
    }
}
