use std::{
    fs::File,
    io::{BufRead, BufReader, Lines},
};

use pyo3::{prelude::*, types::PyString};

use crate::iterators::base::PReaderBaseFileIterator;

#[pyclass(extends = PReaderBaseFileIterator)]
pub struct PReaderLineIterator {
    lines: Lines<BufReader<File>>,
}

impl From<BufReader<File>> for PReaderLineIterator {
    fn from(reader: BufReader<File>) -> Self {
        Self {
            lines: reader.lines(),
        }
    }
}

impl PReaderLineIterator {
    pub fn new(reader: BufReader<File>) -> std::io::Result<PyClassInitializer<Self>> {
        let base = PReaderBaseFileIterator::try_from(reader.get_ref())?;
        let class = PyClassInitializer::from(base).add_subclass(reader.into());

        Ok(class)
    }

    fn read_line(&mut self) -> std::io::Result<Option<String>> {
        self.lines.next().transpose()
    }
}

#[pymethods]
impl PReaderLineIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__<'py>(mut slf: PyRefMut<'py, Self>) -> PyResult<Option<Bound<'py, PyString>>> {
        let Some(line) = slf.read_line()? else {
            return Ok(None);
        };
        slf.as_super().bytes_read += line.len() + 1;

        Ok(Some(PyString::new(slf.py(), &line)))
    }
}
