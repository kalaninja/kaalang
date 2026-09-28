# kaalang

[![crates.io][crates-io-badge]][crates-io] [![docs.rs][docs-rs-badge]][docs-rs]
[![CI][ci-badge]][ci]

> We be of one blood, thou and I - man and snake together.
>
> Kaa - from Rudyard Kipling's [“The Spring Running”][spring-running], _The
> Second Jungle Book_

kaalang is a visual flow dialect of Rust for understanding what a program does
and how its parts fit together. Inspired by DRAKON, it makes the structure of an
algorithm explicit in code.

You write an algorithm as described steps, decisions, and cycles, with explicit
inputs and outputs. Rust expressions implement the computations inside those
blocks. The same validated flow becomes ordinary Rust code and a readable
diagram.

Whether written by a person or AI, the code keeps its decisions and dependencies
explicit, while descriptions explain each step's purpose. That makes the flow
easier to read, explain, change, and review.

_Work in progress._

## A first flow: FizzBuzz

A flow is a Rust function marked with `#[kaalang]`. Here, it chooses what to say
based on whether a number is divisible by three, five, both, or neither.

```rust
use kaalang::kaalang;

#[kaalang]
fn fizzbuzz(number: u32) -> String {
    #[choice("Which of 3 and 5 divide the number?")]
    #[case("Both 3 and 5.")]
    #[case("Only 3.")]
    #[case("Only 5.")]
    #[case("Neither 3 nor 5.")]
    let (fizz_buzz, fizz, buzz, plain) = |number| match (number % 3, number % 5) {
        (0, 0) => (),
        (0, _) => (),
        (_, 0) => (),
        _ => number,
    };

    #[action("🎉 Use `FizzBuzz` for this number.")]
    let end = |fizz_buzz| String::from("FizzBuzz");

    #[action("🫧 Use `Fizz` for this number.")]
    let end = |fizz| String::from("Fizz");

    #[action("🐝 Use `Buzz` for this number.")]
    let end = |buzz| String::from("Buzz");

    #[action("🔢 Use the number itself, written as text.")]
    let end = |plain| plain.to_string();

    |end| return end;
}
```

[![Diagram of the FizzBuzz flow](crates/kaalang/tests/gallery/fizzbuzz/fizzbuzz.svg)](crates/kaalang/tests/gallery/fizzbuzz/fizzbuzz.svg)

[Source](crates/kaalang/tests/gallery/fizzbuzz/mod.rs)

## Reading the flow

A **wire** is a named value passed between blocks. The function parameter
provides the first wire, `number`. Each block lists the wires it captures
between `|...|` and names the wires it produces after `let`.

| In the example                             | What happens                                                                                                                     |
| ------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------- |
| `let (fizz_buzz, fizz, buzz, plain) = ...` | The `choice` evaluates its `match` and provides only the selected output. Cases, outputs, and match arms correspond by position. |
| `let end = \|fizz\| ...`                   | Capturing `fizz` places this action on the Fizz branch, even though its body does not use that unit value.                       |
| Four declarations of `end`                 | The branches are mutually exclusive. Their results merge into one wire for the shared continuation.                              |
| `\|end\| return end;`                      | The flow returns whichever branch supplied `end`.                                                                                |

For `number = 9`, only `fizz` is selected, so only the “Use `Fizz` for this
number” action runs. The diagram's connections show execution paths; input and
output names appear as labels along those paths.

The `#[kaalang]` macro translates the capture notation into ordinary Rust
bindings and control flow. Blocks execute in source order along the selected
path. Rust checks types, ownership, borrowing, and lifetimes.

The diagram is generated from the same validated model used to produce the Rust
code. kaalang checks that the flow admits a diagram under its visual rules
before compiling it.

## Anatomy of a block

Here is the last action from FizzBuzz, with its Rust body wrapped in braces:

```rust
#[action("🔢 Say the number itself.")]  // kind + description
let end = |plain| {                     // output = |inputs|
    plain.to_string()                   // Rust body
};
```

| Part                 | In this action          | Variations                                                                          |
| -------------------- | ----------------------- | ----------------------------------------------------------------------------------- |
| Kind and description | `#[action("...")]`      | Both are required for an action. The description labels its diagram node.           |
| Outputs              | `let end =`             | Omit for an effect returning `()`. Use `let (left, right) =` to expose two outputs. |
| Inputs               | `\|plain\|`             | Omit it when there are no inputs. Every wire used by the body must be listed here.  |
| Body                 | `{ plain.to_string() }` | A Rust expression. An action may omit the braces, as in the full example.           |

Actions can also create a value without inputs, consume inputs without producing
outputs, or perform an effect with neither:

```rust
// No inputs: the capture list can be omitted.
#[action("Start with fifteen.")]
let number = 15;

// No outputs.
#[action("Print the result.")]
|&end| { println!("{end}") };

// Neither: the empty capture list can be omitted too.
#[action("Announce the flow.")]
{
    println!("start");
};
```

Captures express how an action accesses a wire:

| Input       | Access                                     |
| ----------- | ------------------------------------------ |
| `name`      | Move or copy the value.                    |
| `mut name`  | Move or copy into a mutable local binding. |
| `&name`     | Borrow the value.                          |
| `&mut name` | Mutably borrow a wire declared mutable.    |

See the
[capture rules](docs/rfcs/0001-language.md#6-wires-producers-and-captures) for
the details.

## Block kinds

FizzBuzz uses `choice` and `action`. The other kinds let a flow ask a yes/no
question, call an existing function, or repeat a sequence:

| Kind       | Role in a flow                                                               |
| ---------- | ---------------------------------------------------------------------------- |
| `action`   | Perform a computation or effect with a Rust expression.                      |
| `call`     | Call one named Rust function. Its name supplies the description if omitted.  |
| `question` | Select one of two branch outputs using a boolean expression.                 |
| `choice`   | Select one output per visit from the cases of a Rust `match`.                |
| `cycle`    | Repeat a nested kaalang sequence whose blocks capture the surrounding wires. |
| `continue` | Start the next iteration of the directly containing cycle.                   |
| `return`   | Complete the root flow and hand back its result.                             |

Actions, calls, and cycles can have no outputs. Questions and choices always
declare their branch outputs. A cycle contains kaalang blocks; each route
through it either repeats at its one `continue` or reaches the end of the body
with one of the cycle's
[declared outputs](docs/rfcs/0007-language-refinements.md#22-alternative-outputs),
which then becomes available after the cycle. In the diagram, `continue` routes
meet at the cycle's back edge, completing routes leave through the cycle
boundary, and `return` reaches the flow end.

Flows with stages are defined in [RFC 0006](docs/rfcs/0006-stages.md) and drawn
as [silhouettes](docs/rfcs/0006-stages.md#7-visual-representation). See the
executable [KMP search example](crates/kaalang/tests/gallery/kmp_search/mod.rs).

The [language RFC](docs/rfcs/0001-language.md#4-block-kinds) and its proposed
[refinements](docs/rfcs/0007-language-refinements.md) define each kind; the
[visual language RFC](docs/rfcs/0002-visual-language.md#4-node-kinds) defines
its representation.

## Putting it together: binary search

Searching a sorted slice adds state and repetition. Each iteration checks
whether any candidates remain, compares the middle value with the target, and
either narrows the range or finishes with `Some(index)` or `None`.

```rust
use std::cmp::Ordering;

use kaalang::kaalang;

#[kaalang]
fn binary_search(values: &[i32], target: i32) -> Option<usize> {
    #[action("📏 Start with the entire sorted list.")]
    let (mut left, mut right) = |values| (0, values.len());

    #[cycle("🔍 Narrow the range until the target is found or ruled out.")]
    let result = {
        #[question("Are any values left in the search range?")]
        #[yes("YES")]
        #[no("NO")]
        let (iterate, leave) = |left, right| left < right;

        #[action("🚫 Report that the target is absent.")]
        let result = |leave| None;

        #[action("📍 Select the middle value of this range.")]
        let (mid, value) = |iterate, values, left, right| {
            let mid = left + (right - left) / 2;
            (mid, values[mid])
        };

        #[choice("How does this value compare with the target?")]
        #[case("Less than the target.")]
        #[case("Greater than the target.")]
        #[case("Equal to the target.")]
        let (less, greater, equal) = |value, target| match value.cmp(&target) {
            Ordering::Less => (),
            Ordering::Greater => (),
            Ordering::Equal => (),
        };

        #[action("➡️ Discard this value and everything to its left.")]
        let stepped = |less, mid, &mut left| *left = mid + 1;

        #[action("⬅️ Discard this value and everything to its right.")]
        let stepped = |greater, mid, &mut right| *right = mid;

        #[action("🎯 Report the position of this matching value.")]
        let result = |equal, mid| Some(mid);

        |stepped| continue;
    };

    |result| return result;
}
```

[![Diagram of binary search in kaalang](crates/kaalang/tests/gallery/binary_search/binary_search.svg)](crates/kaalang/tests/gallery/binary_search/binary_search.svg)

[Source](crates/kaalang/tests/gallery/binary_search/mod.rs)

The cycle's blocks capture the surrounding `left` and `right`, declared `mut` so
the `less` and `greater` branches can update them. Both then produce `stepped`,
merging before the shared `continue` that starts the next iteration. The `equal`
and `leave` branches instead produce `result`, the cycle's declared output. They
merge at the end of the body, and the cycle hands `result` to the flow, which
returns it.

<details>
<summary>See the same flow with its cycle collapsed</summary>

[![Binary search with its cycle collapsed](crates/kaalang/tests/gallery/binary_search/binary_search_collapsed.svg)](crates/kaalang/tests/gallery/binary_search/binary_search_collapsed.svg)

The cycle's description and interface remain visible while its body is hidden.

</details>

Both examples are [executable gallery tests](crates/kaalang/tests/gallery), with
SVGs generated from their source. The gallery also includes
[swap](crates/kaalang/tests/gallery/swap/mod.rs), a flow that returns its inputs
without a computational block,
[bubble sort](crates/kaalang/tests/gallery/sorting/bubble_sort.rs), with nested
cycles, [quicksort](crates/kaalang/tests/gallery/sorting/quick_sort.rs), with
stages for in-place partitioning and recursion, and
[KMP search](crates/kaalang/tests/gallery/kmp_search/mod.rs), with stages for
comparison, advancement, and prefix fallback.

## Try it

Install the released CLI from crates.io:

```sh
cargo install kaalang-cli
```

Or install the version from the current repository checkout:

```sh
cargo install --path crates/kaalang-cli
```

From a repository checkout, draw the FizzBuzz example:

```sh
cargo kaalang diagram crates/kaalang/tests/gallery/fizzbuzz/mod.rs --flow fizzbuzz -o fizzbuzz.svg
```

Open `fizzbuzz.svg` in a browser. Replace the source path and flow name to
render another example; add `--collapse-loops` for the compact cycle view shown
above. The output is a standalone SVG with no external rendering tools required.

## Read more

- [RFCs](docs/rfcs/), the source of truth for syntax, semantics, and diagrams.
- [Contributing](CONTRIBUTING.md)

[spring-running]: https://www.gutenberg.org/cache/epub/37364/pg37364-images.html
[ci]: https://github.com/kalaninja/kaalang/actions/workflows/ci.yml
[ci-badge]:
  https://github.com/kalaninja/kaalang/actions/workflows/ci.yml/badge.svg
[crates-io]: https://crates.io/crates/kaalang
[crates-io-badge]: https://img.shields.io/crates/v/kaalang.svg
[docs-rs]: https://docs.rs/kaalang
[docs-rs-badge]: https://docs.rs/kaalang/badge.svg
