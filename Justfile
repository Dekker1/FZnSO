default: build check

# Build packages and header files.
build:
    cargo build
    cbindgen --config cbindgen_types.toml --crate fznso-types --output c/fznso_types.h
    cbindgen --config cbindgen_template.toml --crate fznso-solver-template --output c/fznso_solver_template.c
    just strip-doc-links

# Rewrite rustdoc's `[`Foo::bar`]` intra-doc links in the generated C artifacts.
#
# The doc comments are written for rustdoc, where those brackets are real links.
# In a C header they are noise, and Doxygen reads the bare `Foo::bar` as a symbol
# to link anyway — so both audiences get the better rendering from one source.
[private]
strip-doc-links:
    #!/usr/bin/env sh
    set -e
    for f in c/fznso_types.h c/fznso_solver_template.c; do
        perl -0pi -e 's/\[`([^`\]]+)`\]/$1/g' "$f"
    done

# Run the test suite.
test: test-cpp test-python
    cargo test --workspace --all-features
    cargo check --manifest-path python/Cargo.toml --all-targets

# Exercise the C++ bindings.
test-cpp:
    #!/usr/bin/env sh
    set -e
    case "$(uname)" in
        Darwin) ext=dylib ;;
        *) ext=so ;;
    esac
    warn="-std=c++17 -Wall -Wextra -Werror"
    san="-g -fsanitize=address,undefined -fno-omit-frame-pointer"
    cargo build --example example_solver --example bad_abi_solver
    c++ $warn -shared -fPIC -I c -I cpp -o "target/libcpp_example_solver.$ext" \
        cpp/tests/example_solver.cpp
    c++ $warn -shared -fPIC -I c -I cpp -o "target/libthrowing_solver.$ext" \
        cpp/tests/throwing_solver.cpp
    c++ $warn $san -I c -I cpp -o target/test_consumer cpp/tests/test_consumer.cpp
    c++ $warn $san -I c -I cpp -o target/test_value_source cpp/tests/test_value_source.cpp
    ./target/test_value_source
    ./target/test_consumer "target/debug/examples/libexample_solver.$ext"
    ./target/test_consumer "target/libcpp_example_solver.$ext"
    c++ $warn $san -I c -I cpp -o target/test_exceptions cpp/tests/test_exceptions.cpp
    ./target/test_exceptions "target/libthrowing_solver.$ext"
    c++ $warn $san -I c -I cpp -o target/test_abi_version cpp/tests/test_abi_version.cpp
    ./target/test_abi_version "target/debug/examples/libbad_abi_solver.$ext"
    if command -v x86_64-w64-mingw32-g++ >/dev/null 2>&1; then
        x86_64-w64-mingw32-g++ $warn -static -I c -I cpp \
            -o target/test_consumer.exe cpp/tests/test_consumer.cpp
        x86_64-w64-mingw32-g++ $warn -static -I c -I cpp \
            -o target/test_value_source.exe cpp/tests/test_value_source.cpp
        x86_64-w64-mingw32-g++ $warn -shared -I c -I cpp \
            -o target/cpp_example_solver.dll cpp/tests/example_solver.cpp
        x86_64-w64-mingw32-g++ $warn -shared -I c -I cpp \
            -o target/throwing_solver.dll cpp/tests/throwing_solver.cpp
        x86_64-w64-mingw32-g++ $warn -static -I c -I cpp \
            -o target/test_exceptions.exe cpp/tests/test_exceptions.cpp
    else
        echo "skipping Windows cross-build: x86_64-w64-mingw32-g++ not installed (brew install mingw-w64)"
    fi

# Build and test the Python bindings.
test-python:
    #!/usr/bin/env sh
    set -e
    cargo build --example example_solver --example threaded_solver
    cd python
    [ -d .venv ] || uv venv --python 3.12
    . .venv/bin/activate
    uv pip install -q maturin pytest
    maturin develop
    pytest -q

# Run the Rust suite under miri.
#
# `--lib --tests` is not optional: a plain `--workspace` also builds the
# `cdylib` examples, which miri cannot produce a target for. The dynamic-loading
# tests skip themselves here, since miri cannot `dlopen`; `tests/abi.rs` drives
# the generated entry points directly so they are covered anyway.
miri:
    cargo +nightly miri test --workspace --lib --tests

# Check the codebase using all CI linters.
check: check-headers
    #!/usr/bin/env sh
    set -e
    cargo clippy --tests -- -D warnings
    cargo +nightly fmt --check
    # pyo3 resolves the interpreter at build time, and the crate requires
    # abi3-py310 — so a system `python3` older than 3.10 fails the build. Prefer
    # the project's venv, which `just test-python` and `just docs-api` create.
    if [ -x python/.venv/bin/python ]; then
        PYO3_PYTHON="$PWD/python/.venv/bin/python" \
            cargo clippy --manifest-path python/Cargo.toml --all-targets -- -D warnings
    else
        cargo clippy --manifest-path python/Cargo.toml --all-targets -- -D warnings
    fi
    cargo +nightly fmt --check --manifest-path python/Cargo.toml
    just --unstable --fmt --check

# Format all the files in the project.
format:
    cargo +nightly fmt
    cargo +nightly fmt --manifest-path python/Cargo.toml
    just --unstable --fmt

# Build the documentation site, including the API reference.
docs: docs-api
    pnpm --dir docs install --frozen-lockfile
    pnpm --dir docs build

# Serve the documentation site with live reload.
docs-serve: docs-api
    pnpm --dir docs install
    pnpm --dir docs dev

# Dump each API surface in machine-readable form, then render the reference
# pages from it.
#
# Doxygen and the Python toolchain are optional locally: a missing one skips its
# pages with a warning rather than failing, so the prose stays workable without
# them. CI installs both, so a deploy always has all six pages.
[private]
docs-api:
    #!/usr/bin/env sh
    set -e
    # `--document-private-items` is deliberately absent: the reference shows the
    # public surface. `-D warnings` keeps intra-doc links honest, since they
    # become real cross-page links in the rendered reference.
    RUSTDOCFLAGS="-D warnings" cargo +nightly rustdoc -p fznso-types -- \
        -Zunstable-options --output-format json
    RUSTDOCFLAGS="-D warnings" cargo +nightly rustdoc -p fznso-solver-template -- \
        -Zunstable-options --output-format json
    RUSTDOCFLAGS="-D warnings" cargo +nightly rustdoc -p fznso -- \
        -Zunstable-options --output-format json

    if command -v doxygen >/dev/null 2>&1; then
        doxygen docs/Doxyfile
    else
        echo "skipping C++ reference: doxygen not installed (brew install doxygen)"
    fi

    if [ -x python/.venv/bin/griffe ]; then
        (cd python && . .venv/bin/activate && maturin develop -q \
            && griffe dump fznso --force-inspection -o /tmp/fznso-griffe.json >/dev/null)
    else
        echo "skipping Python reference: run \`just docs-python-deps\` first"
    fi

    node docs/scripts/gen-api.mjs

# Install the Python tooling needed for the Python API reference.
docs-python-deps:
    #!/usr/bin/env sh
    set -e
    cd python
    [ -d .venv ] || uv venv --python 3.12
    . .venv/bin/activate
    uv pip install -q maturin griffe

# Check that the (generated) headers compile.
check-headers:
    cc -fsyntax-only -x c c/fznso_types.h
    cc -fsyntax-only -x c++ c/fznso_types.h
    cc -fsyntax-only -I c -x c c/fznso_solver_template.c
    c++ -std=c++17 -Wall -Wextra -Werror -fsyntax-only -I c -x c++ cpp/fznso.hpp
    c++ -std=c++17 -Wall -Wextra -Werror -fsyntax-only -I c -x c++ cpp/fznso_export.hpp
