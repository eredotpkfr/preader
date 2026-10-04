use std::fs;

use preader::{Config, Error, IteratorBuild, IteratorRead};
use rstest::rstest;

use crate::common::{
    constants::{TEST_ALPHABET, TEST_OTHER_STATE_NAME, TEST_STATE_NAME},
    fixtures::sandbox,
    funcs::{items, take},
    guards::Blocked,
    sandbox::Sandbox,
};

const SIZE: u64 = TEST_ALPHABET.len() as u64;

fn block_the_state_dir(sandbox: &Sandbox) {
    fs::write(sandbox.state_dir(), b"not a directory").unwrap();
}

#[rstest]
fn a_finished_read_reports_full_progress(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let mut bytes = sandbox.reader().bytes(&path).build().unwrap();

    assert_eq!(bytes.state().percent(), 0.0);

    while bytes.read().unwrap().is_some() {}

    assert_eq!(bytes.state().position, SIZE);
    assert_eq!(bytes.state().percent(), 100.0);
}

#[rstest]
fn autosave_off_writes_nothing(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let mut bytes = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    while bytes.read().unwrap().is_some() {}

    drop(bytes);

    assert!(!sandbox.states().exists(TEST_STATE_NAME));
}

#[rstest]
fn a_zero_threshold_saves_only_at_the_end(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.autosaving(0);
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    for _ in 0..5 {
        bytes.read().unwrap();
    }

    assert!(!sandbox.states().exists(TEST_STATE_NAME));

    while bytes.read().unwrap().is_some() {}

    assert_eq!(
        sandbox.states().load(TEST_STATE_NAME).unwrap().position,
        SIZE
    );
}

#[rstest]
fn a_threshold_saves_during_iteration(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.autosaving(10);
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();
    let mut positions = Vec::new();

    while bytes.read().unwrap().is_some() {
        if let Some(state) = sandbox.states().find(TEST_STATE_NAME)
            && positions.last() != Some(&state.position)
        {
            positions.push(state.position);
        }
    }

    assert_eq!(positions, [10, 20]);
}

#[rstest]
fn a_threshold_records_every_item_boundary(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.autosaving(1);
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();
    let mut positions = Vec::new();

    while bytes.read().unwrap().is_some() {
        positions.push(sandbox.states().load(TEST_STATE_NAME).unwrap().position);
    }

    assert_eq!(positions, (1..=SIZE).collect::<Vec<u64>>());
}

#[rstest]
fn a_late_final_save_flushes_the_tail(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.autosaving(1000);
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    for _ in 0..3 {
        bytes.read().unwrap();
    }

    assert!(!sandbox.states().exists(TEST_STATE_NAME));

    while bytes.read().unwrap().is_some() {}

    assert_eq!(
        sandbox.states().load(TEST_STATE_NAME).unwrap().position,
        SIZE
    );
}

#[rstest]
fn reading_past_exhaustion_does_not_resave(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.autosaving(100 * 1024 * 1024);
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    while bytes.read().unwrap().is_some() {}

    assert_eq!(
        sandbox.states().load(TEST_STATE_NAME).unwrap().position,
        SIZE
    );

    sandbox.states().delete(TEST_STATE_NAME).unwrap();

    assert!(bytes.read().unwrap().is_none());
    assert!(!sandbox.states().exists(TEST_STATE_NAME));
}

#[rstest]
fn dropping_an_unfinished_iterator_saves_its_progress(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.autosaving(100 * 1024 * 1024);
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    for _ in 0..3 {
        bytes.read().unwrap();
    }

    drop(bytes);

    assert_eq!(sandbox.states().load(TEST_STATE_NAME).unwrap().position, 3);
}

#[rstest]
fn dropping_without_progress_writes_nothing(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.autosaving(100 * 1024 * 1024);

    drop(reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap());

    assert!(!sandbox.states().exists(TEST_STATE_NAME));
}

#[rstest]
fn dropping_with_autosave_off_writes_nothing(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let mut bytes = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    bytes.read().unwrap();
    drop(bytes);

    assert!(!sandbox.states().exists(TEST_STATE_NAME));
}

#[rstest]
fn a_manual_save_records_the_current_position(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let mut bytes = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    for _ in 0..4 {
        bytes.read().unwrap();
    }

    bytes.state().save().unwrap();

    assert_eq!(sandbox.states().load(TEST_STATE_NAME).unwrap().position, 4);
}

#[rstest]
fn a_manual_save_does_not_reset_the_autosave_baseline(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.autosaving(10);
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    for _ in 0..5 {
        bytes.read().unwrap();
    }

    bytes.state().save().unwrap();

    assert_eq!(sandbox.states().load(TEST_STATE_NAME).unwrap().position, 5);

    for _ in 0..5 {
        bytes.read().unwrap();
    }

    assert_eq!(sandbox.states().load(TEST_STATE_NAME).unwrap().position, 10);
}

#[rstest]
fn a_resumed_read_continues_where_it_stopped(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.resuming();
    let mut first = reader.bytes(&path).state(TEST_STATE_NAME).limit(4).build().unwrap();

    while first.read().unwrap().is_some() {}

    first.state().save().unwrap();
    drop(first);

    let resumed = items(reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap());

    assert_eq!(resumed.concat(), &TEST_ALPHABET[4..]);
}

#[rstest]
fn a_resumed_read_floors_the_last_saved_position(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let mut first = sandbox.reader().bytes(&path).state(TEST_STATE_NAME).limit(7).build().unwrap();

    while first.read().unwrap().is_some() {}

    first.state().save().unwrap();
    drop(first);

    let resumed = sandbox.reader_with(Config {
        auto_load_state: true,
        auto_save_state: true,
        auto_save_state_bytes: 3,
        ..sandbox.config()
    });
    let mut bytes = resumed.bytes(&path).state(TEST_STATE_NAME).build().unwrap();
    let mut positions = vec![7];

    while bytes.read().unwrap().is_some() {
        let position = sandbox.states().load(TEST_STATE_NAME).unwrap().position;

        if positions.last() != Some(&position) {
            positions.push(position);
        }
    }

    assert_eq!(positions, [7, 9, 12, 15, 18, 21, 24]);
    assert_eq!(
        sandbox.states().load(TEST_STATE_NAME).unwrap().position,
        SIZE
    );
}

#[rstest]
fn replacing_the_state_with_an_earlier_one_keeps_reading(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.reader_with(Config {
        auto_save_state: true,
        auto_save_state_bytes: 5,
        auto_load_state: true,
        ..sandbox.config()
    });
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    for _ in 0..10 {
        bytes.read().unwrap();
    }

    drop(bytes);

    let rewound = sandbox.named_state(&path, TEST_OTHER_STATE_NAME);
    let mut resumed = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    assert_eq!(resumed.state().position, 10);

    *resumed.state() = rewound;

    assert_eq!(resumed.read().unwrap(), Some(TEST_ALPHABET[10]));
    assert_eq!(resumed.state().position, 1);
}

#[rstest]
fn a_failing_threshold_save_stops_the_read(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);

    block_the_state_dir(&sandbox);

    let reader = sandbox.autosaving(5);
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
    assert_eq!(yielded, &TEST_ALPHABET[..4]);
}

#[rstest]
fn a_failed_save_exhausts_the_iterator(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);

    block_the_state_dir(&sandbox);

    let reader = sandbox.autosaving(1);
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    bytes.read().unwrap_err();

    for _ in 0..3 {
        assert_eq!(bytes.read().unwrap(), None);
    }
}

#[rstest]
fn a_failing_final_save_reaches_every_item_first(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.autosaving(0);
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
    assert_eq!(yielded, TEST_ALPHABET);
}

#[rstest]
fn an_error_ignoring_loop_still_terminates(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.autosaving(1);
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
fn a_read_error_reaches_the_caller_before_any_save(sandbox: Sandbox) {
    let path = sandbox.file(b"foo\n\xff\xfe\n");
    let reader = sandbox.autosaving(0);
    let mut lines = reader.lines(&path).state("../../escape").build().unwrap();

    assert_eq!(lines.read().unwrap(), Some("foo"));
    assert!(matches!(lines.read().unwrap_err(), Error::Utf8(_)));
}

#[cfg(unix)]
#[rstest]
fn a_resumed_read_covers_what_a_failed_save_left_behind(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.reader_with(Config {
        auto_save_state: true,
        auto_save_state_bytes: 5,
        auto_load_state: true,
        ..sandbox.config()
    });
    let mut blocked = Blocked::default();
    let mut first = Vec::new();
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    for _ in 0..10 {
        first.push(bytes.read().unwrap().unwrap());
    }

    drop(bytes);

    let checkpoint = sandbox.states().load(TEST_STATE_NAME).unwrap().position;

    blocked.read_only(sandbox.state_dir());

    if fs::write(sandbox.state_dir().join("probe"), b"x").is_ok() {
        return;
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
    drop(blocked);

    assert!(matches!(error, Error::Io(_)), "{error}");
    assert_eq!(
        sandbox.states().load(TEST_STATE_NAME).unwrap().position,
        checkpoint
    );

    let third: Vec<u8> =
        items(reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap()).concat();

    assert_eq!([first.as_slice(), third.as_slice()].concat(), TEST_ALPHABET);
    assert!(second.iter().all(|byte| third.contains(byte)));
}

#[rstest]
fn two_iterators_with_one_name_advance_independently(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.reader();
    let mut first = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();
    let mut second = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    first.read().unwrap();

    assert_eq!(first.state().position, 1);
    assert_eq!(second.state().position, 0);
    assert_eq!(second.read().unwrap(), Some(TEST_ALPHABET[0]));
}

#[rstest]
fn a_resumed_read_applies_the_limit_again(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.resuming();
    let mut first = reader.bytes(&path).state(TEST_STATE_NAME).limit(3).build().unwrap();

    while first.read().unwrap().is_some() {}

    first.state().save().unwrap();
    drop(first);

    let second = items(reader.bytes(&path).state(TEST_STATE_NAME).limit(3).build().unwrap());

    assert_eq!(second, [b"d", b"e", b"f"]);
}

#[rstest]
fn a_resumed_read_ignores_a_start_it_is_already_past(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.resuming();
    let mut first = reader.bytes(&path).state(TEST_STATE_NAME).limit(5).build().unwrap();

    while first.read().unwrap().is_some() {}

    first.state().save().unwrap();
    drop(first);

    let second = take(
        &mut reader.bytes(&path).state(TEST_STATE_NAME).start(2).build().unwrap(),
        1,
    );

    assert_eq!(second, [b"f"]);
}

#[rstest]
fn an_end_below_the_position_does_not_rewind_the_state(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.resuming();
    let mut first = reader.bytes(&path).state(TEST_STATE_NAME).limit(10).build().unwrap();

    while first.read().unwrap().is_some() {}

    first.state().save().unwrap();
    drop(first);

    let mut second = reader.bytes(&path).state(TEST_STATE_NAME).end(4).build().unwrap();

    assert!(second.read().unwrap().is_none());
    assert_eq!(second.state().position, 10);
}

#[rstest]
fn a_fully_consumed_file_yields_nothing_on_resume(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.resuming();
    let mut first = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    while first.read().unwrap().is_some() {}

    first.state().save().unwrap();
    drop(first);

    let mut second = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    assert!(second.read().unwrap().is_none());
    assert_eq!(second.state().position, SIZE);
}

#[rstest]
fn file_growth_during_iteration_is_ignored(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let mut bytes = sandbox.reader().bytes(&path).build().unwrap();

    bytes.read().unwrap();
    sandbox.append(&path, b"0123456789");

    let mut count = 1;

    while bytes.read().unwrap().is_some() {
        count += 1;
    }

    assert_eq!(count, TEST_ALPHABET.len());
    assert_eq!(bytes.state().file.size, SIZE);
}

#[rstest]
fn iteration_survives_the_file_being_deleted(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let mut bytes = sandbox.reader().bytes(&path).build().unwrap();

    bytes.read().unwrap();
    fs::remove_file(&path).unwrap();

    let mut count = 1;

    while bytes.read().unwrap().is_some() {
        count += 1;
    }

    assert_eq!(count, TEST_ALPHABET.len());
}

#[rstest]
fn iteration_stops_at_a_truncation(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.capped(1);
    let mut bytes = reader.bytes(&path).build().unwrap();

    bytes.read().unwrap();
    sandbox.truncate(&path, 4);

    let mut count = 1;

    while bytes.read().unwrap().is_some() {
        count += 1;
    }

    assert_eq!(count, 4);
    assert_eq!(bytes.state().position, 4);
}

#[rstest]
fn clearing_the_registry_does_not_affect_a_live_iterator(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.autosaving(1);
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    bytes.read().unwrap();
    sandbox.states().clear().unwrap();

    assert_eq!(bytes.read().unwrap(), Some(TEST_ALPHABET[1]));
    assert_eq!(sandbox.states().load(TEST_STATE_NAME).unwrap().position, 2);
}

#[rstest]
fn the_recorded_size_never_refreshes(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let mut bytes = sandbox.reader().bytes(&path).build().unwrap();

    sandbox.append(&path, b"more");

    bytes.read().unwrap();

    assert_eq!(bytes.state().file.size, SIZE);
}

#[cfg(unix)]
#[rstest]
fn a_mid_item_read_error_keeps_the_position(sandbox: Sandbox) {
    use chrono::Utc;
    use preader::{FileMetadata, StateData, Timestamps};

    let directory = sandbox.dir_at("folder");
    let data = StateData {
        name: TEST_STATE_NAME.to_owned(),
        file: FileMetadata {
            path: directory.clone(),
            size: 64,
            mtime: Utc::now(),
            fingerprint: String::new(),
        },
        position: 0,
        timestamps: Timestamps::now(),
        checksum: String::new(),
    };
    let state = sandbox.manager().state(data);
    let mut lines = sandbox.lenient().lines(&directory).state(state).build().unwrap();

    assert!(matches!(lines.read().unwrap_err(), Error::Io(_)));
    assert_eq!(lines.state().position, 0);
    assert!(lines.read().is_err());
}

#[rstest]
fn dropping_reports_a_failed_save_without_panicking(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let reader = sandbox.autosaving(0);
    let mut bytes = reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap();

    for _ in 0..3 {
        bytes.read().unwrap();
    }

    block_the_state_dir(&sandbox);
    drop(bytes);

    assert!(!sandbox.states().exists(TEST_STATE_NAME));
}
