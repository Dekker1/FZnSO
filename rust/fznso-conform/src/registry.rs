//! Reading the registry TOMLs.
//!
//! The registry is prose and tables meant for humans, so this reads only the
//! two fields a conformance check needs — the identifier and the types — and
//! ignores everything the documentation generator cares about.

use std::{collections::BTreeMap, fs, io, path::Path};

use fznso_types::{FznsoType, FznsoTypeBase};

/// One name the registry defines, with the types it defines it at.
#[derive(Clone, Debug)]
pub struct Entry {
	/// Where it was defined, for error messages.
	pub source: String,
	/// The widest types the name takes. A solver may narrow these.
	pub args: Vec<FznsoType>,
}

/// Every name the registry defines, across all five categories.
#[derive(Default)]
pub struct Registry {
	pub constraints: BTreeMap<String, Entry>,
	pub objectives: BTreeMap<String, Entry>,
	pub options: BTreeMap<String, Entry>,
	pub statistics: BTreeMap<String, Entry>,
}

/// Read a type as the registry spells it.
///
/// The qualifiers are stripped outermost first, in the order the registry
/// writes them, so `list of var set of int` is a list of variable sets. An
/// unrecognised spelling is an error rather than a default: the set of types
/// the registry uses is closed, and silently reading a new one as `int` would
/// make the check pass for the wrong reason.
pub fn parse_type(spelling: &str) -> Result<FznsoType, String> {
	let mut rest = spelling.trim();
	let mut ty = FznsoType {
		list_of: false,
		decision: false,
		set_of: false,
		opt: false,
		base: FznsoTypeBase::FznsoTypeBaseInt,
	};
	if let Some(r) = rest.strip_prefix("list of ") {
		ty.list_of = true;
		rest = r;
	}
	if let Some(r) = rest.strip_prefix("var ") {
		ty.decision = true;
		rest = r;
	}
	if let Some(r) = rest.strip_prefix("opt ") {
		ty.opt = true;
		rest = r;
	}
	if let Some(r) = rest.strip_prefix("set of ") {
		ty.set_of = true;
		rest = r;
	}
	ty.base = match rest {
		"int" => FznsoTypeBase::FznsoTypeBaseInt,
		"bool" => FznsoTypeBase::FznsoTypeBaseBool,
		"float" => FznsoTypeBase::FznsoTypeBaseFloat,
		"string" => FznsoTypeBase::FznsoTypeBaseString,
		other => return Err(format!("unknown base type `{other}` in `{spelling}`")),
	};
	Ok(ty)
}

/// Split `"type: name"`, which is how the registry writes an argument.
fn arg_type(arg: &str) -> Result<FznsoType, String> {
	let spelling = match arg.split_once(": ") {
		Some((ty, _name)) => ty,
		// Categories other than constraints give a bare type with no name.
		None => arg,
	};
	parse_type(spelling)
}

/// The reified and half-reified forms of `ident`.
///
/// Every constraint names both, unless the entry marks itself `functional` —
/// one that computes a value rather than checking a relation. The names are
/// derived rather than read, because the registry does not write them down:
/// both spelling and meaning follow from the base entry.
///
/// They take the base arguments followed by a Boolean, which the registry also
/// leaves implicit, so it is appended here and never read from `args`.
fn reified(entry: &toml::Value, base: &Entry) -> Vec<(String, Entry)> {
	if entry
		.get("functional")
		.and_then(toml::Value::as_bool)
		.unwrap_or(false)
	{
		return Vec::new();
	}
	let mut args = base.args.clone();
	args.push(FznsoType {
		list_of: false,
		decision: true,
		set_of: false,
		opt: false,
		base: FznsoTypeBase::FznsoTypeBaseBool,
	});
	let ident = entry
		.get("ident")
		.and_then(toml::Value::as_str)
		.unwrap_or_default();
	["_reif", "_imp"]
		.iter()
		.map(|suffix| {
			(
				format!("{ident}{suffix}"),
				Entry {
					source: base.source.clone(),
					args: args.clone(),
				},
			)
		})
		.collect()
}

/// Read one TOML file's `[[entry]]` blocks, taking the types from `field`.
///
/// `field` differs by category: constraints give an array of `args`, objectives
/// a single `arg`, options and statistics a `type`.
fn read(path: &Path, field: &str, with_reified: bool) -> Result<BTreeMap<String, Entry>, String> {
	let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
	let doc: toml::Value = toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
	let source = path
		.file_name()
		.map_or_else(String::new, |n| n.to_string_lossy().into_owned());

	let mut out = BTreeMap::new();
	let entries = doc
		.get("entry")
		.and_then(toml::Value::as_array)
		.map_or(&[][..], Vec::as_slice);
	for entry in entries {
		let Some(ident) = entry.get("ident").and_then(toml::Value::as_str) else {
			continue;
		};
		let args = match entry.get(field) {
			Some(toml::Value::Array(list)) => list
				.iter()
				.filter_map(toml::Value::as_str)
				.map(arg_type)
				.collect::<Result<Vec<_>, _>>()
				.map_err(|e| format!("{source}: `{ident}`: {e}"))?,
			Some(toml::Value::String(one)) => {
				vec![arg_type(one).map_err(|e| format!("{source}: `{ident}`: {e}"))?]
			}
			// A category that types nothing still contributes its name.
			_ => Vec::new(),
		};
		let base = Entry {
			source: source.clone(),
			args,
		};
		if with_reified {
			for (name, variant) in reified(entry, &base) {
				out.insert(name, variant);
			}
		}
		out.insert(ident.to_string(), base);
	}
	Ok(out)
}

impl Registry {
	/// Read every category from a `docs/registry` directory.
	pub fn load(root: &Path) -> Result<Self, String> {
		let mut reg = Registry::default();

		let constraints = root.join("constraints");
		let mut dir: Vec<_> = fs::read_dir(&constraints)
			.map_err(|e: io::Error| format!("{}: {e}", constraints.display()))?
			.filter_map(Result::ok)
			.map(|e| e.path())
			.filter(|p| p.extension().is_some_and(|e| e == "toml"))
			.collect();
		dir.sort();
		for path in dir {
			reg.constraints.extend(read(&path, "args", true)?);
		}

		reg.objectives = read(&root.join("objectives.toml"), "arg", false)?;
		reg.options = read(&root.join("options.toml"), "type", false)?;
		reg.statistics = read(&root.join("statistics.toml"), "type", false)?;
		// Message scopes are deliberately absent: they are the one registry
		// vocabulary a solver does not declare up front, so there is no
		// capability list to check them against.
		Ok(reg)
	}
}

/// Whether `declared` is the registry's `expected` type or a narrowing of it.
///
/// Narrowing restricts what a name accepts without changing what it means: a
/// solver that takes only a fixed set where the registry allows a variable one
/// has declared the same constraint, restricted to what its propagator can do.
/// The base type and the shape may not move at all, and widening is never
/// allowed — an application reading the registry would have no way to know the
/// extra cases existed.
pub fn narrows(declared: &FznsoType, expected: &FznsoType) -> bool {
	declared.base == expected.base
		&& declared.list_of == expected.list_of
		&& declared.set_of == expected.set_of
		&& (declared.decision == expected.decision || !declared.decision)
		&& (declared.opt == expected.opt || !declared.opt)
}

/// How a type reads in the registry's own notation, for error messages.
pub fn spell(ty: &FznsoType) -> String {
	let mut s = String::new();
	if ty.list_of {
		s.push_str("list of ");
	}
	if ty.decision {
		s.push_str("var ");
	}
	if ty.opt {
		s.push_str("opt ");
	}
	if ty.set_of {
		s.push_str("set of ");
	}
	s.push_str(match ty.base {
		FznsoTypeBase::FznsoTypeBaseInt => "int",
		FznsoTypeBase::FznsoTypeBaseBool => "bool",
		FznsoTypeBase::FznsoTypeBaseFloat => "float",
		FznsoTypeBase::FznsoTypeBaseString => "string",
	});
	s
}

/// The registry name `ident` would be if its words were in another order.
///
/// The registry names a constraint for the type it takes and puts that type
/// first, so a name that drifted from an older scheme — `array_int_element`,
/// `minimize_int` — is usually the right entry with its words transposed. That
/// is worth saying out loud, because such a name is not a new constraint but a
/// stale one, and nothing else distinguishes the two.
pub fn transposed(ident: &str) -> Vec<String> {
	let mut words: Vec<&str> = ident.split('_').collect();
	words.sort_unstable();
	words.iter().map(|w| (*w).to_string()).collect()
}
