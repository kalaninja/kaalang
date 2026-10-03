set working-directory := '..'

mod tooling

# List the available recipes.
[default]
list:
    @just --list rust

# Format the workspace in place.
fmt: (tooling::build "fmt")
    ./scripts/docker/run-tooling.sh kaalang-fmt cargo fmt --all

# Check Rust formatting.
fmt-check: (tooling::build "fmt")
    ./scripts/docker/run-tooling.sh kaalang-fmt cargo fmt --all -- --check

# Type-check the workspace, the quick local loop.
check: (tooling::build "rust")
    ./scripts/docker/run-tooling.sh kaalang-rust cargo check --workspace

# Lint the workspace with warnings denied.
clippy: (tooling::build "rust")
    ./scripts/docker/run-tooling.sh kaalang-rust cargo clippy --workspace --all-targets -- -D warnings

# Run the workspace tests.
test: (tooling::build "rust")
    ./scripts/docker/run-tooling.sh kaalang-rust cargo test --workspace

# Redraw the behavior diagrams and show the diff to review.
svg: (tooling::build "rust")
    ./scripts/docker/run-tooling.sh kaalang-rust cargo test -p kaalang --test diagrams
    git diff -- '*.svg'

# Run the exhaustive comparisons the baseline leaves out.
test-exhaustive: (tooling::build "rust")
    ./scripts/docker/run-tooling.sh kaalang-rust cargo test --release -p kaalang-compiler --lib -- --ignored --nocapture

# Measure the performance budgets and print what they found.
perf: (tooling::build "rust")
    #!/usr/bin/env bash
    set -uo pipefail
    for crate in kaalang-compiler kaalang-render kaalang-svg; do
        printf '\n%s\n' "$crate"
        if ! measured=$(./scripts/docker/run-tooling.sh kaalang-rust cargo test -q -p "$crate" --test performance -- --nocapture --test-threads=1 2>&1); then
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
