
module abcstd {
	struct Types {
		string_cvt: abc::Converter<String>,
		string: abc::Type,
		status_cvt: abc::Converter<Status>,
		status: abc::Type,
	}
}

module fs {
	#[derive(Clone)]
	struct Types {
		std: abcstd.Types,
		fs_cvt: abc::Converter<Fs>,
		string_cvt: abc::Converter<String>,
		selector_cvt: abc::Converter<Selector>,
	}

	impl Types {
		fn new(registry: &mut abc.Registry, std: &abcstd::Types) -> Result<Types> {
			let fs_cvt = registry.new_type<Fs>("fs");
			let selector_cvt = registry.new_type<Selector>("selector");
			Types{
				fs_cvt: fs_cvt,
				fs: fs_cvt.get_type(),
				string_cvt: string_cvt,
				string: string_cvt.get_type(),
				selector_cvt: selector_cvt,
				selector: selector_cvt.get_type(),
			}
		}
		fn fs(self: &Types) -> abc::Type {
			return self.fs_cvt.get_type().clone()
		}
		fn to_fs(
		fn string(self: &Types) -> abc::Type {
			return self.std.string()
		}
		fn selector
	}

	type Fs = recv::Root
}
			let string_cvt = registry.new_type<Void>("void");
			let void_cvt = registry


typeset has:

	defined type for each
	list of types
