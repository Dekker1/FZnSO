default: build check

# Build packages and header files.
build:
    cargo build
    cbindgen --config cbindgen_types.toml --crate fznso-types --output c/fznso_types.h
    cbindgen --config cbindgen_template.toml --crate fznso-solver-template --output c/fznso_solver_template.c

# Run the test suite.
test: test-cpp test-python
    cargo test --workspace --all-features
    cargo check --manifest-path python/Cargo.toml --all-targets

# Exercise the C++ bindings.
test-cpp:
    #!/usr/bin/env sh
    set -e
    ext=$(case "$(uname)" in Darwin) echo dylib;; *) echo so;; esac)
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
    cargo build --example example_solver
    cd python
    [ -d .venv ] || uv venv --python 3.12
    . .venv/bin/activate
    uv pip install -q maturin pytest
    maturin develop
    pytest -q

# Check the codebase using all CI linters.
check: check-headers
    cargo clippy --tests -- -D warnings
    cargo +nightly fmt --check
    cargo clippy --manifest-path python/Cargo.toml --all-targets -- -D warnings
    cargo +nightly fmt --check --manifest-path python/Cargo.toml
    just --unstable --fmt --check

# Format all the files in the project.
format:
    cargo +nightly fmt
    cargo +nightly fmt --manifest-path python/Cargo.toml
    just --unstable --fmt

# Check that the (generated) headers compile.
check-headers:
    cc -fsyntax-only -x c c/fznso_types.h
    cc -fsyntax-only -x c++ c/fznso_types.h
    cc -fsyntax-only -I c -x c c/fznso_solver_template.c
    c++ -std=c++17 -Wall -Wextra -Werror -fsyntax-only -I c -x c++ cpp/fznso.hpp
    c++ -std=c++17 -Wall -Wextra -Werror -fsyntax-only -I c -x c++ cpp/fznso_export.hpp
