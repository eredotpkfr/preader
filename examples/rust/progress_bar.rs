//! Shows an indicatif progress bar that follows the read percentage and resumes with it

use indicatif::{ProgressBar, ProgressStyle};
use preader::{Config, LineIterator, PReader, Result};

const BAR: &str = "{percent:>3}%|{bar:10}| {msg}";

fn read_with_progress_bar(lines: &mut LineIterator, stop: Option<usize>) -> Result<()> {
    let style = ProgressStyle::with_template(BAR)
        .expect("BAR is a valid template")
        .progress_chars("█▉▊▋▌▍▎▏ ");

    let bar = ProgressBar::new(100).with_style(style);
    let update = |percent: f64| {
        bar.set_position(percent.round() as u64);
        bar.set_message(format!("{percent:.1}%"));
    };

    update(lines.percent());

    let mut count = 0;

    while let Some(line) = lines.next() {
        line?;
        count += 1;

        update(lines.percent());

        if Some(count) == stop {
            break;
        }
    }

    update(lines.percent());
    bar.abandon();

    Ok(())
}

fn main() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let state_dir = tmp.path().join("states");

    let file = tmp.path().join("data.bin");
    let content: String = (0..200_000).map(|index| format!("line-{index}\n")).collect();

    std::fs::write(&file, content)?;

    let config = Config {
        state_dir,
        ..Config::default()
    };
    let reader = PReader::from(config);

    let mut first = reader.lines(&file).state("job-1").build()?;

    read_with_progress_bar(&mut first, Some(80_000))?;

    first.save()?;

    read_with_progress_bar(&mut reader.lines(&file).state("job-1").build()?, None)?;

    Ok(())
}
