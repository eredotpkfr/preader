macro_rules! assert_err_eq {
    ($outcome:expr, $message:expr $(,)?) => {{
        let error = $outcome.unwrap_err();

        assert_eq!(error.to_string(), $message);
    }};
}

macro_rules! assert_err_is {
    ($outcome:expr, $($variant:tt)+) => {{
        let error = $outcome.unwrap_err();

        assert!(matches!(&error, $($variant)+), "{error}");
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

pub(crate) use assert_err_eq;
pub(crate) use assert_err_is;
pub(crate) use assert_percent;
