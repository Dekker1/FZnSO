//! The log format: a stream of MessagePack values, one [`Header`] and then one
//! [`Entry`] per recorded call.
//!
//! Structs are written as maps with their field names, and enum variants by
//! name, so any MessagePack reader can show a log without this crate. Values,
//! annotations, types and statuses use the `serde` support of [`fznso`].

use std::io::{self, Read, Write};

use fznso::{DecisionIdx, OwnedAnnotation, OwnedValue, Status, Type};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// The `magic` field of every log's [`Header`].
pub const MAGIC: &str = "fznso-record";

/// The version of this format, in the `format` field of the [`Header`].
pub const FORMAT: u32 = 1;

/// The first value in a log.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Header {
	/// Always [`MAGIC`].
	pub magic: String,
	/// The format version, [`FORMAT`] when written by this crate.
	pub format: u32,
	/// The `FZNSO_ABI_VERSION` of the recorder, for a person reading the log.
	pub abi: u32,
	/// The recorded solver, as given in `FZNSO_RECORD_SOLVER`: a name or a
	/// path.
	pub solver: String,
}

/// One recorded event, and the solver instance it belongs to.
///
/// The recorder writes an `Entry<&Event>`, so it need not give up the event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry<E = Event> {
	/// The solver instance: its position in the order of `solver_create`
	/// calls, from 0.
	pub solver: u32,
	/// What happened.
	pub event: E,
}

/// A recorded event.
///
/// The events of one `run` are a [`Run`](Event::Run), then any
/// [`Solution`](Event::Solution)s and at most one [`StopAt`](Event::StopAt),
/// then a [`RunEnd`](Event::RunEnd). A log whose solver crashed ends
/// without the `RunEnd`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Event {
	/// `solver_create`.
	Create,
	/// `solver_free`.
	Free,
	/// `solver_option_set`, and the error it reported, if any.
	OptionSet {
		/// The option.
		ident: String,
		/// The value set.
		value: OwnedValue,
		/// The error message, if the solver refused the value.
		error: Option<String>,
	},
	/// `solver_option_get`, and its result.
	OptionGet {
		/// The option.
		ident: String,
		/// The result.
		value: OwnedValue,
	},
	/// `solver_statistic`, and its result.
	Statistic {
		/// The statistic.
		ident: String,
		/// The result, for a person reading the log. Replay does not compare
		/// it, since a statistic may measure time.
		value: OwnedValue,
	},
	/// The start of `solver_run`.
	Run {
		/// The part of the model the recorder had not written before.
		snapshot: Snapshot,
		/// Whether the application passed an `on_message` callback.
		on_message: bool,
		/// Whether the application passed a `should_stop` callback.
		should_stop: bool,
	},
	/// A solution: the value of each decision with `decision_in_solution`, in
	/// index order.
	Solution {
		/// The values.
		values: Vec<OwnedValue>,
	},
	/// The first `should_stop` call that returned `true`, counted from 0.
	StopAt {
		/// The call's position.
		call: u64,
	},
	/// The end of `solver_run`.
	RunEnd {
		/// What the run returned.
		status: Status,
	},
	/// The model broke a rule of the specification. Written before the
	/// [`Run`](Event::Run) it belongs to.
	Violation {
		/// What was wrong.
		message: String,
	},
}

/// A model as a run saw it, from layer `from_layer` upward.
///
/// Layers below `from_layer` are the ones the previous snapshot of the same
/// solver ended with, unchanged.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
	/// The first layer in `layers`.
	pub from_layer: usize,
	/// `layer_unchanged`.
	pub unchanged: usize,
	/// `layer_permanent`.
	pub permanent: usize,
	/// `layer_redundant_index` for each index below `layer_redundant_len`.
	pub redundant: Vec<usize>,
	/// Layers `from_layer..layer_len`.
	pub layers: Vec<Layer>,
	/// The objective.
	pub objective: Objective,
}

/// The decisions and constraints one layer adds.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Layer {
	/// The decisions, in index order.
	pub decisions: Vec<Decision>,
	/// The constraints, in index order.
	pub constraints: Vec<Constraint>,
}

/// A decision, as the model accessors describe it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Decision {
	/// `decision_type`.
	#[serde(rename = "type")]
	pub ty: Type,
	/// `decision_domain`.
	pub domain: OwnedValue,
	/// `decision_name`.
	pub name: Option<String>,
	/// `decision_defined`.
	pub defined: bool,
	/// `decision_in_solution`.
	pub in_solution: bool,
	/// `decision_annotation` for each index below `decision_annotation_len`.
	pub annotations: Vec<OwnedAnnotation>,
}

/// A constraint, as the model accessors describe it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Constraint {
	/// `constraint_ident`.
	pub ident: String,
	/// `constraint_argument` for each index below `constraint_argument_len`.
	pub arguments: Vec<OwnedValue>,
	/// `constraint_defines`.
	pub defines: Option<DecisionIdx>,
	/// `constraint_annotation` for each index below
	/// `constraint_annotation_len`.
	pub annotations: Vec<OwnedAnnotation>,
}

/// The objective, as the model accessors describe it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Objective {
	/// `objective_ident`, or `None` for a satisfaction problem.
	pub ident: Option<String>,
	/// `objective_arg`.
	pub arg: OwnedValue,
	/// `objective_annotation` for each index below `objective_annotation_len`.
	pub annotations: Vec<OwnedAnnotation>,
}

/// Write one value of the log.
pub fn write<T: Serialize>(out: &mut impl Write, item: &T) -> io::Result<()> {
	rmp_serde::encode::write_named(out, item).map_err(io::Error::other)
}

/// Read one value of the log, or `None` at the end of the input.
pub fn read<T: DeserializeOwned>(
	input: &mut impl Read,
) -> Result<Option<T>, rmp_serde::decode::Error> {
	match rmp_serde::decode::from_read(input) {
		Ok(item) => Ok(Some(item)),
		Err(rmp_serde::decode::Error::InvalidMarkerRead(e))
			if e.kind() == io::ErrorKind::UnexpectedEof =>
		{
			Ok(None)
		}
		Err(e) => Err(e),
	}
}

/// Read the header of a log, and check that it is a log this crate reads.
pub fn read_header(input: &mut impl Read) -> Result<Header, String> {
	let header: Header = read(input)
		.map_err(|e| e.to_string())?
		.ok_or("the log is empty")?;
	if header.magic != MAGIC || header.format != FORMAT {
		return Err(format!("not a format {FORMAT} fznso-record log"));
	}
	Ok(header)
}

/// The entries of a log, read one at a time after its header.
pub fn entries(
	mut input: impl Read,
) -> impl Iterator<Item = Result<Entry, rmp_serde::decode::Error>> {
	std::iter::from_fn(move || read(&mut input).transpose())
}

#[cfg(test)]
mod tests {
	use fznso::{RangeList, TypeBase, fznso_types::FznsoConstraintIdx};

	use super::*;

	/// Every event and value kind survives a write and a read.
	#[test]
	fn round_trip() {
		let all_values = OwnedValue::List(vec![
			OwnedValue::Absent,
			OwnedValue::Bool(true),
			OwnedValue::Int(-7),
			OwnedValue::Float(1.5),
			OwnedValue::String("s".into()),
			OwnedValue::Decision(DecisionIdx::from(3)),
			OwnedValue::Constraint(FznsoConstraintIdx(4)),
			OwnedValue::IntSet(RangeList::from_iter([1..=2, 5..=9])),
			OwnedValue::FloatSet(RangeList::from(0.5..=1.0)),
		]);
		let ann = OwnedAnnotation::new("output_var", vec![all_values.clone()]);
		let snapshot = Snapshot {
			from_layer: 1,
			unchanged: 1,
			permanent: 2,
			redundant: vec![0],
			layers: vec![Layer {
				decisions: vec![Decision {
					ty: Type::new(TypeBase::FznsoTypeBaseInt)
						.decision(true)
						.opt(true),
					domain: OwnedValue::IntSet(RangeList::from(0..=10)),
					name: Some("x".into()),
					defined: false,
					in_solution: true,
					annotations: vec![ann.clone()],
				}],
				constraints: vec![Constraint {
					ident: "int_le".into(),
					arguments: vec![all_values.clone()],
					defines: Some(DecisionIdx::from(0)),
					annotations: vec![ann.clone()],
				}],
			}],
			objective: Objective {
				ident: Some("minimize".into()),
				arg: OwnedValue::Decision(DecisionIdx::from(0)),
				annotations: vec![ann],
			},
		};
		let events = [
			Event::Create,
			Event::OptionSet {
				ident: "o".into(),
				value: all_values.clone(),
				error: Some("no".into()),
			},
			Event::OptionGet {
				ident: "o".into(),
				value: OwnedValue::Int(1),
			},
			Event::Statistic {
				ident: "s".into(),
				value: OwnedValue::Float(0.25),
			},
			Event::Violation {
				message: "bad".into(),
			},
			Event::Run {
				snapshot,
				on_message: true,
				should_stop: false,
			},
			Event::Solution {
				values: vec![all_values],
			},
			Event::StopAt { call: 12 },
			Event::RunEnd {
				status: Status::Error("e".into()),
			},
			Event::RunEnd {
				status: Status::Incomplete,
			},
			Event::Free,
		];

		let header = Header {
			magic: MAGIC.into(),
			format: FORMAT,
			abi: 1,
			solver: "example".into(),
		};
		let mut buf = Vec::new();
		write(&mut buf, &header).unwrap();
		for (i, event) in events.iter().enumerate() {
			let entry = Entry {
				solver: u32::try_from(i).unwrap(),
				event: event.clone(),
			};
			write(&mut buf, &entry).unwrap();
		}

		let mut input = buf.as_slice();
		assert_eq!(read::<Header>(&mut input).unwrap(), Some(header));
		for (i, event) in events.into_iter().enumerate() {
			let entry: Entry = read(&mut input).unwrap().unwrap();
			assert_eq!(entry.solver, u32::try_from(i).unwrap());
			assert_eq!(entry.event, event);
		}
		assert_eq!(read::<Entry>(&mut input).unwrap(), None);
	}
}
