// Checks that loading a solver whose reported ABI version does not match is
// rejected with a clear error, rather than proceeding into undefined behaviour.

#include "fznso.hpp"

#include <cstdio>
#include <cstdlib>
#include <string>

namespace {

void check(bool condition, const char* what) {
	if (!condition) {
		std::fprintf(stderr, "FAILED: %s\n", what);
		std::abort();
	}
}

} // namespace

int main(int argc, char** argv) {
	check(argc == 2, "usage: test_abi_version <path-to-bad-abi-solver>");

	bool threw = false;
	try {
		fznso::Library library{argv[1]};
	} catch (const std::runtime_error& error) {
		threw = true;
		check(std::string{error.what()}.find("ABI version") != std::string::npos,
		      "the error names the ABI version");
	}
	check(threw, "loading an ABI-mismatched solver throws");

	std::printf("abi-version rejection check passed\n");
	return 0;
}
