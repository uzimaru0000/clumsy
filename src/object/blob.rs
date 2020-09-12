use super::ObjectType;
#[cfg(feature = "json")]
use serde::Serialize;
use sha1::{Digest, Sha1};
use std::fmt;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "json", derive(Serialize))]
pub struct Blob {
    pub size: usize,
    pub content: String,
}

impl Blob {
    pub fn new(content: String) -> Self {
        Self {
            size: content.len(),
            content,
        }
    }

    pub fn from(bytes: &[u8]) -> Option<Self> {
        let content = String::from_utf8(bytes.to_vec());

        match content {
            Ok(content) => Some(Self {
                size: content.len(),
                content,
            }),
            _ => None,
        }
    }

    pub fn calc_hash(&self) -> Vec<u8> {
        Vec::from(Sha1::digest(&self.as_bytes()).as_slice())
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let header = format!("{} {}\0", ObjectType::Blob.to_string(), self.size);
        let store = format!("{}{}", header, self.to_string());

        Vec::from(store.as_bytes())
    }
}

impl fmt::Display for Blob {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.content)
    }
}
