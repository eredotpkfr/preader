use rstest_reuse::template;

#[template]
#[rstest]
#[case::traversal("../../etc/passwd", native("path escapes root: ../../etc/passwd"))]
#[case::parent("..", native("path escapes root: .."))]
#[case::inner_traversal("foo/../bar", native("path escapes root: foo/../bar"))]
#[case::root("/", native("path escapes root: /"))]
#[case::absolute("/tmp", native("path escapes root: /tmp"))]
#[case::empty("", native("path must not be empty"))]
#[case::current_dir(".", native("path must not be empty"))]
fn unsafe_names(#[case] name: &str, #[case] message: String) {}

#[template]
#[rstest]
#[case::drive_absolute(r"C:\job-1", r"path escapes root: C:\job-1")]
#[case::drive_forward_slash("C:/job-1", r"path escapes root: C:\job-1")]
#[case::drive_relative("C:job-1", "path escapes root: C:job-1")]
#[case::root_relative(r"\job-1", r"path escapes root: \job-1")]
#[case::unc_share(r"\\server\share\job-1", r"path escapes root: \\server\share\job-1")]
#[case::verbatim_drive(r"\\?\C:\job-1", r"path escapes root: \\?\C:\job-1")]
#[case::backslash_traversal(r"..\..\etc\passwd", r"path escapes root: ..\..\etc\passwd")]
fn windows_unsafe_names(#[case] name: &str, #[case] message: &str) {}

#[template]
#[rstest]
#[case::unparsable("not valid json", "expected ident")]
#[case::missing_fields("{}", "missing field `name`")]
#[case::empty("", "EOF while parsing")]
fn malformed_payloads(#[case] payload: &str, #[case] message: &str) {}

#[template]
#[rstest]
#[case::plain("README.md")]
#[case::suffix_shaped("job-1.state.json.bak")]
#[case::partial("job-1.state")]
#[case::a_temporary_file("job-1.state.json.tmp")]
fn non_state_files(#[case] name: &str) {}

#[template]
#[rstest]
#[case::newline(b'\n')]
#[case::null(b'\0')]
#[case::letter(b'x')]
#[case::high_byte(0xE9)]
fn delimiter_characters(#[case] character: u8) {}
