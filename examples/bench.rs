use std::{
    fs::{self, File},
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    time::Instant,
};

use preader::{Config, IteratorBuild, IteratorRead, PReader, Result};

const LINES: usize = 200_000;
const ROUNDS: usize = 7;

fn best(rounds: usize, mut run: impl FnMut() -> usize) -> (f64, usize) {
    let mut fastest = f64::MAX;
    let mut checksum = 0;

    for _ in 0..rounds {
        let started = Instant::now();
        let total = run();
        let elapsed = started.elapsed().as_secs_f64() * 1000.0;

        fastest = fastest.min(elapsed);
        checksum = total;
    }

    (fastest, checksum)
}

fn main() -> Result<()> {
    let root = PathBuf::from(std::env::args().nth(1).expect("usage: bench <dir>"));

    fs::create_dir_all(&root)?;

    let path = root.join("bench.log");
    let mut file = File::create(&path)?;

    for index in 0..LINES {
        writeln!(
            file,
            "line {index:06} lorem ipsum dolor sit amet consectetur adipiscing"
        )?;
    }

    file.sync_all()?;
    drop(file);

    let size = fs::metadata(&path)?.len();
    let reader = PReader::from(Config {
        state_dir: root.join("states"),
        ..Config::default()
    });

    let (baseline, baseline_bytes) = best(ROUNDS, || {
        let mut handle = BufReader::with_capacity(64 * 1024, File::open(&path).unwrap());
        let mut buffer = String::new();
        let mut total = 0;

        loop {
            buffer.clear();

            match handle.read_line(&mut buffer).unwrap() {
                0 => break,
                _ => total += buffer.trim_end_matches('\n').len(),
            }
        }

        total
    });

    let (control, _) = best(ROUNDS, || {
        let mut handle = BufReader::with_capacity(64 * 1024, File::open(&path).unwrap());
        let mut buffer = String::new();
        let mut total = 0;

        loop {
            buffer.clear();

            match handle.read_line(&mut buffer).unwrap() {
                0 => break,
                _ => total += buffer.trim_end_matches('\n').len(),
            }
        }

        total
    });

    let (preader, preader_bytes) = best(ROUNDS, || {
        let mut lines = reader.lines(&path).build().unwrap();
        let mut total = 0;

        while let Some(line) = lines.read().unwrap() {
            total += line.len();
        }

        total
    });

    assert_eq!(
        baseline_bytes, preader_bytes,
        "the two loops must read the same bytes"
    );

    println!(
        "file           {} lines / {:.2} MiB",
        LINES,
        size as f64 / (1024.0 * 1024.0)
    );
    println!("baseline       {baseline:.2} ms  (hand-written BufReader loop)");
    println!("preader        {preader:.2} ms");
    println!("control        {control:.2} ms  (ayni dongu, ikinci sirada)");
    println!("harness gurultu{:+.1}%", (control / baseline - 1.0) * 100.0);
    println!("overhead       {:+.1}%", (preader / baseline - 1.0) * 100.0);

    Ok(())
}
