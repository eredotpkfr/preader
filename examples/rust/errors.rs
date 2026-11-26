//! Handles an invalid line, an unsafe state name and a replaced file

use std::path::Path;

use log::{LevelFilter, info, warn};
use preader::{Config, Error, Mismatch, PReader, Result};

fn skip_invalid_lines(reader: &PReader, file: &Path) -> Result<()> {
    std::fs::write(file, b"foo\n\xff\nbar\n")?;

    let lines = reader.lines(file).build()?;

    for line in lines {
        match line {
            Ok(line) => info!("read {line}"),
            Err(error) => warn!("skipped a line: {error}"),
        }
    }

    Ok(())
}

fn reject_unsafe_name(reader: &PReader, file: &Path) {
    if let Err(Error::Name(error)) = reader.lines(file).state("../job-1").build() {
        warn!("rejected the name: {error}");
    }
}

fn start_over_on_a_replaced_file(reader: &PReader, file: &Path) -> Result<()> {
    reader.lines(file).state("job-1").start(4).build()?.save()?;

    std::fs::write(file, b"replaced\n")?;

    let lines = match reader.lines(file).state("job-1").build() {
        Err(
            error @ Error::Mismatch {
                kind: Mismatch::Identity,
                ..
            },
        ) => {
            warn!("{error}, starting over");

            reader.states().delete("job-1")?;
            reader.lines(file).state("job-1").build()?
        }
        lines => lines?,
    };

    info!("read {:?}", lines.collect::<Result<Vec<_>>>()?);

    Ok(())
}

fn main() -> Result<()> {
    env_logger::Builder::new().filter_level(LevelFilter::Info).init();

    let tmp = tempfile::tempdir()?;
    let state_dir = tmp.path().join("states");
    let file = tmp.path().join("data.bin");

    let config = Config {
        state_dir,
        ..Config::default()
    };
    let reader = PReader::from(config);

    skip_invalid_lines(&reader, &file)?;
    reject_unsafe_name(&reader, &file);
    start_over_on_a_replaced_file(&reader, &file)?;

    Ok(())
}
