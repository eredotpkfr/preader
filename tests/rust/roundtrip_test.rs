use preader::{Config, IteratorBuild, IteratorRead, PReader};
use rstest::rstest;
use tempfile::TempDir;

use crate::common::{
    fixtures::tmp_dir,
    funcs::{reader, write},
};

const CYCLES: usize = 5;
const SEEDS: [u64; 3] = [1, 2, 3];
const SIZES: [usize; 3] = [512, 4096, 51_200];
const WORDS: [&str; 12] = [
    "preader",
    "iterator",
    "state",
    "foo",
    "bar",
    "baz",
    "café",
    "Ünicode",
    "日本語",
    "🦀",
    "a;b",
    ",",
];

// A deterministic stream, so a failure reproduces from the seed alone.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

fn seeded_bytes(seed: u64, size: usize) -> Vec<u8> {
    let mut rng = Rng::new(seed);

    (0..size).map(|_| (rng.next() & 0xFF) as u8).collect()
}

fn seeded_text(seed: u64, size: usize) -> String {
    let mut rng = Rng::new(seed);
    let mut text = String::new();

    while text.len() < size {
        let count = rng.next() % 6;

        for word in 0..count {
            if word > 0 {
                text.push(' ');
            }

            text.push_str(WORDS[(rng.next() % WORDS.len() as u64) as usize]);
        }

        text.push('\n');
    }

    text
}

fn collect<I: IteratorRead>(mut iterator: I) -> Vec<I::Owned> {
    let mut collected = Vec::new();

    while let Some(item) = iterator.read().unwrap() {
        collected.push(item.into());
    }

    collected
}

fn take<I: IteratorRead>(iterator: &mut I, count: usize, sink: &mut Vec<I::Owned>) {
    for _ in 0..count {
        match iterator.read().unwrap() {
            Some(item) => sink.push(item.into()),
            None => return,
        }
    }
}

fn resuming(tmp_dir: &TempDir) -> PReader {
    reader(
        tmp_dir,
        Config {
            auto_load_state: true,
            auto_save_state: true,
            ..Config::default()
        },
    )
}

#[rstest]
fn bytes_reproduce_the_content(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());

    for seed in SEEDS {
        for size in SIZES {
            let content = seeded_bytes(seed, size);
            let path = write(&tmp_dir, "data.bin", &content);

            assert_eq!(
                collect(reader.bytes(&path).build().unwrap()),
                content,
                "seed {seed} size {size}"
            );
        }
    }
}

#[rstest]
#[case(1)]
#[case(7)]
#[case(4096)]
fn chunks_reproduce_the_content(tmp_dir: TempDir, #[case] size_of: usize) {
    let reader = reader(&tmp_dir, Config::default());

    for seed in SEEDS {
        for size in SIZES {
            let content = seeded_bytes(seed, size);
            let path = write(&tmp_dir, "data.bin", &content);
            let rebuilt: Vec<u8> = collect(reader.chunks(&path).size(size_of).build().unwrap())
                .into_iter()
                .flatten()
                .collect();

            assert_eq!(rebuilt, content, "seed {seed} size {size}");
        }
    }
}

#[rstest]
#[case(1024)]
#[case(65_536)]
fn drop_partial_stops_at_the_last_whole_chunk(tmp_dir: TempDir, #[case] capacity: usize) {
    let reader = reader(
        &tmp_dir,
        Config {
            buffer_capacity: capacity,
            ..Config::default()
        },
    );

    for seed in SEEDS {
        for size in SIZES {
            let content = seeded_bytes(seed, size);
            let path = write(&tmp_dir, "data.bin", &content);
            let chunks = reader.chunks(&path).size(7).drop_partial(true).build().unwrap();
            let rebuilt: Vec<u8> = collect(chunks).into_iter().flatten().collect();

            assert_eq!(rebuilt, content[..content.len() - content.len() % 7]);
        }
    }
}

#[rstest]
fn delimiter_segments_reproduce_the_content(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());

    for seed in SEEDS {
        for size in SIZES {
            let content = seeded_bytes(seed, size);
            let path = write(&tmp_dir, "data.bin", &content);
            let segments = reader.delimiter(&path).keep(true).build().unwrap();
            let rebuilt: Vec<u8> = collect(segments).into_iter().flatten().collect();

            assert_eq!(rebuilt, content, "seed {seed} size {size}");
        }
    }
}

#[rstest]
#[case(1024)]
#[case(65_536)]
fn lines_reproduce_the_text(tmp_dir: TempDir, #[case] capacity: usize) {
    let reader = reader(
        &tmp_dir,
        Config {
            buffer_capacity: capacity,
            ..Config::default()
        },
    );

    for seed in SEEDS {
        for size in SIZES {
            let text = seeded_text(seed, size);
            let path = write(&tmp_dir, "data.txt", text.as_bytes());

            assert_eq!(
                collect(reader.lines(&path).keepends(true).build().unwrap()).concat(),
                text
            );
            assert_eq!(
                collect(reader.lines(&path).build().unwrap()).concat(),
                text.replace('\n', "")
            );
            assert_eq!(
                collect(reader.lines(&path).build().unwrap()).len(),
                text.matches('\n').count()
            );
        }
    }
}

#[rstest]
fn lines_reject_binary_content(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", &seeded_bytes(1, 4096));
    let mut lines = reader.lines(&path).build().unwrap();
    let mut error = None;

    while error.is_none() {
        match lines.read() {
            Ok(Some(_)) => {}
            Ok(None) => break,
            Err(reported) => error = Some(reported),
        }
    }

    let error = error.expect("binary content produced no error");

    assert!(error.to_string().contains("valid UTF-8"), "got {error}");
}

#[rstest]
fn skip_empty_drops_the_blank_lines(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());

    for seed in SEEDS {
        let text = seeded_text(seed, 4096);
        let path = write(&tmp_dir, "data.txt", text.as_bytes());
        let expected: Vec<&str> = text.lines().filter(|line| !line.is_empty()).collect();

        assert_eq!(
            collect(reader.lines(&path).skip_empty(true).build().unwrap()),
            expected
        );
    }
}

#[rstest]
fn percent_reaches_a_hundred_for_every_iterator(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let text = seeded_text(1, 4096);
    let path = write(&tmp_dir, "data.txt", text.as_bytes());
    let size = text.len() as u64;

    macro_rules! drain {
        ($builder:expr) => {{
            let mut iterator = $builder.build().unwrap();

            while iterator.read().unwrap().is_some() {
                assert!(iterator.state().position <= size);
            }

            assert_eq!(iterator.state().position, size);
            assert_eq!(iterator.state().percent(), 100.0);
        }};
    }

    drain!(reader.bytes(&path));
    drain!(reader.chunks(&path).size(7));
    drain!(reader.delimiter(&path).keep(true));
    drain!(reader.lines(&path).keepends(true));
}

#[rstest]
fn a_resume_rebuilds_the_file_in_cycles(tmp_dir: TempDir) {
    let plain = reader(&tmp_dir, Config::default());
    let cycling = resuming(&tmp_dir);
    let content = seeded_bytes(1, 4096);
    let text = seeded_text(1, 4096);
    let binary = write(&tmp_dir, "data.bin", &content);
    let textual = write(&tmp_dir, "data.txt", text.as_bytes());

    macro_rules! cycle {
        ($name:literal, $plain:expr, $cycling:expr) => {{
            let expected = collect($plain.build().unwrap());
            let cycle = expected.len().div_ceil(CYCLES - 1);
            let mut rebuilt = Vec::new();

            for _ in 0..CYCLES {
                take(
                    &mut $cycling.state($name).build().unwrap(),
                    cycle,
                    &mut rebuilt,
                );
            }

            assert_eq!(rebuilt, expected, "{}", $name);
        }};
    }

    cycle!("bytes", plain.bytes(&binary), cycling.bytes(&binary));
    cycle!(
        "chunks",
        plain.chunks(&binary).size(7),
        cycling.chunks(&binary).size(7)
    );
    cycle!(
        "delimiter",
        plain.delimiter(&binary).keep(true),
        cycling.delimiter(&binary).keep(true)
    );
    cycle!(
        "lines",
        plain.lines(&textual).keepends(true),
        cycling.lines(&textual).keepends(true)
    );
}

#[rstest]
fn options_window_the_byte_range(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let content = seeded_bytes(1, 4096);
    let path = write(&tmp_dir, "data.bin", &content);

    assert_eq!(
        collect(reader.bytes(&path).start(100).end(500).build().unwrap()),
        content[100..500]
    );

    let chunked: Vec<u8> =
        collect(reader.chunks(&path).size(7).start(100).end(500).build().unwrap())
            .into_iter()
            .flatten()
            .collect();

    assert_eq!(chunked, content[100..500]);
}
