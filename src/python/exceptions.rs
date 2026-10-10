use pyo3::{exceptions::PyException, prelude::*, types::PyTuple};

#[pyclass(module = "preader", extends = PyException, subclass)]
pub struct StateError;

#[pymethods]
impl StateError {
    #[new]
    #[pyo3(signature = (*_args: "object"))]
    fn py_new(_args: &Bound<'_, PyTuple>) -> Self {
        Self
    }
}

#[pyclass(module = "preader", extends = StateError, subclass)]
pub struct StateMismatchError;

#[pymethods]
impl StateMismatchError {
    #[new]
    #[pyo3(signature = (*_args: "object"))]
    fn py_new(_args: &Bound<'_, PyTuple>) -> PyClassInitializer<Self> {
        PyClassInitializer::from(StateError).add_subclass(Self)
    }
}
