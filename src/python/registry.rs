use std::path::{Path, PathBuf};

use pyo3::prelude::*;

use crate::{Result, State, StateIterator, StateRegistry, python::macros::repr::pyrepr};

#[pymethods]
impl StateRegistry {
    /// Directory that holds the state files
    #[getter]
    #[pyo3(name = "state_dir")]
    fn py_state_dir(&self) -> &Path {
        self.state_dir()
    }

    /// Returns an iterator over the names of every saved state, in no particular order
    ///
    /// Names use `/` between directories on every platform, and a missing state directory lists
    /// nothing. Entries that are not valid state names, such as temporary files and symlinks, are
    /// skipped. Raises `OSError` when the state directory path is not a directory
    #[pyo3(name = "names")]
    fn py_names(&self) -> Result<StateIterator> {
        self.names()
    }

    /// Returns an iterator over the saved state names that match the regular expression `pattern`
    ///
    /// The match is unanchored, so `pattern` may match any part of a name; anchor it with `^` and
    /// `$` to match whole names. The syntax is that of the Rust `regex` crate, without lookaround
    /// or backreferences. Raises `ValueError` for an invalid pattern and `OSError` when the state
    /// directory path is not a directory
    #[pyo3(name = "search")]
    fn py_search(&self, pattern: &str) -> Result<StateIterator> {
        self.search(pattern)
    }

    /// Returns the state saved under `name`, checked against its file when `verify_state` is on
    ///
    /// Raises `KeyError` when nothing is saved under `name`, `ValueError` for an invalid name,
    /// `StateError` for an aliased name, `StateCorruptError` when the state file does not parse
    /// or belongs to another name, and `OSError` when a file cannot be read. With `verify_state`
    /// on, it also raises `StateCorruptError` for a state file edited outside preader,
    /// `StateOutdatedError` when the tracked file is missing or changed in size or mtime, and
    /// `StateMismatchError` when it is another file
    #[pyo3(name = "load")]
    fn py_load(&self, name: &str) -> Result<State> {
        self.load(name)
    }

    /// Returns the state saved under `name`, or `None` when it cannot be loaded
    ///
    /// Every error `load()` would raise becomes `None`, not only a missing state
    #[pyo3(name = "find")]
    fn py_find(&self, name: &str) -> Option<State> {
        self.find(name)
    }

    /// Returns every saved state as a list
    ///
    /// The first state that fails to load aborts the call and raises the same errors as `load()`.
    /// Raises `OSError` when the state directory path is not a directory
    #[pyo3(name = "all")]
    fn py_all(&self) -> Result<Vec<State>> {
        self.all()
    }

    /// Returns whether a state file is saved under `name`, without reading or checking it
    ///
    /// An invalid or aliased name returns `False` instead of raising
    #[pyo3(name = "exists")]
    fn py_exists(&self, name: &str) -> bool {
        self.exists(name)
    }

    /// Returns the path of the state file for `name`, whether or not it exists
    ///
    /// Raises `ValueError` for an invalid name and `StateError` when a part of the name is a
    /// symlink or a case or Unicode alias
    #[pyo3(name = "path")]
    fn py_path(&self, name: &str) -> Result<PathBuf> {
        self.path(name)
    }

    /// Accepts a moved or changed file for the state saved under `name`, saves it and returns it
    ///
    /// Use it after a `StateOutdatedError`, as `reader.states.resync(error.name, file)`. The state
    /// keeps its name and its position, clamped to the new file size. It records the canonical
    /// `path`, so after a move an auto-named state resumes only when its name is passed as `state`.
    /// An iterator still holding the old state raises `StateConflictError` on its next save.
    /// Raises `KeyError`, `ValueError`, `StateError` and `StateCorruptError` as `load()` does for
    /// the saved state, `StateMismatchError` when `verify_state` finds that `path` holds another
    /// file, and `OSError` when `path` cannot be read or the state cannot be written
    #[pyo3(name = "resync")]
    fn py_resync(&self, name: &str, path: PathBuf) -> Result<State> {
        self.resync(name, path)
    }

    /// Deletes the state file saved under `name` without reading it
    ///
    /// An iterator that still holds the state writes it again on its next save, manual or
    /// autosave. Raises `KeyError` when nothing is saved under `name`, `ValueError` for an invalid
    /// name, `StateError` for an aliased name, and `OSError` when the file cannot be removed
    #[pyo3(name = "delete")]
    fn py_delete(&self, name: &str) -> Result<()> {
        self.delete(name)
    }

    /// Deletes every saved state, including corrupt ones
    ///
    /// It stops at the first failure, so some states may already be deleted when it raises
    /// `OSError`. An iterator that still holds a state writes it again on its next save, manual or
    /// autosave
    #[pyo3(name = "clear")]
    fn py_clear(&self) -> Result<()> {
        self.clear()
    }

    /// Returns whether a state is saved under `name`, like `exists(name)`
    fn __contains__(&self, name: &str) -> bool {
        self.exists(name)
    }

    /// Deletes the state saved under `name`, like `delete(name)`
    fn __delitem__(&self, name: &str) -> Result<()> {
        self.delete(name)
    }

    /// Returns the state saved under `name`, like `load(name)`
    fn __getitem__(&self, name: &str) -> Result<State> {
        self.load(name)
    }

    /// Returns an iterator over the saved state names, like `names()`
    fn __iter__(&self) -> Result<StateIterator> {
        self.names()
    }

    /// Returns the number of saved states, without reading them
    ///
    /// Raises `OSError` when the state directory cannot be listed
    fn __len__(&self) -> Result<usize> {
        self.count()
    }

    /// Returns the class name with the state directory
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        pyrepr!(slf, state_dir)
    }
}
