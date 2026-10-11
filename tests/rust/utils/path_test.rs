use std::path::Path;
#[cfg(unix)]
use std::{fs, os::unix::fs::symlink};

use preader::{DEFAULT_STATE_DIR, NameError, default_state_dir, resolves_in_place, scoped_join};
use rstest::{fixture, rstest};
use rstest_reuse::apply;

use crate::common::{
    fixtures::sandbox, macros::asserts::assert_err_is, sandbox::Sandbox,
    templates::name::valid_names,
};

#[fixture]
fn root() -> &'static Path {
    Path::new("/tmp/preader")
}

#[apply(valid_names)]
fn scoped_join_keeps_a_valid_name_inside_the_root(root: &Path, #[case] name: &str) {
    let joined = scoped_join(root, name).unwrap();
    let parts: Vec<&str> = joined
        .strip_prefix(root)
        .unwrap()
        .iter()
        .map(|part| part.to_str().unwrap())
        .collect();

    assert!(joined.starts_with(root));

    assert_eq!(parts.join("/"), name);
}

#[rstest]
fn scoped_join_uses_the_native_separator(root: &Path) {
    let joined = scoped_join(root, "sub-1/sub-2/job-1").unwrap();

    assert_eq!(
        joined.as_os_str(),
        root.join("sub-1").join("sub-2").join("job-1").as_os_str()
    );
}

#[rstest]
#[case::empty("", NameError::Empty)]
#[case::escapes("../x", NameError::Escapes)]
#[case::invalid("job:1", NameError::Invalid)]
fn scoped_join_fails_when_the_name_is_invalid(
    root: &Path,
    #[case] name: &str,
    #[case] expected: NameError,
) {
    assert_err_is!(scoped_join(root, name), error if *error == expected);
}

#[rstest]
fn scoped_join_accepts_an_empty_root() {
    let joined = scoped_join(Path::new(""), DEFAULT_STATE_DIR).unwrap();

    assert_eq!(joined, Path::new(DEFAULT_STATE_DIR));
}

#[rstest]
fn scoped_join_does_not_validate_the_root() {
    let root = Path::new("/tmp/../preader");

    assert_eq!(scoped_join(root, "job-1").unwrap(), root.join("job-1"));
}

#[rstest]
fn default_state_dir_ends_with_directory_name() {
    assert_eq!(default_state_dir().file_name().unwrap(), DEFAULT_STATE_DIR);
}

#[rstest]
fn default_state_dir_is_absolute() {
    assert!(default_state_dir().is_absolute());
}

#[rstest]
fn resolves_in_place_accepts_a_real_path(sandbox: Sandbox) {
    assert!(resolves_in_place(
        sandbox.path(),
        &sandbox.path().join("job-1.state.json")
    ));
}

#[rstest]
fn resolves_in_place_accepts_a_missing_root() {
    let missing = Path::new("/tmp/preader-never-created");

    assert!(resolves_in_place(
        missing,
        &missing.join("job-1.state.json")
    ));
}

#[cfg(unix)]
#[rstest]
fn resolves_in_place_rejects_a_symlinked_file(sandbox: Sandbox) {
    let real = sandbox.path().join("real.state.json");
    let alias = sandbox.path().join("alias.state.json");

    fs::write(&real, b"{}").unwrap();

    symlink(&real, &alias).unwrap();

    assert!(resolves_in_place(sandbox.path(), &real));
    assert!(!resolves_in_place(sandbox.path(), &alias));
}

#[cfg(unix)]
#[rstest]
fn resolves_in_place_rejects_an_escaping_symlink(sandbox: Sandbox) {
    let root = sandbox.path().join("root");
    let outside = sandbox.path().join("outside");

    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&outside).unwrap();

    symlink(&outside, root.join("link")).unwrap();

    assert!(!resolves_in_place(
        &root,
        &root.join("link").join("job-1.state.json")
    ));
}
