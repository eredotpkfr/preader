use std::{ffi::OsString, fs, path::PathBuf};

use preader::{Config, Error, IteratorBuild, IteratorRead, PReader, State};
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
    assert_eq!(*PReader::default().config(), Config::default());
}

#[rstest]
fn a_config_converts_into_a_reader() {
    let config = Config {
        buffer_capacity: 111,
        ..Config::default()
    };
    let reader: PReader = config.clone().into();

    assert_eq!(*reader.config(), config);
    assert_eq!(*PReader::from(config.clone()).config(), config);
}

#[rstest]
fn states_points_at_the_configured_directory(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());

    assert_eq!(reader.states().state_dir(), reader.config().state_dir);
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

    let auto = reader.bytes(&path).state(None::<&str>).build().unwrap().state().clone();
    let omitted = reader.bytes(&path).build().unwrap().state().clone();

    assert_eq!(auto.name, omitted.name);
    assert_ne!(auto.name, TEST_STATE_NAME);

    let mut resumed = reader.bytes(&path).state(omitted).build().unwrap();

    assert_eq!(resumed.state().name, auto.name);
}

#[rstest]
fn new_matches_the_default_config() {
    assert_eq!(*PReader::new().config(), Config::default());
}

#[rstest]
fn a_symlink_is_resolved_to_its_target(tmp_dir: TempDir) {
    use std::os::unix::fs::symlink;

    let reader = reader(&tmp_dir, Config::default());
    let target = write(&tmp_dir, "data.bin", b"foo\n");
    let link = tmp_dir.path().join("link.bin");

    symlink(&target, &link).unwrap();

    assert_eq!(
        reader.bytes(&link).build().unwrap().state().file.path,
        target.canonicalize().unwrap()
    );
}

#[rstest]
fn a_directory_is_rejected(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let directory = tmp_dir.path().join("a-directory");

    fs::create_dir(&directory).unwrap();

    let error = reader.bytes(&directory).build().unwrap_err();

    assert!(error.to_string().contains("not a file"), "got {error}");
}

#[rstest]
fn a_socket_is_rejected(tmp_dir: TempDir) {
    use std::os::unix::net::UnixListener;

    let reader = reader(&tmp_dir, Config::default());
    let socket = tmp_dir.path().join("a-socket");
    let _listener = UnixListener::bind(&socket).unwrap();
    let error = reader.bytes(&socket).build().unwrap_err();

    assert!(error.to_string().contains("not a file"), "got {error}");
}

#[rstest]
fn an_unreadable_file_is_rejected(tmp_dir: TempDir) {
    use std::os::unix::fs::PermissionsExt;

    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", b"foo\n");

    fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();

    if fs::read(&path).is_ok() {
        return; // permissions are not enforced here
    }

    let error = reader.bytes(&path).build().unwrap_err();

    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();

    assert!(
        matches!(&error, Error::Io(io) if io.kind() == std::io::ErrorKind::PermissionDenied),
        "got {error}"
    );
}

#[rstest]
fn a_non_utf8_path_is_rejected(tmp_dir: TempDir) {
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

    let reader = reader(&tmp_dir, Config::default());
    let path = tmp_dir.path().join(OsStr::from_bytes(b"data-\xff.bin"));

    if fs::write(&path, b"foo").is_err() {
        return; // a non-UTF-8 name cannot be created here
    }

    let error = reader.bytes(&path).build().unwrap_err();

    assert!(error.to_string().contains("invalid UTF-8"), "got {error}");
}

#[rstest]
fn a_pre_epoch_mtime_is_rejected(tmp_dir: TempDir) {
    use std::time::{Duration, SystemTime};

    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", b"foo\n");
    let before_epoch = SystemTime::UNIX_EPOCH - Duration::from_secs(86_400);

    if fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(before_epoch)
        .is_err()
    {
        return; // a pre-epoch mtime cannot be set here
    }

    let error = reader.bytes(&path).build().unwrap_err();

    assert!(matches!(error, Error::Time(_)), "got {error}");
}

#[rstest]
fn a_single_byte_file_yields_one_item(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", b"a");
    let collected: Vec<u8> =
        reader.bytes(&path).build().unwrap().map(|byte| byte.unwrap()).collect();

    assert_eq!(collected, b"a");
}

#[rstest]
fn the_maximum_start_and_end_yield_nothing(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", b"foo\n");
    let mut bytes = reader.bytes(&path).start(u64::MAX).end(u64::MAX).build().unwrap();

    assert!(bytes.read().unwrap().is_none());
}

#[rstest]
fn the_maximum_start_beyond_the_end_is_rejected(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", b"foo\n");
    let error = reader.bytes(&path).start(u64::MAX).end(0).build().unwrap_err();

    assert!(matches!(error, Error::InvalidRange { .. }), "got {error}");
    assert!(error.to_string().contains("must be <="));
}

#[rstest]
fn an_unnormalized_path_resolves_to_the_same_autoname(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", b"foo\n");
    let detoured = tmp_dir.path().join(".").join("data.bin");

    assert_eq!(
        reader.bytes(&path).build().unwrap().state().name,
        reader.bytes(&detoured).build().unwrap().state().name
    );
}

#[rstest]
fn a_long_path_still_produces_a_digest_name(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, &format!("{}.bin", "y".repeat(180)), b"foo");

    assert_eq!(reader.bytes(&path).build().unwrap().state().name.len(), 64);
}

#[rstest]
fn an_unusual_path_is_read_verbatim(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());

    for name in [
        "a b.bin",
        "a'b.bin",
        "a$b.bin",
        "café.bin",
        "日本語.bin",
        "a;b.bin",
    ] {
        let path = write(&tmp_dir, name, b"foo");
        let collected: Vec<u8> =
            reader.bytes(&path).build().unwrap().map(|byte| byte.unwrap()).collect();

        assert_eq!(collected, b"foo", "{name}");
    }
}

#[rstest]
fn states_resolve_under_the_configured_state_dir(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());

    assert_eq!(
        reader.states().path(TEST_STATE_NAME).unwrap().parent(),
        Some(reader.config().state_dir.as_path())
    );
}
