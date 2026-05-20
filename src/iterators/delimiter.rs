use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use pyo3::{prelude::*, types::PyBytes};

use crate::iterators::base::PReaderBaseFileIterator;

#[pyclass(extends = PReaderBaseFileIterator)]
pub struct PReaderDelimiterIterator {
    reader: BufReader<File>,
    delimiter: u8,
    buffer: Vec<u8>,
}

impl PReaderDelimiterIterator {
    pub fn new(
        reader: BufReader<File>,
        delimiter: u8,
    ) -> std::io::Result<PyClassInitializer<Self>> {
        let base = PReaderBaseFileIterator::try_from(reader.get_ref())?;
        let this = Self {
            reader,
            delimiter,
            buffer: Vec::new(),
        };
        let class = PyClassInitializer::from(base).add_subclass(this);

        Ok(class)
    }

    fn read_segment(&mut self) -> std::io::Result<Option<usize>> {
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

    fn __next__<'py>(mut slf: PyRefMut<'py, Self>) -> PyResult<Option<Bound<'py, PyBytes>>> {
        let Some(read_count) = slf.read_segment()? else {
            return Ok(None);
        };
        slf.as_super().bytes_read += read_count;

        Ok(Some(PyBytes::new(slf.py(), &slf.buffer)))
    }
}
