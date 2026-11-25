/// Builds a `__repr__` string such as `Config(state_dir=..., verify_state=True)`
///
/// It reads each listed field back through Python `getattr` and `repr`, so the output shows
/// the Python values and the runtime class name. It returns from the caller with the Python
/// error if an attribute or `repr` call fails
macro_rules! pyrepr {
    ($slf:expr, $($field:ident),* $(,)?) => {{
        let slf = $slf.as_any();
        let fields = [$(::std::format!(
            ::std::concat!(::std::stringify!($field), "={}"),
            slf.getattr(::std::stringify!($field))?.repr()?
        )),*];

        Ok(::std::format!("{}({})", slf.get_type().name()?, fields.join(", ")))
    }};
}

/// Makes `pyrepr!` importable by path inside the crate
pub(crate) use pyrepr;
