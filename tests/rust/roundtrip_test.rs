use preader::{Config, Error, IteratorBuild, IteratorRead};
use rstest::rstest;

use crate::common::{
    fixtures::sandbox,
    funcs::{items, texts},
    macros::{cycle::cycle, drain::drain},
    rng::{seeded_bytes, seeded_text},
    sandbox::Sandbox,
};

const SEEDS: [u64; 3] = [1, 2, 3];
const SIZES: [usize; 3] = [512, 4096, 51_200];
const CYCLES: usize = 5;
const CHUNK: usize = 7;

#[rstest]
fn bytes_reproduce_the_content(sandbox: Sandbox) {
    let reader = sandbox.reader();

    for seed in SEEDS {
        for size in SIZES {
            let content = seeded_bytes(seed, size);
            let path = sandbox.file(&content);

            assert_eq!(
                items(reader.bytes(&path).build().unwrap()).concat(),
                content,
                "seed {seed} size {size}"
            );
        }
    }
}

#[rstest]
#[case::single_byte(1)]
#[case::uneven(CHUNK)]
#[case::page(4096)]
fn chunks_reproduce_the_content(sandbox: Sandbox, #[case] size_of: usize) {
    let reader = sandbox.reader();

    for seed in SEEDS {
        for size in SIZES {
            let content = seeded_bytes(seed, size);
            let path = sandbox.file(&content);

            assert_eq!(
                items(reader.chunks(&path).size(size_of).build().unwrap()).concat(),
                content,
                "seed {seed} size {size}"
            );
        }
    }
}

#[rstest]
#[case::small(1024)]
#[case::default(65_536)]
fn drop_partial_stops_at_the_last_whole_chunk(sandbox: Sandbox, #[case] capacity: usize) {
    let reader = sandbox.capped(capacity);

    for seed in SEEDS {
        for size in SIZES {
            let content = seeded_bytes(seed, size);
            let path = sandbox.file(&content);
            let chunks = reader.chunks(&path).size(CHUNK).drop_partial(true).build().unwrap();

            assert_eq!(
                items(chunks).concat(),
                content[..content.len() - content.len() % CHUNK],
                "seed {seed} size {size}"
            );
        }
    }
}

#[rstest]
fn segments_reproduce_the_content(sandbox: Sandbox) {
    let reader = sandbox.reader();

    for seed in SEEDS {
        for size in SIZES {
            let content = seeded_bytes(seed, size);
            let path = sandbox.file(&content);
            let segments = reader.delimiter(&path).keep(true).build().unwrap();

            assert_eq!(items(segments).concat(), content, "seed {seed} size {size}");
        }
    }
}

#[rstest]
#[case::small(1024)]
#[case::default(65_536)]
fn lines_reproduce_the_text(sandbox: Sandbox, #[case] capacity: usize) {
    let reader = sandbox.capped(capacity);

    for seed in SEEDS {
        for size in SIZES {
            let text = seeded_text(seed, size);
            let path = sandbox.file(text.as_bytes());
            let kept = items(reader.lines(&path).keepends(true).build().unwrap());
            let stripped = items(reader.lines(&path).build().unwrap());

            assert_eq!(texts(&kept).concat(), text);
            assert_eq!(texts(&stripped).concat(), text.replace('\n', ""));
            assert_eq!(stripped.len(), text.matches('\n').count());
        }
    }
}

#[rstest]
fn lines_reject_binary_content(sandbox: Sandbox) {
    let path = sandbox.file(&seeded_bytes(1, 4096));
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
fn skip_empty_drops_the_blank_lines(sandbox: Sandbox) {
    let reader = sandbox.reader();

    for seed in SEEDS {
        let text = seeded_text(seed, 4096);
        let path = sandbox.file(text.as_bytes());
        let expected: Vec<&str> = text.lines().filter(|line| !line.is_empty()).collect();
        let read = items(reader.lines(&path).skip_empty(true).build().unwrap());

        assert_eq!(texts(&read), expected, "seed {seed}");
    }
}

#[rstest]
fn percent_reaches_a_hundred_for_every_iterator(sandbox: Sandbox) {
    let reader = sandbox.reader();
    let text = seeded_text(1, 4096);
    let path = sandbox.file(text.as_bytes());
    let size = text.len() as u64;

    drain!(reader.bytes(&path), size);
    drain!(reader.chunks(&path).size(CHUNK), size);
    drain!(reader.delimiter(&path).keep(true), size);
    drain!(reader.lines(&path).keepends(true), size);
}

#[rstest]
fn a_resume_rebuilds_the_file_in_cycles(sandbox: Sandbox) {
    let plain = sandbox.reader();
    let cycling = sandbox.reader_with(Config {
        auto_load_state: true,
        auto_save_state: true,
        ..sandbox.config()
    });
    let binary = sandbox.write("data.bin", &seeded_bytes(1, 4096));
    let textual = sandbox.write("data.txt", seeded_text(1, 4096).as_bytes());

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
    let content = seeded_bytes(1, 4096);
    let path = sandbox.file(&content);
    let bytes = items(reader.bytes(&path).start(100).end(500).build().unwrap());
    let chunked = items(reader.chunks(&path).size(CHUNK).start(100).end(500).build().unwrap());

    assert_eq!(bytes.concat(), content[100..500]);
    assert_eq!(chunked.concat(), content[100..500]);
}
