use std::fmt::Debug;

use proptest::{
    strategy::Strategy,
    test_runner::{Config, RngSeed, TestCaseResult, TestRunner},
};

use crate::common::{constants::TEST_CASES, seed::Seed};

pub fn check<S>(seed: Seed, strategy: S, test: impl Fn(S::Value) -> TestCaseResult)
where
    S: Strategy,
    S::Value: Debug,
{
    let config = Config {
        cases: TEST_CASES,
        failure_persistence: None,
        rng_seed: RngSeed::Fixed(seed.0),
        ..Config::default()
    };

    if let Err(error) = TestRunner::new(config).run(&strategy, test) {
        panic!("{seed}\n{error}");
    }
}
