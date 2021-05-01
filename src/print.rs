use snafu::{Snafu, ResultExt};
use super::fstream;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Snafu)]
pub enum Error {
	ErrFstream{
		source: fstream::Error,
	},
}

pub async fn print(root: fstream::RecvRoot) -> Result<()> {
	let (path, dir) = root.dir().await.context(ErrFstream)?;
	let mut path = path;
	print_dir(&mut path, dir).await?;
	Ok(())
}

pub async fn print_dir(path: &mut std::path::PathBuf, dir: fstream::RecvDir) -> Result<()> {
	let mut dir = dir;
	let mut depth = 1;
	while depth >= 1 {
		match dir.entry().await.context(ErrFstream)? {
			fstream::RecvEntry::File(entry, action) => {
				path.push(entry.file_name());
				println!("f {}", path.display());
				path.pop();
				dir = action.next().await.context(ErrFstream)?;
			},
			fstream::RecvEntry::Dir(entry, action) => {
				path.push(entry.file_name());
				println!("d {}", path.display());
				dir = action.down().await.context(ErrFstream)?;
				depth += 1;
			},
			fstream::RecvEntry::End(Some(dir1)) => {
				path.pop();
				dir = dir1;
				depth -= 1;
			},
			fstream::RecvEntry::End(None) => {
				return Ok(());
			},
		}
	}
	Ok(())
}

//#[async_recursion]
//pub async fn print_dir(path: &mut std::path::PathBuf, dir: fstream::RecvDir) -> Result<Option<fstream::RecvDir>> {
//	Ok(match dir.entry().await.context(ErrFstream)? {
//		fstream::RecvEntry::File(entry, action) => {
//			path.push(entry.file_name());
//			println!("f {}", path.display());
//			path.pop();
//			print_dir(path, action.next().await.context(ErrFstream)?).await?
//		},
//		fstream::RecvEntry::Dir(entry, action) => {
//			path.push(entry.file_name());
//			println!("d {}", path.display());
//			let opt_dir = print_dir(path, action.down().await.context(ErrFstream)?).await?;
//			path.pop();
//			opt_dir
//		},
//		fstream::RecvEntry::End(opt_dir) => {
//			path.pop();
//			if let Some(dir) = opt_dir {
//				print_dir(path, dir).await?
//			} else {
//				None
//			}
//		},
//	})
//}
