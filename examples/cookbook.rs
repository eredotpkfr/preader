use std::{fs, path::PathBuf};

use preader::{Config, IteratorBuild, IteratorOptions, IteratorRead, PReader, Result};

fn main() -> Result<()> {
    let root = PathBuf::from(std::env::args().nth(1).expect("usage: cookbook <dir>"));

    fs::create_dir_all(&root)?;

    let text = root.join("app.log");
    let csv = root.join("rows.csv");

    fs::write(&text, "alpha\n\nbeta\ngamma\n")?;
    fs::write(&csv, "a,b,,c")?;

    let reader = PReader::from(Config {
        state_dir: root.join("states"),
        auto_load_state: true,
        ..Config::default()
    });

    // 1 - lines, the shortest form
    let mut lines = reader.lines(&text).build()?;
    let mut collected = Vec::new();

    while let Some(line) = lines.read()? {
        collected.push(line.to_owned());
    }

    assert_eq!(collected, ["alpha", "", "beta", "gamma"]);
    assert_eq!(lines.state().percent(), 100.0);

    // 2 - flags: keepends + skip_empty
    let mut kept = reader.lines(&text).keepends(true).skip_empty(true).build()?;
    let mut bodies = Vec::new();

    while let Some(line) = kept.read()? {
        bodies.push(line.to_owned());
    }

    assert_eq!(bodies, ["alpha\n", "beta\n", "gamma\n"]);

    // 3 - windowing on bytes
    let mut window = reader.bytes(&text).start(6).limit(4).build()?;
    let mut bytes = Vec::new();

    while let Some(byte) = window.read()? {
        bytes.push(byte);
    }

    assert_eq!(bytes, b"\nbet");

    // 4 - chunks with drop_partial
    let mut chunks = reader.chunks(&text).size(5).drop_partial(true).build()?;
    let mut sizes = Vec::new();

    while let Some(chunk) = chunks.read()? {
        sizes.push(chunk.len());
    }

    assert_eq!(sizes, [5, 5, 5]);

    // 5 - delimiter: default ',' plus a setter, and skip_empty
    let mut fields = reader.delimiter(&csv).skip_empty(true).build()?;
    let mut segments = Vec::new();

    while let Some(field) = fields.read()? {
        segments.push(field.to_vec());
    }

    assert_eq!(segments, [b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]);

    // 6 - state lifecycle: save, resume by name, verify, resync
    let mut first = reader.lines(&text).state("nightly").limit(2).build()?;

    while first.read()?.is_some() {}

    let position = first.state().position;
    let mut checkpoint = first.state().clone();

    checkpoint.save()?;
    drop(first);

    let mut resumed = reader.lines(&text).state("nightly").build()?;

    assert_eq!(resumed.state().position, position);

    let mut tail = Vec::new();

    while let Some(line) = resumed.read()? {
        tail.push(line.to_owned());
    }

    assert_eq!(tail, ["beta", "gamma"]);

    // 7 - resume from a State object, then registry operations
    let state = reader.states().load("nightly")?;

    state.verify()?;

    let moved = root.join("moved.log");

    fs::copy(&text, &moved)?;

    let resynced = state.resync(&moved)?;

    assert_eq!(resynced.file.path, dunce::canonicalize(&moved)?);
    assert_eq!(resynced.position, state.position);

    let registry = reader.states();

    assert!(registry.exists("nightly"));
    assert_eq!(registry.count()?, 1);
    assert_eq!(registry.names()?.collect::<Result<Vec<_>>>()?, ["nightly"]);
    assert!(registry.find("missing")?.is_none());
    assert_eq!(registry.search("^night")?.count(), 1);

    registry.clear()?;

    assert_eq!(registry.count()?, 0);

    // 8 - options as one value, and Display
    let narrow = IteratorOptions {
        start: 7,
        end: 11,
        ..IteratorOptions::default()
    };
    let mut scoped = reader.lines(&text).options(narrow).build()?;

    assert_eq!(scoped.read()?, Some("beta"));
    assert!(format!("{reader:?}").starts_with("PReader { config: Config {"));

    println!("cookbook: every scenario passed");

    Ok(())
}
