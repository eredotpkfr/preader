use preader::NameError;
use rstest::rstest;

#[rstest]
#[case::empty(NameError::Empty, "state name must not be empty")]
#[case::escapes(NameError::Escapes, "state name escapes the state directory")]
#[case::invalid(NameError::Invalid, "state name is not valid")]
#[case::alias(
    NameError::Alias,
    "state name is an alias of another entry (symlink, case or Unicode)"
)]
fn every_variant_describes_itself(#[case] error: NameError, #[case] expected: &str) {
    assert_eq!(error.to_string(), expected);
}

#[rstest]
fn name_error_converts_into_an_error() {
    let error = preader::Error::from(NameError::Alias);

    assert!(matches!(error, preader::Error::Name(NameError::Alias)));

    assert_eq!(error.to_string(), NameError::Alias.to_string());
}
