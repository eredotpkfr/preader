use pyo3::{Py, prelude::*, types::PyAny};

use crate::types::state::PReaderState;

pub type PReaderItem = Py<PyAny>;
pub(crate) type AutoSaveStateFn = fn(&mut PReaderState, u64) -> PyResult<()>;
