use std::{fs, io::ErrorKind};

use preader::{Config, Error, IteratorBuild, IteratorRead};
use rstest::rstest;
use tempfile::TempDir;

use crate::common::{
    constants::TEST_STATE_NAME,
    fixtures::tmp_dir,
    funcs::{reader, write},
};

const CONTENT: &[u8] = b"abcdefghijklmnopqrstuvwxyz";

fn drain(tmp_dir: &TempDir, config: Config) -> u64 {
    let reader = reader(tmp_dir, config);
    let path = write(tmp_dir, "data.bin", CONTENT);
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    while bytes.read().unwrap().is_some() {}

    bytes.state().position
}

#[rstest]
fn a_finished_read_reports_full_progress(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", CONTENT);
    let mut bytes = reader.bytes(&path).build().unwrap();

    assert_eq!(bytes.state().percent(), 0.0);

    while bytes.read().unwrap().is_some() {}

    assert_eq!(bytes.state().position, CONTENT.len() as u64);
    assert_eq!(bytes.state().percent(), 100.0);
}

#[rstest]
fn auto_save_disabled_writes_nothing(tmp_dir: TempDir) {
    assert_eq!(drain(&tmp_dir, Config::default()), CONTENT.len() as u64);
    assert!(!reader(&tmp_dir, Config::default()).states().exists(TEST_STATE_NAME));
}

#[rstest]
fn a_zero_threshold_saves_only_at_the_end(tmp_dir: TempDir) {
    let config = Config {
        auto_save_state: true,
        auto_save_state_bytes: 0,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.bin", CONTENT);
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    for _ in 0..5 {
        bytes.read().unwrap();
    }

    assert!(!reader.states().exists(TEST_STATE_NAME));

    while bytes.read().unwrap().is_some() {}

    assert_eq!(
        reader.states().load(TEST_STATE_NAME).unwrap().position,
        CONTENT.len() as u64
    );
}

#[rstest]
fn a_threshold_saves_during_iteration(tmp_dir: TempDir) {
    let config = Config {
        auto_save_state: true,
        auto_save_state_bytes: 10,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.bin", CONTENT);
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();
    let mut saved = Vec::new();

    while bytes.read().unwrap().is_some() {
        if let Some(state) = reader.states().find(TEST_STATE_NAME).unwrap()
            && saved.last() != Some(&state.position)
        {
            saved.push(state.position);
        }
    }

    assert_eq!(saved, [10, 20]);
}

#[rstest]
fn reading_past_exhaustion_does_not_resave(tmp_dir: TempDir) {
    let config = Config {
        auto_save_state: true,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.bin", CONTENT);
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    while bytes.read().unwrap().is_some() {}

    assert_eq!(
        reader.states().load(TEST_STATE_NAME).unwrap().position,
        CONTENT.len() as u64
    );

    // Idempotence comes from the pending delta reaching zero, not from a latch.
    // Deleting the file makes a second write observable: the timestamps are stored
    // at one-second resolution, so comparing them would prove nothing.
    reader.states().delete(TEST_STATE_NAME).unwrap();

    assert!(bytes.read().unwrap().is_none());
    assert!(!reader.states().exists(TEST_STATE_NAME));
}

#[rstest]
fn a_late_final_save_still_flushes_the_tail(tmp_dir: TempDir) {
    let config = Config {
        auto_save_state: true,
        auto_save_state_bytes: 1000,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.bin", CONTENT);
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    // The threshold never fires, so the whole run depends on the final save.
    for _ in 0..3 {
        bytes.read().unwrap();
    }

    assert!(!reader.states().exists(TEST_STATE_NAME));

    while bytes.read().unwrap().is_some() {}

    assert_eq!(
        reader.states().load(TEST_STATE_NAME).unwrap().position,
        CONTENT.len() as u64
    );
}

#[rstest]
fn dropping_an_unfinished_iterator_saves_its_progress(tmp_dir: TempDir) {
    let config = Config {
        auto_save_state: true,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.bin", CONTENT);
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    for _ in 0..3 {
        bytes.read().unwrap();
    }

    drop(bytes);

    assert_eq!(reader.states().load(TEST_STATE_NAME).unwrap().position, 3);
}

#[rstest]
fn save_records_the_current_position(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", CONTENT);
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    for _ in 0..4 {
        bytes.read().unwrap();
    }

    let mut saved = bytes.state().clone();

    saved.save().unwrap();

    assert_eq!(reader.states().load(TEST_STATE_NAME).unwrap().position, 4);
}

#[rstest]
fn a_resumed_read_continues_where_it_stopped(tmp_dir: TempDir) {
    let config = Config {
        auto_load_state: true,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.bin", CONTENT);
    let mut first = reader.bytes(&path).state(TEST_STATE_NAME).limit(4).build().unwrap();

    while first.read().unwrap().is_some() {}

    let mut saved = first.state().clone();

    saved.save().unwrap();
    drop(first);

    let mut resumed = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();
    let mut collected = Vec::new();

    while let Some(byte) = resumed.read().unwrap() {
        collected.push(byte);
    }

    assert_eq!(collected, &CONTENT[4..]);
}

#[rstest]
fn a_state_object_resumes_without_auto_load(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", CONTENT);
    let mut first = reader.bytes(&path).state(TEST_STATE_NAME).limit(6).build().unwrap();

    while first.read().unwrap().is_some() {}

    let mut saved = first.state().clone();

    saved.save().unwrap();

    let saved = reader.states().load(TEST_STATE_NAME).unwrap();
    let mut resumed = reader.bytes(&path).state(saved).build().unwrap();
    let mut collected = Vec::new();

    while let Some(byte) = resumed.read().unwrap() {
        collected.push(byte);
    }

    assert_eq!(collected, &CONTENT[6..]);
}

#[rstest]
fn a_state_object_for_another_file_is_rejected(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let tracked = write(&tmp_dir, "tracked.bin", CONTENT);
    let untracked = write(&tmp_dir, "untracked.bin", CONTENT);
    let saved = reader.bytes(&tracked).build().unwrap().state().clone();
    let error = reader.bytes(&untracked).state(saved).build().unwrap_err();

    assert!(error.to_string().contains("file path mismatch"));
}

#[rstest]
fn verification_rejects_a_grown_file(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", CONTENT);
    let saved = reader.bytes(&path).build().unwrap().state().clone();

    fs::write(&path, [CONTENT, b"more"].concat()).unwrap();

    let error = reader.bytes(&path).state(saved).build().unwrap_err();

    assert!(error.to_string().contains("file size mismatch"));
}

#[rstest]
fn build_rejects_a_start_beyond_the_end(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", CONTENT);
    let error = reader.bytes(&path).start(10).end(5).build().unwrap_err();

    assert!(error.to_string().contains("start (10) must be <= end (5)"));
}

#[rstest]
fn build_fails_when_the_file_is_missing(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let error = reader.bytes(tmp_dir.path().join("missing.bin")).build().unwrap_err();

    assert!(matches!(error, Error::Io(error) if error.kind() == ErrorKind::NotFound));
}

#[rstest]
fn a_resumed_read_floors_the_last_saved_position(tmp_dir: TempDir) {
    let path = write(&tmp_dir, "data.bin", CONTENT);
    let plain = reader(&tmp_dir, Config::default());
    let mut first = plain.bytes(&path).state(TEST_STATE_NAME).limit(7).build().unwrap();

    while first.read().unwrap().is_some() {}

    let mut saved = first.state().clone();

    saved.save().unwrap();
    drop(first);

    let resumed = reader(
        &tmp_dir,
        Config {
            auto_load_state: true,
            auto_save_state: true,
            auto_save_state_bytes: 3,
            ..Config::default()
        },
    );
    let mut bytes = resumed.bytes(&path).state(TEST_STATE_NAME).build().unwrap();
    let mut saved = vec![7];

    while bytes.read().unwrap().is_some() {
        let position = resumed.states().load(TEST_STATE_NAME).unwrap().position;

        if saved.last() != Some(&position) {
            saved.push(position);
        }
    }

    // Resuming at 7 with a threshold of 3 floors the baseline to 6, so the first
    // save lands at 9 rather than at 10.
    // Resuming at 7 with a threshold of 3 floors the baseline to 6, so the first
    // save lands at 9 rather than at 10. The closing save happens inside the
    // read() that reports exhaustion, so the loop never observes it.
    assert_eq!(saved, [7, 9, 12, 15, 18, 21, 24]);
    assert_eq!(
        resumed.states().load(TEST_STATE_NAME).unwrap().position,
        CONTENT.len() as u64
    );
}

#[rstest]
fn a_failing_final_save_is_reported_once(tmp_dir: TempDir) {
    let config = Config {
        auto_save_state: true,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.bin", b"abc");
    let mut yielded = 0;
    let mut errors = 0;

    // A `for` loop that does not bail on the first error must still terminate:
    // retrying the final save would yield the same error forever.
    for item in reader.bytes(&path).state("../../escape").build().unwrap() {
        match item {
            Ok(_) => yielded += 1,
            Err(error) => {
                errors += 1;

                assert!(error.to_string().contains("path escapes root"));
            }
        }

        assert!(
            yielded + errors <= 4,
            "the iterator kept retrying the final save"
        );
    }

    assert_eq!((yielded, errors), (3, 1));
}
