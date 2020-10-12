use super::{FileSystem, Metadata};
use std::env;
use std::fs;
use std::fs::File;
use std::io;
use std::io::prelude::*;
#[cfg(target_os = "macos")]
use std::os::macos::fs::MetadataExt;
use std::path::PathBuf;

#[cfg(target_os = "macos")]
pub struct MacOSFileSystem {
    root: PathBuf,
}

#[cfg(target_os = "macos")]
impl MacOSFileSystem {
    pub fn init() -> io::Result<Self> {
        Ok(MacOSFileSystem {
            root: env::current_dir()?,
        })
    }
}

#[cfg(target_os = "macos")]
impl FileSystem for MacOSFileSystem {
    fn read(&self, path: String) -> io::Result<Vec<u8>> {
        let mut file = File::open(self.root.join(path))?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        Ok(buf)
    }

    fn write(&mut self, path: String, data: &[u8]) -> io::Result<()> {
        let mut file = File::create(self.root.join(path))?;
        file.write_all(data)?;
        file.flush()?;
        Ok(())
    }

    fn stat(&self, path: String) -> io::Result<Metadata> {
        let path = self.root.join(path);
        let metadata = path.metadata()?;

        Ok(Metadata {
            dev: metadata.st_dev() as u32,
            ino: metadata.st_ino() as u32,
            mode: metadata.st_mode(),
            uid: metadata.st_uid(),
            gid: metadata.st_gid(),
            size: metadata.st_size() as u32,
            mtime: metadata.st_mtime() as u32,
            mtime_nsec: metadata.st_mtime_nsec() as u32,
            ctime: metadata.st_ctime() as u32,
            ctime_nsec: metadata.st_ctime_nsec() as u32,
        })
    }

    fn create_dir(&mut self, path: String) -> io::Result<()> {
        let path = self.root.join(path);
        fs::create_dir_all(path)
    }
}
