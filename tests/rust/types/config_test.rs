use preader::{
    AutoSave, Config, DEFAULT_AUTO_SAVE_STATE_BYTES, DEFAULT_BUFFER_CAPACITY, DEFAULT_VERIFY_STATE,
    default_state_dir,
};
use rstest::rstest;
use serde_json::json;

#[rstest]
fn defaults_match_the_constants() {
    let config = Config::default();

    assert_eq!(config.buffer_capacity, DEFAULT_BUFFER_CAPACITY);
    assert_eq!(config.state_dir, default_state_dir());
    assert!(!config.auto_save_state);
    assert_eq!(config.auto_save_state_bytes, DEFAULT_AUTO_SAVE_STATE_BYTES);
    assert!(!config.auto_load_state);
    assert_eq!(config.verify_state, DEFAULT_VERIFY_STATE);
}

#[rstest]
fn compares_by_value() {
    let config = Config {
        buffer_capacity: 1024,
        ..Config::default()
    };

    assert_eq!(
        config,
        Config {
            buffer_capacity: 1024,
            ..Config::default()
        }
    );
    assert_ne!(
        config,
        Config {
            buffer_capacity: 2048,
            ..Config::default()
        }
    );
}

#[rstest]
#[case::disabled(false, 0, AutoSave::Never)]
#[case::disabled_with_a_threshold(false, 222, AutoSave::Never)]
#[case::only_at_the_end(true, 0, AutoSave::AtEnd)]
#[case::every_threshold(true, 222, AutoSave::EveryBytes(222))]
fn autosave_reads_both_knobs(
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
fn autosave_ignores_unrelated_knobs() {
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
fn roundtrips_through_json() {
    let config = Config {
        state_dir: "/tmp/states".into(),
        buffer_capacity: 1024,
        auto_save_state: true,
        auto_save_state_bytes: 2048,
        auto_load_state: true,
        verify_state: false,
    };
    let payload = serde_json::to_value(&config).unwrap();

    assert_eq!(
        payload,
        json!({
            "buffer_capacity": 1024,
            "state_dir": "/tmp/states",
            "auto_save_state": true,
            "auto_save_state_bytes": 2048,
            "auto_load_state": true,
            "verify_state": false,
        })
    );
    assert_eq!(serde_json::from_value::<Config>(payload).unwrap(), config);
}
