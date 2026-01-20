//! Implementations and extensions that helps interacting with the ABI FZnSO
//! types in Rust.

use std::{ffi, mem};

use crate::{
	FznsoConstraintIdx, FznsoDecisionIdx, FznsoValue, FznsoValueKind, FznsoValueMethods,
	FznsoValueRef,
};

/// Macro used to create [`FznsoValueRef`] from vector and option types.
macro_rules! fznso_value_impls {
	(vec $t:ty) => {
		impl<'a> From<&'a Vec<$t>> for FznsoValueRef<'a> {
			fn from(value: &'a Vec<$t>) -> Self {
				extern "C" fn kind(_: &FznsoValue) -> FznsoValueKind {
					FznsoValueKind::FznsoValueList
				}
				extern "C" fn len(val: &FznsoValue) -> usize {
					// SAFETY: The value is a &'a Vec<$t>, but was transmuted to a FznsoValue to fit [`FznsoValueRef`].
					let val: &Vec<$t> = unsafe { mem::transmute(val) };
					val.len()
				}
				extern "C" fn get_element<'a>(val: &'a FznsoValue, idx: usize) -> FznsoValueRef<'a> {
					// SAFETY: The value is a &'a Vec<$t>, but was transmuted to a FznsoValue to fit [`FznsoValueRef`].
					let val: &Vec<$t> = unsafe { mem::transmute(val) };
					FznsoValueRef::from(&val[idx])
				}
				const FZNSO_LIST_VALUE_METHODS: &'static FznsoValueMethods = &FznsoValueMethods {
					kind,
					len,
					get_element,
					..*FznsoValueRef::FZNSO_DEFAULT_VALUE_METHODS
				};

				// SAFETY: The value is a &'a Vec<$t>, and will never be used without transmuting back.
				let data: &FznsoValue = unsafe { mem::transmute(value) };
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
		impl From<&Option<$t>> for FznsoValueRef<'_> {
			fn from(value: &Option<$t>) -> Self {
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

impl FznsoValueRef<'_> {
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
	extern "C" fn absent_kind(_: &FznsoValue) -> FznsoValueKind {
		FznsoValueKind::FznsoValueAbsent
	}
	/// Fallback implementation for different value methods that always panics.
	extern "C" fn no_call<T>(_: &FznsoValue) -> T {
		panic!("This value implementation does not provide this callback");
	}
	/// Fallback implementation for value methods that take a `usize` argument
	/// that always panics.
	extern "C" fn no_call_idx<T>(v: &FznsoValue, _: usize) -> T {
		Self::no_call(v)
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

	/// Wrapper for a call to [`FznsoValueMethods::len`] with the
	/// [`FznsoValueRef::data`] attribute, returning `true` if the `len` is
	/// zero.
	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}

	/// Rust call to [`FznsoValueMethods::get_decision`] with the
	/// [`FznsoValueRef::data`] attribute.
	pub fn get_decision(&self) -> FznsoDecisionIdx {
		(self.methods.as_decision)(self.data)
	}

	/// Rust call to [`FznsoValueMethods::get_constraint`] with the
	/// [`FznsoValueRef::data`] attribute.
	pub fn get_constraint(&self) -> FznsoConstraintIdx {
		(self.methods.as_constraint)(self.data)
	}

	/// Rust call to [`FznsoValueMethods::get_int`] with the
	/// [`FznsoValueRef::data`] attribute.
	pub fn get_int(&self) -> i64 {
		(self.methods.as_int)(self.data)
	}

	/// Rust call to [`FznsoValueMethods::get_float`] with the
	/// [`FznsoValueRef::data`] attribute.
	pub fn get_float(&self) -> f64 {
		(self.methods.as_float)(self.data)
	}

	/// Rust call to [`FznsoValueMethods::get_string`] with the
	/// [`FznsoValueRef::data`] attribute.
	pub fn get_string(&self) -> &'_ ffi::CStr {
		let s = (self.methods.as_string)(self.data);
		// SAFETY: The string is valid UTF-8 and null-terminated.
		unsafe { ffi::CStr::from_ptr(s) }
	}

	/// Rust call to [`FznsoValueMethods::get_bool`] with the
	/// [`FznsoValueRef::data`] attribute.
	pub fn get_bool(&self) -> bool {
		(self.methods.as_bool)(self.data)
	}

	/// Rust call to [`FznsoValueMethods::get_range_int`] with the
	/// [`FznsoValueRef::data`] attribute.
	pub fn get_range_int(&self, idx: usize) -> (i64, i64) {
		let [min, max] = (self.methods.get_range_int)(self.data, idx);
		(min, max)
	}

	/// Rust call to [`FznsoValueMethods::get_range_float`] with the
	/// [`FznsoValueRef::data`] attribute.
	pub fn get_range_float(&self, idx: usize) -> (f64, f64) {
		let [min, max] = (self.methods.get_range_float)(self.data, idx);
		(min, max)
	}

	/// Rust call to [`FznsoValueMethods::get_element`] with the
	/// [`FznsoValueRef::data`] attribute.
	pub fn get_element(&self, idx: usize) -> FznsoValueRef<'_> {
		(self.methods.get_element)(self.data, idx)
	}
}

impl From<usize> for FznsoConstraintIdx {
	fn from(value: usize) -> Self {
		Self(value)
	}
}

impl Into<usize> for FznsoConstraintIdx {
	fn into(self) -> usize {
		self.0
	}
}

impl From<usize> for FznsoDecisionIdx {
	fn from(value: usize) -> Self {
		Self(value)
	}
}

impl Into<usize> for FznsoDecisionIdx {
	fn into(self) -> usize {
		self.0
	}
}

impl From<&()> for FznsoValueRef<'_> {
	fn from(value: &()) -> Self {
		// SAFETY: The value is a &'a (), but was transmuted to a &'a FznsoValue
		// to fit [`FznsoValueRef`].
		let data: &FznsoValue = unsafe { mem::transmute(value) };
		Self {
			data,
			methods: Self::FZNSO_DEFAULT_VALUE_METHODS,
		}
	}
}

impl From<&bool> for FznsoValueRef<'_> {
	fn from(value: &bool) -> Self {
		extern "C" fn kind(_: &FznsoValue) -> FznsoValueKind {
			FznsoValueKind::FznsoValueBool
		}
		extern "C" fn get_bool(val: &FznsoValue) -> bool {
			// SAFETY: The value is a &'a bool, but was transmuted to a &'a FznsoValue
			// to fit [`FznsoValueRef`].
			let val: &bool = unsafe { mem::transmute(val) };
			*val
		}
		const FZNSO_BOOL_VALUE_METHODS: &FznsoValueMethods = &FznsoValueMethods {
			kind,
			as_bool: get_bool,
			..*FznsoValueRef::FZNSO_DEFAULT_VALUE_METHODS
		};

		// SAFETY: The value is a &'a bool, but was transmuted to a &'a FznsoValue
		// to fit [`FznsoValueRef`].
		let data: &FznsoValue = unsafe { mem::transmute(value) };
		Self {
			data,
			methods: FZNSO_BOOL_VALUE_METHODS,
		}
	}
}

impl From<&f64> for FznsoValueRef<'_> {
	fn from(value: &f64) -> Self {
		extern "C" fn kind(_: &FznsoValue) -> FznsoValueKind {
			FznsoValueKind::FznsoValueInt
		}
		extern "C" fn get_float(val: &FznsoValue) -> f64 {
			// SAFETY: The value is a &'a f64, but was transmuted to a &'a FznsoValue
			// to fit [`FznsoValueRef`].
			let val: &f64 = unsafe { mem::transmute(val) };
			*val
		}
		const FZNSO_F64_VALUE_METHODS: &FznsoValueMethods = &FznsoValueMethods {
			kind,
			as_float: get_float,
			..*FznsoValueRef::FZNSO_DEFAULT_VALUE_METHODS
		};

		// SAFETY: The value is a &'a f64, but was transmuted to a &'a FznsoValue to
		// fit [`FznsoValueRef`].
		let data: &FznsoValue = unsafe { mem::transmute(value) };
		Self {
			data,
			methods: FZNSO_F64_VALUE_METHODS,
		}
	}
}

impl From<&i64> for FznsoValueRef<'_> {
	fn from(value: &i64) -> Self {
		extern "C" fn kind(_: &FznsoValue) -> FznsoValueKind {
			FznsoValueKind::FznsoValueInt
		}
		extern "C" fn get_int(val: &FznsoValue) -> i64 {
			// SAFETY: The value is a &'a i64, but was transmuted to a &'a FznsoValue
			// to fit [`FznsoValueRef`].
			let val: &i64 = unsafe { mem::transmute(val) };
			*val
		}
		const FZNSO_I64_VALUE_METHODS: &FznsoValueMethods = &FznsoValueMethods {
			kind,
			as_int: get_int,
			..*FznsoValueRef::FZNSO_DEFAULT_VALUE_METHODS
		};

		// SAFETY: The value is a &'a i64, but was transmuted to a &'a FznsoValue to
		// fit [`FznsoValueRef`].
		let data: &FznsoValue = unsafe { mem::transmute(value) };
		Self {
			data,
			methods: FZNSO_I64_VALUE_METHODS,
		}
	}
}

impl From<&ffi::CStr> for FznsoValueRef<'_> {
	fn from(value: &ffi::CStr) -> Self {
		extern "C" fn kind(_: &FznsoValue) -> FznsoValueKind {
			FznsoValueKind::FznsoValueString
		}
		extern "C" fn get_string(val: &FznsoValue) -> *const ffi::c_char {
			let val: *const _ = val;
			val as *const ffi::c_char
		}
		const FZNSO_STR_VALUE_METHODS: &FznsoValueMethods = &FznsoValueMethods {
			kind,
			as_string: get_string,
			..*FznsoValueRef::FZNSO_DEFAULT_VALUE_METHODS
		};

		// SAFETY: The value is a &'a ffi::CStr, but was transmuted to a &'a
		// FznsoValue to fit [`FznsoValueRef`].
		let data: &FznsoValue = unsafe { &*(value.as_ptr() as *const FznsoValue) };
		Self {
			data,
			methods: FZNSO_STR_VALUE_METHODS,
		}
	}
}

impl From<&ffi::CString> for FznsoValueRef<'_> {
	fn from(value: &ffi::CString) -> Self {
		value.as_c_str().into()
	}
}

impl From<&FznsoConstraintIdx> for FznsoValueRef<'_> {
	fn from(value: &FznsoConstraintIdx) -> Self {
		extern "C" fn kind(_: &FznsoValue) -> FznsoValueKind {
			FznsoValueKind::FznsoValueDecision
		}
		extern "C" fn get_constraint(val: &FznsoValue) -> FznsoConstraintIdx {
			// SAFETY: The value is a &'a FznsoConstraintIdx, but was transmuted to a
			// &'a FznsoValue to fit [`FznsoValueRef`].
			let val: &FznsoConstraintIdx = unsafe { mem::transmute(val) };
			*val
		}
		const FZNSO_CONSTRAINT_VALUE_METHODS: &FznsoValueMethods = &FznsoValueMethods {
			kind,
			as_constraint: get_constraint,
			..*FznsoValueRef::FZNSO_DEFAULT_VALUE_METHODS
		};

		// SAFETY: The value is a &'a FznsoConstraintIdx, and will never be used
		// without transmuting back.
		let data: &FznsoValue = unsafe { mem::transmute(value) };
		Self {
			data,
			methods: FZNSO_CONSTRAINT_VALUE_METHODS,
		}
	}
}

impl From<&FznsoDecisionIdx> for FznsoValueRef<'_> {
	fn from(value: &FznsoDecisionIdx) -> Self {
		extern "C" fn kind(_: &FznsoValue) -> FznsoValueKind {
			FznsoValueKind::FznsoValueDecision
		}
		extern "C" fn get_decision(val: &FznsoValue) -> FznsoDecisionIdx {
			// SAFETY: The value is a &'a FznsoDecisionIdx, but was transmuted to a
			// &'a FznsoValue to fit [`FznsoValueRef`].
			let val: &FznsoDecisionIdx = unsafe { mem::transmute(val) };
			*val
		}
		const FZNSO_DECISION_VALUE_METHODS: &FznsoValueMethods = &FznsoValueMethods {
			kind,
			as_decision: get_decision,
			..*FznsoValueRef::FZNSO_DEFAULT_VALUE_METHODS
		};

		// SAFETY: The value is a &'a FznsoDecisionIdx, and will never be used
		// without transmuting back.
		let data: &FznsoValue = unsafe { mem::transmute(value) };
		Self {
			data,
			methods: FZNSO_DECISION_VALUE_METHODS,
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

impl<'a> TryFrom<&'a FznsoValueRef<'a>> for &'a ffi::CStr {
	type Error = ();

	fn try_from(value: &'a FznsoValueRef<'a>) -> Result<Self, Self::Error> {
		if value.kind() == FznsoValueKind::FznsoValueString {
			Ok(value.get_string())
		} else {
			Err(())
		}
	}
}

impl TryFrom<&FznsoValueRef<'_>> for ffi::CString {
	type Error = ();

	fn try_from(value: &FznsoValueRef<'_>) -> Result<Self, Self::Error> {
		if value.kind() == FznsoValueKind::FznsoValueString {
			Ok(value.get_string().to_owned())
		} else {
			Err(())
		}
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

fznso_value_impls! { i64, f64, bool, FznsoConstraintIdx, FznsoDecisionIdx, ffi::CString }
