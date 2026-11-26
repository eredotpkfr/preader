use pyo3::{Borrowed, exceptions::PyTypeError, prelude::*};
#[cfg(feature = "experimental-inspect")]
use pyo3::{
    PyTypeInfo,
    inspect::PyStaticExpr,
    type_hint_union,
    types::{PyNone, PyString},
};

use crate::{State, StateSource};

/// Extracts the `state=` argument of the [`PReader`](crate::PReader) iterator methods
impl<'a, 'py> FromPyObject<'a, 'py> for StateSource {
    /// The Python exception raised for an unsupported argument type
    type Error = PyErr;

    /// The type hint `State | str | None` shown in the generated stubs
    #[cfg(feature = "experimental-inspect")]
    const INPUT_TYPE: PyStaticExpr =
        type_hint_union!(State::TYPE_HINT, PyString::TYPE_HINT, PyNone::TYPE_HINT);

    /// Maps `None` to [`Auto`](Self::Auto), a [`State`] to [`Existing`](Self::Existing) and a `str`
    /// to [`Named`](Self::Named)
    ///
    /// Raises `TypeError` for any other type
    fn extract(object: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if object.is_none() {
            return Ok(Self::Auto);
        }

        if let Ok(state) = object.extract::<State>() {
            return Ok(Self::from(state));
        }

        if let Ok(name) = object.extract::<String>() {
            return Ok(Self::from(name));
        }

        Err(PyTypeError::new_err(
            "state must be None, a name (str), or a preader.State object",
        ))
    }
}
