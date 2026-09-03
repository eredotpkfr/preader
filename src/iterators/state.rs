use std::{
    fs,
    path::{Path, PathBuf},
};

use pyo3::{exceptions::PyStopIteration, prelude::*};
use regex::Regex;
use walkdir::{IntoIter, WalkDir};

use crate::{Error, utils::path::has_no_symlinks};

pub(crate) const STATE_FILE_EXT: &str = ".state.json";

#[pyclass(module = "preader")]
pub struct StateIterator {
    state_dir: PathBuf,
    entries: IntoIter,
    pattern: Option<Regex>,
}

impl StateIterator {
    pub(crate) fn new(dir: &Path, pattern: Option<&str>) -> Result<Self, Error> {
        fs::create_dir_all(dir)?;

        Ok(Self {
            state_dir: dir.to_path_buf(),
            entries: WalkDir::new(dir).min_depth(1).into_iter(),
            pattern: pattern.map(Regex::new).transpose()?,
        })
    }
}

impl Iterator for StateIterator {
    type Item = Result<String, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let (root, pattern) = (&self.state_dir, self.pattern.as_ref());

        self.entries.find_map(|entry| {
            let entry = match entry {
                Err(error) => return Some(Err(Error::Io(error.into()))),
                Ok(entry) => entry,
            };
            let path = entry.path();
            let name =
                path.strip_prefix(root).ok()?.to_str()?.strip_suffix(STATE_FILE_EXT)?.to_owned();

            (path.is_file()
                && has_no_symlinks(root, path)
                && pattern.is_none_or(|regex| regex.is_match(&name)))
            .then_some(Ok(name))
        })
    }
}

#[pymethods]
impl StateIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> PyResult<String> {
        slf.next().transpose()?.ok_or_else(|| PyStopIteration::new_err(()))
    }

    fn __repr__(&self) -> String {
        crate::macros::pyrepr!("StateIterator" {
            state_dir = format!("'{}'", self.state_dir.display()),
            pattern = self.pattern.as_ref().map_or_else(|| "None".to_owned(), |regex| format!("'{regex}'")),
        })
    }
}
