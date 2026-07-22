default: build check

# Build packages and header files
build:
    cargo build
    cbindgen --config cbindgen_types.toml --crate fznso-types --output c/fznso_types.h
    cbindgen --config cbindgen_template.toml --crate fznso-solver-template --output c/fznso_solver_template.c

# Run the test suite.
test:
    cargo test --workspace --all-features

# Check the codebase using all CI linters
check: check-headers
    cargo clippy --tests -- -D warnings
    cargo +nightly fmt --check
    just --unstable --fmt --check

# Format all the files in the project
format:
    cargo +nightly fmt
    just --unstable --fmt

# The generated headers are the actual deliverable for C consumers, so make sure
# they still compile rather than only checking that cbindgen exited cleanly.
check-headers:
    cc -fsyntax-only -x c c/fznso_types.h
    cc -fsyntax-only -x c++ c/fznso_types.h
    cc -fsyntax-only -I c -x c c/fznso_solver_template.c
