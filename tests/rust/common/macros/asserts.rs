macro_rules! assert_err_eq {
    ($outcome:expr, $message:expr $(,)?) => {{
        let error = $outcome.unwrap_err();

        assert_eq!(error.to_string(), $message);
    }};
}

macro_rules! assert_err_is {
    ($outcome:expr, $($variant:tt)+) => {{
        let error = $outcome.unwrap_err();

        std::assert_matches!(&error, $($variant)+);
    }};
}

pub(crate) use assert_err_eq;
pub(crate) use assert_err_is;
