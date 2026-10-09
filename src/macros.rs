macro_rules! ensure {
    ($condition:expr, $error:expr $(,)?) => {
        if !$condition {
            return Err($error.into());
        }
    };
}

pub(crate) use ensure;
