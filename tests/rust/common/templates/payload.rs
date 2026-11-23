use rstest_reuse::template;

#[template]
#[rstest]
#[case::unparsable("not valid json", "expected ident")]
#[case::missing_fields("{}", "missing field")]
#[case::empty("", "EOF while parsing")]
fn malformed_payloads(#[case] payload: &str, #[case] message: &str) {}
