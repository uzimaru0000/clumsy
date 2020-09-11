pub mod inmem;

use std::io;

pub trait FileSystem {
    fn read(&self, path: String) -> io::Result<Vec<u8>>;
    fn write(&mut self, path: String, data: &[u8]) -> io::Result<()>;
    fn stat(&self, path: String) -> io::Result<Metadata>;
    fn create_dir(&mut self, path: String) -> io::Result<()>;
}

#[derive(Debug)]
pub struct Metadata {
    pub dev: u32,
    pub ino: u32,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub size: u32,
    pub mtime: u32,
    pub mtime_nsec: u32,
    pub ctime: u32,
    pub ctime_nsec: u32,
}
