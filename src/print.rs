use super::fstream;
use snafu::{ResultExt, Snafu};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Snafu)]
pub enum Error {
    ErrFstream { source: fstream::Error },
}

pub async fn print(root: fstream::RecvRoot) -> Result<()> {
    let (path, dir) = root.dir().await.context(ErrFstream)?;
    let mut path = path;
    print_dir(&mut path, dir).await?;
    Ok(())
}

pub async fn print_dir(path: &mut std::path::PathBuf, dir: fstream::RecvDir) -> Result<()> {
    let mut dir = dir;
    loop {
        match dir.entry().await.context(ErrFstream)? {
            fstream::RecvEntry::File(entry, action) => {
                path.push(entry.file_name());
                println!("f {}", path.display());
                path.pop();
                dir = action.next().await.context(ErrFstream)?;
            }
            fstream::RecvEntry::Dir(entry, action) => {
                path.push(entry.file_name());
                println!("d {}", path.display());
                dir = action.down().await.context(ErrFstream)?;
            }
            fstream::RecvEntry::End(Some(dir1)) => {
                path.pop();
                dir = dir1;
            }
            fstream::RecvEntry::End(None) => {
                return Ok(());
            }
        }
    }
}
