use std::path::PathBuf;

use preader::{AutoSave, Config, DEFAULT_VERIFY_STATE, StateManagerConfig, default_state_dir};
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
#[case::disabled(false, 0, AutoSave::Off)]
#[case::disabled_with_a_threshold(false, 222, AutoSave::Off)]
#[case::only_at_the_end(true, 0, AutoSave::Final)]
#[case::every_threshold(true, 222, AutoSave::Every(222))]
fn auto_save_collapses_the_config_pair(
    mut config: Config,
    #[case] enabled: bool,
    #[case] threshold: u64,
    #[case] expected: AutoSave,
) {
    config.auto_save_state = enabled;
    config.auto_save_state_bytes = threshold;

    assert_eq!(AutoSave::from(&config), expected);
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

    assert_eq!(AutoSave::from(&config), AutoSave::from(&flipped));

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

#[rstest]
fn a_config_survives_a_json_round_trip(config: Config) {
    let encoded = serde_json::to_string(&config).unwrap();

    assert_eq!(serde_json::from_str::<Config>(&encoded).unwrap(), config);
}

#[rstest]
fn a_serialized_config_names_every_field(config: Config) {
    let encoded = serde_json::to_value(&config).unwrap();
    let mut fields = encoded.as_object().unwrap().keys().cloned().collect::<Vec<_>>();

    fields.sort();

    assert_eq!(
        fields,
        [
            "auto_load_state",
            "auto_save_state",
            "auto_save_state_bytes",
            "buffer_capacity",
            "state_dir",
            "verify_state",
        ]
    );
}
