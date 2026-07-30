// Builds a model in C++, dynamically loads a solver library and runs it.
//
// The model, the value vtables and the callbacks are all exercised on the C++
// side here, so this checks that a model built through these bindings is
// readable by a solver across the interface.

#include "fznso.hpp"

#include <atomic>
#include <cassert>
#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <string>
#include <vector>

namespace {

/// Abort with a message unless `condition` holds.
void check(bool condition, const char* what) {
	if (!condition) {
		std::fprintf(stderr, "FAILED: %s\n", what);
		std::abort();
	}
}

/// Every decision in these fixtures is an integer variable.
constexpr FznsoType INT_VAR = fznso::decision_type(FznsoTypeBaseInt);

/// Set an environment variable. `setenv` is POSIX; Windows spells it `_putenv_s`.
void set_env(const char* name, const char* value) {
#if defined(_WIN32)
	(void)::_putenv_s(name, value);
#else
	(void)::setenv(name, value, 1);
#endif
}

/// Remove an environment variable; on Windows an empty value unsets it.
void unset_env(const char* name) {
#if defined(_WIN32)
	(void)::_putenv_s(name, "");
#else
	(void)::unsetenv(name);
#endif
}

/// A model with two integer decisions whose domains start at 3 and 10.
void build(fznso::LayeredModel& model) {
	fznso::Decision a = model.add_decision(fznso::OwnedValue::int_range(3, 20), "a");
	model.add_decision(fznso::OwnedValue::int_range(10, 20), "b");
	model.add_constraint("int_le", {fznso::OwnedValue{std::int64_t{0}}, fznso::OwnedValue{a}});
	model.set_objective("minimize", fznso::OwnedValue{a});
}

} // namespace

int main(int argc, char** argv) {
	check(argc == 2, "usage: test_consumer <path-to-solver-library>");
	fznso::Library library{argv[1]};
	fznso::DynSolver solver = library.create_solver();

	// Options round trip across the opaque solver handle.
	check(solver.option_get("solution_limit").as_int() == 1, "default solution_limit is 1");
	fznso::OwnedValue three{std::int64_t{3}};
	check(!solver.option_set("solution_limit", fznso::Value{three}).has_value(),
	      "setting a known option succeeds");
	check(solver.option_get("solution_limit").as_int() == 3, "solution_limit round trips");

	// An unknown option is reported rather than silently accepted.
	check(solver.option_set("no_such_option", fznso::Value{three}).has_value(),
	      "unknown option is rejected");

	// Solver-level statistics are read from the solver instance; a name that is
	// unknown, or declared solution-only, is absent rather than an error.
	check(solver.statistic("runs").as_int() == 0, "runs starts at zero");
	check(solver.statistic("no_such_statistic").kind() == FznsoValueAbsent,
	      "unknown statistic is absent");
	check(solver.statistic("solutions").kind() == FznsoValueAbsent,
	      "solution-only statistic is absent on the solver");

	// The declared metadata says where each statistic can be read from.
	FznsoStatisticList stats = library.statistics();
	check(stats.len == 2, "two statistics are declared");
	check(fznso::detail::to_view(stats.stats[0].ident) == "solutions" && stats.stats[0].solution &&
	          !stats.stats[0].solver,
	      "solutions is solution-scoped");
	check(fznso::detail::to_view(stats.stats[1].ident) == "runs" && !stats.stats[1].solution &&
	          stats.stats[1].solver,
	      "runs is solver-scoped");

	fznso::LayeredModel model;
	build(model);

	std::vector<std::pair<std::int64_t, std::int64_t>> solutions;
	std::vector<std::pair<std::string, std::string>> messages;
	fznso::Status status = solver.run(
		model,
		[&](const fznso::Solution& sol) {
			solutions.emplace_back(sol[fznso::Decision{0}].as_int(), sol[fznso::Decision{1}].as_int());
		},
		[&](std::string_view scope, const fznso::Value& value) {
			messages.emplace_back(std::string{scope}, std::string{value.as_string()});
		});

	check(status.complete(), "run completes");
	check(solutions.size() == 3, "three solutions");
	check(solutions[0] == std::make_pair(std::int64_t{3}, std::int64_t{10}), "first solution");
	check(solutions[2] == std::make_pair(std::int64_t{5}, std::int64_t{10}), "third solution");
	check(messages.size() == 1, "one message");
	check(messages[0].first == "warn", "message scope");
	check(messages[0].second == "ignoring 1 constraint(s)", "message payload");

	// A model published across the interface and read back must agree with the
	// original, including the `defines` link that is carried in the handle word.
	fznso::LayeredModel linked;
	fznso::Decision y = linked.add_decision(fznso::OwnedValue::int_range(0, 9), "y");
	fznso::Constraint c =
		linked.add_constraint("defines_y", {fznso::OwnedValue{y}}, y,
	                          {fznso::OwnedAnnotation{"defines_var", {fznso::OwnedValue{y}}}});
	fznso::ModelRefAdapter back{fznso::model_ref(linked)};
	check(back.constraint_count() == 1, "adapter sees the constraint");
	check(back.constraint_ident(c) == "defines_y", "adapter reads the ident");
	check(back.constraint_defines(c) == std::optional<fznso::Decision>{y}, "defines round trips");
	check(!back.constraint_defines(fznso::Constraint{0}).has_value() == false, "defines is present");
	check(back.decision_name(y) == std::optional<std::string_view>{"y"}, "name round trips");
	check(back.constraint_annotation_count(c) == 1, "adapter sees the annotation");
	check(back.constraint_annotation(c, 0).ident() == "defines_var", "annotation ident");
	check(back.constraint_annotation(c, 0)[0].as_decision() == y, "annotation argument");

	// A model with no `defines` reports absent rather than a bogus decision.
	fznso::LayeredModel plain;
	plain.add_decision(fznso::OwnedValue::int_range(0, 1), "z");
	fznso::Constraint pc = plain.add_constraint("bool_eq", {});
	fznso::ModelRefAdapter plain_back{fznso::model_ref(plain)};
	check(!plain_back.constraint_defines(pc).has_value(), "absent defines round trips");

	// The variant resolves kind and payload together.
	fznso::LayeredModel domain_model;
	fznso::Decision d = domain_model.add_decision(fznso::OwnedValue::int_range(-5, 5), "x");
	FznsoModelRef raw = fznso::model_ref(domain_model);
	fznso::Value dom = fznso::ModelRefAdapter{raw}.decision_domain(d);
	fznso::ValueVariant domain = dom.variant();
	check(std::holds_alternative<fznso::IntRanges>(domain), "domain is an int set");
	fznso::IntRanges ranges = std::get<fznso::IntRanges>(domain);
	check(ranges.size() == 1 && ranges[0] == (fznso::Range<std::int64_t>{-5, 5}), "domain range");

	// Indexing a value is list indexing; `[]` on a list argument reads elements.
	fznso::LayeredModel list_model;
	list_model.add_constraint("array_int_element",
	                          {std::vector<fznso::OwnedValue>{std::int64_t{10}, std::int64_t{20},
	                                                          std::int64_t{30}}});
	fznso::ModelRefAdapter list_back{fznso::model_ref(list_model)};
	fznso::Value arg = list_back.constraint_argument(fznso::Constraint{0}, 0);
	check(std::holds_alternative<fznso::ValueList>(arg.variant()), "argument is a list");
	check(arg.size() == 3, "list length");
	check(arg[1].as_int() == 20, "value[] indexes list elements");

	// An optional argument: present takes the value's kind, empty is absent.
	fznso::LayeredModel opt_model;
	opt_model.add_constraint("opt_int", {fznso::OwnedValue{std::optional<std::int64_t>{42}},
	                                     fznso::OwnedValue{std::optional<std::int64_t>{}}});
	fznso::ModelRefAdapter opt_back{fznso::model_ref(opt_model)};
	fznso::Value present = opt_back.constraint_argument(fznso::Constraint{0}, 0);
	fznso::Value empty = opt_back.constraint_argument(fznso::Constraint{0}, 1);
	check(present.kind() == FznsoValueInt && present.as_int() == 42, "present optional is its value");
	check(empty.kind() == FznsoValueAbsent, "empty optional is absent");

	// A run can be abandoned part-way: `should_stop` returning true after the
	// first solution ends the search, and the run reports Incomplete.
	{
		fznso::OwnedValue five{std::int64_t{5}};
		check(!solver.option_set("solution_limit", fznso::Value{five}).has_value(),
		      "raising solution_limit succeeds");

		std::atomic<bool> stop{false};
		int seen = 0;
		fznso::Status stopped = solver.run(
			model, [&](const fznso::Solution&) { ++seen; stop.store(true); },
			[](std::string_view, const fznso::Value&) {}, [&] { return stop.load(); });
		check(seen == 1, "the solver stopped after the first solution");
		check(stopped.kind == fznso::Status::Kind::Incomplete, "a stopped run is incomplete");

		// Already asked to stop before the run begins: no search at all.
		seen = 0;
		fznso::Status none = solver.run(
			model, [&](const fznso::Solution&) { ++seen; },
			[](std::string_view, const fznso::Value&) {}, [] { return true; });
		check(seen == 0, "a pre-stopped run does not search");
		check(none.kind == fznso::Status::Kind::Incomplete, "a pre-stopped run is incomplete");
	}

	// The same solver is reachable by name off the search path, not just by path.
	// Stage a copy under an `fznso` directory pointed at by FZNSO_SOLVER_PATH.
	{
		namespace fs = std::filesystem;
		std::string derived{fznso::detail::name_from_path(argv[1])};
		fs::path dir = fs::temp_directory_path() / "fznso-consumer-find";
		fs::remove_all(dir);
		fs::create_directories(dir);
		fs::copy_file(argv[1], dir / fs::path{argv[1]}.filename(),
		              fs::copy_options::overwrite_existing);
		set_env("FZNSO_SOLVER_PATH", dir.string().c_str());

		bool listed = false;
		for (const fznso::Library::Discovered& d : fznso::Library::discover()) {
			listed = listed || d.name == derived;
		}
		check(listed, "the staged solver is discovered by name");

		std::unique_ptr<fznso::Library> lib = fznso::Library::find(derived);
		fznso::DynSolver solver = lib->create_solver();
		check(solver.option_get("solution_limit").as_int() == 1, "loaded-by-name solver runs");

		// The staged example carries no version, so an empty request finds it and
		// a specific one does not.
		check(fznso::Library::find_version(derived, "") != nullptr, "empty version finds it");
		bool version_threw = false;
		try {
			fznso::Library::find_version(derived, "9");
		} catch (const std::runtime_error&) {
			version_threw = true;
		}
		check(version_threw, "a version that is not installed throws");

		bool threw = false;
		try {
			fznso::Library::find("definitely_absent");
		} catch (const std::runtime_error&) {
			threw = true;
		}
		check(threw, "an absent solver name throws");

		unset_env("FZNSO_SOLVER_PATH");
		fs::remove_all(dir);
	}

	std::printf("all C++ consumer checks passed\n");
	return 0;
}
