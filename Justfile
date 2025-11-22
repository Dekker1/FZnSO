default: build check

# Build packages and header files
build:
    cargo build
    cbindgen --config cbindgen_types.toml --crate fznso-types --output c/fznso_types.h
    cbindgen --config cbindgen_template.toml --crate fznso-solver-template --output c/fznso_solver_template.c

# Check the codebase using all CI linters
check:
    cargo clippy --tests -- -D warnings
    cargo +nightly fmt --check
    just --unstable --fmt --check

# Format all the files in the project
format:
    cargo +nightly fmt
    just --unstable --fmt
