use rstest_reuse::template;

#[template]
#[rstest]
#[case::unparsable("not valid json")]
#[case::missing_fields("{}")]
#[case::empty("")]
fn malformed_payloads(#[case] payload: &str) {}
