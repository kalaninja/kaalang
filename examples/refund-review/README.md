# Refund review example

This example tests executable Contour lowering against a realistic flat graph.
It is intentionally a small policy example rather than a production refund
service.

The canonical graph is [`src/flow.rs`](src/flow.rs). It is an ordinary
`#[contour]` Rust function: parameters introduce `request` and `policy`, block
arrows declare outputs, and every control and data dependency is listed
explicitly.
Every block has required natural-language intent as a nonempty Rust string in
its marker attribute. The descriptions, inputs, arrow outputs, and bodies form
the graph an author can hand to an implementation agent. Ordinary source
comments may remain nearby but have no graph semantics. `#[contour]` lowers the
graph without creating runtime closures.

## Diagram

```text
                  ╭────────────────────────────────────────╮
                  │ Start: review_refund(request, policy)  │
                  ╰────────────────────┬───────────────────╯
                                       │
                                       ▼
                  ┌────────────────────────────────────────┐
                  │ ? Is the refund request valid?         │── invalid / no ──▶ [Reject invalid] ───────────────┐
                  └────────────────────┬───────────────────┘                                                    │
                           valid / yes │                                                                        │
                                       ▼                                                                        │
                  ┌────────────────────────────────────────┐                                                    │
                  │ ? Is the refund eligible?              │── ineligible / no ─▶ [Reject ineligible] ──────────┤
                  └────────────────────┬───────────────────┘                                                    │
                        eligible / yes │                                                                        │
                                       ▼                                                                        │
                  ┌────────────────────────────────────────┐                                                    │
                  │ Authorize the refund.                  │                                                    │
                  └────────────────────┬───────────────────┘                                                    │
                              approved │                                                                        │
                                       ▼                                                                        │
                  ╭────────────────────────────────────────╮                                                    │
                  │ End: RefundDecision                    │◀───────────────────────────────────────────────────┘
                  ╰────────────────────────────────────────╯
```

Start and End visualize the function boundary; neither is an authored block.

The example demonstrates that question outputs contain no request data: every
downstream action lists both its branch control and `&request` again.

The function return type is the terminal contract. Each terminal action has one
unconsumed result, and a future descriptor or renderer will connect those
mutually exclusive results to one synthetic DRAKON End icon.

The generated block bodies do not change the graph topology. `#[contour]`
lowers them to ordinary Rust control flow without a runtime scheduler. The
integration test executes the approved, validation-rejected, and
eligibility-rejected terminal branches.

Run all syntax checks from the repository root:

```sh
cargo test --workspace
```
