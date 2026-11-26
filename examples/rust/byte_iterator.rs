//! Reads a file byte by byte, then only a window of it

use log::{LevelFilter, info};
use preader::{PReader, Result};

fn main() -> Result<()> {
    env_logger::Builder::new().filter_level(LevelFilter::Info).init();

    let tmp = tempfile::tempdir()?;
    let file = tmp.path().join("data.bin");

    std::fs::write(&file, b"abcdefghijklmnopqrstuvwxyz")?;

    let reader = PReader::default();

    let every = reader.bytes(&file).build()?.collect::<Result<Vec<_>>>()?;
    let window = reader.bytes(&file).skip(3).limit(4).build()?.collect::<Result<Vec<_>>>()?;

    info!("every byte: {}", String::from_utf8_lossy(&every));
    info!(
        "4 bytes after the first 3: {}",
        String::from_utf8_lossy(&window)
    );

    Ok(())
}
