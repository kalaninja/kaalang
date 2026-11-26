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
run: docs::fmt-check rust::fmt-check validate

# Run every check but formatting, as GitHub CI does after its formatting job.
validate: ide::test rust::clippy rust::test wasm::test rust::svg
    git diff --check
