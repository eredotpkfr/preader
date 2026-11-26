//! Lists, searches, loads and deletes saved states, and resumes a loaded state

use std::path::Path;

use log::{LevelFilter, info};
use preader::{Config, PReader, Result};

fn save_states(reader: &PReader, file: &Path) -> Result<()> {
    for name in ["job-1", "sub-1/job-1", "sub-1/job-2"] {
        reader.lines(file).state(name).start(7).build()?.save()?;
    }

    info!("{} states saved", reader.states().count()?);

    Ok(())
}

fn search_states(reader: &PReader) -> Result<()> {
    let mut nested = reader.states().search("^sub-1/")?.collect::<Result<Vec<_>>>()?;

    nested.sort();

    info!("under sub-1/: {nested:?}");

    Ok(())
}

fn load_state(reader: &PReader, file: &Path) -> Result<()> {
    let state = reader.states().load("job-1")?;

    info!("job-1 resumes at byte {}", state.position);

    let mut lines = reader.lines(file).state(state).build()?;

    if let Some(line) = lines.next() {
        info!("loaded job-1 resumes with {}", line?);
    }

    Ok(())
}

fn delete_state(reader: &PReader) -> Result<()> {
    reader.states().delete("sub-1/job-2")?;

    info!(
        "sub-1/job-2 is still saved: {}",
        reader.states().exists("sub-1/job-2")
    );

    Ok(())
}

fn clear_states(reader: &PReader) -> Result<()> {
    reader.states().clear()?;

    info!("{} states left", reader.states().count()?);

    Ok(())
}

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

    save_states(&reader, &file)?;
    search_states(&reader)?;
    load_state(&reader, &file)?;
    delete_state(&reader)?;
    clear_states(&reader)?;

    Ok(())
}
