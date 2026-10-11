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

pub(crate) use pyrepr;
