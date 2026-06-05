use std::{fs, path::Path};

use pyo3::prelude::*;
use regex::Regex;

use crate::StateError;

pub(crate) const STATE_FILE_EXT: &str = ".state.json";

#[pyclass]
pub struct StateIterator {
    entries: fs::ReadDir,
    pattern: Option<Regex>,
}

impl StateIterator {
    pub(crate) fn new(dir: &Path, pattern: Option<&str>) -> PyResult<Self> {
        fs::create_dir_all(dir).map_err(StateError::from_io)?;

        let pattern = pattern.map(Regex::new).transpose().map_err(StateError::from_regex)?;
        let entries = fs::read_dir(dir).map_err(StateError::from_io)?;

        Ok(Self { entries, pattern })
    }
}

impl Iterator for StateIterator {
    type Item = PyResult<String>;

    fn next(&mut self) -> Option<PyResult<String>> {
        let pattern = self.pattern.as_ref();

        self.entries.find_map(|entry| match entry {
            Err(err) => Some(Err(StateError::from_io(err))),
            Ok(entry) => {
                let file_name = entry.file_name().to_string_lossy().into_owned();
                let name = file_name.strip_suffix(STATE_FILE_EXT)?.to_string();

                pattern.is_none_or(|re| re.is_match(&name)).then_some(Ok(name))
            }
        })
    }
}

#[pymethods]
impl StateIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> PyResult<Option<String>> {
        slf.next().transpose()
    }
}
