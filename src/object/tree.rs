use super::ObjectType;
use sha1::{Digest, Sha1};
use std::fmt;

#[derive(Debug)]
pub struct Tree {
    contents: Vec<File>,
}

#[derive(Debug)]
pub struct File {
    mode: usize,
    name: String,
    hash: String,
}

impl File {
    pub fn new(header: &[u8], hash: &[u8]) -> Option<Self> {
        let split_header = String::from_utf8(header.to_vec()).ok()?;

        let mut iter = split_header.split_whitespace();

        let mode = iter.next().and_then(|x| x.parse::<usize>().ok())?;
        let name = iter.next()?;

        Some(Self {
            mode,
            name: String::from(name),
            hash: hash
                .into_iter()
                .map(|x| format!("{:02x}", x))
                .collect::<String>(),
        })
    }

    pub fn encode(&self) -> Option<Vec<u8>> {
        let hash = hex::decode(self.hash.as_str()).ok()?;
        let header = format!("{} {}\0", self.mode, self.name);
        Some([header.as_bytes(), hash.as_slice()].concat())
    }
}

impl fmt::Display for File {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:>06} ??? {}\t{}", self.mode, self.hash, self.name)
    }
}

impl Tree {
    pub fn new(contents: Vec<File>) -> Self {
        Self { contents }
    }

    pub fn from(bytes: &[u8]) -> Option<Self> {
        let contents: Vec<File> = Vec::new();
        let mut iter = bytes.split(|&b| b == b'\0');

        let mut header = iter.next()?;
        let contents = iter.try_fold(contents, |mut acc, x| {
            let (hash, next_header) = x.split_at(20);
            let file = File::new(header, hash)?;

            acc.push(file);
            header = next_header;
            Some(acc)
        })?;

        Some(Self { contents })
    }
    pub fn calc_hash(&self) -> Option<String> {
        let content =
            self.contents
                .iter()
                .map(|x| x.encode())
                .try_fold(Vec::new(), |mut acc, x| {
                    acc.append(&mut x?);
                    Some(acc)
                })?;
        let header = format!("{} {}\0", ObjectType::Tree.to_string(), content.len());
        let store = [header.as_bytes(), content.as_slice()].concat();

        Some(format!("{:x}", Sha1::digest(store.as_slice())))
    }
}

impl fmt::Display for Tree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            (&self.contents)
                .into_iter()
                .map(|f| format!("{}", f))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}
