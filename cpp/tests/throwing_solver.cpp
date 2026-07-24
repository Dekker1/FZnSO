// A solver whose entry points throw, to check that the export layer never lets
// an exception cross the C interface: `run` and `option_set` must convert it to
// an error the caller can read.

#include "fznso_export.hpp"

#include <optional>
#include <stdexcept>
#include <string>
#include <string_view>

namespace {

class ThrowingSolver final : public fznso::Solver {
public:
	fznso::Value option_get(std::string_view) const override { return fznso::Value{}; }

	std::optional<std::string> option_set(std::string_view, fznso::Value) override {
		throw std::runtime_error("option boom");
	}

	fznso::Status run(const fznso::Model&, fznso::SolutionSink&, fznso::MessageSink&) override {
		throw std::runtime_error("run boom");
	}
};

} // namespace

FZNSO_EXPORT_SOLVER(ThrowingSolver, throwing_solver);
