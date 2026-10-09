use std::{fs, path::MAIN_SEPARATOR};

use regex::Regex;
use walkdir::{IntoIter, WalkDir};

use crate::{
    Error, Result,
    constants::{NAME_SEPARATOR, STATE_FILE_EXTENSION},
    manager::StateManager,
};

#[cfg_attr(feature = "python", pyo3::pyclass(module = "preader"))]
#[derive(Debug)]
pub struct StateIterator {
    pub(crate) manager: StateManager,
    pub(crate) pattern: Option<Regex>,
    pub(crate) entries: IntoIter,
}

impl StateIterator {
    pub(crate) fn new(manager: &StateManager, pattern: Option<&str>) -> Result<Self> {
        fs::create_dir_all(&manager.state_dir)?;

        Ok(Self {
            manager: manager.clone(),
            pattern: pattern.map(Regex::new).transpose()?,
            entries: WalkDir::new(&manager.state_dir).min_depth(1).into_iter(),
        })
    }
}

impl Iterator for StateIterator {
    type Item = Result<String>;

    fn next(&mut self) -> Option<Self::Item> {
        let (manager, pattern) = (&self.manager, self.pattern.as_ref());

        self.entries.find_map(|entry| {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => return Some(Err(Error::Io(error.into()))),
            };
            let path = entry.path();
            let file = path
                .strip_prefix(&manager.state_dir)
                .ok()?
                .to_str()?
                .replace(MAIN_SEPARATOR, NAME_SEPARATOR);
            let name = file.strip_suffix(STATE_FILE_EXTENSION)?;

            (entry.file_type().is_file()
                && pattern.is_none_or(|regex| regex.is_match(name))
                && manager.path(name).is_ok_and(|located| located == path))
            .then(|| Ok(name.to_owned()))
        })
    }
}
