use std::{
    fs::File,
    io::{BufRead, BufReader, Result},
};

use pyo3::{prelude::*, types::PyBytes};

use crate::types::{PReaderItem, progress::ProgressState};

#[pyclass]
pub struct PReaderDelimiterIterator {
    progress: ProgressState,
    reader: BufReader<File>,
    delimiter: u8,
    buffer: Vec<u8>,
}

impl PReaderDelimiterIterator {
    pub fn new(reader: BufReader<File>, delimiter: u8) -> Result<Self> {
        Ok(Self {
            progress: ProgressState::try_from(reader.get_ref())?,
            reader,
            delimiter,
            buffer: Vec::new(),
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
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> PyResult<Option<PReaderItem>> {
        let py = slf.py();

        let Some(segment) = slf.read_segment()? else {
            return Ok(None);
        };

        let value = PyBytes::new(py, segment).unbind();
        let consumed = segment.len();

        slf.progress.advance(consumed);

        Ok(Some((&slf.progress, value).into()))
    }
}
