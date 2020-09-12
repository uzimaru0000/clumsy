pub mod blob;
pub mod commit;
pub mod tree;

use blob::Blob;
use commit::Commit;
#[cfg(feature = "json")]
use serde::Serialize;
use std::fmt;
use tree::Tree;

#[derive(Debug, Copy, Clone)]
pub enum ObjectType {
    Blob,
    Tree,
    Commit,
}

impl ObjectType {
    pub fn to_string(self) -> String {
        match self {
            ObjectType::Blob => String::from("blob"),
            ObjectType::Tree => String::from("tree"),
            ObjectType::Commit => String::from("commit"),
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "json", derive(Serialize))]
pub enum GitObject {
    Blob(Blob),
    Tree(Tree),
    Commit(Commit),
}

impl GitObject {
    pub fn new(obj_type: ObjectType, bytes: &[u8]) -> Option<Self> {
        match obj_type {
            ObjectType::Blob => Blob::from(bytes).map(GitObject::Blob),
            ObjectType::Tree => Tree::from(bytes).map(GitObject::Tree),
            ObjectType::Commit => Commit::from(bytes).map(GitObject::Commit),
        }
    }

    pub fn calc_hash(&self) -> Vec<u8> {
        match self {
            Self::Blob(obj) => obj.calc_hash(),
            Self::Tree(obj) => obj.calc_hash().unwrap_or_default(),
            Self::Commit(obj) => obj.calc_hash(),
        }
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        match self {
            Self::Blob(obj) => obj.as_bytes(),
            Self::Tree(obj) => obj.as_bytes().unwrap_or_default(),
            Self::Commit(obj) => obj.as_bytes(),
        }
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
