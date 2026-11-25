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

/// Iterator over the names of the states saved in a state directory
///
/// The `names` and `search` methods of [`StateRegistry`] return it. Each name uses `/` as
/// separator, the order is unspecified, and a part of the directory that cannot be read gives
/// an error
#[cfg_attr(feature = "python", pyo3::pyclass(module = "preader"))]
#[derive(Debug)]
pub struct StateIterator {
    /// Registry that owns the walked state directory and maps each name back to its path
    pub(crate) registry: StateRegistry,
    /// Expression a name must match to be yielded, if any
    pub(crate) pattern: Option<Regex>,
    /// Recursive walk over the state directory, following no symlink below its root
    pub(crate) entries: IntoIter,
}

impl StateIterator {
    /// Creates an iterator over the state names of `registry`, filtered by `pattern` if given
    ///
    /// A state directory that does not exist yet is accepted and yields no names
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`](crate::Error::Io) with [`ErrorKind::NotADirectory`] when the state
    /// directory exists but is not a directory, and [`Error::Regex`](crate::Error::Regex) when
    /// `pattern` is not a valid regular expression
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
    /// A state name, or the error met while walking the state directory
    type Item = Result<String>;

    /// Returns the next saved state name
    ///
    /// Only regular files with the [`STATE_FILE_EXTENSION`] extension whose name maps back to the
    /// same path and matches the pattern, if any, are yielded. Temporary files, symlinks, invalid
    /// names and non-UTF-8 paths are skipped silently, and a missing state directory ends the
    /// iteration at once
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
    ///
    /// let dir = tempfile::tempdir()?;
    /// let file = dir.path().join("data.txt");
    ///
    /// let reader = PReader::from(Config {
    ///     state_dir: dir.path().join("states"),
    ///     ..Config::default()
    /// });
    ///
    /// std::fs::write(&file, "foo\n")?;
    ///
    /// reader.lines(&file).state("job-1").build()?.save()?;
    ///
    /// let first = reader.states().names()?.next().transpose()?;
    ///
    /// assert_eq!(first.as_deref(), Some("job-1"));
    /// # Ok::<(), preader::Error>(())
    /// ```
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
