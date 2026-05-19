use std::{
    fs::File,
    io::{BufReader, Read},
};

use pyo3::{prelude::*, types::PyBytes};

#[pyclass]
pub struct PReaderChunkIterator {
    reader: BufReader<File>,
    buffer: Vec<u8>,
    #[pyo3(get)]
    pub bytes_read: usize,
    #[pyo3(get)]
    pub total_bytes: usize,
}

impl PReaderChunkIterator {
    pub fn new(file: File, buffer_capacity: usize, chunk_size: usize) -> std::io::Result<Self> {
        let total_bytes = file.metadata()?.len() as usize;
        Ok(Self {
            reader: BufReader::with_capacity(buffer_capacity, file),
            buffer: vec![0u8; chunk_size],
            bytes_read: 0,
            total_bytes,
        })
    }

    fn read_chunk(&mut self) -> std::io::Result<usize> {
        let read_count = self.reader.read(&mut self.buffer)?;
        self.bytes_read += read_count;
        Ok(read_count)
    }
}

#[pymethods]
impl PReaderChunkIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__<'py>(mut slf: PyRefMut<'py, Self>) -> PyResult<Option<Bound<'py, PyBytes>>> {
        match slf.read_chunk()? {
            0 => Ok(None),
            read_count => Ok(Some(PyBytes::new(slf.py(), &slf.buffer[..read_count]))),
        }
    }
}
