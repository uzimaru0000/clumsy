mod object;

use libflate::zlib::Decoder;
use object::GitObject;
use object::ObjectType;
use std::io;
use std::io::prelude::*;

fn check_type(header: String) -> Option<ObjectType> {
    let mut header = header.split_whitespace();

    header.next().and_then(|t| match t {
        "blob" => Some(ObjectType::Blob),
        "tree" => Some(ObjectType::Tree),
        "commit" => Some(ObjectType::Commit),
        _ => None,
    })
}

pub fn cat_file_p(bytes: &[u8]) -> io::Result<GitObject> {
    let mut d = Decoder::new(&bytes[..])?;
    let mut buf = Vec::new();
    d.read_to_end(&mut buf)?;

    let mut iter = buf.splitn(2, |&byte| byte == b'\0');

    iter.next()
        .and_then(|x| String::from_utf8(x.to_vec()).ok())
        .and_then(check_type)
        .and_then(|t| iter.next().map(|x| (t, x)))
        .and_then(|(t, b)| GitObject::new(t, b))
        .ok_or(io::Error::from(io::ErrorKind::InvalidData))
}
