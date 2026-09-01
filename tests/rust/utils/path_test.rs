use std::path::{MAIN_SEPARATOR_STR, Path};
#[cfg(unix)]
use std::{fs, os::unix::fs::symlink};

use preader::{
    DEFAULT_STATE_DIRECTORY, default_state_dir, has_no_symlinks, normalize_path, path_stem,
    scoped_join, strip_extensions,
};
use rstest::{fixture, rstest};
use tempfile::TempDir;

use crate::common::fixtures::tmp_dir;

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
#[case::doubled_suffix("job-1.state.json.state.json", "job-1")]
#[case::tripled_suffix("job-1.state.json.state.json.state.json", "job-1")]
fn strip_extensions_removes_every_exact_match(#[case] name: &str, #[case] expected: &str) {
    assert_eq!(strip_extensions(name, "state.json"), expected);
}

#[rstest]
#[case::plain("job-1", "job-1")]
#[case::nested("sub/job-1", "sub/job-1")]
#[case::repeated_separator("sub//job-1", "sub/job-1")]
#[case::trailing_separator("sub/job-1/", "sub/job-1")]
#[case::interior_current_dir("sub/./job-1", "sub/job-1")]
#[case::leading_current_dir("./job-1", "job-1")]
#[case::only_current_dirs("./.", "")]
#[case::empty("", "")]
#[case::parent("..", "..")]
#[case::traversal("../../etc/passwd", "../../etc/passwd")]
fn normalize_path_reduces_to_the_native_form(#[case] path: &str, #[case] expected: &str) {
    assert_eq!(
        normalize_path(path),
        expected.replace('/', MAIN_SEPARATOR_STR)
    );
}

#[cfg(unix)]
#[rstest]
fn normalize_path_treats_a_backslash_as_a_name_character() {
    assert_eq!(normalize_path("sub\\job-1"), "sub\\job-1");
}

#[rstest]
#[case::plain("job-1", "job-1")]
#[case::suffixed("job-1.state.json", "job-1")]
#[case::doubled_suffix("job-1.state.json.state.json", "job-1")]
#[case::nested("sub/job-1.state.json", "sub/job-1")]
#[case::trailing_separator("job-1.state.json/", "job-1")]
#[case::suffix_shaped_parent(
    "sub-1/.state.json/sub-2/job-1.state.json",
    "sub-1/.state.json/sub-2/job-1"
)]
#[case::only_looks_suffixed("mystate.json", "mystate.json")]
fn path_stem_strips_the_last_extension(#[case] path: &str, #[case] expected: &str) {
    let stem = path_stem(path, "state.json");

    assert_eq!(stem, expected.replace('/', MAIN_SEPARATOR_STR));
    assert_eq!(path_stem(&stem, "state.json"), stem);
}

#[rstest]
#[case::empty("")]
#[case::current_dir(".")]
#[case::only_the_suffix(".state.json")]
#[case::suffix_as_the_last_component("job-1/.state.json")]
#[case::suffix_as_two_components("job-1/.state.json/.state.json")]
#[case::suffix_as_three_components("a/.state.json/.state.json/.state.json")]
#[case::stem_is_a_current_dir("..state.json")]
#[case::stem_is_a_parent_dir("...state.json")]
fn path_stem_is_empty_without_a_usable_stem(#[case] path: &str) {
    assert!(path_stem(path, "state.json").is_empty());
}

#[rstest]
#[case::parent("..")]
#[case::traversal("../../etc/passwd")]
#[case::root("/")]
fn path_stem_keeps_an_unsafe_path_verbatim(#[case] path: &str) {
    let stem = path_stem(path, "state.json");

    assert!(scoped_join(Path::new("/tmp/preader"), &stem).is_err());
}

#[rstest]
fn has_no_symlinks_accepts_a_real_path(tmp_dir: TempDir) {
    assert!(has_no_symlinks(
        tmp_dir.path(),
        &tmp_dir.path().join("job-1.state.json")
    ));
}

#[rstest]
fn has_no_symlinks_accepts_a_missing_root() {
    let missing = Path::new("/tmp/preader-never-created");

    assert!(has_no_symlinks(missing, &missing.join("job-1.state.json")));
}

#[cfg(unix)]
#[rstest]
fn has_no_symlinks_rejects_a_symlinked_file(tmp_dir: TempDir) {
    let real = tmp_dir.path().join("real.state.json");
    let alias = tmp_dir.path().join("alias.state.json");

    fs::write(&real, b"{}").unwrap();
    symlink(&real, &alias).unwrap();

    assert!(has_no_symlinks(tmp_dir.path(), &real));
    assert!(!has_no_symlinks(tmp_dir.path(), &alias));
}

#[cfg(unix)]
#[rstest]
fn has_no_symlinks_rejects_an_escaping_symlink(tmp_dir: TempDir) {
    let root = tmp_dir.path().join("root");
    let outside = tmp_dir.path().join("outside");

    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&outside).unwrap();

    symlink(&outside, root.join("link")).unwrap();

    assert!(!has_no_symlinks(
        &root,
        &root.join("link").join("job-1.state.json")
    ));
}
