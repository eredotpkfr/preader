use preader::{
    Config, DEFAULT_BUFFER_CAPACITY, Error, IteratorBuild, IteratorOptions, IteratorRead,
};
use rstest::rstest;

use crate::common::{
    constants::{TEST_FILE_NAME, TEST_TIMEOUT},
    fixtures::sandbox,
    funcs::items,
    macros::{asserts::assert_percent, cycle::cycle},
    rng::{seeded_bytes, seeded_text},
    sandbox::Sandbox,
};

const CYCLES: usize = 5;
const CHUNK: usize = 7;
const SAMPLE: usize = 4096;

#[rstest]
fn bytes_reproduce_the_content(
    sandbox: Sandbox,
    #[values(1, 2, 3)] seed: u64,
    #[values(512, SAMPLE, 51_200)] size: usize,
) {
    let content = seeded_bytes(seed, size);
    let path = sandbox.file(&content);

    assert_eq!(
        items(sandbox.reader().bytes(&path).build().unwrap()),
        content
    );
}

#[rstest]
fn chunks_reproduce_the_content(
    sandbox: Sandbox,
    #[values(1, 2, 3)] seed: u64,
    #[values(512, SAMPLE, 51_200)] size: usize,
    #[values(1, CHUNK, SAMPLE)] size_of: usize,
) {
    let content = seeded_bytes(seed, size);
    let path = sandbox.file(&content);
    let chunks = sandbox.reader().chunks(&path).size(size_of).build().unwrap();

    assert_eq!(items(chunks).concat(), content);
}

#[rstest]
fn segments_reproduce_the_content(
    sandbox: Sandbox,
    #[values(1, 2, 3)] seed: u64,
    #[values(512, SAMPLE, 51_200)] size: usize,
) {
    let content = seeded_bytes(seed, size);
    let path = sandbox.file(&content);
    let segments = sandbox.reader().delimiter(&path).keep(true).build().unwrap();

    assert_eq!(items(segments).concat(), content);
}

#[rstest]
fn drop_partial_stops_at_the_last_whole_chunk(
    sandbox: Sandbox,
    #[values(1, 2, 3)] seed: u64,
    #[values(512, SAMPLE, 51_200)] size: usize,
    #[values(1024, DEFAULT_BUFFER_CAPACITY)] capacity: usize,
) {
    let content = seeded_bytes(seed, size);
    let path = sandbox.file(&content);
    let chunks = sandbox
        .capped(capacity)
        .chunks(&path)
        .size(CHUNK)
        .drop_partial(true)
        .build()
        .unwrap();

    assert_eq!(
        items(chunks).concat(),
        content[..content.len() - content.len() % CHUNK]
    );
}

#[rstest]
fn lines_reproduce_the_text(
    sandbox: Sandbox,
    #[values(1, 2, 3)] seed: u64,
    #[values(512, SAMPLE, 51_200)] size: usize,
    #[values(1024, DEFAULT_BUFFER_CAPACITY)] capacity: usize,
) {
    let text = seeded_text(seed, size);
    let path = sandbox.file(text.as_bytes());
    let reader = sandbox.capped(capacity);
    let kept = items(reader.lines(&path).keepends(true).build().unwrap());
    let stripped = items(reader.lines(&path).build().unwrap());

    assert_eq!(kept.concat(), text);
    assert_eq!(stripped.concat(), text.replace('\n', ""));
    assert_eq!(stripped.len(), text.matches('\n').count());
}

#[rstest]
fn skip_empty_drops_the_blank_lines(sandbox: Sandbox, #[values(1, 2, 3)] seed: u64) {
    let text = seeded_text(seed, SAMPLE);
    let path = sandbox.file(text.as_bytes());
    let expected: Vec<&str> = text.lines().filter(|line| !line.is_empty()).collect();
    let read = items(sandbox.reader().lines(&path).skip_empty(true).build().unwrap());

    assert_eq!(read, expected);
}

#[rstest]
fn lines_fail_when_the_content_is_binary(sandbox: Sandbox) {
    let path = sandbox.file(&seeded_bytes(1, SAMPLE));
    let mut lines = sandbox.reader().lines(&path).build().unwrap();
    let mut error = None;

    while error.is_none() {
        match lines.read() {
            Ok(Some(_)) => {}
            Ok(None) => break,
            Err(reported) => error = Some(reported),
        }
    }

    assert!(matches!(error.unwrap(), Error::Utf8(_)));
}

#[rstest]
#[timeout(TEST_TIMEOUT)]
fn percent_reaches_hundred_for_every_iterator(sandbox: Sandbox) {
    let reader = sandbox.reader();
    let text = seeded_text(1, SAMPLE);
    let path = sandbox.file(text.as_bytes());
    let size = text.len() as u64;

    assert_percent!(reader.bytes(&path), size);
    assert_percent!(reader.chunks(&path).size(CHUNK), size);
    assert_percent!(reader.delimiter(&path).keep(true), size);
    assert_percent!(reader.lines(&path).keepends(true), size);
}

#[rstest]
#[timeout(TEST_TIMEOUT)]
fn resume_rebuilds_the_file_in_cycles(sandbox: Sandbox) {
    let plain = sandbox.reader();
    let cycling = sandbox.reader_with(Config {
        auto_load_state: true,
        auto_save_state: true,
        ..sandbox.config()
    });
    let binary = sandbox.write(TEST_FILE_NAME, &seeded_bytes(1, SAMPLE));
    let textual = sandbox.write("data.txt", seeded_text(1, SAMPLE).as_bytes());

    cycle!(
        CYCLES,
        "bytes",
        plain.bytes(&binary),
        cycling.bytes(&binary).state("bytes")
    );
    cycle!(
        CYCLES,
        "chunks",
        plain.chunks(&binary).size(CHUNK),
        cycling.chunks(&binary).size(CHUNK).state("chunks")
    );
    cycle!(
        CYCLES,
        "delimiter",
        plain.delimiter(&binary).keep(true),
        cycling.delimiter(&binary).keep(true).state("delimiter")
    );
    cycle!(
        CYCLES,
        "lines",
        plain.lines(&textual).keepends(true),
        cycling.lines(&textual).keepends(true).state("lines")
    );
}

#[rstest]
fn options_window_the_byte_range(sandbox: Sandbox) {
    let reader = sandbox.reader();
    let content = seeded_bytes(1, SAMPLE);
    let path = sandbox.file(&content);
    let options = IteratorOptions {
        start: 100,
        end: 500,
        ..IteratorOptions::default()
    };
    let bytes = items(reader.bytes(&path).options(options).build().unwrap());
    let chunked = items(reader.chunks(&path).size(CHUNK).options(options).build().unwrap());

    assert_eq!(bytes, content[options.start as usize..options.end as usize]);
    assert_eq!(chunked.concat(), bytes);
}
