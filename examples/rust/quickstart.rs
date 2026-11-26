//! Reads a file line by line, follows the read percentage and saves the position

use log::{LevelFilter, info};
use preader::{Config, PReader, Result};

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

    let mut lines = reader.lines(&file).state("job-1").build()?;

    while let Some(line) = lines.next() {
        let line = line?;

        info!("read {line}, {:.1}% done", lines.percent());

        if line == "line-2" {
            break;
        }
    }

    let state = lines.save()?;

    info!("saved {} at byte {}", state.name, state.position);

    Ok(())
}
