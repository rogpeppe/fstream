use super::fstream;

use super::CommandType;
use super::Value;

pub fn new_command() -> impl super::Command {
    Command(CommandType {
        flags: vec![],
        args: vec![super::Type::String],
        var_args: None,
        ret: super::Type::Selector,
    })
}

struct Command(CommandType);

impl super::Command for Command {
    fn fs_type(&self) -> &super::CommandType {
        return &self.0;
    }
    fn start(
        &self,
        _tasks: &mut super::Tasks,
        _flags: Vec<String>,
        args: Vec<Value>,
        _rest: Vec<Value>,
    ) -> fstream::Result<Value> {
        let mut args = args;
        let spec = args.pop().unwrap().as_string()?;
        match spec.as_ref() {
            "d" => Ok(super::Value::Selector(Box::new(|entry, _path| {
                entry.file_type().expect("file type").is_dir()
            }))),
            _ => Err(fstream::ErrUsage {
                msg: format!("invalid mode {}", spec),
            }
            .build()),
        }
    }
}
