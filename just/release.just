set working-directory := '..'

# List the available recipes.
[default]
list:
    @just --list release

# Package and verify every publishable crate together.
package:
    cargo package --workspace --exclude kaalang-testing --allow-dirty

# Verify the complete publication without uploading.
publish-dry-run:
    cargo publish --workspace --exclude kaalang-testing --dry-run

# Publish every crate in dependency order.
publish:
    cargo publish --workspace --exclude kaalang-testing

# Publish one crate when resuming a partial release.
publish-crate crate:
    cargo publish --package {{ crate }}
