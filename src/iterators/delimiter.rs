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

    fn read_segment(&mut self) -> Result<Option<usize>> {
        self.buffer.clear();

        let read_count = self.reader.read_until(self.delimiter, &mut self.buffer)?;

        Ok((read_count > 0).then_some(read_count))
    }
}

#[pymethods]
impl PReaderDelimiterIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> PyResult<Option<PReaderItem>> {
        let Some(read_count) = slf.read_segment()? else {
            return Ok(None);
        };
        let value = PyBytes::new(slf.py(), &slf.buffer).unbind();

        Ok(Some(slf.progress.yield_item(value, read_count)))
    }
}
