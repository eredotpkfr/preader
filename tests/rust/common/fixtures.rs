use rstest::fixture;

use crate::common::{sandbox::Sandbox, seed::Seed};

#[fixture]
pub fn sandbox() -> Sandbox {
    Sandbox::default()
}

#[fixture]
pub fn seed() -> Seed {
    Seed::default()
}
