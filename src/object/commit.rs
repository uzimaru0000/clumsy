use chrono::{DateTime, FixedOffset, TimeZone, Utc};
#[cfg(feature = "json")]
use serde::Serialize;
use sha1::{Digest, Sha1};
use std::fmt;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "json", derive(Serialize))]
pub struct User {
    pub name: String,
    pub email: String,
    pub ts: DateTime<FixedOffset>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "json", derive(Serialize))]
pub struct Commit {
    pub tree: String,
    pub parent: Option<String>,
    pub author: User,
    pub committer: User,
    pub message: String,
}

impl User {
    pub fn new(name: String, email: String, ts: DateTime<FixedOffset>) -> Self {
        Self { name, email, ts }
    }

    pub fn from(bytes: &[u8]) -> Option<Self> {
        let name = String::from_utf8(
            bytes
                .into_iter()
                .take_while(|&&x| x != b'<')
                .map(|&x| x)
                .collect(),
        )
        .map(|x| String::from(x.trim()))
        .ok()?;

        let info = String::from_utf8(
            bytes
                .into_iter()
                .skip_while(|&&x| x != b'<')
                .map(|&x| x)
                .collect(),
        )
        .ok()?;

        let mut info_iter = info.splitn(3, " ");

        let email = info_iter
            .next()
            .map(|x| String::from(x.trim_matches(|x| x == '<' || x == '>')))?;
        let ts = Utc.timestamp(info_iter.next().and_then(|x| x.parse::<i64>().ok())?, 0);
        let offset = info_iter
            .next()
            .and_then(|x| x.parse::<i32>().ok())
            .map(|x| {
                if x < 0 {
                    FixedOffset::west(x / 100 * 60 * 60)
                } else {
                    FixedOffset::east(x / 100 * 60 * 60)
                }
            })?;

        Some(Self::new(
            name,
            email,
            offset.from_utc_datetime(&ts.naive_utc()),
        ))
    }
}

impl fmt::Display for User {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} <{}> {} {:+05}",
            self.name,
            self.email,
            self.ts.timestamp(),
            self.ts.offset().local_minus_utc() / 36
        )
    }
}

impl Commit {
    pub fn new(
        tree: String,
        parent: Option<String>,
        author: User,
        committer: User,
        message: String,
    ) -> Self {
        Self {
            tree,
            parent,
            author,
            committer,
            message,
        }
    }

    pub fn from(bytes: &[u8]) -> Option<Self> {
        let mut iter = bytes.split(|&x| x == b'\n').filter(|x| x != b"");

        let tree = iter
            .next()
            .map(|x| {
                x.splitn(2, |&x| x == b' ')
                    .skip(1)
                    .flatten()
                    .map(|&x| x)
                    .collect::<Vec<_>>()
            })
            .and_then(|x| String::from_utf8(x).ok())?;

        let parent = &iter
            .next()
            .map(|x| {
                x.splitn(2, |&x| x == b' ')
                    .map(Vec::from)
                    .map(|x| String::from_utf8(x).ok().unwrap_or_default())
                    .collect::<Vec<_>>()
            })
            .ok_or(Vec::new())
            .and_then(|x| match x[0].as_str() {
                "parent" => Ok(x[1].clone()),
                _ => Err([x[0].as_bytes(), b" ", x[1].as_bytes()].concat()),
            });

        let author = match parent {
            Ok(_) => iter.next().map(|x| Vec::from(x)),
            Err(v) => Some(v.clone()),
        }
        .map(|x| {
            x.splitn(2, |&x| x == b' ')
                .skip(1)
                .flatten()
                .map(|&x| x)
                .collect::<Vec<_>>()
        })
        .and_then(|x| User::from(x.as_slice()))?;

        let committer = iter
            .next()
            .map(|x| {
                x.splitn(2, |&x| x == b' ')
                    .skip(1)
                    .flatten()
                    .map(|&x| x)
                    .collect::<Vec<_>>()
            })
            .and_then(|x| User::from(x.as_slice()))?;

        let message = iter
            .next()
            .map(Vec::from)
            .and_then(|x| String::from_utf8(x).ok())?;

        Some(Self::new(
            tree,
            parent.clone().ok(),
            author,
            committer,
            message,
        ))
    }

    pub fn calc_hash(&self) -> Vec<u8> {
        Vec::from(Sha1::digest(&self.as_bytes()).as_slice())
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let content = format!("{}", self);
        let header = format!("commit {}\0", content.len());
        let val = format!("{}{}", header, content);

        Vec::from(val.as_bytes())
    }
}

impl fmt::Display for Commit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let tree = format!("tree {}", self.tree);
        let parent = self
            .parent
            .clone()
            .map(|x| format!("parent {}\n", x))
            .unwrap_or_default();
        let author = format!("author {}", self.author);
        let committer = format!("committer {}", self.committer);

        write!(
            f,
            "{}\n{}{}\n{}\n\n{}\n",
            tree, parent, author, committer, self.message
        )
    }
}
