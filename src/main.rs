use clumsy::fs::inmem::InMemFileSystem;
use clumsy::fs::mac::MacOSFileSystem;
use clumsy::fs::FileSystem;
use clumsy::object::GitObject;
use clumsy::*;
use std::io;

use libflate::zlib::{Decoder, Encoder};
use std::fs::File;
use std::io::prelude::*;

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let fs = MacOSFileSystem::init()?;
    let mut git = Git::new(fs);

    let sub_cmd = args.get(1).unwrap().clone();
    match sub_cmd.as_str() {
        "cat-file" => {
            let obj = cat_file_p(args.get(2).unwrap().clone())?;
            println!("{}", obj);
            Ok(())
        }
        "hash-object" => {
            let blob = hash_object(args.get(2).unwrap().clone())?;
            println!("{}", hex::encode(blob.calc_hash()));
            Ok(())
        }
        "add" => {
            let bytes = git.file_system.read(args.get(2).unwrap().clone())?;
            add(&mut git, args.get(2).unwrap().clone(), &bytes)
        }
        "commit" => commit(&mut git, args.get(2).unwrap().clone()),
        _ => Ok(()),
    }

    // let branch = args.get(1).unwrap().clone();
    // switch(&mut git, branch)?;
}

pub fn cat_file_p(hash: String) -> io::Result<GitObject> {
    let (sub_dir, file) = hash.split_at(2);
    let path = format!(".git/objects/{}/{}", sub_dir, file);

    let mut file = File::open(path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    let mut d = Decoder::new(&buf[..])?;
    let mut buf = Vec::new();
    d.read_to_end(&mut buf)?;

    GitObject::new(&buf).ok_or(io::Error::from(io::ErrorKind::InvalidData))
}

pub fn hash_object(path: String) -> io::Result<object::blob::Blob> {
    let mut file = File::open(path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;

    object::blob::Blob::from(&buf).ok_or(io::Error::from(io::ErrorKind::InvalidInput))
}

pub fn add<F: FileSystem>(git: &mut Git<F>, file_name: String, bytes: &[u8]) -> io::Result<()> {
    // git hash-object -w path
    let blob = git.hash_object(&bytes).map(GitObject::Blob)?;
    git.write_object(&blob)?;

    // git update-index --add --cacheinfo <mode> <hash> <name>
    let index = git.update_index(&blob.calc_hash(), file_name)?;
    git.write_index(&index)?;
    println!("write_index");

    Ok(())
}

fn commit<F: FileSystem>(git: &mut Git<F>, message: String) -> io::Result<()> {
    // git write-tree
    let tree = git.write_tree().map(GitObject::Tree)?;
    git.write_object(&tree)?;

    let tree_hash = tree.calc_hash();
    // echo message | git commit-tree <hash>
    let commit = git
        .commit_tree(
            "uzimaru0000".to_string(),
            "shuji365630@gmail.com".to_string(),
            hex::encode(tree_hash),
            message,
        )
        .map(GitObject::Commit)?;
    git.write_object(&commit)?;

    // git update-ref refs/heads/master <hash>
    git.update_ref(git.head_ref()?, &commit.calc_hash())?;

    Ok(())
}

fn log<F: FileSystem>(git: &mut Git<F>) -> io::Result<GitObject> {
    let commit_hash = git.head_ref().and_then(|x| git.read_ref(x))?;
    let commit = git.read_object(commit_hash)?;
    git.cat_file_p(&commit)
}

fn switch<F: FileSystem>(git: &mut Git<F>, branch: String) -> io::Result<()> {
    let commit_hash = git.read_ref(format!("refs/heads/{}", branch))?;
    let commit = git
        .read_object(commit_hash)
        .and_then(|x| git.cat_file_p(&x))
        .and_then(|x| {
            if let GitObject::Commit(commit) = x {
                Ok(commit)
            } else {
                Err(io::Error::from(io::ErrorKind::InvalidData))
            }
        })?;
    let tree = git
        .read_object(commit.tree)
        .and_then(|x| git.cat_file_p(&x))
        .and_then(|x| {
            if let GitObject::Tree(tree) = x {
                Ok(tree)
            } else {
                Err(io::Error::from(io::ErrorKind::InvalidData))
            }
        })?;

    walk(git, String::new(), tree)?;

    git.file_system.write(
        ".git/HEAD".to_string(),
        format!("ref: refs/heads/{}", branch).as_bytes(),
    )?;

    Ok(())
}

fn walk<F: FileSystem>(git: &mut Git<F>, path: String, tree: object::tree::Tree) -> io::Result<()> {
    tree.contents.into_iter().try_for_each(|x| match x.mode {
        100644 => {
            let file = git
                .read_object(hex::encode(x.hash))
                .and_then(|x| git.cat_file_p(&x))
                .and_then(|x| {
                    if let GitObject::Blob(blob) = x {
                        Ok(blob)
                    } else {
                        Err(io::Error::from(io::ErrorKind::InvalidData))
                    }
                })?;

            git.file_system
                .write(format!("{}{}", path, x.name), file.content.as_bytes())
        }
        40000 => {
            let tree = git
                .read_object(hex::encode(x.hash))
                .and_then(|x| git.cat_file_p(&x))
                .and_then(|x| {
                    if let GitObject::Tree(tree) = x {
                        Ok(tree)
                    } else {
                        Err(io::Error::from(io::ErrorKind::InvalidData))
                    }
                })?;
            walk(git, format!("{}{}/", path, x.name), tree)
        }
        _ => Err(io::Error::from(io::ErrorKind::InvalidData)),
    })
}
