use pyo3::{exceptions::PyException, prelude::*};

use crate::python::macros::exception::pyexception;

/// Base of every preader state error; `name` is the state it concerns
///
/// It is raised itself when a part of the state name is a symlink or a case or Unicode alias
#[pyclass(module = "preader", extends = PyException, subclass)]
pub struct StateError {
    /// The message `str(error)` returns
    message: String,
    /// Name of the state the error concerns
    #[pyo3(get)]
    name: String,
}

#[pymethods]
impl StateError {
    /// Creates the error from a message and the state name, both optional and positional
    #[new]
    #[pyo3(signature = (message = String::new(), name = String::new(), /))]
    fn py_new(message: String, name: String) -> PyClassInitializer<Self> {
        Self { message, name }.into()
    }

    /// Returns the error message
    fn __str__(&self) -> &str {
        &self.message
    }
}

pyexception!(
    /// The file is no longer the tracked one, or a passed `State` has unsaved progress
    ///
    /// Start over under a new state, or pass the `State` that `save()` returned. It is also the
    /// base of `StateOutdatedError`
    StateMismatchError extends StateError
);
pyexception!(
    /// The tracked file moved, disappeared or changed
    ///
    /// Accept the change of a saved state with `reader.states.resync(error.name, file)`
    StateOutdatedError extends StateMismatchError
);
pyexception!(
    /// The state file does not parse, was edited outside preader, or belongs to another name
    ///
    /// Delete it with `reader.states.delete(error.name)` to start over
    StateCorruptError extends StateError
);
pyexception!(
    /// Another writer saved the state since this iterator loaded or last saved it
    ///
    /// The check starts once this iterator holds the saved state: an iterator that started fresh,
    /// or from a `State` other than the saved one, overwrites it on its first save. Open a new
    /// iterator to continue
    StateConflictError extends StateError
);
