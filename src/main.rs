use clumsy::object::GitObject;
use clumsy::*;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io::Result;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    match args[1].as_str() {
        "add" => add(args[2].clone()),
        "commit" => commit(args[2].clone()),
        _ => Ok(()),
    }
}

fn add(file_name: String) -> Result<()> {
    let path = env::current_dir().map(|x| x.join(&file_name))?;
    let mut file = File::open(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;

    // git hash-object -w path
    let blob = hash_object(&bytes).map(GitObject::Blob)?;
    write_object(&blob)?;

    // git update-index --add --cacheinfo <mode> <hash> <name>
    let index = update_index(&blob.calc_hash(), file_name)?;
    write_index(&index)?;

    Ok(())
}

fn commit(message: String) -> Result<()> {
    // git write-tree
    let tree = write_tree().map(GitObject::Tree)?;
    write_object(&tree)?;

    let tree_hash = tree.calc_hash();
    // echo message | git commit-tree <hash>
    let commit = commit_tree(
        "uzimaru0000".to_string(),
        "shuji365630@gmail.com".to_string(),
        hex::encode(tree_hash),
        message,
    )
    .map(GitObject::Commit)?;
    write_object(&commit)?;

    // git update-ref refs/heads/master <hash>
    update_ref(head_ref()?, &commit.calc_hash())?;

    Ok(())
}
