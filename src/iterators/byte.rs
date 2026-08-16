use std::{
    fs::File,
    io::{BufReader, Bytes, Read, Result, Seek, SeekFrom},
};

use pyo3::{prelude::*, types::PyBytes};

use crate::{
    State,
    iterators::base::IteratorBase,
    types::{config::iterator::IteratorConfig, core::Item, options::IteratorOptions},
};

#[pyclass(extends = IteratorBase)]
pub struct ByteIterator {
    bytes: Bytes<BufReader<File>>,
}

impl ByteIterator {
    pub(crate) fn new(
        config: IteratorConfig,
        mut state: State,
        opts: IteratorOptions,
    ) -> PyResult<PyClassInitializer<Self>> {
        opts.validate()?;

        let end = opts.end.min(state.file.size);
        let initial_position = state.position.max(opts.start.saturating_add(opts.skip)).min(end);

        state.position = initial_position;

        let file = File::open(&state.file.path)?;
        let mut reader = BufReader::with_capacity(config.buffer_capacity, file);

        reader.seek(SeekFrom::Start(initial_position))?;

        let base = IteratorBase::new(config, state, end, opts.limit);
        let sub = Self {
            bytes: reader.bytes(),
        };

        Ok(PyClassInitializer::from(base).add_subclass(sub))
    }

    #[inline]
    fn read_byte(&mut self) -> Result<Option<u8>> {
        self.bytes.next().transpose()
    }
}

#[pymethods]
impl ByteIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> PyResult<Option<Item>> {
        let py = slf.py();

        if slf.as_super().should_stop() {
            slf.as_super().finalize()?;

            return Ok(None);
        }

        let Some(byte) = slf.read_byte()? else {
            slf.as_super().finalize()?;

            return Ok(None);
        };

        let value = PyBytes::new(py, &[byte]).unbind().into_any();

        slf.as_super().count_yield();
        slf.as_super().advance(1)?;

        Ok(Some(value))
    }

    fn __repr__(slf: PyRef<'_, Self>) -> String {
        crate::macros::pyrepr!("ByteIterator" {
            state = slf.as_super().state.__repr__(),
        })
    }
}
