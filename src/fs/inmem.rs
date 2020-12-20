use super::{FileSystem, Metadata};
#[cfg(feature = "json")]
use serde::ser::SerializeMap;
#[cfg(feature = "json")]
use serde::{Serialize, Serializer};
use std::collections::HashMap;
use std::io;

#[derive(Debug, Clone)]
enum Entity {
    Dir(HashMap<String, Entity>),
    File(Vec<u8>),
}

impl Entity {
    pub fn change_dir(&self, path: String) -> io::Result<&Entity> {
        path.split("/").try_fold(self, |st, x| match st {
            Self::File(_) => Err(io::Error::from(io::ErrorKind::NotFound)),
            Self::Dir(dir) => dir.get(x).ok_or(io::Error::from(io::ErrorKind::NotFound)),
        })
    }

    pub fn change_dir_mut(&mut self, path: String) -> io::Result<&mut Entity> {
        path.split("/").try_fold(self, |st, x| match st {
            Self::File(_) => Err(io::Error::from(io::ErrorKind::NotFound)),
            Self::Dir(dir) => dir
                .get_mut(x)
                .ok_or(io::Error::from(io::ErrorKind::NotFound)),
        })
    }

    pub fn read(&self) -> io::Result<Vec<u8>> {
        if let Self::File(data) = self {
            Ok(data.clone())
        } else {
            Err(io::Error::from(io::ErrorKind::NotFound))
        }
    }

    pub fn write(&mut self, name: String, data: &[u8]) -> io::Result<()> {
        if let Self::Dir(dir) = self {
            dir.insert(name, Self::File(data.to_vec()));
            Ok(())
        } else {
            Err(io::Error::from(io::ErrorKind::NotFound))
        }
    }

    pub fn make_dir(&mut self, name: String) -> io::Result<()> {
        if let Self::Dir(dir) = self {
            dir.insert(name, Self::Dir(HashMap::new()));
            Ok(())
        } else {
            Err(io::Error::from(io::ErrorKind::NotFound))
        }
    }

    pub fn remove(&mut self, name: String) -> io::Result<()> {
        let (path, name) = path_split(name);
        match path.len() {
            0 => if let Self::Dir(dir) = self { 
                dir.remove(&name).ok_or(io::Error::from(io::ErrorKind::InvalidInput)).map(|_| ())
            } else {
                Err(io::Error::from(io::ErrorKind::InvalidInput))
            },
            _ => self.change_dir_mut(path.join("/")).and_then(|x| x.remove(name))
        }
    }
}

#[cfg(feature = "json")]
impl Serialize for Entity {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Dir(dir) => {
                let mut s = serializer.serialize_map(Some(dir.len()))?;
                for (k, v) in dir {
                    s.serialize_entry(k, v)?;
                }
                s.end()
            }
            Self::File(file) => serializer.serialize_bytes(file),
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "json", derive(Serialize))]
pub struct InMemFileSystem {
    root: Entity,
}

impl InMemFileSystem {
    pub fn init() -> Self {
        let root = Entity::Dir(
            vec![(
                ".git".to_owned(),
                Entity::Dir(
                    vec![
                        ("objects".to_owned(), Entity::Dir(HashMap::new())),
                        (
                            "refs".to_owned(),
                            Entity::Dir(
                                vec![("heads".to_owned(), Entity::Dir(HashMap::new()))]
                                    .into_iter()
                                    .collect::<HashMap<_, _>>(),
                            ),
                        ),
                        (
                            "HEAD".to_owned(),
                            Entity::File(b"ref: refs/heads/master".to_vec()),
                        ),
                    ]
                    .into_iter()
                    .collect::<HashMap<_, _>>(),
                ),
            )]
            .into_iter()
            .collect::<HashMap<_, _>>(),
        );

        Self { root }
    }
}

impl FileSystem for InMemFileSystem {
    fn read(&self, path: String) -> io::Result<Vec<u8>> {
        self.root.change_dir(path).and_then(|x| x.read())
    }

    fn write(&mut self, path: String, data: &[u8]) -> io::Result<()> {
        let (dir_name, file) = path_split(path);

        if dir_name.len() > 0 {
            self.root.change_dir_mut(dir_name.join("/"))
        } else {
            Ok(&mut self.root)
        }
        .and_then(|x| x.write(file, data))
    }

    fn stat(&self, path: String) -> io::Result<Metadata> {
        let entity = self.root.change_dir(path)?;

        if let Entity::File(_) = entity {
            Ok(Metadata {
                dev: 0,
                ino: 0,
                mode: 33188,
                uid: 0,
                gid: 0,
                size: 0,
                mtime: 0,
                mtime_nsec: 0,
                ctime: 0,
                ctime_nsec: 0,
            })
        } else {
            Err(io::Error::from(io::ErrorKind::InvalidData))
        }
    }

    fn create_dir(&mut self, path: String) -> io::Result<()> {
        let (dir_name, dir) = path_split(path);
        self.root
            .change_dir_mut(dir_name.join("/"))
            .and_then(|x| x.make_dir(dir))
    }

    fn rename(&mut self, from: String, to: String) -> io::Result<()> {
        let file = self.read(from.clone())?;
        self.remove(from.clone())?;
        self.write(to, &file)
    }

    fn remove(&mut self, path: String) -> io::Result<()> {
        self.root.remove(path)
    }
}

fn path_split(path: String) -> (Vec<String>, String) {
    let iter = path.split("/").collect::<Vec<_>>();

    match iter.as_slice() {
        [path @ .., last] => (
            path.iter().map(|&x| String::from(x)).collect::<Vec<_>>(),
            last.to_string(),
        ),
        _ => (Vec::new(), String::new()),
    }
}

#[test]
fn test_fs_read() {
    let fs = InMemFileSystem::init();
    let data = fs.read(".git/HEAD".to_string());

    println!("{:?}", data);

    if let Err(_) = data {
        assert!(false);
    }
}

#[test]
fn test_fs_write() {
    let mut fs = InMemFileSystem::init();
    let result = fs.write(".git/objects/hoge".to_string(), b"hello");

    println!("{:?}", fs);

    if let Err(_) = result {
        assert!(false);
    }
}
