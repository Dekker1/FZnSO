//! Annotations: an identifier and its arguments, attached to a decision, a
//! constraint or the objective.

use fznso_core::{
	AnnotationRef, OwnedAnnotation,
	fznso_types::{FznsoAnnotation, FznsoAnnotationMethods, FznsoPtr, FznsoStr, FznsoValueRef},
};
use pyo3::{ffi, prelude::*, types::PyList};

use crate::value::{native_value_ref, py_to_value};

/// An annotation: an identifier and the argument values that go with it.
///
/// Annotations carry advice a solver may act on or ignore — a search strategy,
/// an output hint. Attach them to a decision, a constraint or the objective
/// with the `annotations` argument of the [`LayeredModel`] builder, or return
/// them from a custom [`Model`].
///
/// ```python
/// fznso.Annotation("output_var")
/// fznso.Annotation("int_search", [[x, y], "first_fail", "indomain_min"])
/// ```
#[pyclass(frozen, module = "fznso")]
pub(crate) struct Annotation {
	/// The annotation identifier, e.g. `"output_var"`.
	#[pyo3(get)]
	ident: String,
	/// The annotation's arguments, as ordinary Python objects.
	arguments: Vec<PyObject>,
}

#[pymethods]
impl Annotation {
	#[new]
	#[pyo3(signature = (ident, arguments=None))]
	fn new(ident: String, arguments: Option<Vec<PyObject>>) -> Self {
		Annotation {
			ident,
			arguments: arguments.unwrap_or_default(),
		}
	}

	/// The annotation's arguments.
	#[getter]
	fn arguments(&self, py: Python<'_>) -> Vec<PyObject> {
		self.arguments.iter().map(|a| a.clone_ref(py)).collect()
	}

	fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
		let args = PyList::new(py, self.arguments.iter().map(|a| a.clone_ref(py)))?;
		Ok(format!("Annotation({:?}, {})", self.ident, args.repr()?))
	}
}

/// Convert the `annotations` argument of a builder call into owned ones.
pub(crate) fn py_to_annotations(
	annotations: Option<&Bound<'_, PyList>>,
) -> PyResult<Vec<OwnedAnnotation>> {
	let Some(list) = annotations else {
		return Ok(Vec::new());
	};
	let mut out = Vec::with_capacity(list.len());
	for item in list.iter() {
		let ann = item.extract::<PyRef<'_, Annotation>>()?;
		let mut args = Vec::with_capacity(ann.arguments.len());
		for arg in &ann.arguments {
			args.push(py_to_value(arg.bind(item.py()))?);
		}
		out.push(OwnedAnnotation::new(ann.ident.clone(), args));
	}
	Ok(out)
}

/// Wrap a borrowed [`Annotation`] pointer as an FZnSO annotation. The caller is
/// responsible for keeping the object alive for as long as it is used.
pub(crate) fn native_annotation_ref<'a>(ptr: *mut ffi::PyObject) -> AnnotationRef<'a> {
	AnnotationRef {
		data: FznsoPtr::from_addr(ptr as usize),
		methods: &PY_ANNOTATION_METHODS,
	}
}

/// Recover the object an annotation handle points at.
fn annotation_object<'py>(
	py: Python<'py>,
	data: FznsoPtr<'_, FznsoAnnotation>,
) -> Bound<'py, PyAny> {
	// SAFETY: `data` holds the pointer of an object the model still owns.
	unsafe { Bound::from_borrowed_ptr(py, data.addr() as *mut ffi::PyObject) }
}

extern "C" fn py_ann_ident(data: FznsoPtr<'_, FznsoAnnotation>) -> FznsoStr<'_> {
	// A model that put something other than an `Annotation` in its list has no
	// identifier to give; an empty one is the harmless answer.
	let Some((ptr, len)) = Python::with_gil(|py| {
		annotation_object(py, data)
			.downcast::<Annotation>()
			.ok()
			.map(|ann| {
				let ident = ann.get().ident.as_str();
				(ident.as_ptr(), ident.len())
			})
	}) else {
		return "".into();
	};
	// SAFETY: the bytes live in the `Annotation`'s own `String`. The class is
	// frozen, so the model cannot change it while it owns the object.
	unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr, len)) }.into()
}

extern "C" fn py_ann_argument_len(data: FznsoPtr<'_, FznsoAnnotation>) -> usize {
	Python::with_gil(|py| {
		annotation_object(py, data)
			.downcast::<Annotation>()
			.map_or(0, |ann| ann.get().arguments.len())
	})
}

extern "C" fn py_ann_argument(
	data: FznsoPtr<'_, FznsoAnnotation>,
	index: usize,
) -> FznsoValueRef<'_> {
	Python::with_gil(|py| {
		let ptr = annotation_object(py, data)
			.downcast::<Annotation>()
			.ok()
			.and_then(|ann| ann.get().arguments.get(index).map(|arg| arg.as_ptr()))
			// The argument is owned by the annotation, so its pointer stays valid
			// without a reference of our own.
			.unwrap_or_else(|| unsafe { ffi::Py_None() });
		native_value_ref(ptr)
	})
}

/// The annotation vtable backing every [`native_annotation_ref`].
static PY_ANNOTATION_METHODS: FznsoAnnotationMethods = FznsoAnnotationMethods {
	ident: py_ann_ident,
	argument_len: py_ann_argument_len,
	argument: py_ann_argument,
};
