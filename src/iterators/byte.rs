use std::{
    fs::File,
    io::{BufReader, Bytes, Read, Result, Seek, SeekFrom},
};

use pyo3::{prelude::*, types::PyBytes};

use crate::{
    PReaderState,
    iterators::base::PReaderIteratorBase,
    types::{PReaderItem, config::PReaderIteratorConfig},
};

#[pyclass(extends = PReaderIteratorBase)]
pub struct PReaderByteIterator {
    bytes: Bytes<BufReader<File>>,
}

impl PReaderByteIterator {
    pub(crate) fn new(
        config: PReaderIteratorConfig,
        state: PReaderState,
    ) -> PyResult<PyClassInitializer<Self>> {
        let file = File::open(&state.file.path)?;
        let mut reader = BufReader::with_capacity(config.buffer_capacity, file);

        if state.position > 0 {
            reader.seek(SeekFrom::Start(state.position))?;
        }

        let base = PReaderIteratorBase::new(config, state);
        let sub = Self {
            bytes: reader.bytes(),
        };

        Ok(PyClassInitializer::from(base).add_subclass(sub))
    }

    #[inline]
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
            slf.as_super().finalize()?;

            return Ok(None);
        };

        let value = PyBytes::new(py, &[byte]).unbind().into_any();

        slf.as_super().advance(1)?;

        Ok(Some(value))
    }
}
