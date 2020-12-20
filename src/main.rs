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
        "switch" => switch(&mut git, args.get(2).unwrap().clone()),
        "log" => {
            let obj = log(&mut git)?;
            obj.iter().for_each(|x| println!("{}", x));
            Ok(())
        },
        _ => Ok(()),
    }
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
    let index = git.read_index().and_then(|x| git.ls_files_stage(&x))?;
    let index = git.update_index(index, &blob.calc_hash(), file_name)?;
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

fn log<F: FileSystem>(git: &mut Git<F>) -> io::Result<Vec<GitObject>> {
    let commit = git
        .head_ref()
        .and_then(|x| git.read_ref(x))
        .and_then(|x| git.read_object(x))
        .and_then(|x| git.cat_file_p(&x))?;

    Ok((0..)
        .scan(Some(commit), |st, _| {
            let next = match st {
                Some(GitObject::Commit(commit)) => {
                    if let Some(parent) = &commit.parent {
                        git
                            .read_object(parent.clone())
                            .and_then(|x| git.cat_file_p(&x))
                            .ok()
                    } else {
                        None
                    }
                }
                _ => None,
            };
            let curr = st.clone();
            *st = next;
            curr
        })
        .collect::<Vec<_>>())
}

fn switch<F: FileSystem>(git: &mut Git<F>, branch: String) -> io::Result<()> {
    let commit_hash = git.read_ref(format!("refs/heads/{}", branch))?;
    let diff = git.reset_index(commit_hash.clone())?;

    git.diff_apply(diff)?;

    let commit = git
        .read_object(commit_hash)
        .and_then(|x| git.cat_file_p(&x))
        .and_then(|x| match x {
            GitObject::Commit(commit) => Ok(commit),
            _ => Err(io::Error::from(io::ErrorKind::InvalidData)),
        })?;
    let idx = git.tree2index(commit.tree)?;

    git.file_system.write(
        ".git/HEAD".to_string(),
        format!("ref: refs/heads/{}", branch).as_bytes(),
    )?;

    git.write_index(&idx)?;

    Ok(())
}
