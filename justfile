# CI commands
mod ci 'just/ci.just'

# Documentation commands
mod docs 'just/docs.just'

# Rust commands
mod rust 'just/rust.just'

# List the available recipes.
[default]
list:
    @just --list

# Format Markdown and Rust sources in place.
fmt: docs::fmt rust::fmt

# Check Markdown and Rust formatting.
fmt-check: docs::fmt-check rust::fmt-check
