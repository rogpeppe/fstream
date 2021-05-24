
module abc {
	struct Registry {
	}

	impl Registry {
		fn new_type<V>(name: String) -> Type<V> {
		}
	}

	pub struct Type {
		registry: Registry,
		id: TypeId,
	}

	pub struct Converter<V> {
		vtype: Type,
	}

	impl Converter<V> {
		fn get_type(&self) -> Type
		fn to_value(&self, v: 'a V) -> 'a Value {
			Value{
				v: Box::new(v)
				vtype: v,
			}
		}

		fn from_value(v: Value) -> Option<V> {
			v.v.downcast::<V>
		}
	}

	struct Value {
		v: Box<dyn Any>)
		vtype: Type,
	}

	impl Value<V> {
		fn downcast<T>(self) -> Option<T> {
			let any: Box<dyn Any> = Box::new(self);
			any.downcast<Value<T>>.v
		}

		fn abctype(&self) -> Type {
			self.vtype.clone()
		}

		fn value(self) -> V {
			self.v
		}
	}

	struct DynValue = Box<dyn TypedValue>;

	trait DynValue {
		downcast<T>(self) -> Option<T>;
		abctype(&self) -> Type;
	}

	impl Clone for Type {
		fn clone(&self) -> Type {
		}
	}

	module abcstd {
		struct Types {
			string_cvt: abc::Converter<String>,
			string: abc::Type,
			status_cvt: abc::Converter<Status>,
			status: abc::Type,
		}
	}

	module fs {
		struct Types {
			fs_cvt: abc::Converter<Fs>,
			fs: abc::Type,
			string_cvt: abc::Type<String>,
			gate_type: abc::Type<Gate>,
		}

		impl Types {
			fn new(registry: &mut abc.Registry) -> Types,

		}

		type Fs = recv::Root
	}
	typeset has:

		defined type for each
		list of types

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
}

module abcstd {
}
