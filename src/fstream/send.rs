use super::common;
use snafu::{ResultExt};
use tokio::sync::mpsc;

pub type Sender = mpsc::Sender<common::FsMsg>;

#[derive(Debug)]
pub struct Dir {
	depth_n: i32,
	c: Sender,
	reply_tx: mpsc::Sender<common::Action>,
	reply_rx: mpsc::Receiver<common::Action>,
}

pub struct Root {
	dir: Dir,
}

// TODO how can we make this available only to the fstream module?
pub fn new_root(c: Sender) -> Root {
	let (reply_tx, reply_rx) = mpsc::channel(1);
	Root {
		dir: Dir {
			depth_n: 0,
			c: c,
			reply_tx: reply_tx,
			reply_rx: reply_rx,
		},
	}
}

impl Root {
	pub async fn dir(mut self, path: std::path::PathBuf) -> common::Result<Option<Dir>> {
		self.dir.c
			.send(common::FsMsg{
				data: common::FsData::Root(path),
				reply: self.dir.reply_tx.clone(),
			})
			.await?;
		Ok(match common::recv(&mut self.dir.reply_rx).await? {
			common::Action::Down => Some(self.dir.down()),
			_ => None,
		})
	}
}

impl Dir {
	// file sends a file entry. The name should always compare
	// greater than the previous entry sent for the directory.
	// It's an error if entry represents a directory.
	pub async fn file(mut self, entry: common::DirEntry) -> common::Result<FileEntryAction> {
		if Self::is_dir(&entry)? {
			return common::ErrNotADirectory { entry }.fail();
		}
		self.c
			.send(common::FsMsg{
				data: common::FsData::FileEntry(entry),
				reply: self.reply_tx.clone(),
			})
			.await?;
		Ok(match common::recv(&mut self.reply_rx).await? {
			common::Action::Down => FileEntryAction::Down(File {
				dir: self.down(),
			}),
			common::Action::Skip => {
				if let Some(parent) = self.up() {
					FileEntryAction::Skip(parent)
				} else {
					FileEntryAction::End
				}
			},
			common::Action::Next => FileEntryAction::Next(self),
		})
	}

	pub fn depth(&self) -> i32 {
		self.depth_n
	}

	fn is_dir(entry: &common::DirEntry) -> common::Result<bool> {
		Ok(entry.metadata().context(common::ErrIO)?.is_dir())
	}

	// dir sends a directory entry. The name should always compare
	// greater than the previous entry sent for the directory.
	// It's an error if entry doesn't represent a directory.
	pub async fn dir(mut self, entry: common::DirEntry) -> common::Result<DirEntryAction> {
		if !Self::is_dir(&entry)? {
			return common::ErrNotADirectory { entry }.fail();
		}
		self.c
			.send(common::FsMsg{
				data: common::FsData::DirEntry(entry),
				reply: self.reply_tx.clone(),
			})
			.await?;
		Ok(match common::recv(&mut self.reply_rx).await? {
			common::Action::Down => DirEntryAction::Down(self.down()),
			common::Action::Skip => {
				if let Some(parent) = self.up() {
					DirEntryAction::Skip(parent)
				} else {
					DirEntryAction::End
				}
			}
			common::Action::Next => DirEntryAction::Next(self),
		})
	}

	// end indicates the end of the directory. It returns the parent
	// directory or None if the parent is the root.
	pub async fn end(mut self) -> common::Result<Option<Dir>> {
		self.c
			.send(common::FsMsg{
				data: common::FsData::End,
				reply: self.reply_tx.clone(),
			})
			.await?;
		// Note: it doesn't matter what the response is to an end-of-directory.
		common::recv(&mut self.reply_rx).await?;
		Ok(self.up())
	}

	fn down(self) -> Dir {
		Dir {
			depth_n: self.depth_n + 1,
			c: self.c,
			reply_tx: self.reply_tx,
			reply_rx: self.reply_rx,
		}
	}
	fn up(self) -> Option<Dir> {
		if self.depth_n <= 1 {
			None
		} else {
			Some(Dir {
				depth_n: self.depth_n - 1,
				c: self.c,
				reply_tx: self.reply_tx,
				reply_rx: self.reply_rx,
			})
		}
	}
}

#[derive(Debug)]
pub enum DirEntryAction {
	// Down descends into the directory beneath this entry.
	// Dir is used to send the contents of the directory.
	Down(Dir),
	// Next goes to the next entry in the current directory.
	// Dir is used to send the rest of the current directory.
	Next(Dir),
	// Skip skips to the end of the current directory.
	// Dir is used to send the rest of the parent directory.
	Skip(Dir),
	// End indicates the end of sending, when
	// there can be no more entries sent.
	End,
}

#[derive(Debug)]
pub enum FileEntryAction {
	// Down descends into the file contents beneath
	// this entry. File is used to send the actual data.
	Down(File),
	// Next moves on to the next entry in the current directory.
	// Dir is used to send the rest of the current directory.
	Next(Dir),
	// Skip skips to the end of the current directory.
	// Dir is used to send the rest of the parent directory,
	// or None if there is no parent directory.
	Skip(Dir),
	// End indicates the end of sending, when
	// there can be no more entries sent.
	End,
}

#[derive(Debug)]
pub struct File {
	dir: Dir,
}

impl File {
	pub async fn data(mut self, b: Vec<u8>) -> common::Result<FileAction> {
		self.dir.c
			.send(common::FsMsg{
				data: common::FsData::Data(b),
				reply: self.dir.reply_tx.clone(),
			})
			.await?;
		Ok(match common::recv(&mut self.dir.reply_rx).await? {
			common::Action::Down | common::Action::Next =>
				FileAction::Next(self),
			common::Action::Skip =>
				FileAction::Skip(self.dir.up().unwrap()),
		})
	}

	pub async fn end(mut self) -> common::Result<Dir> {
		self.dir.c
			.send(common::FsMsg{
				data: common::FsData::End,
				reply: self.dir.reply_tx.clone(),
			})
			.await?;
		// Note: it doesn't matter what the response is to an end-of-file.
		common::recv(&mut self.dir.reply_rx).await?;
		Ok(self.dir.up().unwrap())
	}
}

#[derive(Debug)]
pub enum FileAction {
	Next(File),
	Skip(Dir),
}
