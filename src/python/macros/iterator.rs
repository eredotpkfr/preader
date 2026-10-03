macro_rules! pyiterator {
    (
        $name:ident($inner:ty) as $label:literal -> $output:ty,
        |$py:ident, $item:ident| $convert:expr,
        [$($field:ident),* $(,)?]
        $($extra:tt)*
    ) => {
        #[::pyo3::pyclass(
            module = "preader",
            extends = $crate::python::bases::iterator::IteratorBase
        )]
        pub struct $name(pub(crate) $inner);

        #[::pyo3::pymethods]
        impl $name {
            fn percent(&mut self) -> f64 {
                self.0.state().percent()
            }

            #[getter]
            fn state(&mut self) -> $crate::State {
                self.0.state().clone()
            }

            fn __iter__(slf: ::pyo3::PyRef<'_, Self>) -> ::pyo3::PyRef<'_, Self> {
                slf
            }

            fn __next__(mut slf: ::pyo3::PyRefMut<'_, Self>) -> ::pyo3::PyResult<$output> {
                let $py = slf.py();
                let Some($item) = $crate::IteratorRead::read(&mut slf.0)? else {
                    return Err(::pyo3::exceptions::PyStopIteration::new_err(()));
                };

                Ok($convert)
            }

            fn __repr__(&mut self) -> String {
                $crate::python::macros::repr::pyrepr!($label {
                    state = self.state().__repr__(),
                    $($field = self.$field()),*
                })
            }

            $($extra)*
        }
    };
}

pub(crate) use pyiterator;
