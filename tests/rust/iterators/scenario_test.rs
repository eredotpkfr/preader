use std::{
    fs::{self, OpenOptions},
    io::Write,
};

use preader::{Config, Error, IteratorBuild, IteratorRead};
use rstest::rstest;
use tempfile::TempDir;

use crate::common::{
    constants::TEST_STATE_NAME,
    fixtures::tmp_dir,
    funcs::{reader, write},
};

const LINES: &[u8] = b"alpha\nbeta\ngamma\ndelta\n";

fn resuming(tmp_dir: &TempDir) -> Config {
    Config {
        auto_save_state: true,
        auto_save_state_bytes: 0,
        auto_load_state: true,
        state_dir: tmp_dir.path().join("states"),
        ..Config::default()
    }
}

#[rstest]
#[case(1)]
#[case(2)]
#[case(3)]
#[case(4096)]
fn a_tiny_buffer_capacity_does_not_change_the_output(tmp_dir: TempDir, #[case] capacity: usize) {
    let config = Config {
        buffer_capacity: capacity,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.txt", LINES);

    let mut lines = reader.lines(&path).build().unwrap();
    let mut collected = Vec::new();

    while let Some(line) = lines.read().unwrap() {
        collected.push(line.to_owned());
    }

    assert_eq!(collected, ["alpha", "beta", "gamma", "delta"]);

    let mut chunks = reader.chunks(&path).size(5).build().unwrap();
    let mut sizes = Vec::new();

    while let Some(chunk) = chunks.read().unwrap() {
        sizes.push(chunk.len());
    }

    assert_eq!(sizes, [5, 5, 5, 5, 3]);
}

#[rstest]
fn a_zero_buffer_capacity_still_reads_every_item(tmp_dir: TempDir) {
    let config = Config {
        buffer_capacity: 0,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.txt", LINES);
    let mut lines = reader.lines(&path).build().unwrap();
    let mut collected = Vec::new();

    while let Some(line) = lines.read().unwrap() {
        collected.push(line.to_owned());
    }

    assert_eq!(collected, ["alpha", "beta", "gamma", "delta"]);
}

#[rstest]
fn iteration_stops_at_a_truncation(tmp_dir: TempDir) {
    let config = Config {
        buffer_capacity: 1,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.txt", LINES);
    let mut lines = reader.lines(&path).build().unwrap();

    assert_eq!(lines.read().unwrap(), Some("alpha"));

    write(&tmp_dir, "data.txt", b"al");

    assert_eq!(lines.read().unwrap(), None);
    assert_eq!(lines.state().position, 6);
}

#[rstest]
fn iteration_survives_the_file_being_deleted(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.txt", LINES);
    let mut lines = reader.lines(&path).build().unwrap();

    assert_eq!(lines.read().unwrap(), Some("alpha"));

    fs::remove_file(&path).unwrap();

    let mut collected = Vec::new();

    while let Some(line) = lines.read().unwrap() {
        collected.push(line.to_owned());
    }

    assert_eq!(collected, ["beta", "gamma", "delta"]);
}

#[rstest]
fn file_growth_during_iteration_is_ignored(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.txt", LINES);
    let mut lines = reader.lines(&path).build().unwrap();

    assert_eq!(lines.read().unwrap(), Some("alpha"));

    let mut handle = OpenOptions::new().append(true).open(&path).unwrap();

    handle.write_all(b"epsilon\nzeta\n").unwrap();
    handle.flush().unwrap();

    let mut collected = Vec::new();

    while let Some(line) = lines.read().unwrap() {
        collected.push(line.to_owned());
    }

    assert_eq!(collected, ["beta", "gamma", "delta"]);
    assert_eq!(lines.state().file.size, LINES.len() as u64);
}

#[rstest]
fn a_second_iteration_of_a_resumed_state_yields_nothing(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, resuming(&tmp_dir));
    let path = write(&tmp_dir, "data.txt", LINES);

    {
        let mut lines = reader.lines(&path).state(TEST_STATE_NAME).build().unwrap();

        while lines.read().unwrap().is_some() {}
    }

    let mut again = reader.lines(&path).state(TEST_STATE_NAME).build().unwrap();

    assert_eq!(again.read().unwrap(), None);
    assert_eq!(again.state().position, LINES.len() as u64);
}

#[rstest]
fn a_resume_ignores_start_once_the_position_is_past_it(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, resuming(&tmp_dir));
    let path = write(&tmp_dir, "data.txt", LINES);

    {
        let mut lines = reader.lines(&path).state(TEST_STATE_NAME).limit(2).build().unwrap();

        while lines.read().unwrap().is_some() {}
    }

    let mut resumed = reader.lines(&path).state(TEST_STATE_NAME).start(0).build().unwrap();
    let mut collected = Vec::new();

    while let Some(line) = resumed.read().unwrap() {
        collected.push(line.to_owned());
    }

    assert_eq!(collected, ["gamma", "delta"]);
}

#[rstest]
fn an_end_below_the_position_does_not_rewind_the_state(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, resuming(&tmp_dir));
    let path = write(&tmp_dir, "data.txt", LINES);

    {
        let mut lines = reader.lines(&path).state(TEST_STATE_NAME).limit(2).build().unwrap();

        while lines.read().unwrap().is_some() {}
    }

    let mut resumed = reader.lines(&path).state(TEST_STATE_NAME).end(3).build().unwrap();

    assert_eq!(resumed.read().unwrap(), None);
    assert_eq!(resumed.state().position, 11);
}

#[rstest]
fn a_resume_applies_the_limit_again(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, resuming(&tmp_dir));
    let path = write(&tmp_dir, "data.txt", LINES);

    for expected in [["alpha", "beta"], ["gamma", "delta"]] {
        let mut lines = reader.lines(&path).state(TEST_STATE_NAME).limit(2).build().unwrap();
        let mut collected = Vec::new();

        while let Some(line) = lines.read().unwrap() {
            collected.push(line.to_owned());
        }

        assert_eq!(collected, expected);
    }
}

#[rstest]
fn a_resume_skips_again_when_the_position_equals_the_start(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, resuming(&tmp_dir));
    let path = write(&tmp_dir, "data.txt", LINES);

    {
        let mut lines = reader.lines(&path).state(TEST_STATE_NAME).limit(0).build().unwrap();

        while lines.read().unwrap().is_some() {}
    }

    let mut resumed = reader.lines(&path).state(TEST_STATE_NAME).skip(1).build().unwrap();

    assert_eq!(resumed.state().position, 0);
    assert_eq!(resumed.read().unwrap(), Some("beta"));
}

#[rstest]
fn two_states_for_the_same_file_advance_independently(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, resuming(&tmp_dir));
    let path = write(&tmp_dir, "data.txt", LINES);

    {
        let mut first = reader.lines(&path).state("first").limit(1).build().unwrap();

        while first.read().unwrap().is_some() {}
    }

    let mut second = reader.lines(&path).state("second").build().unwrap();

    assert_eq!(second.read().unwrap(), Some("alpha"));

    let mut first = reader.lines(&path).state("first").build().unwrap();

    assert_eq!(first.read().unwrap(), Some("beta"));
}

#[rstest]
fn a_threshold_auto_save_records_every_item_boundary(tmp_dir: TempDir) {
    let config = Config {
        auto_save_state: true,
        auto_save_state_bytes: 1,
        ..resuming(&tmp_dir)
    };
    let reader = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.txt", LINES);
    let mut lines = reader.lines(&path).state(TEST_STATE_NAME).build().unwrap();
    let mut positions = Vec::new();

    while lines.read().unwrap().is_some() {
        positions.push(reader.states().load(TEST_STATE_NAME).unwrap().position);
    }

    assert_eq!(positions, [6, 11, 17, 23]);
}

#[rstest]
fn lines_read_multi_byte_characters_and_count_bytes(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.txt", "çğü\nüğç\n".as_bytes());
    let mut lines = reader.lines(&path).build().unwrap();
    let mut collected = Vec::new();

    while let Some(line) = lines.read().unwrap() {
        collected.push(line.to_owned());
    }

    assert_eq!(collected, ["çğü", "üğç"]);
    assert_eq!(lines.state().position, 14);
}

#[rstest]
fn lines_fail_when_a_skipped_item_is_invalid_utf8(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.txt", b"\xff\xfe\nok\n");
    let mut lines = reader.lines(&path).skip(1).build().unwrap();
    let error = lines.read().err().unwrap();

    assert!(matches!(error, Error::Io(_)), "unexpected error: {error}");
}

#[rstest]
fn mixed_line_endings_are_both_stripped(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.txt", b"a\r\nb\nc\r\n");
    let mut lines = reader.lines(&path).build().unwrap();
    let mut collected = Vec::new();

    while let Some(line) = lines.read().unwrap() {
        collected.push(line.to_owned());
    }

    assert_eq!(collected, ["a", "b", "c"]);
}

#[rstest]
fn the_default_chunk_size_is_one_kibibyte(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", &[7_u8; 3000]);
    let mut chunks = reader.chunks(&path).build().unwrap();
    let mut sizes = Vec::new();

    while let Some(chunk) = chunks.read().unwrap() {
        sizes.push(chunk.len());
    }

    assert_eq!(sizes, [1024, 1024, 952]);
}

#[rstest]
fn a_chunk_skip_saturates_instead_of_overflowing(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", LINES);
    let mut chunks = reader.chunks(&path).size(1 << 20).skip(u64::MAX).build().unwrap();

    assert_eq!(chunks.read().unwrap(), None);
}

#[rstest]
fn a_chunk_start_is_not_realigned_to_the_chunk_grid(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", b"0123456789");
    let mut chunks = reader.chunks(&path).size(4).start(3).build().unwrap();
    let mut collected = Vec::new();

    while let Some(chunk) = chunks.read().unwrap() {
        collected.push(chunk.to_vec());
    }

    assert_eq!(collected, [b"3456".to_vec(), b"789".to_vec()]);
}

#[rstest]
fn a_resume_honours_a_different_chunk_size(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, resuming(&tmp_dir));
    let path = write(&tmp_dir, "data.bin", b"0123456789");

    {
        let mut chunks =
            reader.chunks(&path).size(4).limit(1).state(TEST_STATE_NAME).build().unwrap();

        while chunks.read().unwrap().is_some() {}
    }

    let mut resumed = reader.chunks(&path).size(3).state(TEST_STATE_NAME).build().unwrap();
    let mut collected = Vec::new();

    while let Some(chunk) = resumed.read().unwrap() {
        collected.push(chunk.to_vec());
    }

    assert_eq!(collected, [b"456".to_vec(), b"789".to_vec()]);
}

#[rstest]
fn drop_partial_reads_past_a_buffer_refill(tmp_dir: TempDir) {
    let config = Config {
        buffer_capacity: 4,
        ..Config::default()
    };
    let reader = reader(&tmp_dir, config);
    let path = write(&tmp_dir, "data.bin", &[9_u8; 30]);
    let mut chunks = reader.chunks(&path).size(10).drop_partial(true).build().unwrap();
    let mut sizes = Vec::new();

    while let Some(chunk) = chunks.read().unwrap() {
        sizes.push(chunk.len());
    }

    assert_eq!(sizes, [10, 10, 10]);
}

#[rstest]
fn a_null_byte_delimiter_splits_the_content(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", b"/a/b\0/c/d\0");
    let mut segments = reader.delimiter(&path).character(0).build().unwrap();
    let mut collected = Vec::new();

    while let Some(segment) = segments.read().unwrap() {
        collected.push(segment.to_vec());
    }

    assert_eq!(collected, [b"/a/b".to_vec(), b"/c/d".to_vec()]);
}

#[rstest]
fn a_delimiter_only_file_yields_blank_segments(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", b";;;");

    for (keep, expected) in [
        (false, vec![vec![], vec![], vec![]]),
        (true, vec![b";".to_vec(); 3]),
    ] {
        let mut segments = reader.delimiter(&path).character(b';').keep(keep).build().unwrap();
        let mut collected = Vec::new();

        while let Some(segment) = segments.read().unwrap() {
            collected.push(segment.to_vec());
        }

        assert_eq!(collected, expected, "keep={keep}");
    }
}

#[rstest]
fn a_resume_honours_a_different_delimiter(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, resuming(&tmp_dir));
    let path = write(&tmp_dir, "data.bin", b"a,b;c,d;");

    {
        let mut segments = reader
            .delimiter(&path)
            .character(b',')
            .limit(1)
            .state(TEST_STATE_NAME)
            .build()
            .unwrap();

        while segments.read().unwrap().is_some() {}
    }

    let mut resumed =
        reader.delimiter(&path).character(b';').state(TEST_STATE_NAME).build().unwrap();
    let mut collected = Vec::new();

    while let Some(segment) = resumed.read().unwrap() {
        collected.push(segment.to_vec());
    }

    assert_eq!(collected, [b"b".to_vec(), b"c,d".to_vec()]);
}

#[rstest]
fn every_byte_value_survives_a_round_trip(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let content: Vec<u8> = (0..=255).collect();
    let path = write(&tmp_dir, "data.bin", &content);

    let mut bytes = reader.bytes(&path).build().unwrap();
    let mut collected = Vec::new();

    while let Some(byte) = bytes.read().unwrap() {
        collected.push(byte);
    }

    assert_eq!(collected, content);

    let mut chunks = reader.chunks(&path).size(7).build().unwrap();
    let mut rebuilt = Vec::new();

    while let Some(chunk) = chunks.read().unwrap() {
        rebuilt.extend_from_slice(chunk);
    }

    assert_eq!(rebuilt, content);
}
