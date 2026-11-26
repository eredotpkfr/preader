/// Returns early with `Err($error.into())` when `$condition` is false
///
/// The error is converted with [`Into`], so a [`NameError`](crate::NameError) or an
/// [`io::Error`](std::io::Error) can be returned from a function that returns
/// [`Result`](crate::Result)
macro_rules! ensure {
    ($condition:expr, $error:expr $(,)?) => {
        if !$condition {
            return Err($error.into());
        }
    };
}

pub(crate) use ensure;
