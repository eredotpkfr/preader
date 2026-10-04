macro_rules! assert_err {
    ($outcome:expr, $message:expr $(,)?) => {{
        let error = $outcome.unwrap_err();

        assert!(error.to_string().contains($message), "{error}");
    }};
}

macro_rules! assert_percent {
    ($builder:expr, $size:expr) => {{
        let mut iterator = $builder.build().unwrap();

        while iterator.read().unwrap().is_some() {
            assert!(iterator.state().position <= $size);
        }

        assert_eq!(iterator.state().position, $size);
        assert_eq!(iterator.state().percent(), 100.0);
    }};
}

pub(crate) use assert_err;
pub(crate) use assert_percent;
