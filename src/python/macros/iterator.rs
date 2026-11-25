/// Declares a Python iterator class `$name` that wraps the Rust iterator `$inner`
///
/// The class gets the shared `state`, `percent`, `save`, `__iter__`, `__next__` and `__repr__`
/// members. `$convert` turns each Rust item, bound to `$item` with the GIL token `$py`, into
/// `$output`. `$extra` adds per-class getters, the listed `$field`s are appended to the repr, and
/// leading doc comments become the class docs
macro_rules! pyiterator {
    (
        $(#[$doc:meta])*
        $name:ident($inner:ty) -> $output:ty,
        |$py:ident, $item:ident| $convert:expr,
        [$($field:ident),* $(,)?]
        $($extra:tt)*
    ) => {
        $(#[$doc])*
        #[::pyo3::pyclass(
            module = "preader",
            extends = $crate::python::bases::iterator::IteratorBase
        )]
        pub struct $name(pub(crate) $inner);

        #[::pyo3::pymethods]
        impl $name {
            /// Copy of the current state, positioned at the end of the last finished item
            ///
            /// An item counts as finished once the next one is requested, so a state saved right
            /// after reading an item resumes at that item. With `verify_state` on, passing a copy
            /// as `state` raises `StateMismatchError` when the position moved after the state was
            /// created, loaded or saved. Reading, `options.start` and a byte or chunk
            /// `options.skip` move the position, so pass the `State` that `save()` returned instead
            #[getter]
            fn state(&self) -> $crate::State {
                self.0.state.clone()
            }

            $($extra)*

            /// Returns the share of the file finished so far, from 0.0 to 100.0
            ///
            /// It counts finished items, so it stays below 100.0 while the last item is unfinished.
            /// It reaches 100.0 only when the read reaches the end of the file; `options.end`,
            /// `options.limit` or, for `ChunkIterator`, `drop_partial` stop it earlier
            fn percent(&self) -> f64 {
                self.0.percent()
            }

            /// Saves the current state to the reader's state directory and returns it
            ///
            /// The state holds finished items only, so unless the iteration has ended the last
            /// yielded item is read again on resume. It works whether or not `auto_save_state` is
            /// on, and a failed save does not stop the iteration. Raises `StateConflictError` when
            /// another writer saved the state since this iterator loaded or last saved it,
            /// `StateError` when the state name has become an alias, and `OSError` when the state
            /// file cannot be written. An iterator that did not start from the saved state
            /// overwrites it on its first save. A saved state that was deleted, cannot be read,
            /// does not parse or belongs to another name is always written over. A state is meant
            /// to have one writer at a time, so concurrent writers of one state must be coordinated
            /// by the caller
            fn save(&mut self) -> $crate::Result<$crate::State> {
                self.0.save().cloned()
            }

            /// Returns the iterator itself
            fn __iter__(slf: ::pyo3::PyRef<'_, Self>) -> ::pyo3::PyRef<'_, Self> {
                slf
            }

            /// Returns the next item and raises `StopIteration` once the read is finished
            ///
            /// Raises `OSError` when reading the file fails and, for `LineIterator`, `ValueError`
            /// when a line is not valid UTF-8; iteration can continue after either, except after an
            /// `OSError` that read nothing, which ends it. Like any item, a failed one is finished
            /// only once the next item is requested, so a state saved right after the error resumes
            /// at it and raises it again. An invalid line also counts toward `options.limit`. When
            /// `auto_save_state` is on, a failed autosave raises the errors of `save()` and ends
            /// the iteration
            fn __next__(
                &mut self,
                $py: ::pyo3::Python<'_>,
            ) -> $crate::Result<Option<$output>> {
                Ok($crate::IteratorRead::read(&mut self.0)?.map(|$item| $convert))
            }

            /// Returns the class name with the state and the iterator settings
            fn __repr__(slf: &::pyo3::Bound<'_, Self>) -> ::pyo3::PyResult<String> {
                $crate::python::macros::repr::pyrepr!(slf, state, $($field),*)
            }
        }
    };
}

/// Makes `pyiterator!` importable by path inside the crate
pub(crate) use pyiterator;
