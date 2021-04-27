use snafu::Snafu;
use tokio::sync::mpsc;

#[derive(Debug, Snafu)]
pub enum Error {
	#[snafu(display("channel was closed unexpectedly"))]
	ErrUnexpectedClose,
}

// recv makes it slightly easier to receive from a channel without using ok_or.
pub async fn recv<T>(c: &mut mpsc::Receiver<T>) -> Result<T, Error> {
	c.recv().await.ok_or(ErrUnexpectedClose.build())
}
