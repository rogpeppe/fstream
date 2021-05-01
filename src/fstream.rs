use tokio::sync::mpsc;

mod send;
mod recv;
mod common;

pub use common::DirEntry;
pub use common::Error;

pub use send::Root as SendRoot;
pub use send::Dir as SendDir;
pub use send::DirEntryAction as SendDirEntryAction;
pub use send::FileEntryAction as SendFileEntryAction;
pub use send::File as SendFile;
pub use send::FileAction as SendFileAction;

pub use recv::Root as RecvRoot;
pub use recv::Dir as RecvDir;
pub use recv::DirEntryAction as RecvDirEntryAction;
pub use recv::FileEntryAction as RecvFileEntryAction;
pub use recv::File as RecvFile;
pub use recv::Data as RecvData;

// new creates sending and receiving halves of
// a channel that can be used to send the contents
// of a directory.
pub fn new() -> (send::Root, recv::Root) {
	let (tx, rx) = mpsc::channel(1);
	(send::new_root(tx), recv::new_root(rx))
}
