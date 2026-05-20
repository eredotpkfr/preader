use std::{
    fs::File,
    io::{BufReader, Read},
};

use pyo3::{prelude::*, types::PyBytes};

use crate::iterators::base::PReaderBaseFileIterator;

#[pyclass(extends = PReaderBaseFileIterator)]
pub struct PReaderChunkIterator {
    reader: BufReader<File>,
    buffer: Vec<u8>,
}

impl PReaderChunkIterator {
    pub fn new(
        reader: BufReader<File>,
        chunk_size: usize,
    ) -> std::io::Result<PyClassInitializer<Self>> {
        let base = PReaderBaseFileIterator::try_from(reader.get_ref())?;
        let this = Self {
            reader,
            buffer: vec![0u8; chunk_size],
        };
        let class = PyClassInitializer::from(base).add_subclass(this);

        Ok(class)
    }

    fn read_chunk(&mut self) -> std::io::Result<Option<usize>> {
        let read_count = self.reader.read(&mut self.buffer)?;
        Ok((read_count > 0).then_some(read_count))
    }
}

#[pymethods]
impl PReaderChunkIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__<'py>(mut slf: PyRefMut<'py, Self>) -> PyResult<Option<Bound<'py, PyBytes>>> {
        let Some(read_count) = slf.read_chunk()? else {
            return Ok(None);
        };
        slf.as_super().bytes_read += read_count;

        Ok(Some(PyBytes::new(slf.py(), &slf.buffer[..read_count])))
    }
}
