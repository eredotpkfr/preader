//! Saves checkpoints while reading and resumes from the last one in a later run
//!
//! The line being read when a checkpoint is saved is not finished yet, so it is read again

use log::{LevelFilter, info, warn};
use preader::{Config, PReader, Result};

const CHECKPOINT_EVERY: usize = 2;
const STOP_AT: usize = 5;

fn main() -> Result<()> {
    env_logger::Builder::new().filter_level(LevelFilter::Info).init();

    let tmp = tempfile::tempdir()?;
    let state_dir = tmp.path().join("states");

    let file = tmp.path().join("data.bin");
    let content: String = (0..6).map(|index| format!("line-{index}\n")).collect();

    std::fs::write(&file, content)?;

    let config = Config {
        state_dir,
        ..Config::default()
    };
    let reader = PReader::from(config);

    let mut first = reader.lines(&file).state("job-1").build()?;
    let mut count = 0;

    while let Some(line) = first.next() {
        count += 1;

        info!("first run read {}", line?);

        if count % CHECKPOINT_EVERY == 0 {
            info!("checkpoint at byte {}", first.save()?.position);
        }

        if count == STOP_AT {
            warn!("first run stops without a checkpoint");
            break;
        }
    }

    for line in reader.lines(&file).state("job-1").build()? {
        info!("second run read {}", line?);
    }

    Ok(())
}
