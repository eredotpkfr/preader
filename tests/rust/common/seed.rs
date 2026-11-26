use std::{
    env, fmt,
    hash::{BuildHasher, RandomState},
};

const VARIABLE: &str = "PREADER_TEST_SEED";

#[derive(Clone, Copy, Debug)]
pub struct Seed(pub u64);

impl Default for Seed {
    fn default() -> Self {
        env::var(VARIABLE)
            .map_or_else(
                |_| RandomState::new().hash_one(()),
                |seed| seed.parse().expect("PREADER_TEST_SEED must be a u64"),
            )
            .into()
    }
}

impl From<u64> for Seed {
    fn from(seed: u64) -> Self {
        Self(seed)
    }
}

impl fmt::Display for Seed {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{VARIABLE}={}", self.0)
    }
}
