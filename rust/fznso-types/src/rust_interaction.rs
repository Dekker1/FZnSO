//! Implementations and extensions that helps interacting with the ABI FZnSO
//! types in Rust.

/// Macro used to create [`FznsoValueRef`] from vector and option types.
macro_rules! fznso_value_impls {
	(vec $t:ty) => {
		impl<'a> From<&'a Vec<$t>> for FznsoValueRef<'a> {
			fn from(value: &'a Vec<$t>) -> Self {
				extern "C" fn kind(_: FznsoPtr<'_, FznsoValue>) -> FznsoValueKind {
					FznsoValueKind::FznsoValueList
				}
				extern "C" fn len(val: FznsoPtr<'_, FznsoValue>) -> usize {
					// SAFETY: The value is a &'a Vec<$t>, but was transmuted to a FznsoValue to fit [`FznsoValueRef`].
					let val: &Vec<$t> = unsafe { &*val.cast() };
					val.len()
				}
				extern "C" fn get_element<'a>(val: FznsoPtr<'_, FznsoValue>, idx: usize) -> FznsoValueRef<'a> {
					// SAFETY: The value is a &'a Vec<$t>, but was transmuted to a FznsoValue to fit [`FznsoValueRef`].
					let val: &Vec<$t> = unsafe { &*val.cast() };
					FznsoValueRef::from(&val[idx])
				}
				const FZNSO_LIST_VALUE_METHODS: &'static FznsoValueMethods = &FznsoValueMethods {
					kind,
					len,
					get_element,
					..*FznsoValueRef::FZNSO_DEFAULT_VALUE_METHODS
				};

				let data = FznsoPtr::from_ref(value);
				Self {
					data,
					methods: FZNSO_LIST_VALUE_METHODS,
				}
			}
		}

		impl TryFrom<&FznsoValueRef<'_>> for Vec<$t> {
			type Error = ();

			fn try_from(value: &FznsoValueRef<'_>) -> Result<Self, Self::Error> {
				if value.kind() != FznsoValueKind::FznsoValueList {
					return Err(());
				}
				let len = value.len();
				let mut result = Vec::with_capacity(len);
				for i in 0..len {
					result.push((&value.get_element(i)).try_into()?);
				}
				Ok(result)
			}
		}
	};

	(opt $t:ty) => {
		impl<'a> From<&'a Option<$t>> for FznsoValueRef<'a> {
			fn from(value: &'a Option<$t>) -> Self {
				match value {
					Some(t) => FznsoValueRef::from(t),
					None => FznsoValueRef::from(&()),
				}
			}
		}

		impl TryFrom<&FznsoValueRef<'_>> for Option<$t> {
			type Error = ();

			fn try_from(value: &FznsoValueRef<'_>) -> Result<Self, Self::Error> {
				if value.kind() == FznsoValueKind::FznsoValueAbsent {
					Ok(None)
				} else {
					let val = value.try_into()?;
					Ok(Some(val))
				}
			}
		}
	};
	($( $t:ty ),+) => {
		$(
			fznso_value_impls!{vec $t}
			fznso_value_impls!{opt $t}
			fznso_value_impls!{vec Option<$t>}
		)+
	};
}

use std::{ffi, fmt, marker::PhantomData, ptr};

use crate::{
	FznsoAnnotationRef, FznsoConstraintIdx, FznsoConstraintList, FznsoDecisionIdx,
	FznsoObjectiveList, FznsoOptionList, FznsoPtr, FznsoStatisticList, FznsoStr, FznsoTypeList,
	FznsoValue, FznsoValueKind, FznsoValueMethods, FznsoValueRef,
};

impl<T> Clone for FznsoPtr<'_, T> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<T> Copy for FznsoPtr<'_, T> {}

impl<T> fmt::Debug for FznsoPtr<'_, T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Debug::fmt(&self.ptr, f)
	}
}

impl<'a, T> FznsoPtr<'a, T> {
	/// Read back a payload stored with [`FznsoPtr::from_addr`].
	pub fn addr(self) -> usize {
		self.ptr.addr()
	}

	/// Reinterpret this handle as a pointer to `U`.
	///
	/// This is the inverse of [`FznsoPtr::from_ref`]. Dereferencing the result
	/// is only sound if `U` is the type the handle was originally built from.
	pub const fn cast<U>(self) -> *const U {
		self.ptr.cast()
	}

	/// Store a plain integer payload directly in the pointer word.
	///
	/// The result carries no provenance and must never be dereferenced; only
	/// [`FznsoPtr::addr`] may read it back. This is how small values cross the
	/// interface without an allocation.
	pub const fn from_addr(addr: usize) -> Self {
		Self {
			ptr: ptr::without_provenance(addr),
			lifetime: PhantomData,
		}
	}

	/// Wrap a reference to the implementation's own type as an opaque handle.
	///
	/// The handle keeps permission to access the whole of `S`, so the callbacks
	/// may cast it back with [`FznsoPtr::cast`].
	pub const fn from_ref<S>(value: &'a S) -> Self {
		Self {
			ptr: ptr::from_ref(value).cast(),
			lifetime: PhantomData,
		}
	}

	/// Wrap a raw pointer that already points at the handle's contents.
	pub const fn new(ptr: *const T) -> Self {
		Self {
			ptr,
			lifetime: PhantomData,
		}
	}
}

impl From<usize> for FznsoConstraintIdx {
	fn from(value: usize) -> Self {
		Self(value)
	}
}

impl TryFrom<&FznsoValueRef<'_>> for FznsoConstraintIdx {
	type Error = ();

	fn try_from(value: &FznsoValueRef<'_>) -> Result<Self, Self::Error> {
		if value.kind() == FznsoValueKind::FznsoValueConstraint {
			Ok(value.get_constraint())
		} else {
			Err(())
		}
	}
}

impl FznsoConstraintList<'static> {
	/// Returns an empty list with no constraint types.
	pub const fn empty() -> Self {
		Self {
			len: 0,
			constraints: ptr::null(),
			lifetime: PhantomData,
		}
	}
}

impl From<usize> for FznsoDecisionIdx {
	fn from(value: usize) -> Self {
		Self(value)
	}
}

impl TryFrom<&FznsoValueRef<'_>> for FznsoDecisionIdx {
	type Error = ();

	fn try_from(value: &FznsoValueRef<'_>) -> Result<Self, Self::Error> {
		if value.kind() == FznsoValueKind::FznsoValueDecision {
			Ok(value.get_decision())
		} else {
			Err(())
		}
	}
}

impl FznsoObjectiveList<'static> {
	/// Returns an empty list with no objective types.
	pub const fn empty() -> Self {
		Self {
			len: 0,
			objectives: ptr::null(),
			lifetime: PhantomData,
		}
	}
}

impl FznsoOptionList<'static> {
	/// Returns an empty list with no options.
	pub const fn empty() -> Self {
		Self {
			len: 0,
			options: ptr::null(),
			lifetime: PhantomData,
		}
	}
}

impl FznsoStatisticList<'static> {
	/// Returns an empty list with no statistics.
	pub const fn empty() -> Self {
		Self {
			len: 0,
			stats: ptr::null(),
			lifetime: PhantomData,
		}
	}
}

impl FznsoTypeList<'static> {
	/// Returns an empty list with no decision types.
	pub const fn empty() -> Self {
		Self {
			len: 0,
			types: ptr::null(),
			lifetime: PhantomData,
		}
	}
}

impl<'a> FznsoStr<'a> {
	/// Borrow a `&str` as an interface string.
	///
	/// Usable in const context, so that the capability lists a solver declares
	/// — which are `static` — can name their identifiers directly.
	pub const fn new(value: &'a str) -> Self {
		Self {
			ptr: value.as_ptr().cast(),
			len: value.len(),
			lifetime: PhantomData,
		}
	}

	/// The absent string, signalled by a null `ptr`.
	pub const NONE: Self = Self {
		ptr: ptr::null(),
		len: 0,
		lifetime: PhantomData,
	};

	/// Interpret this string as a `&str`, or `None` if `ptr` is null.
	///
	/// # Safety
	///
	/// `ptr` must point to `len` bytes of valid UTF-8 that stay live for `'a`.
	pub unsafe fn as_str(self) -> Option<&'a str> {
		if self.ptr.is_null() {
			return None;
		}
		// SAFETY: Guaranteed by the caller.
		Some(unsafe {
			std::str::from_utf8_unchecked(std::slice::from_raw_parts(
				self.ptr as *const u8,
				self.len,
			))
		})
	}
}

impl<'a> From<&'a str> for FznsoStr<'a> {
	fn from(value: &'a str) -> Self {
		Self::new(value)
	}
}

impl<'a> FznsoAnnotationRef<'a> {
	/// The annotation's identifier string (e.g. `"output_var"`).
	pub fn ident(&self) -> &'a str {
		// SAFETY: The string returned by the annotation vtable is valid UTF-8 for
		// at least as long as the annotation reference, i.e. 'a.
		unsafe { (self.methods.ident)(self.data).as_str() }.unwrap_or("")
	}

	/// Number of arguments this annotation carries.
	pub fn argument_len(&self) -> usize {
		(self.methods.argument_len)(self.data)
	}

	/// The `index`-th argument of this annotation.
	pub fn argument(&self, index: usize) -> FznsoValueRef<'a> {
		(self.methods.argument)(self.data, index)
	}

	/// Iterator over all arguments of this annotation.
	pub fn arguments(&self) -> impl Iterator<Item = FznsoValueRef<'a>> + 'a {
		// Copy both pointer fields so the closure owns them and lives for 'a.
		let data = self.data;
		let methods = self.methods;
		let len = (methods.argument_len)(data);
		(0..len).map(move |i| (methods.argument)(data, i))
	}
}

impl<'a> FznsoValueRef<'a> {
	/// Default [`FznsoValueMethods`] implementation, which can be used for
	/// absent values or as a base for other value types.
	pub const FZNSO_DEFAULT_VALUE_METHODS: &'static FznsoValueMethods = &FznsoValueMethods {
		kind: Self::absent_kind,
		len: Self::no_call,
		as_decision: Self::no_call,
		as_constraint: Self::no_call,
		as_int: Self::no_call,
		as_float: Self::no_call,
		as_string: Self::no_call,
		as_bool: Self::no_call,
		get_range_int: Self::no_call_idx,
		get_range_float: Self::no_call_idx,
		get_element: Self::no_call_idx,
	};

	/// Function that always returns [`FznsoValueKind::FznsoValueAbsent`].
	extern "C" fn absent_kind(_: FznsoPtr<'_, FznsoValue>) -> FznsoValueKind {
		FznsoValueKind::FznsoValueAbsent
	}

	/// Rust call to [`FznsoValueMethods::get_bool`] with the
	/// [`FznsoValueRef::data`] attribute.
	pub fn get_bool(&self) -> bool {
		(self.methods.as_bool)(self.data)
	}

	/// Rust call to [`FznsoValueMethods::get_constraint`] with the
	/// [`FznsoValueRef::data`] attribute.
	pub fn get_constraint(&self) -> FznsoConstraintIdx {
		(self.methods.as_constraint)(self.data)
	}

	/// Rust call to [`FznsoValueMethods::get_decision`] with the
	/// [`FznsoValueRef::data`] attribute.
	pub fn get_decision(&self) -> FznsoDecisionIdx {
		(self.methods.as_decision)(self.data)
	}

	/// Rust call to [`FznsoValueMethods::get_element`] with the
	/// [`FznsoValueRef::data`] attribute.
	pub fn get_element(&self, idx: usize) -> FznsoValueRef<'a> {
		(self.methods.get_element)(self.data, idx)
	}

	/// Rust call to [`FznsoValueMethods::get_float`] with the
	/// [`FznsoValueRef::data`] attribute.
	pub fn get_float(&self) -> f64 {
		(self.methods.as_float)(self.data)
	}

	/// Rust call to [`FznsoValueMethods::get_int`] with the
	/// [`FznsoValueRef::data`] attribute.
	pub fn get_int(&self) -> i64 {
		(self.methods.as_int)(self.data)
	}

	/// Rust call to [`FznsoValueMethods::get_range_float`] with the
	/// [`FznsoValueRef::data`] attribute.
	pub fn get_range_float(&self, idx: usize) -> (f64, f64) {
		let r = (self.methods.get_range_float)(self.data, idx);
		(r.min, r.max)
	}

	/// Rust call to [`FznsoValueMethods::get_range_int`] with the
	/// [`FznsoValueRef::data`] attribute.
	pub fn get_range_int(&self, idx: usize) -> (i64, i64) {
		let r = (self.methods.get_range_int)(self.data, idx);
		(r.min, r.max)
	}

	/// Rust call to [`FznsoValueMethods::as_string`] and
	/// [`FznsoValueMethods::len`] to obtain the string as a `&str`.
	///
	/// The returned slice is valid for the lifetime of the value.  The
	/// underlying bytes are not required to be null-terminated.
	pub fn get_string(&self) -> &'a str {
		let ptr = (self.methods.as_string)(self.data) as *const u8;
		let len = (self.methods.len)(self.data);
		// SAFETY: as_string returns a pointer to valid UTF-8 bytes of length
		// `len` for the lifetime of this FznsoValueRef.
		unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr, len)) }
	}

	/// Wrapper for a call to [`FznsoValueMethods::len`] with the
	/// [`FznsoValueRef::data`] attribute, returning `true` if the `len` is
	/// zero.
	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}

	/// Rust call to [`FznsoValueMethods::kind`] with the
	/// [`FznsoValueRef::data`] attribute.
	pub fn kind(&self) -> FznsoValueKind {
		(self.methods.kind)(self.data)
	}

	/// Rust call to [`FznsoValueMethods::len`] with the [`FznsoValueRef::data`]
	/// attribute.
	pub fn len(&self) -> usize {
		(self.methods.len)(self.data)
	}
	/// Fallback implementation for different value methods that always panics.
	extern "C" fn no_call<T>(_: FznsoPtr<'_, FznsoValue>) -> T {
		panic!("This value implementation does not provide this callback");
	}
	/// Fallback implementation for value methods that take a `usize` argument
	/// that always panics.
	extern "C" fn no_call_idx<T>(v: FznsoPtr<'_, FznsoValue>, _: usize) -> T {
		Self::no_call(v)
	}
}

impl<'a> From<&'a ()> for FznsoValueRef<'a> {
	fn from(value: &'a ()) -> Self {
		let data = FznsoPtr::from_ref(value);
		Self {
			data,
			methods: Self::FZNSO_DEFAULT_VALUE_METHODS,
		}
	}
}

impl<'a> From<&'a FznsoConstraintIdx> for FznsoValueRef<'a> {
	fn from(value: &'a FznsoConstraintIdx) -> Self {
		extern "C" fn kind(_: FznsoPtr<'_, FznsoValue>) -> FznsoValueKind {
			FznsoValueKind::FznsoValueConstraint
		}
		extern "C" fn as_constraint(val: FznsoPtr<'_, FznsoValue>) -> FznsoConstraintIdx {
			// SAFETY: The value is a &'a FznsoConstraintIdx, but was transmuted to a
			// &'a FznsoValue to fit [`FznsoValueRef`].
			let val: &FznsoConstraintIdx = unsafe { &*val.cast() };
			*val
		}
		const FZNSO_CONSTRAINT_VALUE_METHODS: &FznsoValueMethods = &FznsoValueMethods {
			kind,
			as_constraint,
			..*FznsoValueRef::FZNSO_DEFAULT_VALUE_METHODS
		};

		let data = FznsoPtr::from_ref(value);
		Self {
			data,
			methods: FZNSO_CONSTRAINT_VALUE_METHODS,
		}
	}
}

impl<'a> From<&'a FznsoDecisionIdx> for FznsoValueRef<'a> {
	fn from(value: &'a FznsoDecisionIdx) -> Self {
		extern "C" fn kind(_: FznsoPtr<'_, FznsoValue>) -> FznsoValueKind {
			FznsoValueKind::FznsoValueDecision
		}
		extern "C" fn as_decision(val: FznsoPtr<'_, FznsoValue>) -> FznsoDecisionIdx {
			// SAFETY: The value is a FznsoDecisionIdx, but was transmuted to a
			// &'a FznsoValue to fit [`FznsoValueRef`].
			let val: &FznsoDecisionIdx = unsafe { &*val.cast() };
			*val
		}
		const FZNSO_CONSTRAINT_VALUE_METHODS: &FznsoValueMethods = &FznsoValueMethods {
			kind,
			as_decision,
			..*FznsoValueRef::FZNSO_DEFAULT_VALUE_METHODS
		};

		let data = FznsoPtr::from_ref(value);
		Self {
			data,
			methods: FZNSO_CONSTRAINT_VALUE_METHODS,
		}
	}
}

impl<'a> From<&'a String> for FznsoValueRef<'a> {
	fn from(value: &'a String) -> Self {
		extern "C" fn kind(_: FznsoPtr<'_, FznsoValue>) -> FznsoValueKind {
			FznsoValueKind::FznsoValueString
		}
		extern "C" fn as_string(val: FznsoPtr<'_, FznsoValue>) -> *const ffi::c_char {
			// SAFETY: val was created from &String via transmute below.
			let s: &String = unsafe { &*val.cast() };
			s.as_ptr() as *const ffi::c_char
		}
		extern "C" fn len(val: FznsoPtr<'_, FznsoValue>) -> usize {
			// SAFETY: val was created from &String via transmute below.
			let s: &String = unsafe { &*val.cast() };
			s.len()
		}
		const FZNSO_STRING_VALUE_METHODS: &FznsoValueMethods = &FznsoValueMethods {
			kind,
			len,
			as_string,
			..*FznsoValueRef::FZNSO_DEFAULT_VALUE_METHODS
		};

		let data = FznsoPtr::from_ref(value);
		Self {
			data,
			methods: FZNSO_STRING_VALUE_METHODS,
		}
	}
}

impl<'a> From<&'a bool> for FznsoValueRef<'a> {
	fn from(value: &'a bool) -> Self {
		extern "C" fn kind(_: FznsoPtr<'_, FznsoValue>) -> FznsoValueKind {
			FznsoValueKind::FznsoValueBool
		}
		extern "C" fn as_bool(val: FznsoPtr<'_, FznsoValue>) -> bool {
			// SAFETY: The value is a &'a bool, but was transmuted to a &'a FznsoValue
			// to fit [`FznsoValueRef`].
			let val: &bool = unsafe { &*val.cast() };
			*val
		}
		const FZNSO_BOOL_VALUE_METHODS: &FznsoValueMethods = &FznsoValueMethods {
			kind,
			as_bool,
			..*FznsoValueRef::FZNSO_DEFAULT_VALUE_METHODS
		};

		let data = FznsoPtr::from_ref(value);
		Self {
			data,
			methods: FZNSO_BOOL_VALUE_METHODS,
		}
	}
}

impl<'a> From<&'a f64> for FznsoValueRef<'a> {
	fn from(value: &'a f64) -> Self {
		extern "C" fn kind(_: FznsoPtr<'_, FznsoValue>) -> FznsoValueKind {
			FznsoValueKind::FznsoValueFloat
		}
		extern "C" fn as_float(val: FznsoPtr<'_, FznsoValue>) -> f64 {
			// SAFETY: The value is a &'a f64, but was transmuted to a &'a FznsoValue
			// to fit [`FznsoValueRef`].
			let val: &f64 = unsafe { &*val.cast() };
			*val
		}
		const FZNSO_F64_VALUE_METHODS: &FznsoValueMethods = &FznsoValueMethods {
			kind,
			as_float,
			..*FznsoValueRef::FZNSO_DEFAULT_VALUE_METHODS
		};

		let data = FznsoPtr::from_ref(value);
		Self {
			data,
			methods: FZNSO_F64_VALUE_METHODS,
		}
	}
}

impl<'a> From<&'a ffi::CStr> for FznsoValueRef<'a> {
	fn from(value: &'a ffi::CStr) -> Self {
		extern "C" fn kind(_: FznsoPtr<'_, FznsoValue>) -> FznsoValueKind {
			FznsoValueKind::FznsoValueString
		}
		extern "C" fn as_string(val: FznsoPtr<'_, FznsoValue>) -> *const ffi::c_char {
			val.cast()
		}
		extern "C" fn len(val: FznsoPtr<'_, FznsoValue>) -> usize {
			// SAFETY: data was set to value.as_ptr(), so it points to the first
			// byte of a valid null-terminated C string (CStr invariant).
			unsafe { ffi::CStr::from_ptr(val.cast()).to_bytes().len() }
		}
		const FZNSO_CSTR_VALUE_METHODS: &FznsoValueMethods = &FznsoValueMethods {
			kind,
			len,
			as_string,
			..*FznsoValueRef::FZNSO_DEFAULT_VALUE_METHODS
		};

		// data points to the first byte of the CStr content; as_string returns it
		// directly and len re-derives the length from the null byte.
		let data = FznsoPtr::new(value.as_ptr().cast());
		Self {
			data,
			methods: FZNSO_CSTR_VALUE_METHODS,
		}
	}
}

impl<'a> From<&'a ffi::CString> for FznsoValueRef<'a> {
	fn from(value: &'a ffi::CString) -> Self {
		value.as_c_str().into()
	}
}

impl<'a> From<&'a i64> for FznsoValueRef<'a> {
	fn from(value: &'a i64) -> Self {
		extern "C" fn kind(_: FznsoPtr<'_, FznsoValue>) -> FznsoValueKind {
			FznsoValueKind::FznsoValueInt
		}
		extern "C" fn as_int(val: FznsoPtr<'_, FznsoValue>) -> i64 {
			// SAFETY: The value is a &'a i64, but was transmuted to a &'a FznsoValue
			// to fit [`FznsoValueRef`].
			let val: &i64 = unsafe { &*val.cast() };
			*val
		}
		const FZNSO_I64_VALUE_METHODS: &FznsoValueMethods = &FznsoValueMethods {
			kind,
			as_int,
			..*FznsoValueRef::FZNSO_DEFAULT_VALUE_METHODS
		};

		let data = FznsoPtr::from_ref(value);
		Self {
			data,
			methods: FZNSO_I64_VALUE_METHODS,
		}
	}
}

impl From<FznsoConstraintIdx> for FznsoValueRef<'static> {
	fn from(value: FznsoConstraintIdx) -> Self {
		extern "C" fn kind(_: FznsoPtr<'_, FznsoValue>) -> FznsoValueKind {
			FznsoValueKind::FznsoValueConstraint
		}
		extern "C" fn as_constraint(val: FznsoPtr<'_, FznsoValue>) -> FznsoConstraintIdx {
			// The index is encoded in the pointer word itself.
			FznsoConstraintIdx(val.addr())
		}
		const FZNSO_DECISION_VALUE_METHODS: &FznsoValueMethods = &FznsoValueMethods {
			kind,
			as_constraint,
			..*FznsoValueRef::FZNSO_DEFAULT_VALUE_METHODS
		};

		Self {
			// The index is encoded in the pointer word; nothing is dereferenced.
			data: FznsoPtr::from_addr(value.0),
			methods: FZNSO_DECISION_VALUE_METHODS,
		}
	}
}

impl From<FznsoDecisionIdx> for FznsoValueRef<'static> {
	fn from(value: FznsoDecisionIdx) -> Self {
		extern "C" fn kind(_: FznsoPtr<'_, FznsoValue>) -> FznsoValueKind {
			FznsoValueKind::FznsoValueDecision
		}
		extern "C" fn as_decision(val: FznsoPtr<'_, FznsoValue>) -> FznsoDecisionIdx {
			// The index is encoded in the pointer word itself.
			FznsoDecisionIdx(val.addr())
		}
		const FZNSO_DECISION_VALUE_METHODS: &FznsoValueMethods = &FznsoValueMethods {
			kind,
			as_decision,
			..*FznsoValueRef::FZNSO_DEFAULT_VALUE_METHODS
		};

		Self {
			// The index is encoded in the pointer word; nothing is dereferenced.
			data: FznsoPtr::from_addr(value.0),
			methods: FZNSO_DECISION_VALUE_METHODS,
		}
	}
}

impl From<bool> for FznsoValueRef<'static> {
	fn from(value: bool) -> Self {
		extern "C" fn kind(_: FznsoPtr<'_, FznsoValue>) -> FznsoValueKind {
			FznsoValueKind::FznsoValueBool
		}
		extern "C" fn as_bool(val: FznsoPtr<'_, FznsoValue>) -> bool {
			// The value is encoded in the pointer word itself.
			val.addr() != 0
		}
		const FZNSO_BOOL_VALUE_METHODS: &FznsoValueMethods = &FznsoValueMethods {
			kind,
			as_bool,
			..*FznsoValueRef::FZNSO_DEFAULT_VALUE_METHODS
		};

		Self {
			// The value is encoded in the pointer word; nothing is dereferenced.
			data: FznsoPtr::from_addr(value as usize),
			methods: FZNSO_BOOL_VALUE_METHODS,
		}
	}
}

impl TryFrom<&FznsoValueRef<'_>> for String {
	type Error = ();

	fn try_from(value: &FznsoValueRef<'_>) -> Result<Self, Self::Error> {
		if value.kind() == FznsoValueKind::FznsoValueString {
			Ok(value.get_string().to_owned())
		} else {
			Err(())
		}
	}
}

impl TryFrom<&FznsoValueRef<'_>> for bool {
	type Error = ();

	fn try_from(value: &FznsoValueRef<'_>) -> Result<Self, Self::Error> {
		if (value.methods.kind)(value.data) == FznsoValueKind::FznsoValueBool {
			Ok((value.methods.as_bool)(value.data))
		} else {
			Err(())
		}
	}
}

impl TryFrom<&FznsoValueRef<'_>> for f64 {
	type Error = ();

	fn try_from(value: &FznsoValueRef<'_>) -> Result<Self, Self::Error> {
		if value.kind() == FznsoValueKind::FznsoValueFloat {
			Ok(value.get_float())
		} else {
			Err(())
		}
	}
}

impl TryFrom<&FznsoValueRef<'_>> for ffi::CString {
	type Error = ();

	fn try_from(value: &FznsoValueRef<'_>) -> Result<Self, Self::Error> {
		if value.kind() == FznsoValueKind::FznsoValueString {
			// Returns Err if the string contains interior null bytes.
			ffi::CString::new(value.get_string()).map_err(|_| ())
		} else {
			Err(())
		}
	}
}

impl TryFrom<&FznsoValueRef<'_>> for i64 {
	type Error = ();

	fn try_from(value: &FznsoValueRef<'_>) -> Result<Self, Self::Error> {
		if value.kind() == FznsoValueKind::FznsoValueInt {
			Ok(value.get_int())
		} else {
			Err(())
		}
	}
}

impl<'a> TryFrom<&'a FznsoValueRef<'a>> for &'a str {
	type Error = ();

	fn try_from(value: &'a FznsoValueRef<'a>) -> Result<Self, Self::Error> {
		if value.kind() == FznsoValueKind::FznsoValueString {
			Ok(value.get_string())
		} else {
			Err(())
		}
	}
}

impl From<FznsoConstraintIdx> for usize {
	fn from(val: FznsoConstraintIdx) -> Self {
		val.0
	}
}

impl From<FznsoDecisionIdx> for usize {
	fn from(val: FznsoDecisionIdx) -> Self {
		val.0
	}
}

fznso_value_impls! { i64, f64, bool, FznsoConstraintIdx, FznsoDecisionIdx, ffi::CString, String }
