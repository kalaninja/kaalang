# kaalang Glossary

The names used across kaalang's documentation, crates, diagnostics, and tests.

## Language

- **flow**: one function marked `#[kaa]`; its **flow inputs** are its parameters
  and its **flow output** is the structural `return` value.
- **block**: one authored statement of a flow; **computational block** when it
  has a Rust body (action, call, question, choice).
- **block kinds**: action, call, question, choice, cycle, continue, return.
- **description**: the string in a block's attribute, such as
  `#[kaa::action("...")]`, and the text of a question branch, a case, or a
  stage. A call without one is described by the function it names. A datatype, a
  field's `value` or `refer`, and an enum variant's `case` carry one too.
- **wire**: a named value a producer provides for capture.
- **input**, **output**: a block's captured wires and the wires it produces.
- **producer**: one occurrence of a named flow input or block output that
  provides a wire.
- **alternative producers**: mutually exclusive producers of the same logical
  wire, joined by an implicit merge before capture.
- **consumer**: a block whose input captures a wire.
- **capture**: an input's association with a logical wire, resolved to an
  available producer whenever the consumer runs.
- **capture dependency**: a producer-to-consumer dependency that exists when
  some execution resolves a capture to that producer.
- **selector**: a question, a choice, or a cycle with alternative outputs: a
  block whose run selects one of its outcomes.
- **branch**: one alternative continuation selected by a selector.
- **branch output**: a selector's output that selects a branch and provides its
  wire. A cycle's single output is an ordinary result.
- **case**: the description of one choice branch, not a block, or of one enum
  variant.
- **branch ancestry**: the branches a block belongs to through its captures and
  enclosing cycles; merged wires retain shared ancestry.
- **decides**: a selector decides a block when changing only that selection can
  change whether the block runs, among executions that reach its comparison
  context.
- **continuation**: the authored blocks whose participation can depend on a
  branch output through captures and merges.
- **branch set**: for one selector and a downstream block, the branches whose
  continuations contain that block.
- **convergence group**: at least two branches with shared continuation; group
  overlap is compared by route.
- **shared continuation**: the nonempty set of blocks whose branch set is
  exactly one convergence group.
- **continuing branch**: a branch belonging to a convergence group, even if some
  of its nested routes finish without entering the shared continuation.
- **terminal branch**: a branch belonging to no convergence group of its
  selector.
- **entry block**: a block with no predecessor in its shared continuation; its
  inputs are consumed after their merges.
- **merge**, also **convergence point**: the implicit junction of alternative
  producers of one wire before any capture.
- **frame**: one sequence level: the root, preparation, a stage body, or one
  iteration of a cycle body.
- **cycle**: the only name for the kaalang repetition block. A loop cycle
  completes through a declared output and repeats at `continue`; a for cycle
  completes when its items run out. "loop" names only Rust's `loop`, the loop
  cycle form, and the loop marker.
- **iteration**: one execution of a cycle body.
- **gate**: a wire whose availability lets a block or transfer take part in a
  route; a cycle header may name one.
- **export**: passes one selected value out of a cycle at its **result
  boundary**, or to another stage through a transition. At a cycle, a **boundary
  consumer** is the implicit by-value use of the exported wire.
- **loop cycle**, **for cycle**: the two cycle forms, with a Rust `loop` body or
  a Rust `for` loop.
- **item wires**: the wires a for cycle's item pattern binds for one iteration.
- **for-entry**, **for-end**: the caps above and below an expanded for cycle's
  body.
- **preparation**: the blocks of a staged flow before its first stage; they run
  once and select the initial stage.
- **stage**: a named sequence entered through one incoming signal; a **visit**
  is one run of its body, with fresh local wires.
- **transition**: ends a route by selecting the next stage; its **signal** names
  that stage and carries the value it receives.
- **entry**: names four things. An entry block starts a shared continuation; a
  stage entry is the wire a stage receives and the node that draws it; a cycle
  entry is where its body begins and its back edge returns; a for-entry is a for
  cycle's top cap.
- **datatype**: a struct, enum, union, or type alias marked `#[kaa]`; a **plain
  type** is one without the mark.
- **relationship**: one `refer` annotation, from a field or an alias to the type
  it names, a datatype or a plain type; neither a wire nor a connection.
- **compilation target**: the platform the source is read for, named by a triple
  such as `x86_64-unknown-linux-gnu`; its **cfg set** decides which `cfg` and
  `cfg_attr` predicates hold. Not a relationship's target, nor a package's
  target such as its library or tests.

## Diagram

- **diagram**: the complete drawing of one flow; a **type diagram** draws one
  focus type with its incoming types.
- **focus type**, **incoming type**: the type a type diagram is drawn for, a
  datatype or a plain type, and a datatype with a relationship to it.
- **card**: the drawing of one type: its name with its form and representation,
  its description when it has one, and its tiles.
- **tile**: the drawing of one field, enum variant, or alias right-hand side
  inside a card.
- **groove**, **bus**, **port**, **trunk**: the strip in the focus under each
  enum variant; the line a row's lines drop into, along a groove or below a
  struct's row; the square where a groove's bus leaves the focus; and the line
  beside a column of incoming cards, or beside the focus, that their lines join
  on the way to the focus.
- **node**: a drawn unit for a block, a merge, or the synthetic start and end.
- **node kinds**: start, stage entry, action, call, question, select, case,
  cycle, for-entry, for-end, transition, merge, end. A return draws only its
  route to end.
- **connection**: one drawn control-flow link; not a wire.
- **exit**, **hand-over**, **distributor**: where connections leave a node, the
  wires labeled there, and the shared segment a select's cases fan out from.
- **label**, **parameter panel**: text on a node or connection, and the panel
  beside start listing the flow's Rust parameters.
- **column**, **row**: the vertical and horizontal positions nodes and segments
  align to; in a type diagram, those of the incoming cards.
- **cycle boundary**: the region drawn around an expanded loop cycle; its body
  stays inside and its interface sits on the edge. An expanded for cycle has its
  caps instead.
- **iteration back edge**, also **back edge**: the climb that repeats a loop
  cycle, from its **iteration tail**, where an iteration ends, to the cycle
  entry.
- **loop marker**: the sign on an expanded loop cycle's boundary and on every
  collapsed cycle node.
- **footprint**: the columns a convergence group's branches and shared
  continuation occupy.
- **formula**, **restricted Markdown**: the notation a description may use.

## Implementation

### Analysis

- **local flow**: a flow, or preparation or one stage of a staged flow, analyzed
  and arranged on its own; a staged flow's local flows are its **parts**.
- **execution**, also **history**: a finite summary of one run of a local flow
  with at most one iteration per cycle: selected branches, blocks run, capture
  dependencies, and outcome (`Execution`). `Executions` holds them in one of two
  representations.
- **enumeration**: the ordinary execution list, used while the count stays
  within `ENUMERATED_HISTORY_LIMIT`.
- **decision graph**: the symbolic alternative above that limit: an ordered,
  reduced graph whose **conditions** each stand for a set of executions. Its
  **observations** are concrete executions that witness those conditions so
  projection can take union facts.
- **execution plan**: the verified nested branch and join plan that lowering
  emits (`ExecutionPlan`). A **join** is where the plan rejoins a selector's
  branches; it is distinct from a convergence group.
- **analysis**: the flow, its plan, executions, merges, convergence groups, and
  stage analyses (`Analysis`).
- **semantic model**: an analysis with its topology and verified arrangement,
  plus one model per stage (`SemanticModel`).
- **projection**: building a topology from an analysis; **expanded** draws each
  cycle as a region, **collapsed** draws it as one node.

### Topology

- **topology**: every drawn vertex, the connections between them, the
  placement-only order, and the cycles' back edges (`Topology`).
- **vertex**: a node or a junction as a unit of placement (`Vertex`).
- **junction**: the point where a merge's producers meet, or a cycle or transfer
  boundary, with no computation of its own; merges with identical arrivals share
  one (`Junction`).
- **placement-only order**: precedence the arrangement respects but never draws:
  iteration ends before what follows them, a cycle body before its result or
  for-end, and every sink before end and each transition (`Topology::order`).
- **cycle** (in a topology): a repeating loop cycle's header, entry, and tail;
  for cycles have no back edge and are not listed (`Cycle`).

### Arrangement

- **arrangement**: abstract ranks, columns, exit offsets, routes, gap lanes,
  contours, and back routes of one topology; a presentation assigns dimensions
  and spacing and may not reorder or re-route it (`Arrangement`).
- **rank**: a vertex's abstract row.
- **gap**: the space below a rank, before the next one; it holds lanes.
- **lane**: a horizontal routing line inside a gap, or, for a contour, one step
  outside a body's outermost column.
- **route**, also **corridor**: a connection's path as a departure column, an
  arrival column, and sideways **runs**, each on a rank line or a gap lane
  (`Route`, `Run`, `RunLine`).
- **corridor shape**: where a forward route turns, widened on conflict from
  direct to deferred to aside.
- **exit offset**: the displacement from a node's column to its exit's default
  branch column (`Arrangement::exit_offset`).
- **exit column**: the absolute branch column an exit owns: its node's column
  plus its exit offset (`Arrangement::exit_column`).
- **contour**: the side, column boundary, and lane of a straight back-edge climb
  (`Contour`); a **back route** records the runs of a bent climb.
- **witness**: an arrangement a search returns; it is accepted only after the
  independent checks pass.
- **preferred search**: the conflict-guided first search.
- **deciding sweep**: the fallback search, with straight then flexible back
  edges.
- **obstruction**: a reported reason a candidate fails, which guides the
  preferred search's next attempt (`Obstruction`).
- **arrangement checks**: the independent verification every witness passes,
  shared with the renderers (`ArrangementChecks`).
- **normalization**: rewriting a witness to canonical columns and dense lanes.

### Rendering

- **compaction**: iterated verified local simplifications of an arrangement to a
  deterministic local fixed point.
- **arrangement verifier**: the arrangement checks plus cycle-boundary checks,
  applied to every compaction candidate (`ArrangementVerifier`).
- **captions**: the labels and source text derived from a model for each node,
  exit, junction, and cycle (`Captions`).
- **scene**: the pixel realization of a compacted arrangement: placed nodes,
  labels, connections, and cycle regions (`Scene`); a **staged scene** composes
  the parts with their transitions on one row (`StagedScene`).
- **narrow gaps**, **standard spacing**, **slack**: the first pass gives
  routing-only columns narrow gaps and retries with standard spacing; slack is
  the extra gap added while labels and back edges collide.
- **correspondence**: the check that pixels preserve the arrangement's rows,
  columns, ports, and contour anchors.

### Tests

- **fixture**: a test source file of flows or types. A flow fixture holds one
  executable flow, a type fixture the types of one case, a gallery file may
  group several, a stress fixture is a heavy hand-written flow, and a
  compile-fail fixture is a rejected program with its diagnostic.
- **corpus**: the flows of every executable fixture; compile-fail fixtures stay
  out.
- **suite**: a corpus selection: ordinary, stress, or all (`Suite`).
- **fixture diagram**: the `.svg` drawn beside its source for each flow, or each
  type, of an executable fixture, redrawn by the test suite and reviewed in the
  diff, never edited by hand.
- **probe**: a generated flow of a chosen size for the performance budgets.
- **shape**: a generated small flow from a family enumerated exhaustively to
  compare searches and renders; unrelated to corridor shapes.
- **budget**: a time limit the performance harness asserts per item or per pass.
