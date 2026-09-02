//! The values that cross the interface, and the Python objects that spell them.
//!
//! A value is *borrowed* for a stated window, never owned: the vtable at the
//! bottom of this module answers the solver's queries straight out of whatever
//! Python object the model already holds.

use std::ffi::c_char;

use fznso_core::{
	OwnedValue, Value, ValueExt as _,
	fznso_types::{
		FznsoConstraintIdx, FznsoDecisionIdx, FznsoFloatRange, FznsoIntRange, FznsoPtr, FznsoType,
		FznsoTypeBase, FznsoValue, FznsoValueKind, FznsoValueMethods, FznsoValueRef,
	},
};
use pyo3::{
	exceptions::PyValueError,
	ffi,
	prelude::*,
	types::{PyBool, PyFloat, PyInt, PyList, PyString},
};

/// A reference to a decision variable, returned by `LayeredModel.add_decision`.
#[pyclass(frozen, eq, hash, module = "fznso")]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Decision {
	pub(crate) index: usize,
}

#[pymethods]
impl Decision {
	#[new]
	fn new(index: usize) -> Self {
		Decision { index }
	}
	fn __index__(&self) -> usize {
		self.index
	}
	fn __repr__(&self) -> String {
		format!("Decision({})", self.index)
	}
}

/// A reference to a constraint, returned by `LayeredModel.add_constraint`.
#[pyclass(frozen, eq, hash, module = "fznso")]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Constraint {
	pub(crate) index: usize,
}

#[pymethods]
impl Constraint {
	#[new]
	fn new(index: usize) -> Self {
		Constraint { index }
	}
	fn __index__(&self) -> usize {
		self.index
	}
	fn __repr__(&self) -> String {
		format!("Constraint({})", self.index)
	}
}

/// An integer set, an ordered union of inclusive `[min, max]` ranges.
#[pyclass(frozen, module = "fznso")]
#[derive(Clone)]
pub(crate) struct IntSet {
	ranges: Vec<(i64, i64)>,
}

/// A float set, an ordered union of inclusive `[min, max]` ranges.
#[pyclass(frozen, module = "fznso")]
#[derive(Clone)]
pub(crate) struct FloatSet {
	ranges: Vec<(f64, f64)>,
}

/// A value type: a base type plus the qualifiers that apply to it.
///
/// Any type the interface can carry is expressible. Name the qualifiers you
/// want as keyword arguments:
///
/// ```python
/// fznso.Type("int", decision=True)                # var int
/// fznso.Type("int", decision=True, set_of=True)   # var set of int
/// fznso.Type("bool", list_of=True)                # list of bool
/// ```
///
/// The Rust and C++ bindings spell this as a builder, because neither has
/// keyword arguments. Each qualifier is readable back under the name it was
/// given, so a type can be taken apart and rebuilt.
#[pyclass(frozen, eq, hash, module = "fznso")]
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct Type {
	pub(crate) ty: FznsoType,
}

#[pymethods]
impl Type {
	/// A type of the given base, carrying only the qualifiers named.
	///
	/// The qualifiers are keyword-only: a bare `True` at a call site would say
	/// nothing about which one it sets.
	#[new]
	#[pyo3(signature = (base, *, list_of=false, decision=false, set_of=false, opt=false))]
	fn new(base: &str, list_of: bool, decision: bool, set_of: bool, opt: bool) -> PyResult<Self> {
		let base = match base {
			"bool" => FznsoTypeBase::FznsoTypeBaseBool,
			"int" => FznsoTypeBase::FznsoTypeBaseInt,
			"float" => FznsoTypeBase::FznsoTypeBaseFloat,
			"string" => FznsoTypeBase::FznsoTypeBaseString,
			other => {
				return Err(PyValueError::new_err(format!(
					"unknown base type `{other}` (expected bool, int, float or string)"
				)));
			}
		};
		Ok(Self {
			ty: FznsoType::new(base)
				.list(list_of)
				.decision(decision)
				.set(set_of)
				.opt(opt),
		})
	}

	/// The base type: `"bool"`, `"int"`, `"float"` or `"string"`.
	#[getter]
	fn base(&self) -> &'static str {
		match self.ty.base {
			FznsoTypeBase::FznsoTypeBaseBool => "bool",
			FznsoTypeBase::FznsoTypeBaseInt => "int",
			FznsoTypeBase::FznsoTypeBaseFloat => "float",
			FznsoTypeBase::FznsoTypeBaseString => "string",
		}
	}
	/// Whether this type is a list of its element type.
	#[getter]
	fn list_of(&self) -> bool {
		self.ty.list_of
	}
	/// Whether this type is a decision variable rather than a parameter.
	#[getter]
	fn decision(&self) -> bool {
		self.ty.decision
	}
	/// Whether this type is over sets of its base type.
	#[getter]
	fn set_of(&self) -> bool {
		self.ty.set_of
	}
	/// Whether this type also permits the absent value.
	#[getter]
	fn opt(&self) -> bool {
		self.ty.opt
	}

	/// The type the way the protocol spells one, e.g. `var set of int`.
	fn __str__(&self) -> String {
		self.ty.to_string()
	}

	/// The expression that builds this type, so a type round-trips through
	/// `eval`.
	pub(crate) fn __repr__(&self) -> String {
		let mut out = format!("Type({:?}", self.base());
		for (name, set) in [
			("list_of", self.ty.list_of),
			("decision", self.ty.decision),
			("set_of", self.ty.set_of),
			("opt", self.ty.opt),
		] {
			if set {
				out += &format!(", {name}=True");
			}
		}
		out + ")"
	}
}

/// Convert a Python object into an owned model value.
///
/// `bool` is checked before `int`, since Python's `bool` is a subclass of
/// `int`. A `range` becomes an inclusive integer set; a `list` a value list.
pub(crate) fn py_to_value(obj: &Bound<'_, PyAny>) -> PyResult<OwnedValue> {
	if obj.is_none() {
		return Ok(OwnedValue::Absent);
	}
	if let Ok(d) = obj.extract::<Decision>() {
		return Ok(OwnedValue::Decision(FznsoDecisionIdx(d.index)));
	}
	if let Ok(c) = obj.extract::<Constraint>() {
		return Ok(OwnedValue::Constraint(FznsoConstraintIdx(c.index)));
	}
	if let Ok(set) = obj.extract::<IntSet>() {
		return Ok(OwnedValue::IntSet(
			set.ranges.iter().map(|&(a, b)| a..=b).collect(),
		));
	}
	if let Ok(set) = obj.extract::<FloatSet>() {
		return Ok(OwnedValue::FloatSet(
			set.ranges.iter().map(|&(a, b)| a..=b).collect(),
		));
	}
	if obj.is_instance_of::<PyBool>() {
		return Ok(OwnedValue::Bool(obj.extract::<bool>()?));
	}
	// A unit-step `range` becomes an inclusive integer set.
	if let Ok((start, stop, step)) = range_bounds(obj) {
		if step != 1 {
			return Err(PyValueError::new_err("only unit-step ranges are supported"));
		}
		return Ok(OwnedValue::IntSet(
			std::iter::once(start..=stop - 1).collect(),
		));
	}
	if obj.is_instance_of::<PyInt>() {
		return Ok(OwnedValue::Int(obj.extract::<i64>()?));
	}
	if obj.is_instance_of::<PyFloat>() {
		return Ok(OwnedValue::Float(obj.extract::<f64>()?));
	}
	if obj.is_instance_of::<PyString>() {
		return Ok(OwnedValue::String(obj.extract::<String>()?));
	}
	if let Ok(list) = obj.downcast::<PyList>() {
		let mut items = Vec::with_capacity(list.len());
		for item in list.iter() {
			items.push(py_to_value(&item)?);
		}
		return Ok(OwnedValue::List(items));
	}
	Err(PyValueError::new_err(format!(
		"cannot use {} as a model value",
		obj.get_type().name()?
	)))
}

/// Extract `(start, stop, step)` from a Python `range`, or `Err` otherwise.
fn range_bounds(obj: &Bound<'_, PyAny>) -> PyResult<(i64, i64, i64)> {
	if obj.get_type().name()? != "range" {
		return Err(PyValueError::new_err("not a range"));
	}
	Ok((
		obj.getattr("start")?.extract()?,
		obj.getattr("stop")?.extract()?,
		obj.getattr("step")?.extract()?,
	))
}

/// Convert a borrowed value into a Python object.
///
/// Sets become lists of `(min, max)` tuples; lists become Python lists.
pub(crate) fn value_to_py(py: Python<'_>, value: &Value<'_>) -> PyResult<PyObject> {
	use fznso_core::ValueView;
	Ok(match value.view() {
		ValueView::Absent => py.None(),
		ValueView::Bool(b) => b.into_pyobject(py)?.to_owned().unbind().into_any(),
		ValueView::Int(i) => i.into_pyobject(py)?.unbind().into_any(),
		ValueView::Float(f) => f.into_pyobject(py)?.unbind().into_any(),
		ValueView::Str(s) => s.into_pyobject(py)?.unbind().into_any(),
		ValueView::Decision(d) => Decision { index: d.0 }
			.into_pyobject(py)?
			.unbind()
			.into_any(),
		ValueView::Constraint(c) => Constraint { index: c.0 }
			.into_pyobject(py)?
			.unbind()
			.into_any(),
		ValueView::IntSet(set) => PyList::new(py, set.iter().collect::<Vec<_>>())?
			.unbind()
			.into_any(),
		ValueView::FloatSet(set) => PyList::new(py, set.iter().collect::<Vec<_>>())?
			.unbind()
			.into_any(),
		ValueView::List(list) => {
			let items = PyList::empty(py);
			for elem in list.iter() {
				items.append(value_to_py(py, &elem)?)?;
			}
			items.unbind().into_any()
		}
	})
}

/// Recover the Python object a value handle points at.
fn value_object<'py>(py: Python<'py>, data: FznsoPtr<'_, FznsoValue>) -> Bound<'py, PyAny> {
	// SAFETY: `data` holds the pointer of an object the model still owns, or of
	// the immortal `None` singleton.
	unsafe { Bound::from_borrowed_ptr(py, data.addr() as *mut ffi::PyObject) }
}

/// Which FZnSO kind a Python object presents as. Mirrors [`py_to_value`]; an
/// unsupported object is reported as absent rather than raising here.
fn classify(obj: &Bound<'_, PyAny>) -> FznsoValueKind {
	use FznsoValueKind as K;
	if obj.is_none() {
		K::FznsoValueAbsent
	} else if obj.is_instance_of::<Decision>() {
		K::FznsoValueDecision
	} else if obj.is_instance_of::<Constraint>() {
		K::FznsoValueConstraint
	} else if obj.is_instance_of::<IntSet>() {
		K::FznsoValueSetInt
	} else if obj.is_instance_of::<FloatSet>() {
		K::FznsoValueSetFloat
	} else if obj.is_instance_of::<PyBool>() {
		K::FznsoValueBool
	} else if range_bounds(obj).is_ok() {
		K::FznsoValueSetInt
	} else if obj.is_instance_of::<PyInt>() {
		K::FznsoValueInt
	} else if obj.is_instance_of::<PyFloat>() {
		K::FznsoValueFloat
	} else if obj.is_instance_of::<PyString>() {
		K::FznsoValueString
	} else if obj.is_instance_of::<PyList>() {
		K::FznsoValueList
	} else {
		K::FznsoValueAbsent
	}
}

/// Wrap a borrowed Python object pointer as an FZnSO value. The caller is
/// responsible for keeping the object alive for as long as the value is used.
pub(crate) fn native_value_ref<'a>(ptr: *mut ffi::PyObject) -> FznsoValueRef<'a> {
	FznsoValueRef {
		data: FznsoPtr::from_addr(ptr as usize),
		methods: &PY_VALUE_METHODS,
	}
}

extern "C" fn py_kind(data: FznsoPtr<'_, FznsoValue>) -> FznsoValueKind {
	Python::with_gil(|py| classify(&value_object(py, data)))
}

extern "C" fn py_len(data: FznsoPtr<'_, FznsoValue>) -> usize {
	Python::with_gil(|py| {
		let obj = value_object(py, data);
		if let Ok(s) = obj.downcast::<PyString>() {
			s.to_str().map_or(0, str::len)
		} else if let Ok(list) = obj.downcast::<PyList>() {
			list.len()
		} else if let Ok(set) = obj.extract::<IntSet>() {
			set.ranges.len()
		} else if let Ok(set) = obj.extract::<FloatSet>() {
			set.ranges.len()
		} else if range_bounds(&obj).is_ok() {
			1
		} else {
			0
		}
	})
}

extern "C" fn py_as_bool(data: FznsoPtr<'_, FznsoValue>) -> bool {
	Python::with_gil(|py| value_object(py, data).extract::<bool>().unwrap_or(false))
}

extern "C" fn py_as_int(data: FznsoPtr<'_, FznsoValue>) -> i64 {
	Python::with_gil(|py| value_object(py, data).extract::<i64>().unwrap_or(0))
}

extern "C" fn py_as_float(data: FznsoPtr<'_, FznsoValue>) -> f64 {
	Python::with_gil(|py| value_object(py, data).extract::<f64>().unwrap_or(0.0))
}

extern "C" fn py_as_string(data: FznsoPtr<'_, FznsoValue>) -> *const c_char {
	Python::with_gil(|py| {
		value_object(py, data)
			.downcast::<PyString>()
			.ok()
			.and_then(|s| s.to_str().ok())
			.map_or(std::ptr::null(), |s| s.as_ptr().cast::<c_char>())
	})
}

extern "C" fn py_as_decision(data: FznsoPtr<'_, FznsoValue>) -> FznsoDecisionIdx {
	Python::with_gil(|py| {
		value_object(py, data)
			.extract::<Decision>()
			.map_or(FznsoDecisionIdx(0), |d| FznsoDecisionIdx(d.index))
	})
}

extern "C" fn py_as_constraint(data: FznsoPtr<'_, FznsoValue>) -> FznsoConstraintIdx {
	Python::with_gil(|py| {
		value_object(py, data)
			.extract::<Constraint>()
			.map_or(FznsoConstraintIdx(0), |c| FznsoConstraintIdx(c.index))
	})
}

extern "C" fn py_get_range_int(data: FznsoPtr<'_, FznsoValue>, index: usize) -> FznsoIntRange {
	Python::with_gil(|py| {
		let obj = value_object(py, data);
		if let Ok(set) = obj.extract::<IntSet>() {
			let (min, max) = set.ranges.get(index).copied().unwrap_or((0, 0));
			FznsoIntRange { min, max }
		} else if let Ok((start, stop, _)) = range_bounds(&obj) {
			FznsoIntRange {
				min: start,
				max: stop - 1,
			}
		} else {
			FznsoIntRange { min: 0, max: 0 }
		}
	})
}

extern "C" fn py_get_range_float(data: FznsoPtr<'_, FznsoValue>, index: usize) -> FznsoFloatRange {
	Python::with_gil(|py| {
		let (min, max) = value_object(py, data)
			.extract::<FloatSet>()
			.ok()
			.and_then(|set| set.ranges.get(index).copied())
			.unwrap_or((0.0, 0.0));
		FznsoFloatRange { min, max }
	})
}

extern "C" fn py_get_element(data: FznsoPtr<'_, FznsoValue>, index: usize) -> FznsoValueRef<'_> {
	Python::with_gil(|py| {
		let ptr = value_object(py, data)
			.downcast::<PyList>()
			.ok()
			.and_then(|list| list.get_item(index).ok())
			// The element is owned by the model's list, so its pointer stays valid
			// without a reference of our own.
			.map_or_else(|| unsafe { ffi::Py_None() }, |elem| elem.as_ptr());
		native_value_ref(ptr)
	})
}

/// The value vtable backing every [`native_value_ref`].
static PY_VALUE_METHODS: FznsoValueMethods = FznsoValueMethods {
	kind: py_kind,
	len: py_len,
	as_decision: py_as_decision,
	as_constraint: py_as_constraint,
	as_int: py_as_int,
	as_float: py_as_float,
	as_string: py_as_string,
	as_bool: py_as_bool,
	get_range_int: py_get_range_int,
	get_range_float: py_get_range_float,
	get_element: py_get_element,
};

/// An integer set covering the inclusive range `[min, max]`.
#[pyfunction]
pub(crate) fn int_set(min: i64, max: i64) -> IntSet {
	IntSet {
		ranges: vec![(min, max)],
	}
}

/// A float set covering the inclusive range `[min, max]`.
#[pyfunction]
pub(crate) fn float_set(min: f64, max: f64) -> FloatSet {
	FloatSet {
		ranges: vec![(min, max)],
	}
}
