use std::path::Path;
#[cfg(unix)]
use std::{fs, os::unix::fs::symlink};

use preader::{NameError, STATE_DIR_NAME, path};
use rstest::{fixture, rstest};
use rstest_reuse::apply;

use crate::common::{
    constants::{TEST_NESTED_STATE_NAME, TEST_STATE_FILE, TEST_STATE_NAME},
    fixtures::sandbox,
    macros::asserts::assert_err_is,
    sandbox::Sandbox,
    templates::name::valid_names,
};

#[fixture]
fn root() -> &'static Path {
    Path::new("/tmp/preader")
}

#[rstest]
fn default_state_dir_is_absolute() {
    assert!(path::default_state_dir().is_absolute());
}

#[rstest]
fn default_state_dir_ends_with_directory_name() {
    assert_eq!(
        path::default_state_dir().file_name().unwrap(),
        STATE_DIR_NAME
    );
}

#[rstest]
#[case::empty("", NameError::Empty)]
#[case::escapes("../job-1", NameError::Escapes)]
#[case::invalid("job:1", NameError::Invalid)]
fn scoped_join_fails_when_the_name_is_invalid(
    root: &Path,
    #[case] name: &str,
    #[case] expected: NameError,
) {
    assert_err_is!(path::scoped_join(root, name), error if *error == expected);
}

#[rstest]
fn resolves_in_place_accepts_a_missing_file(sandbox: Sandbox) {
    assert!(path::resolves_in_place(
        sandbox.path(),
        &sandbox.path().join(TEST_STATE_FILE)
    ));
}

#[rstest]
fn resolves_in_place_accepts_a_missing_root(sandbox: Sandbox) {
    let missing = sandbox.path().join("missing");

    assert!(path::resolves_in_place(
        &missing,
        &missing.join(TEST_STATE_FILE)
    ));
}

#[rstest]
fn scoped_join_accepts_an_empty_root() {
    let joined = path::scoped_join(Path::new(""), STATE_DIR_NAME).unwrap();

    assert_eq!(joined, Path::new(STATE_DIR_NAME));
}

#[rstest]
fn scoped_join_does_not_validate_the_root() {
    let root = Path::new("/tmp/../preader");

    assert_eq!(
        path::scoped_join(root, TEST_STATE_NAME).unwrap(),
        root.join(TEST_STATE_NAME)
    );
}

#[rstest]
fn resolves_in_place_accepts_a_path_outside_the_root(sandbox: Sandbox) {
    let root = sandbox.dir_at("root");
    let outside = sandbox.write("outside.bin", b"foo");

    assert!(path::resolves_in_place(&root, &outside));
}

#[rstest]
fn scoped_join_uses_the_native_separator(root: &Path) {
    let joined = path::scoped_join(root, TEST_NESTED_STATE_NAME).unwrap();

    assert_eq!(
        joined.as_os_str(),
        root.join("sub-1").join("sub-2").join(TEST_STATE_NAME).as_os_str()
    );
}

#[cfg(unix)]
#[rstest]
fn resolves_in_place_rejects_a_dangling_symlink(sandbox: Sandbox) {
    let alias = sandbox.path().join("alias.state.json");

    symlink(sandbox.path().join("missing.state.json"), &alias).unwrap();

    assert!(!path::resolves_in_place(sandbox.path(), &alias));
}

#[cfg(unix)]
#[rstest]
fn resolves_in_place_rejects_an_escaping_symlink(sandbox: Sandbox) {
    let root = sandbox.dir_at("root");
    let outside = sandbox.dir_at("outside");

    symlink(&outside, root.join("link")).unwrap();

    assert!(!path::resolves_in_place(
        &root,
        &root.join("link").join(TEST_STATE_FILE)
    ));
}

#[apply(valid_names)]
fn scoped_join_keeps_a_valid_name_inside_the_root(root: &Path, #[case] name: String) {
    let joined = path::scoped_join(root, &name).unwrap();
    let parts: Vec<&str> = joined
        .strip_prefix(root)
        .unwrap()
        .iter()
        .map(|part| part.to_str().unwrap())
        .collect();

    assert!(joined.starts_with(root));

    assert_eq!(parts.join("/"), name);
}

#[cfg(unix)]
#[rstest]
fn resolves_in_place_rejects_a_symlinked_file(sandbox: Sandbox) {
    let real = sandbox.path().join("real.state.json");
    let alias = sandbox.path().join("alias.state.json");

    fs::write(&real, b"{}").unwrap();

    symlink(&real, &alias).unwrap();

    assert!(path::resolves_in_place(sandbox.path(), &real));
    assert!(!path::resolves_in_place(sandbox.path(), &alias));
}
