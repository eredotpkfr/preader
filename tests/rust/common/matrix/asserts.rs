use std::fmt::Debug;

use preader::{Error, IteratorOptions};

pub fn assert_bounds<T: Debug>(built: Result<T, Error>, options: IteratorOptions) {
    let error = built.unwrap_err();
    let reported = Error::InvalidRange {
        start: options.start,
        end: options.end,
    };

    assert_eq!(error.to_string(), reported.to_string(), "{options:?}");
}
