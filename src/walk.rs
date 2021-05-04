use super::fstream;
use async_recursion::async_recursion;
use snafu::{ResultExt, Snafu};
use std::io::Read;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("root path is not a directory"))]
    ErrNotDirectory,
    ErrFstream {
        source: fstream::Error,
    },
    ErrIO {
        source: std::io::Error,
    },
}

// walk walks the directory hierarchy rooted at the given path, sending the results to root.
pub async fn walk<P: AsRef<std::path::Path>>(path_ref: P, root: fstream::SendRoot) -> Result<()> {
    let mut path = std::path::PathBuf::new();
    path.push(path_ref.as_ref());
    let d = std::fs::metadata(path_ref.as_ref()).context(ErrIO)?;
    if !d.is_dir() {
        return Err(ErrNotDirectory.build());
    }
    Ok(
        if let Some(dir) = root.dir(path.clone()).await.context(ErrFstream)? {
            walk_dir(&mut path, dir).await?;
        },
    )
}

#[async_recursion]
async fn walk_dir(
    path: &mut std::path::PathBuf,
    dir: fstream::SendDir,
) -> Result<Option<fstream::SendDir>> {
    let mut dir = dir;
    let mut paths: Vec<fstream::DirEntry> = vec![];
    for entry in std::fs::read_dir(&path).context(ErrIO)? {
        paths.push(entry.context(ErrIO)?);
    }
    paths.sort_by_key(|entry| entry.path());
    for entry in paths {
        // We need to push the file name before calling the
        // dir method because we're handing off ownership
        // by doing that.
        path.push(entry.file_name());
        // Could use defer to pop the path here?
        if entry.file_type().context(ErrIO)?.is_dir() {
            match dir.dir(entry).await.context(ErrFstream)? {
                fstream::SendDirEntryAction::Down(child) => {
                    // Note: subdirectories will always return Some(dir)
                    // because None can only happen at the root and
                    // we know that the child is at least one level down.
                    dir = walk_dir(path, child).await?.unwrap();
                }
                fstream::SendDirEntryAction::Next(next) => {
                    dir = next;
                }
                fstream::SendDirEntryAction::Skip(parent) => {
                    path.pop();
                    return Ok(Some(parent));
                }
                fstream::SendDirEntryAction::End => {
                    path.pop();
                    return Ok(None);
                }
            }
        } else {
            match dir.file(entry).await.context(ErrFstream)? {
                fstream::SendFileEntryAction::Down(file) => {
                    dir = walk_file(path, file).await?;
                }
                fstream::SendFileEntryAction::Next(next) => dir = next,
                fstream::SendFileEntryAction::Skip(parent) => {
                    path.pop();
                    return Ok(Some(parent));
                }
                fstream::SendFileEntryAction::End => {
                    path.pop();
                    return Ok(None);
                }
            }
        }
        path.pop();
    }
    Ok(dir.end().await.context(ErrFstream)?)
}

const BLOCK_SIZE: usize = 8192;

pub async fn walk_file(
    path: &mut std::path::PathBuf,
    file: fstream::SendFile,
) -> Result<fstream::SendDir> {
    let mut file = file;
    let mut f = std::fs::File::open(path).context(ErrIO)?;
    loop {
        let mut data = vec![0; BLOCK_SIZE];
        let n = f.read(&mut data).context(ErrIO)?;
        if n == 0 {
            return Ok(file.end().await.context(ErrFstream)?);
        }
        data.truncate(n);
        match file.data(data).await.context(ErrFstream)? {
            fstream::SendFileAction::Next(next) => {
                file = next;
            }
            fstream::SendFileAction::Skip(dir) => return Ok(dir),
        }
    }
}
