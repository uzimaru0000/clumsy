pub mod blob;
pub mod commit;
pub mod tree;

use blob::Blob;
use commit::Commit;
#[cfg(feature = "json")]
use serde::ser::{Serialize, SerializeStruct, Serializer};
use std::fmt;
use tree::Tree;

#[derive(Debug, Copy, Clone)]
pub enum ObjectType {
    Blob,
    Tree,
    Commit,
}

impl ObjectType {
    pub fn from(s: &str) -> Option<Self> {
        let mut header = s.split_whitespace();

        match header.next()? {
            "blob" => Some(ObjectType::Blob),
            "tree" => Some(ObjectType::Tree),
            "commit" => Some(ObjectType::Commit),
            _ => None,
        }
    }

    pub fn to_string(self) -> String {
        match self {
            ObjectType::Blob => String::from("blob"),
            ObjectType::Tree => String::from("tree"),
            ObjectType::Commit => String::from("commit"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum GitObject {
    Blob(Blob),
    Tree(Tree),
    Commit(Commit),
}

impl GitObject {
    pub fn new(bytes: &[u8]) -> Option<Self> {
        let mut iter = bytes.splitn(2, |&byte| byte == b'\0');

        let obj_type = iter
            .next()
            .and_then(|x| String::from_utf8(x.to_vec()).ok())
            .and_then(|x| ObjectType::from(&x))?;

        match obj_type {
            ObjectType::Blob => Blob::from(bytes).map(GitObject::Blob),
            ObjectType::Tree => Tree::from(bytes).map(GitObject::Tree),
            ObjectType::Commit => Commit::from(bytes).map(GitObject::Commit),
        }
    }

    pub fn calc_hash(&self) -> Vec<u8> {
        match self {
            Self::Blob(obj) => obj.calc_hash(),
            Self::Tree(obj) => obj.calc_hash(),
            Self::Commit(obj) => obj.calc_hash(),
        }
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        match self {
            Self::Blob(obj) => obj.as_bytes(),
            Self::Tree(obj) => obj.as_bytes(),
            Self::Commit(obj) => obj.as_bytes(),
        }
    }
}

#[cfg(feature = "json")]
impl Serialize for GitObject {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("GitObject", 2)?;
        match self {
            GitObject::Blob(blob) => {
                s.serialize_field("Blob", blob)?;
            }
            GitObject::Tree(tree) => {
                s.serialize_field("Tree", tree)?;
            }
            GitObject::Commit(commit) => {
                s.serialize_field("Commit", commit)?;
            }
        }
        s.serialize_field("hash", &hex::encode(self.calc_hash()))?;
        s.end()
    }
}

impl fmt::Display for GitObject {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Blob(obj) => obj.fmt(f),
            Self::Tree(obj) => obj.fmt(f),
            Self::Commit(obj) => obj.fmt(f),
        }
    }
}
