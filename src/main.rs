use gitto::*;
use std::env;
use std::fs::File;
use std::io::{Read, Result};
use std::path::PathBuf;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    let path = parse_path(args[1].clone())?;

    let mut file = File::open(path)?;
    let mut buf = Vec::new();
    let result = file
        .read_to_end(&mut buf)
        .and_then(|_| cat_file_p(buf.as_slice()))?;

    print!("{}", result);
    Ok(())
}

fn parse_path(hash: String) -> Result<PathBuf> {
    let current_dir = env::current_dir()?;
    let (sub_dir, file) = hash.split_at(2);
    Ok(current_dir.join(sub_dir).join(file))
}
