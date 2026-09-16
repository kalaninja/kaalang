set working-directory := '..'

# List the available recipes.
[default]
list:
    @just --list rust

# Format the workspace in place.
fmt:
    cargo fmt --all

# Check Rust formatting.
fmt-check:
    cargo fmt --all -- --check

# Type-check the workspace, the quick local loop.
check:
    cargo check --workspace

# Lint the workspace with warnings denied.
clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Run the workspace tests.
test:
    cargo test --workspace

# Redraw the behavior diagrams and show the diff to review.
svg:
    cargo test -p kaalang
    git diff -- '*.svg'
