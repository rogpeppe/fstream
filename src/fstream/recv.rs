use super::common::*;
//use snafu::{Snafu, ResultExt};
use tokio::sync::mpsc;

type Receiver = mpsc::Receiver<FsMsg>;

// TODO how can we make this available only to the fstream module?
pub fn new_root(c: Receiver) -> Root {
	Root {
		c: c,
	}
}

pub struct Root {
	c: Receiver,
}

impl Root {
	// dir returns the top level directory entry and
	// the directory that's underneath it.
	pub async fn dir(mut self) -> Result<(DirEntry, Dir)> {
		let msg = recv(&mut self.c).await?;
		if let FsData::DirEntry(entry) = msg.data {
			msg.reply.send(Action::Down).await?;
			Ok((entry, Dir{c: self.c, depth: 1}))
		} else {
			// TODO more specific error.
			Err(ErrUnexpectedMessage.build())
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
		Dir{
			c: self.c,
			depth: self.depth+1,
		}
	}
	fn up(self) -> Option<Dir> {
		if self.depth > 1 {
			Some(Dir{
				c: self.c,
				depth: self.depth-1,
			})
		} else {
			None
		}
	}
	pub async fn entry(mut self) -> Result<Entry> {
		let msg = recv(&mut self.c).await?;
		Ok(match msg.data {
		FsData::DirEntry(entry) =>
			Entry::Dir(entry, DirAction{
				dir: self,
				reply: msg.reply,
			}),
		FsData::FileEntry(entry) =>
			Entry::File(entry, FileAction{
				dir: self,
				reply: msg.reply,
			}),
		FsData::Data(_) =>
			unreachable!("no data allowed at this level"),
		FsData::Root(_) =>
			unreachable!("root not allowed at this level"),
		FsData::End => {
				msg.reply.send(Action::Next).await?;	// it doesn't actually matter which action we send.
				Entry::End(self.up())
			}
		})
	}
}

pub enum Entry {
	File(DirEntry, FileAction),
	Dir(DirEntry, DirAction),
	End(Option<Dir>),
}

pub struct DirAction {
	dir: Dir,
	reply: mpsc::Sender<Action>,
}

impl DirAction {
	pub async fn down(self) -> Result<Dir> {
		self.reply.send(Action::Down).await?;
		Ok(self.dir.down())
	}
	pub async fn next(self) -> Result<Dir> {
		self.reply.send(Action::Next).await?;
		Ok(self.dir)
	}
	pub async fn skip(self) -> Result<Option<Dir>> {
		self.reply.send(Action::Skip).await?;
		Ok(self.dir.up())
	}
}

pub struct FileAction {
	dir: Dir,
	reply: mpsc::Sender<Action>,
}

impl FileAction {
	pub async fn down(self) -> Result<File> {
		self.reply.send(Action::Down).await?;
		Ok(File{
			dir: self.dir.down(),
		})
	}
	pub async fn next(self) -> Result<Dir> {
		self.reply.send(Action::Next).await?;
		Ok(self.dir)
	}
	pub async fn skip(self) -> Result<Option<Dir>> {
		self.reply.send(Action::Skip).await?;
		Ok(self.dir.up())
	}
}

struct File {
	dir: Dir,
}

enum Data {
	Bytes(Vec<u8>, File),
	End(Dir),
}

impl File {
	pub async fn data(mut self) -> Result<Data> {
		let msg = recv(&mut self.dir.c).await?;
		Ok(match msg.data {
		FsData::Data(data) =>
			Data::Bytes(data, self),
		FsData::End =>
			// Note: the up call can't fail because files are at least two levels deep.
			Data::End(self.dir.up().unwrap()),
		_ =>
			unreachable!("unexpected message received"),
		})
	}
	pub async fn skip(self) -> Result<Dir> {
		// Note: the up call can't fail because files are at least two levels deep.
		Ok(self.dir.up().unwrap())
	}
}
