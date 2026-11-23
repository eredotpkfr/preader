macro_rules! pyiterator {
    (
        $name:ident($inner:ty) -> $output:ty,
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
            #[getter]
            fn state(&self) -> $crate::State {
                self.0.state.clone()
            }

            $($extra)*

            fn percent(&self) -> f64 {
                self.0.percent()
            }

            fn save(&mut self) -> $crate::Result<$crate::State> {
                self.0.save().cloned()
            }

            fn __iter__(slf: ::pyo3::PyRef<'_, Self>) -> ::pyo3::PyRef<'_, Self> {
                slf
            }

            fn __next__(mut slf: ::pyo3::PyRefMut<'_, Self>) -> $crate::Result<Option<$output>> {
                let $py = slf.py();

                Ok($crate::IteratorRead::read(&mut slf.0)?.map(|$item| $convert))
            }

            fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<String> {
                $crate::python::macros::repr::pyrepr!(slf, state, $($field),*)
            }
        }
    };
}

pub(crate) use pyiterator;
