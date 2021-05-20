use tokio::sync::mpsc;

mod common;
mod recv;
mod send;

pub use common::*;

pub use send::Dir as SendDir;
pub use send::DirEntryAction as SendDirEntryAction;
pub use send::File as SendFile;
pub use send::FileAction as SendFileAction;
pub use send::FileEntryAction as SendFileEntryAction;
pub use send::Root as SendRoot;

pub use recv::Data as RecvData;
pub use recv::Dir as RecvDir;
pub use recv::DirEntryAction as RecvDirEntryAction;
pub use recv::Entry as RecvEntry;
pub use recv::File as RecvFile;
pub use recv::FileEntryAction as RecvFileEntryAction;
pub use recv::Root as RecvRoot;

// new creates sending and receiving halves of
// a channel that can be used to send the contents
// of a directory.
pub fn new() -> (send::Root, recv::Root) {
    let (tx, rx) = mpsc::channel(1);
    (send::new_root(tx), recv::new_root(rx))
}
