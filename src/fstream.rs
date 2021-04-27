use snafu::{Snafu};
use tokio::sync::mpsc;

mod send;
mod recv;
mod common;

pub use send::Root as SendRoot;
pub use send::Dir as SendDir;
pub use send::DirAction as SendDirAction;
pub use send::FileAction as SendFileAction;
pub use send::Data as SendData;
pub use send::DataAction as SendDataAction;

pub use recv::Root as RecvRoot;
pub use recv::Dir as RecvDir;
//pub use recv::FileEntry as RecvFileEntry;
pub use recv::DirAction as RecvDirAction;
//pub use recv::Data as RecvData;
//pub use recv::DataAction as RecvData;

// new creates sending and receiving halves of
// a channel that can be used to send the contents
// of the directory represented by dir, which must
// be a directory entry. It returns an error if dir
// does not represent a directory.
fn new() -> (send::Root, recv::Root) {
	let (tx, rx) = mpsc::channel(0);
	(send::new_root(tx), recv::new_root(rx))
}
