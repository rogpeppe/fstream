use snafu::{ensure, Backtrace, ErrorCompat, ResultExt, Snafu};
use tokio::sync::mpsc;

mod syncext;
mod send;
mod recv;

#[derive(Debug, Copy, Clone)]
enum Action {
	Down,
	Next,
	Skip,
}

type DirEntry = std::fs::DirEntry;

#[derive(Debug)]
struct FsMsg(FsData, mpsc::Sender<Action>);

#[derive(Debug)]
enum FsData {
	// Root represents the root entry.
	Root(String),
	FileEntry(DirEntry),
	DirEntry(DirEntry),
	Data(Vec<u8>),
	End,
}

#[derive(Debug, Snafu)]
pub enum Error {
	#[snafu(display("unexpected directory {}", entry.path().display()))]
	ErrIsADirectory { entry: DirEntry },
	#[snafu(display("unexpected non-directory {}", entry.path().display()))]
	ErrNotADirectory { entry: DirEntry },
	#[snafu(display("receiving from closed channel"))]
	ErrUnexpectedClosedChannel,
	#[snafu(display("IO error"))]
	ErrIO { source: std::io::Error },
	#[snafu(display("send on closed channel of type {}", type_name))]
	ErrChanSend { type_name: String },
}

impl From<syncext::Error> for Error {
	fn from(_: syncext::Error) -> Self {
		ErrUnexpectedClosedChannel.build()
	}
}

impl<T> From<mpsc::error::SendError<T>> for Error {
	fn from(_: mpsc::error::SendError<T>) -> Self {
		ErrChanSend {
			type_name: std::any::type_name::<T>(),
		}
		.build()
	}
}

type Result<T> = std::result::Result<T, Error>;

// new creates sending and receiving halves of
// a channel that can be used to send the contents
// of the directory represented by dir, which must
// be a directory entry. It returns an error if dir
// does not represent a directory.
//	fn new() -> Result<(send.Root, recv.Root)> {
//		let tx, rx = mpsc::channel(0);
//		(send.new_root(tx), recv.new_root(rx))
//	}



