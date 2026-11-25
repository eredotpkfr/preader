//! Reads only part of a file with start, end, skip, limit and align

use log::{LevelFilter, info};
use preader::{LineBuilder, PReader, Result};

fn main() -> Result<()> {
    env_logger::Builder::new().filter_level(LevelFilter::Info).init();

    let tmp = tempfile::tempdir()?;
    let file = tmp.path().join("data.bin");
    let content: String = (0..10).map(|index| format!("line-{index}\n")).collect();

    std::fs::write(&file, content)?;

    let reader = PReader::default();
    let read = |builder: LineBuilder| builder.build()?.collect::<Result<Vec<_>>>();

    let middle = reader.lines(&file).skip(1).limit(3);
    let head = reader.lines(&file).end(21);
    let tail = reader.lines(&file).start(52).align(true);

    info!("skip 1, take 3: {:?}", read(middle)?);
    info!("up to byte 21: {:?}", read(head)?);
    info!("from the first line after byte 52: {:?}", read(tail)?);

    Ok(())
}
