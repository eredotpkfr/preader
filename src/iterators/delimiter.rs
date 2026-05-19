use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use pyo3::{prelude::*, types::PyBytes};

#[pyclass]
pub struct PReaderDelimiterIterator {
    reader: BufReader<File>,
    delimiter: u8,
    buffer: Vec<u8>,
    #[pyo3(get)]
    pub bytes_read: usize,
    #[pyo3(get)]
    pub total_bytes: usize,
}

impl PReaderDelimiterIterator {
    pub fn new(file: File, buffer_capacity: usize, delimiter: u8) -> std::io::Result<Self> {
        let total_bytes = file.metadata()?.len() as usize;
        Ok(Self {
            reader: BufReader::with_capacity(buffer_capacity, file),
            delimiter,
            buffer: Vec::new(),
            bytes_read: 0,
            total_bytes,
        })
    }

    fn read_segment(&mut self) -> std::io::Result<usize> {
        self.buffer.clear();
        let read_count = self.reader.read_until(self.delimiter, &mut self.buffer)?;
        self.bytes_read += read_count;
        Ok(read_count)
    }
}

#[pymethods]
impl PReaderDelimiterIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__<'py>(mut slf: PyRefMut<'py, Self>) -> PyResult<Option<Bound<'py, PyBytes>>> {
        match slf.read_segment()? {
            0 => Ok(None),
            _ => Ok(Some(PyBytes::new(slf.py(), &slf.buffer))),
        }
    }
}
