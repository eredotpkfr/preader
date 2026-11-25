//! Splits a file on a delimiter, without empty segments and with the delimiter kept

use log::{LevelFilter, info};
use preader::{PReader, Result, SegmentBuilder};

fn main() -> Result<()> {
    env_logger::Builder::new().filter_level(LevelFilter::Info).init();

    let tmp = tempfile::tempdir()?;
    let file = tmp.path().join("data.bin");

    std::fs::write(&file, b"seg-0,,seg-2,")?;

    let reader = PReader::default();
    let read = |builder: SegmentBuilder| -> Result<Vec<String>> {
        builder
            .build()?
            .map(|segment| Ok(String::from_utf8_lossy(&segment?).into()))
            .collect()
    };

    let segments = reader.segments(&file).delimiter(b',');
    let filled = reader.segments(&file).delimiter(b',').skip_empty(true);
    let kept = reader.segments(&file).delimiter(b',').keep(true);

    info!("segments: {:?}", read(segments)?);
    info!("without empty segments: {:?}", read(filled)?);
    info!("with the delimiter: {:?}", read(kept)?);

    Ok(())
}
