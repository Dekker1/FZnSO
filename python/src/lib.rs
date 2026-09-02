//! Python bindings for FZnSO, built on the `fznso` crate.
//!
//! Python is a *consumer*: it loads a solver library, builds models and runs
//! them. Solvers are not written in Python.
//!
//! One module per role, mirroring the `fznso` crate: [`value`] for what crosses
//! the interface, [`annotation`] for the advice attached to it, [`model`] for
//! the two ways to describe a problem, [`library`] for loading a solver and
//! asking what it can do, and [`solver`] for running one.

mod annotation;
mod library;
mod model;
mod solver;
mod value;

use pyo3::prelude::*;

use crate::{
	annotation::Annotation,
	library::{ConstraintType, Discovered, Library, Objective, OptionDef, Statistic},
	model::{LayeredModel, Model},
	solver::{DynSolver, Solution, Status},
	value::{Constraint, Decision, FloatSet, IntSet, Type, float_set, int_set},
};

#[pymodule]
fn fznso(m: &Bound<'_, PyModule>) -> PyResult<()> {
	m.add_class::<Library>()?;
	m.add_class::<Discovered>()?;
	m.add_class::<DynSolver>()?;
	m.add_class::<LayeredModel>()?;
	m.add_class::<Model>()?;
	m.add_class::<Solution>()?;
	m.add_class::<Decision>()?;
	m.add_class::<Constraint>()?;
	m.add_class::<IntSet>()?;
	m.add_class::<FloatSet>()?;
	m.add_class::<Type>()?;
	m.add_class::<Annotation>()?;
	m.add_class::<ConstraintType>()?;
	m.add_class::<Objective>()?;
	m.add_class::<OptionDef>()?;
	m.add_class::<Statistic>()?;
	m.add_class::<Status>()?;
	m.add_function(wrap_pyfunction!(int_set, m)?)?;
	m.add_function(wrap_pyfunction!(float_set, m)?)?;
	Ok(())
}
