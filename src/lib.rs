pub mod fs;
pub mod index;
pub mod object;

use chrono::{Local, TimeZone, Utc};
use fs::FileSystem;
use index::{Entry, Index};
use index::diff::{diff_index, Diff};
use libflate::zlib::{Decoder, Encoder};
use object::blob::Blob;
use object::commit;
use object::commit::Commit;
use object::tree;
use object::tree::Tree;
use object::GitObject;
use std::io;
use std::io::prelude::*;

#[derive(Debug)]
pub struct Git<F: FileSystem> {
    pub file_system: F,
}

impl<F: FileSystem> Git<F> {
    pub fn new(file_system: F) -> Self {
        Self { file_system }
    }

    pub fn read_index(&self) -> io::Result<Vec<u8>> {
        self.file_system.read(".git/index".to_string())
    }

    pub fn write_index(&mut self, index: &Index) -> io::Result<()> {
        self.file_system
            .write(".git/index".to_string(), &index.as_bytes())
    }

    pub fn read_object(&self, hash: String) -> io::Result<Vec<u8>> {
        let (sub_dir, file) = hash.split_at(2);
        self.file_system
            .read(format!(".git/objects/{}/{}", sub_dir, file))
    }

    pub fn write_object(&mut self, object: &GitObject) -> io::Result<()> {
        let hash = hex::encode(object.calc_hash());
        let (sub_dir, file) = hash.split_at(2);

        let path = format!(".git/objects/{}", sub_dir);
        // ディレクトリがなかったら
        if let Err(_) = self.file_system.stat(path.clone()) {
            self.file_system.create_dir(path.clone())?;
        }

        let path = format!("{}/{}", path, file);

        let mut encoder = Encoder::new(Vec::new())?;
        encoder.write_all(&object.as_bytes())?;
        let bytes = encoder.finish().into_result()?;

        self.file_system.write(path, &bytes)
    }

    pub fn head_ref(&self) -> io::Result<String> {
        let path = ".git/HEAD".to_string();
        let file = self.file_system.read(path)?;
        let refs =
            String::from_utf8(file).map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;

        let (prefix, path) = refs.split_at(5);

        if prefix != "ref: " {
            return Err(io::Error::from(io::ErrorKind::InvalidData));
        }

        Ok(path.trim().to_string())
    }

    pub fn read_ref(&self, path: String) -> io::Result<String> {
        let path = format!(".git/{}", path);
        let file = self.file_system.read(path)?;
        let hash =
            String::from_utf8(file).map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;

        Ok(hash.trim().to_string())
    }

    pub fn write_ref(&mut self, path: String, hash: &[u8]) -> io::Result<()> {
        let path = format!(".git/{}", path);
        self.file_system.write(path, hex::encode(hash).as_bytes())
    }

    pub fn cat_file_p(&self, bytes: &[u8]) -> io::Result<GitObject> {
        let mut d = Decoder::new(&bytes[..])?;
        let mut buf = Vec::new();
        d.read_to_end(&mut buf)?;

        GitObject::new(&buf).ok_or(io::Error::from(io::ErrorKind::InvalidData))
    }

    pub fn ls_files_stage(&self, bytes: &[u8]) -> io::Result<Index> {
        Index::from(&bytes).ok_or(io::Error::from(io::ErrorKind::InvalidData))
    }

    pub fn hash_object(&self, bytes: &[u8]) -> io::Result<Blob> {
        let blob = Blob::from(&bytes).ok_or(io::Error::from(io::ErrorKind::InvalidInput))?;
        Ok(blob)
    }

    pub fn update_index(&self, idx: Index, hash: &[u8], file_name: String) -> io::Result<Index> {
        let metadata = self.file_system.stat(file_name.clone())?;
        let entry = Entry::new(
            Utc.timestamp(metadata.ctime as i64, metadata.ctime_nsec),
            Utc.timestamp(metadata.mtime as i64, metadata.mtime_nsec),
            metadata.dev,
            metadata.ino,
            metadata.mode,
            metadata.uid,
            metadata.gid,
            metadata.size,
            Vec::from(hash),
            file_name.clone(),
        );

        let mut entries: Vec<Entry> = idx 
            .entries
            .into_iter()
            .filter(|x| x.name != entry.name && x.hash != entry.hash)
            .collect();
        entries.push(entry);
        entries.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(Index::new(entries))
    }

    pub fn write_tree(&self) -> io::Result<Tree> {
        let bytes = self.read_index()?;
        let index = self.ls_files_stage(&bytes)?;

        let contents = index
            .entries
            .iter()
            // TODO: 一旦modeは `100644` で固定
            .map(|x| tree::File::new(100644, x.name.clone(), &x.hash))
            .collect::<Vec<_>>();

        Ok(Tree::new(contents))
    }

    pub fn commit_tree(
        &self,
        name: String,
        email: String,
        tree_hash: String,
        message: String,
    ) -> io::Result<Commit> {
        let parent = self.head_ref().and_then(|x| self.read_ref(x)).ok();
        let offs = {
            let local = Local::now();
            *local.offset()
        };
        let ts = offs.from_utc_datetime(&Utc::now().naive_utc());
        let author = commit::User::new(name.clone(), email.clone(), ts);
        let commit = Commit::new(tree_hash, parent, author.clone(), author.clone(), message);

        Ok(commit)
    }

    pub fn update_ref(&mut self, path: String, hash: &[u8]) -> io::Result<()> {
        self.write_ref(path, hash)
    }

    pub fn reset_index(&mut self, hash: String) -> io::Result<Vec<Diff>> {
        let commit = self
            .read_object(hash)
            .and_then(|x| self.cat_file_p(&x))
            .and_then(|x| match x {
                GitObject::Commit(commit) => Ok(commit),
                _ => Err(io::Error::from(io::ErrorKind::InvalidData)),
            })?;

        let prev_index = self.read_index().and_then(|x| self.ls_files_stage(&x))?;
        let next_index = self.tree2index(commit.tree.clone())?;

        Ok(diff_index(prev_index, next_index))
    }

    pub fn diff_apply(&mut self, diff: Vec<Diff>) -> io::Result<()> {
        diff.iter().try_for_each(|d| match d {
            Diff::Add(e) => 
                self.read_object(hex::encode(e.hash.clone()))
                    .and_then(|x| self.cat_file_p(&x))
                    .and_then(|x| match x {
                        GitObject::Blob(blob) => Ok(blob),
                        _ => Err(io::Error::from(io::ErrorKind::InvalidData)),
                    })
                    .and_then(|blob| self.file_system.write(e.name.clone(), blob.content.as_bytes())),
            Diff::Modify(e, _) =>
                self.read_object(hex::encode(e.hash.clone()))
                    .and_then(|x| self.cat_file_p(&x))
                    .and_then(|x| match x {
                        GitObject::Blob(blob) => Ok(blob),
                        _ => Err(io::Error::from(io::ErrorKind::InvalidData)),
                    })
                    .and_then(|blob| self.file_system.write(e.name.clone(), blob.content.as_bytes())),
            Diff::Rename(n, p) => self.file_system.rename(p.name.clone(), n.name.clone()),
            Diff::Remove(e) => self.file_system.remove(e.name.clone()),
            Diff::None => Ok(()),
        })
    }

    pub fn tree2index(&mut self, hash: String) -> io::Result<Index>
    {
        let idx = Index::new(Vec::new());
        self.helper_tree2index(idx, hash, String::new())
    }

    fn helper_tree2index(&mut self, idx: Index, hash: String, name: String) -> io::Result<Index> {
        let obj = self
            .read_object(hash.clone())
            .and_then(|x| self.cat_file_p(&x))?;

        match obj {
            GitObject::Blob(blob) => {
                let meta = self.file_system.stat(name.clone()).unwrap_or(fs::Metadata {
                    dev: 0,
                    ino: 0,
                    mode: 33188,
                    uid: 0,
                    gid: 0,
                    size: 0,
                    mtime: 0,
                    mtime_nsec: 0,
                    ctime: 0,
                    ctime_nsec: 0,
                });

                let entry = Entry::new(
                    Utc.timestamp(meta.ctime as i64, meta.ctime_nsec),
                    Utc.timestamp(meta.mtime as i64, meta.mtime_nsec),
                    meta.dev,
                    meta.ino,
                    meta.mode,
                    meta.uid,
                    meta.gid,
                    meta.size,
                    blob.calc_hash(),
                    name
                );

                let mut entries: Vec<Entry> = idx 
                    .entries
                    .into_iter()
                    .filter(|x| x.name != entry.name || x.hash != entry.hash)
                    .collect();
                entries.push(entry);
                entries.sort_by(|a, b| a.name.cmp(&b.name));

                Ok(Index::new(entries))
            },
            GitObject::Tree(tree) => tree.contents.iter().try_fold(idx, |acc, x| {
                self.helper_tree2index(acc, hex::encode(&x.hash), format!("{}{}{}", name, if name.is_empty() { "" } else { "/" }, x.name.clone()))
            }),
            _ => Err(io::Error::from(io::ErrorKind::InvalidData)),
        }
    }
}

