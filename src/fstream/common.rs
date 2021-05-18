use snafu::Snafu;
use tokio::sync::mpsc;
use tokio::task;

// Action holds an action that a receiver decides
// to take after receiving a value.
#[derive(Debug, Copy, Clone)]
pub enum Action {
    // Down requests that the sender descends into
    // the file or directory.
    Down,
    // Next requests that the sender move onto the
    // next entry in a directory or the next block
    // of data in a file.
    Next,
    // Skip requests that the sender skip the remaining
    // contents of the directory or file.
    Skip,
}

pub type DirEntry = std::fs::DirEntry;

// FsMsg is the value that's sent on the channel.
// It consists of some information about what's being
// sent and a reply channel that the receiver
// sends a reply on to indicate what to do next.
#[derive(Debug)]
pub struct FsMsg {
    pub data: FsData,
    pub reply: mpsc::Sender<Action>,
}

// FsData holds one of the possible items of
// data that can be sent.
#[derive(Debug)]
pub enum FsData {
    // Root represents the root entry, including
    // the full path to the root.
    Root(std::path::PathBuf),

    // FileEntry represents a file. The associated directory entry
    // returns false from is_dir.
    FileEntry(DirEntry),

    // FileEntry represents a directory. The associated directory entry
    // returns true from is_dir.
    DirEntry(DirEntry),

    // Data represents a block of bytes within a file.
    Data(Vec<u8>),

    // End represents the end of a file or directory.
    // The next entry will be from the parent directory if there is one.
    End,
}

#[derive(Debug, Snafu)]
#[snafu(visibility = "pub")]
pub enum Error {
    #[snafu(display("unexpected directory {}", entry.path().display()))]
    ErrIsADirectory { entry: DirEntry },
    #[snafu(display("unexpected non-directory {}", entry.path().display()))]
    ErrNotADirectory { entry: DirEntry },
    #[snafu(display("receiving from unexpectedly closed channel"))]
    ErrUnexpectedClosedChannel,
    #[snafu(display("IO error"))]
    ErrIO { source: std::io::Error },
    #[snafu(display("send on closed channel of type {}", type_name))]
    ErrChanSend { type_name: String },
    #[snafu(display("unexpected message type received"))]
    ErrUnexpectedMessage,
    #[snafu(display("running task failed"))]
    ErrTaskJoin{ source: task::JoinError },
}

impl<T> From<mpsc::error::SendError<T>> for Error {
    fn from(_: mpsc::error::SendError<T>) -> Self {
        ErrChanSend {
            type_name: std::any::type_name::<T>(),
        }
        .build()
    }
}

pub type Result<T> = std::result::Result<T, Error>;

// recv makes it slightly easier to receive from a channel without using ok_or.
pub async fn recv<T>(c: &mut mpsc::Receiver<T>) -> Result<T> {
    c.recv().await.ok_or(ErrUnexpectedClosedChannel.build())
}
