use std::{path::PathBuf, time::SystemTime};

use chrono::DateTime;
use preader::{
    Config, DEFAULT_STATE_DIRECTORY, FileMetadata, IteratorBase, IteratorConfig, STATE_FILE_SUFFIX,
    State, StateData, StateManager, Timestamps,
};
use rstest::rstest;
use tempfile::TempDir;

use crate::common::{
    constants::{TEST_FINGERPRINT, TEST_STATE_NAME},
    fixtures::tmp_dir,
};

const FILE_SIZE: u64 = 1_000;
const NO_LIMIT: u64 = u64::MAX;

enum Autosave {
    Off,
    Every(u64),
}

fn config_in(tmp_dir: &TempDir, autosave: Autosave) -> Config {
    let (auto_save_state, auto_save_state_bytes) = match autosave {
        Autosave::Off => (false, 0),
        Autosave::Every(bytes) => (true, bytes),
    };

    Config {
        state_dir: tmp_dir.path().join(DEFAULT_STATE_DIRECTORY),
        auto_save_state,
        auto_save_state_bytes,
        ..Config::default()
    }
}

fn state_file(config: &Config, name: &str) -> PathBuf {
    config.state_dir.join(format!("{name}.{STATE_FILE_SUFFIX}"))
}

fn modified_at(config: &Config) -> SystemTime {
    state_file(config, TEST_STATE_NAME).metadata().unwrap().modified().unwrap()
}

fn state_at(config: &Config, position: u64) -> State {
    let data = StateData {
        name: TEST_STATE_NAME.to_string(),
        file: FileMetadata {
            path: config.state_dir.with_file_name("data.bin"),
            size: FILE_SIZE,
            mtime: DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
            fingerprint: TEST_FINGERPRINT.to_string(),
        },
        position,
        timestamps: Timestamps::now(),
        checksum: "unchecked".to_string(),
    };

    State::from((data, StateManager::from(config)))
}

fn base_at(config: &Config, position: u64, end: u64, limit: u64) -> IteratorBase {
    IteratorBase::new(
        IteratorConfig::from(config),
        state_at(config, position),
        end,
        limit,
    )
}

#[rstest]
#[case::zero_threshold_and_zero_position(0, 0, 0)]
#[case::zero_threshold(0, 150, 150)]
#[case::unit_threshold(1, 7, 7)]
#[case::below_the_first_multiple(10, 9, 0)]
#[case::exactly_a_multiple(10, 10, 10)]
#[case::between_multiples(10, 15, 10)]
#[case::exactly_a_later_multiple(10, 20, 20)]
#[case::between_large_multiples(100, 150, 100)]
#[case::below_a_large_multiple(100, 99, 0)]
fn new_floors_last_saved_position(
    tmp_dir: TempDir,
    #[case] threshold: u64,
    #[case] position: u64,
    #[case] expected: u64,
) {
    let config = config_in(&tmp_dir, Autosave::Every(threshold));
    let base = base_at(&config, position, NO_LIMIT, NO_LIMIT);

    assert_eq!(base.state.manager.last_saved_position, expected);
}

#[rstest]
fn new_starts_with_nothing_yielded(tmp_dir: TempDir) {
    let config = config_in(&tmp_dir, Autosave::Off);
    let base = base_at(&config, 0, NO_LIMIT, NO_LIMIT);

    assert_eq!(base.yielded, 0);
}

#[rstest]
fn new_keeps_the_end_and_limit(tmp_dir: TempDir) {
    let config = config_in(&tmp_dir, Autosave::Off);
    let base = base_at(&config, 0, 42, 7);

    assert_eq!(base.end, 42);
    assert_eq!(base.limit, 7);
}

#[rstest]
#[case::below_both_bounds(5, 10, 10, false)]
#[case::position_reaches_end(10, 10, NO_LIMIT, true)]
#[case::position_passes_end(11, 10, NO_LIMIT, true)]
#[case::zero_end(0, 0, NO_LIMIT, true)]
#[case::zero_limit(0, 100, 0, true)]
fn should_stop_once_either_bound_is_reached(
    tmp_dir: TempDir,
    #[case] position: u64,
    #[case] end: u64,
    #[case] limit: u64,
    #[case] expected: bool,
) {
    let config = config_in(&tmp_dir, Autosave::Off);
    let base = base_at(&config, position, end, limit);

    assert_eq!(base.should_stop(), expected);
}

#[rstest]
fn should_stop_after_yielding_the_limit(tmp_dir: TempDir) {
    let config = config_in(&tmp_dir, Autosave::Off);
    let mut base = base_at(&config, 0, 100, 2);

    base.count_yield();
    base.count_yield();

    assert!(base.should_stop());
}

#[rstest]
fn count_yield_increments_the_counter(tmp_dir: TempDir) {
    let config = config_in(&tmp_dir, Autosave::Off);
    let mut base = base_at(&config, 0, NO_LIMIT, NO_LIMIT);

    base.count_yield();
    base.count_yield();
    base.count_yield();

    assert_eq!(base.yielded, 3);
}

#[rstest]
fn advance_moves_the_position(tmp_dir: TempDir) {
    let config = config_in(&tmp_dir, Autosave::Off);
    let mut base = base_at(&config, 0, NO_LIMIT, NO_LIMIT);

    base.advance(4).unwrap();
    base.advance(6).unwrap();

    assert_eq!(base.state.position, 10);
}

#[rstest]
#[case::autosave_disabled(Autosave::Off, 50, 0)]
#[case::zero_threshold(Autosave::Every(0), 50, 0)]
#[case::below_the_threshold(Autosave::Every(10), 9, 0)]
#[case::reaching_the_threshold(Autosave::Every(10), 10, 10)]
fn advance_saves_only_at_the_threshold(
    tmp_dir: TempDir,
    #[case] autosave: Autosave,
    #[case] advance_by: u64,
    #[case] expected: u64,
) {
    let config = config_in(&tmp_dir, autosave);
    let mut base = base_at(&config, 0, NO_LIMIT, NO_LIMIT);

    base.advance(advance_by).unwrap();

    assert_eq!(base.state.manager.last_saved_position, expected);
}

#[rstest]
fn advance_never_saves_without_autosave(tmp_dir: TempDir) {
    let config = Config {
        state_dir: tmp_dir.path().join(DEFAULT_STATE_DIRECTORY),
        auto_save_state: false,
        auto_save_state_bytes: 1,
        ..Config::default()
    };
    let mut base = base_at(&config, 0, NO_LIMIT, NO_LIMIT);

    base.advance(50).unwrap();

    assert_eq!(base.state.manager.last_saved_position, 0);
}

#[rstest]
fn advance_accumulates_progress(tmp_dir: TempDir) {
    let config = config_in(&tmp_dir, Autosave::Every(10));
    let mut base = base_at(&config, 0, NO_LIMIT, NO_LIMIT);

    base.advance(6).unwrap();
    base.advance(4).unwrap();

    assert_eq!(base.state.manager.last_saved_position, 10);
}

#[rstest]
fn advance_saves_at_each_threshold(tmp_dir: TempDir) {
    let config = config_in(&tmp_dir, Autosave::Every(10));
    let mut base = base_at(&config, 0, NO_LIMIT, NO_LIMIT);

    base.advance(10).unwrap();
    base.advance(9).unwrap();

    assert_eq!(base.state.manager.last_saved_position, 10);

    base.advance(1).unwrap();

    assert_eq!(base.state.manager.last_saved_position, 20);
}

#[rstest]
fn advance_writes_the_state_file(tmp_dir: TempDir) {
    let config = config_in(&tmp_dir, Autosave::Every(10));
    let mut base = base_at(&config, 0, NO_LIMIT, NO_LIMIT);

    base.advance(10).unwrap();

    assert!(state_file(&config, TEST_STATE_NAME).exists());
}

#[rstest]
fn finalize_saves_progress_below_the_threshold(tmp_dir: TempDir) {
    let config = config_in(&tmp_dir, Autosave::Every(1_000));
    let mut base = base_at(&config, 0, NO_LIMIT, NO_LIMIT);

    base.advance(5).unwrap();
    base.finalize().unwrap();

    assert_eq!(base.state.manager.last_saved_position, 5);
}

#[rstest]
#[case::without_progress(Autosave::Every(1_000), 0)]
#[case::autosave_disabled(Autosave::Off, 5)]
fn finalize_writes_nothing(tmp_dir: TempDir, #[case] autosave: Autosave, #[case] advance_by: u64) {
    let config = config_in(&tmp_dir, autosave);
    let mut base = base_at(&config, 0, NO_LIMIT, NO_LIMIT);

    base.advance(advance_by).unwrap();
    base.finalize().unwrap();

    assert!(!state_file(&config, TEST_STATE_NAME).exists());
}

#[rstest]
fn finalize_is_idempotent(tmp_dir: TempDir) {
    let config = config_in(&tmp_dir, Autosave::Every(1_000));
    let mut base = base_at(&config, 0, NO_LIMIT, NO_LIMIT);

    base.advance(5).unwrap();
    base.finalize().unwrap();

    let first = modified_at(&config);

    base.finalize().unwrap();

    let second = modified_at(&config);

    assert_eq!(first, second);
}
