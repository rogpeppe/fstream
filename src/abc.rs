pub struct Registry {
	id: u32,
	types: std::container::MapHash<String, TypeID>,
	commands: Map<String, Box<dyn Command>>,
}

#[derive(Copy,Clone,Debug)]
struct TypeID(u32);

impl Registry {
	pub fn new() -> Registry {
		Registry{
			id: 0,
			types:  std::container::MapHash::new(),
			commands: std::container::MapHash::new(),
		}
	}
	pub fn new_type<T>(&mut self, name: &str) -> Result<Converter<T>> {
		if let Some(_) = self.types.get(name) {
			return Err(ErrAlreadyRegisteredType{name: name.to_string}).build())
		}
		self.id.0 += 1;			// or generate UUID?
		self.types.insert(name.to_string(), self.id);
		Ok(Converter{
			abctype: Type{
			id: self.self.id,
			name: Arc::new(name.to_string()),
		})
	}
	pub fn add_command(&mut self, name: &str, cmd: Box<dyn Command>) Result<()> {
		if let Some(_) = self.commands.get(name) {
			return Err(ErrAlreadyRegisteredCommand{name: name.to_string}.build())
		}
		self.commands.insert(name.to_string(), cmd);
	}
}

#[derive(PartialEq, Debug, Clone)]
pub struct Type {
	id: TypeId,
	name: Arc<String>,
}

impl fmt::Display for Type {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.name);
	}
}

#[derive(Clone)]
pub struct Converter<V> {
	abctype: Type,
}

impl Converter<V> {
	// get_type returns the alphabet type associated with the converter.
	fn get_type(&self) -> &Type {
		&self.abctype
	}

	// to_value converts v to a dynamic Value.
	fn to_dyn(&self, v: 'a V) -> 'a Value {
		Value{
			v: Box::new(v)
			vtype: v,
		}
	}

	// from_value converts the dynamic Value v to the
	// static type.
	fn from_dyn(v: Value) -> Option<V> {
		v.v.downcast::<V>
	}
}

// Value represents a dynamic value holding a value
// with a registered alphabet type.
struct Value {
	v: Box<dyn Any>)
	vtype: Type,
}

struct CommandType {
            flags: Vec<String>,
            args: Vec<Type>,
            var_args: Option<Type>,
            ret: Type,
}

trait Command {
            fn get_type(&self) -> &CommandType;
            fn start(
                &self,
                tasks: &mut Tasks,
                flags: MapHash<String, Value>,
                args: Vec<Value>,
            ) -> fstream::Result<Value>;
}
