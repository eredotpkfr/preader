use std::{
    fs::File,
    io::{BufRead, BufReader, Result, Seek, SeekFrom},
};

use pyo3::{prelude::*, types::PyString};

use crate::{
    iterators::base::PReaderIteratorBase,
    types::{PReaderItem, PReaderState, config::PReaderIteratorConfig},
};

#[pyclass(extends = PReaderIteratorBase)]
pub struct PReaderLineIterator {
    reader: BufReader<File>,
    buffer: String,
    keepends: bool,
}

impl PReaderLineIterator {
    pub(crate) fn new(
        config: PReaderIteratorConfig,
        state: PReaderState,
        keepends: bool,
    ) -> PyResult<PyClassInitializer<Self>> {
        let file = File::open(&state.file.path)?;
        let mut reader = BufReader::with_capacity(config.buffer_capacity, file);
        let buffer = String::new();

        if state.position > 0 {
            reader.seek(SeekFrom::Start(state.position))?;
        }

        let base = PReaderIteratorBase::new(config, state);
        let sub = Self {
            reader,
            buffer,
            keepends,
        };

        Ok(PyClassInitializer::from(base).add_subclass(sub))
    }

    #[inline]
    fn read_line(&mut self) -> Result<Option<(&str, u64)>> {
        self.buffer.clear();

        let read_count = self.reader.read_line(&mut self.buffer)?;

        if read_count == 0 {
            return Ok(None);
        }

        let line = if self.keepends {
            self.buffer.as_str()
        } else {
            self.buffer.trim_end_matches('\n').trim_end_matches('\r')
        };

        Ok(Some((line, read_count as u64)))
    }
}

#[pymethods]
impl PReaderLineIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> PyResult<Option<PReaderItem>> {
        let py = slf.py();

        let Some((line, read_count)) = slf.read_line()? else {
            slf.as_super().finalize()?;

            return Ok(None);
        };

        let value = PyString::new(py, line).unbind().into_any();

        slf.as_super().advance(read_count)?;

        Ok(Some(value))
    }
}
