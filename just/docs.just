set working-directory := '..'

mod tooling

prettier := 'prettier@3.6.2'

# List the available recipes.
[default]
list:
    @just --list docs

# Format Markdown in place.
fmt: (tooling::build "fmt")
    ./scripts/docker/run-tooling.sh kaalang-fmt npx --yes {{ prettier }} --write '**/*.md'

# Check Markdown formatting.
fmt-check: (tooling::build "fmt")
    ./scripts/docker/run-tooling.sh kaalang-fmt npx --yes {{ prettier }} --check '**/*.md'
