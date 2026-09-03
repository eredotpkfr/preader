use std::{
    fs::File,
    io::{BufReader, Bytes, Read, Result},
};

use pyo3::{prelude::*, types::PyBytes};

use crate::{
    State,
    iterators::base::IteratorBase,
    types::{config::iterator::IteratorConfig, options::IteratorOptions},
};

#[pyclass(module = "preader", extends = IteratorBase)]
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

        let window = opts.window(state.position, state.file.size, opts.skip);
        let reader = window.open(&state.file.path, config.buffer_capacity)?;

        state.position = window.position;

        let base = IteratorBase::new(config, state, window.end, opts.limit);
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

    fn __next__(mut slf: PyRefMut<'_, Self>) -> PyResult<Py<PyBytes>> {
        let py = slf.py();

        if slf.as_super().should_stop() {
            return Err(slf.as_super().stop());
        }

        let Some(byte) = slf.read_byte()? else {
            return Err(slf.as_super().stop());
        };

        let value = PyBytes::new(py, &[byte]).unbind();

        slf.as_super().count_yield();
        slf.as_super().advance(1)?;

        Ok(value)
    }

    fn __repr__(slf: PyRef<'_, Self>) -> String {
        crate::macros::pyrepr!("ByteIterator" {
            state = slf.as_super().state.__repr__(),
        })
    }
}
