//! Reads a file in fixed-size chunks, with and without the last partial chunk

use log::{LevelFilter, info};
use preader::{ChunkBuilder, PReader, Result};

fn main() -> Result<()> {
    env_logger::Builder::new().filter_level(LevelFilter::Info).init();

    let tmp = tempfile::tempdir()?;
    let file = tmp.path().join("data.bin");

    std::fs::write(&file, b"abcdefghijklmnopqrstuvwxyz")?;

    let reader = PReader::default();
    let read = |builder: ChunkBuilder| -> Result<Vec<String>> {
        builder
            .build()?
            .map(|chunk| Ok(String::from_utf8_lossy(&chunk?).into()))
            .collect()
    };

    let chunks = reader.chunks(&file).size(7);
    let full = reader.chunks(&file).size(7).drop_partial(true);

    info!("chunks of 7 bytes: {:?}", read(chunks)?);
    info!("without the partial chunk: {:?}", read(full)?);

    Ok(())
}
