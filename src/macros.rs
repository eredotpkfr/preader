macro_rules! pyrepr {
    ($name:literal { $($field:ident = $value:expr),* $(,)? }) => {{
        let parts: Vec<String> = vec![
            $({
                let v = $value.to_string();
                format!("  {}={}", stringify!($field), $crate::utils::text::indent_lines(&v, 2))
            }),*
        ];

        format!("{}(\n{}\n)", $name, parts.join(",\n"))
    }};
}

pub(crate) use pyrepr;
