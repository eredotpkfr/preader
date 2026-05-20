use std::{
    fs::File,
    io::{BufReader, Bytes, Read},
};

use pyo3::{prelude::*, types::PyBytes};

use crate::iterators::base::PReaderBaseFileIterator;

#[pyclass(extends = PReaderBaseFileIterator)]
pub struct PReaderByteIterator {
    bytes: Bytes<BufReader<File>>,
}

impl From<BufReader<File>> for PReaderByteIterator {
    fn from(reader: BufReader<File>) -> Self {
        Self {
            bytes: reader.bytes(),
        }
    }
}

impl PReaderByteIterator {
    pub fn new(reader: BufReader<File>) -> std::io::Result<PyClassInitializer<Self>> {
        let base = PReaderBaseFileIterator::try_from(reader.get_ref())?;
        let class = PyClassInitializer::from(base).add_subclass(reader.into());

        Ok(class)
    }

    fn read_byte(&mut self) -> std::io::Result<Option<u8>> {
        self.bytes.next().transpose()
    }
}

#[pymethods]
impl PReaderByteIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__<'py>(mut slf: PyRefMut<'py, Self>) -> PyResult<Option<Bound<'py, PyBytes>>> {
        let Some(byte) = slf.read_byte()? else {
            return Ok(None);
        };
        slf.as_super().bytes_read += 1;

        Ok(Some(PyBytes::new(slf.py(), &[byte])))
    }
}
