set working-directory := '..'

prettier := 'prettier@3.6.2'

# List the available recipes.
[default]
list:
    @just --list docs

# Format Markdown in place.
fmt:
    npx --yes {{ prettier }} --write '**/*.md'

# Check Markdown formatting.
fmt-check:
    npx --yes {{ prettier }} --check '**/*.md'
