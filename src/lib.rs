use flate2::read::ZlibDecoder;
use std::io::prelude::*;
use std::io::Result;

pub fn cat_file_p(bytes: &[u8]) -> Result<String> {
    let mut d = ZlibDecoder::new(bytes);
    let mut s = String::new();
    d.read_to_string(&mut s).map(|_| s)
}
