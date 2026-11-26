use std::num::NonZeroU64;

use preader::{Config, path};
use rstest::rstest;
use serde_json::json;

#[rstest]
fn defaults() {
    let config = Config::default();

    assert!(!config.auto_save_state);
    assert!(!config.auto_load_state);
    assert!(config.verify_state);
    assert!(config.state_dir.is_absolute());

    assert_eq!(config.buffer_capacity, 64 * 1024);
    assert_eq!(config.auto_save_state_bytes.get(), 100 * 1024 * 1024);
    assert_eq!(config.state_dir, path::default_state_dir());
    assert_eq!(config.state_dir.file_name().unwrap(), "preader");
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
        auto_save_state_bytes: NonZeroU64::new(2048).unwrap(),
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
