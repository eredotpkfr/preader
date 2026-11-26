use std::assert_matches;

use preader::NameError;
use rstest::rstest;

#[rstest]
#[case::empty(NameError::Empty, "state name must not be empty")]
#[case::escapes(NameError::Escapes, "state name escapes the state directory")]
#[case::invalid(NameError::Invalid, "state name is not valid")]
fn every_variant_describes_itself(#[case] error: NameError, #[case] expected: &str) {
    assert_eq!(error.to_string(), expected);
}

#[rstest]
fn name_error_converts_into_an_error() {
    let error = preader::Error::from(NameError::Invalid);

    assert_eq!(error.to_string(), NameError::Invalid.to_string());

    assert_matches!(error, preader::Error::Name(NameError::Invalid));
}
