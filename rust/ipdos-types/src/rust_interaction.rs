use std::{ffi, mem};

use crate::{
	IpdosConstraintIdx, IpdosDecisionIdx, IpdosValue, IpdosValueKind, IpdosValueMethods,
	IpdosValueRef,
};

macro_rules! ipdos_value_impls {
	(vec $t:ty) => {
		impl<'a> From<&'a Vec<$t>> for IpdosValueRef<'a> {
			fn from(value: &'a Vec<$t>) -> Self {
				extern "C" fn kind(_: &IpdosValue) -> IpdosValueKind {
					IpdosValueKind::IpdosValueList
				}
				extern "C" fn len(val: &IpdosValue) -> usize {
					let val: &Vec<$t> = unsafe { mem::transmute(val) };
					val.len()
				}
				extern "C" fn get_element<'a>(val: &'a IpdosValue, idx: usize) -> IpdosValueRef<'a> {
					let val: &Vec<$t> = unsafe { mem::transmute(val) };
					IpdosValueRef::from(&val[idx])
				}
				const IPDOS_LIST_VALUE_METHODS: &'static IpdosValueMethods = &IpdosValueMethods {
					kind,
					len,
					get_element,
					..*IpdosValueRef::IPDOS_DEFAULT_VALUE_METHODS
				};

				Self {
					data: unsafe { mem::transmute(value) },
					methods: IPDOS_LIST_VALUE_METHODS,
				}
			}
		}

		impl TryFrom<&IpdosValueRef<'_>> for Vec<$t> {
			type Error = ();

			fn try_from(value: &IpdosValueRef<'_>) -> Result<Self, Self::Error> {
				if value.kind() != IpdosValueKind::IpdosValueList {
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
		impl From<&Option<$t>> for IpdosValueRef<'_> {
			fn from(value: &Option<$t>) -> Self {
				match value {
					Some(t) => IpdosValueRef::from(t),
					None => IpdosValueRef::from(&()),
				}
			}
		}

		impl TryFrom<&IpdosValueRef<'_>> for Option<$t> {
			type Error = ();

			fn try_from(value: &IpdosValueRef<'_>) -> Result<Self, Self::Error> {
				if value.kind() == IpdosValueKind::IpdosValueAbsent {
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
			ipdos_value_impls!{vec $t}
			ipdos_value_impls!{opt $t}
			ipdos_value_impls!{vec Option<$t>}
		)+
	};
}

impl IpdosValueRef<'_> {
	const IPDOS_DEFAULT_VALUE_METHODS: &'static IpdosValueMethods = &IpdosValueMethods {
		kind: Self::absent_kind,
		len: Self::no_call,
		get_decision: Self::no_call,
		get_constraint: Self::no_call,
		get_int: Self::no_call,
		get_float: Self::no_call,
		get_string: Self::no_call,
		get_bool: Self::no_call,
		get_range_int: Self::no_call_idx,
		get_range_float: Self::no_call_idx,
		get_element: Self::no_call_idx,
	};

	extern "C" fn absent_kind(_: &IpdosValue) -> IpdosValueKind {
		IpdosValueKind::IpdosValueAbsent
	}
	extern "C" fn no_call<T>(_: &IpdosValue) -> T {
		panic!("This value implementation does not provide this callback");
	}
	extern "C" fn no_call_idx<T>(v: &IpdosValue, _: usize) -> T {
		Self::no_call(v)
	}

	/// Rust call to [`IpdosValueMethods::kind`] with the
	/// [`IpdosValueRef::data`] attribute.
	pub fn kind(&self) -> IpdosValueKind {
		(self.methods.kind)(self.data)
	}

	/// Rust call to [`IpdosValueMethods::len`] with the [`IpdosValueRef::data`]
	/// attribute.
	pub fn len(&self) -> usize {
		(self.methods.len)(self.data)
	}

	/// Rust call to [`IpdosValueMethods::get_decision`] with the
	/// [`IpdosValueRef::data`] attribute.
	pub fn get_decision(&self) -> IpdosDecisionIdx {
		(self.methods.get_decision)(self.data)
	}

	/// Rust call to [`IpdosValueMethods::get_constraint`] with the
	/// [`IpdosValueRef::data`] attribute.
	pub fn get_constraint(&self) -> IpdosConstraintIdx {
		(self.methods.get_constraint)(self.data)
	}

	/// Rust call to [`IpdosValueMethods::get_int`] with the
	/// [`IpdosValueRef::data`] attribute.
	pub fn get_int(&self) -> i64 {
		(self.methods.get_int)(self.data)
	}

	/// Rust call to [`IpdosValueMethods::get_float`] with the
	/// [`IpdosValueRef::data`] attribute.
	pub fn get_float(&self) -> f64 {
		(self.methods.get_float)(self.data)
	}

	/// Rust call to [`IpdosValueMethods::get_string`] with the
	/// [`IpdosValueRef::data`] attribute.
	pub fn get_string(&self) -> &'_ ffi::CStr {
		let s = (self.methods.get_string)(self.data);
		unsafe { ffi::CStr::from_ptr(s) }
	}

	/// Rust call to [`IpdosValueMethods::get_bool`] with the
	/// [`IpdosValueRef::data`] attribute.
	pub fn get_bool(&self) -> bool {
		(self.methods.get_bool)(self.data)
	}

	/// Rust call to [`IpdosValueMethods::get_range_int`] with the
	/// [`IpdosValueRef::data`] attribute.
	pub fn get_range_int(&self, idx: usize) -> (i64, i64) {
		let [min, max] = (self.methods.get_range_int)(self.data, idx);
		(min, max)
	}

	/// Rust call to [`IpdosValueMethods::get_range_float`] with the
	/// [`IpdosValueRef::data`] attribute.
	pub fn get_range_float(&self, idx: usize) -> (f64, f64) {
		let [min, max] = (self.methods.get_range_float)(self.data, idx);
		(min, max)
	}

	/// Rust call to [`IpdosValueMethods::get_element`] with the
	/// [`IpdosValueRef::data`] attribute.
	pub fn get_element(&self, idx: usize) -> IpdosValueRef<'_> {
		(self.methods.get_element)(self.data, idx)
	}
}

impl From<&()> for IpdosValueRef<'_> {
	fn from(value: &()) -> Self {
		Self {
			data: unsafe { mem::transmute(value) },
			methods: Self::IPDOS_DEFAULT_VALUE_METHODS,
		}
	}
}

impl From<&bool> for IpdosValueRef<'_> {
	fn from(value: &bool) -> Self {
		extern "C" fn kind(_: &IpdosValue) -> IpdosValueKind {
			IpdosValueKind::IpdosValueBool
		}
		extern "C" fn get_bool(val: &IpdosValue) -> bool {
			let val: &bool = unsafe { mem::transmute(val) };
			*val
		}
		const IPDOS_BOOL_VALUE_METHODS: &'static IpdosValueMethods = &IpdosValueMethods {
			kind,
			get_bool,
			..*IpdosValueRef::IPDOS_DEFAULT_VALUE_METHODS
		};

		Self {
			data: unsafe { mem::transmute(value) },
			methods: IPDOS_BOOL_VALUE_METHODS,
		}
	}
}

impl From<&f64> for IpdosValueRef<'_> {
	fn from(value: &f64) -> Self {
		extern "C" fn kind(_: &IpdosValue) -> IpdosValueKind {
			IpdosValueKind::IpdosValueInt
		}
		extern "C" fn get_float(val: &IpdosValue) -> f64 {
			let val: &f64 = unsafe { mem::transmute(val) };
			*val
		}
		const IPDOS_F64_VALUE_METHODS: &'static IpdosValueMethods = &IpdosValueMethods {
			kind,
			get_float,
			..*IpdosValueRef::IPDOS_DEFAULT_VALUE_METHODS
		};

		Self {
			data: unsafe { mem::transmute(value) },
			methods: IPDOS_F64_VALUE_METHODS,
		}
	}
}

impl From<&i64> for IpdosValueRef<'_> {
	fn from(value: &i64) -> Self {
		extern "C" fn kind(_: &IpdosValue) -> IpdosValueKind {
			IpdosValueKind::IpdosValueInt
		}
		extern "C" fn get_int(val: &IpdosValue) -> i64 {
			let val: &i64 = unsafe { mem::transmute(val) };
			*val
		}
		const IPDOS_I64_VALUE_METHODS: &'static IpdosValueMethods = &IpdosValueMethods {
			kind,
			get_int,
			..*IpdosValueRef::IPDOS_DEFAULT_VALUE_METHODS
		};

		Self {
			data: unsafe { mem::transmute(value) },
			methods: IPDOS_I64_VALUE_METHODS,
		}
	}
}

impl From<&ffi::CStr> for IpdosValueRef<'_> {
	fn from(value: &ffi::CStr) -> Self {
		extern "C" fn kind(_: &IpdosValue) -> IpdosValueKind {
			IpdosValueKind::IpdosValueString
		}
		extern "C" fn get_string(val: &IpdosValue) -> *const ffi::c_char {
			let val: *const ffi::c_char = unsafe { mem::transmute(val) };
			val
		}
		const IPDOS_STR_VALUE_METHODS: &'static IpdosValueMethods = &IpdosValueMethods {
			kind,
			get_string,
			..*IpdosValueRef::IPDOS_DEFAULT_VALUE_METHODS
		};

		Self {
			data: unsafe { mem::transmute(value.as_ptr()) },
			methods: IPDOS_STR_VALUE_METHODS,
		}
	}
}

impl From<&ffi::CString> for IpdosValueRef<'_> {
	fn from(value: &ffi::CString) -> Self {
		value.as_c_str().into()
	}
}

impl From<&IpdosConstraintIdx> for IpdosValueRef<'_> {
	fn from(value: &IpdosConstraintIdx) -> Self {
		extern "C" fn kind(_: &IpdosValue) -> IpdosValueKind {
			IpdosValueKind::IpdosValueDecision
		}
		extern "C" fn get_constraint(val: &IpdosValue) -> IpdosConstraintIdx {
			let val: &IpdosConstraintIdx = unsafe { mem::transmute(val) };
			*val
		}
		const IPDOS_CONSTRAINT_VALUE_METHODS: &'static IpdosValueMethods = &IpdosValueMethods {
			kind,
			get_constraint,
			..*IpdosValueRef::IPDOS_DEFAULT_VALUE_METHODS
		};

		Self {
			data: unsafe { mem::transmute(value) },
			methods: IPDOS_CONSTRAINT_VALUE_METHODS,
		}
	}
}

impl From<&IpdosDecisionIdx> for IpdosValueRef<'_> {
	fn from(value: &IpdosDecisionIdx) -> Self {
		extern "C" fn kind(_: &IpdosValue) -> IpdosValueKind {
			IpdosValueKind::IpdosValueDecision
		}
		extern "C" fn get_decision(val: &IpdosValue) -> IpdosDecisionIdx {
			let val: &IpdosDecisionIdx = unsafe { mem::transmute(val) };
			*val
		}
		const IPDOS_DECISION_VALUE_METHODS: &'static IpdosValueMethods = &IpdosValueMethods {
			kind,
			get_decision,
			..*IpdosValueRef::IPDOS_DEFAULT_VALUE_METHODS
		};

		Self {
			data: unsafe { mem::transmute(value) },
			methods: IPDOS_DECISION_VALUE_METHODS,
		}
	}
}

impl TryFrom<&IpdosValueRef<'_>> for bool {
	type Error = ();

	fn try_from(value: &IpdosValueRef<'_>) -> Result<Self, Self::Error> {
		if (value.methods.kind)(value.data) == IpdosValueKind::IpdosValueBool {
			Ok((value.methods.get_bool)(value.data))
		} else {
			Err(())
		}
	}
}

impl TryFrom<&IpdosValueRef<'_>> for f64 {
	type Error = ();

	fn try_from(value: &IpdosValueRef<'_>) -> Result<Self, Self::Error> {
		if value.kind() == IpdosValueKind::IpdosValueFloat {
			Ok(value.get_float())
		} else {
			Err(())
		}
	}
}

impl TryFrom<&IpdosValueRef<'_>> for i64 {
	type Error = ();

	fn try_from(value: &IpdosValueRef<'_>) -> Result<Self, Self::Error> {
		if value.kind() == IpdosValueKind::IpdosValueInt {
			Ok(value.get_int())
		} else {
			Err(())
		}
	}
}

impl<'a> TryFrom<&'a IpdosValueRef<'a>> for &'a ffi::CStr {
	type Error = ();

	fn try_from(value: &'a IpdosValueRef<'a>) -> Result<Self, Self::Error> {
		if value.kind() == IpdosValueKind::IpdosValueString {
			Ok(value.get_string())
		} else {
			Err(())
		}
	}
}

impl TryFrom<&IpdosValueRef<'_>> for ffi::CString {
	type Error = ();

	fn try_from(value: &IpdosValueRef<'_>) -> Result<Self, Self::Error> {
		if value.kind() == IpdosValueKind::IpdosValueString {
			Ok(value.get_string().to_owned())
		} else {
			Err(())
		}
	}
}

impl TryFrom<&IpdosValueRef<'_>> for IpdosConstraintIdx {
	type Error = ();

	fn try_from(value: &IpdosValueRef<'_>) -> Result<Self, Self::Error> {
		if value.kind() == IpdosValueKind::IpdosValueConstraint {
			Ok(value.get_constraint())
		} else {
			Err(())
		}
	}
}

impl TryFrom<&IpdosValueRef<'_>> for IpdosDecisionIdx {
	type Error = ();

	fn try_from(value: &IpdosValueRef<'_>) -> Result<Self, Self::Error> {
		if value.kind() == IpdosValueKind::IpdosValueDecision {
			Ok(value.get_decision())
		} else {
			Err(())
		}
	}
}

ipdos_value_impls! { i64, f64, bool, IpdosConstraintIdx, IpdosDecisionIdx, ffi::CString }
