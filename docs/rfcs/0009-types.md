# RFC 0009: Types

- Status: proposed
- Language: [RFC 0001: kaalang Language](0001-language.md)
- Visual language: [RFC 0002: kaalang Visual Language](0002-visual-language.md)
- Renderer: [RFC 0003: kaalang SVG Renderer](0003-svg-renderer.md)

## 1. Motivation and scope

A flow describes one function. Types need a corresponding view that explains one
type's structure and its relationships without drawing an entire domain model. A
field's Rust type explains how it stores a value, but does not always identify
the domain concept it refers to. A member key and a borrowed member can both
refer to the same domain concept.

This RFC proposes annotations for Rust structs, enums, unions, and type aliases,
and a diagram centered on one selected declaration. Descriptions and
relationships are authored explicitly. Rust remains responsible for storage,
ownership, borrowing, and type checking.

### 1.1 Terms

- An **annotated type** is a Rust type declaration marked `#[kaa]`.
- A **relationship** is one explicit `refer` annotation on a field or alias. It
  has a source component, the annotated field or alias, a target, and a
  description.
- The **focus type** is the annotated type selected for a type diagram.
- An **incoming type** is another annotated type with an explicit relationship
  targeting the focus type.
- A **type card** contains a declaration's name, optional description, and
  structure.

These terms describe type diagrams. A relationship is neither a flow wire nor a
control-flow connection.

## 2. Annotations

### 2.1 Annotated types

```rust
#[kaa("A member that can appear in several roles.")]
struct Member {
    #[kaa::value("The member's display name.")]
    name: String,
}
```

`#[kaa]` marks a type declaration as it marks a flow. On a type it may carry one
description, a nonempty Rust string literal, as `#[kaa("description")]`; a type
without one is described by its name. The attribute applies to named, tuple, and
unit structs, enums, unions, and type aliases. Generic parameters and lifetimes
remain part of the Rust declaration.

The Rust identifier supplies the name; a card header qualifies it with its
module path (§4.2). There is no separate display-name argument. Descriptions
come from attributes, including on aliases. Rustdoc is not a description source:
it documents a type for its users, while a description explains the type to a
reviewer of the diagram, and the two texts may differ.

Descriptions use the restricted Markdown of [RFC 0005](0005-markdown.md), as
block descriptions do. Field names, positions, Rust types, and target names are
source-derived labels and stay literal.

The attribute has the same behavior for all supported declaration forms. Rust's
declaration form supplies the structural distinction.

### 2.2 Fields

A field may have one of two annotations:

```rust
#[kaa("An article in the catalog.")]
struct Article<'a> {
    #[kaa::value("The display title.")]
    title: String,
    #[kaa::refer(Member, "The member responsible for this article.")]
    owner: &'a Member,
    #[kaa::refer(Member, "Reviewers, resolved through the directory.")]
    reviewers: Vec<MemberId>,
}
```

`value("description")` describes a field without declaring a relationship.

`refer(Target, "description")` describes a relationship and names exactly one
target: an annotated type, written without parameters as a name or a path, as
the field's module sees it. The field's Rust type remains a separate part of its
declaration. A relationship description also serves as the field description;
there is no second description attribute.

Both descriptions are nonempty Rust string literals. The annotations apply to
struct fields, enum-variant fields, and union fields. Tuple fields use their
positions instead of invented names. An unannotated field retains its Rust name
or position and type, but has no authored description or relationship.

An enum variant may carry `#[kaa::case("description")]`, like a choice branch in
a flow. It describes the variant as a whole; a unit variant has no field to
annotate, so `case` is its only description.

A field cannot have both `value` and `refer`, or repeat either annotation. One
field does not declare multiple targets in this RFC.

### 2.3 Aliases

An alias can give an existing representation a domain-specific name and
description:

```rust
#[kaa("The description for the current revision.")]
type CurrentDetails = Details;

#[kaa("The description retained for an earlier revision.")]
type RevisionDetails = Details;

#[kaa("An ordered sequence of earlier descriptions.")]
#[kaa::refer(RevisionDetails, "Earlier descriptions in revision order.")]
type History = Vec<RevisionDetails>;
```

An alias has no field on which to put its relationship. An optional `refer`
therefore belongs directly to the alias, alongside `#[kaa]`. It describes the
aliased value as a whole. The alias's structure shows its right-hand-side Rust
type and, when present, the relationship description and target.

Aliases remain distinct authored declarations in diagrams: `CurrentDetails` and
`RevisionDetails` retain their names even though both name `Details`. No
implicit relationship is inferred from an alias's right-hand side, and the
renderer does not copy the underlying declaration's fields into the alias. An
alias without `refer` has no outgoing relationships.

This is a distinction in the diagram, not additional Rust type identity. Rust
aliases name an existing type rather than define a new one.
[Rust Reference: type aliases](https://doc.rust-lang.org/reference/items/type-aliases.html).

### 2.4 Generics

```rust
#[kaa("An attachment with a configurable payload.")]
struct Attachment<T> {
    #[kaa::value("The boxed attachment payload.")]
    payload: Box<T>,
}
```

A type parameter is not a relationship target: `refer(T, ...)` is rejected,
since there is no declaration for the relationship to point at. The field keeps
its Rust type `Box<T>` and a `value` description.

A field using this generic declaration writes:

```rust
#[kaa::refer(Attachment, "The attached content.")]
content: Attachment<Payload>,
```

This is one relationship to `Attachment`. The Rust type preserves `Payload`, but
does not create a second relationship to it or specialize the relationship
inside `Attachment`. `refer(Attachment<Payload>, ...)` is rejected. A
relationship to `Payload` needs its own `refer(Payload, ...)` on another field.

### 2.5 The `kaa` namespace

The attribute is `#[kaa]`, or `#[kaalang::kaa]` through the crate path, and each
annotation is written under its path: `#[kaa::value(...)]`,
`#[kaa::refer(...)]`, `#[kaa::case(...)]`. This RFC names an annotation by its
last segment, so `value` means `#[kaa::value(...)]`. The language keeps its
name; the attribute is short because a flow repeats it on every block.

A type's fields and variants also carry the helper attributes of every derive on
that type, and those take short common names: `#[derive(clap::ValueEnum)]` reads
`#[value(...)]` on an enum and its variants. A bare `value` could not tell the
two apart. The path leaves bare names to other macros. It needs no import of its
own: the attribute removes its annotations before Rust resolves them.

A flow's body is less exposed. A derive cannot attach to a statement, and an
attribute macro on a statement is unstable, so a collision would take another
macro on the same function that interprets attributes in its body. Flows take
the same namespace for consistency. This supersedes the attribute spelling of
RFCs 0001 to 0008, wherever they write it: a flow is marked `#[kaa]` instead of
`#[kaalang]`, and `#[kaalang::kaa]` replaces `#[kaalang::kaalang]`; the block
attributes become `#[kaa::action]`, `#[kaa::call]`, `#[kaa::question]`,
`#[kaa::yes]`, `#[kaa::no]`, `#[kaa::choice]`, `#[kaa::case]`, `#[kaa::cycle]`,
and `#[kaa::stage]`. Their arguments, placement, and meaning do not change, and
a bare block attribute is no longer kaalang's. Every name under `kaa::` belongs
to kaalang, and one it does not define is rejected, on a type as in a flow.

## 3. Relationship meaning

`refer` expresses domain intent independently of storage:

| Field type         | Relationship target | Meaning supplied by the description                  |
| ------------------ | ------------------- | ---------------------------------------------------- |
| `Member`           | `Member`            | A member stored in the value.                        |
| `&'a Member`       | `Member`            | A borrowed member.                                   |
| `MemberId`         | `Member`            | A member resolved through a directory.               |
| `Option<MemberId>` | `Member`            | An optional member resolved through a directory.     |
| `Vec<MemberId>`    | `Member`            | Members resolved from an ordered collection of keys. |

The same annotation covers these cases. It generates no lookup, allocation,
borrow, ownership transfer, or integrity check. A target need not equal the
field's Rust type, and a matching Rust type alone does not create a
relationship. There are no separate ownership, direct-reference, or lookup
relationship kinds in this RFC.

A registry may store `HashMap<EntryKey, Document<'a>>` and explicitly refer to
`Document`. The key stays a separate Rust type, and kaalang does not infer
identity, uniqueness, or cardinality from a field or container.

## 4. Type diagrams

### 4.1 Scope and direction

A type diagram shows:

1. One card for the focus type, including its complete immediate structure.
2. One card for each incoming type in the selected source, placed above the
   focus, including that type's immediate structure.
3. One directed line for each explicit relationship from an incoming type to the
   focus.

The selected source is the workspace the focus type belongs to. A relationship
target and an incoming type may be declared in any crate of that workspace.

The diagram does not recursively add callers of incoming types. Types used by
the focus appear in its field types and relationship targets; they do not get
separate cards below it. There are no lines from the focus to other cards. An
incoming type with several relationships to the focus is drawn once; each
relationship keeps its own source field or alias.

### 4.2 Structure

Every card uses the same order: name, description when supplied, structure. The
header shows the name as the declaration's path from its crate root, prefixed
with the crate name when the crate differs from the focus type's, with the
generic parameters and lifetimes as declared: `model::Attachment<T>`. It also
identifies the Rust declaration form and its representation where applicable.

Products are horizontal: a struct's fields occupy a row in declaration order.
Sums are vertical: enum variants occupy successive rows in declaration order,
and each variant's fields form their own horizontal product after its name and
its description, when supplied. Variants keep their actual field counts; shorter
variants do not acquire artificial fields or equal-width filler cells.

A union presents its fields as overlapping storage alternatives, vertically. It
is not drawn as a tagged enum. An alias shows its definition as described in
§2.3. Unit structs and variants show their names without invented components.

Only annotated declarations have type cards. The renderer does not expand the
implementation of `String`, `Vec`, `HashMap`, `Box`, or other unannotated types.
An array is one field with its Rust type, including the length, not one cell per
element. A field that names another annotated type still occupies one cell; its
target's fields are not recursively inlined.

A field cell reads from top to bottom:

1. Field name or tuple position.
2. Authored Rust type, including lifetimes and generic arguments.
3. Description, when supplied.
4. Relationship target, when supplied.

Relationship targets align with the bottom of their product row. They show an
arrowhead, the same for every relationship, and the target name, without a
`refer` label. Incoming lines identify their source component, so neither the
field name nor its description is duplicated on the line.

### 4.3 Arrangement

Incoming cards follow source order from left to right above the focus, whether
they are declared before or after it: the two layers show the direction of
relationships, not declaration order. Source order reads each crate from its
root: items in declaration order, with a module's items at the position of its
`mod` declaration. The focus type's crate comes first, then the other workspace
members by package name. Field and variant order is never changed to improve
routing. The same source and rendering settings produce the same arrangement;
hash-map iteration or compiler field order must not determine card order.

Lines point from the source component toward the focus card. Prefer a straight
vertical segment; any necessary bends are right angles. Routes must avoid text
and unrelated card interiors. A relationship whose target is the focus itself
leaves its cell through the card's bottom edge, passes outside the card, and
arrives at its top edge like any incoming line; no second card is drawn.
Mutually recursive types need no rule: each is an ordinary incoming type of the
other. This RFC does not establish a general routing algorithm or a planarity
guarantee for arbitrary numbers of incoming relationships.

The renderer keeps the layers close enough to read as one view. It does not
reserve space for omitted target cards.

## 5. Representation labels

Read representation from Rust's `repr` attributes. Show `repr(Rust)` when a
nominal type uses the default, and preserve explicit representations and
modifiers, including `C`, `transparent`, integer enum representations, `align`,
and `packed`. Rust decides which combinations are valid. An alias has no
independent representation label.

Representation does not specify a relationship kind. `repr(C)` on one type does
not change the representation of its field types, and a `repr(Rust)` struct need
not store fields in source order.
[Rust Reference: representations](https://doc.rust-lang.org/reference/type-layout.html#representations).

## 6. Command-line interface

The command selects a flow or a focus type by its Rust path:

```text
cargo kaalang diagram [-p <package>] --flow <item> [--collapse-cycles] [-o <path>]
cargo kaalang diagram [-p <package>] --type <item> [-o <path>]
```

The item is the declaration's Rust path from its crate root: `model::Member`, or
`review::Report::approve` for a method. The command runs inside a cargo
workspace and searches every member crate, for the item and for incoming types.
When the path exists in several packages, `-p` names the package to take it
from. Each target of a package (its library, binaries, tests, examples, and
benches) is a separate crate; a path declared in more than one of them cannot be
selected. Exactly one of `--flow` and `--type` is given. `--collapse-cycles`
applies to flows and is rejected with `--type`. Without `-o`, the command writes
`./<name>.svg` from the last path segment, or `./<name>_collapsed.svg` with
`--collapse-cycles`. A path that names no `#[kaa]` declaration of the requested
kind is a failure; one that matches several without `-p` is a failure that lists
them with their packages. The command creates or replaces the output only after
the selected declaration and, for a type, its incoming types are validated and
the diagram rendered. On success it prints the output path; on failure it writes
a diagnostic to stderr and exits with a nonzero status.

This supersedes three provisions of [RFC 0003](0003-svg-renderer.md): the part
of §4 under which the renderer selects one function by its unqualified name and
does not descend into inline modules, §5, which selects a flow by source file
and name, and the part of §6 under which the renderer does not search an entire
crate or resolve external modules. The renderer instead selects a flow by its
path within the file: the enclosing inline modules, then the function's name. A
method's path starts with the last segment of its `impl` block's self type, and
a trait's default method starts with the trait's name. Conditional compilation
and macro-expanded source remain outside the renderer's scope: declarations
under `cfg` are read as written.

## 7. Validation and acceptance scenarios

The annotation parser must diagnose duplicate or malformed `kaa` attributes,
unsupported declaration forms, an unknown name under `kaa::`, empty
descriptions, a `value`, `refer`, or `case` without its description, repeated or
conflicting field, variant, and alias annotations, a `case` outside an enum
variant, a declaration-level `value`, a declaration-level `refer` on anything
but an alias, and parameterized `refer` targets at their source locations. An
unannotated field is not a missing-description error.

Relationship resolution must diagnose an unknown target, including a type
parameter of the enclosing declaration. It must preserve alias names and must
not substitute field storage types for explicit relationship targets. Checking
references across declarations requires the selected source context; this RFC
does not assume that an attribute invocation can discover it alone.

Acceptance scenarios:

| Scenario                                               | Required result                                                    |
| ------------------------------------------------------ | ------------------------------------------------------------------ |
| Named, tuple, and unit structs; enums; unions; aliases | Preserve the Rust declaration form.                                |
| Bare `#[kaa]` on a type                                | Draw the card without a description line.                          |
| Unequal enum variant field counts                      | Draw each variant's own product.                                   |
| `case` on a unit variant                               | Draw the description beside the variant's name.                    |
| `value` and an unannotated field                       | Show values without inferring relationships.                       |
| `refer(Member)` on `MemberId`                          | Keep `Member` as the target.                                       |
| Aliases sharing a right-hand side                      | Keep their authored names and descriptions.                        |
| Declaration-level `refer` on `History`                 | Associate it with the alias, without inventing a field.            |
| `refer(Attachment)` on `Attachment<Payload>`           | Create exactly one relationship to `Attachment`.                   |
| `refer(T)` inside `Attachment<T>`                      | Reject the type parameter as a target.                             |
| A field referring to the focus itself                  | Route the line around the card; draw no second card.               |
| Multiple fields referring to the focus                 | Draw the incoming type once and retain the separate relationships. |
| An unannotated container or a large array              | Keep one field cell, with no element expansion.                    |
| Focus view                                             | Draw incoming types above it and omit target cards below.          |
| The item path declared in two packages                 | Fail listing both packages; `-p` selects one.                      |
| The item path declared in two crates of one package    | Fail listing both crates, even with `-p`.                          |
| Incoming types from two crates                         | Order the focus type's crate first, then by package and target.    |

## 8. Compatibility and exclusions

This RFC adds a proposed type view alongside flows. Beyond the provisions §2.5
and §6 name, it does not supersede the execution, lowering, or control-flow
drawing rules in the accepted RFCs. Their flow terms retain their existing
meanings. A type relationship is not evidence of execution order or of a value
being moved or borrowed.

The annotations must preserve ordinary Rust type behavior, including native
representation attributes. They add no runtime registry or generated lookup. The
attribute emits the declaration unchanged, removing only its `value`, `refer`,
and `case` annotations; derives, representation attributes, and conditional
compilation pass through. Type cards show no traits, methods, or behavior, and
whole-domain diagrams are outside this RFC's scope.

## 9. Open questions

Before implementation, resolve:

- **Discovery and resolution.** How the workspace's declarations are found and
  how a path or a `refer` target is resolved across crates, modules, and
  imports. An attribute macro receives its item, not an already resolved graph
  of neighboring declarations.
  [Rust Reference: attribute macros](https://doc.rust-lang.org/reference/procedural-macros.html#attribute-macros).
- **Large incoming sets.** Routing, grouping, and placement when incoming cards
  no longer fit comfortably in one row, while keeping source order and
  deterministic results.

This draft establishes the annotation and visual direction without selecting a
new compiler architecture or promising an implementation schedule.
