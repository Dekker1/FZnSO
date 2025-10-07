default: build check

# Build the Huub solver
build:
    cargo build
    cbindgen --config cbindgen_types.toml --crate ipdos-types --output c/ipdos_types.h
    cbindgen --config cbindgen_template.toml --crate ipdos-solver-template --output c/ipdos_solver_template.c

# Check the codebase using all CI linters
check:
    cargo clippy --tests -- -D warnings
    cargo +nightly fmt --check
    just --unstable --fmt --check

# Format all the files in the project
format:
    cargo +nightly fmt
    just --unstable --fmt
