use std::{
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
};

use pyo3::{exceptions::PyValueError, prelude::*, types::PyBytes};

use crate::{
    iterators::base::IteratorBase,
    types::{Item, IteratorOptions, State, config::IteratorConfig},
};

#[pyclass(extends = IteratorBase)]
pub struct ChunkIterator {
    reader: BufReader<File>,
    buffer: Vec<u8>,
    drop_partial: bool,
}

impl ChunkIterator {
    pub(crate) fn new(
        config: IteratorConfig,
        mut state: State,
        chunk_size: usize,
        opts: IteratorOptions,
        drop_partial: bool,
    ) -> PyResult<PyClassInitializer<Self>> {
        if opts.start > opts.end {
            return Err(PyValueError::new_err(format!(
                "start ({}) must be <= end ({})",
                opts.start, opts.end
            )));
        }

        let end = opts.end.min(state.file.size);
        let skip_bytes = opts.skip.saturating_mul(chunk_size as u64);
        let initial_position = state.position.max(opts.start.saturating_add(skip_bytes)).min(end);

        state.position = initial_position;

        let file = File::open(&state.file.path)?;
        let mut reader = BufReader::with_capacity(config.buffer_capacity, file);
        reader.seek(SeekFrom::Start(initial_position))?;

        let buffer = vec![0u8; chunk_size];
        let base = IteratorBase::new(config, state, end, opts.limit);
        let sub = Self {
            reader,
            buffer,
            drop_partial,
        };

        Ok(PyClassInitializer::from(base).add_subclass(sub))
    }

    #[inline]
    fn read_chunk(&mut self, max_bytes: usize) -> std::io::Result<Option<&[u8]>> {
        let read_count = self.reader.read(&mut self.buffer[..max_bytes])?;

        Ok((read_count > 0).then_some(&self.buffer[..read_count]))
    }
}

#[pymethods]
impl ChunkIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> PyResult<Option<Item>> {
        let py = slf.py();

        if slf.as_super().should_stop() {
            slf.as_super().finalize()?;

            return Ok(None);
        }

        let chunk_size = slf.buffer.len();
        let position = slf.as_super().state.position;
        let end = slf.as_super().end;
        let max_bytes = ((end - position) as usize).min(chunk_size);

        let drop_partial = slf.drop_partial;
        if drop_partial && max_bytes < chunk_size {
            slf.as_super().finalize()?;

            return Ok(None);
        }

        let Some(chunk) = slf.read_chunk(max_bytes)? else {
            slf.as_super().finalize()?;

            return Ok(None);
        };

        let chunk_len = chunk.len();

        if drop_partial && chunk_len < chunk_size {
            slf.as_super().advance(chunk_len as u64)?;
            slf.as_super().finalize()?;

            return Ok(None);
        }

        let value = PyBytes::new(py, chunk).unbind().into_any();

        slf.as_super().count_yield();
        slf.as_super().advance(chunk_len as u64)?;

        Ok(Some(value))
    }
}
