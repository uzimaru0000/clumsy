use super::{Index, Entry};
use std::collections::HashMap;

#[derive(Debug)]
pub enum Diff {
    Remove(Entry),
    Rename(Entry, Entry),
    Modify(Entry, Entry),
    Add(Entry),
    None
}

struct DiffBuilder {
    flag: bool
}

impl DiffBuilder {
    fn new(flag: bool) -> Self {
        Self {
            flag 
        }
    }

    fn modify(&self, f: Entry, s: Entry) -> Diff {
        if self.flag {
            Diff::Modify(f, s)
        } else {
            Diff::Modify(s, f)
        }
    }

    fn rename(&self, f: Entry, s: Entry) -> Diff {
        if self.flag {
            Diff::Rename(f, s)
        } else {
            Diff::Rename(s, f)
        }
    }

    fn add(&self, f: Entry) -> Diff {
        if self.flag {
            Diff::Add(f)
        } else {
            Diff::Remove(f)
        }
    }

    fn remove(&self, f: Entry) -> Diff {
        if self.flag {
            Diff::Remove(f)
        } else {
            Diff::Add(f)
        }
    }
}

pub fn diff_index(prev: Index, next: Index) -> Vec<Diff> {
    let (
        builder,
        entries_by_name,
        entries_by_hash,
        iter
    ) = if prev.entries.len() <= next.entries.len() {
        (
            DiffBuilder::new(true),
            entries2hashmap(&prev.entries, |x| x.name.clone()),
            entries2hashmap(&prev.entries, |x| hex::encode(&x.hash)),
            next.entries.iter()
        )
    } else {
        (
            DiffBuilder::new(false),
            entries2hashmap(&next.entries, |x| x.name.clone()),
            entries2hashmap(&next.entries, |x| hex::encode(&x.hash)),
            prev.entries.iter()
        )
    };

    let diff = iter.map(|entry| match entries_by_name.get(&entry.name) {
        Some(e) => if entry.hash != e.hash {
            builder.modify(entry.clone(), e.clone())
        } else {
            Diff::None
        },
        None => if let Some(e) = entries_by_hash.get(&hex::encode(&entry.hash)) {
            if let None = entries_by_name.get(&e.name) {
                builder.rename(entry.clone(), e.clone())
            } else {
                builder.add(entry.clone())
            }
        } else {
            builder.remove(entry.clone())
        },
    }).collect();

    diff
}

fn entries2hashmap<F>(entries: &[Entry], key_fn: F) -> HashMap<String, Entry>
    where F: Fn(&Entry) -> String
{
    entries.iter().map(|x| (key_fn(x), x.clone())).collect::<HashMap<_, _>>()
}
