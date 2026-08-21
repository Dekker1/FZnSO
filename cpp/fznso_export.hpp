// FZnSO — header-only C++17 solver export.
//
// Implement a solver by subclassing `fznso::Solver`, then expand
// `FZNSO_EXPORT_SOLVER(YourSolver, yoursolver)` once in a `cdylib` / shared-library
// build to expose it across the interface. The second argument is the library's
// name, which prefixes every entry point (`fznso_yoursolver_solver_run`, …).
// Everything except those thirteen entry points is done by templates, which the
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
		[](const FznsoSolution* p, FznsoStr name) noexcept {
			return reinterpret_cast<const S*>(p)->S::statistic(to_view(name)).raw();
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
///
/// The sink may be *absent*, meaning nobody is listening. Check `wanted()` before
/// doing any work to produce a message: the point of the absent case is that a
/// solver skips formatting diagnostics rather than handing them to something that
/// discards them.
class MessageSink {
public:
	MessageSink(void* context, void (*on_message)(void*, FznsoStr, FznsoValueRef))
		: context_(context), on_message_(on_message) {}

	/// Whether anything is listening. When false, `message` does nothing.
	bool wanted() const { return on_message_ != nullptr; }

	/// Report a message. `value` must outlive this call. Does nothing when no
	/// listener is attached, so guard the *construction* with `wanted()`.
	void message(std::string_view scope, Value value) {
		if (on_message_ != nullptr) {
			on_message_(context_, detail::from_view(scope), value.raw());
		}
	}

private:
	void* context_;
	void (*on_message_)(void*, FznsoStr, FznsoValueRef);
};

/// Polled by a solver to ask whether the caller wants the search abandoned.
///
/// Answering `true` means stop as soon as convenient and return
/// `Status::Kind::Incomplete`. A solver is expected to poll once before starting,
/// again after each reported solution (so a caller can stop from inside its own
/// callback), and otherwise as often as is reasonable so that a cancel is acted
/// on promptly. The answer is monotone — once `true` it stays `true` — so it may
/// be cached.
///
/// It is cheap and never blocks, but it *is* an indirect call, so polling every
/// node is wasteful; once per restart or node batch is the intent. It is safe to
/// call from any thread, including concurrently from several at once.
///
/// The signal may be *absent*, meaning the caller will never ask to stop;
/// `requested()` is then constantly false and `pollable()` says so, letting a
/// solver drop the check from its loop entirely. A caller that only wants a
/// deadline should set the `time_limit` option instead of polling a clock here,
/// since a solver told its deadline up front can plan its search around it.
class StopSignal {
public:
	StopSignal(void* context, bool (*should_stop)(void*))
		: context_(context), should_stop_(should_stop) {}

	/// Whether there is anything to poll at all.
	bool pollable() const { return should_stop_ != nullptr; }

	/// Whether the caller has asked the search to stop.
	bool requested() const { return should_stop_ != nullptr && should_stop_(context_); }

private:
	void* context_;
	bool (*should_stop_)(void*);
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

	/// The current value of a named solver-level statistic, or absent if unknown.
	///
	/// Only statistics this solver declares with the `solver` flag set in
	/// `statistic_list()` are readable here; statistics carrying only the
	/// `solution` flag are reported through `SolutionSource::statistic` instead.
	/// A solver with no solver-level statistics need not override this.
	virtual Value statistic(std::string_view /*name*/) const { return Value{}; }

	/// Run against `model`, reporting solutions and messages through the sinks.
	///
	/// Threading contract, if the solver searches on several threads: the sinks
	/// must be driven *serially* — `SolutionSink::solution` and
	/// `MessageSink::message` may be called from any thread, but never two at
	/// once and never one while the other runs (funnel them through the thread
	/// that updates the incumbent, or a dedicated reporting thread). `model` is a
	/// read-only view for the duration of the call and may be queried
	/// concurrently from any number of threads. `stop` may be polled from any
	/// thread too, including several at once.
	virtual Status run(const Model& model, SolutionSink& solutions, MessageSink& messages,
	                   const StopSignal& stop) = 0;

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

/// The thunks that back the thirteen exported entry points for a solver `S`.
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

	static FznsoValueRef option_get(const FznsoSolver* p, FznsoStr name) {
		return at(p).S::option_get(detail::to_view(name)).raw();
	}

	static FznsoValueRef statistic(const FznsoSolver* p, FznsoStr name) {
		return at(p).S::statistic(detail::to_view(name)).raw();
	}

	static bool option_set(FznsoSolver* p, FznsoStr name, FznsoValueRef value) {
		try {
			std::optional<std::string> error =
				at(p).S::option_set(detail::to_view(name), Value{value});
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

	static void read_error(FznsoSolver* p, void* context, void (*read)(void*, FznsoStr)) {
		read(context, str(base(p).last_error_));
	}

	static FznsoStatus run(FznsoSolver* p, FznsoModelRef model, void* context,
	                       void (*on_solution)(void*, FznsoSolutionRef),
	                       void (*on_message)(void*, FznsoStr, FznsoValueRef),
	                       bool (*should_stop)(void*)) {
		try {
			ModelRefAdapter adapter{model};
			SolutionSink solutions{context, on_solution};
			MessageSink messages{context, on_message};
			StopSignal stop{context, should_stop};
			Status status = at(p).S::run(adapter, solutions, messages, stop);
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

/// Marks the exported entry points for export from a dynamically loadable library.
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

/// Define the thirteen interface entry points for a `fznso::Solver` subclass.
///
/// Expand this exactly once, at namespace scope, in the dynamically loadable library that is
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
		const FznsoSolver* solver, FznsoStr ident) noexcept {                                      \
		return ::fznso::detail::Export<SolverType>::option_get(solver, ident);                     \
	}                                                                                              \
	FZNSO_EXPORTED bool FZNSO_SYMBOL(Name, _solver_option_set)(                                    \
		FznsoSolver* solver, FznsoStr ident, FznsoValueRef value) noexcept {                       \
		return ::fznso::detail::Export<SolverType>::option_set(solver, ident, value);              \
	}                                                                                              \
	FZNSO_EXPORTED FznsoValueRef FZNSO_SYMBOL(Name, _solver_statistic)(                            \
		const FznsoSolver* solver, FznsoStr ident) noexcept {                                      \
		return ::fznso::detail::Export<SolverType>::statistic(solver, ident);                      \
	}                                                                                              \
	FZNSO_EXPORTED void FZNSO_SYMBOL(Name, _solver_read_error)(                                    \
		FznsoSolver* solver, void* context, void (*read_error)(void*, FznsoStr)) noexcept {        \
		::fznso::detail::Export<SolverType>::read_error(solver, context, read_error);              \
	}                                                                                              \
	FZNSO_EXPORTED FznsoStatus FZNSO_SYMBOL(Name, _solver_run)(                                    \
		FznsoSolver* solver, FznsoModelRef model, void* context,                                   \
		void (*on_solution)(void*, FznsoSolutionRef),                                              \
		void (*on_message)(void*, FznsoStr, FznsoValueRef),                                        \
		bool (*should_stop)(void*)) noexcept {                                                     \
		return ::fznso::detail::Export<SolverType>::run(solver, model, context, on_solution,       \
		                                                on_message, should_stop);                  \
	}                                                                                              \
	static_assert(true, "require a trailing semicolon")

#endif // FZNSO_EXPORT_HPP
