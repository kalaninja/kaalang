set working-directory := '..'

mod tooling
mod wasm

port := '8080'

# List the available recipes.
[default]
list:
    @just --list ide

# Run the page's tests in Node.
test: (tooling::build "rust")
    ./scripts/docker/run-tooling.sh kaalang-rust node --test 'ide/page/tests/*.test.mjs'

# Serve the page's test bench at http://127.0.0.1:8080/ide/page/bench/ and its race checks at http://127.0.0.1:8080/ide/page/bench/checks.html.
bench: wasm::build
    TOOLING_PORT={{ port }} ./scripts/docker/run-tooling.sh kaalang-rust node ide/page/bench/serve.mjs . {{ port }}
