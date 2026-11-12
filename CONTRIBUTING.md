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
Rust documentation, diagnostics, test names, and test fixtures use the
terminology of the relevant RFC. Do not duplicate the language contract in other
documentation.

RFCs describe syntax, semantics, observable behavior, and design decisions.
Algorithms, transformation steps, their rationale, and correctness arguments
belong in the RFC covering that subject. Crate choices, library versions, and
build or runtime configuration belong in code or development documentation.

Accepted RFCs are immutable. A new RFC changes an accepted decision by naming
the provisions it supersedes, and takes precedence for them.

Comments and Rust documentation explain contracts and non-obvious decisions,
never what the code shows, and link to RFCs for detailed rationale.

Always write the language name as `kaalang`, including at the start of a
sentence and in headings.

Behavior tests are executable examples of accepted programs. Compile-fail tests
document rejected programs and their diagnostics.

## Code organization

### Block kind modules

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
3. Add a new RFC, or update a proposed one, together with the implementation.
4. Add behavior and compile-fail coverage as appropriate.
5. Check terminology against `GLOSSARY.md` across documentation, Rust
   documentation, diagnostics, identifiers, and test names.

## Validation

Recipes live in `justfile` and `just/` and run from the repository root with
[just](https://just.systems). The Rust and Markdown recipes run in Docker images
built from `Dockerfile`, so the host needs only just and Docker; the release
recipes run cargo on the host for its crates.io credentials. The images install
the tools pinned in `mise/` with [mise](https://mise.jdx.dev). The nightly
rustfmt in the `fmt` image applies the import options in `rustfmt.toml`. A
recipe rebuilds its image when those files change. Downloaded crates and npm
packages stay in the ignored `.cache/`.

Format sources:

```sh
just fmt
```

It runs [Prettier](https://prettier.io/docs/cli), pinned in `just/docs.just`, on
Markdown and the nightly `cargo fmt` on Rust.

Run the full baseline before committing:

```sh
just ci run
```

It checks formatting, then runs `cargo clippy` with warnings denied,
`cargo test`, `just rust svg`, and `git diff --check`. `just rust` and
`just docs` list the per-step recipes.

One comparison is too slow for the baseline and sits behind `#[ignore]`:

```sh
just rust test-exhaustive
```

It takes about half a minute. It compares the construction with the independent
procedure over the whole declared domain, and the two analysis paths over every
generated cycle shape; no other check reaches the nested cycle shapes in bulk.
Run it before committing a change to `construct/`, `analyze/`, `symbolic/`, the
shapes in `kaalang-testing`, or any rule those answers rest on.

`just rust svg` redraws every generated diagram and shows the diff; review it
before committing. The diagram generator is an ignored test, so `cargo test`
leaves the SVG files untouched.

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
bare name. Every fixture is read for 64-bit Linux, fixed in `tests/diagrams.rs`,
so the diagrams do not depend on the machine that draws them.

Hand-written stress fixtures live in `crates/kaalang/tests/stress/<flow>/mod.rs`
with their tests and diagrams. They combine language features heavily and get
the stress tier of the corpus budgets.

### Packaging a release

Set the version in the root `Cargo.toml`, in `[workspace.package]` and in each
path crate's entry in `[workspace.dependencies]`, and date its section in
`CHANGELOG.md`. Then verify the publishable packages together:

```sh
just release package
```

Commit the version and the date; the publishing recipes refuse an uncommitted
tree. Preview the publication without uploading:

```sh
just release publish-dry-run
```

Publish the workspace in dependency order:

```sh
just release publish
```

To resume a partial release, publish one crate at a time with
`just release publish-crate <crate>`, in dependency order: `kaalang-compiler`,
`kaalang-macros`, `kaalang-render`, `kaalang-svg`, `kaalang`, then
`kaalang-cli`. `kaalang-testing` is never published.

Then tag the released commit and push the tag:

```sh
git tag -a v<version> -m "kaalang <version>"
git push origin v<version>
```

## Performance budgets

Each crate bounds the stages it owns: `kaalang-compiler` analysis, topology
construction, and lowering to Rust; `kaalang-render` structural compaction; and
`kaalang-svg` measurement and serialization. They share fixture sources and
generated probes through `kaalang-testing`, and the compiler and SVG corpus
budgets share one harness for warm-up, measurement, statistics, and the
aggregate and per-item checks. Keep it that way: a budget over its own corpus,
probes, or statistic stops comparing with the others.

The fixture corpus is read from the checked-in test fixtures, so it grows with
the language. Ordinary and stress fixtures run in separate corpus tests, each
with an aggregate budget of `count * allowance` and its own per-item limit. The
allowance is below the per-item limit, so the aggregate still catches
distributed regressions. Compiler passes take every fixture flow once; SVG
passes take expanded and collapsed diagrams of cycle fixtures. The render
crate's generated probes cover large compaction shapes outside the corpus.

The bounds are wall-clock time in the unoptimized dev profile, the profile a
macro expansion runs in. They sit over a recorded median with headroom for a
loaded machine: they catch a large regression, not drift. Record the measured
median beside any constant you change, and run the suite twice: a budget that
only passes once is not a gate.

`just rust perf` runs the three crates serially and prints a table. Prefer its
numbers to those of `cargo test`, which runs the budgets side by side and so
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
