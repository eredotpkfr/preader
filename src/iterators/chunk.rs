use std::{
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
};

use pyo3::{prelude::*, types::PyBytes};

use crate::{
    iterators::base::PReaderIteratorBase,
    types::{PReaderItem, PReaderState, config::PReaderIteratorConfig},
};

#[pyclass(extends = PReaderIteratorBase)]
pub struct PReaderChunkIterator {
    reader: BufReader<File>,
    buffer: Vec<u8>,
}

impl PReaderChunkIterator {
    pub(crate) fn new(
        config: PReaderIteratorConfig,
        state: PReaderState,
        chunk_size: usize,
    ) -> PyResult<PyClassInitializer<Self>> {
        let file = File::open(&state.file.path)?;
        let mut reader = BufReader::with_capacity(config.buffer_capacity, file);
        let buffer = vec![0u8; chunk_size];

        if state.position > 0 {
            reader.seek(SeekFrom::Start(state.position))?;
        }

        let base = PReaderIteratorBase::new(config, state);
        let sub = Self { reader, buffer };

        Ok(PyClassInitializer::from(base).add_subclass(sub))
    }

    #[inline]
    fn read_chunk(&mut self) -> std::io::Result<Option<&[u8]>> {
        let read_count = self.reader.read(&mut self.buffer)?;

        Ok((read_count > 0).then_some(&self.buffer[..read_count]))
    }
}

#[pymethods]
impl PReaderChunkIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> PyResult<Option<PReaderItem>> {
        let py = slf.py();

        let Some(chunk) = slf.read_chunk()? else {
            slf.as_super().finalize()?;

            return Ok(None);
        };

        let consumed = chunk.len() as u64;
        let value = PyBytes::new(py, chunk).unbind().into_any();

        slf.as_super().advance(consumed)?;

        Ok(Some(value))
    }
}
