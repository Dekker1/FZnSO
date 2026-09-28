//! Read a model into a [`Snapshot`], and check it against the rules of the
//! specification.

use fznso::{
	AnnotationRef, DecisionIdx, Model, OwnedAnnotation, OwnedValue, ValueExt, ValueView,
	fznso_types::{FznsoConstraintIdx, FznsoDecisionIdx},
};

use crate::format::{Constraint, Decision, Layer, Objective, Snapshot};

/// What the recorder remembers of a solver's previous run.
#[derive(Debug)]
pub(crate) struct Seen {
	/// Every layer of the model. The layers the snapshot holds are missing
	/// until [`Seen::restore`] puts them back.
	layers: Vec<Layer>,
	/// `layer_permanent`.
	permanent: usize,
	/// The redundant layer indices, in the order the model gave them.
	redundant: Vec<usize>,
}

/// A snapshot to write, with what the recorder needs from it during the run.
#[derive(Debug)]
pub(crate) struct Taken {
	/// The part of the model not written before.
	pub(crate) snapshot: Snapshot,
	/// The index of every decision with `decision_in_solution`.
	pub(crate) in_solution: Vec<usize>,
	/// A message for each rule the model breaks.
	pub(crate) violations: Vec<String>,
}

/// Copies a model, and collects the rules it breaks on the way.
///
/// Values are checked while still borrowed, because the copy stores a set as a
/// `RangeList`, which would hide ranges that are out of order.
#[derive(Debug)]
struct Reader<'m, M> {
	/// The model.
	model: &'m M,
	/// `decision_len`.
	decision_len: usize,
	/// `constraint_len`.
	constraint_len: usize,
	/// A message for each rule the model breaks.
	violations: Vec<String>,
}

/// Read all of `model`, check it against the solver's previous run in `seen`,
/// and remember it there for the next one.
///
/// The snapshot starts at the first layer that differs from `seen`. After a
/// violation it starts at 0, so replay does not build on a wrong layer.
///
/// The snapshot's layers move out of `seen` rather than being copied: give
/// them back with [`Seen::restore`] once the snapshot is written.
pub(crate) fn take<M: Model>(model: &M, seen: &mut Option<Seen>) -> Taken {
	let mut reader = Reader {
		model,
		decision_len: model.decision_len(),
		constraint_len: model.constraint_len(),
		violations: Vec::new(),
	};
	let mut layers = reader.layers();
	let objective = model.objective();
	let place = || "the objective".to_owned();
	let objective = Objective {
		ident: (!objective.is_satisfy()).then(|| objective.ident().to_owned()),
		arg: reader.value(objective.arg(), &place),
		annotations: objective
			.annotations()
			.map(|a| reader.annotation(a, &place))
			.collect(),
	};
	let mut violations = reader.violations;
	let unchanged = model.layer_unchanged();
	let permanent = model.layer_permanent();
	let redundant: Vec<usize> = (0..model.layer_redundant_len())
		.map(|i| model.layer_redundant_index(i))
		.collect();

	let layer_len = layers.len();
	if unchanged > layer_len {
		violations.push(format!(
			"layer_unchanged is {unchanged}, but there are {layer_len} layers"
		));
	}
	if permanent > layer_len {
		violations.push(format!(
			"layer_permanent is {permanent}, but there are {layer_len} layers"
		));
	}
	for &r in &redundant {
		if r >= permanent {
			violations.push(format!(
				"layer {r} is redundant, but only layers below {permanent} are permanent"
			));
		}
	}

	let from_layer = match seen {
		None => {
			if unchanged != 0 {
				violations.push(format!(
					"the solver has not run before, but layer_unchanged is {unchanged}"
				));
			}
			0
		}
		Some(prev) => {
			let same = prev
				.layers
				.iter()
				.zip(&layers)
				.take_while(|(a, b)| a == b)
				.count();
			if unchanged > same {
				violations.push(format!(
					"layer_unchanged is {unchanged}, but layer {same} differs from the previous run"
				));
			}
			if permanent < prev.permanent {
				violations.push(format!(
					"layer_permanent went down from {} to {permanent}",
					prev.permanent
				));
			}
			if same < prev.permanent {
				violations.push(format!(
					"permanent layer {same} differs from the previous run"
				));
			}
			if let Some(r) = prev.redundant.iter().find(|r| !redundant.contains(r)) {
				violations.push(format!("layer {r} is no longer redundant"));
			}
			// Replay keeps the redundant list it has and adds to its end, so it
			// can only continue from a list that grew at the end.
			if violations.is_empty() && redundant.starts_with(&prev.redundant) {
				same
			} else {
				0
			}
		}
	};

	let in_solution = layers
		.iter()
		.flat_map(|l| &l.decisions)
		.enumerate()
		.filter(|(_, d)| d.in_solution)
		.map(|(i, _)| i)
		.collect();
	let snapshot = Snapshot {
		from_layer,
		unchanged,
		permanent,
		redundant: redundant.clone(),
		layers: layers.split_off(from_layer),
		objective,
	};
	*seen = Some(Seen {
		layers,
		permanent,
		redundant,
	});
	Taken {
		snapshot,
		in_solution,
		violations,
	}
}

impl Seen {
	/// Put back the layers [`take`] lent to the snapshot.
	pub(crate) fn restore(&mut self, layers: &mut Vec<Layer>) {
		self.layers.append(layers);
	}
}

impl<M: Model> Reader<'_, M> {
	/// Read every layer.
	///
	/// A layer end outside the valid range is a violation, and is clamped so
	/// no accessor is called with an index out of range.
	fn layers(&mut self) -> Vec<Layer> {
		let mut layers = Vec::new();
		let (mut d_start, mut c_start) = (0, 0);
		for l in 0..self.model.layer_len() {
			let d_end = self.clamp_end(
				"decision",
				l,
				self.model.decision_layer_end(l),
				d_start,
				self.decision_len,
			);
			let c_end = self.clamp_end(
				"constraint",
				l,
				self.model.constraint_layer_end(l),
				c_start,
				self.constraint_len,
			);
			layers.push(Layer {
				decisions: (d_start..d_end)
					.map(|i| self.decision(FznsoDecisionIdx(i)))
					.collect(),
				constraints: (c_start..c_end)
					.map(|i| self.constraint(FznsoConstraintIdx(i)))
					.collect(),
			});
			(d_start, c_start) = (d_end, c_end);
		}
		if d_start != self.decision_len || c_start != self.constraint_len {
			self.violations.push(format!(
				"the layers end at decision {d_start} and constraint {c_start}, but decision_len is \
				 {} and constraint_len is {}",
				self.decision_len, self.constraint_len
			));
		}
		layers
	}

	/// Check the end of layer `layer` against the end of the one below it and
	/// the total length, and clamp it into that range.
	fn clamp_end(
		&mut self,
		what: &str,
		layer: usize,
		end: usize,
		start: usize,
		len: usize,
	) -> usize {
		if (start..=len).contains(&end) {
			end
		} else {
			self.violations.push(format!(
				"{what}_layer_end({layer}) is {end}, outside {start}..={len}"
			));
			end.clamp(start, len)
		}
	}

	/// Read one decision.
	fn decision(&mut self, d: DecisionIdx) -> Decision {
		let place = || format!("decision {}", d.0);
		let model = self.model;
		Decision {
			ty: model.decision_type(d),
			domain: self.value(model.decision_domain(d), &place),
			name: model.decision_name(d).map(str::to_owned),
			defined: model.decision_defined(d),
			in_solution: model.decision_in_solution(d),
			annotations: (0..model.decision_annotation_len(d))
				.map(|i| self.annotation(model.decision_annotation(d, i), &place))
				.collect(),
		}
	}

	/// Read one constraint.
	fn constraint(&mut self, c: FznsoConstraintIdx) -> Constraint {
		let place = || format!("constraint {}", c.0);
		let model = self.model;
		let defines = model.constraint_defines(c);
		if let Some(d) = defines {
			self.check(d.into(), &place);
		}
		Constraint {
			ident: model.constraint_ident(c).to_owned(),
			arguments: (0..model.constraint_argument_len(c))
				.map(|i| self.value(model.constraint_argument(c, i), &place))
				.collect(),
			defines,
			annotations: (0..model.constraint_annotation_len(c))
				.map(|i| self.annotation(model.constraint_annotation(c, i), &place))
				.collect(),
		}
	}

	/// Copy an annotation, checking its arguments.
	fn annotation(
		&mut self,
		ann: AnnotationRef<'_>,
		place: &dyn Fn() -> String,
	) -> OwnedAnnotation {
		let arguments = ann.arguments().map(|a| self.value(a, place)).collect();
		OwnedAnnotation::new(ann.ident(), arguments)
	}

	/// Copy a value, checking it first.
	fn value(&mut self, value: fznso::Value<'_>, place: &dyn Fn() -> String) -> OwnedValue {
		self.check(value, place);
		value.into()
	}

	/// Record the first rule `value` breaks, if any, found in `place`.
	fn check(&mut self, value: fznso::Value<'_>, place: &dyn Fn() -> String) {
		if let Some(problem) = value_problem(value, self.decision_len, self.constraint_len) {
			self.violations.push(format!("{}: {problem}", place()));
		}
	}
}

/// The first rule `value` breaks, if any: an index out of range, or a set
/// whose ranges are empty, out of order or overlapping.
fn value_problem(
	value: fznso::Value<'_>,
	decision_len: usize,
	constraint_len: usize,
) -> Option<String> {
	/// Check the ranges of a set.
	fn ranges_problem<T: Copy + PartialOrd + std::fmt::Debug>(
		ranges: impl Iterator<Item = (T, T)>,
	) -> Option<String> {
		let mut prev: Option<(T, T)> = None;
		for r in ranges {
			if r.0 > r.1 {
				return Some(format!("the set has an empty range {r:?}"));
			}
			if let Some(p) = prev
				&& p.1 >= r.0
			{
				return Some(format!(
					"the set ranges {p:?} and {r:?} overlap or are out of order"
				));
			}
			prev = Some(r);
		}
		None
	}
	match value.view() {
		ValueView::Decision(d) if d.0 >= decision_len => Some(format!(
			"decision {} does not exist (decision_len is {decision_len})",
			d.0
		)),
		ValueView::Constraint(c) if c.0 >= constraint_len => Some(format!(
			"constraint {} does not exist (constraint_len is {constraint_len})",
			c.0
		)),
		ValueView::IntSet(ranges) => ranges_problem(ranges.iter()),
		ValueView::FloatSet(ranges) => ranges_problem(ranges.iter()),
		ValueView::List(items) => items
			.iter()
			.find_map(|v| value_problem(v, decision_len, constraint_len)),
		_ => None,
	}
}
