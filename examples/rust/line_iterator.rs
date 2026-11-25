//! Reads lines as they are, without empty lines and with their line endings

use log::{LevelFilter, info};
use preader::{LineBuilder, PReader, Result};

fn main() -> Result<()> {
    env_logger::Builder::new().filter_level(LevelFilter::Info).init();

    let tmp = tempfile::tempdir()?;
    let file = tmp.path().join("data.bin");

    std::fs::write(&file, b"line-0\n\nline-2\n")?;

    let reader = PReader::default();
    let read = |builder: LineBuilder| builder.build()?.collect::<Result<Vec<_>>>();

    let lines = reader.lines(&file);
    let filled = reader.lines(&file).skip_empty(true);
    let ended = reader.lines(&file).keepends(true);

    info!("lines: {:?}", read(lines)?);
    info!("without empty lines: {:?}", read(filled)?);
    info!("with line endings: {:?}", read(ended)?);

    Ok(())
}
