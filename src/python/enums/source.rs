use pyo3::{Borrowed, exceptions::PyTypeError, prelude::*};
#[cfg(feature = "experimental-inspect")]
use pyo3::{
    PyTypeInfo,
    inspect::PyStaticExpr,
    type_hint_union,
    types::{PyNone, PyString},
};

use crate::{State, StateSource};

impl<'a, 'py> FromPyObject<'a, 'py> for StateSource {
    type Error = PyErr;

    #[cfg(feature = "experimental-inspect")]
    const INPUT_TYPE: PyStaticExpr =
        type_hint_union!(State::TYPE_HINT, PyString::TYPE_HINT, PyNone::TYPE_HINT);

    fn extract(object: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if object.is_none() {
            return Ok(Self::Auto);
        }

        if let Ok(state) = object.extract::<State>() {
            return Ok(Self::from(state));
        }

        object.extract::<String>().map(Self::from).map_err(|_| {
            PyTypeError::new_err("state must be None, a name (str), or a preader.State object")
        })
    }
}
