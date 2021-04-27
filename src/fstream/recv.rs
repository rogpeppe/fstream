//use super::{Action, DirEntry, FsData, FsMsg};
//use snafu::{ensure, Backtrace, ErrorCompat, ResultExt, Snafu};
//use tokio::sync::mpsc;
//use super::super::syncext;
//
//type Receiver = mpsc::Receiver<FsMsg>;
//type Result<T> = std::result::Result<T, Error>;
//
//struct Root {
//	c: Receiver,
//}
//
//impl Root {
//	// dir returns the top level directory entry and
//	// the directory that's underneath it.
//	pub async fn dir(mut self) -> Result<(DirEntry, Dir)> {
//		let (entry, reply) = syncext::recv(&mut self.c).await?;
//		reply.send(Action::Down).await.context(ErrSendAction)?;
//		(entry, Dir{c: self.c})
//	}
//}
//
//#[derive(Debug)]
//pub struct Dir {
//	c: Receiver,
//	depth: i32,
//}
//
//impl Dir {
//	pub async fn entry(mut self) -> Result<Entry> {
//		Ok(match syncext::recv(&mut self.c).await? {
//		(FsData::DirEntry(entry), reply) =>
//			Entry::Dir(DirEntry{
//				entry: entry,
//				c: self.c,
//				reply: reply,
//				depth: self.depth,
//			}),
//		(FsData::FileEntry(stat), reply) =>
//			Entry::File(FileEntry{
//				entry: entry,
//				c: self.c,
//				reply: reply,
//				depth: self.depth,
//			}),
//		(Data(_), _) =>
//			unreachable!("no data allowed at this level"),
//		(End, reply) => {
//				self.reply.send(Action::Next).await?;	// it doesn't actually matter which action we send.
//				if self.depth > 0 {
//					Entry::End(Some(Dir{
//						c: self.c,
//						depth: self.depth-1,
//					}))
//				} else {
//					Entry::End(None)
//				}
//			}
//		})
//	}
//}
//
//enum Entry {
//	File(FileAction),
//	Dir(DirAction),
//	End(Option<Dir>),
//}
//
//struct DirAction {
//	pub entry: DirEntry,
//	c: Receiver,
//	reply: mpsc::Sender<Action>,
//	depth: i32,
//}
//
//impl DirAction {
//	pub async fn down(mut self) -> Result<Dir> {
//		self.reply.send(Action::Down).await?;
//		Ok(Dir{
//			c: c,
//			depth: self.depth+1,
//		})
//	}
//	pub async fn next(self) -> Result<Dir> {
//		self.reply.send(Action::Next).await?;
//		Ok(Dir{
//			c: c,
//			depth: self.depth,
//		})
//	}
//	pub async fn skip(self) -> Result<Option<Dir>> {
//		self.reply.send(Action::Skip).await?;
//		Ok(if self.depth == 0 {
//			None
//		} else {
//			Dir{
//				c: c,
//				depth: self.depth-1,
//			}
//		})
//	}
//}
//
//struct FileAction {
//	c: Receiver,
//}
//
////		impl FileEntry {
////			pub async fn down(self) -> Result<File>
////			pub async fn next(self) -> Result<Dir>
////			pub async fn skip(self) -> Result<Option<Dir>>
////		}
//
////		struct File {}
//
////		impl File {
////			pub async fn data(self) -> Result<Option<(Vec<u8>, File)>> {
////			}
////		}
//
