//! Saves progress automatically and resumes the auto-named state in the next run

use std::num::NonZeroU64;

use log::{LevelFilter, info};
use preader::{Config, PReader, Result};

const SAVE_EVERY: NonZeroU64 = NonZeroU64::new(1024).unwrap();

fn main() -> Result<()> {
    env_logger::Builder::new().filter_level(LevelFilter::Info).init();

    let tmp = tempfile::tempdir()?;
    let state_dir = tmp.path().join("states");

    let file = tmp.path().join("data.bin");
    let content: String = (0..1000).map(|index| format!("line-{index}\n")).collect();

    std::fs::write(&file, content)?;

    let config = Config {
        state_dir,
        auto_save_state: true,
        auto_save_state_bytes: SAVE_EVERY,
        auto_load_state: true,
        ..Config::default()
    };
    let reader = PReader::from(config);

    let read = reader.lines(&file).build()?.take(300).count();

    info!("first run read {read} lines");

    let mut resumed = reader.lines(&file).build()?;

    info!("second run resumes at {:.1}%", resumed.percent());

    if let Some(line) = resumed.next() {
        info!("second run starts with {}", line?);
    }

    let rest = resumed.by_ref().collect::<Result<Vec<_>>>()?;

    info!("second run ends with {}", rest[rest.len() - 1]);
    info!("second run finished at {:.1}%", resumed.percent());

    Ok(())
}
