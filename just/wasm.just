set working-directory := '..'

mod tooling

# List the available recipes.
[default]
list:
    @just --list wasm

# Type-check the compiler, the renderers, and the bindings for WebAssembly.
check: (tooling::build "rust")
    ./scripts/docker/run-tooling.sh kaalang-rust cargo check --target wasm32-unknown-unknown -p kaalang-compiler -p kaalang-render -p kaalang-svg -p kaalang-js

# Build the bindings as a page loads them, into target/wasm/web.
build: (tooling::build "rust")
    ./scripts/docker/run-tooling.sh kaalang-rust cargo build --release --target wasm32-unknown-unknown -p kaalang-js
    ./scripts/docker/run-tooling.sh kaalang-rust wasm-bindgen --target web --out-dir target/wasm/web target/wasm32-unknown-unknown/release/kaalang_js.wasm

# Build the bindings and run them in Node.
test: build
    ./scripts/docker/run-tooling.sh kaalang-rust node --test crates/kaalang-js/tests/bindings.test.mjs
