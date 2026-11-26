//! Resyncs a saved state after its file moved or grew, so the read goes on

use std::{io::Write, path::Path};

use log::{LevelFilter, info, warn};
use preader::{Config, Error, LineIterator, Mismatch, PReader, Result};

fn resume(reader: &PReader, file: &Path) -> Result<LineIterator> {
    match reader.lines(file).state("job-1").build() {
        Err(
            error @ Error::Mismatch {
                kind: Mismatch::Path | Mismatch::Size | Mismatch::Mtime | Mismatch::Missing,
                ..
            },
        ) => {
            warn!("{error}, resyncing");

            reader.states().resync("job-1", file)?;
            reader.lines(file).state("job-1").build()
        }
        lines => lines,
    }
}

fn main() -> Result<()> {
    env_logger::Builder::new().filter_level(LevelFilter::Info).init();

    let tmp = tempfile::tempdir()?;
    let state_dir = tmp.path().join("states");
    let file = tmp.path().join("data.bin");
    let moved = tmp.path().join("moved.bin");

    std::fs::write(&file, b"line-0\nline-1\n")?;

    let config = Config {
        state_dir,
        ..Config::default()
    };
    let reader = PReader::from(config);

    let mut lines = reader.lines(&file).state("job-1").build()?;

    for line in lines.by_ref() {
        info!("read {}", line?);
    }

    lines.save()?;

    std::fs::rename(&file, &moved)?;

    let lines = resume(&reader, &moved)?;

    info!("resumes at {:.1}% after the file moved", lines.percent());

    std::fs::OpenOptions::new()
        .append(true)
        .open(&moved)?
        .write_all(b"line-2\nline-3\n")?;

    for line in resume(&reader, &moved)? {
        info!("read {} after the file grew", line?);
    }

    Ok(())
}
