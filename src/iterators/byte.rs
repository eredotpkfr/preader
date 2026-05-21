use std::{
    fs::File,
    io::{BufReader, Bytes, Read, Result},
};

use pyo3::{prelude::*, types::PyBytes};

use crate::types::{PReaderItem, progress::ProgressState};

#[pyclass]
pub struct PReaderByteIterator {
    progress: ProgressState,
    bytes: Bytes<BufReader<File>>,
}

impl PReaderByteIterator {
    pub fn new(reader: BufReader<File>) -> Result<Self> {
        Ok(Self {
            progress: ProgressState::try_from(reader.get_ref())?,
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

        Ok(Some(slf.progress.yield_item(value, 1)))
    }
}
