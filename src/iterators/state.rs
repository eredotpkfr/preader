use std::{
    fs,
    path::{Path, PathBuf},
};

use regex::Regex;
use walkdir::{IntoIter, WalkDir};

use crate::{Error, Result, constants::STATE_FILE_EXTENSION, utils::path::has_no_symlinks};

#[derive(Debug)]
pub struct StateIterator {
    state_dir: PathBuf,
    entries: IntoIter,
    pattern: Option<Regex>,
}

impl StateIterator {
    pub(crate) fn new(dir: &Path, pattern: Option<&str>) -> Result<Self> {
        fs::create_dir_all(dir)?;

        Ok(Self {
            state_dir: dir.to_path_buf(),
            entries: WalkDir::new(dir).min_depth(1).into_iter(),
            pattern: pattern.map(Regex::new).transpose()?,
        })
    }
}

impl Iterator for StateIterator {
    type Item = Result<String>;

    fn next(&mut self) -> Option<Self::Item> {
        let (root, pattern) = (&self.state_dir, self.pattern.as_ref());

        self.entries.find_map(|entry| {
            let entry = match entry {
                Err(error) => return Some(Err(Error::Io(error.into()))),
                Ok(entry) => entry,
            };
            let path = entry.path();
            let file = path.strip_prefix(root).ok()?.to_str()?;
            let name = file.strip_suffix(STATE_FILE_EXTENSION)?.to_owned();

            (path.is_file()
                && has_no_symlinks(root, path)
                && pattern.is_none_or(|regex| regex.is_match(&name)))
            .then_some(Ok(name))
        })
    }
}
