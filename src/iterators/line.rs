use std::{
    fs::File,
    io::{BufRead, BufReader, Read, Result, Seek, SeekFrom},
};

use pyo3::{prelude::*, types::PyString};

use crate::{
    iterators::base::IteratorBase,
    types::{Item, IteratorOptions, State, config::IteratorConfig},
};

#[pyclass(extends = IteratorBase)]
pub struct LineIterator {
    reader: BufReader<File>,
    buffer: String,
    keepends: bool,
    skip_empty: bool,
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

        let end = opts.end.min(state.file.size);

        let (initial_position, fresh_path) = if state.position >= end {
            (end, false)
        } else if state.position > opts.start {
            (state.position, false)
        } else {
            (opts.start.min(end), true)
        };

        let skip_remaining = if fresh_path {
            let align_extra = if align_start && initial_position > 0 {
                peek_needs_align(&state.file.path, initial_position, b'\n')?
            } else {
                0
            };

            opts.skip.saturating_add(align_extra)
        } else {
            0
        };

        state.position = initial_position;

        let file = File::open(&state.file.path)?;
        let mut reader = BufReader::with_capacity(config.buffer_capacity, file);
        reader.seek(SeekFrom::Start(initial_position))?;

        let base = IteratorBase::new(config, state, end, opts.limit);
        let sub = Self {
            reader,
            buffer: String::new(),
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

fn peek_needs_align(path: &std::path::Path, position: u64, boundary_byte: u8) -> Result<u64> {
    let mut peeker = File::open(path)?;

    peeker.seek(SeekFrom::Start(position - 1))?;

    let mut peek = [0u8; 1];
    let read_count = peeker.read(&mut peek)?;

    Ok(if read_count == 1 && peek[0] != boundary_byte {
        1
    } else {
        0
    })
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
}
