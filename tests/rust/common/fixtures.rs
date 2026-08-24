use rstest::fixture;
use tempfile::TempDir;

#[fixture]
pub fn tmp_dir() -> TempDir {
    TempDir::new().unwrap()
}
