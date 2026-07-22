//! A total, allocation-free view over a [`Value`].
//!
//! The accessors on [`Value`] mirror the ABI: each one is only valid for a
//! particular value kind and panics otherwise. [`ValueView`] turns that into an
//! ordinary `match`, so the kind is checked once and the payload comes back
//! already typed.

use crate::{ConstraintIdx, DecisionIdx, Value, fznso_types::FznsoValueKind};

/// The ranges of a float set, borrowed from the value that produced them.
#[derive(Clone, Copy, Debug)]
pub struct FloatRanges<'a>(Value<'a>);

/// The ranges of an integer set, borrowed from the value that produced them.
#[derive(Clone, Copy, Debug)]
pub struct IntRanges<'a>(Value<'a>);

/// Adds [`view`](ValueExt::view) to [`Value`].
///
/// [`Value`] is an alias for the ABI's own handle type, so this cannot be an
/// inherent method. Import the trait to use it.
pub trait ValueExt<'a> {
	/// Resolve this value into its payload.
	///
	/// See [`ValueView`] for the full list of shapes and an example.
	fn view(&self) -> ValueView<'a>;
}

/// The elements of a list value, borrowed from the value that produced them.
#[derive(Clone, Copy, Debug)]
pub struct ValueList<'a>(Value<'a>);

/// A [`Value`] resolved into its payload.
///
/// Obtained from [`ValueExt::view`]. Matching on this is the safe alternative
/// to checking the value's kind and then calling the matching `get_*`
/// accessor, which panics if the two disagree.
///
/// Sets and lists keep borrowing the original value rather than being
/// collected, so viewing a value never allocates.
///
/// ```rust
/// use fznso::{Value, ValueExt, ValueView};
///
/// fn describe(value: &Value<'_>) -> String {
///     match value.view() {
///         ValueView::Absent => "absent".to_owned(),
///         ValueView::Bool(b) => format!("{b}"),
///         ValueView::Int(i) => format!("{i}"),
///         ValueView::Float(f) => format!("{f}"),
///         ValueView::Str(s) => s.to_owned(),
///         ValueView::Decision(d) => format!("x{}", d.0),
///         ValueView::Constraint(c) => format!("c{}", c.0),
///         ValueView::IntSet(set) => format!("{:?}", set.iter().collect::<Vec<_>>()),
///         ValueView::FloatSet(set) => format!("{:?}", set.iter().collect::<Vec<_>>()),
///         ValueView::List(list) => format!("{} elements", list.len()),
///     }
/// }
///
/// assert_eq!(describe(&(&42_i64).into()), "42");
/// assert_eq!(describe(&(&()).into()), "absent");
/// ```
#[derive(Clone, Copy, Debug)]
pub enum ValueView<'a> {
	/// No value is present.
	Absent,
	/// A Boolean.
	Bool(bool),
	/// A reference to a constraint by index.
	Constraint(ConstraintIdx),
	/// A reference to a decision variable by index.
	Decision(DecisionIdx),
	/// A 64-bit float.
	Float(f64),
	/// A set of floats, as an ordered list of inclusive ranges.
	FloatSet(FloatRanges<'a>),
	/// A 64-bit signed integer.
	Int(i64),
	/// A set of integers, as an ordered list of inclusive ranges.
	IntSet(IntRanges<'a>),
	/// An ordered list of values.
	List(ValueList<'a>),
	/// A UTF-8 string.
	Str(&'a str),
}

impl<'a> FloatRanges<'a> {
	/// The inclusive `(min, max)` range at `index`.
	///
	/// # Panics
	///
	/// Panics if `index` is not less than [`len`](Self::len).
	pub fn get(&self, index: usize) -> (f64, f64) {
		assert!(index < self.len(), "range index {index} out of bounds");
		self.0.get_range_float(index)
	}

	/// Whether the set is empty.
	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}

	/// Iterator over the inclusive `(min, max)` ranges, in order.
	pub fn iter(&self) -> impl Iterator<Item = (f64, f64)> + 'a {
		let value = self.0;
		(0..value.len()).map(move |i| value.get_range_float(i))
	}

	/// The number of ranges in the set.
	pub fn len(&self) -> usize {
		self.0.len()
	}
}

impl<'a> IntRanges<'a> {
	/// The inclusive `(min, max)` range at `index`.
	///
	/// # Panics
	///
	/// Panics if `index` is not less than [`len`](Self::len).
	pub fn get(&self, index: usize) -> (i64, i64) {
		assert!(index < self.len(), "range index {index} out of bounds");
		self.0.get_range_int(index)
	}

	/// Whether the set is empty.
	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}

	/// Iterator over the inclusive `(min, max)` ranges, in order.
	pub fn iter(&self) -> impl Iterator<Item = (i64, i64)> + 'a {
		let value = self.0;
		(0..value.len()).map(move |i| value.get_range_int(i))
	}

	/// The number of ranges in the set.
	pub fn len(&self) -> usize {
		self.0.len()
	}
}

impl<'a> ValueExt<'a> for Value<'a> {
	fn view(&self) -> ValueView<'a> {
		match self.kind() {
			FznsoValueKind::FznsoValueAbsent => ValueView::Absent,
			FznsoValueKind::FznsoValueBool => ValueView::Bool(self.get_bool()),
			FznsoValueKind::FznsoValueConstraint => ValueView::Constraint(self.get_constraint()),
			FznsoValueKind::FznsoValueDecision => ValueView::Decision(self.get_decision()),
			FznsoValueKind::FznsoValueFloat => ValueView::Float(self.get_float()),
			FznsoValueKind::FznsoValueInt => ValueView::Int(self.get_int()),
			FznsoValueKind::FznsoValueString => ValueView::Str(self.get_string()),
			FznsoValueKind::FznsoValueSetInt => ValueView::IntSet(IntRanges(*self)),
			FznsoValueKind::FznsoValueSetFloat => ValueView::FloatSet(FloatRanges(*self)),
			FznsoValueKind::FznsoValueList => ValueView::List(ValueList(*self)),
		}
	}
}

impl<'a> ValueList<'a> {
	/// The element at `index`.
	///
	/// # Panics
	///
	/// Panics if `index` is not less than [`len`](Self::len).
	pub fn get(&self, index: usize) -> Value<'a> {
		assert!(index < self.len(), "list index {index} out of bounds");
		self.0.get_element(index)
	}

	/// Whether the list is empty.
	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}

	/// Iterator over the elements, in order.
	pub fn iter(&self) -> impl Iterator<Item = Value<'a>> + 'a {
		let value = self.0;
		(0..value.len()).map(move |i| value.get_element(i))
	}

	/// The number of elements in the list.
	pub fn len(&self) -> usize {
		self.0.len()
	}
}

#[cfg(test)]
mod tests {
	use fznso_types::{FznsoConstraintIdx, FznsoDecisionIdx};

	use crate::{
		Value,
		value::{ValueExt, ValueView},
	};

	/// Viewing a value must agree with the raw accessors for every kind.
	#[test]
	fn value_views() {
		assert!(matches!(Value::from(&()).view(), ValueView::Absent));
		assert!(matches!(Value::from(&1.5_f64).view(), ValueView::Float(f) if f == 1.5));
		assert!(matches!(Value::from(&7_i64).view(), ValueView::Int(7)));
		assert!(matches!(Value::from(true).view(), ValueView::Bool(true)));
		assert!(matches!(
			Value::from(FznsoDecisionIdx(4)).view(),
			ValueView::Decision(FznsoDecisionIdx(4))
		));
		assert!(matches!(
			Value::from(FznsoConstraintIdx(9)).view(),
			ValueView::Constraint(FznsoConstraintIdx(9))
		));

		let text = String::from("hello");
		assert!(matches!(Value::from(&text).view(), ValueView::Str("hello")));

		let items = vec![1_i64, 2, 3];
		let ValueView::List(list) = Value::from(&items).view() else {
			panic!("expected a list");
		};
		assert_eq!(list.len(), 3);
		assert!(!list.is_empty());
		assert_eq!(list.get(1).get_int(), 2);
		assert_eq!(
			list.iter().map(|v| v.get_int()).collect::<Vec<_>>(),
			vec![1, 2, 3]
		);
	}
}
