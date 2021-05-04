use super::common;
//use snafu::{Snafu, ResultExt};
use tokio::sync::mpsc;

type Receiver = mpsc::Receiver<common::FsMsg>;

// TODO how can we make this available only to the fstream module?
pub fn new_root(c: Receiver) -> Root {
    Root { c: c }
}

#[derive(Debug)]
pub struct Root {
    c: Receiver,
}

impl Root {
    // dir returns the top level directory entry and
    // the directory that's underneath it.
    pub async fn dir(mut self) -> common::Result<(std::path::PathBuf, Dir)> {
        let msg = common::recv(&mut self.c).await?;
        if let common::FsData::Root(path) = msg.data {
            msg.reply.send(common::Action::Down).await?;
            Ok((
                path,
                Dir {
                    c: self.c,
                    depth: 1,
                },
            ))
        } else {
            // TODO more specific error.
            Err(common::ErrUnexpectedMessage.build())
        }
    }
}

#[derive(Debug)]
pub struct Dir {
    c: Receiver,
    depth: i32,
}

impl Dir {
    fn down(self) -> Dir {
        Dir {
            c: self.c,
            depth: self.depth + 1,
        }
    }
    fn up(self) -> Option<Dir> {
        if self.depth > 1 {
            Some(Dir {
                c: self.c,
                depth: self.depth - 1,
            })
        } else {
            None
        }
    }
    pub async fn entry(mut self) -> common::Result<Entry> {
        let msg = common::recv(&mut self.c).await?;
        Ok(match msg.data {
            common::FsData::DirEntry(entry) => Entry::Dir(
                entry,
                DirEntryAction {
                    dir: self,
                    reply: msg.reply,
                },
            ),
            common::FsData::FileEntry(entry) => Entry::File(
                entry,
                FileEntryAction {
                    dir: self,
                    reply: msg.reply,
                },
            ),
            common::FsData::Data(_) => unreachable!("no data allowed at this level"),
            common::FsData::Root(_) => unreachable!("root not allowed at this level"),
            common::FsData::End => {
                msg.reply.send(common::Action::Next).await?; // it doesn't actually matter which action we send.
                Entry::End(self.up())
            }
        })
    }
}

#[derive(Debug)]
pub enum Entry {
    File(common::DirEntry, FileEntryAction),
    Dir(common::DirEntry, DirEntryAction),
    End(Option<Dir>),
}

#[derive(Debug)]
pub struct DirEntryAction {
    dir: Dir,
    reply: mpsc::Sender<common::Action>,
}

impl DirEntryAction {
    pub async fn down(self) -> common::Result<Dir> {
        self.reply.send(common::Action::Down).await?;
        Ok(self.dir.down())
    }
    pub async fn next(self) -> common::Result<Dir> {
        self.reply.send(common::Action::Next).await?;
        Ok(self.dir)
    }
    pub async fn skip(self) -> common::Result<Option<Dir>> {
        self.reply.send(common::Action::Skip).await?;
        Ok(self.dir.up())
    }
}

#[derive(Debug)]
pub struct FileEntryAction {
    dir: Dir,
    reply: mpsc::Sender<common::Action>,
}

impl FileEntryAction {
    pub async fn down(self) -> common::Result<File> {
        self.reply.send(common::Action::Down).await?;
        Ok(File {
            dir: self.dir.down(),
        })
    }
    pub async fn next(self) -> common::Result<Dir> {
        self.reply.send(common::Action::Next).await?;
        Ok(self.dir)
    }
    pub async fn skip(self) -> common::Result<Option<Dir>> {
        self.reply.send(common::Action::Skip).await?;
        Ok(self.dir.up())
    }
}

#[derive(Debug)]
pub struct File {
    dir: Dir,
}

#[derive(Debug)]
pub enum Data {
    Bytes(Vec<u8>, File),
    End(Dir),
}

impl File {
    pub async fn data(mut self) -> common::Result<Data> {
        let msg = common::recv(&mut self.dir.c).await?;
        Ok(match msg.data {
            common::FsData::Data(data) => Data::Bytes(data, self),
            common::FsData::End =>
            // Note: the up call can't fail because files are at least two levels deep.
            {
                Data::End(self.dir.up().unwrap())
            }
            _ => unreachable!("unexpected message received"),
        })
    }
    pub async fn skip(self) -> common::Result<Dir> {
        // Note: the up call can't fail because files are at least two levels deep.
        Ok(self.dir.up().unwrap())
    }
}
