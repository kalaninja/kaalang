# Development Approach

## Principles

- Keep changes small, focused, and reversible.
- Implement current language decisions without speculative syntax or
  abstractions.
- Preserve unrelated work already present in the working tree.
- Use English for documentation, filenames, identifiers, source code, comments,
  tests, commit messages, and pull request content.

## Sources of truth

Implementation code, Rust documentation, diagnostics, test names, and test
fixtures use the terminology of the relevant RFC. Do not duplicate the language
contract in other documentation; link the RFC paragraph in force, which
`docs/language.md` lists by subject.

RFCs in `docs/rfcs/` describe syntax, semantics, observable behavior, and design
decisions. Algorithms, transformation steps, their rationale, and correctness
arguments belong in the RFC covering that subject. Crate choices, library
versions, and build or runtime configuration belong in code or development
documentation. So does the organization of the code: which crate or function
does what, library interfaces, the names of types and functions, and tests. An
RFC names none of them. `docs/architecture.md` describes the repository, its
tooling, and the implementation.

Accepted RFCs are immutable. A new RFC changes an accepted decision by naming
the provisions it supersedes, and takes precedence for them.

Comments and Rust documentation explain contracts and non-obvious decisions,
never what the code shows, and link to RFCs for detailed rationale.

Always write the language name as `kaalang`, including at the start of a
sentence and in headings.

## Block kind modules

Put behavior specific to one block kind in a module named for that kind, in
every phase that needs it, such as `parse/action.rs`, `analyze/question.rs`, or
`codegen/choice.rs`. The parent phase module keeps only shared data flow and
algorithms, structural traversal, cross-kind invariants, structural plan
variants, and an exhaustive dispatch that hands each authored block variant to
its module.

## Dependencies

Every dependency is declared once, in `[workspace.dependencies]` in the root
`Cargo.toml`: local workspace crates, declared by `path`, first, then a blank
line and external crates, each group alphabetical. Member crates inherit with
`{ workspace = true }`, never with a version or path of their own, and may add
features only they need.

## Change workflow

Move and rename tracked files with `git mv`, so review shows the rename.

For syntax or semantic changes:

1. Read the relevant RFC, implementation, and tests.
2. State the design problem and chosen tradeoff.
3. Add a new RFC, or update a proposed one, together with the implementation,
   and list the paragraphs it puts in force in `docs/language.md`.
4. Add behavior and compile-fail coverage as appropriate.
5. Check terminology against `GLOSSARY.md` across documentation, Rust
   documentation, diagnostics, identifiers, and test names.

## Validation

Before committing:

- Run `just fmt`, then the full baseline, `just ci run`.
- Review the diagram diff `just rust svg` shows; the baseline runs it. Never
  edit a generated `.svg` by hand.
- After a change to `construct/`, `analyze/`, `symbolic/`, the shapes in
  `kaalang-testing`, or any rule those answers rest on, run
  `just rust test-exhaustive`. It takes about half a minute and stays out of the
  baseline: it compares the construction with the independent procedure over the
  whole declared domain, and the two analysis paths over every generated cycle
  shape; no other check reaches the nested cycle shapes in bulk.
- After a change to the IDE page, try it with `just ide browser`; every check on
  its `checks.html` must pass.
- After a change to the VS Code extension, or to the page it shows, check the
  cases of `ide/sample/README.md` with `just ide vscode`.

Flow fixtures live in `crates/kaalang/tests/flow/`, one directory per language
feature and one flow per file, named for its flow, with its diagram beside it. A
feature's `compile_fail/` holds the programs it rejects.

Gallery modules and their SVGs live in `crates/kaalang/examples/gallery/`. Its
`main.rs` runs each module's checks, under
`cargo run -p kaalang --example gallery` and during `cargo test`.
`just rust gallery` draws it through the CLI from the flows, types, and
collapsed views listed in `scripts/draw-gallery.sh`; keep that list in step with
the examples.

Type fixtures live in `crates/kaalang/tests/datatype/`, one directory per case,
named for the case. Each declares only the types its case needs and a test of
their behavior. A fixture without flows draws a diagram into its directory for
every type its `mod.rs` declares, datatype or plain type, named after the type
in snake case. The first type is the case's subject, and its diagram is named
after the directory instead, as `many_incoming_types/many_incoming_types.svg`. A
type's incoming types there are those in the same file that refer to it by its
bare name. The Rust and WebAssembly tests draw the fixtures for the same 64-bit
Linux target from `crates/kaalang/tests/diagram-target.cfg`.

Hand-written stress fixtures live in `crates/kaalang/tests/stress/<flow>/mod.rs`
with their tests and diagrams. They combine language features heavily and get
the stress tier of the corpus budgets.

### Packaging a release

1. Set the version in the root `Cargo.toml`, in `[workspace.package]` and in
   each path crate's entry in `[workspace.dependencies]`, and date its section
   in `CHANGELOG.md`.
2. Run `just release package`, then commit the version and the date: the
   publishing recipes refuse an uncommitted tree.
3. Run `just release publish-dry-run`, then `just release publish`.
4. To resume a partial release, publish one crate at a time with
   `just release publish-crate <crate>`, in dependency order:
   `kaalang-compiler`, `kaalang-macros`, `kaalang-render`, `kaalang-svg`,
   `kaalang-workspace`, `kaalang`, then `kaalang-cli`.
5. Tag the released commit and push the tag:

   ```sh
   git tag -a v<version> -m "kaalang <version>"
   git push origin v<version>
   ```

## Performance budgets

Every budget takes its fixture sources, generated probes, and harness from
`kaalang-testing`: a budget over its own corpus, probes, or statistic stops
comparing with the others.

The bounds are wall-clock time in the unoptimized dev profile, the profile a
macro expansion runs in. They sit over a recorded median with headroom for a
loaded machine: they catch a large regression, not drift. Record the measured
median beside any constant you change, and run the suite twice: a budget that
only passes once is not a gate.

`just rust perf` runs each crate's budgets serially and prints a table. Prefer
its numbers to those of `cargo test`, which runs the budgets side by side and so
measures contention as well as work. Each run prints the distribution behind its
verdict; a p100 several times the p50 means the machine was busy, not that
something got slower.

### Refactoring against the budgets

A refactoring may land with comparable performance, never with a reproducible
slowdown, however few lines it saves. The budgets cannot tell: their headroom
lets a change halve the margin and still pass. When a refactoring touches a hot
path, compare both sides:

1. Record `just rust perf` twice on the unchanged tree.
2. Apply the change and record it twice again.
3. Compare the same named lines, heaviest shapes first: the largest generated
   probes and the stress fixtures separate two implementations long before the
   corpus medians do.

Overlapping measurements are consistent with unchanged performance and no reason
to reject a refactoring. Report only differences the samples support, and no
speedup for comparable results. If noise leaves a suspected slowdown unresolved,
improve the conditions and repeat.

## Commits

Use Conventional Commits:

```text
<type>: <concise imperative summary>
```

The type is `feat`, `fix`, `refactor`, `test`, `docs`, or `chore`; add a scope
only when it makes the subject clearer. Keep one logical change per commit, and
inspect the staged diff and recent subjects before committing.

Do not describe validation results in the message or add `Co-Authored-By` or
other AI-attribution trailers. Rewrite an existing commit only when explicitly
requested.
