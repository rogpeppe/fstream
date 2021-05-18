#![recursion_limit = "500"]

use snafu::{ResultExt, Snafu, IntoError};
use tokio::task;

pub mod filter;
pub mod fstream;
pub mod print;
pub mod walk;
pub mod parse;

#[tokio::main]
async fn main() {
    let node = parse::parse("walk / | filter {filetype d} | print").expect("parse error");
    let node = depipe(node);
    println!("{}", node);

    //walk_tmp().await;
}

fn primitives() -> std::collections::HashMap<String, Box<dyn Command>> {
	let table = vec!(
		("print", Box::new(print::new_command())),
	);
	todo!()
}

type Result<T> = std::result::Result<T, Error>;

fn depipe(node: parse::ASTNode) -> parse::ASTNode {
	match node {
		parse::ASTNode::Word(_) => node,
		parse::ASTNode::Command(c) => {
			let mut args = vec!();
			for arg in c.args.into_iter() {
				args.push(depipe(arg));
			}
			parse::ASTNode::Command(parse::Command{
				name: c.name,
				args: args,
			})
		},
		parse::ASTNode::Pipe(left, right) => {
			match (depipe(*left), depipe(parse::ASTNode::Command(right))) {
				(left, parse::ASTNode::Command(right)) => {
					// The left hand of the pipe gets inserted as the first
					// argument to the right hand side.
					let mut right = right;
					right.args.insert(0, left);
					parse::ASTNode::Command(parse::Command{
						name: right.name,
						args: right.args,
					})
				},
				(left, right) => {
					unreachable!("depipe should always return commands, but returned {} | {}", left, right);
				},
			}
		},
	}
}

//fn exec(node: parse::ASTNode)

async fn walk_tmp() {
    // Set up an arbitrary configuration, the equivalent of:
    //
    // 	walk /tmp | filter {mode +d} | print
    //
    // TODO implement a parser to enable setting this
    // up dynamically according to some specified syntax.

    let (send_root1, recv_root1) = fstream::new();
    let walker = tokio::spawn(async { walk::walk("/tmp", send_root1).await.context(ErrWalk) });
    let (send_root2, recv_root2) = fstream::new();
    let filterer = tokio::spawn(async {
        filter::filter(recv_root1, send_root2, |entry, _path| {
            // TODO change filter function to return Result?
            entry.file_type().expect("file type").is_dir()
        })
        .await
        .context(ErrFilter)
    });
    let printer = tokio::spawn(async { print::print(recv_root2).await.context(ErrPrint) });

    if let Err(err) = walker.await.unwrap() {
        println!("walker error: {}", err);
    } else {
        println!("walker ok");
    }
    if let Err(err) = filterer.await.unwrap() {
        println!("filter error: {}", err);
    } else {
        println!("filter ok");
    }
    if let Err(err) = printer.await.unwrap() {
        println!("printer error: {}", err);
    } else {
        println!("printer ok");
    }
}

pub struct Tasks {
	tasks: Vec<task::JoinHandle<fstream::Result<()>>>,
}

impl Tasks {
	// TODO return all errors

	fn add(&mut self, t: task::JoinHandle<fstream::Result<()>>) {
		self.tasks.push(t);
	}
	async fn join(self) -> Result<()> {
		async fn join1(t: task::JoinHandle<fstream::Result<()>>) -> Result<()> {
			t.await?;
			Ok(())
		}
		futures::future::try_join_all(self.tasks.into_iter().map(join1)).await?;
		Ok(())
	}
}

#[derive(Debug, Snafu)]
enum Error {
    ErrTaskJoin { source: task::JoinError },
    ErrFstream { source: fstream::Error },
    ErrPrint { source: print::Error },
    ErrWalk { source: walk::Error },
    ErrFilter { source: filter::Error },
}

impl From<task::JoinError> for Error {
	fn from(err: task::JoinError) -> Self {
		ErrTaskJoin.into_error(err)
	}
}

#[derive(Debug)]
enum Type {
	Void,
	Fs,
	Selector,
	String,
	// TODO Entries
}

pub struct CommandType {
	// TODO allow arguments to flags.
	flags: Vec<String>,
	args: Vec<Type>,
	var_args: Option<Type>,
}

pub trait Command {
	fn fs_type(&self) -> &CommandType;
	fn start(&self, tasks: &mut Tasks, flags: Vec<String>, args: Vec<Value>, rest: Vec<Value>) -> fstream::Result<Value>;
}

pub enum Value {
	Void,
	Fs(fstream::RecvRoot),
	String(String),
	Selector(Box<dyn Fn(&fstream::DirEntry, &std::path::PathBuf)->bool>),
}

impl Value {
	fn fs_type(&self) -> Type {
		match self {
		Value::Void => Type::Void,
		Value::Fs(_) => Type::Fs,
		Value::String(_) => Type::String,
		Value::Selector(_) => Type::Selector,
		}
	}
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    	write!(f, "{:?}", self.fs_type())		// TODO don't use debug
    }
}

impl Value {
	fn as_fs(self) -> fstream::Result<fstream::RecvRoot> {
		if let Value::Fs(root) = self {
			Ok(root)
		} else {
			unreachable!("unexpected value type {:?}", self.fs_type());
		}
	}
}
