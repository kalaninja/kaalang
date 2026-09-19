set working-directory := '..'

# List the available recipes.
[default]
list:
    @just --list release

# Package and verify every publishable crate together.
package:
    cargo package --workspace --exclude kaalang-testing --allow-dirty

# Publish one crate after its dependencies are available from crates.io.
publish crate:
    cargo publish --package {{ crate }}
