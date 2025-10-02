default: build check

# Build the Huub solver
build:
    cargo build
    cbindgen --config crates/ipdos_protocol/cbindgen.toml --crate ipdos_protocol --output crates/ipdos_protocol/ipdos_protocol.h

# Check the codebase using all CI linters
check:
    cargo clippy --tests -- -D warnings
    cargo +nightly fmt --check
    just --unstable --fmt --check

# Format all the files in the project
format:
    cargo +nightly fmt
    just --unstable --fmt
