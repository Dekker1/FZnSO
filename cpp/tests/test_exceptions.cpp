// Drives the throwing solver and checks that exceptions thrown by its entry
// points arrive as errors rather than crossing the C interface as undefined
// behaviour.

#include "fznso.hpp"

#include <cstdio>
#include <cstdlib>

namespace {

void check(bool condition, const char* what) {
	if (!condition) {
		std::fprintf(stderr, "FAILED: %s\n", what);
		std::abort();
	}
}

} // namespace

int main(int argc, char** argv) {
	check(argc == 2, "usage: test_exceptions <path-to-throwing-solver-library>");
	fznso::Library library{argv[1]};
	fznso::DynSolver solver = library.create_solver();

	// A throwing option_set becomes an error carrying the exception's message.
	std::optional<std::string> option_error = solver.option_set("x", fznso::Value{});
	check(option_error.has_value(), "throwing option_set reports an error");
	check(*option_error == "option boom", "option_set error carries the message");

	// A throwing run becomes Status::Error carrying the exception's message.
	fznso::LayeredModel model;
	fznso::Status status = solver.run(model, [](const fznso::Solution&) {});
	check(status.failed(), "throwing run reports an error");
	check(status.error == "run boom", "run error carries the message");

	std::printf("all exception-safety checks passed\n");
	return 0;
}
