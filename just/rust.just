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
    cargo test -p kaalang --test diagrams
    git diff -- '*.svg'

# Run the exhaustive comparisons the baseline leaves out.
test-exhaustive:
    cargo test --release -p kaalang-compiler --lib -- --ignored --nocapture

# Measure the performance budgets and print what they found.
perf:
    #!/usr/bin/env bash
    set -uo pipefail
    for crate in kaalang-compiler kaalang-render kaalang-svg; do
        printf '\n%s\n' "$crate"
        if ! measured=$(cargo test -q -p "$crate" --test performance -- --nocapture --test-threads=1 2>&1); then
            printf '%s\n' "$measured"
            exit 1
        fi
        printf '%s\n' "$measured" \
            | sed 's/^\.*//' \
            | grep -E '^[a-z].*: ' \
            | grep -vE '^(test|running|warning|error|note|help)' \
            | awk -F': ' '{
                value = $NF
                label = substr($0, 1, length($0) - length(value) - 2)
                printf "  %s\t%s\n", label, value
            }' \
            | column -t -s $'\t' || true
    done
