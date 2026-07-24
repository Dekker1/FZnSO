// FZnSO — header-only C++17 solver export.
//
// Implement a solver by subclassing `fznso::Solver`, then expand
// `FZNSO_EXPORT_SOLVER(YourSolver, yoursolver)` once in a `cdylib`/shared-object
// build to expose it across the interface. The second argument is the library's
// name, which prefixes every entry point (`fznso_yoursolver_solver_run`, …).
// Everything except those twelve entry points is done by templates, which the
// macro forwards to.
//
// The consumer side — building models and loading solvers — is in `fznso.hpp`.

#ifndef FZNSO_EXPORT_HPP
#define FZNSO_EXPORT_HPP

#include "fznso.hpp"

#include <exception>
#include <optional>
#include <string>
#include <string_view>
#include <utility>

namespace fznso {

// ---------------------------------------------------------------------------
// Solutions a solver reports
// ---------------------------------------------------------------------------

/// The values a solver publishes for one solution.
///
/// Subclass this and hand an instance to `SolutionSink::solution`. The returned
/// values borrow storage the source owns, which must outlive the `solution`
/// call.
class SolutionSource {
public:
	virtual ~SolutionSource() = default;

	/// The value assigned to a decision variable.
	virtual Value value(Decision decision) const = 0;
	/// A named statistic for this solution, or absent if unknown.
	virtual Value statistic(std::string_view /*name*/) const { return Value{}; }
};

namespace detail {

/// The solution vtable specialised to a concrete `SolutionSource` type `S`.
///
/// The thunks cast to `S` and name its methods, so they dispatch directly.
template <class S>
const FznsoSolutionMethods& solution_methods() {
	static const FznsoSolutionMethods table = {
		[](const FznsoSolution* p, std::size_t i) noexcept {
			return reinterpret_cast<const S*>(p)->S::value(Decision{i}).raw();
		},
		[](const FznsoSolution* p, const char* name) noexcept {
			return reinterpret_cast<const S*>(p)->S::statistic(std::string_view{name}).raw();
		},
	};
	return table;
}

} // namespace detail

/// Receives the solutions a solver finds during a run.
class SolutionSink {
public:
	SolutionSink(void* context, void (*on_solution)(void*, FznsoSolutionRef))
		: context_(context), on_solution_(on_solution) {}

	/// Report a solution. `src` must outlive this call.
	template <class S>
	void solution(const S& src) {
		static_assert(std::is_base_of<SolutionSource, S>::value,
		              "a solution must derive from fznso::SolutionSource");
		on_solution_(context_, FznsoSolutionRef{reinterpret_cast<const FznsoSolution*>(&src),
		                                        &detail::solution_methods<S>()});
	}

private:
	void* context_;
	void (*on_solution_)(void*, FznsoSolutionRef);
};

/// Receives the non-fatal diagnostics a solver reports during a run.
class MessageSink {
public:
	MessageSink(void* context, void (*on_message)(void*, FznsoStr, FznsoValueRef))
		: context_(context), on_message_(on_message) {}

	/// Report a message. `value` must outlive this call.
	void message(std::string_view scope, Value value) {
		on_message_(context_, detail::from_view(scope), value.raw());
	}

private:
	void* context_;
	void (*on_message_)(void*, FznsoStr, FznsoValueRef);
};

// ---------------------------------------------------------------------------
// Solver
// ---------------------------------------------------------------------------

namespace detail {
template <class S>
struct Export;
} // namespace detail

/// The interface a solver implements.
///
/// Subclass this, implement the three instance methods, and expand
/// `FZNSO_EXPORT_SOLVER(YourSolver, yourname)`. The capability lists are `static`
/// and default to empty; hide the ones your solver supports.
class Solver {
public:
	virtual ~Solver() = default;

	/// The current value of a named option. Borrows storage the solver owns.
	virtual Value option_get(std::string_view name) const = 0;

	/// Set a named option. Return a message to reject the value; the consumer
	/// receives it. Return `std::nullopt` on success.
	virtual std::optional<std::string> option_set(std::string_view name, Value value) = 0;

	/// Run against `model`, reporting solutions and messages through the sinks.
	virtual Status run(const Model& model, SolutionSink& solutions, MessageSink& messages) = 0;

	/// The constraints this solver accepts. Empty by default.
	static FznsoConstraintList constraint_list() { return {0, nullptr}; }
	/// The decision-variable types this solver accepts. Empty by default.
	static FznsoTypeList decision_list() { return {0, nullptr}; }
	/// The objective strategies this solver supports. Empty by default.
	static FznsoObjectiveList objective_list() { return {0, nullptr}; }
	/// The options this solver accepts. Empty by default.
	static FznsoOptionList option_list() { return {0, nullptr}; }
	/// The statistics this solver reports. Empty by default.
	static FznsoStatisticList statistic_list() { return {0, nullptr}; }

private:
	template <class S>
	friend struct detail::Export;

	/// The most recent failure message, surfaced through `fznso_solver_read_error`.
	std::string last_error_;
};

namespace detail {

/// The thunks that back the twelve exported entry points for a solver `S`.
///
/// The solver methods are named through `S::`, so they dispatch directly rather
/// than through the virtual interface — consistent with the model, value and
/// solution thunks (though the solver methods are cold, run once per solve).
template <class S>
struct Export {
	static_assert(std::is_base_of<Solver, S>::value, "a solver must derive from fznso::Solver");

	/// The message of the exception currently being handled.
	static std::string current_error() {
		try {
			throw;
		} catch (const std::exception& e) {
			return e.what();
		} catch (...) {
			return "unknown exception";
		}
	}

	static S& at(FznsoSolver* p) { return *reinterpret_cast<S*>(p); }
	static const S& at(const FznsoSolver* p) { return *reinterpret_cast<const S*>(p); }
	static Solver& base(FznsoSolver* p) { return at(p); }

	static FznsoSolver* create() { return reinterpret_cast<FznsoSolver*>(new S()); }
	static void free(FznsoSolver* p) { delete reinterpret_cast<S*>(p); }

	static FznsoValueRef option_get(const FznsoSolver* p, const char* name) {
		return at(p).S::option_get(std::string_view{name}).raw();
	}

	static bool option_set(FznsoSolver* p, const char* name, FznsoValueRef value) {
		try {
			std::optional<std::string> error =
				at(p).S::option_set(std::string_view{name}, Value{value});
			if (error.has_value()) {
				base(p).last_error_ = std::move(*error);
				return false;
			}
			return true;
		} catch (...) {
			base(p).last_error_ = current_error();
			return false;
		}
	}

	static void read_error(FznsoSolver* p, void* context,
	                       void (*read)(void*, const char*)) {
		read(context, base(p).last_error_.c_str());
	}

	static FznsoStatus run(FznsoSolver* p, FznsoModelRef model, void* context,
	                       void (*on_solution)(void*, FznsoSolutionRef),
	                       void (*on_message)(void*, FznsoStr, FznsoValueRef)) {
		try {
			ModelRefAdapter adapter{model};
			SolutionSink solutions{context, on_solution};
			MessageSink messages{context, on_message};
			Status status = at(p).S::run(adapter, solutions, messages);
			switch (status.kind) {
			case Status::Kind::Complete:
				return FznsoComplete;
			case Status::Kind::Incomplete:
				return FznsoIncomplete;
			case Status::Kind::Error:
				base(p).last_error_ = std::move(status.error);
				return FznsoError;
			}
			return FznsoComplete;
		} catch (...) {
			base(p).last_error_ = current_error();
			return FznsoError;
		}
	}
};

} // namespace detail

} // namespace fznso

/// Marks the exported entry points for export from a shared object.
#if defined(_WIN32)
#define FZNSO_EXPORTED extern "C" __declspec(dllexport)
#else
#define FZNSO_EXPORTED extern "C" __attribute__((visibility("default")))
#endif

/// Paste tokens into one symbol name. The extra level of indirection lets the
/// arguments expand before they are pasted.
#define FZNSO_CONCAT_(a, b) a##b
#define FZNSO_CONCAT(a, b) FZNSO_CONCAT_(a, b)

/// Build an exported entry-point name `fznso_<Name><Suffix>`.
#define FZNSO_SYMBOL(Name, Suffix) FZNSO_CONCAT(fznso_, FZNSO_CONCAT(Name, Suffix))

/// Define the twelve interface entry points for a `fznso::Solver` subclass.
///
/// Expand this exactly once, at namespace scope, in the shared object that is
/// the solver. `Name` is the library's name: every entry point is named
/// `fznso_<Name>_solver_run` and so on, so that several solvers can be linked
/// into one binary without their entry points colliding — with each other or
/// with the library each one wraps. The shared `fznso_` marker keeps them
/// greppable and out of any wrapped library's namespace. `Name` must match the
/// base name of the library file (e.g. `gecode` for `libgecode.so`) so a loader
/// can recover it.
///
/// The entry points are `noexcept`: an exception must never cross the C
/// interface. `run` and `option_set` convert one into an error the caller can
/// read; the rest have no error channel, so a leaked exception terminates the
/// process rather than causing undefined behaviour.
#define FZNSO_EXPORT_SOLVER(SolverType, Name)                                                      \
	FZNSO_EXPORTED std::uint32_t FZNSO_SYMBOL(Name, _abi_version)(void) noexcept {                  \
		return FZNSO_ABI_VERSION;                                                                   \
	}                                                                                              \
	FZNSO_EXPORTED FznsoConstraintList FZNSO_SYMBOL(Name, _constraint_list)(void) noexcept {       \
		return SolverType::constraint_list();                                                      \
	}                                                                                              \
	FZNSO_EXPORTED FznsoTypeList FZNSO_SYMBOL(Name, _decision_list)(void) noexcept {               \
		return SolverType::decision_list();                                                        \
	}                                                                                              \
	FZNSO_EXPORTED FznsoObjectiveList FZNSO_SYMBOL(Name, _objective_list)(void) noexcept {         \
		return SolverType::objective_list();                                                       \
	}                                                                                              \
	FZNSO_EXPORTED FznsoOptionList FZNSO_SYMBOL(Name, _option_list)(void) noexcept {               \
		return SolverType::option_list();                                                          \
	}                                                                                              \
	FZNSO_EXPORTED FznsoStatisticList FZNSO_SYMBOL(Name, _statistic_list)(void) noexcept {         \
		return SolverType::statistic_list();                                                       \
	}                                                                                              \
	FZNSO_EXPORTED FznsoSolver* FZNSO_SYMBOL(Name, _solver_create)(void) noexcept {                \
		return ::fznso::detail::Export<SolverType>::create();                                      \
	}                                                                                              \
	FZNSO_EXPORTED void FZNSO_SYMBOL(Name, _solver_free)(FznsoSolver* solver) noexcept {           \
		::fznso::detail::Export<SolverType>::free(solver);                                         \
	}                                                                                              \
	FZNSO_EXPORTED FznsoValueRef FZNSO_SYMBOL(Name, _solver_option_get)(                           \
		const FznsoSolver* solver, const char* ident) noexcept {                                   \
		return ::fznso::detail::Export<SolverType>::option_get(solver, ident);                     \
	}                                                                                              \
	FZNSO_EXPORTED bool FZNSO_SYMBOL(Name, _solver_option_set)(                                    \
		FznsoSolver* solver, const char* ident, FznsoValueRef value) noexcept {                    \
		return ::fznso::detail::Export<SolverType>::option_set(solver, ident, value);              \
	}                                                                                              \
	FZNSO_EXPORTED void FZNSO_SYMBOL(Name, _solver_read_error)(                                    \
		FznsoSolver* solver, void* context, void (*read_error)(void*, const char*)) noexcept {     \
		::fznso::detail::Export<SolverType>::read_error(solver, context, read_error);              \
	}                                                                                              \
	FZNSO_EXPORTED FznsoStatus FZNSO_SYMBOL(Name, _solver_run)(                                    \
		FznsoSolver* solver, FznsoModelRef model, void* context,                                   \
		void (*on_solution)(void*, FznsoSolutionRef),                                              \
		void (*on_message)(void*, FznsoStr, FznsoValueRef)) noexcept {                             \
		return ::fznso::detail::Export<SolverType>::run(solver, model, context, on_solution,       \
		                                                on_message);                               \
	}                                                                                              \
	static_assert(true, "require a trailing semicolon")

#endif // FZNSO_EXPORT_HPP
