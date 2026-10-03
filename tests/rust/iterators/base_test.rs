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
        if let Some(state) = reader.states().find(TEST_STATE_NAME)
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

    assert_eq!(saved, [7, 9, 12, 15, 18, 21, 24]);
    assert_eq!(
        resumed.states().load(TEST_STATE_NAME).unwrap().position,
        CONTENT.len() as u64
    );
}

#[rstest]
fn a_manual_save_does_not_reset_the_autosave_baseline(tmp_dir: TempDir) {
    let config = Config {
        auto_save_state: true,
        auto_save_state_bytes: 10,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.bin", CONTENT);
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    for _ in 0..5 {
        bytes.read().unwrap();
    }

    bytes.state().save().unwrap();

    assert_eq!(reader.states().load(TEST_STATE_NAME).unwrap().position, 5);

    for _ in 0..5 {
        bytes.read().unwrap();
    }

    assert_eq!(reader.states().load(TEST_STATE_NAME).unwrap().position, 10);
}

#[rstest]
fn a_failing_threshold_save_stops_the_read(tmp_dir: TempDir) {
    let config = Config {
        auto_save_state: true,
        auto_save_state_bytes: 5,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.bin", CONTENT);

    fs::write(tmp_dir.path().join("states"), b"not a directory").unwrap();

    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();
    let mut yielded = Vec::new();

    let error = loop {
        match bytes.read() {
            Ok(Some(byte)) => yielded.push(byte),
            Ok(None) => panic!("the read finished without reporting the failed save"),
            Err(error) => break error,
        }
    };

    assert!(matches!(error, Error::Io(_)), "{error}");
    assert_eq!(yielded, &CONTENT[..4]);
}

#[rstest]
fn a_failed_save_exhausts_the_iterator(tmp_dir: TempDir) {
    let config = Config {
        auto_save_state: true,
        auto_save_state_bytes: 1,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.bin", CONTENT);

    fs::write(tmp_dir.path().join("states"), b"not a directory").unwrap();

    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    bytes.read().unwrap_err();

    for _ in 0..3 {
        assert_eq!(bytes.read().unwrap(), None);
    }
}

#[rstest]
fn a_failing_final_save_reaches_every_item_first(tmp_dir: TempDir) {
    let config = Config {
        auto_save_state: true,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.bin", CONTENT);
    let mut bytes = reader.bytes(&path).state("../../escape").build().unwrap();
    let mut yielded = Vec::new();

    let error = loop {
        match bytes.read() {
            Ok(Some(byte)) => yielded.push(byte),
            Ok(None) => panic!("the read finished without reporting the failed save"),
            Err(error) => break error,
        }
    };

    assert!(error.to_string().contains("path escapes root"), "{error}");
    assert_eq!(yielded, CONTENT);
}

#[rstest]
fn an_error_ignoring_loop_still_terminates(tmp_dir: TempDir) {
    let config = Config {
        auto_save_state: true,
        auto_save_state_bytes: 1,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.bin", CONTENT);
    let mut polls = 0;

    for item in reader.bytes(&path).state("../../escape").build().unwrap() {
        polls += 1;

        assert!(
            polls <= 2,
            "the iterator kept yielding after the failed save"
        );

        if let Err(error) = item {
            assert!(error.to_string().contains("path escapes root"), "{error}");
        }
    }

    assert_eq!(polls, 1);
}

#[rstest]
fn a_read_error_reaches_the_caller_before_any_save(tmp_dir: TempDir) {
    let config = Config {
        auto_save_state: true,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.txt", b"foo\n\xff\xfe\n");
    let mut lines = reader.lines(&path).state("../../escape").build().unwrap();

    assert_eq!(lines.read().unwrap(), Some("foo"));

    let error = lines.read().unwrap_err();

    assert!(matches!(error, Error::Io(_)), "{error}");
}

#[cfg(unix)]
#[rstest]
fn a_resumed_read_covers_what_a_failed_save_left_behind(tmp_dir: TempDir) {
    use std::os::unix::fs::PermissionsExt;

    let config = Config {
        auto_save_state: true,
        auto_save_state_bytes: 5,
        auto_load_state: true,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.bin", CONTENT);
    let states = reader.states().state_dir().to_path_buf();
    let mut first = Vec::new();

    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    for _ in 0..10 {
        first.push(bytes.read().unwrap().unwrap());
    }

    drop(bytes);

    let checkpoint = reader.states().load(TEST_STATE_NAME).unwrap().position;

    fs::set_permissions(&states, fs::Permissions::from_mode(0o500)).unwrap();

    if fs::write(states.join("probe"), b"x").is_ok() {
        fs::set_permissions(&states, fs::Permissions::from_mode(0o700)).unwrap();

        return; // permissions are not enforced here
    }

    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();
    let mut second = Vec::new();

    let error = loop {
        match bytes.read() {
            Ok(Some(byte)) => second.push(byte),
            Ok(None) => panic!("the read finished without reporting the failed save"),
            Err(error) => break error,
        }
    };

    drop(bytes);
    fs::set_permissions(&states, fs::Permissions::from_mode(0o700)).unwrap();

    assert!(matches!(error, Error::Io(_)), "{error}");
    assert_eq!(
        reader.states().load(TEST_STATE_NAME).unwrap().position,
        checkpoint,
        "a failed save must leave the stored position untouched"
    );

    let mut third = Vec::new();
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    while let Some(byte) = bytes.read().unwrap() {
        third.push(byte);
    }

    drop(bytes);

    let covered = [first.as_slice(), third.as_slice()].concat();

    assert_eq!(covered, CONTENT, "resuming must leave no gap");
    assert!(
        second.iter().all(|byte| third.contains(byte)),
        "the bytes delivered before the failure must be delivered again"
    );
}
