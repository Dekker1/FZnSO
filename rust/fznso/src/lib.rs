//! FZnSO Protocol Library
//!
//! This crate defines an interface for the FZnSO protocol, which allows the
//! incremental usage of solvers that can solve decision and optimization
//! problems. The goal of the interface is to easily use different solvers in a
//! unified way, and to allow the dynamic loading of solver libraries (as DLLs).

mod library;
mod model;
mod solution;
mod solver;

use fznso_types::{
	FznsoConstraintIdx, FznsoConstraintList, FznsoConstraintType, FznsoDecisionIdx, FznsoObjective,
	FznsoObjectiveList, FznsoOption, FznsoOptionList, FznsoType, FznsoTypeList, FznsoValueRef,
};

pub use crate::{library::Library, model::Model, solution::Solution, solver::Solver};

/// Status enumeration that represents the outcome of [`Solver::run`].
///
/// This is an extended version of [`fznso_types::FznsoStatus`], which does not
/// include the actual error message when an error occurs.
pub enum Status {
	Complete,
	Incomplete,
	Error(String),
}

pub type ConstraintList<'a> = FznsoConstraintList<'a>;
pub type ConstraintType<'a> = FznsoConstraintType<'a>;
pub type Objective<'a> = FznsoObjective<'a>;
pub type ObjectiveList<'a> = FznsoObjectiveList<'a>;
pub type Option<'a> = FznsoOption<'a>;
pub type OptionList<'a> = FznsoOptionList<'a>;
pub type Type = FznsoType;
pub type TypeList<'a> = FznsoTypeList<'a>;
pub type Value<'a> = FznsoValueRef<'a>;
pub type DecisionIdx = FznsoDecisionIdx;
pub type ConstraintIdx = FznsoConstraintIdx;

#[cfg(test)]
mod tests {}
