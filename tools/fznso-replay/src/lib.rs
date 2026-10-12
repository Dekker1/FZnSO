//! Replay the solver's side of a log to an application, in place of the solver
//! that was recorded.
//!
//! Built as `libreplay`, this crate is a solver named `replay` that declares
//! what the recorded solver declared and answers every call from a log that
//! `librecord` wrote. No solver runs. The `fznso-replay` binary in this crate
//! is its counterpart: it replays the application's side of a log to a solver.
//!
//! The log comes from the environment variable `FZNSO_REPLAY_FILE`.
//!
//! An application is expected to make the calls the log holds, in the same
//! order for each solver instance. Where it does not, the replay solver says so
//! on standard error and goes on:
//!
//! - A call the log has further on is answered from there, and the recorded
//!   calls before it are skipped. The search stops at the next run.
//! - A call the log does not have gets an empty answer: an absent value, a
//!   successful `option_set`, or an error from `run`.
//! - A model that differs from the recorded one is described, and the run still
//!   plays the recorded solutions.

#![allow(missing_docs, reason = "the exported ABI entry points are generated")]

use std::{
	collections::HashMap,
	fs::File,
	io::BufReader,
	sync::{
		Arc, OnceLock,
		atomic::{AtomicU32, AtomicUsize, Ordering},
	},
};

use fznso::{
	ConstraintList, Model, ObjectiveList, OptionList, OwnedValue, Solution, Solver, SolverType,
	StatisticList, Status, TypeList, Value, fznso_export,
};
use fznso_record_format::{
	capabilities::Lists,
	format::{self, Entry, Event},
	snapshot::{self, Seen},
};

/// The process-wide state of the replay solver, set up by the first call that
/// needs it.
#[derive(Debug)]
struct Global {
	/// The lists the recorded solver declared.
	lists: Lists,
	/// The events of each recorded solver instance, in order.
	events: HashMap<u32, Vec<Event>>,
	/// The id of the next solver instance.
	next_id: AtomicU32,
}

/// A solver instance that answers from the log.
#[derive(Debug)]
pub struct Replay {
	/// This instance's id in the log.
	id: u32,
	/// The recorded events of this instance.
	events: &'static [Event],
	/// The position in `events` of the next one to play.
	next: AtomicUsize,
	/// The model of the previous run.
	seen: Option<Seen>,
}

/// A recorded solution.
#[derive(Debug)]
pub struct ReplaySolution {
	/// The recorded values, one for each decision with `decision_in_solution`.
	values: &'static [OwnedValue],
	/// The recorded statistics.
	statistics: &'static [(String, OwnedValue)],
	/// For each decision, the position of its value in `values`, or
	/// `usize::MAX` if it has none.
	positions: Arc<[usize]>,
}

/// The replay solver's state, set up on first use.
///
/// Aborts if it cannot be set up: no entry point can report an error before a
/// solver exists.
fn global() -> &'static Global {
	/// The state, once set up.
	static GLOBAL: OnceLock<Global> = OnceLock::new();
	GLOBAL.get_or_init(|| {
		Global::open().unwrap_or_else(|message| {
			eprintln!("libreplay: {message}");
			std::process::abort()
		})
	})
}

/// Report that the application and the log of solver `id` do not agree.
fn warn(id: u32, message: &str) {
	eprintln!("libreplay: solver {id}: {message}");
}

impl Global {
	/// Read the whole log. Answers borrow from it for the rest of the process.
	fn open() -> Result<Self, String> {
		let path = std::env::var("FZNSO_REPLAY_FILE")
			.map_err(|_| "set FZNSO_REPLAY_FILE to the log to play back".to_owned())?;
		let file = File::open(&path).map_err(|e| format!("cannot open `{path}`: {e}"))?;
		let mut input = BufReader::new(file);
		let header = format::read_header(&mut input).map_err(|e| format!("`{path}`: {e}"))?;
		let mut events: HashMap<u32, Vec<Event>> = HashMap::new();
		for entry in format::entries(input) {
			let Entry { solver, event } =
				entry.map_err(|e| format!("cannot read `{path}`: {e}"))?;
			events.entry(solver).or_default().push(event);
		}
		Ok(Self {
			lists: header.capabilities.leak(),
			events,
			next_id: AtomicU32::new(0),
		})
	}
}

impl Replay {
	/// Find the next recorded event that `pick` accepts, and move past it.
	///
	/// The search stops at the next run, so that a call the log does not hold
	/// cannot skip one.
	fn find<T>(&self, call: &str, pick: impl Fn(&'static Event) -> Option<T>) -> Option<T> {
		let start = self.next.load(Ordering::Relaxed);
		for (skipped, event) in self.events[start..].iter().enumerate() {
			if let Some(found) = pick(event) {
				if skipped > 0 {
					warn(
						self.id,
						&format!(
							"{call}: skipped {skipped} recorded events the application did not ask for"
						),
					);
				}
				self.next.store(start + skipped + 1, Ordering::Relaxed);
				return Some(found);
			}
			if matches!(event, Event::Run { .. } | Event::Free) {
				break;
			}
		}
		warn(
			self.id,
			&format!("{call}: the log does not hold this call here"),
		);
		None
	}

	/// Move past the rest of the run that is playing.
	fn skip_run(&mut self) {
		let next = self.next.get_mut();
		while self.events.get(*next).is_some_and(Event::in_run) {
			*next += 1;
		}
	}
}

impl Solution for ReplaySolution {
	fn statistic(&self, name: &str) -> Value<'_> {
		(self.statistics.iter())
			.find(|(ident, _)| ident == name)
			.map_or_else(|| (&()).into(), |(_, value)| value.into())
	}

	fn value(&self, decision_idx: usize) -> Value<'_> {
		(self.positions.get(decision_idx))
			.and_then(|&position| self.values.get(position))
			.map_or_else(|| (&()).into(), Into::into)
	}
}

impl Solver for Replay {
	type Solution<'s> = ReplaySolution;

	fn option_get(&self, name: &str) -> Value<'_> {
		self.find(&format!("option_get `{name}`"), |event| match event {
			Event::OptionGet { ident, value } if ident == name => Some(value),
			_ => None,
		})
		.map_or_else(|| (&()).into(), Into::into)
	}

	fn option_set(&mut self, name: &str, value: Value<'_>) -> Result<(), String> {
		let call = format!("option_set `{name}`");
		let Some((recorded, error)) = self.find(&call, |event| match event {
			Event::OptionSet {
				ident,
				value,
				error,
			} if ident == name => Some((value, error)),
			_ => None,
		}) else {
			return Ok(());
		};
		let value = OwnedValue::from(value);
		if *recorded != value {
			warn(
				self.id,
				&format!(
					"{call}: the value differs\n  recorded: {recorded:?}\n  now:      {value:?}"
				),
			);
		}
		error.clone().map_or(Ok(()), Err)
	}

	fn statistic(&self, name: &str) -> Value<'_> {
		self.find(&format!("statistic `{name}`"), |event| match event {
			Event::Statistic { ident, value } if ident == name => Some(value),
			_ => None,
		})
		.map_or_else(|| (&()).into(), Into::into)
	}

	fn run<M, F, G, H>(
		&mut self,
		model: &M,
		on_solution: &mut F,
		mut on_message: Option<&mut G>,
		should_stop: Option<&H>,
	) -> Status
	where
		M: Model + Sync,
		F: for<'s> FnMut(&'s Self::Solution<'s>) + Send,
		G: FnMut(&str, Value<'_>) + Send,
		H: Fn() -> bool + Send + Sync,
	{
		let Some(recorded) = self.find("run", |event| match event {
			Event::Run { snapshot, .. } => Some(snapshot),
			_ => None,
		}) else {
			return Status::Error("libreplay: the log holds no run here".to_owned());
		};

		// Read the model as the recorder does, so that the same model gives the
		// same snapshot, and say where it is not the recorded one.
		let mut taken = snapshot::take(model, &mut self.seen);
		for violation in &taken.violations {
			warn(self.id, violation);
		}
		if let Some(difference) = snapshot::difference(recorded, &taken.snapshot) {
			warn(
				self.id,
				&format!("run: the model is not the recorded one: {difference}"),
			);
		}
		if let Some(seen) = &mut self.seen {
			seen.restore(&mut taken.snapshot.layers);
		}
		let mut positions = vec![usize::MAX; model.decision_len()];
		for (position, &decision) in taken.in_solution.iter().enumerate() {
			positions[decision] = position;
		}
		let positions: Arc<[usize]> = positions.into();

		// As a solver does: ask before the search, and after each solution.
		let stopped = || should_stop.is_some_and(|poll| poll());
		if stopped() {
			self.skip_run();
			return Status::Incomplete;
		}
		loop {
			let next = self.next.get_mut();
			let Some(event) = self.events.get(*next).filter(|e| e.in_run()) else {
				return Status::Error("libreplay: the recording stops during this run".to_owned());
			};
			*next += 1;
			match event {
				Event::Solution { values, statistics } => {
					on_solution(&ReplaySolution {
						values,
						statistics,
						positions: Arc::clone(&positions),
					});
					if stopped() {
						self.skip_run();
						return Status::Incomplete;
					}
				}
				Event::Message { scope, value } => {
					if let Some(emit) = &mut on_message {
						emit(scope, value.into());
					}
				}
				Event::RunEnd { status } => return status.clone(),
				// When the recorded application asked to stop: this one decides
				// for itself.
				_ => {}
			}
		}
	}
}

impl SolverType for Replay {
	fn new() -> Self {
		let g = global();
		let id = g.next_id.fetch_add(1, Ordering::Relaxed);
		let events = g.events.get(&id).map_or(&[][..], Vec::as_slice);
		let created = matches!(events.first(), Some(Event::Create));
		if !created {
			warn(id, "solver_create: the log does not hold this solver");
		}
		Self {
			id,
			events,
			next: AtomicUsize::new(usize::from(created)),
			seen: None,
		}
	}

	fn constraint_list() -> ConstraintList<'static> {
		global().lists.constraints.clone()
	}

	fn decision_list() -> TypeList<'static> {
		global().lists.decisions.clone()
	}

	fn objective_list() -> ObjectiveList<'static> {
		global().lists.objectives.clone()
	}

	fn option_list() -> OptionList<'static> {
		global().lists.options.clone()
	}

	fn statistic_list() -> StatisticList<'static> {
		global().lists.statistics.clone()
	}
}

fznso_export!(Replay, "replay");
