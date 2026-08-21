// A throwaway C++ solver, built as a dynamically loadable library so the export path can be
// exercised end to end.
//
// It is behaviourally identical to the Rust example solver, so the same
// `test_consumer` harness and assertions drive both: it assigns every decision
// the lower bound of its domain, shifts the first by the solution index, and
// warns that it ignores the constraints.

#include "fznso_export.hpp"

#include <cstdint>
#include <optional>
#include <string>
#include <string_view>
#include <vector>

namespace {

/// The assignment the solver reports for one solution.
class ExampleSolution final : public fznso::SolutionSource {
public:
	ExampleSolution(std::vector<std::int64_t> values, std::int64_t index)
		: values_(std::move(values)), index_(index) {}

	fznso::Value value(fznso::Decision decision) const override {
		return decision.index < values_.size() ? fznso::Value{values_[decision.index]}
		                                        : fznso::Value{};
	}
	fznso::Value statistic(std::string_view name) const override {
		return name == "solutions" ? fznso::Value{index_} : fznso::Value{};
	}

private:
	std::vector<std::int64_t> values_;
	std::int64_t index_;
};

/// A solver that reports domain lower bounds, ignoring all constraints.
class ExampleSolver final : public fznso::Solver {
public:
	fznso::Value option_get(std::string_view name) const override {
		return name == "solution_limit" ? fznso::Value{solution_limit_} : fznso::Value{};
	}

	fznso::Value statistic(std::string_view name) const override {
		return name == "runs" ? fznso::Value{runs_} : fznso::Value{};
	}

	/// One statistic of each scope: `solutions` is read from a solution, `runs`
	/// from the solver instance. A statistic may also set both flags.
	static FznsoStatisticList statistic_list() {
		static const FznsoType int_stat = fznso::Type{FznsoTypeBaseInt};
		static const FznsoStatistic stats[] = {
			{fznso::str("solutions"), int_stat, true, false},
			{fznso::str("runs"), int_stat, false, true},
		};
		return {sizeof(stats) / sizeof(stats[0]), stats};
	}

	std::optional<std::string> option_set(std::string_view name, fznso::Value value) override {
		if (name == "solution_limit") {
			solution_limit_ = value.as_int();
			return std::nullopt;
		}
		return "unknown option `" + std::string{name} + "`";
	}

	fznso::Status run(const fznso::Model& model, fznso::SolutionSink& solutions,
	                  fznso::MessageSink& messages, const fznso::StopSignal& stop) override {
		++runs_;
		// Polled before starting, and again after each solution is reported, so a
		// caller can stop from inside its own callback.
		if (stop.requested()) {
			return fznso::Status{fznso::Status::Kind::Incomplete, {}};
		}

		// This fixture ignores the constraints, which is worth saying out loud — but
		// only builds the message if someone is listening for it.
		if (messages.wanted()) {
			std::string warning =
				"ignoring " + std::to_string(model.constraint_count()) + " constraint(s)";
			messages.message("warn", fznso::Value{warning});
		}

		// Lower bound of each decision's domain, or 0 when it has none.
		std::vector<std::int64_t> base;
		for (std::size_t i = 0; i < model.decision_count(); ++i) {
			fznso::ValueVariant domain = model.decision_domain(fznso::Decision{i}).variant();
			const auto* set = std::get_if<fznso::IntRanges>(&domain);
			base.push_back(set != nullptr && !set->empty() ? (*set)[0].min : 0);
		}

		for (std::int64_t i = 0; i < solution_limit_; ++i) {
			// Shift the first variable each round so successive solutions differ.
			std::vector<std::int64_t> values = base;
			if (!values.empty()) {
				values[0] += i;
			}
			solutions.solution(ExampleSolution{std::move(values), i});
			if (stop.requested()) {
				return fznso::Status{fznso::Status::Kind::Incomplete, {}};
			}
		}
		return fznso::Status{fznso::Status::Kind::Complete, {}};
	}

private:
	std::int64_t solution_limit_ = 1;
	std::int64_t runs_ = 0;
};

} // namespace

// The library name matches the built library's base name
// (`libcpp_example_solver.*` / `cpp_example_solver.dll`), so a loader recovers
// it from the file name.
FZNSO_EXPORT_SOLVER(ExampleSolver, cpp_example_solver);
