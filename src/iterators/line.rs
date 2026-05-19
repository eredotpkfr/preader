use std::{
    fs::File,
    io::{BufRead, BufReader, Lines},
};

use pyo3::{prelude::*, types::PyString};

#[pyclass]
pub struct PReaderLineIterator {
    lines: Lines<BufReader<File>>,
    #[pyo3(get)]
    pub bytes_read: usize,
    #[pyo3(get)]
    pub total_bytes: usize,
}

impl PReaderLineIterator {
    pub fn new(file: File, buffer_capacity: usize) -> std::io::Result<Self> {
        let total_bytes = file.metadata()?.len() as usize;
        Ok(Self {
            lines: BufReader::with_capacity(buffer_capacity, file).lines(),
            bytes_read: 0,
            total_bytes,
        })
    }

    fn read_line(&mut self) -> std::io::Result<Option<String>> {
        let Some(line) = self.lines.next().transpose()? else {
            return Ok(None);
        };
        self.bytes_read += line.len() + 1;
        Ok(Some(line))
    }
}

#[pymethods]
impl PReaderLineIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__<'py>(mut slf: PyRefMut<'py, Self>) -> PyResult<Option<Bound<'py, PyString>>> {
        Ok(slf.read_line()?.map(|line| PyString::new(slf.py(), &line)))
    }
}
