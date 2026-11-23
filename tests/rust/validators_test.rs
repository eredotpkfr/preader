use preader::{NameError, validate_name};
use rstest::rstest;
use rstest_reuse::apply;

use crate::common::{
    macros::asserts::assert_err_is,
    templates::name::{invalid_names, valid_names},
};

#[apply(valid_names)]
fn validate_name_returns_the_name(#[case] name: String) {
    assert_eq!(validate_name(&name).unwrap(), name);
}

#[apply(invalid_names)]
fn validate_name_fails_when_the_name_is_invalid(#[case] name: String, #[case] expected: NameError) {
    assert_err_is!(validate_name(&name), error if *error == expected);
}
