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

## Code organization

### Module layout

Use the classic directory layout for non-root Rust modules with children: put
the parent in `name/mod.rs` and its children in `name/*.rs`. Reserve `name.rs`
for leaf modules; crate roots remain `lib.rs` or `main.rs`.

### Block kind modules

Put behavior that is specific to one block kind in a module named for that kind
within each implementation phase that needs it, such as `parse/action.rs`,
`analyze/question.rs`, or `codegen/choice.rs`. Parent phase modules own shared
data flow, structural traversal, cross-kind invariants, and thin exhaustive
dispatch only. An exhaustive dispatch delegates authored block variants to
their corresponding modules. Structural plan variants remain in the parent;
no match arm accumulates a block kind's parsing, validation, layout, or
generation logic.

When adding a block kind, add its module to every phase that needs kind-specific
behavior. Keep genuinely shared algorithms in the parent phase instead of
duplicating them across kind modules.

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

A change to the SVG renderer also changes the golden diagram, which is compared
byte for byte. Regenerate it with the CLI the crate ships, then open the result
and confirm the diagram still reads correctly before committing it:

```sh
cargo run -p contour-svg --bin cargo-contour -- diagram \
  crates/contour-svg/tests/fixtures/all_blocks.rs --flow route \
  -o crates/contour-svg/tests/fixtures/all_blocks.svg
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
