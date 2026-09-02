//! The two ways to describe a problem: [`LayeredModel`], built by the bindings,
//! and [`Model`], subclassed in Python and read back through
//! [`PyModelAdapter`].

use fznso_core::{
	AnnotationRef, ConstraintIdx, DecisionIdx, Model as _, OwnedValue, Value,
	fznso_types::{FznsoDecisionIdx, FznsoType, FznsoTypeBase},
};
use pyo3::{
	exceptions::{PyIndexError, PyNotImplementedError, PyRuntimeError},
	prelude::*,
	types::{PyDict, PyList, PyString, PyTuple},
};

use crate::{
	annotation::{native_annotation_ref, py_to_annotations},
	value::{Constraint, Decision, Type, native_value_ref, py_to_value},
};

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
pub(crate) struct Model;

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

	/// The [`Annotation`]s on the decision at `index`. Defaults to none.
	///
	/// Like the other accessors this *borrows*: return a list the model owns,
	/// not one built for the call.
	fn decision_annotations(&self, index: usize) -> Vec<PyObject> {
		let _ = index;
		Vec::new()
	}
	/// The [`Annotation`]s on the constraint at `index`. Defaults to none.
	fn constraint_annotations(&self, index: usize) -> Vec<PyObject> {
		let _ = index;
		Vec::new()
	}
	/// The [`Annotation`]s on the objective. Defaults to none.
	fn objective_annotations(&self) -> Vec<PyObject> {
		Vec::new()
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

/// Presents a Python object implementing the [`Model`] protocol to the solver,
/// calling back into Python on demand.
pub(crate) struct PyModelAdapter {
	object: PyObject,
	/// The first error raised by a Python method, re-raised after the run.
	///
	/// A `Mutex` rather than a `RefCell`: the model is a read-only view the
	/// solver may query from several threads at once, so it must be `Sync`.
	pub(crate) error: std::sync::Mutex<Option<PyErr>>,
}

impl PyModelAdapter {
	pub(crate) fn new(object: PyObject) -> Self {
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

	/// The length of a `*_annotations` list, defaulting to none on error.
	fn annotation_len(&self, method: &str, index: Option<usize>) -> usize {
		let list = self.call_object(method, index);
		Python::with_gil(|py| {
			list.bind(py).len().unwrap_or_else(|err| {
				self.record(err);
				0
			})
		})
	}

	/// Present the `index`th entry of a `*_annotations` list as an annotation
	/// borrowing it. As with values, the model must own the object so it
	/// outlives the annotation.
	fn borrow_annotation(
		&self,
		method: &str,
		arg: Option<usize>,
		index: usize,
	) -> AnnotationRef<'_> {
		let list = self.call_object(method, arg);
		let entry = Python::with_gil(|py| {
			list.bind(py)
				.get_item(index)
				.map(Bound::unbind)
				.unwrap_or_else(|err| {
					self.record(err);
					py.None()
				})
		});
		// The model keeps its own reference to the annotation, so releasing
		// ours leaves it pointing at a live object.
		native_annotation_ref(entry.as_ptr())
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
		// A model that cannot answer records the exception, which is raised
		// once the run unwinds; the type returned here only keeps the
		// traversal going until then.
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

	// --- Annotations ---
	fn decision_annotation_len(&self, decision: DecisionIdx) -> usize {
		self.annotation_len("decision_annotations", Some(decision.0))
	}
	fn decision_annotation(&self, decision: DecisionIdx, index: usize) -> AnnotationRef<'_> {
		self.borrow_annotation("decision_annotations", Some(decision.0), index)
	}
	fn constraint_annotation_len(&self, con: ConstraintIdx) -> usize {
		self.annotation_len("constraint_annotations", Some(con.0))
	}
	fn constraint_annotation(&self, con: ConstraintIdx, index: usize) -> AnnotationRef<'_> {
		self.borrow_annotation("constraint_annotations", Some(con.0), index)
	}
	fn objective_annotation_len(&self) -> usize {
		self.annotation_len("objective_annotations", None)
	}
	fn objective_annotation(&self, index: usize) -> AnnotationRef<'_> {
		self.borrow_annotation("objective_annotations", None, index)
	}
}

/// A model to hand to a solver.
///
/// Decisions and constraints are added to the current layer. Layers can be
/// pushed, popped and committed for incremental solving.
#[pyclass(module = "fznso")]
pub(crate) struct LayeredModel {
	pub(crate) inner: fznso_core::LayeredModel,
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
	#[pyo3(signature = (ty, domain=None, name=None, defined=false, in_solution=true, annotations=None))]
	fn add_decision(
		&mut self,
		ty: &Type,
		domain: Option<&Bound<'_, PyAny>>,
		name: Option<String>,
		defined: bool,
		in_solution: bool,
		annotations: Option<&Bound<'_, PyList>>,
	) -> PyResult<Decision> {
		let domain = match domain {
			Some(d) => py_to_value(d)?,
			None => OwnedValue::Absent,
		};
		let idx = self.inner.add_decision(
			ty.ty,
			domain,
			name,
			defined,
			in_solution,
			py_to_annotations(annotations)?,
		);
		Ok(Decision { index: idx.0 })
	}

	/// Add a constraint with the given identifier and arguments.
	#[pyo3(signature = (ident, arguments, defines=None, annotations=None))]
	fn add_constraint(
		&mut self,
		ident: &str,
		arguments: &Bound<'_, PyList>,
		defines: Option<Decision>,
		annotations: Option<&Bound<'_, PyList>>,
	) -> PyResult<Constraint> {
		let mut args = Vec::with_capacity(arguments.len());
		for arg in arguments.iter() {
			args.push(py_to_value(&arg)?);
		}
		let idx = self.inner.add_constraint(
			ident,
			args,
			defines.map(|d| FznsoDecisionIdx(d.index)),
			py_to_annotations(annotations)?,
		);
		Ok(Constraint { index: idx.0 })
	}

	/// Set the objective. Pass no identifier for a satisfaction problem.
	#[pyo3(signature = (ident, argument=None, annotations=None))]
	fn set_objective(
		&mut self,
		ident: Option<String>,
		argument: Option<Bound<'_, PyAny>>,
		annotations: Option<&Bound<'_, PyList>>,
	) -> PyResult<()> {
		let arg = match argument {
			Some(a) => py_to_value(&a)?,
			None => OwnedValue::Absent,
		};
		self.inner
			.set_objective(ident, arg, py_to_annotations(annotations)?);
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

	/// Mark a permanent layer as redundant, letting the solver discard it.
	///
	/// Raises `IndexError` if the layer is not permanent: only a layer that can
	/// never be retracted can be declared vacuous.
	fn mark_redundant(&mut self, layer: usize) -> PyResult<()> {
		let permanent = self.inner.layer_permanent();
		if layer >= permanent {
			return Err(PyIndexError::new_err(format!(
				"layer {layer} is not permanent (only 0..{permanent} are)"
			)));
		}
		self.inner.mark_redundant(layer);
		Ok(())
	}

	/// Declare how many leading layers are unchanged since the last run, so the
	/// solver can keep the state it built for them.
	///
	/// This is 0 until you say otherwise, which is always correct but re-reads
	/// the whole model every run. Reporting a layer as unchanged when it is not
	/// is the one way to get a wrong answer.
	fn set_unchanged(&mut self, count: usize) {
		self.inner.set_unchanged(count);
	}
}
