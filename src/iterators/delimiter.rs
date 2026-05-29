use std::{
    fs::File,
    io::{BufRead, BufReader, Result, Seek, SeekFrom},
};

use pyo3::{prelude::*, types::PyBytes};

use crate::{
    iterators::base::PReaderIteratorBase,
    types::{PReaderItem, PReaderState, config::PReaderIteratorConfig},
};

#[pyclass(extends = PReaderIteratorBase)]
pub struct PReaderDelimiterIterator {
    reader: BufReader<File>,
    delimiter: u8,
    buffer: Vec<u8>,
}

impl PReaderDelimiterIterator {
    pub(crate) fn new(
        config: PReaderIteratorConfig,
        state: PReaderState,
        delimiter: u8,
    ) -> PyResult<PyClassInitializer<Self>> {
        let file = File::open(&state.file.path)?;
        let mut reader = BufReader::with_capacity(config.buffer_capacity, file);
        let buffer = Vec::new();

        if state.position > 0 {
            reader.seek(SeekFrom::Start(state.position))?;
        }

        let base = PReaderIteratorBase::new(config, state);
        let sub = Self {
            reader,
            delimiter,
            buffer,
        };

        Ok(PyClassInitializer::from(base).add_subclass(sub))
    }

    fn read_segment(&mut self) -> Result<Option<&[u8]>> {
        self.buffer.clear();
        self.reader.read_until(self.delimiter, &mut self.buffer)?;

        Ok((!self.buffer.is_empty()).then_some(self.buffer.as_slice()))
    }
}

#[pymethods]
impl PReaderDelimiterIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> PyResult<Option<PReaderItem>> {
        let py = slf.py();

        let Some(segment) = slf.read_segment()? else {
            slf.as_super().finalize()?;

            return Ok(None);
        };

        let consumed = segment.len() as u64;
        let value = PyBytes::new(py, segment).unbind().into_any();

        slf.as_super().advance(consumed)?;

        Ok(Some(value))
    }
}
