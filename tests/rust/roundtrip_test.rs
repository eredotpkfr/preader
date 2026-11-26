use std::assert_eq;

use preader::{DEFAULT_AUTO_SAVE_STATE_BYTES, DEFAULT_BUFFER_CAPACITY, Error};
use rstest::rstest;
use rstest_reuse::apply;

use crate::common::{
    constants::{TEST_CHUNK_SIZE, TEST_DEFAULT_DELIMITER, TEST_STATE_NAME, TEST_TIMEOUT},
    fixtures::{sandbox, seed},
    funcs::{items, range, take, try_items},
    iterators::{Plan, Reproducer},
    macros::asserts::assert_err_is,
    rng::{Kind, seeded_content},
    sandbox::Sandbox,
    seed::Seed,
    templates::roundtrip::{archives, contents, flows, texts, windows},
};

const CYCLES: usize = 5;
const COMMA: u8 = TEST_DEFAULT_DELIMITER;
const NEWLINE: u8 = b'\n';
const NULL: u8 = b'\0';

fn split_stream(content: &[u8], delimiter: u8, keep: bool) -> Vec<Vec<u8>> {
    let mut segments: Vec<Vec<u8>> =
        content.split(|byte| *byte == delimiter).map(<[u8]>::to_vec).collect();
    let tail = segments.pop().unwrap();

    if keep {
        segments.iter_mut().for_each(|segment| segment.push(delimiter));
    }

    if !tail.is_empty() {
        segments.push(tail);
    }

    segments
}

#[rstest]
#[case::little_endian("café,鉄".encode_utf16().flat_map(u16::to_le_bytes).collect(), TEST_DEFAULT_DELIMITER)]
#[case::big_endian("café,鉄".encode_utf16().flat_map(u16::to_be_bytes).collect(), TEST_DEFAULT_DELIMITER)]
#[case::newline_inside_a_character("ਊ".encode_utf16().flat_map(u16::to_le_bytes).collect(), b'\n')]
fn segments_split_utf16_on_bytes(
    sandbox: Sandbox,
    #[case] content: Vec<u8>,
    #[case] delimiter: u8,
) {
    let path = sandbox.file(&content);
    let segments = items(sandbox.reader().segments(&path).delimiter(delimiter).build().unwrap());

    assert_eq!(segments, split_stream(&content, delimiter, false));
}

#[apply(windows)]
fn iterators_reproduce_the_content(
    sandbox: Sandbox,
    seed: Seed,
    #[case] reproducer: Reproducer,
    #[case] kind: Kind,
    #[case] size: usize,
) {
    let content = seeded_content(kind, size, seed);
    let read = reproducer.open(&sandbox.reader(), &sandbox.file(&content), Plan::default());

    assert_eq!(items(read).concat(), content, "{seed}");
}

#[apply(archives)]
fn lines_fails_when_the_content_is_binary(
    sandbox: Sandbox,
    seed: Seed,
    #[case] kind: Kind,
    #[case] size: usize,
) {
    let path = sandbox.file(&seeded_content(kind, size, seed));
    let lines = sandbox.reader().lines(&path).build().unwrap();

    assert_err_is!(try_items(lines), Error::Utf8(_), "{seed}");
}

#[apply(contents)]
fn chunks_reproduce_the_content(
    sandbox: Sandbox,
    seed: Seed,
    #[case] kind: Kind,
    #[case] size: usize,
    #[values(1, 4096)] chunk_size: usize,
) {
    let content = seeded_content(kind, size, seed);
    let path = sandbox.file(&content);
    let chunks = sandbox.reader().chunks(&path).size(chunk_size).build().unwrap();

    assert_eq!(items(chunks).concat(), content, "{seed}");
}

#[apply(contents)]
fn drop_partial_stops_at_the_last_whole_chunk(
    sandbox: Sandbox,
    seed: Seed,
    #[case] kind: Kind,
    #[case] size: usize,
    #[values(1024, DEFAULT_BUFFER_CAPACITY)] capacity: usize,
) {
    let content = seeded_content(kind, size, seed);
    let path = sandbox.file(&content);
    let chunks = sandbox
        .capped(capacity)
        .chunks(&path)
        .size(TEST_CHUNK_SIZE)
        .drop_partial(true)
        .build()
        .unwrap();

    assert_eq!(
        items(chunks).concat(),
        content[..content.len() - content.len() % TEST_CHUNK_SIZE],
        "{seed}"
    );
}

#[apply(contents)]
fn segments_skip_empty_drops_the_blank_segments(
    sandbox: Sandbox,
    seed: Seed,
    #[case] kind: Kind,
    #[case] size: usize,
) {
    let content = seeded_content(kind, size, seed);
    let path = sandbox.file(&content);
    let kept = items(sandbox.reader().segments(&path).skip_empty(true).build().unwrap());
    let mut expected = split_stream(&content, TEST_DEFAULT_DELIMITER, false);

    expected.retain(|segment| !segment.is_empty());

    assert_eq!(kept, expected, "{seed}");
}

#[apply(windows)]
fn options_window_the_byte_range(
    sandbox: Sandbox,
    seed: Seed,
    #[case] reproducer: Reproducer,
    #[case] kind: Kind,
    #[case] size: usize,
) {
    let content = seeded_content(kind, size, seed);

    let plan = Plan {
        options: range(100, 500),
        ..Plan::default()
    };

    let windowed = reproducer.open(&sandbox.reader(), &sandbox.file(&content), plan);
    let end = content.len().min(500);

    assert_eq!(
        items(windowed).concat(),
        content[end.min(100)..end],
        "{seed}"
    );
}

#[apply(texts)]
fn skip_empty_drops_the_blank_lines(
    sandbox: Sandbox,
    seed: Seed,
    #[case] kind: Kind,
    #[case] size: usize,
) {
    let content = seeded_content(kind, size, seed);
    let path = sandbox.file(&content);
    let text = String::from_utf8(content).unwrap();
    let expected: Vec<&str> = text.lines().filter(|line| !line.is_empty()).collect();

    assert_eq!(
        items(sandbox.reader().lines(&path).skip_empty(true).build().unwrap()),
        expected,
        "{seed}"
    );
}

#[apply(contents)]
fn segments_split_like_a_stream(
    sandbox: Sandbox,
    seed: Seed,
    #[case] kind: Kind,
    #[case] size: usize,
    #[values(COMMA, NEWLINE, NULL)] delimiter: u8,
    #[values(1024, DEFAULT_BUFFER_CAPACITY)] capacity: usize,
) {
    let content = seeded_content(kind, size, seed);
    let path = sandbox.file(&content);
    let reader = sandbox.capped(capacity);

    let stripped = items(reader.segments(&path).delimiter(delimiter).build().unwrap());
    let kept = items(reader.segments(&path).delimiter(delimiter).keep(true).build().unwrap());

    assert_eq!(stripped, split_stream(&content, delimiter, false), "{seed}");
    assert_eq!(kept, split_stream(&content, delimiter, true), "{seed}");
}

#[apply(texts)]
fn lines_reproduce_the_text(
    sandbox: Sandbox,
    seed: Seed,
    #[case] kind: Kind,
    #[case] size: usize,
    #[values(1024, DEFAULT_BUFFER_CAPACITY)] capacity: usize,
) {
    let content = seeded_content(kind, size, seed);
    let path = sandbox.file(&content);
    let text = String::from_utf8(content).unwrap();
    let reader = sandbox.capped(capacity);

    let kept = items(reader.lines(&path).keepends(true).build().unwrap());
    let stripped = items(reader.lines(&path).build().unwrap());

    assert_eq!(kept.concat(), text, "{seed}");
    assert_eq!(
        stripped,
        text.split_terminator('\n').collect::<Vec<_>>(),
        "{seed}"
    );
}

#[apply(flows)]
#[timeout(TEST_TIMEOUT)]
fn percent_tracks_the_position(
    sandbox: Sandbox,
    seed: Seed,
    #[case] reproducer: Reproducer,
    #[case] kind: Kind,
    #[case] size: usize,
) {
    let content = seeded_content(kind, size, seed);

    let mut iterator = reproducer.open(&sandbox.reader(), &sandbox.file(&content), Plan::default());

    while let Some(item) = iterator.next() {
        item.unwrap();

        let expected = iterator.state().position as f64 * 100.0 / content.len() as f64;

        assert_eq!(iterator.percent(), expected, "{seed}");
    }

    assert_eq!(iterator.state().position, content.len() as u64, "{seed}");
    assert_eq!(iterator.percent(), 100.0, "{seed}");
}

#[apply(contents)]
fn segments_windows_cover_the_content_once(
    sandbox: Sandbox,
    seed: Seed,
    #[case] kind: Kind,
    #[case] size: usize,
    #[values(COMMA, NEWLINE, NULL)] delimiter: u8,
) {
    let content = seeded_content(kind, size, seed);
    let path = sandbox.file(&content);
    let length = content.len() as u64;
    let cuts = [0, length / 3, length * 2 / 3, length];

    let segments: Vec<Vec<u8>> = cuts
        .windows(2)
        .flat_map(|cut| {
            let segments = sandbox
                .reader()
                .segments(&path)
                .options(range(cut[0], cut[1]))
                .delimiter(delimiter)
                .keep(true)
                .align(cut[0] > 0)
                .build()
                .unwrap();

            items(segments)
        })
        .collect();

    assert_eq!(segments.concat(), content, "{seed}");
}

#[apply(flows)]
#[timeout(TEST_TIMEOUT)]
fn resume_rebuilds_the_file_in_cycles(
    sandbox: Sandbox,
    seed: Seed,
    #[case] reproducer: Reproducer,
    #[case] kind: Kind,
    #[case] size: usize,
) {
    let path = sandbox.file(&seeded_content(kind, size, seed));
    let autosaving = sandbox.autosaving(DEFAULT_AUTO_SAVE_STATE_BYTES.get());

    let expected = items(reproducer.open(&sandbox.reader(), &path, Plan::default()));
    let cycle = expected.len().div_ceil(CYCLES - 1) + 1;

    let named = || Plan {
        state: TEST_STATE_NAME.into(),
        ..Plan::default()
    };

    let rebuilt: Vec<Vec<u8>> = (0..CYCLES)
        .flat_map(|_| take(&mut reproducer.open(&autosaving, &path, named()), cycle))
        .collect();
    let overlapping: Vec<Vec<u8>> = (0..CYCLES)
        .flat_map(|index| expected.iter().skip(index * (cycle - 1)).take(cycle).cloned())
        .collect();

    assert_eq!(rebuilt, overlapping, "{seed}");
}
