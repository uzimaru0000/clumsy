pub mod index;
pub mod object;

use chrono::{Local, TimeZone, Utc};
use index::{Entry, Index};
use libflate::zlib::{Decoder, Encoder};
use object::blob::Blob;
use object::commit;
use object::commit::Commit;
use object::tree;
use object::tree::Tree;
use object::GitObject;
use object::ObjectType;
use std::env;
use std::fs::{create_dir, File};
use std::io;
use std::io::prelude::*;
use std::os::macos::fs::MetadataExt;
use std::path::PathBuf;

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

pub fn ls_files_stage(bytes: &[u8]) -> io::Result<Index> {
    Index::from(&bytes).ok_or(io::Error::from(io::ErrorKind::InvalidData))
}

pub fn hash_object(bytes: &[u8]) -> io::Result<Blob> {
    let blob = Blob::from(&bytes).ok_or(io::Error::from(io::ErrorKind::InvalidInput))?;
    Ok(blob)
}

pub fn update_index(hash: &[u8], file_name: String) -> io::Result<Index> {
    let bytes = read_index()
        .unwrap_or([*b"DIRC", 0x0002u32.to_be_bytes(), 0x0000u32.to_be_bytes()].concat());
    let index = ls_files_stage(&bytes)?;

    let metadata = env::current_dir().and_then(|x| x.join(&file_name).metadata())?;
    let entry = Entry::new(
        Utc.timestamp(metadata.st_ctime(), metadata.st_ctime_nsec() as u32),
        Utc.timestamp(metadata.st_mtime(), metadata.st_mtime_nsec() as u32),
        metadata.st_dev() as u32,
        metadata.st_ino() as u32,
        metadata.st_mode(),
        metadata.st_uid(),
        metadata.st_gid(),
        metadata.st_size() as u32,
        Vec::from(hash),
        file_name.clone(),
    );

    let mut entries: Vec<Entry> = index
        .entries
        .into_iter()
        .filter(|x| x.name != entry.name && x.hash != entry.hash)
        .collect();
    entries.push(entry);

    Ok(Index::new(entries))
}

pub fn write_tree() -> io::Result<Tree> {
    let bytes = read_index()?;
    let index = ls_files_stage(&bytes)?;

    let contents = index
        .entries
        .iter()
        // TODO: 一旦modeは `100644` で固定
        .map(|x| tree::File::new(100644, x.name.clone(), &x.hash))
        .collect::<Vec<_>>();

    Ok(Tree::new(contents))
}

pub fn commit_tree(
    name: String,
    email: String,
    tree_hash: String,
    message: String,
) -> io::Result<Commit> {
    let parent = head_ref().and_then(read_ref).ok();
    let ts = Utc::now();
    let offset = {
        let local = Local::now();
        *local.offset()
    };
    let author = commit::User::new(name.clone(), email.clone(), ts, offset);
    let commit = Commit::new(tree_hash, parent, author.clone(), author.clone(), message);

    Ok(commit)
}

pub fn update_ref(path: PathBuf, hash: &[u8]) -> io::Result<()> {
    write_ref(path, hash)
}

pub fn read_index() -> io::Result<Vec<u8>> {
    let path = env::current_dir().map(|x| x.join(".git/index"))?;
    let mut file = File::open(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;

    Ok(bytes)
}

pub fn write_index(index: &Index) -> io::Result<()> {
    let mut file = File::create(".git/index")?;
    file.write_all(&index.as_bytes())?;
    file.flush()?;

    Ok(())
}

pub fn read_object(hash: String) -> io::Result<Vec<u8>> {
    let current_dir = env::current_dir()?;
    let (sub_dir, file) = hash.split_at(2);
    let path = current_dir.join(".git/objects").join(sub_dir).join(file);
    let mut file = File::open(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;

    Ok(bytes)
}

pub fn write_object(object: &GitObject) -> io::Result<()> {
    let hash = hex::encode(object.calc_hash());
    let (sub_dir, file) = hash.split_at(2);

    let path = env::current_dir()?;
    let path = path.join(".git/objects").join(sub_dir);

    // ディレクトリがなかったら
    if let Err(_) = path.metadata() {
        create_dir(&path)?;
    }

    let path = path.join(file);

    let mut encoder = Encoder::new(Vec::new())?;
    encoder.write_all(&object.as_bytes())?;
    let bytes = encoder.finish().into_result()?;

    let mut file = File::create(path)?;
    file.write_all(&bytes)?;
    file.flush()?;

    Ok(())
}

pub fn head_ref() -> io::Result<PathBuf> {
    let path = env::current_dir().map(|x| x.join(".git/HEAD"))?;
    let mut file = File::open(path)?;
    let mut refs = String::new();
    file.read_to_string(&mut refs)?;

    let (prefix, path) = refs.split_at(5);

    if prefix != "ref: " {
        return Err(io::Error::from(io::ErrorKind::InvalidData));
    }

    Ok(PathBuf::from(path.trim()))
}

pub fn read_ref(path: PathBuf) -> io::Result<String> {
    let path = env::current_dir().map(|x| x.join(".git").join(path))?;
    let mut file = File::open(path)?;
    let mut hash = String::new();
    file.read_to_string(&mut hash)?;

    Ok(hash.trim().to_string())
}

pub fn write_ref(path: PathBuf, hash: &[u8]) -> io::Result<()> {
    let path = env::current_dir().map(|x| x.join(".git").join(path))?;
    let mut file = File::create(path)?;
    file.write_all(hex::encode(hash).as_bytes())?;
    file.flush()?;

    Ok(())
}
