use preader::{AutoSave, Config, IteratorBuild, IteratorRead};
use rstest::rstest;

use crate::common::{constants::TEST_STATE_NAME, fixtures::sandbox, sandbox::Sandbox};

#[rstest]
#[case::disabled(false, 0, AutoSave::Never)]
#[case::disabled_with_a_threshold(false, 222, AutoSave::Never)]
#[case::only_at_the_end(true, 0, AutoSave::AtEnd)]
#[case::every_threshold(true, 222, AutoSave::EveryBytes(222))]
fn from_config_reads_both_knobs(
    #[case] auto_save_state: bool,
    #[case] auto_save_state_bytes: u64,
    #[case] expected: AutoSave,
) {
    let config = Config {
        auto_save_state,
        auto_save_state_bytes,
        ..Config::default()
    };

    assert_eq!(AutoSave::from(&config), expected);
}

#[rstest]
fn from_config_ignores_unrelated_knobs() {
    let config = Config::default();
    let flipped = Config {
        auto_load_state: !config.auto_load_state,
        verify_state: !config.verify_state,
        buffer_capacity: config.buffer_capacity + 1,
        ..config.clone()
    };

    assert_eq!(AutoSave::from(&config), AutoSave::from(&flipped));
}

#[rstest]
#[case::never(false, 0, false)]
#[case::at_the_end(true, 0, true)]
#[case::every_byte(true, 1, true)]
fn policy_decides_whether_a_state_is_written(
    sandbox: Sandbox,
    #[case] auto_save_state: bool,
    #[case] threshold: u64,
    #[case] written: bool,
) {
    let reader = sandbox.reader_with(Config {
        auto_save_state,
        auto_save_state_bytes: threshold,
        ..sandbox.config()
    });
    let path = sandbox.line_file();
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    while bytes.read().unwrap().is_some() {}

    drop(bytes);

    assert_eq!(sandbox.states().exists(TEST_STATE_NAME), written);
}
