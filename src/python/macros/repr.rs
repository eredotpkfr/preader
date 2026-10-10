macro_rules! pyrepr {
    ($name:literal { $($field:ident = $value:expr),* $(,)? }) => {{
        let fields = [$(format!(
            "  {}={}",
            stringify!($field),
            $crate::python::utils::text::nest(&$value.to_string())
        )),*];

        format!("{}(\n{}\n)", $name, fields.join(",\n"))
    }};
}

pub(crate) use pyrepr;
