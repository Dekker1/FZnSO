//! Loading a solver library, and asking what it can do.

use std::sync::Arc;

use fznso_core::fznso_types::FznsoStr;
use pyo3::{exceptions::PyRuntimeError, prelude::*};

use crate::{
	solver::DynSolver,
	value::{Type, value_to_py},
};

/// A solver library found on the search path, reported without loading it.
#[pyclass(frozen, module = "fznso")]
pub(crate) struct Discovered {
	/// The solver's name, derived from the file name.
	#[pyo3(get)]
	name: String,
	/// The version parsed from the file name, or empty if it carries none.
	/// Compared component-wise, so `10` is newer than `9`.
	#[pyo3(get)]
	version: String,
	/// The full path to the library.
	#[pyo3(get)]
	path: String,
}

#[pymethods]
impl Discovered {
	fn __repr__(&self) -> String {
		format!(
			"Discovered(name={:?}, version={:?}, path={:?})",
			self.name, self.version, self.path
		)
	}
}

impl From<fznso_core::Discovered> for Discovered {
	fn from(found: fznso_core::Discovered) -> Self {
		Self {
			name: found.name,
			version: found.version,
			path: found.path.to_string_lossy().into_owned(),
		}
	}
}

/// Borrow a capability list's `len` + pointer pair as a slice.
///
/// A library that declares nothing may spell an empty list with a null
/// pointer, which `from_raw_parts` does not allow.
///
/// # Safety
///
/// `ptr` must point at `len` initialised `T`s that outlive `'a`.
unsafe fn borrow_list<'a, T>(ptr: *const T, len: usize) -> &'a [T] {
	if len == 0 {
		return &[];
	}
	// SAFETY: guaranteed by the caller.
	unsafe { std::slice::from_raw_parts(ptr, len) }
}

/// Copy an identifier out of a capability list.
///
/// The lists are `static` in the solver library and stay valid for as long as
/// it is loaded, but a Python object cannot hold that borrow — so this one-shot
/// query copies, unlike everything that reads a model.
fn ident_to_string(ident: FznsoStr<'_>) -> String {
	// SAFETY: the library guarantees each identifier is valid UTF-8 for as long
	// as it stays loaded.
	unsafe { ident.as_str() }.unwrap_or_default().to_owned()
}

/// Spell a list of types the way `repr` does, for the capability `repr`s.
fn repr_types(types: &[Type]) -> String {
	let items: Vec<String> = types.iter().map(Type::__repr__).collect();
	format!("[{}]", items.join(", "))
}

/// A constraint solvers from this library implement natively.
#[pyclass(frozen, get_all, module = "fznso")]
pub(crate) struct ConstraintType {
	/// The registry identifier, e.g. `"int_lin_le"`.
	ident: String,
	/// The type of each argument, in order. A solver may declare a *narrower*
	/// type than the registry does, so check these too.
	argument_types: Vec<Type>,
}

#[pymethods]
impl ConstraintType {
	fn __repr__(&self) -> String {
		format!(
			"ConstraintType({:?}, {})",
			self.ident,
			repr_types(&self.argument_types)
		)
	}
}

/// An objective strategy solvers from this library implement.
#[pyclass(frozen, get_all, module = "fznso")]
pub(crate) struct Objective {
	/// The registry identifier, e.g. `"int_minimize"`.
	ident: String,
	/// The type of the objective's argument.
	argument_type: Type,
}

#[pymethods]
impl Objective {
	fn __repr__(&self) -> String {
		format!(
			"Objective({:?}, {})",
			self.ident,
			self.argument_type.__repr__()
		)
	}
}

/// An option solvers from this library accept.
#[pyclass(frozen, get_all, name = "Option", module = "fznso")]
pub(crate) struct OptionDef {
	/// The identifier passed to `option_set` and `option_get`.
	ident: String,
	/// The type of value the option takes.
	argument_type: Type,
	/// The value the option has before anything sets it.
	default: PyObject,
}

#[pymethods]
impl OptionDef {
	fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
		Ok(format!(
			"Option({:?}, {}, default={})",
			self.ident,
			self.argument_type.__repr__(),
			self.default.bind(py).repr()?
		))
	}
}

/// A statistic solvers from this library report.
#[pyclass(frozen, get_all, module = "fznso")]
pub(crate) struct Statistic {
	/// The identifier passed to `Solution.statistic` or `DynSolver.statistic`.
	ident: String,
	/// The type of the value reported.
	value_type: Type,
	/// Whether it can be read from a solution.
	solution: bool,
	/// Whether it can be read from the solver instance.
	solver: bool,
}

#[pymethods]
impl Statistic {
	fn __repr__(&self) -> String {
		format!(
			"Statistic({:?}, {}, solution={}, solver={})",
			self.ident,
			self.value_type.__repr__(),
			if self.solution { "True" } else { "False" },
			if self.solver { "True" } else { "False" }
		)
	}
}

/// A loaded solver library.
#[pyclass(module = "fznso")]
pub(crate) struct Library {
	inner: Arc<fznso_core::Library>,
}

#[pymethods]
impl Library {
	/// Load a solver library from `path`.
	#[new]
	fn new(path: &str) -> PyResult<Self> {
		// SAFETY: the caller supplies a valid FZnSO solver library.
		let inner = unsafe { fznso_core::Library::new(path) }
			.map_err(|e| PyRuntimeError::new_err(format!("could not load solver: {e}")))?;
		Ok(Self { inner })
	}

	/// The directories searched for solver libraries, in search order.
	///
	/// Solvers live in a directory of their own rather than on the normal
	/// library path. The order is `$FZNSO_SOLVER_PATH`, then the per-user
	/// directory, then the system directories.
	#[staticmethod]
	fn search_paths() -> Vec<String> {
		fznso_core::Library::search_paths()
			.iter()
			.map(|path| path.to_string_lossy().into_owned())
			.collect()
	}

	/// List the solver libraries on the search path, without loading any.
	///
	/// Directories are visited in `search_paths` order and, within one
	/// directory, the highest version of a solver comes first. Nothing is
	/// opened, so the ABI version is not checked here; `find` skips solvers
	/// built for another ABI.
	#[staticmethod]
	fn discover() -> Vec<Discovered> {
		fznso_core::Library::discover()
			.into_iter()
			.map(Discovered::from)
			.collect()
	}

	/// Load a solver by name from the search path.
	///
	/// Without a `version` the newest is chosen, preferring earlier
	/// directories. A `version` selects by whole dotted components, so `"6"`
	/// matches `6.2.1` and `"6.2"` matches `6.2.1` but not `6.1.0`.
	///
	/// Raises `RuntimeError` if no matching solver is installed, naming the
	/// directories that were searched.
	#[staticmethod]
	#[pyo3(signature = (name, version=None))]
	fn find(name: &str, version: Option<&str>) -> PyResult<Self> {
		// SAFETY: the libraries on the search path are valid FZnSO solvers.
		let found = unsafe {
			match version {
				Some(version) => fznso_core::Library::find_version(name, version),
				None => fznso_core::Library::find(name),
			}
		};
		let inner =
			found.map_err(|e| PyRuntimeError::new_err(format!("could not load solver: {e}")))?;
		Ok(Self { inner })
	}

	/// Create a solver instance.
	fn create_solver(&self) -> DynSolver {
		DynSolver {
			inner: self.inner.create_solver(),
		}
	}

	/// The constraints solvers from this library implement natively.
	///
	/// Do not post a constraint that is not in this list, and check the
	/// argument types too — a solver may accept one only in a narrower form
	/// than the registry declares.
	fn constraint_types(&self) -> Vec<ConstraintType> {
		let list = self.inner.constraint_types();
		// SAFETY: the library returns `len` constraint types, each with
		// `arg_len` argument types, valid for as long as it stays loaded.
		unsafe { borrow_list(list.constraints, list.len) }
			.iter()
			.map(|con| ConstraintType {
				ident: ident_to_string(con.ident),
				// SAFETY: as above.
				argument_types: unsafe { borrow_list(con.arg_types, con.arg_len) }
					.iter()
					.map(|&ty| Type { ty })
					.collect(),
			})
			.collect()
	}

	/// The decision-variable types solvers from this library can create.
	fn decision_types(&self) -> Vec<Type> {
		let list = self.inner.decision_types();
		// SAFETY: the library returns `len` types, valid for as long as it
		// stays loaded.
		unsafe { borrow_list(list.types, list.len) }
			.iter()
			.map(|&ty| Type { ty })
			.collect()
	}

	/// The objective strategies solvers from this library implement.
	fn objectives(&self) -> Vec<Objective> {
		let list = self.inner.objective_list();
		// SAFETY: the library returns `len` objectives, valid for as long as it
		// stays loaded.
		unsafe { borrow_list(list.objectives, list.len) }
			.iter()
			.map(|obj| Objective {
				ident: ident_to_string(obj.ident),
				argument_type: Type { ty: obj.arg_type },
			})
			.collect()
	}

	/// The options solvers from this library accept, with types and defaults.
	fn options(&self, py: Python<'_>) -> PyResult<Vec<OptionDef>> {
		let list = self.inner.option_list();
		// SAFETY: the library returns `len` options, valid for as long as it
		// stays loaded.
		unsafe { borrow_list(list.options, list.len) }
			.iter()
			.map(|opt| {
				Ok(OptionDef {
					ident: ident_to_string(opt.ident),
					argument_type: Type { ty: opt.arg_ty },
					default: value_to_py(py, &opt.arg_def)?,
				})
			})
			.collect()
	}

	/// The statistics solvers from this library report, and where each can be
	/// read from.
	fn statistics(&self) -> Vec<Statistic> {
		let list = self.inner.statistic_list();
		// SAFETY: the library returns `len` statistics, valid for as long as it
		// stays loaded.
		unsafe { borrow_list(list.stats, list.len) }
			.iter()
			.map(|stat| Statistic {
				ident: ident_to_string(stat.ident),
				value_type: Type { ty: stat.ty },
				solution: stat.solution,
				solver: stat.solver,
			})
			.collect()
	}
}
