
#[tokio::main]
async fn main() {}

mod fstream {
    use tokio::sync::mpsc;
    use snafu::{ensure, Backtrace, ErrorCompat, ResultExt, Snafu};

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

    // new creates sending and receiving halves of
    // a channel that can be used to send the contents
    // of the directory represented by dir, which must
    // be a directory entry. It returns an error if dir
    // does not represent a directory.
    //	fn new() -> Result<(send.Root, recv.Root)> {
    //		let tx, rx = mpsc::channel(0);
    //		(send.new_root(tx), recv.new_root(rx))
    //	}

    mod send {
        use super::{Action, DirEntry, FsData, FsMsg};
        use snafu::{ensure, Backtrace, ErrorCompat, ResultExt, Snafu};
        use tokio::sync::mpsc;

        type Sender = mpsc::Sender<FsMsg>;

        type Result<T> = std::result::Result<T, Error>;

        #[derive(Debug)]
        pub struct Dir {
            depth_n: i32,
            c: Sender,
            reply_tx: mpsc::Sender<Action>,
            reply_rx: mpsc::Receiver<Action>,
        }

        pub struct Root {
            dir: Dir,
        }

        #[derive(Debug, Snafu)]
        pub enum Error {
            #[snafu(display("Could not open config"))]
            ErrIsADirectory{
            	entry: DirEntry,
            },
            ErrNotADirectory{
            	entry: DirEntry,
            },
             ErrUnexpectedClosedChannel,
             ErrIO{
            	source: std::io::Error,
            },
             ErrSendFsMsg{
                source: tokio::sync::mpsc::error::SendError<FsMsg>,
            },
        }

        //impl From<tokio::sync::mpsc::error::SendError<FsMsg>> for Error {
       //    fn from(e: tokio::sync::mpsc::error::SendError<FsMsg>) -> Error {
       //         Error::Other(Box::new(e))
       //    }
       //}

        fn new_root(c: Sender) -> Root {
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
            pub async fn dir(self, entry: DirEntry) -> Result<Option<Dir>> {
                match self.dir.dir(entry).await? {
                    DirAction::Down(dir) => Ok(Some(dir)),
                    _ => Ok(None),
                }
            }
        }

        impl Dir {
            // file sends a file entry. The name should always compare
            // greater than the previous entry sent for the directory.
            // It's an error if entry represents a directory.
            pub async fn file(mut self, entry: DirEntry) -> Result<FileAction> {
                if Self::is_dir(&entry)? {
                    return ErrNotADirectory{entry}.fail();
                }
                self.c
                    .send(FsMsg(FsData::FileEntry(entry), self.reply_tx.clone()))
                    .await.context(ErrSendFsMsg)?;
                Ok(match self.reply_rx.recv().await.ok_or(ErrUnexpectedClosedChannel.build())? {
                    Action::Down => FileAction::Down(Data {
                        c: self.c,
                        depth_n: self.depth_n + 1,
                        reply_rx: self.reply_rx,
                        reply_tx: self.reply_tx,
                    }),
                    Action::Skip => {
                        if let Some(parent) = self.parent() {
                            FileAction::Skip(parent)
                        } else {
                            FileAction::End
                        }
                    }
                    Action::Next => FileAction::Next(self),
                })
            }

            pub fn depth(&self) -> i32 {
                self.depth_n
            }

            fn is_dir(entry: &DirEntry) -> Result<bool> {
                Ok(entry.metadata().context(ErrIO)?.is_dir())
            }

            // dir sends a directory entry. The name should always compare
            // greater than the previous entry sent for the directory.
            // It's an error if entry doesn't represent a directory.
            pub async fn dir(mut self, entry: DirEntry) -> Result<DirAction> {
                if !Self::is_dir(&entry)? {
                    return ErrNotADirectory{entry}.fail();
                }
                self.c
                    .send(FsMsg(FsData::DirEntry(entry), self.reply_tx.clone()))
                    .await.context(ErrSendFsMsg)?;
                Ok(match self.reply_rx.recv().await.ok_or(ErrUnexpectedClosedChannel.build())? {
                    Action::Down => DirAction::Down(Dir {
                        depth_n: self.depth_n + 1,
                        c: self.c,
                        reply_rx: self.reply_rx,
                        reply_tx: self.reply_tx,
                    }),
                    Action::Skip => {
                        if let Some(parent) = self.parent() {
                            DirAction::Skip(parent)
                        } else {
                            DirAction::End
                        }
                    }
                    Action::Next => DirAction::Next(self),
                })
            }

            // end indicates the end of the directory. It returns the parent
            // directory or None if the parent is the root.
            pub async fn end(mut self) -> Result<Option<Dir>> {
                self.c
                    .send(FsMsg(FsData::End, self.reply_tx.clone()))
                    .await.context(ErrSendFsMsg)?;
                self.reply_rx.recv().await.ok_or(ErrUnexpectedClosedChannel.build())?;
                Ok(self.parent())
            }

            fn parent(self) -> Option<Dir> {
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
        pub enum DirAction {
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
        pub enum FileAction {
            // Down descends into the file contents beneath
            // this entry. Data is used to send the actual data.
            Down(Data),
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
        pub struct Data {
            c: Sender,
            depth_n: i32,
            reply_tx: mpsc::Sender<Action>,
            reply_rx: mpsc::Receiver<Action>,
        }

        impl Data {
            pub async fn data(b: Vec<u8>) -> Result<DataAction> {
                todo!();
            }
        }

        #[derive(Debug)]
        pub enum DataAction {
            Next(Data),
            Skip(Dir),
        }
    }
}
