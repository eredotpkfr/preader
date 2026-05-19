use std::{
    fs::File,
    io::{BufReader, Bytes, Read},
};

use pyo3::{prelude::*, types::PyBytes};

#[pyclass]
pub struct PReaderByteIterator {
    bytes: Bytes<BufReader<File>>,
    #[pyo3(get)]
    pub bytes_read: usize,
    #[pyo3(get)]
    pub total_bytes: usize,
}

impl PReaderByteIterator {
    pub fn new(file: File, buffer_capacity: usize) -> std::io::Result<Self> {
        let total_bytes = file.metadata()?.len() as usize;
        Ok(Self {
            bytes: BufReader::with_capacity(buffer_capacity, file).bytes(),
            bytes_read: 0,
            total_bytes,
        })
    }

    fn read_byte(&mut self) -> std::io::Result<Option<u8>> {
        let Some(byte) = self.bytes.next().transpose()? else {
            return Ok(None);
        };
        self.bytes_read += 1;
        Ok(Some(byte))
    }
}

#[pymethods]
impl PReaderByteIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__<'py>(mut slf: PyRefMut<'py, Self>) -> PyResult<Option<Bound<'py, PyBytes>>> {
        Ok(slf.read_byte()?.map(|byte| PyBytes::new(slf.py(), &[byte])))
    }
}
