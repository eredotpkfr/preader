use preader::PathError;
use rstest::rstest;

use crate::common::constants::TEST_STATE_NAME;

#[rstest]
#[case::empty(PathError::Empty, "path must not be empty")]
#[case::escapes(PathError::Escapes("../etc".to_owned()), "path escapes root: ../etc")]
#[case::invalid(PathError::Invalid("job:1".to_owned()), "path is invalid: job:1")]
#[case::alias(PathError::Alias("link/job".to_owned()), "path is a symlink or an alias of another entry: link/job")]
fn every_variant_describes_itself(#[case] error: PathError, #[case] expected: &str) {
    assert_eq!(error.to_string(), expected);
}

#[rstest]
fn path_error_converts_into_an_error() {
    let error = preader::Error::from(PathError::Alias(TEST_STATE_NAME.to_owned()));

    assert!(matches!(error, preader::Error::Path(PathError::Alias(_))));

    assert_eq!(
        error.to_string(),
        format!("path is a symlink or an alias of another entry: {TEST_STATE_NAME}")
    );
}
