# Sample workspace

The workspace every supported IDE opens for manual checks.

## Types

Put the cursor in `Order` (`shop/src/lib.rs`).

| Case                          | Expected                                                                                                                                               |
| ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Same file                     | `Hold` (enum), `Slot` (union), `Backlog` (alias), `Event<T>` (generic), and the focus's own `previous` line to itself.                                 |
| Other files and modules       | `customers::Customer`, the inline `payments::Payment` and `payments::methods::Method`.                                                                 |
| Renamed import                | `aliases::Refund`, whose target is `Purchase`, `use crate::Order as Purchase`.                                                                         |
| Target groups and packages    | Rows: the library with the binary's `shop::Counter`, then the test's `checkout::Basket`, the example's `receipt::Receipt<'a>`, and `billing::Invoice`. |
| Default feature               | The `express` field.                                                                                                                                   |
| Feature off                   | No `gift_note` field and no `GiftWrap` card: the editor shows both dimmed.                                                                             |
| Build-script cfg              | The `shelf` field. The CLI does not run build scripts and leaves it out.                                                                               |
| File included twice           | One `shared::Audit` card: the engine reads `shared.rs` once. The CLI also draws `twin::Audit`.                                                         |
| Plain type as focus           | Cursor in `CustomerId` (`customers.rs`): `Customer` refers to it, though `CustomerId` has no `#[kaa]`.                                                 |
| Ctrl-click a tile's type path | `catalog::Line` in `Vec<catalog::Line>` opens `Line`; `String` opens the standard library in the editor only.                                          |
| Ctrl-click a target or header | A `▸` target opens its definition; an incoming card's header opens that type.                                                                          |

## Flows

| Where                             | Case                                                                                                                                                            |
| --------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `shop/src/flows.rs`, `review`     | Question, choice, three `hold` producers merging, and a call: Ctrl-click `total` opens it, `due` opens `Money`. Click a connection or merge to list its ribbon. |
| `count_items`, `boxes`, `restock` | A `for` cycle, a `loop` cycle leaving through a question, and stages.                                                                                           |
| `Order::ship`                     | A method with its receiver as a wire, and a parameter the off `gift` feature removes.                                                                           |
| `shop/src/generated/ids.rs`       | A flow in a `#[path]` module.                                                                                                                                   |
| `shop/src/main.rs`, `billing`     | Flows that call a flow of the library and of another package.                                                                                                   |

Before the cursor enters a flow, a file's diagram shows its first flow.
