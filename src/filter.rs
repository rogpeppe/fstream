use super::fstream;
use snafu::{ResultExt, Snafu};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Snafu)]
pub enum Error {
    ErrFstream { source: fstream::Error },
}

pub async fn filter<F>(
    recv_root: fstream::RecvRoot,
    send_root: fstream::SendRoot,
    keep: F,
) -> Result<()>
where
    F: Fn(&fstream::DirEntry, &std::path::PathBuf) -> bool,
{
    let (path, recv_dir) = recv_root.dir().await.context(ErrFstream)?;
    let mut path = path;
    Ok(
        if let Some(send_dir) = send_root.dir(path.clone()).await.context(ErrFstream)? {
            filter_dir(&mut path, recv_dir, send_dir, keep).await?
        },
    )
}

pub async fn filter_dir<F>(
    path: &mut std::path::PathBuf,
    recv_dir: fstream::RecvDir,
    send_dir: fstream::SendDir,
    keep: F,
) -> Result<()>
where
    F: Fn(&fstream::DirEntry, &std::path::PathBuf) -> bool,
{
    let mut recv_dir = recv_dir;
    let mut send_dir = send_dir;
    loop {
        let entry = recv_dir.entry().await.context(ErrFstream)?;
        match entry {
            fstream::RecvEntry::File(entry, action) => {
                path.push(entry.file_name());
                if !keep(&entry, &path) {
                    // The file doesn't pass the filter, so discard it.
                    recv_dir = action.next().await.context(ErrFstream)?;
                    continue;
                }
                // Let's see if downstream wants it.
                match send_dir.file(entry).await.context(ErrFstream)? {
                    fstream::SendFileEntryAction::Down(send_file) => {
                        // Downstream wants it.
                        let recv_file = action.down().await.context(ErrFstream)?;
                        // TODO use destructuring assignment if it's available.
                        let (send_dir1, recv_dir1) = transfer_file(send_file, recv_file).await?;
                        send_dir = send_dir1;
                        recv_dir = recv_dir1;
                    }
                    fstream::SendFileEntryAction::Next(next) => {
                        // Downstream doesn't want it.
                        send_dir = next;
                        recv_dir = action.next().await.context(ErrFstream)?;
                    }
                    fstream::SendFileEntryAction::Skip(send_parent) => {
                        // Downstream doesn't want it or any of the rest of the directory.
                        path.pop();
                        // TODO can this actually return None?
                        recv_dir = action.skip().await.context(ErrFstream)?.unwrap();
                        send_dir = send_parent;
                    }
                    fstream::SendFileEntryAction::End => {
                        path.pop();
                        // We expect this to return None.
                        action.skip().await.context(ErrFstream)?;
                        return Ok(());
                    }
                }
            }
            fstream::RecvEntry::Dir(entry, action) => {
                path.push(entry.file_name());
                if !keep(&entry, &path) {
                    // The directory doesn't pass the filter, so discard it.
                    recv_dir = action.next().await.context(ErrFstream)?;
                    continue;
                }
                // Let's see if downstream wants it.
                match send_dir.dir(entry).await.context(ErrFstream)? {
                    fstream::SendDirEntryAction::Down(child_dir) => {
                        // Downstream wants it.
                        recv_dir = action.down().await.context(ErrFstream)?;
                        send_dir = child_dir;
                    }
                    fstream::SendDirEntryAction::Next(next) => {
                        // Downstream doesn't want it.
                        send_dir = next;
                        recv_dir = action.next().await.context(ErrFstream)?;
                    }
                    fstream::SendDirEntryAction::Skip(send_parent) => {
                        // Downstream doesn't want it or any of the rest of the directory.
                        path.pop();
                        // TODO can this actually return None?
                        recv_dir = action.skip().await.context(ErrFstream)?.unwrap();
                        send_dir = send_parent;
                    }
                    fstream::SendDirEntryAction::End => {
                        path.pop();
                        // We expect this to return None.
                        if let Some(_d) = action.skip().await.context(ErrFstream)? {
                            unreachable!("should not have got continuation directory")
                        }
                        return Ok(());
                    }
                }
            }
            fstream::RecvEntry::End(opt_recv_dir) => {
                let opt_send_dir = send_dir.end().await.context(ErrFstream)?;
                match (opt_recv_dir, opt_send_dir) {
                    (Some(recv_dir1), Some(send_dir1)) => {
                        recv_dir = recv_dir1;
                        send_dir = send_dir1;
                    }
                    (None, None) => {
                        return Ok(());
                    }
                    _ => {
                        unreachable!("mismatched end directories")
                    }
                }
            }
        }
    }
}

async fn transfer_file(
    send_file: fstream::SendFile,
    recv_file: fstream::RecvFile,
) -> Result<(fstream::SendDir, fstream::RecvDir)> {
    let mut send_file = send_file;
    let mut recv_file = recv_file;
    loop {
        match recv_file.data().await.context(ErrFstream)? {
            fstream::RecvData::Bytes(data, recv_file1) => {
                match send_file.data(data).await.context(ErrFstream)? {
                    fstream::SendFileAction::Next(send_file1) => {
                        send_file = send_file1;
                        recv_file = recv_file1;
                    }
                    fstream::SendFileAction::Skip(send_dir) => {
                        let recv_dir = recv_file1.skip().await.context(ErrFstream)?;
                        return Ok((send_dir, recv_dir));
                    }
                }
            }
            fstream::RecvData::End(recv_dir) => {
                let send_dir = send_file.end().await.context(ErrFstream)?;
                return Ok((send_dir, recv_dir));
            }
        }
    }
}
