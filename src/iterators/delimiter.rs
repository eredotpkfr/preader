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
    buffer: Vec<u8>,
    delimiter: u8,
    keep_delimiter: bool,
}

impl PReaderDelimiterIterator {
    pub(crate) fn new(
        config: PReaderIteratorConfig,
        state: PReaderState,
        delimiter: u8,
        keep_delimiter: bool,
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
            keep_delimiter,
        };

        Ok(PyClassInitializer::from(base).add_subclass(sub))
    }

    #[inline]
    fn read_segment(&mut self) -> Result<Option<(&[u8], u64)>> {
        self.buffer.clear();

        let read_count = self.reader.read_until(self.delimiter, &mut self.buffer)?;

        if read_count == 0 {
            return Ok(None);
        }

        let slice = if self.keep_delimiter {
            self.buffer.as_slice()
        } else {
            self.buffer.strip_suffix(&[self.delimiter]).unwrap_or(&self.buffer)
        };

        Ok(Some((slice, read_count as u64)))
    }
}

#[pymethods]
impl PReaderDelimiterIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> PyResult<Option<PReaderItem>> {
        let py = slf.py();

        let Some((segment, read_count)) = slf.read_segment()? else {
            slf.as_super().finalize()?;

            return Ok(None);
        };

        let value = PyBytes::new(py, segment).unbind().into_any();

        slf.as_super().advance(read_count)?;

        Ok(Some(value))
    }
}
