macro_rules! pyexception {
    ($(#[$doc:meta])* $name:ident extends $base:ident) => {
        $(#[$doc])*
        #[::pyo3::pyclass(module = "preader", extends = $base, subclass)]
        pub struct $name;

        #[::pyo3::pymethods]
        impl $name {
            #[new]
            #[pyo3(signature = (message = String::new(), name = String::new(), /))]
            fn py_new(message: String, name: String) -> ::pyo3::PyClassInitializer<Self> {
                $base::py_new(message, name).add_subclass(Self)
            }
        }
    };
}

pub(crate) use pyexception;
