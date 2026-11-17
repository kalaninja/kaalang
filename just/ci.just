set working-directory := '..'

mod docs
mod ide
mod rust
mod wasm

# List the available recipes.
[default]
list:
    @just --list ci

# Run the full validation baseline.
run: docs::fmt-check rust::fmt-check rust::clippy rust::test wasm::test ide::test rust::svg
    git diff --check
