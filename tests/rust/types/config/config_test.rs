use std::path::PathBuf;

use preader::{
    Config, DEFAULT_VERIFY_STATE, IteratorConfig, StateManagerConfig, default_state_dir,
};
use rstest::{fixture, rstest};

#[fixture]
fn config() -> Config {
    Config {
        buffer_capacity: 111,
        state_dir: PathBuf::from("/tmp/preader-distinctive"),
        auto_save_state: true,
        auto_save_state_bytes: 222,
        auto_load_state: true,
        verify_state: false,
    }
}

#[rstest]
fn iterator_config_carries_its_fields(config: Config) {
    let iterator = IteratorConfig::from(&config);

    assert_eq!(iterator.buffer_capacity, config.buffer_capacity);
    assert_eq!(iterator.auto_save_state, config.auto_save_state);
    assert_eq!(iterator.auto_save_state_bytes, config.auto_save_state_bytes);
}

#[rstest]
fn manager_config_carries_its_fields(config: Config) {
    let manager = StateManagerConfig::from(&config);

    assert_eq!(manager.state_dir, config.state_dir);
    assert_eq!(manager.verify_state, config.verify_state);
}

#[rstest]
fn auto_load_state_reaches_neither_sub_config(config: Config) {
    let mut flipped = config.clone();
    flipped.auto_load_state = !config.auto_load_state;

    let iterator = IteratorConfig::from(&config);
    let flipped_iterator = IteratorConfig::from(&flipped);

    assert_eq!(iterator.buffer_capacity, flipped_iterator.buffer_capacity);
    assert_eq!(iterator.auto_save_state, flipped_iterator.auto_save_state);
    assert_eq!(
        iterator.auto_save_state_bytes,
        flipped_iterator.auto_save_state_bytes
    );

    let manager = StateManagerConfig::from(&config);
    let flipped_manager = StateManagerConfig::from(&flipped);

    assert_eq!(manager.state_dir, flipped_manager.state_dir);
    assert_eq!(manager.verify_state, flipped_manager.verify_state);
}

#[rstest]
fn manager_config_default_uses_the_crate_defaults() {
    let manager = StateManagerConfig::default();

    assert_eq!(manager.state_dir, default_state_dir());
    assert_eq!(manager.verify_state, DEFAULT_VERIFY_STATE);
}

#[rstest]
fn manager_config_default_and_conversion_agree() {
    let from_config = StateManagerConfig::from(&Config::default());
    let standalone = StateManagerConfig::default();

    assert_eq!(from_config.state_dir, standalone.state_dir);
    assert_eq!(from_config.verify_state, standalone.verify_state);
}
