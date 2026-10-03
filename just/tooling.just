set working-directory := '..'

# List the available recipes.
[default]
list:
    @just --list tooling

# Build one tooling image: `rust` for builds and tests, `fmt` for formatting.
# CI builds them in cached steps of their own and sets SKIP_IMAGE_BUILD.
build image:
    @if [ -z "${SKIP_IMAGE_BUILD:-}" ]; then docker build --target {{ image }} --tag kaalang-{{ image }} .; fi
