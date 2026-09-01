use std::{
    fs::File,
    io::{BufRead, BufReader, Result},
};

use pyo3::{prelude::*, types::PyString};

use crate::{
    iterators::base::IteratorBase,
    types::{config::iterator::IteratorConfig, core::Item, options::IteratorOptions, state::State},
    utils::file::starts_mid_item,
};

#[pyclass(module = "preader", extends = IteratorBase)]
pub struct LineIterator {
    reader: BufReader<File>,
    buffer: String,
    #[pyo3(get)]
    keepends: bool,
    #[pyo3(get)]
    skip_empty: bool,
    #[pyo3(get)]
    skip_remaining: u64,
}

impl LineIterator {
    pub(crate) fn new(
        config: IteratorConfig,
        mut state: State,
        keepends: bool,
        opts: IteratorOptions,
        align_start: bool,
        skip_empty: bool,
    ) -> PyResult<PyClassInitializer<Self>> {
        opts.validate()?;

        let window = opts.window(state.position, state.file.size, 0);
        let reader = window.open(&state.file.path, config.buffer_capacity)?;

        let skip_remaining = if window.from_start {
            opts.skip.saturating_add(u64::from(
                align_start && starts_mid_item(reader.get_ref(), window.position, b'\n')?,
            ))
        } else {
            0
        };

        state.position = window.position;

        let buffer = String::new();
        let base = IteratorBase::new(config, state, window.end, opts.limit);

        let sub = Self {
            reader,
            buffer,
            keepends,
            skip_empty,
            skip_remaining,
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
impl LineIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> PyResult<Option<Item>> {
        let py = slf.py();

        loop {
            if slf.as_super().should_stop() {
                slf.as_super().finalize()?;

                return Ok(None);
            }

            if slf.skip_remaining > 0 {
                let Some((_, read_count)) = slf.read_line()? else {
                    slf.as_super().finalize()?;

                    return Ok(None);
                };

                slf.skip_remaining -= 1;
                slf.as_super().advance(read_count)?;

                continue;
            }

            let keepends = slf.keepends;
            let skip_empty = slf.skip_empty;

            let Some((line, read_count)) = slf.read_line()? else {
                slf.as_super().finalize()?;

                return Ok(None);
            };

            let is_blank = if keepends {
                line.trim_end_matches('\n').trim_end_matches('\r').is_empty()
            } else {
                line.is_empty()
            };

            if skip_empty && is_blank {
                slf.as_super().advance(read_count)?;

                continue;
            }

            let value = PyString::new(py, line).unbind().into_any();

            slf.as_super().count_yield();
            slf.as_super().advance(read_count)?;

            return Ok(Some(value));
        }
    }

    fn __repr__(slf: PyRef<'_, Self>) -> String {
        crate::macros::pyrepr!("LineIterator" {
            state = slf.as_super().state.__repr__(),
            keepends = slf.keepends,
            skip_empty = slf.skip_empty,
            skip_remaining = slf.skip_remaining,
        })
    }
}
