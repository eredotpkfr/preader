use preader::PathError;
use rstest::rstest;

use crate::common::constants::TEST_STATE_NAME;

#[rstest]
#[case::empty(PathError::Empty, "path must not be empty")]
#[case::escapes(PathError::Escapes("../etc".to_owned()), "path escapes root: ../etc")]
#[case::nameless(PathError::Nameless(".".to_owned()), "path must name an entry: .")]
#[case::symlink(PathError::Symlink("link/job".to_owned()), "path escapes root via symlink: link/job")]
fn every_variant_describes_itself(#[case] error: PathError, #[case] expected: &str) {
    assert_eq!(error.to_string(), expected);
}

#[rstest]
fn path_error_converts_into_an_error() {
    let error = preader::Error::from(PathError::Symlink(TEST_STATE_NAME.to_owned()));

    assert!(matches!(error, preader::Error::Path(PathError::Symlink(_))));

    assert_eq!(
        error.to_string(),
        format!("path escapes root via symlink: {TEST_STATE_NAME}")
    );
}
