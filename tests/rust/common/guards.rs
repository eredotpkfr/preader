use std::{
    fs::{self, File, FileTimes},
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
#[cfg(unix)]
use std::{os::unix::fs::PermissionsExt, path::PathBuf};

const PRE_EPOCH_OFFSET: Duration = Duration::from_secs(86_400);

pub fn mtime(path: &Path) -> SystemTime {
    fs::metadata(path).unwrap().modified().unwrap()
}

pub fn set_mtime(path: &Path, stamp: SystemTime) {
    File::options().write(true).open(path).unwrap().set_modified(stamp).unwrap();
}

pub fn set_pre_epoch_mtime(path: &Path) -> bool {
    let times = FileTimes::new().set_modified(UNIX_EPOCH - PRE_EPOCH_OFFSET);

    File::options()
        .write(true)
        .open(path)
        .is_ok_and(|file| file.set_times(times).is_ok())
}

#[cfg(unix)]
#[derive(Debug, Default)]
pub struct Blocked {
    paths: Vec<PathBuf>,
}

#[cfg(unix)]
impl Blocked {
    pub fn enforced(probe: &Path) -> bool {
        fs::create_dir_all(probe).unwrap();
        fs::set_permissions(probe, fs::Permissions::from_mode(0o000)).unwrap();

        let enforced = fs::read_dir(probe).is_err();

        fs::set_permissions(probe, fs::Permissions::from_mode(0o755)).unwrap();

        enforced
    }

    pub fn block(&mut self, path: &Path) -> PathBuf {
        self.chmod(path, 0o000)
    }

    pub fn read_only(&mut self, path: &Path) -> PathBuf {
        self.chmod(path, 0o500)
    }

    fn chmod(&mut self, path: &Path, mode: u32) -> PathBuf {
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
        self.paths.push(path.to_path_buf());

        path.to_path_buf()
    }
}

#[cfg(unix)]
impl Drop for Blocked {
    fn drop(&mut self) {
        for path in &self.paths {
            let mode = if path.is_dir() { 0o755 } else { 0o644 };

            drop(fs::set_permissions(path, fs::Permissions::from_mode(mode)));
        }
    }
}
