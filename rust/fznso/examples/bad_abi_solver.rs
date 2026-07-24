//! A solver that reports an incompatible ABI version, built as a `cdylib`
//! example so the loaders' version check can be tested.
//!
//! A loader must reject it before resolving any other entry point, so reporting
//! the version is all this library needs to export.

/// Report an ABI version the loader will reject.
///
/// The built cdylib is `libbad_abi_solver.*`, so its name is `bad_abi_solver`
/// and the loader looks up this symbol.
#[unsafe(export_name = "fznso_bad_abi_solver_abi_version")]
pub extern "C" fn abi_version() -> u32 {
	fznso_types::FZNSO_ABI_VERSION + 1
}
