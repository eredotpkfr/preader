use std::{
    fs,
    io::{self, ErrorKind},
    path::MAIN_SEPARATOR,
};

use regex::Regex;
use walkdir::{IntoIter, WalkDir};

use crate::{
    Result, StateRegistry,
    constants::{NAME_SEPARATOR, STATE_FILE_EXTENSION},
    macros::ensure,
};

#[cfg_attr(feature = "python", pyo3::pyclass(module = "preader"))]
#[derive(Debug)]
pub struct StateIterator {
    pub(crate) registry: StateRegistry,
    pub(crate) pattern: Option<Regex>,
    pub(crate) entries: IntoIter,
}

impl StateIterator {
    pub(crate) fn new(registry: &StateRegistry, pattern: Option<&str>) -> Result<Self> {
        let root = &registry.state_dir;

        ensure!(
            fs::metadata(root).map_or(true, |metadata| metadata.is_dir()),
            io::Error::from(ErrorKind::NotADirectory)
        );

        Ok(Self {
            registry: registry.clone(),
            pattern: pattern.map(Regex::new).transpose()?,
            entries: WalkDir::new(root).min_depth(1).into_iter(),
        })
    }
}

impl Iterator for StateIterator {
    type Item = Result<String>;

    fn next(&mut self) -> Option<Self::Item> {
        let (registry, pattern) = (&self.registry, self.pattern.as_ref());
        let missing_root = |error: &walkdir::Error| {
            error.depth() == 0 && error.io_error().map(io::Error::kind) == Some(ErrorKind::NotFound)
        };

        self.entries.find_map(|entry| {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) if missing_root(&error) => return None,
                Err(error) => return Some(Err(error.into())),
            };
            let path = entry.path();
            let file = path
                .strip_prefix(&registry.state_dir)
                .ok()?
                .to_str()?
                .replace(MAIN_SEPARATOR, NAME_SEPARATOR);
            let name = file.strip_suffix(STATE_FILE_EXTENSION)?.strip_suffix('.')?;

            (entry.file_type().is_file()
                && pattern.is_none_or(|regex| regex.is_match(name))
                && registry.path(name).is_ok_and(|located| located == path))
            .then(|| Ok(name.to_owned()))
        })
    }
}
