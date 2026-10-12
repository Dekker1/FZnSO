//! The log of the calls into an FZnSO solver.
//!
//! `fznso-record` writes a log, and `fznso-replay` replays one: the
//! application's side to a solver, or the solver's side to an application.
//! This crate is what they share: the [`format`], the solver's declared
//! [`capabilities`], and reading a model into a [`snapshot`].

pub mod capabilities;
pub mod format;
pub mod snapshot;

use std::sync::Arc;

use fznso::{Library, LoadError};

/// Load the solver `spec` names: a path if it contains a path separator, and
/// otherwise a name to find on the search path.
///
/// # Safety
///
/// As for [`Library::new`]: the library must be an FZnSO solver.
pub unsafe fn load_solver(spec: &str) -> Result<Arc<Library>, LoadError> {
	// SAFETY: forwarded to the caller.
	unsafe {
		if spec.contains(['/', '\\']) {
			Library::new(spec)
		} else {
			Library::find(spec)
		}
	}
}
