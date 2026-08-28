# Contour

Contour is an experiment in writing flat, DRAKON-inspired graphs as valid Rust.
The current version parses, validates, and directly executes a small synchronous
subset of that graph language.

English is the working language for repository content, including source code,
documentation, examples, tests, and commit messages.

## Current prototype

An ordinary Rust function is the graph boundary. `#[contour]` owns flow-wide
checks and lowering. Local action, question, and choice attributes contain each
block's required natural-language description:

```rust
use contour::contour;

#[contour]
fn decide(request: Request) -> Decision {
    #[question("Is the request valid?")]
    |&request| -> (valid, invalid) { todo!() };

    #[action("Approve the valid request.")]
    |valid, &request| -> approved { todo!() };

    #[action("Reject the invalid request.")]
    |invalid, &request| -> rejected { todo!() };
}
```

This is a complete author-written graph skeleton. An implementation agent
replaces only `todo!()` without changing descriptions, captures, output
bindings, or routing:

```rust
#[question("Is the request valid?")]
|&request| -> (valid, invalid) { request.is_valid() };

#[action("Approve the valid request.")]
|valid, &request| -> approved {
    Decision::approved(request)
};
```

A choice makes every possible route visible without putting Rust patterns in
the topology:

```rust
#[choice("What is the sign of the value?")]
#[case("The value is negative.")]
#[case("The value is zero.")]
#[case("The value is positive.")]
|value| -> (negative, zero, positive) {
    match value {
        ..0 => (),
        0 => (),
        _ => (),
    }
};
```

Case descriptions map positionally to the output names. An implemented choice
body is exactly one Rust `match` with the same number of arms. Cases, outputs,
and arms correspond by position. The selected arm expression becomes the value
of its output wire, so `Some(value) => value` carries `value`, while `None => ()`
creates a unit-valued control wire. Different arms may produce different
payload types because they feed different output wires. Only exact whole-body
`todo!()` is accepted as an unimplemented choice.

A nonempty string literal inside every marker attribute is the block's exact
natural-language intent. An adjacent `//` comment remains an ordinary source
comment and does not affect the graph.

The closure-shaped statement lists inputs before `->`, outputs after it, and
contains the leaf implementation. `name` consumes a wire and `&name` borrows
it. Every name is one wire, never a bundle. A question has exactly two
positional outputs; a choice has a tuple with one output per case; an action may
declare an identifier or tuple.
`todo!()` means "not implemented" and panics only if execution reaches that
block.

Consumption is a graph rule independent of Rust's `Copy` trait. Lowering stores
wires under internal bindings and exposes a wire's source-level name only as a
local alias for an input listed by that block. Rust scoping therefore rejects
hidden reads of omitted or already consumed wires, including `Copy` values.

This is valid Rust syntax and is formatted by `rustfmt`. `#[contour]` consumes
the closure-shaped expression as graph syntax, reinterpreting Rust's return-type
position as output names. Generated code does not create or call a runtime
closure: it creates ordinary `let` bindings, and later blocks explicitly name
every incoming connection:

- the first question output is yes/true and the second is no/false;
- choice cases, outputs, and match arms correspond positionally;
- control outputs carry no hidden data;
- every reachable path ends at an action with one unconsumed output;
- the function return type is the contract for all terminal outputs.

There is no public End block. The function boundary already supplies one
logical end, which a future descriptor or renderer can represent as a synthetic
DRAKON End icon.

## Execution

`#[contour]` parses the flat block statements, checks descriptions, wire-name
uniqueness, declaration order, question arity, reachability, and terminal paths,
then lowers the graph to ordinary nested Rust `if`, `match`, and `let`
expressions. Rust checks body types, ownership, and the function return type. A
question or choice body is evaluated once and only its selected branch runs.
Choice lowering keeps the authored `match` and passes the selected arm value to
the corresponding continuation; it creates no selection enum or second
dispatch.
There is no runtime scheduler or wire wrapper.

The current subset requires exactly one next block to be ready at every point;
parallel paths, joins, and merges remain unsupported. The local marker
attributes are valid only inside a `#[contour]` function and need no import or
separate definition.

Block IDs, descriptors, scheduling, rendering, loops, merges, and subflows are
deliberately deferred. See
[RFC 0001](docs/rfcs/0001-core-model.md) for the draft contract and the
[`playground`](examples/playground) for executable examples. Its current
canonical graph is the refund review.

## Run

```sh
cargo test --workspace
```
