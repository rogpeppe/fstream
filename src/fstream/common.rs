use snafu::{Snafu};
use tokio::sync::mpsc;

#[derive(Debug, Copy, Clone)]
pub enum Action {
	Down,
	Next,
	Skip,
}

pub type DirEntry = std::fs::DirEntry;

#[derive(Debug)]
pub struct FsMsg{
	pub data: FsData,
	pub reply: mpsc::Sender<Action>,
}

#[derive(Debug)]
pub enum FsData {
	// Root represents the root entry.
	Root(String),
	FileEntry(DirEntry),
	DirEntry(DirEntry),
	Data(Vec<u8>),
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
