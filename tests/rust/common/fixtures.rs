use rstest::fixture;
use tempfile::TempDir;

use crate::common::sandbox::Sandbox;

#[fixture]
pub fn sandbox() -> Sandbox {
    Sandbox::default()
}

#[fixture]
pub fn tmp_dir() -> TempDir {
    TempDir::new().unwrap()
}
