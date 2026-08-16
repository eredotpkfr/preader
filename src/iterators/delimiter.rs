use std::{
    fs::File,
    io::{BufRead, BufReader, Read, Result, Seek, SeekFrom},
};

use pyo3::{prelude::*, types::PyBytes};

use crate::{
    iterators::base::IteratorBase,
    types::{config::iterator::IteratorConfig, core::Item, options::IteratorOptions, state::State},
};

#[pyclass(extends = IteratorBase)]
pub struct DelimiterIterator {
    reader: BufReader<File>,
    buffer: Vec<u8>,
    #[pyo3(get)]
    delimiter: u8,
    #[pyo3(get)]
    keep_delimiter: bool,
    #[pyo3(get)]
    skip_empty: bool,
    #[pyo3(get)]
    skip_remaining: u64,
}

impl DelimiterIterator {
    pub(crate) fn new(
        config: IteratorConfig,
        mut state: State,
        delimiter: u8,
        keep_delimiter: bool,
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
                peek_needs_align(&state.file.path, initial_position, delimiter)?
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
            buffer: Vec::new(),
            delimiter,
            keep_delimiter,
            skip_empty,
            skip_remaining,
        };

        Ok(PyClassInitializer::from(base).add_subclass(sub))
    }

    #[inline]
    fn read_segment(&mut self) -> Result<Option<(&[u8], u64)>> {
        self.buffer.clear();

        let read_count = self.reader.read_until(self.delimiter, &mut self.buffer)?;

        if read_count == 0 {
            return Ok(None);
        }

        let slice = if self.keep_delimiter {
            self.buffer.as_slice()
        } else {
            self.buffer.strip_suffix(&[self.delimiter]).unwrap_or(&self.buffer)
        };

        Ok(Some((slice, read_count as u64)))
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
impl DelimiterIterator {
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
                let Some((_, read_count)) = slf.read_segment()? else {
                    slf.as_super().finalize()?;

                    return Ok(None);
                };

                slf.skip_remaining -= 1;
                slf.as_super().advance(read_count)?;

                continue;
            }

            let delimiter_byte = slf.delimiter;
            let keep_delim = slf.keep_delimiter;
            let skip_empty = slf.skip_empty;

            let Some((segment, read_count)) = slf.read_segment()? else {
                slf.as_super().finalize()?;

                return Ok(None);
            };

            let is_blank = if keep_delim {
                segment.strip_suffix(&[delimiter_byte]).unwrap_or(segment).is_empty()
            } else {
                segment.is_empty()
            };

            if skip_empty && is_blank {
                slf.as_super().advance(read_count)?;

                continue;
            }

            let value = PyBytes::new(py, segment).unbind().into_any();

            slf.as_super().count_yield();
            slf.as_super().advance(read_count)?;

            return Ok(Some(value));
        }
    }

    fn __repr__(slf: PyRef<'_, Self>) -> String {
        crate::macros::pyrepr!("DelimiterIterator" {
            state = slf.as_super().state.__repr__(),
            delimiter = slf.delimiter,
            keep_delimiter = slf.keep_delimiter,
            skip_empty = slf.skip_empty,
            skip_remaining = slf.skip_remaining,
        })
    }
}
