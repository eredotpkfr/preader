use chrono::TimeDelta;
use preader::Timestamps;
use rstest::rstest;
use serde_json::{Value, json};

#[rstest]
fn now_starts_both_stamps() {
    let timestamps = Timestamps::now();

    assert_eq!(timestamps.created_at, timestamps.updated_at);
}

#[rstest]
fn compares_by_value() {
    let timestamps = Timestamps::now();
    let touched = Timestamps {
        updated_at: timestamps.updated_at + TimeDelta::seconds(1),
        ..timestamps.clone()
    };

    assert_eq!(timestamps, timestamps.clone());
    assert_ne!(timestamps, touched);
}

#[rstest]
fn serializes_as_unix_seconds() {
    let timestamps = Timestamps::now();
    let payload: Value = serde_json::to_value(&timestamps).unwrap();

    assert_eq!(
        payload,
        json!({
            "created_at": timestamps.created_at.timestamp(),
            "updated_at": timestamps.updated_at.timestamp(),
        })
    );
}

#[rstest]
fn deserializes_from_unix_seconds() {
    let payload = json!({ "created_at": 1, "updated_at": 2 });
    let timestamps: Timestamps = serde_json::from_value(payload).unwrap();

    assert_eq!(timestamps.created_at.timestamp(), 1);
    assert_eq!(timestamps.updated_at.timestamp(), 2);
}
