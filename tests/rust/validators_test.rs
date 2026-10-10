use preader::{NameError, validate_name};
use rstest::rstest;
use rstest_reuse::apply;

use crate::common::{
    macros::asserts::assert_err_is,
    templates::name::{
        device_names, invalid_names, longest_names, too_long_names, unportable_characters,
        valid_names, windows_invalid_names,
    },
};

#[apply(valid_names)]
fn validate_name_returns_the_name(#[case] name: &str) {
    assert_eq!(validate_name(name).unwrap(), name);
}

#[apply(invalid_names)]
fn validate_name_fails_when_the_name_is_invalid(#[case] name: &str, #[case] expected: NameError) {
    assert_err_is!(validate_name(name), error if *error == expected);
}

#[apply(windows_invalid_names)]
fn validate_name_fails_when_the_name_is_not_portable(#[case] name: &str) {
    assert_err_is!(validate_name(name), NameError::Invalid);
}

#[apply(unportable_characters)]
fn validate_name_fails_when_a_character_is_not_portable(
    #[case] character: char,
    #[values("{}", "job{}1", "{}job-1", "job-1{}", "sub/job{}1", "sub{}/job-1")] shape: &str,
) {
    let name = shape.replace("{}", &character.to_string());

    assert_err_is!(validate_name(&name), NameError::Invalid);
}

#[apply(device_names)]
fn validate_name_fails_when_the_name_is_a_device(#[case] name: &str) {
    assert_err_is!(validate_name(name), NameError::Invalid);
}

#[apply(longest_names)]
fn validate_name_accepts_the_longest_portable_name(#[case] name: String) {
    assert_eq!(validate_name(&name).unwrap(), name);
}

#[apply(too_long_names)]
fn validate_name_fails_when_the_name_is_too_long(#[case] name: String) {
    assert_err_is!(validate_name(&name), NameError::Invalid);
}
