# RFC 0010: Type card paths

- Status: accepted
- Types: [RFC 0009: Types](0009-types.md)

## 1. Motivation and scope

Types beside the focus often repeat its module path in every card. A diagram of
`order::Order`, for example, repeats `order::` before `Payment` and `Shipment`.
Keep the focus's full path from its crate root as context and shorten each
incoming card's path relative to it.

This supersedes the crate-root path rule for incoming card headers in RFC 0009
§4.2.

## 2. Displayed paths

For each incoming card in the focus type's crate, remove the longest leading
sequence of whole module segments shared with the focus type's module path.
Compare each incoming card with the focus independently. Never remove a type's
final segment.

A card from another crate keeps its crate name and full path from that crate's
root, even when its module segments match the focus's. Generic parameters and
lifetimes still follow the displayed type name.

For a focus of `shop::orders::Order`:

| Declaration                                        | Card header                    |
| -------------------------------------------------- | ------------------------------ |
| `shop::orders::Order`                              | `shop::orders::Order`          |
| `shop::orders::Line`                               | `Line`                         |
| `shop::orders::archive::Entry<T>`                  | `archive::Entry<T>`            |
| `shop::payments::Payment`                          | `payments::Payment`            |
| `shop::ordering::Note`                             | `ordering::Note`               |
| `shop::Summary`                                    | `Summary`                      |
| `shopping::Receipt`                                | `shopping::Receipt`            |
| `external::shop::orders::Line` in crate `external` | `external::shop::orders::Line` |

### 2.1 Collisions

Compare the shortened paths against every card's displayed path, including the
focus and cards from other crates. If paths coincide, restore one preceding
module segment to every conflicting path that was shortened. Repeat the
comparison for the entire diagram: restoring a segment can create a collision
with a third card. The focus and already complete paths stay unchanged.

For example, `shop::orders::Payment` and `shop::Payment` both first become
`Payment`. Restore them to `orders::Payment` and `shop::Payment`. If another
card already displays `orders::Payment`, restore the first path further to
`shop::orders::Payment`.

Each round restores at least one segment or finishes. Stop when no path can be
extended. This resolves collisions introduced by shortening; it does not rename
crates whose complete displayed paths already coincide. Compare paths before
generic parameters and lifetimes, so these do not distinguish otherwise equal
paths.

### 2.2 Other labels

The diagram title, accessible description, and relationship titles use the same
displayed names as the cards. Authored field types and relationship targets
remain literal. CLI queries, name resolution, diagnostics, and output filenames
keep their existing rules.

## 3. Acceptance scenarios

Check same-module, nested-module, sibling-module, ancestor-module, and unrelated
paths. A textual prefix inside one segment is not a shared module. Root types
keep their names, the focus keeps its full path with or without incoming types,
and a dependency rename does not change another crate's displayed name or trim
its module path. Self-reference titles keep the full focus path, and incoming
generic card headers keep their parameters after the shortened path.

Check collisions between shortened paths, with a full root path, with the focus,
and with a path from another crate. Include a collision introduced by restoring
a segment, and retain short labels that do not collide.
