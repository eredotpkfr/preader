use pyo3::prelude::*;

#[pyclass(eq, eq_int, from_py_object)]
#[derive(Clone, PartialEq)]
pub enum PReaderConfigFormat {
    JSON,
    TOML,
    YML,
}
