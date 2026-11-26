use std::fmt::Debug;

use preader::{Error, IteratorOptions, Result};
use proptest::{prop_assert, prop_assert_eq, test_runner::TestCaseResult};

use crate::common::funcs::try_items;

pub fn assert_reference<I, T>(
    built: Result<I>,
    options: IteratorOptions,
    expected: impl Into<Option<Vec<T>>>,
) -> TestCaseResult
where
    I: Iterator<Item = Result<T>>,
    T: Debug + PartialEq,
{
    if options.start > options.end {
        prop_assert!(matches!(built, Err(Error::InvalidRange)));
    } else if let Some(expected) = expected.into() {
        prop_assert_eq!(try_items(built.unwrap()).unwrap(), expected);
    } else {
        prop_assert!(matches!(try_items(built.unwrap()), Err(Error::Utf8(_))));
    }

    Ok(())
}
