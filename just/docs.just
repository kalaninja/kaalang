set working-directory := '..'

mod tooling

prettier := 'prettier@3.6.2'
# Markdown, and the web sources of the IDE page, its hosts, and the bindings.
sources := "'**/*.md' 'ide/**/*.{js,mjs,css,html,ts}' 'crates/kaalang-js/tests/*.mjs'"

# List the available recipes.
[default]
list:
    @just --list docs

# Format Markdown and web sources in place.
fmt: (tooling::build "fmt")
    ./scripts/docker/run-tooling.sh kaalang-fmt npx --yes {{ prettier }} --write {{ sources }}

# Check Markdown and web source formatting.
fmt-check: (tooling::build "fmt")
    ./scripts/docker/run-tooling.sh kaalang-fmt npx --yes {{ prettier }} --check {{ sources }}
