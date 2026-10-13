//! The five capability lists of a solver, as data: copied out of a library to
//! write to a log, and turned back into lists to play a log back.

use std::{marker::PhantomData, slice};

use fznso::{
	ConstraintList, ConstraintType, Library, Objective, ObjectiveList, OptionDef, OptionList,
	OwnedValue, Statistic, StatisticList, Str, Type, TypeList,
};
use serde::{Deserialize, Serialize};

/// What a solver declares it supports.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Capabilities {
	/// `constraint_list`.
	pub constraints: Vec<ConstraintDecl>,
	/// `decision_list`.
	pub decisions: Vec<Type>,
	/// `objective_list`.
	pub objectives: Vec<ObjectiveDecl>,
	/// `option_list`.
	pub options: Vec<OptionDecl>,
	/// `statistic_list`.
	pub statistics: Vec<StatisticDecl>,
}

/// A constraint a solver accepts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConstraintDecl {
	/// The identifier.
	pub ident: String,
	/// The type of each argument.
	pub arguments: Vec<Type>,
}

/// An objective a solver supports.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ObjectiveDecl {
	/// The identifier.
	pub ident: String,
	/// The type of the argument.
	#[serde(rename = "type")]
	pub ty: Type,
}

/// An option a solver accepts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OptionDecl {
	/// The identifier.
	pub ident: String,
	/// The type of the value.
	#[serde(rename = "type")]
	pub ty: Type,
	/// The default value.
	pub default: OwnedValue,
}

/// A statistic a solver reports.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StatisticDecl {
	/// The identifier.
	pub ident: String,
	/// The type of the value.
	#[serde(rename = "type")]
	pub ty: Type,
	/// Whether a solution reports it.
	pub solution: bool,
	/// Whether the solver instance reports it.
	pub solver: bool,
}

/// The five lists in the form the entry points return, valid for the rest of
/// the process.
#[derive(Clone, Debug)]
pub struct Lists {
	/// For `constraint_list`.
	pub constraints: ConstraintList<'static>,
	/// For `decision_list`.
	pub decisions: TypeList<'static>,
	/// For `objective_list`.
	pub objectives: ObjectiveList<'static>,
	/// For `option_list`.
	pub options: OptionList<'static>,
	/// For `statistic_list`.
	pub statistics: StatisticList<'static>,
}

// SAFETY: the lists point at leaked data that nothing changes, so they can be
// read from any thread.
unsafe impl Send for Lists {}
// SAFETY: as above.
unsafe impl Sync for Lists {}

/// The elements of a list a library returned.
///
/// # Safety
///
/// `ptr` is valid for `len` elements, unless `len` is 0.
unsafe fn elements<'a, T>(ptr: *const T, len: usize) -> &'a [T] {
	if len == 0 || ptr.is_null() {
		&[]
	} else {
		// SAFETY: forwarded to the caller.
		unsafe { slice::from_raw_parts(ptr, len) }
	}
}

/// Copy the identifier of a list element.
fn ident(ident: Str<'_>) -> String {
	// SAFETY: a library guarantees its identifiers are valid UTF-8 for as long
	// as it is loaded.
	unsafe { ident.as_str() }.unwrap_or_default().to_owned()
}

/// Give `items` to the rest of the process, for a list to point at.
fn leak<T>(items: Vec<T>) -> &'static [T] {
	Box::leak(items.into_boxed_slice())
}

impl Capabilities {
	/// Copy the lists `lib` declares.
	pub fn of(lib: &Library) -> Self {
		let constraints = lib.constraint_types();
		let decisions = lib.decision_types();
		let objectives = lib.objective_list();
		let options = lib.option_list();
		let statistics = lib.statistic_list();
		// SAFETY: a library returns lists that are valid for their stated
		// lengths for as long as it is loaded, which `lib` shows it is.
		unsafe {
			Self {
				constraints: elements(constraints.constraints, constraints.len)
					.iter()
					.map(|c| ConstraintDecl {
						ident: ident(c.ident),
						arguments: elements(c.arg_types, c.arg_len).to_vec(),
					})
					.collect(),
				decisions: elements(decisions.types, decisions.len).to_vec(),
				objectives: elements(objectives.objectives, objectives.len)
					.iter()
					.map(|o| ObjectiveDecl {
						ident: ident(o.ident),
						ty: o.arg_type,
					})
					.collect(),
				options: elements(options.options, options.len)
					.iter()
					.map(|o| OptionDecl {
						ident: ident(o.ident),
						ty: o.arg_ty,
						default: o.arg_def.into(),
					})
					.collect(),
				statistics: elements(statistics.stats, statistics.len)
					.iter()
					.map(|s| StatisticDecl {
						ident: ident(s.ident),
						ty: s.ty,
						solution: s.solution,
						solver: s.solver,
					})
					.collect(),
			}
		}
	}

	/// Turn the capabilities into lists that stay valid for the rest of the
	/// process, which is what a solver's entry points must return. The memory
	/// is never freed, so call this once.
	pub fn leak(self) -> Lists {
		let name = |ident: String| Str::new(String::leak(ident));
		let constraints = leak(
			self.constraints
				.into_iter()
				.map(|c| {
					let arguments = leak(c.arguments);
					ConstraintType {
						ident: name(c.ident),
						arg_len: arguments.len(),
						arg_types: arguments.as_ptr(),
						lifetime: PhantomData,
					}
				})
				.collect(),
		);
		let decisions = leak(self.decisions);
		let objectives = leak(
			self.objectives
				.into_iter()
				.map(|o| Objective {
					ident: name(o.ident),
					arg_type: o.ty,
					lifetime: PhantomData,
				})
				.collect(),
		);
		let options = leak(
			self.options
				.into_iter()
				.map(|o| {
					let default: &'static OwnedValue = Box::leak(Box::new(o.default));
					OptionDef {
						ident: name(o.ident),
						arg_ty: o.ty,
						arg_def: default.into(),
						lifetime: PhantomData,
					}
				})
				.collect(),
		);
		let statistics = leak(
			self.statistics
				.into_iter()
				.map(|s| Statistic {
					ident: name(s.ident),
					ty: s.ty,
					solution: s.solution,
					solver: s.solver,
					lifetime: PhantomData,
				})
				.collect(),
		);
		Lists {
			constraints: ConstraintList {
				len: constraints.len(),
				constraints: constraints.as_ptr(),
				lifetime: PhantomData,
			},
			decisions: TypeList {
				len: decisions.len(),
				types: decisions.as_ptr(),
				lifetime: PhantomData,
			},
			objectives: ObjectiveList {
				len: objectives.len(),
				objectives: objectives.as_ptr(),
				lifetime: PhantomData,
			},
			options: OptionList {
				len: options.len(),
				options: options.as_ptr(),
				lifetime: PhantomData,
			},
			statistics: StatisticList {
				len: statistics.len(),
				stats: statistics.as_ptr(),
				lifetime: PhantomData,
			},
		}
	}
}

#[cfg(test)]
mod tests {
	use std::sync::OnceLock;

	use fznso::TypeBase;

	use super::*;

	/// Where the test keeps its lists, as a solver would: memory a static
	/// reaches is not a leak to miri, and anything `leak` strands still is.
	static LISTS: OnceLock<Lists> = OnceLock::new();

	/// Leaked lists read back as the capabilities they came from.
	#[test]
	fn leak_keeps_every_field() {
		let int = Type::new(TypeBase::FznsoTypeBaseInt);
		let lists = LISTS.get_or_init(|| {
			Capabilities {
				constraints: vec![ConstraintDecl {
					ident: "int_le".into(),
					arguments: vec![int, int.decision(true)],
				}],
				decisions: vec![int.decision(true)],
				objectives: vec![ObjectiveDecl {
					ident: "minimize".into(),
					ty: int.decision(true),
				}],
				options: vec![OptionDecl {
					ident: "threads".into(),
					ty: int,
					default: OwnedValue::Int(1),
				}],
				statistics: vec![StatisticDecl {
					ident: "nodes".into(),
					ty: int,
					solution: true,
					solver: false,
				}],
			}
			.leak()
		});

		// SAFETY: `leak` made each list valid for its length.
		unsafe {
			let c = &elements(lists.constraints.constraints, lists.constraints.len)[0];
			assert_eq!(c.ident.as_str(), Some("int_le"));
			assert_eq!(elements(c.arg_types, c.arg_len), [int, int.decision(true)]);
			assert_eq!(
				elements(lists.decisions.types, lists.decisions.len),
				[int.decision(true)]
			);
			let o = &elements(lists.objectives.objectives, lists.objectives.len)[0];
			assert_eq!(o.ident.as_str(), Some("minimize"));
			let o = &elements(lists.options.options, lists.options.len)[0];
			assert_eq!(
				(o.ident.as_str(), o.arg_def.get_int()),
				(Some("threads"), 1)
			);
			let s = &elements(lists.statistics.stats, lists.statistics.len)[0];
			assert_eq!(
				(s.ident.as_str(), s.solution, s.solver),
				(Some("nodes"), true, false)
			);
		}
	}
}
