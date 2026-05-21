use std::{
    fs::File,
    io::{BufRead, BufReader, Lines, Result},
};

use pyo3::{prelude::*, types::PyString};

use crate::types::{PReaderItem, progress::ProgressState};

#[pyclass]
pub struct PReaderLineIterator {
    progress: ProgressState,
    lines: Lines<BufReader<File>>,
}

impl PReaderLineIterator {
    pub fn new(reader: BufReader<File>) -> Result<Self> {
        Ok(Self {
            progress: ProgressState::try_from(reader.get_ref())?,
            lines: reader.lines(),
        })
    }

    fn read_line(&mut self) -> Result<Option<String>> {
        self.lines.next().transpose()
    }
}

#[pymethods]
impl PReaderLineIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> PyResult<Option<PReaderItem>> {
        let py = slf.py();

        let Some(line) = slf.read_line()? else {
            return Ok(None);
        };

        let value = PyString::new(py, &line).unbind();
        let consumed = line.len() + 1;

        slf.progress.advance(consumed);

        Ok(Some((&slf.progress, value).into()))
    }
}
