//! `fznso-replay`: make the calls a log recorded, on a solver, and check that
//! it answers the same.
//!
//! ```text
//! fznso-replay <log> [--solver <name or path>] [--strict]
//! ```
//!
//! By default the solver is the one the log recorded. An answer that differs
//! from the recording is a warning, and the replay goes on: the log, not the
//! solver's answers, decides every later call. Statistics are asked for but
//! not compared, since they may measure time. With `--strict`, the replay
//! stops at the first difference.
//!
//! Exits with 0 if every answer matches, 2 if some differ, and 1 on an error.

use std::{
	collections::HashMap,
	fmt::Debug,
	fs::File,
	io::BufReader,
	process::ExitCode,
	sync::atomic::{AtomicU64, Ordering},
};

use fznso::{DynSolution, DynSolver, LayeredModel, Model, OwnedValue, Solution, Solver};
use fznso_record_format::{
	format::{self, Entry, Event, Snapshot},
	load_solver,
};

/// How to call the program.
const USAGE: &str = "usage: fznso-replay <log> [--solver <name or path>] [--strict]";

fn main() -> ExitCode {
	match replay() {
		Ok((entries, 0)) => {
			println!("fznso-replay: all {entries} entries match the recording");
			ExitCode::SUCCESS
		}
		Ok((entries, differences)) => {
			println!(
				"fznso-replay: {differences} of the {entries} replayed entries differ from the \
				 recording"
			);
			ExitCode::from(2)
		}
		Err(message) => {
			eprintln!("fznso-replay: {message}");
			ExitCode::FAILURE
		}
	}
}

/// A recorded solver instance, as the replay rebuilds it.
#[derive(Debug)]
struct Instance {
	/// The solver to replay on.
	solver: DynSolver,
	/// The model, as the latest snapshot left it.
	model: LayeredModel,
	/// The index of every decision in the model with `decision_in_solution`.
	in_solution: Vec<usize>,
}

/// Replay the log named on the command line, and return how many entries it
/// replayed and how many of them differ.
fn replay() -> Result<(usize, usize), String> {
	let mut args = std::env::args().skip(1);
	let (mut log, mut solver, mut strict) = (None, None, false);
	while let Some(arg) = args.next() {
		match arg.as_str() {
			"--solver" => solver = Some(args.next().ok_or(USAGE)?),
			"--strict" => strict = true,
			_ if log.is_none() => log = Some(arg),
			_ => return Err(USAGE.to_owned()),
		}
	}
	let log = log.ok_or(USAGE)?;

	let file = File::open(&log).map_err(|e| format!("cannot open `{log}`: {e}"))?;
	let mut input = BufReader::new(file);
	let header = format::read_header(&mut input).map_err(|e| format!("`{log}`: {e}"))?;
	let spec = solver.unwrap_or(header.solver);
	// SAFETY: the user names an FZnSO solver to replay on.
	let lib = unsafe { load_solver(&spec) }.map_err(|e| format!("cannot load `{spec}`: {e}"))?;
	let bad = |e| format!("cannot read `{log}`: {e}");

	let mut entries = format::entries(input).peekable();
	let mut instances: HashMap<u32, Instance> = HashMap::new();
	let (mut count, mut differences) = (0, 0);
	while let Some(entry) = entries.next() {
		let Entry { solver: id, event } = entry.map_err(bad)?;
		count += 1;
		let at = format!("entry {count} (solver {id})");
		let mut differ = |what: &str, recorded: &dyn Debug, replayed: &dyn Debug| {
			differences += 1;
			eprintln!(
				"fznso-replay: warning: {at}: {what} differs\n  recorded: {recorded:?}\n  \
				 replayed: {replayed:?}"
			);
		};
		let missing = || format!("{at}: the solver does not exist");
		match event {
			Event::Create => {
				let instance = Instance {
					solver: lib.create_solver(),
					model: LayeredModel::empty(),
					in_solution: Vec::new(),
				};
				let _ = instances.insert(id, instance);
			}
			Event::Free => {
				let _ = instances.remove(&id).ok_or_else(missing)?;
			}
			Event::OptionSet {
				ident,
				value,
				error,
			} => {
				let instance = instances.get_mut(&id).ok_or_else(missing)?;
				let replayed = instance.solver.option_set(&ident, (&value).into()).err();
				if error != replayed {
					differ(&format!("option_set `{ident}`"), &error, &replayed);
				}
			}
			Event::OptionGet { ident, value } => {
				let instance = instances.get_mut(&id).ok_or_else(missing)?;
				let replayed = OwnedValue::from(instance.solver.option_get(&ident));
				if value != replayed {
					differ(&format!("option_get `{ident}`"), &value, &replayed);
				}
			}
			Event::Statistic { ident, .. } => {
				let instance = instances.get_mut(&id).ok_or_else(missing)?;
				let _ = instance.solver.statistic(&ident);
			}
			Event::Violation { message } => {
				eprintln!("fznso-replay: {at}: the recorder found a problem: {message}");
			}
			Event::Run {
				snapshot,
				on_message,
				should_stop,
			} => {
				let Instance {
					solver,
					model,
					in_solution,
				} = instances.get_mut(&id).ok_or_else(missing)?;
				apply(model, in_solution, snapshot);

				// The rest of the run: its solutions, when it was stopped, and
				// how it ended. A run without an end is one the recorded
				// solver did not return from.
				let (mut recorded, mut stop_at, mut end) = (Vec::new(), None, None);
				while let Some(entry) = entries.next_if(
					|e| matches!(e, Ok(Entry { solver, event }) if *solver == id && event.in_run()),
				) {
					count += 1;
					match entry.map_err(bad)?.event {
						Event::Solution { values, .. } => recorded.push(values),
						Event::StopAt { call } => stop_at = Some(call),
						Event::Message { .. } => {}
						Event::RunEnd { status } => {
							end = Some(status);
							break;
						}
						_ => unreachable!("`in_run` is false for every other event"),
					}
				}

				// Compare each solution as it arrives, and keep only the first
				// that differs: a different search decision usually changes
				// every solution after it.
				let (mut replayed, mut first_difference) = (0, None);
				let calls = AtomicU64::new(0);
				let stop = || calls.fetch_add(1, Ordering::Relaxed) >= stop_at.unwrap_or(u64::MAX);
				let mut ignore = |_: &str, _: fznso::Value<'_>| {};
				let status = solver.run(
					&*model,
					&mut |sol: &DynSolution<'_>| {
						if first_difference.is_none()
							&& let Some(expected) = recorded.get(replayed)
						{
							let values: Vec<OwnedValue> =
								in_solution.iter().map(|&d| sol.value(d).into()).collect();
							if *expected != values {
								first_difference = Some((replayed, values));
							}
						}
						replayed += 1;
					},
					on_message.then_some(&mut ignore),
					should_stop.then_some(&stop),
				);

				if let Some((n, values)) = &first_difference {
					differ(&format!("solution {n}"), &recorded[*n], values);
				}
				if recorded.len() != replayed {
					differ("the number of solutions", &recorded.len(), &replayed);
				}
				match end {
					Some(end) if end == status => {}
					Some(end) => differ("the run's status", &end, &status),
					None => differ(
						"the run's end (the recorded solver crashed or was killed)",
						&"no end",
						&status,
					),
				}
			}
			Event::Solution { .. }
			| Event::Message { .. }
			| Event::StopAt { .. }
			| Event::RunEnd { .. } => {
				return Err(format!("{at}: {event:?} outside a run"));
			}
		}
		if strict && differences > 0 {
			break;
		}
	}
	Ok((count, differences))
}

/// Bring `model` to the state `snapshot` records, and keep `in_solution` in
/// step with it.
fn apply(model: &mut LayeredModel, in_solution: &mut Vec<usize>, snapshot: Snapshot) {
	if snapshot.from_layer == 0 {
		*model = LayeredModel::empty();
	} else {
		model.truncate(snapshot.from_layer);
	}
	let mut next = model.decision_len();
	in_solution.retain(|&d| d < next);
	for layer in snapshot.layers {
		model.push_layer();
		for d in layer.decisions {
			if d.in_solution {
				in_solution.push(next);
			}
			next += 1;
			let _ = model.add_decision(
				d.ty,
				d.domain,
				d.name,
				d.defined,
				d.in_solution,
				d.annotations,
			);
		}
		for c in layer.constraints {
			let _ = model.add_constraint(c.ident, c.arguments, c.defines, c.annotations);
		}
	}
	// A recorded count above the number of layers is a violation the recorder
	// reported; a `LayeredModel` cannot hold one.
	model.mark_permanent_to(snapshot.permanent.min(model.layer_len()));
	for r in snapshot.redundant {
		// Likewise a redundant layer that is not permanent.
		if r < model.layer_permanent() {
			model.mark_redundant(r);
		}
	}
	let objective = snapshot.objective;
	model.set_objective(objective.ident, objective.arg, objective.annotations);
	model.set_unchanged(snapshot.unchanged);
}
