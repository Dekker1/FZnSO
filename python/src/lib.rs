//! Python bindings for FZnSO, built on the `fznso` crate.
//!
//! Python is a *consumer*: it loads a solver library, builds models and runs
//! them. Solvers are not written in Python.

use std::{ffi::c_char, sync::Arc};

use fznso_core::{
	AnnotationRef, ConstraintIdx, DecisionIdx, OwnedValue, Solver as _, Status as CoreStatus,
	Value, ValueExt as _,
	fznso_types::{
		FznsoConstraintIdx, FznsoDecisionIdx, FznsoFloatRange, FznsoIntRange, FznsoPtr, FznsoType,
		FznsoTypeBase, FznsoValue, FznsoValueKind, FznsoValueMethods, FznsoValueRef,
	},
};
use pyo3::{
	exceptions::{PyNotImplementedError, PyRuntimeError, PyValueError},
	ffi,
	prelude::*,
	types::{PyBool, PyDict, PyFloat, PyInt, PyList, PyString, PyTuple},
};

/// A reference to a decision variable, returned by `LayeredModel.add_decision`.
#[pyclass(frozen, eq, hash, module = "fznso")]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Decision {
	index: usize,
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
struct Constraint {
	index: usize,
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
struct IntSet {
	ranges: Vec<(i64, i64)>,
}

/// A float set, an ordered union of inclusive `[min, max]` ranges.
#[pyclass(frozen, module = "fznso")]
#[derive(Clone)]
struct FloatSet {
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
struct Type {
	ty: FznsoType,
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
	fn __repr__(&self) -> String {
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
fn py_to_value(obj: &Bound<'_, PyAny>) -> PyResult<OwnedValue> {
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
fn value_to_py(py: Python<'_>, value: &Value<'_>) -> PyResult<PyObject> {
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

/// Base class for a model implemented in Python.
///
/// Subclass it and override the methods your model needs; the rest describe an
/// empty model. Values are ordinary Python objects (a `range` or `int_set` for
/// a domain, an `int`, a `Decision`, a list, and so on).
///
/// The model is read on demand while the solver runs, and the values and
/// strings it returns are *borrowed, not copied*. The model must therefore keep
/// them alive for the whole run: return objects it owns — for example stored on
/// the instance — rather than freshly built temporaries.
///
/// By default a custom model is a single permanent layer. Override the
/// `layer_*` methods to expose several layers for incremental solving, or use
/// [`LayeredModel`] to build one directly.
#[pyclass(subclass, module = "fznso")]
struct Model;

#[pymethods]
impl Model {
	#[new]
	#[pyo3(signature = (*_args, **_kwargs))]
	fn new(_args: &Bound<'_, PyTuple>, _kwargs: Option<&Bound<'_, PyDict>>) -> Self {
		Model
	}

	/// The number of decision variables.
	fn decision_count(&self) -> usize {
		0
	}
	/// The type of the decision at `index`.
	///
	/// Unlike the other accessors this has no sensible default: a decision's
	/// type is exactly what cannot be worked out from anything else it reports,
	/// so a model that declares decisions has to say what they are.
	fn decision_type(&self, index: usize) -> PyResult<Type> {
		let _ = index;
		Err(PyNotImplementedError::new_err(
			"decision_type must be implemented by a model that has decisions: \
			 a decision's type cannot be inferred from its domain",
		))
	}
	/// The domain of the decision at `index`, or `None` for no explicit domain.
	fn decision_domain(&self, index: usize) -> Option<PyObject> {
		let _ = index;
		None
	}
	/// The name of the decision at `index`, if any.
	fn decision_name(&self, index: usize) -> Option<String> {
		let _ = index;
		None
	}
	/// Whether the decision at `index` is defined by a constraint.
	fn decision_defined(&self, index: usize) -> bool {
		let _ = index;
		false
	}
	/// Whether a solution may be asked for the value of the decision at
	/// `index`. A model that does not say assumes every decision may be, which
	/// is the safe answer: a solver then assigns them all.
	fn decision_in_solution(&self, index: usize) -> bool {
		let _ = index;
		true
	}
	/// The number of constraints.
	fn constraint_count(&self) -> usize {
		0
	}
	/// The identifier of the constraint at `index`.
	fn constraint_ident(&self, index: usize) -> PyResult<String> {
		let _ = index;
		Err(PyRuntimeError::new_err("constraint_ident not implemented"))
	}
	/// The arguments of the constraint at `index`, as a list of values.
	fn constraint_arguments(&self, index: usize) -> Vec<PyObject> {
		let _ = index;
		Vec::new()
	}
	/// The decision the constraint at `index` defines, if any.
	fn constraint_defines(&self, index: usize) -> Option<Decision> {
		let _ = index;
		None
	}
	/// The objective identifier, or `""` for a satisfaction problem.
	fn objective_ident(&self) -> String {
		String::new()
	}
	/// The objective's argument, or `None`.
	fn objective_argument(&self) -> Option<PyObject> {
		None
	}

	/// The number of layers in the model. Defaults to a single layer.
	fn layer_count(&self) -> usize {
		1
	}
	/// The number of permanently committed layers (the solver will not pop
	/// these). Defaults to all layers being permanent.
	fn layer_permanent(slf: &Bound<'_, Self>) -> PyResult<usize> {
		slf.call_method0("layer_count")?.extract()
	}
	/// The number of trailing layers unchanged since the last run, for
	/// incremental reuse. Defaults to none.
	fn layer_unchanged(&self) -> usize {
		0
	}
	/// The layer indices of permanent layers that are redundant. Defaults to
	/// none.
	fn layer_redundant(&self) -> Vec<usize> {
		Vec::new()
	}
	/// One past the global index of the last decision in layers `0..=layer`.
	/// Defaults to every decision living in a single layer.
	fn decision_layer_end(slf: &Bound<'_, Self>, layer: usize) -> PyResult<usize> {
		let _ = layer;
		slf.call_method0("decision_count")?.extract()
	}
	/// One past the global index of the last constraint in layers `0..=layer`.
	/// Defaults to every constraint living in a single layer.
	fn constraint_layer_end(slf: &Bound<'_, Self>, layer: usize) -> PyResult<usize> {
		let _ = layer;
		slf.call_method0("constraint_count")?.extract()
	}
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
fn native_value_ref<'a>(ptr: *mut ffi::PyObject) -> FznsoValueRef<'a> {
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

/// Presents a Python object implementing the [`Model`] protocol to the solver,
/// calling back into Python on demand.
struct PyModelAdapter {
	object: PyObject,
	/// The first error raised by a Python method, re-raised after the run.
	///
	/// A `Mutex` rather than a `RefCell`: the model is a read-only view the
	/// solver may query from several threads at once, so it must be `Sync`.
	error: std::sync::Mutex<Option<PyErr>>,
}

impl PyModelAdapter {
	fn new(object: PyObject) -> Self {
		Self {
			object,
			error: std::sync::Mutex::new(None),
		}
	}

	/// Record a Python error to be re-raised once the run finishes.
	fn record(&self, err: PyErr) {
		let mut slot = self.error.lock().unwrap_or_else(|e| e.into_inner());
		if slot.is_none() {
			*slot = Some(err);
		}
	}

	/// Call a `-> usize` Python method, defaulting to `default` on error.
	fn call_usize(&self, method: &str, arg: Option<usize>, default: usize) -> usize {
		Python::with_gil(|py| {
			let result = match arg {
				Some(a) => self.object.call_method1(py, method, (a,)),
				None => self.object.call_method0(py, method),
			};
			result
				.and_then(|r| r.extract::<usize>(py))
				.unwrap_or_else(|err| {
					self.record(err);
					default
				})
		})
	}

	/// Call a Python method returning a value object, defaulting to `None` on
	/// error.
	fn call_object(&self, method: &str, index: Option<usize>) -> Py<PyAny> {
		Python::with_gil(|py| {
			let result = match index {
				Some(i) => self.object.call_method1(py, method, (i,)),
				None => self.object.call_method0(py, method),
			};
			result.unwrap_or_else(|err| {
				self.record(err);
				py.None()
			})
		})
	}

	/// Present a Python object as a value borrowing it. The model must own the
	/// object so it outlives the value; the interface never copies or pins it.
	fn borrow_value(&self, obj: Py<PyAny>) -> Value<'_> {
		// The model keeps its own reference to `obj`, so releasing ours (as it
		// drops here) leaves the value pointing at a live object.
		native_value_ref(obj.as_ptr())
	}

	/// Borrow a Python object's text as a `&str` valid for `&self`. The model
	/// must own the object so its buffer outlives the borrow. A non-string
	/// (including `None`) borrows as empty.
	fn borrow_str(&self, obj: Py<PyAny>) -> &str {
		let (ptr, len) = Python::with_gil(|py| {
			let text = obj
				.bind(py)
				.downcast::<PyString>()
				.ok()
				.and_then(|s| s.to_str().ok())
				.unwrap_or("");
			(text.as_ptr(), text.len())
		});
		// SAFETY: the bytes live in the object's cached UTF-8 buffer, which the
		// model keeps alive for as long as it owns the object.
		unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr, len)) }
	}

	/// The indices of redundant permanent layers, defaulting to none on error.
	fn layer_redundant(&self) -> Vec<usize> {
		Python::with_gil(|py| {
			self.object
				.call_method0(py, "layer_redundant")
				.and_then(|r| r.extract::<Vec<usize>>(py))
				.unwrap_or_else(|err| {
					self.record(err);
					Vec::new()
				})
		})
	}
}

impl fznso_core::Model for PyModelAdapter {
	fn decision_len(&self) -> usize {
		self.call_usize("decision_count", None, 0)
	}
	fn decision_type(&self, decision: DecisionIdx) -> FznsoType {
		// A model that cannot answer records the exception, which is raised once
		// the run unwinds; the type returned here only keeps the traversal going
		// until then.
		Python::with_gil(|py| {
			self.object
				.call_method1(py, "decision_type", (decision.0,))
				.and_then(|r| r.extract::<Type>(py))
				.map(|t| t.ty)
				.unwrap_or_else(|err| {
					self.record(err);
					FznsoType::new(FznsoTypeBase::FznsoTypeBaseInt).decision(true)
				})
		})
	}
	fn decision_domain(&self, decision: DecisionIdx) -> Value<'_> {
		self.borrow_value(self.call_object("decision_domain", Some(decision.0)))
	}
	fn decision_name(&self, decision: DecisionIdx) -> Option<&str> {
		let name = Python::with_gil(|py| {
			match self.object.call_method1(py, "decision_name", (decision.0,)) {
				Ok(obj) if obj.is_none(py) => None,
				Ok(obj) => Some(obj),
				Err(err) => {
					self.record(err);
					None
				}
			}
		});
		name.map(|obj| self.borrow_str(obj))
	}
	fn decision_defined(&self, decision: DecisionIdx) -> bool {
		Python::with_gil(|py| {
			self.object
				.call_method1(py, "decision_defined", (decision.0,))
				.and_then(|r| r.extract::<bool>(py))
				.unwrap_or_else(|err| {
					self.record(err);
					false
				})
		})
	}

	fn decision_in_solution(&self, decision: DecisionIdx) -> bool {
		Python::with_gil(|py| {
			self.object
				.call_method1(py, "decision_in_solution", (decision.0,))
				.and_then(|r| r.extract::<bool>(py))
				.unwrap_or_else(|err| {
					self.record(err);
					true
				})
		})
	}

	fn constraint_len(&self) -> usize {
		self.call_usize("constraint_count", None, 0)
	}
	fn constraint_ident(&self, con: ConstraintIdx) -> &str {
		self.borrow_str(self.call_object("constraint_ident", Some(con.0)))
	}
	fn constraint_argument_len(&self, con: ConstraintIdx) -> usize {
		Python::with_gil(|py| {
			self.object
				.call_method1(py, "constraint_arguments", (con.0,))
				.and_then(|r| r.bind(py).len())
				.unwrap_or_else(|err| {
					self.record(err);
					0
				})
		})
	}
	fn constraint_argument(&self, con: ConstraintIdx, index: usize) -> Value<'_> {
		let element = Python::with_gil(|py| {
			self.object
				.call_method1(py, "constraint_arguments", (con.0,))
				.and_then(|args| args.bind(py).get_item(index).map(|e| e.unbind()))
				.unwrap_or_else(|err| {
					self.record(err);
					py.None()
				})
		});
		self.borrow_value(element)
	}
	fn constraint_defines(&self, con: ConstraintIdx) -> Option<DecisionIdx> {
		Python::with_gil(|py| {
			self.object
				.call_method1(py, "constraint_defines", (con.0,))
				.and_then(|r| r.extract::<Option<Decision>>(py))
				.unwrap_or_else(|err| {
					self.record(err);
					None
				})
				.map(|d| FznsoDecisionIdx(d.index))
		})
	}

	fn objective_ident(&self) -> &str {
		self.borrow_str(self.call_object("objective_ident", None))
	}
	fn objective_arg(&self) -> Value<'_> {
		self.borrow_value(self.call_object("objective_argument", None))
	}

	// --- Layers ---
	fn layer_len(&self) -> usize {
		self.call_usize("layer_count", None, 1)
	}
	fn layer_permanent(&self) -> usize {
		self.call_usize("layer_permanent", None, 1)
	}
	fn layer_unchanged(&self) -> usize {
		self.call_usize("layer_unchanged", None, 0)
	}
	fn layer_redundant_len(&self) -> usize {
		self.layer_redundant().len()
	}
	fn layer_redundant_index(&self, index: usize) -> usize {
		self.layer_redundant().get(index).copied().unwrap_or(0)
	}
	fn decision_layer_end(&self, layer: usize) -> usize {
		self.call_usize("decision_layer_end", Some(layer), self.decision_len())
	}
	fn constraint_layer_end(&self, layer: usize) -> usize {
		self.call_usize("constraint_layer_end", Some(layer), self.constraint_len())
	}

	// --- Annotations: none from a Python model (getters never reached) ---
	fn decision_annotation_len(&self, _decision: DecisionIdx) -> usize {
		0
	}
	fn decision_annotation(&self, _decision: DecisionIdx, _index: usize) -> AnnotationRef<'_> {
		unreachable!("no decision annotations")
	}
	fn constraint_annotation_len(&self, _con: ConstraintIdx) -> usize {
		0
	}
	fn constraint_annotation(&self, _con: ConstraintIdx, _index: usize) -> AnnotationRef<'_> {
		unreachable!("no constraint annotations")
	}
	fn objective_annotation_len(&self) -> usize {
		0
	}
	fn objective_annotation(&self, _index: usize) -> AnnotationRef<'_> {
		unreachable!("no objective annotations")
	}
}

/// An integer set covering the inclusive range `[min, max]`.
#[pyfunction]
fn int_set(min: i64, max: i64) -> IntSet {
	IntSet {
		ranges: vec![(min, max)],
	}
}

/// A float set covering the inclusive range `[min, max]`.
#[pyfunction]
fn float_set(min: f64, max: f64) -> FloatSet {
	FloatSet {
		ranges: vec![(min, max)],
	}
}
/// A model to hand to a solver.
///
/// Decisions and constraints are added to the current layer. Layers can be
/// pushed, popped and committed for incremental solving.
#[pyclass(module = "fznso")]
struct LayeredModel {
	inner: fznso_core::LayeredModel,
}

#[pymethods]
impl LayeredModel {
	#[new]
	fn new() -> Self {
		Self {
			inner: fznso_core::LayeredModel::default(),
		}
	}

	/// Add a decision variable of the given type and domain, returning its
	/// handle.
	#[pyo3(signature = (ty, domain=None, name=None, defined=false, in_solution=true))]
	fn add_decision(
		&mut self,
		ty: &Type,
		domain: Option<&Bound<'_, PyAny>>,
		name: Option<String>,
		defined: bool,
		in_solution: bool,
	) -> PyResult<Decision> {
		let domain = match domain {
			Some(d) => py_to_value(d)?,
			None => OwnedValue::Absent,
		};
		let idx = self
			.inner
			.add_decision(ty.ty, domain, name, defined, in_solution, Vec::new());
		Ok(Decision { index: idx.0 })
	}

	/// Add a constraint with the given identifier and arguments.
	#[pyo3(signature = (ident, arguments, defines=None))]
	fn add_constraint(
		&mut self,
		ident: &str,
		arguments: &Bound<'_, PyList>,
		defines: Option<Decision>,
	) -> PyResult<Constraint> {
		let mut args = Vec::with_capacity(arguments.len());
		for arg in arguments.iter() {
			args.push(py_to_value(&arg)?);
		}
		let idx = self.inner.add_constraint(
			ident,
			args,
			defines.map(|d| FznsoDecisionIdx(d.index)),
			Vec::new(),
		);
		Ok(Constraint { index: idx.0 })
	}

	/// Set the objective. Pass no identifier for a satisfaction problem.
	#[pyo3(signature = (ident, argument=None))]
	fn set_objective(
		&mut self,
		ident: Option<String>,
		argument: Option<Bound<'_, PyAny>>,
	) -> PyResult<()> {
		let arg = match argument {
			Some(a) => py_to_value(&a)?,
			None => OwnedValue::Absent,
		};
		self.inner.set_objective(ident, arg, Vec::new());
		Ok(())
	}

	/// Push a new, empty layer.
	fn push_layer(&mut self) {
		self.inner.push_layer();
	}

	/// Pop the top layer.
	fn pop_layer(&mut self) {
		self.inner.pop_layer();
	}

	/// Mark every current layer as permanent.
	fn mark_permanent(&mut self) {
		self.inner.mark_permanent();
	}
}

/// A solution reported to the `on_solution` callback.
///
/// Valid only during that callback; using it afterwards raises an error.
#[pyclass(unsendable, module = "fznso")]
struct Solution {
	// A raw pointer to the borrowed solution, valid only during the callback.
	solution: *const fznso_core::DynSolution<'static>,
	live: bool,
}

impl Solution {
	fn get(&self) -> PyResult<&fznso_core::DynSolution<'static>> {
		if !self.live {
			return Err(PyRuntimeError::new_err(
				"solution used outside the on_solution callback",
			));
		}
		// SAFETY: `live` is only true while the borrowed solution is alive.
		Ok(unsafe { &*self.solution })
	}
}

#[pymethods]
impl Solution {
	/// The value assigned to a decision variable (a `Decision` or an index).
	fn value(&self, py: Python<'_>, decision: &Bound<'_, PyAny>) -> PyResult<PyObject> {
		let index = match decision.extract::<Decision>() {
			Ok(d) => d.index,
			Err(_) => decision.extract::<usize>()?,
		};
		value_to_py(py, &fznso_core::Solution::value(self.get()?, index))
	}

	/// A named statistic for this solution.
	fn statistic(&self, py: Python<'_>, name: &str) -> PyResult<PyObject> {
		value_to_py(py, &fznso_core::Solution::statistic(self.get()?, name))
	}

	fn __getitem__(&self, py: Python<'_>, decision: &Bound<'_, PyAny>) -> PyResult<PyObject> {
		self.value(py, decision)
	}
}

/// The outcome of a run.
#[pyclass(frozen, eq, module = "fznso")]
#[derive(Clone, PartialEq)]
enum Status {
	/// The solver explored the whole search space.
	Complete,
	/// The solver stopped early; better solutions may exist.
	Incomplete,
}

/// A solver instance created from a [`Library`].
#[pyclass(unsendable, module = "fznso")]
struct DynSolver {
	inner: fznso_core::DynSolver,
}

/// Carries the solver handle into [`Python::allow_threads`].
///
/// `allow_threads` requires its closure to be `Send`, which pyo3 uses as a
/// proxy for "holds no GIL-bound reference" — the closure itself runs on the
/// calling thread, so nothing is actually sent anywhere. A
/// [`fznso_core::DynSolver`] is `!Send` only because it holds the solver's raw
/// handle, which is not a GIL-bound reference, so carrying it in is sound.
struct SendSolver<'a>(&'a mut fznso_core::DynSolver);

// SAFETY: as above. The closure `allow_threads` runs stays on this thread, and
// the wrapped handle borrows nothing owned by the interpreter.
unsafe impl Send for SendSolver<'_> {}

impl<'a> SendSolver<'a> {
	/// Unwrap the handle inside the closure.
	///
	/// A method rather than a field access: `self.0` inside a closure would let
	/// edition-2021 disjoint capture grab the `!Send` reference directly and
	/// defeat the wrapper.
	fn into_inner(self) -> &'a mut fznso_core::DynSolver {
		self.0
	}
}

#[pymethods]
impl DynSolver {
	/// The current value of a named option.
	fn option_get(&self, py: Python<'_>, name: &str) -> PyResult<PyObject> {
		value_to_py(py, &self.inner.option_get(name))
	}

	/// The current value of a named solver-level statistic, or `None` if the
	/// solver does not report it.
	fn statistic(&self, py: Python<'_>, name: &str) -> PyResult<PyObject> {
		value_to_py(py, &self.inner.statistic(name))
	}

	/// Set a named option. Raises `ValueError` with the solver's message on
	/// failure.
	fn option_set(&mut self, name: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
		// `value` lives for this call and the solver reads it synchronously, so
		// borrow it directly rather than copying it into an owned value.
		self.inner
			.option_set(name, native_value_ref(value.as_ptr()))
			.map_err(PyValueError::new_err)
	}

	/// Run the solver over `model`, calling `on_solution` for each solution and
	/// `on_message` for each diagnostic. Returns the completion status, or
	/// raises `RuntimeError` if the solver reports an error.
	///
	/// `model` may be a [`LayeredModel`] built with the builder, or any object
	/// implementing the [`Model`] protocol.
	///
	/// `should_stop`, if given, is polled by the solver and ends the search
	/// when it returns a truthy value; the run then reports
	/// `Status.Incomplete`. Use it to cancel from a watchdog thread. It may be
	/// called from several solver threads at once, so keep it cheap and
	/// thread-safe.
	#[pyo3(signature = (model, on_solution=None, on_message=None, should_stop=None))]
	fn run(
		&mut self,
		// The GIL is *released* for the duration of the search, so that a solver
		// which reports from its own threads can acquire it; everything that
		// re-enters Python takes it back with `Python::with_gil`.
		py: Python<'_>,
		model: &Bound<'_, PyAny>,
		on_solution: Option<PyObject>,
		on_message: Option<PyObject>,
		should_stop: Option<PyObject>,
	) -> PyResult<Status> {
		// A callback error is stashed and re-raised once `run` returns, since the
		// solver's callbacks cannot themselves fail. A `Mutex` rather than a
		// `RefCell`: the callbacks must be `Send`, because the solver may invoke
		// them from any thread (one at a time — see `Solver::run`).
		let callback_error: std::sync::Mutex<Option<PyErr>> = std::sync::Mutex::new(None);
		let stash = |err: PyErr| {
			let mut slot = callback_error.lock().unwrap_or_else(|e| e.into_inner());
			if slot.is_none() {
				*slot = Some(err);
			}
		};
		let failed = || {
			callback_error
				.lock()
				.unwrap_or_else(|e| e.into_inner())
				.is_some()
		};

		let mut on_sol = |sol: &fznso_core::DynSolution<'_>| {
			if failed() {
				return;
			}
			let Some(handler) = &on_solution else {
				return;
			};
			// SAFETY: the wrapper is invalidated before this closure returns, so
			// its pointer never outlives `sol`.
			let widened: *const fznso_core::DynSolution<'static> =
				unsafe { std::mem::transmute(sol as *const fznso_core::DynSolution<'_>) };
			// The GIL is acquired here rather than captured: the callback may run
			// on a solver thread that does not already hold it.
			Python::with_gil(|py| {
				match Py::new(
					py,
					Solution {
						solution: widened,
						live: true,
					},
				) {
					Ok(wrapper) => {
						if let Err(err) = handler.call1(py, (&wrapper,)) {
							stash(err);
						}
						wrapper.borrow_mut(py).live = false;
					}
					Err(err) => stash(err),
				}
			});
		};

		// Built only when a handler was supplied, so a solver that asks "is anyone
		// listening?" gets a truthful no and can skip formatting diagnostics.
		let mut on_msg = on_message.map(|handler| {
			move |scope: &str, value: Value<'_>| {
				if failed() {
					return;
				}
				Python::with_gil(|py| match value_to_py(py, &value) {
					Ok(obj) => {
						if let Err(err) = handler.call1(py, (scope, obj)) {
							stash(err);
						}
					}
					Err(err) => stash(err),
				});
			}
		});

		// Polled by the solver to ask whether to keep searching. Unlike the two
		// handlers this may be called from several solver threads at once, so it is
		// `Fn` and takes the GIL per poll. A callback that has already raised also
		// answers "stop", so a Python exception ends the search promptly instead of
		// being held until the solver finishes on its own.
		let stop = || {
			if failed() {
				return true;
			}
			let Some(handler) = &should_stop else {
				return false;
			};
			Python::with_gil(|py| match handler.call0(py) {
				Ok(answer) => match answer.bind(py).is_truthy() {
					Ok(stop) => stop,
					Err(err) => {
						stash(err);
						true
					}
				},
				Err(err) => {
					stash(err);
					true
				}
			})
		};

		// The predicate is supplied whenever it could answer "stop": either the
		// caller gave one, or a callback exists that might raise and should cut the
		// search short. With neither, the solver is told nothing will cancel.
		let wants_stop = should_stop.is_some() || on_solution.is_some() || on_msg.is_some();
		let message_sink = on_msg.as_mut();
		let stop_poll = if wants_stop { Some(&stop) } else { None };

		// A native `LayeredModel` runs directly; anything else is driven through the
		// adapter, which may itself record a Python error.
		//
		// Both run inside `allow_threads`: holding the GIL across the search would
		// deadlock any solver whose worker threads call back into Python, since
		// the callbacks and the adapter both wait for it. Releasing it costs a
		// real acquire per callback instead of a cheap re-entrant one, which is
		// the price of supporting a threaded solver at all.
		let mut model_error: Option<PyErr> = None;
		let status = if let Ok(native) = model.downcast::<LayeredModel>() {
			let native = native.borrow();
			// The `PyRef` deliberately stays *outside* the closure: it holds pyo3's
			// borrow flag for the whole run, so Python code that tries to mutate the
			// model from a callback raises "already borrowed" rather than aliasing
			// it while the solver reads it.
			let model_ref: &fznso_core::LayeredModel = &native.inner;
			let solver = SendSolver(&mut self.inner);
			py.allow_threads(move || {
				solver
					.into_inner()
					.run(model_ref, &mut on_sol, message_sink, stop_poll)
			})
		} else {
			let adapter = PyModelAdapter::new(model.clone().unbind());
			let status = {
				let adapter_ref = &adapter;
				let solver = SendSolver(&mut self.inner);
				py.allow_threads(move || {
					solver
						.into_inner()
						.run(adapter_ref, &mut on_sol, message_sink, stop_poll)
				})
			};
			model_error = adapter
				.error
				.into_inner()
				.unwrap_or_else(|e| e.into_inner());
			status
		};

		if let Some(err) = callback_error
			.into_inner()
			.unwrap_or_else(|e| e.into_inner())
		{
			return Err(err);
		}
		if let Some(err) = model_error {
			return Err(err);
		}
		match status {
			CoreStatus::Complete => Ok(Status::Complete),
			CoreStatus::Incomplete => Ok(Status::Incomplete),
			CoreStatus::Error(message) => Err(PyRuntimeError::new_err(message)),
		}
	}
}

/// A solver library found on the search path, reported without loading it.
#[pyclass(frozen, module = "fznso")]
struct Discovered {
	/// The solver's name, derived from the file name.
	#[pyo3(get)]
	name: String,
	/// The version parsed from the file name, or empty if it carries none.
	/// Compared component-wise, so `10` is newer than `9`.
	#[pyo3(get)]
	version: String,
	/// The full path to the library.
	#[pyo3(get)]
	path: String,
}

#[pymethods]
impl Discovered {
	fn __repr__(&self) -> String {
		format!(
			"Discovered(name={:?}, version={:?}, path={:?})",
			self.name, self.version, self.path
		)
	}
}

impl From<fznso_core::Discovered> for Discovered {
	fn from(found: fznso_core::Discovered) -> Self {
		Self {
			name: found.name,
			version: found.version,
			path: found.path.to_string_lossy().into_owned(),
		}
	}
}

/// A loaded solver library.
#[pyclass(module = "fznso")]
struct Library {
	inner: Arc<fznso_core::Library>,
}

#[pymethods]
impl Library {
	/// Load a solver library from `path`.
	#[new]
	fn new(path: &str) -> PyResult<Self> {
		// SAFETY: the caller supplies a valid FZnSO solver library.
		let inner = unsafe { fznso_core::Library::new(path) }
			.map_err(|e| PyRuntimeError::new_err(format!("could not load solver: {e}")))?;
		Ok(Self { inner })
	}

	/// The directories searched for solver libraries, in search order.
	///
	/// Solvers live in a directory of their own rather than on the normal
	/// library path. The order is `$FZNSO_SOLVER_PATH`, then the per-user
	/// directory, then the system directories.
	#[staticmethod]
	fn search_paths() -> Vec<String> {
		fznso_core::Library::search_paths()
			.iter()
			.map(|path| path.to_string_lossy().into_owned())
			.collect()
	}

	/// List the solver libraries on the search path, without loading any.
	///
	/// Directories are visited in `search_paths` order and, within one
	/// directory, the highest version of a solver comes first. Nothing is
	/// opened, so the ABI version is not checked here; `find` skips solvers
	/// built for another ABI.
	#[staticmethod]
	fn discover() -> Vec<Discovered> {
		fznso_core::Library::discover()
			.into_iter()
			.map(Discovered::from)
			.collect()
	}

	/// Load a solver by name from the search path.
	///
	/// Without a `version` the newest is chosen, preferring earlier
	/// directories. A `version` selects by whole dotted components, so `"6"`
	/// matches `6.2.1` and `"6.2"` matches `6.2.1` but not `6.1.0`.
	///
	/// Raises `RuntimeError` if no matching solver is installed, naming the
	/// directories that were searched.
	#[staticmethod]
	#[pyo3(signature = (name, version=None))]
	fn find(name: &str, version: Option<&str>) -> PyResult<Self> {
		// SAFETY: the libraries on the search path are valid FZnSO solvers.
		let found = unsafe {
			match version {
				Some(version) => fznso_core::Library::find_version(name, version),
				None => fznso_core::Library::find(name),
			}
		};
		let inner =
			found.map_err(|e| PyRuntimeError::new_err(format!("could not load solver: {e}")))?;
		Ok(Self { inner })
	}

	/// Create a solver instance.
	fn create_solver(&self) -> DynSolver {
		DynSolver {
			inner: self.inner.create_solver(),
		}
	}
}

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
	m.add_class::<Status>()?;
	m.add_function(wrap_pyfunction!(int_set, m)?)?;
	m.add_function(wrap_pyfunction!(float_set, m)?)?;
	Ok(())
}
