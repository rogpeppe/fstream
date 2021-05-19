#![recursion_limit = "500"]

use snafu::{IntoError, ResultExt, Snafu};
use std::collections::HashMap as Map;
use tokio::task;

pub mod filter;
pub mod fstream;
pub mod parse;
pub mod print;
pub mod walk;

#[tokio::main]
async fn main() {
    run("filter {mode +d}").await.expect("pipeline failed");
}

async fn run(expr: &str) -> Result<()> {
    let cmds = Commands::new();
    let mut tasks = Tasks::new();
    let node = compile(expr, &cmds)?;
    let value = start(node, &cmds, &mut tasks)?;
    match value {
        Value::Void => (),
        _ => {
            unreachable!("unexpected value type at top level");
        }
    };
    tasks.join().await
}

fn compile(expr: &str, cmds: &Commands) -> Result<parse::ASTNode> {
    let node = parse::parse(expr)?;
    let node = depipe(node);
    let node = typecheck(node, &cmds)?;
    let node = cmds.convert(node, Type::Void)?;
    Ok(node)
}

fn start(node: parse::ASTNode, cmds: &Commands, tasks: &mut Tasks) -> Result<Value> {
    Ok(match node {
        parse::ASTNode::Word(s) => Value::String(s),
        parse::ASTNode::Pipe(_, _) => {
            unreachable!("pipes should have been eliminated");
        }
        parse::ASTNode::Command(c) => {
            let cmd = cmds.get(&c.name)?;
            // TODO sanity check that the command is actually returning the type
            // that's expected of it.
            let args = c
                .args
                .into_iter()
                .map(|arg| start(arg, cmds, tasks))
                .collect::<Result<_>>()?;
            // TODO flags
            cmd.start(tasks, vec![], args, vec![])?
        }
    })
}

async fn walk_tmp_with_commands() -> Result<()> {
    let mut tasks = Tasks::new();
    let walkfs = walk::new_command().start(
        &mut tasks,
        vec![],
        vec![Value::String("/tmp".to_string())],
        vec![],
    )?;
    let select = Value::Selector(Box::new(|entry, _path| {
        entry.file_type().expect("file type").is_dir()
    }));
    let filterfs = filter::new_command().start(&mut tasks, vec![], vec![walkfs, select], vec![])?;
    print::new_command().start(&mut tasks, vec![], vec![filterfs], vec![])?;
    tasks.join().await
}

struct Commands {
    name2command: Map<String, Box<dyn Command>>,
}

impl Commands {
    fn new() -> Commands {
        let list: Vec<(&str, Box<dyn Command>)> = vec![
            ("print", Box::new(print::new_command())),
            ("walk", Box::new(walk::new_command())),
            ("filter", Box::new(filter::new_command())),
        ];
        let mut map = Map::new();
        for (name, cmd) in list {
            map.insert(name.to_string(), cmd);
        }
        Commands { name2command: map }
    }

    fn get(&self, name: &str) -> Result<&Box<dyn Command>> {
        if let Some(c) = self.name2command.get(name) {
            Ok(c)
        } else {
            Err(ErrCommandNotFound {
                name: name.to_string(),
            }
            .build())
        }
    }

    fn convert(&self, node: parse::ASTNode, to: Type) -> Result<parse::ASTNode> {
        let ntype = match &node {
            parse::ASTNode::Command(c) => self.get(&c.name)?.fs_type().ret,
            parse::ASTNode::Word(_) => Type::String,
            _ => {
                unreachable!("pipes should have been converted to commands by this stage");
            }
        };
        if let Some(node) = self.convert1(node, ntype, to) {
            Ok(node)
        } else {
            Err(ErrConvert {
                //node: format!("{:?}", node),
                from: ntype,
                to: to,
            }
            .build())
        }
    }

    fn convert1(&self, node: parse::ASTNode, ntype: Type, to: Type) -> Option<parse::ASTNode> {
        if ntype == to {
            return Some(node);
        }
        match to {
            Type::Fs => Some(parse::ASTNode::Command(parse::Command {
                name: "walk".to_string(),
                args: vec![self.convert1(node, ntype, Type::String)?],
            })),
            Type::Void => Some(parse::ASTNode::Command(parse::Command {
                name: "print".to_string(),
                args: vec![self.convert1(node, ntype, Type::Fs)?],
            })),
            _ => None,
        }
    }
}

// typecheck checks the types of all commands and arguments and inserts
// conversion commands when necessary.
fn typecheck(node: parse::ASTNode, cmds: &Commands) -> Result<parse::ASTNode> {
    match node {
        parse::ASTNode::Command(c) => {
            let ctype = cmds.get(&c.name)?.fs_type();
            if c.args.len() < ctype.args.len() {
                return Err(ErrTooFewArgs {
                    name: c.name.to_string(),
                }
                .build());
            }
            let arg_types = if let Some(t) = ctype.var_args {
                itertools::Either::Left(itertools::chain(
                    ctype.args.iter().cloned(),
                    std::iter::repeat(t),
                ))
            } else {
                itertools::Either::Right(ctype.args.iter().cloned())
            };
            // TODO check flags
            Ok(parse::ASTNode::Command(parse::Command {
                name: c.name,
                args: c
                    .args
                    .into_iter()
                    .zip(arg_types)
                    .map(|(arg, arg_type)| cmds.convert(arg, arg_type))
                    .collect::<Result<_>>()?,
            }))
        }
        parse::ASTNode::Word(_) => Ok(node),
        parse::ASTNode::Pipe(_, _) => {
            unreachable!("pipes should have been converted to commands by this stage");
        }
    }
}

fn depipe(node: parse::ASTNode) -> parse::ASTNode {
    match node {
        parse::ASTNode::Word(_) => node,
        parse::ASTNode::Command(c) => {
            let mut args = vec![];
            for arg in c.args.into_iter() {
                args.push(depipe(arg));
            }
            parse::ASTNode::Command(parse::Command {
                name: c.name,
                args: args,
            })
        }
        parse::ASTNode::Pipe(left, right) => {
            match (depipe(*left), depipe(parse::ASTNode::Command(right))) {
                (left, parse::ASTNode::Command(right)) => {
                    // The left hand of the pipe gets inserted as the first
                    // argument to the right hand side.
                    let mut right = right;
                    right.args.insert(0, left);
                    parse::ASTNode::Command(parse::Command {
                        name: right.name,
                        args: right.args,
                    })
                }
                (left, right) => {
                    unreachable!(
                        "depipe should always return commands, but returned {} | {}",
                        left, right
                    );
                }
            }
        }
    }
}

//fn exec(node: parse::ASTNode, tasks: &mut Tasks) -> fstream::Result {
//	match node {
//	ASTNode::Command
//	}
//}

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

type Result<T> = std::result::Result<T, Error>;

pub struct Tasks {
    tasks: Vec<task::JoinHandle<fstream::Result<()>>>,
}

impl Tasks {
    fn new() -> Tasks {
        Tasks { tasks: vec![] }
    }
    // TODO return all errors

    // add adds a task to the list of tasks to wait for.
    fn add(&mut self, t: task::JoinHandle<fstream::Result<()>>) {
        self.tasks.push(t);
    }

    // join waits for all the tasks to complete and returns the first failure.
    async fn join(self) -> Result<()> {
        async fn join1(t: task::JoinHandle<fstream::Result<()>>) -> Result<()> {
            Ok(t.await??)
        }
        futures::future::try_join_all(self.tasks.into_iter().map(join1)).await?;
        Ok(())
    }
}

#[derive(Debug, Snafu)]
enum Error {
    ErrTaskJoin {
        source: task::JoinError,
    },
    ErrFstream {
        source: fstream::Error,
    },
    ErrPrint {
        source: print::Error,
    },
    ErrWalk {
        source: walk::Error,
    },
    ErrFilter {
        source: filter::Error,
    },
    ErrParse {
        source: parse::Error,
    },
    ErrCommandNotFound {
        name: String,
    },
    ErrConvert {
        //node: String,
        from: Type,
        to: Type,
    },
    ErrTooFewArgs {
        name: String,
    },
}

impl From<task::JoinError> for Error {
    fn from(err: task::JoinError) -> Self {
        ErrTaskJoin.into_error(err)
    }
}

impl From<fstream::Error> for Error {
    fn from(err: fstream::Error) -> Self {
        ErrFstream.into_error(err)
    }
}

impl From<parse::Error> for Error {
    fn from(err: parse::Error) -> Self {
        ErrParse.into_error(err)
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
enum Type {
    Void,
    Fs,
    Selector,
    String,
    // TODO Entries
}

#[derive(Debug, PartialEq)]
pub struct CommandType {
    // TODO allow arguments to flags.
    flags: Vec<String>,
    args: Vec<Type>,
    var_args: Option<Type>,
    ret: Type,
}

pub trait Command {
    fn fs_type(&self) -> &CommandType;
    fn start(
        &self,
        tasks: &mut Tasks,
        flags: Vec<String>,
        args: Vec<Value>,
        rest: Vec<Value>,
    ) -> fstream::Result<Value>;
}

// TODO change to return Result?
pub type Selector = Box<dyn Fn(&fstream::DirEntry, &std::path::PathBuf) -> bool + Send + Sync>;

pub enum Value {
    Void,
    Fs(fstream::RecvRoot),
    String(String),
    Selector(Selector),
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
        write!(f, "{:?}", self.fs_type()) // TODO don't use debug
    }
}

impl Value {
    fn as_fs(self) -> fstream::Result<fstream::RecvRoot> {
        if let Value::Fs(root) = self {
            Ok(root)
        } else {
            unreachable!("unexpected value type; want fs, got {:?}", self.fs_type());
        }
    }
    fn as_string(self) -> fstream::Result<String> {
        if let Value::String(s) = self {
            Ok(s)
        } else {
            unreachable!(
                "unexpected value type; want string, got {:?}",
                self.fs_type()
            );
        }
    }
    fn as_selector(self) -> fstream::Result<Selector> {
        if let Value::Selector(s) = self {
            Ok(s)
        } else {
            unreachable!(
                "unexpected value type; want selector; got  {:?}",
                self.fs_type()
            );
        }
    }
}
