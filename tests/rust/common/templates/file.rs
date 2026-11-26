use rstest_reuse::template;

#[template]
#[rstest]
#[case::plain("README.md")]
#[case::suffix_shaped("job-1.state.json.bak")]
#[case::partial_suffix("job-1.state")]
#[case::temporary_suffix("job-1.state.json.tmp")]
#[case::uppercase_suffix("job-2.STATE.JSON")]
#[case::only_looks_suffixed("mystate.json")]
fn non_state_files(#[case] name: &str) {}

#[template]
#[rstest]
#[case::doubled_suffix("job-1.state.json.state.json", "job-1.state.json")]
#[case::nested_doubled_suffix("sub/job-1.state.json.state.json", "sub/job-1.state.json")]
#[case::state_shaped_directory("archive.state.json/job-1.state.json", "archive.state.json/job-1")]
#[case::hidden(".job-2.state.json", ".job-2")]
#[case::uppercase("Job-2.state.json", "Job-2")]
#[case::unicode("café.state.json", "café")]
#[case::space("job 2.state.json", "job 2")]
fn foreign_state_files(#[case] file: &str, #[case] name: &str) {}

#[template]
#[rstest]
#[case::bare_suffix(".state.json")]
#[case::nested_bare_suffix("sub/.state.json")]
#[case::current_dir_stem("..state.json")]
#[case::parent_dir_stem("...state.json")]
fn unaddressable_files(#[case] file: &str) {}

#[cfg(unix)]
#[template]
#[rstest]
#[case::backslash(r"sub\job-2.state.json")]
#[case::colon("job:2.state.json")]
#[case::asterisk("job*2.state.json")]
#[case::directory_ending_in_a_dot("sub./job-2.state.json")]
#[case::directory_ending_in_a_space("sub /job-2.state.json")]
#[case::control_character("job\t2.state.json")]
#[case::format_character("job\u{200b}2.state.json")]
#[case::device("con.state.json")]
#[case::nested_device("sub/aux.state.json")]
#[case::too_long_file(format!("{}.state.json", "f".repeat(220)))]
#[case::too_long_whole_name(format!("{0}/{0}/{0}/{0}/{1}.state.json", "d".repeat(100), "f".repeat(109)))]
fn unrepresentable_files(#[case] file: String) {}
