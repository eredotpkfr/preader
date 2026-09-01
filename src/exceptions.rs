use pyo3::{create_exception, exceptions::PyException};

create_exception!(preader, StateError, PyException);
