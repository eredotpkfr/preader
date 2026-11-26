/// Declares a Python exception class `$name` that subclasses `$base` and forwards its docs
///
/// The class takes the same `(message, name)` constructor as
/// [`StateError`](crate::python::exceptions::StateError)
macro_rules! pyexception {
    ($(#[$doc:meta])* $name:ident extends $base:ident) => {
        $(#[$doc])*
        #[::pyo3::pyclass(module = "preader", extends = $base, subclass)]
        pub struct $name;

        #[::pyo3::pymethods]
        impl $name {
            /// Creates the error from a message and the state name, both optional and positional
            #[new]
            #[pyo3(signature = (message = String::new(), name = String::new(), /))]
            fn py_new(message: String, name: String) -> ::pyo3::PyClassInitializer<Self> {
                $base::py_new(message, name).add_subclass(Self)
            }
        }
    };
}

/// Makes `pyexception!` importable by path inside the crate
pub(crate) use pyexception;
