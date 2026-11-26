use preader::Timestamps;
use rstest::rstest;
use serde_json::{Value, error::Category, json};
use time::{Duration, UtcDateTime};

use crate::common::{fixtures::sandbox, sandbox::Sandbox};

#[rstest]
fn fields(sandbox: Sandbox) {
    let path = sandbox.line_file();
    let before = UtcDateTime::now().truncate_to_second();
    let timestamps = sandbox.state(&path).timestamps;
    let after = UtcDateTime::now();

    assert!(before <= timestamps.created_at);
    assert!(timestamps.created_at <= after);

    assert_eq!(timestamps.created_at, timestamps.updated_at);
    assert_eq!(timestamps.created_at.nanosecond(), 0);
}

#[rstest]
fn compares_by_value() {
    let timestamps = Timestamps::now();
    let touched = Timestamps {
        updated_at: timestamps.updated_at + Duration::SECOND,
        ..timestamps.clone()
    };

    assert_eq!(timestamps, timestamps.clone());

    assert_ne!(timestamps, touched);
}

#[rstest]
fn now_starts_both_stamps() {
    let timestamps = Timestamps::now();

    assert_eq!(timestamps.created_at, timestamps.updated_at);
    assert_eq!(timestamps.created_at.nanosecond(), 0);
}

#[rstest]
fn deserialization_fails_when_a_stamp_is_out_of_range() {
    let payload = json!({ "created_at": i64::MAX, "updated_at": 2 });
    let error = serde_json::from_value::<Timestamps>(payload).unwrap_err();

    assert_eq!(error.classify(), Category::Data);
}

#[rstest]
fn serializes_as_unix_seconds() {
    let timestamps = Timestamps::now();
    let payload: Value = serde_json::to_value(&timestamps).unwrap();

    assert_eq!(
        payload,
        json!({
            "created_at": timestamps.created_at.unix_timestamp(),
            "updated_at": timestamps.updated_at.unix_timestamp(),
        })
    );
}

#[rstest]
fn deserializes_from_unix_seconds() {
    let payload = json!({ "created_at": 1, "updated_at": 2 });
    let timestamps: Timestamps = serde_json::from_value(payload).unwrap();

    assert_eq!(timestamps.created_at.unix_timestamp(), 1);
    assert_eq!(timestamps.updated_at.unix_timestamp(), 2);
}
