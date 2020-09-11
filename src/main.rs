use clumsy::fs::inmem::InMemFileSystem;
use clumsy::fs::FileSystem;
use clumsy::object::GitObject;
use clumsy::*;
use std::io;

fn main() -> io::Result<()> {
    let mut fs = InMemFileSystem::init();
    fs.write("test.txt".to_string(), b"Hello, World")?;
    let mut git = Git::new(fs);

    add(&mut git, "test.txt".to_string(), b"Hello, World")?;
    commit(&mut git, "init commit".to_string())?;

    let commit = log(&mut git)?;
    println!("{}", commit);

    Ok(())
}

pub fn add<F: FileSystem>(git: &mut Git<F>, file_name: String, bytes: &[u8]) -> io::Result<()> {
    // git hash-object -w path
    let blob = git.hash_object(&bytes).map(GitObject::Blob)?;
    git.write_object(&blob)?;

    // git update-index --add --cacheinfo <mode> <hash> <name>
    let index = git.update_index(&blob.calc_hash(), file_name)?;
    git.write_index(&index)?;

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
