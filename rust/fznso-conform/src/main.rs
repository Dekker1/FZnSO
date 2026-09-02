//! Check a solver library against the FZnSO registry.
//!
//! The registry's one rule is that a name means what the registry says it
//! means, so that is the only thing checked here: **every declared name that
//! the registry defines must be declared at the registry's types, or at a
//! narrowing of them.** A name the registry does not define is a solver's own
//! and is reported but not failed.
//!
//! The exception worth the code is a name that is a registry name with its
//! words in another order. `minimize_int` where the registry says
//! `int_minimize` is not a solver-specific name, it is a stale one, and by the
//! rule above nothing would otherwise notice.

mod registry;

use std::{
	collections::BTreeMap,
	env,
	path::{Path, PathBuf},
	process::ExitCode,
};

use fznso::Library;
use fznso_types::FznsoType;
use registry::{Entry, Registry, narrows, spell, transposed};

/// What a declared name turned out to be.
enum Verdict {
	/// Declared at the registry's types, or a narrowing of them.
	Conforms,
	/// Declared at types the registry does not allow.
	Mismatch(Vec<String>),
	/// Not a registry name, but a registry name with its words transposed.
	Stale(String),
	/// Not a registry name at all.
	Own,
}

/// The `len` entries at `ptr`, as a slice.
///
/// A library with nothing to declare may report a null pointer alongside a
/// length of zero, which `slice::from_raw_parts` rejects outright — so the
/// empty case is answered without forming a slice at all.
///
/// # Safety
///
/// `ptr` must point at `len` initialised `T`s for as long as the slice lives.
unsafe fn declared<'a, T>(ptr: *const T, len: usize) -> &'a [T] {
	if len == 0 || ptr.is_null() {
		return &[];
	}
	// SAFETY: the caller guarantees `len` entries at a non-null `ptr`.
	unsafe { std::slice::from_raw_parts(ptr, len) }
}

/// Compare one declaration against the registry.
fn check(
	ident: &str,
	declared: &[FznsoType],
	defined: &BTreeMap<String, Entry>,
	transposals: &BTreeMap<Vec<String>, String>,
) -> Verdict {
	let Some(entry) = defined.get(ident) else {
		return match transposals.get(&transposed(ident)) {
			Some(name) if name != ident => Verdict::Stale(name.clone()),
			_ => Verdict::Own,
		};
	};

	let mut problems = Vec::new();
	if declared.len() != entry.args.len() {
		problems.push(format!(
			"takes {} argument(s), the registry defines {}",
			declared.len(),
			entry.args.len()
		));
		return Verdict::Mismatch(problems);
	}
	for (i, (got, want)) in declared.iter().zip(&entry.args).enumerate() {
		if !narrows(got, want) {
			problems.push(format!(
				"argument {}: declared `{}`, registry says `{}`{}",
				i + 1,
				spell(got),
				spell(want),
				if got.base == want.base && got.list_of == want.list_of && got.set_of == want.set_of
				{
					" (widening; only narrowing is allowed)"
				} else {
					""
				}
			));
		}
	}
	if problems.is_empty() {
		Verdict::Conforms
	} else {
		Verdict::Mismatch(problems)
	}
}

/// Tally and print one capability list.
struct Report {
	conforms: usize,
	mismatches: Vec<(String, Vec<String>)>,
	stale: Vec<(String, String)>,
	own: Vec<String>,
}

impl Report {
	fn new() -> Self {
		Report {
			conforms: 0,
			mismatches: Vec::new(),
			stale: Vec::new(),
			own: Vec::new(),
		}
	}

	fn record(&mut self, ident: &str, verdict: Verdict) {
		match verdict {
			Verdict::Conforms => self.conforms += 1,
			Verdict::Mismatch(p) => self.mismatches.push((ident.to_string(), p)),
			Verdict::Stale(name) => self.stale.push((ident.to_string(), name)),
			Verdict::Own => self.own.push(ident.to_string()),
		}
	}

	/// Print the report. Returns true if anything failed.
	fn print(&self, what: &str, prefix: &str) -> bool {
		let total = self.conforms + self.mismatches.len() + self.stale.len() + self.own.len();
		println!("\n{what}: {total} declared, {} conform", self.conforms);

		for (ident, problems) in &self.mismatches {
			println!("  FAIL {ident}");
			for p in problems {
				println!("       {p}");
			}
		}
		for (ident, should_be) in &self.stale {
			println!("  FAIL {ident} — the registry spells this `{should_be}`");
		}
		// A solver's own names are correct by the naming rule as long as they
		// carry its prefix, which is the one thing worth pointing out.
		let unprefixed: Vec<_> = self
			.own
			.iter()
			.filter(|i| !i.starts_with(prefix))
			.cloned()
			.collect();
		if !unprefixed.is_empty() {
			println!(
				"  {} name(s) are neither in the registry nor prefixed `{prefix}`:",
				unprefixed.len()
			);
			for chunk in unprefixed.chunks(4) {
				println!("       {}", chunk.join("  "));
			}
		}
		!self.mismatches.is_empty() || !self.stale.is_empty()
	}
}

/// Index the registry's names by their words, so a transposed name can be
/// found.
fn transposals(defined: &BTreeMap<String, Entry>) -> BTreeMap<Vec<String>, String> {
	let mut out = BTreeMap::new();
	for ident in defined.keys() {
		out.insert(transposed(ident), ident.clone());
	}
	out
}

/// Which registry names a MiniZinc library declares an `fzn_<ident>` for.
///
/// The overlay is what makes a registry constraint reachable: a model calls a
/// global, the global calls `fzn_<ident>`, and a solver declaring that
/// identifier has the body dropped in favour of its own propagator. A registry
/// name with no such predicate anywhere cannot be reached however it is
/// declared, which is what this counts.
fn coverage(registry: &Registry, library: &Path) -> Result<ExitCode, String> {
	let mut declared = std::collections::BTreeSet::new();
	let mut dir: Vec<_> = std::fs::read_dir(library)
		.map_err(|e| format!("{}: {e}", library.display()))?
		.filter_map(Result::ok)
		.map(|e| e.path())
		.filter(|p| p.extension().is_some_and(|e| e == "mzn"))
		.collect();
	dir.sort();

	// A registry name that is also a FlatZinc builtin needs no route: MiniZinc
	// emits the builtin under that very name, and a solver declaring the
	// identifier matches it directly. Only a constraint MiniZinc reaches under
	// some *other* name needs an `fzn_<ident>` to bridge the two.
	//
	// `flatzinc_builtins.mzn` sits in `std`, one directory up from the overlay.
	let builtins = library
		.parent()
		.map(|p| p.join("std/flatzinc_builtins.mzn"));
	if let Some(path) = builtins.filter(|p| p.exists()) {
		let text =
			std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
		for line in text.lines() {
			let Some(rest) = line.trim_start().strip_prefix("predicate ") else {
				continue;
			};
			let ident: String = rest
				.chars()
				.take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
				.collect();
			if !ident.is_empty() {
				declared.insert(ident);
			}
		}
	}

	for path in &dir {
		let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
		for line in text.lines() {
			let Some(rest) = line.trim_start().strip_prefix("predicate fzn_") else {
				continue;
			};
			let ident: String = rest
				.chars()
				.take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
				.collect();
			if !ident.is_empty() {
				declared.insert(ident);
			}
		}
	}

	// Grouped by the type prefix the registry names each constraint for, since
	// that is how the work divides.
	let mut missing: BTreeMap<&str, Vec<&String>> = BTreeMap::new();
	let prefixes = ["int_", "bool_", "float_", "set_of_int_", "graph_"];
	for ident in registry.constraints.keys() {
		if declared.contains(ident) {
			continue;
		}
		let group = prefixes
			.iter()
			.find(|p| ident.starts_with(*p))
			.copied()
			.unwrap_or("other");
		missing.entry(group).or_default().push(ident);
	}

	let total = registry.constraints.len();
	let absent: usize = missing.values().map(Vec::len).sum();
	println!(
		"{} of {total} registry names are reachable — an `fzn_<ident>` in {}, or a\n\
		 FlatZinc builtin of the same name, which needs no rewriting.",
		total - absent,
		library.display()
	);
	for (group, idents) in &missing {
		println!("\n{group} — {} unreachable", idents.len());
		for chunk in idents.chunks(3) {
			println!(
				"  {}",
				chunk
					.iter()
					.map(|i| format!("{i:<38}"))
					.collect::<String>()
					.trim_end()
			);
		}
	}
	Ok(if absent == 0 {
		ExitCode::SUCCESS
	} else {
		ExitCode::FAILURE
	})
}

fn usage(argv0: &str) -> ExitCode {
	eprintln!(
		"usage: {argv0} check <solver library> [registry]\n\
		        {argv0} coverage <minizinc library> [registry]\n\n\
		 `check` reads a solver's capability lists and holds every registry name\n\
		 it declares to the registry's types. `coverage` reads a directory of\n\
		 MiniZinc files and reports the registry names no `fzn_<ident>` reaches.\n\
		 The registry defaults to `docs/registry` beside this checkout."
	);
	ExitCode::FAILURE
}

fn main() -> ExitCode {
	let args: Vec<String> = env::args().collect();
	let argv0 = args
		.first()
		.map_or("fznso-conform", String::as_str)
		.to_string();
	if args.len() < 3 {
		return usage(&argv0);
	}
	let subcommand = args[1].clone();
	let target = PathBuf::from(&args[2]);
	let registry_dir = args.get(3).map_or_else(
		|| {
			PathBuf::from(env!("CARGO_MANIFEST_DIR"))
				.join("../../docs/registry")
				.clean()
		},
		PathBuf::from,
	);

	let registry = match Registry::load(&registry_dir) {
		Ok(r) => r,
		Err(e) => {
			eprintln!("cannot read the registry: {e}");
			return ExitCode::FAILURE;
		}
	};

	match subcommand.as_str() {
		"coverage" => {
			return match coverage(&registry, &target) {
				Ok(code) => code,
				Err(e) => {
					eprintln!("{e}");
					ExitCode::FAILURE
				}
			};
		}
		"check" => {}
		other => {
			eprintln!("unknown subcommand `{other}`\n");
			return usage(&argv0);
		}
	}
	let lib_path = target;

	// SAFETY: loading a shared object runs its initialisers; the path comes
	// from the operator, who is the one asking for it to be inspected.
	let lib = match unsafe { Library::new(&lib_path) } {
		Ok(l) => l,
		Err(e) => {
			eprintln!("cannot load {}: {e}", lib_path.display());
			return ExitCode::FAILURE;
		}
	};

	// The solver's own name, which its prefixed names must start with. It is
	// not exposed by `Library`, so it comes from the file it was loaded from —
	// which is where the entry-point prefix comes from too.
	let name = lib_path
		.file_stem()
		.map(|s| s.to_string_lossy())
		.map_or_else(String::new, |s| {
			s.strip_prefix("lib").unwrap_or(&s).to_string()
		});
	let prefix = format!("{name}_");
	println!(
		"{name} — checked against {} registry constraint(s)",
		registry.constraints.len()
	);

	let mut failed = false;

	// --- constraints ------------------------------------------------------
	let mut report = Report::new();
	let by_words = transposals(&registry.constraints);
	let list = lib.constraint_types();
	// SAFETY: the library reports `len` entries at `constraints`.
	let entries = unsafe { declared(list.constraints, list.len) };
	for c in entries {
		// SAFETY: as above, `arg_len` types at `arg_types`.
		let types = unsafe { declared(c.arg_types, c.arg_len) };
		// SAFETY: the string is the library's own, valid while it is loaded.
		let ident = unsafe { c.ident.as_str() }.unwrap_or_default().to_string();
		let verdict = check(&ident, types, &registry.constraints, &by_words);
		report.record(&ident, verdict);
	}
	failed |= report.print("Constraints", &prefix);

	// --- objectives -------------------------------------------------------
	let mut report = Report::new();
	let by_words = transposals(&registry.objectives);
	let list = lib.objective_list();
	// SAFETY: the library reports `len` entries at `objectives`.
	let entries = unsafe { declared(list.objectives, list.len) };
	for o in entries {
		// SAFETY: the string is the library's own, valid while it is loaded.
		let ident = unsafe { o.ident.as_str() }.unwrap_or_default().to_string();
		let verdict = check(&ident, &[o.arg_type], &registry.objectives, &by_words);
		report.record(&ident, verdict);
	}
	failed |= report.print("Objectives", &prefix);

	// --- options ----------------------------------------------------------
	let mut report = Report::new();
	let by_words = transposals(&registry.options);
	let list = lib.option_list();
	// SAFETY: the library reports `len` entries at `options`.
	let entries = unsafe { declared(list.options, list.len) };
	for o in entries {
		// SAFETY: the string is the library's own, valid while it is loaded.
		let ident = unsafe { o.ident.as_str() }.unwrap_or_default().to_string();
		let verdict = check(&ident, &[o.arg_ty], &registry.options, &by_words);
		report.record(&ident, verdict);
	}
	failed |= report.print("Options", &prefix);

	// --- statistics -------------------------------------------------------
	let mut report = Report::new();
	let by_words = transposals(&registry.statistics);
	let list = lib.statistic_list();
	// SAFETY: the library reports `len` entries at `statistics`.
	let entries = unsafe { declared(list.stats, list.len) };
	for s in entries {
		// SAFETY: the string is the library's own, valid while it is loaded.
		let ident = unsafe { s.ident.as_str() }.unwrap_or_default().to_string();
		let verdict = check(&ident, &[s.ty], &registry.statistics, &by_words);
		report.record(&ident, verdict);
	}
	failed |= report.print("Statistics", &prefix);

	println!();
	if failed {
		println!("does not conform");
		ExitCode::FAILURE
	} else {
		println!("conforms");
		ExitCode::SUCCESS
	}
}

/// Resolve `..` textually, so the default registry path prints readably.
trait Clean {
	fn clean(self) -> PathBuf;
}

impl Clean for PathBuf {
	fn clean(self) -> PathBuf {
		let mut out = PathBuf::new();
		for part in self.components() {
			match part {
				std::path::Component::ParentDir => {
					out.pop();
				}
				other => out.push(other),
			}
		}
		out
	}
}
