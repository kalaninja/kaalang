# Development Approach

## Principles

- Keep changes small, focused, and reversible.
- Implement current language decisions without speculative syntax or
  abstractions.
- Preserve unrelated work already present in the working tree.
- Use English for documentation, filenames, identifiers, source code, comments,
  tests, commit messages, and pull request content.

## Sources of truth

RFCs in `docs/rfcs/` define kaalang syntax and semantics. Implementation code,
Rust documentation, diagnostics, test names, and test fixtures use the same
terminology as the relevant RFC. Do not duplicate the language contract in other
documentation.

Keep comments and Rust documentation concise: explain contracts and non-obvious
decisions, avoid restating code, and link to RFCs for detailed rationale.

Always write the language name as `kaalang`, including at the start of a
sentence and in headings.

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
dispatch only. An exhaustive dispatch delegates authored block variants to their
corresponding modules. Structural plan variants remain in the parent; no match
arm accumulates a block kind's parsing, validation, layout, or generation logic.

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

Use `git mv`, not plain `mv`, when moving or renaming tracked files and
directories so the rename is staged immediately and remains clear in review.

For syntax or semantic changes:

1. Read the relevant RFC, implementation, and tests.
2. State the design problem and chosen tradeoff.
3. Update the RFC together with the implementation.
4. Add behavior and compile-fail coverage as appropriate.
5. Check terminology across documentation, Rust documentation, diagnostics,
   identifiers, and test names.

## Validation

Recipes live in `justfile` and the `just/` modules and run from the repository
root with [just](https://just.systems). Format sources:

```sh
just fmt
```

That formats Markdown with [Prettier](https://prettier.io/docs/cli), which needs
Node.js and npm and respects `.gitignore`, then Rust with `cargo fmt`. The
pinned Prettier version lives in `just/docs.just`; CI uses the same one.

Run the full baseline before committing:

```sh
just ci run
```

It checks Markdown and Rust formatting, then runs `cargo clippy` with warnings
denied, `cargo test`, and `git diff --check`. `just rust` and `just docs` list
the per-step recipes for running one of them alone.

One comparison is too slow for that baseline and sits behind `#[ignore]`:

```sh
just rust test-exhaustive
```

It takes about half a minute and compares the construction with the independent
procedure over the whole declared domain. Run it before committing a change to
`construct/`, to the shapes in `kaalang-testing`, or to any rule those answers
rest on. Nothing else runs it, and it is the only check that reaches the nested
cycle shapes in bulk.

Each `crates/kaalang/tests/*/behavior/` holds one flow per file, named for the
flow it declares, with that flow's diagram beside it. The diagrams redraw
themselves during `cargo test`, so a renderer or model change arrives as a diff
over them. `just rust svg` redraws them and shows that diff. Read it before
committing: nothing else checks that the new diagrams still make sense.

Gallery examples may group related flows in one `mod.rs` and share a test. Each
flow still gets its own `<flow>.svg` beside that module.

## Performance budgets

Each crate bounds the stages it owns: `kaalang-compiler` for analysis, topology
construction and lowering to Rust, `kaalang-svg` for rendering. They share one
corpus, one set of generated stress shapes, and one statistic through
`kaalang-testing`. Keep them sharing it: a budget stated over its own corpus or
its own statistic stops comparing with the others.

The corpus is the flows the fixtures above already declare, read as text, so it
grows with the language instead of being kept in step by hand.

The bounds are wall-clock in the unoptimized dev profile, which is the profile a
macro expansion runs in. They are stated over a recorded median with enough
headroom to survive a loaded machine, because they exist to catch a large
regression rather than drift. Record the measured median beside any constant you
change, and run the suite twice: a budget that only passes once is not a gate.

`just rust perf` runs them one crate at a time, serially, and prints what they
measured as a table. Prefer its numbers to the ones `cargo test` prints: that
runs the budgets beside each other, so it measures contention as well as work.

Each run prints the distribution behind its verdict, not just the number it
asserts on. A p100 several times the p50 means the machine was busy, not that
something got slower.

### Refactoring against the budgets

A refactoring changes how the code reads, not what it costs. Do not land one
that measures slower, however few lines it saves.

The budgets do not answer this on their own. They carry deliberate headroom and
exist to catch a large regression, so a change can halve the margin and still
pass every one of them. Passing is not evidence of no regression; only a
before-and-after comparison is.

So when a refactoring touches a hot path, measure both sides:

1. Record `just rust perf` on the unchanged tree first. Two runs, as above.
2. Apply the change and record it again, the same way.
3. Compare the same named lines, and prefer the heaviest shapes: the refusal
   budgets exhaust the search and separate two implementations long before the
   corpus medians do.

`cargo test --test performance` is not a substitute at this step. It runs the
budgets beside each other, so its numbers move with contention and hide
differences this size.

Two figures that overlap between runs are not a result. Either take enough runs
to separate them or leave the code alone, and say which of the two happened
rather than reporting a percentage the samples do not support.

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
