use std::path::Path;

use preader::{DEFAULT_STATE_DIRECTORY, default_state_dir, scoped_join, strip_extension};
use rstest::{fixture, rstest};

#[fixture]
fn root() -> &'static Path {
    Path::new("/tmp/preader")
}

#[rstest]
#[case::plain("job-1.state.json")]
#[case::nested("nested/job-1.state.json")]
#[case::unnormalized("./job-1")]
fn scoped_join_keeps_a_contained_path_verbatim(root: &Path, #[case] contained: &str) {
    assert_eq!(scoped_join(root, contained).unwrap(), root.join(contained));
}

#[rstest]
#[case::empty("", "must not be empty")]
#[case::absolute("/etc/passwd", "escapes root")]
#[case::root_itself("/", "escapes root")]
#[case::parent_traversal("../../etc/passwd", "escapes root")]
#[case::parent_that_stays_inside("nested/../job-1", "escapes root")]
#[case::bare_parent("..", "escapes root")]
#[case::bare_current_dir(".", "must name an entry")]
#[case::only_current_dirs("./.", "must name an entry")]
fn scoped_join_fails_when_the_path_is_unsafe(
    root: &Path,
    #[case] unsafe_path: &str,
    #[case] message: &str,
) {
    let error = scoped_join(root, unsafe_path).unwrap_err();

    assert!(error.to_string().contains(message));
}

#[cfg(windows)]
#[rstest]
#[case::drive_absolute("C:\\job-1")]
#[case::drive_relative("C:job-1")]
#[case::root_relative("\\job-1")]
#[case::unc_share("\\\\server\\share\\job-1")]
#[case::verbatim_drive("\\\\?\\C:\\job-1")]
#[case::backslash_traversal("..\\..\\etc\\passwd")]
fn scoped_join_fails_when_a_windows_path_is_unsafe(root: &Path, #[case] unsafe_path: &str) {
    let error = scoped_join(root, unsafe_path).unwrap_err();

    assert!(error.to_string().contains("escapes root"));
}

#[rstest]
fn scoped_join_accepts_an_empty_root() {
    let joined = scoped_join(Path::new(""), DEFAULT_STATE_DIRECTORY).unwrap();

    assert_eq!(joined, Path::new(DEFAULT_STATE_DIRECTORY));
}

#[rstest]
fn scoped_join_does_not_validate_the_root() {
    let root = Path::new("/tmp/../preader");

    assert_eq!(scoped_join(root, "job-1").unwrap(), root.join("job-1"));
}

#[rstest]
fn default_state_dir_ends_with_the_directory_name() {
    assert_eq!(
        default_state_dir().file_name().unwrap(),
        DEFAULT_STATE_DIRECTORY
    );
}

#[rstest]
fn default_state_dir_is_absolute() {
    assert!(default_state_dir().is_absolute());
}

#[rstest]
#[case::suffixed("job-1.state.json", "job-1")]
#[case::not_suffixed("job-1", "job-1")]
#[case::empty_name("", "")]
#[case::only_looks_suffixed("mystate.json", "mystate.json")]
#[case::different_case("job-1.STATE.JSON", "job-1.STATE.JSON")]
#[case::bare_suffix("state.json", "state.json")]
#[case::suffix_without_a_name(".state.json", "")]
#[case::doubled_suffix("job-1.state.json.state.json", "job-1.state.json")]
fn strip_extension_removes_only_an_exact_match(#[case] name: &str, #[case] expected: &str) {
    assert_eq!(strip_extension(name, "state.json"), expected);
}
