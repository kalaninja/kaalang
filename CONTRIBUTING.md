# Development Approach

## Principles

- Keep changes small, focused, and reversible.
- Implement current language decisions without speculative syntax or
  abstractions.
- Preserve unrelated work already present in the working tree.
- Use English for documentation, filenames, identifiers, source code, comments,
  tests, commit messages, and pull request content.

## Sources of truth

RFCs in `docs/rfcs/` define Contour syntax and semantics. Implementation code,
Rust documentation, diagnostics, test names, and test fixtures use the same
terminology as the relevant RFC. Do not duplicate the language contract in
other documentation.

Behavior tests are executable examples of accepted programs. Compile-fail tests
document rejected programs and their diagnostics.

## Dependencies

Every dependency is declared once, in `[workspace.dependencies]` in the root
`Cargo.toml`. The table holds two groups separated by a blank line: local
workspace crates (declared by `path`) first, then external crates. Entries
within each group are alphabetical.

Member crates inherit with `{ workspace = true }` and never state a version or a
path of their own. A member may add features on top of an inherited dependency
(`{ workspace = true, features = [...] }`) when only that crate needs them.

Adding a dependency means adding it to the root table first, in its group and in
alphabetical position.

## Change workflow

For syntax or semantic changes:

1. Read the relevant RFC, implementation, and tests.
2. State the design problem and chosen tradeoff.
3. Update the RFC together with the implementation.
4. Add behavior and compile-fail coverage as appropriate.
5. Check terminology across documentation, Rust documentation, diagnostics,
   identifiers, and test names.

## Validation

Run from the repository root:

```sh
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
git diff --check
```

## Commits

Use Conventional Commits:

```text
<type>: <concise imperative summary>
```

Use `feat`, `fix`, `refactor`, `test`, `docs`, or `chore` as the type. Add a
scope only when it makes the subject clearer. Keep one logical change per
commit. Before committing, inspect both the staged diff and recent commit
subjects.

Do not describe validation results in the commit message. Do not add
`Co-Authored-By` or other AI-attribution trailers. Rewrite an existing commit
only when explicitly requested.
