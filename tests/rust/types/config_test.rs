use preader::{
    Config, DEFAULT_AUTO_SAVE_STATE_BYTES, DEFAULT_BUFFER_CAPACITY, DEFAULT_VERIFY_STATE,
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
