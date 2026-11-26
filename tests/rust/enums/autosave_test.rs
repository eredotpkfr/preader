use std::num::NonZeroU64;

use preader::{AutoSave, Config, DEFAULT_AUTO_SAVE_STATE_BYTES};
use rstest::rstest;

use crate::common::{
    constants::TEST_STATE_NAME, fixtures::sandbox, funcs::drain, sandbox::Sandbox,
};

#[rstest]
fn default_is_never() {
    assert_eq!(AutoSave::default(), AutoSave::Never);
}

#[rstest]
#[case::disabled(false, DEFAULT_AUTO_SAVE_STATE_BYTES.get(), AutoSave::Never)]
#[case::disabled_with_a_threshold(false, 222, AutoSave::Never)]
#[case::enabled_with_the_default(true, DEFAULT_AUTO_SAVE_STATE_BYTES.get(), AutoSave::EveryBytes(DEFAULT_AUTO_SAVE_STATE_BYTES))]
#[case::enabled_with_a_threshold(true, 222, AutoSave::EveryBytes(NonZeroU64::new(222).unwrap()))]
fn from_config_reads_both_knobs(
    #[case] auto_save_state: bool,
    #[case] auto_save_state_bytes: u64,
    #[case] expected: AutoSave,
) {
    let config = Config {
        auto_save_state,
        auto_save_state_bytes: NonZeroU64::new(auto_save_state_bytes).unwrap(),
        ..Config::default()
    };

    assert_eq!(AutoSave::from(&config), expected);
}

#[rstest]
fn from_config_ignores_unrelated_knobs() {
    let config = Config {
        auto_save_state: true,
        ..Config::default()
    };
    let flipped = Config {
        auto_load_state: !config.auto_load_state,
        verify_state: !config.verify_state,
        buffer_capacity: config.buffer_capacity + 1,
        ..config.clone()
    };

    assert_eq!(AutoSave::from(&config), AutoSave::from(&flipped));
}

#[rstest]
#[case::never(false, 1, false)]
#[case::at_the_end(true, DEFAULT_AUTO_SAVE_STATE_BYTES.get(), true)]
#[case::every_byte(true, 1, true)]
fn policy_decides_whether_a_state_is_written(
    sandbox: Sandbox,
    #[case] auto_save_state: bool,
    #[case] threshold: u64,
    #[case] written: bool,
) {
    let reader = sandbox.reader_with(Config {
        auto_save_state,
        auto_save_state_bytes: NonZeroU64::new(threshold).unwrap(),
        ..sandbox.config()
    });
    let path = sandbox.line_file();
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    drain(&mut bytes);

    assert_eq!(sandbox.states().exists(TEST_STATE_NAME), written);
}
