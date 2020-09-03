use super::ObjectType;
use sha1::{Digest, Sha1};
use std::fmt;

#[derive(Debug)]
pub struct Blob {
    size: usize,
    content: String,
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

    pub fn calc_hash(&self) -> String {
        let header = format!("{} {}\0", ObjectType::Blob.to_string(), self.size);
        let store = format!("{}{}", header, self.to_string());

        format!("{:x}", Sha1::digest(store.as_bytes()))
    }
}

impl fmt::Display for Blob {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.content)
    }
}
