use super::fstream;

use super::CommandType;
use super::Value;

pub fn new_command() -> impl super::Command {
    Command(CommandType {
        flags: vec![],
        args: vec![],
        var_args: Some(super::Type::Selector),
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
        let selectors = args.into_iter().map(|v| v.as_selector()).collect::<Result<Vec<_>, _>>()?;
        Ok(super::Value::Selector(Box::new(move |entry, path| {
                selectors.iter().any(|selector| selector(entry, path))
         })))
    }
}
