use std::{ffi::OsString, path::PathBuf};

use preader::{Config, IteratorBuild, IteratorRead, PReader, State};
use rstest::rstest;
use tempfile::TempDir;

use crate::common::{
    constants::TEST_STATE_NAME,
    fixtures::tmp_dir,
    funcs::{reader, write},
};

const CONTENT: &[u8] = b"foo";

#[rstest]
fn default_carries_the_config_defaults() {
    assert_eq!(PReader::default().config, Config::default());
}

#[rstest]
fn a_config_converts_into_a_reader() {
    let config = Config {
        buffer_capacity: 111,
        ..Config::default()
    };
    let reader: PReader = config.clone().into();

    assert_eq!(reader.config, config);
    assert_eq!(PReader::from(config.clone()).config, config);
}

#[rstest]
fn states_points_at_the_configured_directory(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());

    assert_eq!(reader.states().state_dir(), reader.config.state_dir);
}

#[rstest]
fn a_file_argument_accepts_every_ownership_shape(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let owned = write(&tmp_dir, "data.bin", CONTENT);
    let text = owned.to_str().unwrap().to_owned();
    let os = OsString::from(&text);

    let shapes: [PathBuf; 6] = [
        reader.bytes(text.as_str()).build().unwrap().state().file.path.clone(),
        reader.bytes(&text).build().unwrap().state().file.path.clone(),
        reader.bytes(text.clone()).build().unwrap().state().file.path.clone(),
        reader.bytes(owned.as_path()).build().unwrap().state().file.path.clone(),
        reader.bytes(&owned).build().unwrap().state().file.path.clone(),
        reader.bytes(os).build().unwrap().state().file.path.clone(),
    ];
    let canonical = dunce::canonicalize(&owned).unwrap();

    assert!(shapes.iter().all(|path| *path == canonical));

    // an owned PathBuf moves rather than being cloned
    let mut moved = reader.bytes(owned).build().unwrap();

    assert_eq!(moved.read().unwrap(), Some(b'f'));
}

#[rstest]
fn a_state_argument_accepts_every_name_shape(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", CONTENT);
    let name = TEST_STATE_NAME.to_owned();

    let named: [State; 4] = [
        reader.bytes(&path).state(TEST_STATE_NAME).build().unwrap().state().clone(),
        reader.bytes(&path).state(&name).build().unwrap().state().clone(),
        reader.bytes(&path).state(name.clone()).build().unwrap().state().clone(),
        reader
            .bytes(&path)
            .state(Some(TEST_STATE_NAME))
            .build()
            .unwrap()
            .state()
            .clone(),
    ];

    assert!(named.iter().all(|state| state.name == TEST_STATE_NAME));

    // None falls back to the auto-derived name, which is a digest of the path
    let auto = reader.bytes(&path).state(None::<&str>).build().unwrap().state().clone();
    let omitted = reader.bytes(&path).build().unwrap().state().clone();

    assert_eq!(auto.name, omitted.name);
    assert_ne!(auto.name, TEST_STATE_NAME);

    // a State object round-trips through the same setter
    let resumed = reader.bytes(&path).state(omitted).build().unwrap();

    assert_eq!(resumed.state().name, auto.name);
}
