use pyo3::{exceptions::PyException, prelude::*, types::PyTuple};

#[pyclass(module = "preader", extends = PyException, subclass)]
pub struct StateError;

#[pymethods]
impl StateError {
    #[new]
    #[pyo3(signature = (*_args))]
    fn new(_args: &Bound<'_, PyTuple>) -> Self {
        Self
    }
}
