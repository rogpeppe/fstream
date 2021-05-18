use super::fstream;
use snafu::{ResultExt, Snafu};

pub type Result<T> = std::result::Result<T, Error>;

use super::CommandType;
use super::Value;

#[derive(Debug, Snafu)]
pub enum Error {
    ErrFstream { source: fstream::Error },
}

pub fn new_command() -> impl super::Command {
	Command(CommandType{
		flags: vec!(),
		args: vec!(super::Type::Fs),		// TODO Entries
		var_args: None,
	})
}

struct Command(CommandType);

impl super::Command for Command {
	fn fs_type(&self) -> &super::CommandType {
		return &self.0
	}
	fn start(&self, tasks: &mut super::Tasks, _flags: Vec<String>, args: Vec<Value>, _rest: Vec<Value>) -> fstream::Result<Value> {
		let mut args = args;
		let root = args.pop().unwrap().as_fs()?;
		tasks.add(tokio::spawn(async {
			print(root).await.context(super::ErrPrint).unwrap();
			Ok(())
		}));
		Ok(Value::Void)
	}
}

// print prints some information about all the entries in root.
pub async fn print(root: fstream::RecvRoot) -> Result<()> {
    let (path, dir) = root.dir().await.context(ErrFstream)?;
    let mut path = path;
    print_dir(&mut path, dir).await?;
    Ok(())
}

async fn print_dir(path: &mut std::path::PathBuf, dir: fstream::RecvDir) -> Result<()> {
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
