use rstest::fixture;

use crate::common::sandbox::Sandbox;

#[fixture]
pub fn sandbox() -> Sandbox {
    Sandbox::default()
}
