use pyo3::{exceptions::PyException, prelude::*};

use crate::python::macros::exception::pyexception;

/// Base of every preader state error; `name` is the state it concerns.
#[pyclass(module = "preader", extends = PyException, subclass)]
pub struct StateError {
    message: String,
    #[pyo3(get)]
    name: String,
}

#[pymethods]
impl StateError {
    #[new]
    #[pyo3(signature = (message = String::new(), name = String::new(), /))]
    fn py_new(message: String, name: String) -> PyClassInitializer<Self> {
        Self { message, name }.into()
    }

    fn __str__(&self) -> &str {
        &self.message
    }
}

pyexception!(
    /// The state no longer matches its file or itself; start over, or pass the saved state.
    StateMismatchError extends StateError
);
pyexception!(
    /// The tracked file moved or changed; `reader.states.resync(error.name, file)` accepts it.
    StateOutdatedError extends StateMismatchError
);
pyexception!(
    /// The state file is unreadable, edited, or belongs to another name; delete it to start over.
    StateCorruptError extends StateError
);
pyexception!(
    /// Another writer saved the state since this iterator read it; open a new iterator.
    StateConflictError extends StateError
);
