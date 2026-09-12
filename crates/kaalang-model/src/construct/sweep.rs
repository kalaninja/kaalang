//! Decides whether a topology has a conforming diagram at all, by sweeping it
//! from top to bottom.
//!
//! The constructor beside this one searches a family of arrangements shaped
//! like the ones RFC 0002 §8 prefers. It is fast and its results are the ones
//! worth drawing, but it commits to ranks and columns before it routes, so its
//! failure proves nothing. This module decides the question: it accepts a
//! topology exactly when some sequence of the states below both finishes and
//! numbers its columns, and the two directions further down say why that is
//! the same as having a conforming drawing.
//!
//! # Lifelines and states
//!
//! Reverse every loop return, so it runs from the entry down to the tail
//! instead of climbing back. Every connection, every placement-only precedence
//! edge of RFC 0002 §§7–8, and every reversed return then descends, and the
//! vertices form a directed acyclic graph. The placement edges into end make
//! it the final vertex; this is a required order, not a search preference.
//!
//! A *lifeline* is one connection's own vertical: the column it descends in
//! between leaving its source and reaching its destination. A *state* is
//!
//! - the set of vertices already placed, which is closed under that graph, and
//! - the *frontier*: the lifelines crossing the cut below them, left to right.
//!
//! One step places one vertex, so a path through the state space is as long as
//! the topology has vertices.
//!
//! # Steps
//!
//! A vertex `v` can be placed once every predecessor is placed and:
//!
//! - **its incoming lifelines are contiguous in the frontier.** This is the
//!   whole spatial rule. The lifelines `v` consumes meet on its rail, which
//!   spans them, so another lifeline between two of them would have to cross
//!   that rail (RFC 0002 §8) — and it cannot end there, because it does not
//!   join `v`.
//! - **`v` continues the column of the lifeline at one end of that run**, and
//!   which end is forced, not chosen. For a convergence it is the leftmost:
//!   the frontier keeps its order, so the leftmost route reaching `v` is the
//!   one its group's first branch took, and RFC 0002 §8 sends the shared
//!   continuation on down that column. An iteration tail instead takes the end
//!   nearest the side its return climbs, because the return leaves the tail
//!   horizontally along the rail's own line: a tail at the far end would send
//!   its return straight back across every route arriving there. No vertex is
//!   both — a tail is a structural junction and merges no wire — and only the
//!   start node continues nothing.
//! - `v`'s outgoing connections take its place in the frontier, in the authored
//!   order of the exits they leave, and the first of them continues `v`'s own
//!   lifeline. Connections leaving one exit share a run, so their order among
//!   themselves is free and the sweep tries each — unless the exit is a
//!   choice's distributor, where RFC 0002 §8 sends the routes down in the
//!   order of the branches they carry and `Fanout::fixed` keeps them there.
//!   They have to take that place
//!   for the same reason: they leave `v`'s own box sideways before they
//!   descend, so a lifeline between `v`'s column and one of theirs is crossed.
//! - a loop entry also emits its reversed return, immediately outside itself on
//!   one side. That side is the only choice a loop contributes.
//!
//! Nothing else moves. Two lifelines cannot exchange places without crossing,
//! so the frontier keeps its order between steps, sideways movement is
//! invisible to the search, and a row used only for routing is not a state.
//!
//! Two orders the frontier cannot express join it when the columns are
//! numbered. Both are cases of the same thing: the frontier only orders two
//! lifelines while both are alive, and a drawing has to hold once one of them
//! is gone. A convergence group reserves the columns of everything its
//! branches draw, and its shared continuation is drawn below, by which time a
//! sibling's own lifelines may have ended; a return climbs outside every
//! column of its body, including vertices placed after the tail, when the
//! return's lifeline has already left the frontier.
//!
//! Those two are part of the decision, not a formality after it. The walk
//! numbers the columns when it has placed everything, and a sequence whose
//! orders contradict each other is no drawing: the walk discards it and carries
//! on looking, exactly as it does for a vertex it cannot place.
//!
//! The return's side decides the contour outright. Every lifeline of the body
//! is inserted where a body lifeline already was, so the body never reaches
//! past the return, and the return climbs outside the body's whole column
//! range for as long as the body lasts. A loop nested in that body opens its
//! own return inside it, so an enclosing return is outside that one too. That
//! one the frontier cannot give: the two returns may share no row at all, in
//! which case they never stand on it together and never cross either. It is
//! recorded with the column orders instead, beside the body vertices drawn
//! below the tail, which outlive the frontier for the same reason.
//!
//! # Both directions
//!
//! *Every conforming drawing is one of these sequences.* Order the vertices by
//! row. Each connection crosses a horizontal cut once: forward routes descend
//! (RFC 0002 §8) and a return's climb is monotone, so the crossings give the
//! frontier its order, and no two may exchange places without crossing. At a
//! vertex, a lifeline between two the vertex consumes would cross its rail
//! unless it ends there too, so what it consumes is contiguous; and its
//! branches leave left to right in authored order.
//!
//! *Every sequence whose columns number is a conforming drawing.* Rank a vertex
//! by its position in the sequence. Order the lifelines by every adjacency the
//! frontier showed, together with the two orders it cannot express: that
//! relation may have a cycle, and a sequence that produces one is discarded at
//! the end of the walk rather than drawn. Where it has none, number it. No
//! lifeline then passes through a vertex, because no two share a column. Each
//! connection needs at most two sideways runs, one below its source and one
//! above its destination, and one rank gap holds at most one of each — so two
//! lanes per gap are always enough. Both runs span only lifelines their own
//! vertex consumes or emits, so neither crosses anything.
//!
//! The frontier-only part of that relation is acyclic on its own: a connection
//! is active over one unbroken run of ranks, so three lifelines that pairwise
//! overlap share a rank and are ordered there. Only the two added orders can
//! close a cycle.
//!
//! # Ending, and what is pruned
//!
//! Every step places a vertex, so each search path has at most as many steps as
//! there are vertices. Each step has finitely many choices, making the whole
//! search finite without a budget.
//!
//! Two reductions cut the space, and both are held to the same boundary: they
//! apply to a refusal the *state* accounts for, and not to one the column
//! numbering raises.
//!
//! That boundary is the whole of their correctness. A step reads the placed
//! set and the frontier and nothing else, so any claim about what a state can
//! still reach is a claim about those two. The numbering is not such a claim:
//! it reads the finished sequence, including the order of lifelines that died
//! before the state was reached, so two prefixes arriving at one state may
//! record different orders and number differently. Rather than argue that they
//! cannot, `walk` keeps the two refusals apart and spends the extra search
//! where they differ. `Refused::Placement` means every continuation ran out of
//! placeable vertices; `Refused::Numbering` means one of them placed
//! everything and could not number it.
//!
//! *One ready vertex is enough* — for a placement refusal. A lifeline has one
//! destination, so two ready vertices consume disjoint runs of the frontier,
//! and replacing one run leaves the other exactly where it was. Neither
//! placement can therefore make the other unready, and either order reaches
//! the same state. A ready vertex also stays ready until it is placed, for the
//! same reason. So in any sequence that finishes, the vertex this walk picks
//! can be swapped forward step by step to the front, which gives a finishing
//! sequence that starts with it. The swap preserves the state at every cut but
//! not the sequence, so when the refusal was the numbering's, the walk goes on
//! to the next ready vertex instead.
//!
//! *A refused state stays refused* — again for a placement refusal, which the
//! placed set and the frontier determine. A state whose refusal involved the
//! numbering is not remembered at all, so a different route to it is searched
//! afresh.
//!
//! Both are therefore exact, and the search is complete: every sequence the
//! unreduced walk accepts is one this walk reaches. What the reductions cost
//! is bounded by how often the numbering refuses, which is never in the corpus
//! or in any generated shape the suite walks.
//!
//! The cost is exponential in the worst case, in the number of closed vertex
//! sets times the orders their frontier can take, which the emission choices
//! generate: `k!` per fan-out of `k` unrelated destinations from one exit, and
//! two per loop for the side its return takes. The frontier itself is the
//! width of the cut, which stays small for these diagrams.
//!
//! The reductions are what `tests::the_reduced_walk_agrees_with_the_whole_state_space`
//! checks, by walking the same space with neither of them.

use std::collections::{BTreeMap, BTreeSet};

use crate::model::{Flow, WireMerge};
use crate::topology::{ExitId, NodeId, Source, Topology, Vertex};

use super::{Arrangement, Contour, Obstruction, Route, Run, Side};

/// One active vertical of the frontier.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(super) enum Lifeline {
    /// The connection at this index, from its source down to its destination.
    Wire(usize),
    /// The reversed return of this loop, from its entry down to its tail.
    Return(usize),
}

/// The connections leaving one exit, and whether their left-to-right order is
/// the authored one or free.
struct Fanout {
    fixed: bool,
    connections: Vec<usize>,
}

/// One placed vertex, in rank order. Every choice the walk made is recorded
/// here, so the same expansion serves any procedure that makes them
/// differently.
pub(super) struct Step {
    pub(super) vertex: usize,
    /// The lifelines it ends, in the order they stood in the frontier.
    pub(super) consumed: Vec<Lifeline>,
    /// Where the emitted lifelines go, once the consumed ones are gone.
    pub(super) position: usize,
    pub(super) emitted: Vec<Lifeline>,
    /// The lifeline whose column the vertex continues. `None` gives it a
    /// column of its own, which only the start node needs.
    pub(super) continues: Option<Lifeline>,
}

/// Applies one step to a frontier.
pub(super) fn replay(frontier: &mut Vec<Lifeline>, step: &Step) {
    frontier.retain(|lifeline| !step.consumed.contains(lifeline));
    let position = step.position.min(frontier.len());
    frontier.splice(position..position, step.emitted.iter().copied());
}

/// What the frontier looks like below a set of placed vertices. A state that
/// cannot be finished is remembered by this, and by nothing else.
type Key = (Vec<u64>, Vec<Lifeline>);

/// The topology facts every step reads, indexed once.
struct Sweep<'a> {
    flow: &'a Flow,
    topology: &'a Topology,
    vertices: &'a [Vertex],
    /// Connection indices arriving at each vertex.
    arrivals: Vec<Vec<usize>>,
    /// Connection indices leaving each vertex, grouped by exit, exits in
    /// authored order. A group whose destinations are the cases of one choice
    /// keeps their authored order; any other group may take any order.
    departures: Vec<Vec<Fanout>>,
    /// Vertices that must be placed before each vertex.
    predecessors: Vec<Vec<usize>>,
    /// The loop whose entry, and the loop whose tail, each vertex is.
    entry_of: Vec<Option<usize>>,
    tail_of: Vec<Option<usize>>,
    /// The side each loop's return takes first.
    preferred: Vec<Side>,
    /// Finished sequences to refuse as though their columns did not number.
    ///
    /// No topology reaches that refusal, and the two reductions turn on it, so
    /// this is how a test reaches the behaviour: the walk must go on to the
    /// other ready vertices and must not remember the states it passed.
    #[cfg(test)]
    unnumbered: std::cell::Cell<usize>,
}

/// The mutable part of one branch of the walk.
struct State {
    placed: Vec<bool>,
    frontier: Vec<Lifeline>,
}

/// Searches the whole space for a conforming arrangement.
///
/// # Errors
///
/// Reports the vertex that could not be drawn in the deepest state the walk
/// reached, once every state has been visited.
pub(super) fn search(
    flow: &Flow,
    merges: &[WireMerge],
    topology: &Topology,
) -> Result<Arrangement, Obstruction> {
    let sweep = Sweep::of(flow, topology);
    let mut state = State {
        placed: vec![false; topology.vertices.len()],
        frontier: Vec::new(),
    };
    let mut failed = BTreeSet::new();
    let mut deepest = Deepest::default();
    let mut steps = Vec::new();
    sweep
        .walk(&mut state, &mut steps, &mut failed, &mut deepest)
        .map_err(|_| sweep.obstruction(flow, merges, &deepest))
}

/// Why a state finishes no drawing.
///
/// The two are kept apart because only the first is a fact about the state.
/// See the reductions in the module documentation.
enum Refused {
    /// Every continuation ran out of placeable vertices.
    Placement,
    /// Some continuation placed everything and could not number its columns.
    Numbering,
}

/// Builds the arrangement one sequence of steps describes, whoever chose them.
///
/// The choices live in the steps; everything below this is arithmetic both a
/// production walk and a test procedure have to do the same way, and the
/// independent check is what holds it to that.
#[cfg(test)]
pub(super) fn expand(flow: &Flow, topology: &Topology, steps: &[Step]) -> Option<Arrangement> {
    Sweep::of(flow, topology).expand(steps)
}

/// What a procedure making its own choices still has to read off the topology.
#[cfg(test)]
pub(super) struct Index {
    /// The loop each vertex is the tail of.
    pub(super) tail_of: Vec<Option<usize>>,
    pub(super) arrivals: Vec<Vec<usize>>,
    /// The connections leaving each vertex, grouped by exit in authored
    /// order, with the groups whose order RFC 0002 §8 fixes marked.
    pub(super) departures: Vec<Vec<(bool, Vec<usize>)>>,
    pub(super) predecessors: Vec<Vec<usize>>,
}

/// Indexes one topology the way the walk does, without any of its choices.
#[cfg(test)]
pub(super) fn index(flow: &Flow, topology: &Topology) -> Index {
    let sweep = Sweep::of(flow, topology);
    Index {
        tail_of: sweep.tail_of.clone(),
        arrivals: sweep.arrivals.clone(),
        departures: sweep
            .departures
            .iter()
            .map(|exits| {
                exits
                    .iter()
                    .map(|exit| (exit.fixed, exit.connections.clone()))
                    .collect()
            })
            .collect(),
        predecessors: sweep.predecessors.clone(),
    }
}

/// The furthest the walk got, and a vertex it could not place there. Reported
/// when the whole space turns out to hold no arrangement.
#[derive(Default)]
struct Deepest {
    placed: usize,
    blocked: Option<usize>,
}

impl<'a> Sweep<'a> {
    fn of(flow: &'a Flow, topology: &'a Topology) -> Self {
        let count = topology.vertices.len();
        let index = |vertex: Vertex| {
            topology
                .vertices
                .binary_search(&vertex)
                .expect("every connected vertex is projected")
        };
        let mut arrivals = vec![Vec::new(); count];
        let mut predecessors = vec![BTreeSet::new(); count];
        for (position, connection) in topology.connections.iter().enumerate() {
            let destination = index(connection.destination);
            arrivals[destination].push(position);
            predecessors[destination].insert(index(Vertex::from(connection.source)));
        }
        for edge in &topology.order {
            predecessors[index(edge.destination)].insert(index(Vertex::from(edge.source)));
        }
        let mut departures = (0..count).map(|_| Vec::new()).collect::<Vec<_>>();
        for vertex in topology.vertices.iter().copied() {
            let mut exits = BTreeMap::<Option<ExitId>, Vec<usize>>::new();
            for (position, connection) in topology.connections.iter().enumerate() {
                if Vertex::from(connection.source) != vertex {
                    continue;
                }
                let exit = match connection.source {
                    Source::Exit(exit) => Some(exit),
                    Source::Junction(_) => None,
                };
                exits.entry(exit).or_default().push(position);
            }
            departures[index(vertex)] = exits
                .into_iter()
                .map(|(exit, connections)| Fanout {
                    // RFC 0002 §8 fixes the order of the branches a choice's
                    // distributor carries. Every other fan-out leaves one exit
                    // for unrelated destinations, which may go either way
                    // round. The check reads the same rule.
                    fixed: exit
                        .is_some_and(|exit| super::regions::carries_branches(topology, exit)),
                    connections,
                })
                .collect();
        }
        let mut entry_of = vec![None; count];
        let mut tail_of = vec![None; count];
        for (loop_index, loop_) in topology.loops.iter().enumerate() {
            entry_of[index(Vertex::Junction(loop_.entry))] = Some(loop_index);
            tail_of[index(Vertex::Junction(loop_.tail))] = Some(loop_index);
            // The reversed return descends from the entry to the tail. Its
            // ends already order those two, so it adds no precedence.
        }
        Self {
            flow,
            topology,
            vertices: &topology.vertices,
            arrivals,
            departures,
            predecessors: predecessors
                .into_iter()
                .map(|set| set.into_iter().collect())
                .collect(),
            entry_of,
            tail_of,
            preferred: topology
                .loops
                .iter()
                .map(|loop_| {
                    if loop_.prefer_left {
                        Side::Left
                    } else {
                        Side::Right
                    }
                })
                .collect(),
            #[cfg(test)]
            unnumbered: std::cell::Cell::new(0),
        }
    }

    /// The arrangement this state finishes with, or why it has none.
    fn walk(
        &self,
        state: &mut State,
        steps: &mut Vec<Step>,
        failed: &mut BTreeSet<Key>,
        deepest: &mut Deepest,
    ) -> Result<Arrangement, Refused> {
        if state.placed.iter().all(|placed| *placed) {
            #[cfg(test)]
            if self.unnumbered.get() > 0 {
                self.unnumbered.set(self.unnumbered.get() - 1);
                return Err(Refused::Numbering);
            }
            // The columns are part of the decision, not a formality after it.
            // A sequence whose reserved areas and return sides cannot all hold
            // at once has no column assignment, so it is not a drawing, and the
            // walk goes on looking for one that is.
            return self.expand(steps).ok_or(Refused::Numbering);
        }
        let key = key(state);
        if failed.contains(&key) {
            return Err(Refused::Placement);
        }
        let Some(mut vertex) = (0..self.vertices.len()).find(|&vertex| self.ready(state, vertex))
        else {
            deepest.record(state, self.blocked(state));
            failed.insert(key);
            return Err(Refused::Placement);
        };
        let mut refused = Refused::Placement;
        loop {
            let (position, count) = self
                .consumed(state, vertex)
                .expect("a ready vertex meets the frontier");
            let consumed = state.frontier[position..position + count].to_vec();
            let continues = self.continues(vertex, &consumed);
            for emitted in self.emissions(vertex) {
                let removed = state
                    .frontier
                    .splice(position..position + count, emitted.iter().copied())
                    .collect::<Vec<_>>();
                state.placed[vertex] = true;
                steps.push(Step {
                    vertex,
                    consumed: consumed.clone(),
                    position,
                    emitted: emitted.clone(),
                    continues,
                });
                let found = self.walk(state, steps, failed, deepest);
                steps.pop();
                state.placed[vertex] = false;
                state
                    .frontier
                    .splice(position..position + emitted.len(), removed);
                match found {
                    Ok(arrangement) => return Ok(arrangement),
                    Err(Refused::Numbering) => refused = Refused::Numbering,
                    Err(Refused::Placement) => {}
                }
            }
            // One ready vertex is enough while the refusals below it are the
            // state's own: placing another first reaches the same states. A
            // numbering refusal is not the state's, so the rest are tried, and
            // only then are they even looked for.
            if matches!(refused, Refused::Placement) {
                break;
            }
            let Some(next) =
                (vertex + 1..self.vertices.len()).find(|&vertex| self.ready(state, vertex))
            else {
                break;
            };
            vertex = next;
        }
        if matches!(refused, Refused::Placement) {
            failed.insert(key);
        }
        Err(refused)
    }

    /// The lifeline whose column one vertex continues.
    ///
    /// This is forced, not chosen. A convergence goes on down the column its
    /// group's first branch arrived in (RFC 0002 §8), which order preservation
    /// makes the leftmost route reaching it. An iteration tail instead takes
    /// the end of its rail nearest the side its return climbs, because the
    /// return leaves it horizontally along that rail's own line and a tail at
    /// the far end would send the return back across everything arriving
    /// there. No vertex is both: a tail is a structural junction and merges no
    /// wire. Only the start node continues nothing.
    fn continues(&self, vertex: usize, consumed: &[Lifeline]) -> Option<Lifeline> {
        // The return climbs on the flank it stands on, which is where the
        // frontier put it: at one end of what the tail ends.
        let returns_right = self.tail_of[vertex]
            .is_some_and(|loop_| consumed.last() == Some(&Lifeline::Return(loop_)));
        let mut arriving = consumed
            .iter()
            .copied()
            .filter(|lifeline| matches!(lifeline, Lifeline::Wire(_)));
        if returns_right {
            arriving.next_back()
        } else {
            arriving.next()
        }
    }

    /// Whether one vertex can be placed now: everything above it is placed and
    /// the lifelines meeting there are side by side.
    fn ready(&self, state: &State, vertex: usize) -> bool {
        !state.placed[vertex]
            && self.predecessors[vertex]
                .iter()
                .all(|&earlier| state.placed[earlier])
            && self.consumed(state, vertex).is_some()
    }

    /// Where one vertex meets the frontier: the position of the lifelines it
    /// consumes, and how many, when they are contiguous.
    fn consumed(&self, state: &State, vertex: usize) -> Option<(usize, usize)> {
        let mut wanted = self.arrivals[vertex]
            .iter()
            .map(|&connection| Lifeline::Wire(connection))
            .collect::<BTreeSet<_>>();
        if let Some(loop_index) = self.tail_of[vertex] {
            wanted.insert(Lifeline::Return(loop_index));
        }
        if wanted.is_empty() {
            // Only the start node arrives from nowhere, and it goes first.
            return state.frontier.is_empty().then_some((0, 0));
        }
        let first = state
            .frontier
            .iter()
            .position(|lifeline| wanted.contains(lifeline))?;
        let run = &state.frontier[first..(first + wanted.len()).min(state.frontier.len())];
        (run.len() == wanted.len() && run.iter().all(|lifeline| wanted.contains(lifeline)))
            .then_some((first, wanted.len()))
    }

    /// Every order the lifelines one vertex emits may take, preferred first.
    /// Connections leaving one exit share that exit's run, so their order among
    /// themselves is free; exits keep their authored order. A choice's
    /// distributor is the exception `Fanout::fixed` names: the routes it fans
    /// out carry the branches themselves, and RFC 0002 §8 orders those.
    fn emissions(&self, vertex: usize) -> Vec<Vec<Lifeline>> {
        let mut orders = vec![Vec::new()];
        for exit in &self.departures[vertex] {
            // ponytail: every order of one free fan-out, so `k!` of them; no
            // fixture leaves one exit for more than two unrelated
            // destinations. Order them by a rule if a flow ever fans one exit
            // out widely.
            let groups = if exit.fixed {
                vec![exit.connections.clone()]
            } else {
                permutations(&exit.connections)
            };
            orders = orders
                .into_iter()
                .flat_map(|start| {
                    groups.clone().into_iter().map(move |group| {
                        let mut next = start.clone();
                        next.extend(group.into_iter().map(Lifeline::Wire));
                        next
                    })
                })
                .collect();
        }
        let Some(loop_index) = self.entry_of[vertex] else {
            return orders;
        };
        // A loop entry also opens its return, immediately outside the body it
        // is about to draw. The preferred side is tried first.
        let sides = match self.preferred[loop_index] {
            Side::Left => [Side::Left, Side::Right],
            Side::Right => [Side::Right, Side::Left],
        };
        sides
            .into_iter()
            .flat_map(|side| {
                orders.iter().map(move |order| {
                    let mut next = order.clone();
                    match side {
                        Side::Left => next.insert(0, Lifeline::Return(loop_index)),
                        Side::Right => next.push(Lifeline::Return(loop_index)),
                    }
                    next
                })
            })
            .collect()
    }

    /// A vertex every predecessor allows but the frontier does not, in the
    /// state the walk got furthest in.
    fn blocked(&self, state: &State) -> Option<usize> {
        (0..self.vertices.len()).find(|&vertex| {
            !state.placed[vertex]
                && self.predecessors[vertex]
                    .iter()
                    .all(|&earlier| state.placed[earlier])
        })
    }

    fn obstruction(&self, flow: &Flow, merges: &[WireMerge], deepest: &Deepest) -> Obstruction {
        let Some(vertex) = deepest.blocked else {
            return super::unarrangeable(flow);
        };
        let vertex = self.vertices[vertex];
        Obstruction {
            span: super::describe::vertex_span(flow, self.topology, vertex),
            message: format!(
                "nothing draws {}: however the routes above it are arranged, another route lies between two that meet there. Reorder the branches so the routes that meet sit side by side",
                super::describe::vertex(flow, merges, self.topology, vertex)
            ),
            loop_index: None,
            connection: None,
        }
    }

    /// Turns one finished sequence of steps into the arrangement it describes.
    ///
    /// Ranks come from the order the steps placed their vertices, and columns
    /// from every left-to-right pair the frontier showed, in an order that
    /// respects all of them. A connection then descends in its own column, with
    /// a sideways run below its source when it has to leave that source's
    /// column, and one above its destination when it has to reach it.
    ///
    /// ponytail: a rank and a column each, which is what the argument for this
    /// sequence assumes and so always draws, but it spreads a wide selection
    /// down a diagonal. Packing the rows needs a lane order per rank gap,
    /// because several vertices would then share one gap; add that if these
    /// diagrams ever have to be compact.
    fn expand(&self, steps: &[Step]) -> Option<Arrangement> {
        let columns = Columns::of(self, steps)?;
        let rank = steps
            .iter()
            .enumerate()
            .map(|(rank, step)| (self.vertices[step.vertex], rank))
            .collect::<BTreeMap<_, _>>();
        let column = steps
            .iter()
            .map(|step| (self.vertices[step.vertex], columns.vertex[step.vertex]))
            .collect::<BTreeMap<_, _>>();
        let exit_offset = self.exit_offsets(&columns, &column);
        let (routes, gap_lanes) =
            self.corridors(steps.len(), &rank, &column, &exit_offset, &columns);
        Some(Arrangement {
            contours: columns.contours.clone(),
            rank,
            ranks: steps.len(),
            column,
            exit_offset,
            routes,
            gap_lanes,
        })
    }

    /// The column each exit leaves its node by: the leftmost of the columns its
    /// own connections descend in. An exit no connection leaves still needs a
    /// column of its own, so it takes the one after the exit before it.
    fn exit_offsets(
        &self,
        columns: &Columns,
        column: &BTreeMap<Vertex, i32>,
    ) -> BTreeMap<ExitId, i32> {
        let mut offsets = BTreeMap::new();
        let mut previous: Option<(NodeId, i32)> = None;
        for exit in &self.topology.exits {
            let own = column[&Vertex::Node(exit.id.node)];
            let taken = self
                .topology
                .connections
                .iter()
                .enumerate()
                .filter(|(_, wire)| wire.source == Source::Exit(exit.id))
                .map(|(index, _)| columns.wire[index])
                .min();
            let offset = match (taken, previous) {
                (Some(column), _) => column - own,
                (None, Some((node, before))) if node == exit.id.node => before + 1,
                (None, _) => 0,
            };
            previous = Some((exit.id.node, offset));
            offsets.insert(exit.id, offset);
        }
        offsets
    }

    /// Every connection's corridor, and how many lanes each rank gap needs.
    ///
    /// A connection that reaches the next rank down turns once, in the one gap
    /// it crosses. A longer one leaves its source's column in the gap below it
    /// and reaches its destination's column in the gap above that — two runs
    /// that never share a gap. So one gap holds at most the runs leaving the
    /// vertex above it and the runs arriving at the vertex below it: two lanes,
    /// the leaving ones first.
    fn corridors(
        &self,
        ranks: usize,
        rank: &BTreeMap<Vertex, usize>,
        column: &BTreeMap<Vertex, i32>,
        exit_offset: &BTreeMap<ExitId, i32>,
        columns: &Columns,
    ) -> (Vec<Route>, Vec<usize>) {
        let mut leaving = vec![false; ranks];
        let mut arriving = vec![false; ranks];
        let mut routes = Vec::with_capacity(self.topology.connections.len());
        for (index, wire) in self.topology.connections.iter().enumerate() {
            let source = Vertex::from(wire.source);
            let departure = match wire.source {
                Source::Exit(exit) => column[&source] + exit_offset[&exit],
                Source::Junction(_) => column[&source],
            };
            let arrival = column[&wire.destination];
            let above = rank[&source];
            let below = rank[&wire.destination];
            let mut runs = Vec::new();
            let own = columns.wire[index];
            if below > above + 1 && departure != own {
                leaving[above] = true;
                runs.push(Run {
                    gap: above,
                    enter: departure,
                    exit: own,
                    lane: 0,
                });
            }
            let enter = if runs.is_empty() { departure } else { own };
            if enter != arrival {
                arriving[below - 1] = true;
                runs.push(Run {
                    gap: below - 1,
                    enter,
                    exit: arrival,
                    lane: 0,
                });
            }
            routes.push(Route {
                departure,
                arrival,
                runs,
            });
        }
        // A run that arrives takes the lane against the rank below, so a run
        // that leaves the rank above never turns down through it.
        let gap_lanes = (0..ranks)
            .map(|gap| usize::from(leaving[gap]) + usize::from(arriving[gap]))
            .collect::<Vec<_>>();
        for (index, route) in routes.iter_mut().enumerate() {
            let below = rank[&self.topology.connections[index].destination];
            for run in &mut route.runs {
                if run.gap == below - 1 && gap_lanes[run.gap] == 2 {
                    run.lane = 1;
                }
            }
        }
        (routes, gap_lanes)
    }
}

impl Deepest {
    fn record(&mut self, state: &State, blocked: Option<usize>) {
        let placed = state.placed.iter().filter(|placed| **placed).count();
        if blocked.is_some() && placed >= self.placed {
            self.placed = placed;
            self.blocked = blocked;
        }
    }
}

/// The placed set packed into words, with the frontier beside it.
fn key(state: &State) -> Key {
    let mut bits = vec![0u64; state.placed.len().div_ceil(64)];
    for (vertex, placed) in state.placed.iter().enumerate() {
        if *placed {
            bits[vertex / 64] |= 1 << (vertex % 64);
        }
    }
    (bits, state.frontier.clone())
}

/// Every order of a handful of connections, the authored one first.
fn permutations(items: &[usize]) -> Vec<Vec<usize>> {
    if items.len() <= 1 {
        return vec![items.to_vec()];
    }
    (0..items.len())
        .flat_map(|taken| {
            let mut rest = items.to_vec();
            let first = rest.remove(taken);
            permutations(&rest).into_iter().map(move |mut order| {
                order.insert(0, first);
                order
            })
        })
        .collect()
}

/// Where every lifeline and every vertex ends up horizontally.
///
/// A lifeline never crosses another, so every pair the frontier ever showed
/// side by side keeps that order for as long as both last. Those pairs have no
/// cycle: a connection is active over one unbroken run of ranks, so three
/// lifelines that pairwise overlap share a rank and are ordered there. Any
/// order that respects all of them therefore numbers the columns.
struct Columns {
    /// The column of each vertex, by vertex index.
    vertex: Vec<i32>,
    /// The column each connection descends in, by connection index.
    wire: Vec<i32>,
    /// The contour of each loop return, by loop index.
    contours: Vec<Contour>,
}

impl Columns {
    fn of(sweep: &Sweep<'_>, steps: &[Step]) -> Option<Self> {
        // A group is one column: the chain of a vertex's incoming lifeline, the
        // vertex itself, and the outgoing lifeline that continues it.
        let mut group_of = BTreeMap::<Lifeline, usize>::new();
        let mut vertex_group = vec![0usize; sweep.vertices.len()];
        let mut sides = vec![Side::Left; sweep.topology.loops.len()];
        let mut groups = 0usize;
        let mut before_pairs = BTreeSet::<(usize, usize)>::new();
        let mut frontier = Vec::<Lifeline>::new();
        for step in steps {
            let mut fresh = || {
                groups += 1;
                groups - 1
            };
            let mut own = step.continues.map(|lifeline| group_of[&lifeline]);
            let own = *own.get_or_insert_with(&mut fresh);
            vertex_group[step.vertex] = own;
            let mut continued = false;
            for &lifeline in &step.emitted {
                let group = match lifeline {
                    Lifeline::Wire(_) if !continued => {
                        continued = true;
                        own
                    }
                    Lifeline::Return(loop_index) => {
                        sides[loop_index] = if step.emitted[0] == lifeline {
                            Side::Left
                        } else {
                            Side::Right
                        };
                        fresh()
                    }
                    Lifeline::Wire(_) => fresh(),
                };
                group_of.insert(lifeline, group);
            }
            replay(&mut frontier, step);
            for pair in frontier.windows(2) {
                before_pairs.insert((group_of[&pair[0]], group_of[&pair[1]]));
            }
        }

        // A convergence group reserves the columns of everything its branches
        // draw, and a sibling outside it keeps clear of that whole area
        // (RFC 0002 §8). The frontier only says so while both are alive: the
        // group's shared continuation is drawn below, by which time the
        // sibling's own lifelines may be gone, so those orders are recorded
        // here instead.
        let reachable = super::regions::reachable(sweep.topology);
        for block in super::regions::branchers(sweep.flow) {
            let regions = super::regions::regions(sweep.flow, sweep.topology, &reachable, block);
            for group in &regions.groups {
                let first = *group.members.first().expect("a group has members");
                let last = *group.members.last().expect("a group has members");
                for branch in 0..regions.branches.len() {
                    if group.members.contains(&branch) {
                        continue;
                    }
                    // A sibling the group encloses may sit inside the reserved
                    // range, so only the two outer sides are an order.
                    let before = branch < first;
                    if !before && branch <= last {
                        continue;
                    }
                    for outside in &regions.outside(group, branch) {
                        for inside in &group.area {
                            let (left, right) = if before {
                                (outside, inside)
                            } else {
                                (inside, outside)
                            };
                            let left = vertex_group[index_of(sweep, *left)];
                            let right = vertex_group[index_of(sweep, *right)];
                            if left != right {
                                before_pairs.insert((left, right));
                            }
                        }
                    }
                }
            }
        }

        let bodies = returns(sweep, &group_of, &vertex_group, &sides, &mut before_pairs);

        let order = extend(groups, &before_pairs)?;
        Some(Self {
            vertex: vertex_group.iter().map(|&group| order[group]).collect(),
            wire: (0..sweep.topology.connections.len())
                .map(|wire| order[group_of[&Lifeline::Wire(wire)]])
                .collect(),
            contours: contours(&sides, &bodies, &order, &|loop_index| {
                order[group_of[&Lifeline::Return(loop_index)]]
            }),
        })
    }
}

/// Records where each return stands against the body it leaves, and answers
/// with the groups that body occupies.
fn returns(
    sweep: &Sweep<'_>,
    group_of: &BTreeMap<Lifeline, usize>,
    vertex_group: &[usize],
    sides: &[Side],
    before_pairs: &mut BTreeSet<(usize, usize)>,
) -> Vec<Vec<usize>> {
    // A return climbs outside everything its body draws, and the frontier
    // only says so while the return is alive. Two things outlive it. A
    // body vertex placed after the tail — a route that leaves the loop, or
    // finishes the flow from inside it — is free of the return by then.
    // And a return nested in that body is never on the frontier beside the
    // enclosing one at all when their rows do not overlap, which is also
    // why no crossing check sees it. Both sides are recorded here instead.
    // Nothing pushes either the other way, so this adds an order, never a
    // contradiction; where the two sides cannot both hold, the numbering
    // fails and the walk backtracks onto the other flank.
    let mut bodies = Vec::with_capacity(sweep.topology.loops.len());
    for (index, loop_) in sweep.topology.loops.iter().enumerate() {
        let climb = group_of[&Lifeline::Return(index)];
        let end = sweep.flow.blocks[loop_.header]
            .loop_end
            .expect("a loop owns a body");
        let body = loop_.header + 1..end;
        let inner = sweep
            .topology
            .loops
            .iter()
            .enumerate()
            .filter(|(_, other)| body.contains(&other.header))
            .map(|(other, _)| group_of[&Lifeline::Return(other)]);
        let vertices = super::loop_block::body_vertices(sweep.flow, sweep.topology, loop_.header)
            .into_iter()
            .map(|vertex| vertex_group[index_of(sweep, vertex)])
            .collect::<Vec<_>>();
        for inside in inner.chain(vertices.iter().copied()) {
            match sides[index] {
                Side::Left => before_pairs.insert((climb, inside)),
                Side::Right => before_pairs.insert((inside, climb)),
            };
        }
        bodies.push(vertices);
    }
    bodies
}

/// Turns the numbered column of each return into the contour a presentation
/// can realize: a lane counted outward from the column its body's outermost
/// vertex occupies.
///
/// The numbering gives every return a column of its own, which says where it
/// stands among the body's columns but not how a drawing reaches it — a
/// presentation has no rail to put in a column, only the space beside the body
/// it measures. The two are the same order, so the column becomes the body's
/// edge and the lane counts the returns the numbering put between them. A
/// return nested in this body is one of those, which is what keeps the
/// enclosing one a lane further out.
fn contours(
    sides: &[Side],
    bodies: &[Vec<usize>],
    order: &[i32],
    climb: &dyn Fn(usize) -> i32,
) -> Vec<Contour> {
    (0..sides.len())
        .map(|index| {
            let side = sides[index];
            let at = climb(index);
            let columns = bodies[index].iter().map(|&group| order[group]);
            let column = match side {
                Side::Left => columns.min(),
                Side::Right => columns.max(),
            }
            .expect("a loop owns an entry and a tail");
            let lane = (0..sides.len())
                .filter(|&other| other != index && sides[other] == side)
                .filter(|&other| match side {
                    Side::Left => column > climb(other) && climb(other) > at,
                    Side::Right => column < climb(other) && climb(other) < at,
                })
                .count();
            Contour { side, column, lane }
        })
        .collect()
}

/// Where one vertex sits in the topology's sorted list.
fn index_of(sweep: &Sweep<'_>, vertex: Vertex) -> usize {
    sweep
        .vertices
        .binary_search(&vertex)
        .expect("a drawn vertex is projected")
}

/// Numbers the groups so that every recorded pair keeps its order. Ties go to
/// the group created first, so one topology always yields one arrangement.
///
/// `None` when the pairs contradict each other. The frontier's own pairs never
/// do — two lifelines keep their order for as long as both last — but the
/// reserved areas and return sides added beside them can, and so can a
/// procedure that reorders the frontier freely.
fn extend(groups: usize, before: &BTreeSet<(usize, usize)>) -> Option<Vec<i32>> {
    let mut waiting = vec![0usize; groups];
    for &(_, right) in before {
        waiting[right] += 1;
    }
    let mut ready = (0..groups)
        .filter(|&group| waiting[group] == 0)
        .collect::<BTreeSet<_>>();
    let mut order = vec![0; groups];
    for column in 0..groups {
        let group = ready.pop_first()?;
        order[group] = i32::try_from(column).unwrap_or(i32::MAX);
        for &(_, right) in before.iter().filter(|&&(left, _)| left == group) {
            waiting[right] -= 1;
            if waiting[right] == 0 {
                ready.insert(right);
            }
        }
    }
    Some(order)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{State, Step, Sweep};

    /// Walks the whole state space: every ready vertex, every order its
    /// emissions may take, and no memory of the states already refused.
    ///
    /// The production walk takes the first ready vertex and remembers
    /// refusals. Those two reductions rest on ready vertices commuting, which
    /// this procedure does not assume, so a disagreement between the two is a
    /// counterexample to them. Only small topologies are within reach: without
    /// the reductions the walk is the full interleaving of every subtree.
    ///
    /// It succeeds on the same condition the production walk does: a sequence
    /// that places every vertex *and* numbers its columns. Stopping at the
    /// last placement would compare two different questions, and the column
    /// numbering is the half the second reduction does not obviously commute
    /// with — it reads the whole sequence, not the state the memo keys on.
    ///
    /// What it does share is the step itself — `ready`, `consumed`,
    /// `continues` and `emissions` — and the expansion that numbers and checks
    /// the result. A procedure sharing neither would have to enumerate raw
    /// ranks, columns and corridors, which is out of reach at any size worth
    /// testing; the step is also the part the correspondence argument
    /// establishes directly, while the reductions are the part that needs a
    /// counterexample hunt.
    fn whole_space(sweep: &Sweep<'_>, state: &mut State, steps: &mut Vec<Step>) -> bool {
        if state.placed.iter().all(|placed| *placed) {
            return sweep.expand(steps).is_some();
        }
        for vertex in 0..sweep.vertices.len() {
            if !sweep.ready(state, vertex) {
                continue;
            }
            let (position, count) = sweep
                .consumed(state, vertex)
                .expect("a ready vertex meets the frontier");
            let consumed = state.frontier[position..position + count].to_vec();
            let continues = sweep.continues(vertex, &consumed);
            for emitted in sweep.emissions(vertex) {
                let removed = state
                    .frontier
                    .splice(position..position + count, emitted.iter().copied())
                    .collect::<Vec<_>>();
                state.placed[vertex] = true;
                steps.push(Step {
                    vertex,
                    consumed: consumed.clone(),
                    position,
                    emitted: emitted.clone(),
                    continues,
                });
                let found = whole_space(sweep, state, steps);
                steps.pop();
                state.placed[vertex] = false;
                state
                    .frontier
                    .splice(position..position + emitted.len(), removed);
                if found {
                    return true;
                }
            }
        }
        false
    }

    /// Contradicting column orders have no numbering.
    ///
    /// `walk` treats that refusal apart from a placement refusal, and neither
    /// reduction applies to it, so the branch has to exist whether or not a
    /// topology reaches it. None does today: the corpus and every generated
    /// loop shape number every sequence they finish.
    #[test]
    fn contradicting_column_orders_do_not_number() {
        assert!(super::extend(2, &BTreeSet::from([(0, 1), (1, 0)])).is_none());
        assert_eq!(
            super::extend(2, &BTreeSet::from([(1, 0)])),
            Some(vec![1, 0])
        );
    }

    /// A refusal the numbering raised sends the walk on, and is forgotten.
    ///
    /// Neither reduction may be applied to it: the numbering reads the whole
    /// sequence, not the state the memo keys on, so a state it refused may
    /// still finish by another route and a ready vertex it refused under may
    /// still be the wrong one to have picked. No topology reaches that
    /// refusal, so the walk is made to raise it here instead.
    ///
    /// Both halves are pinned. Refusing the first finished sequence must still
    /// leave the walk an arrangement — it has to try the rest — and that
    /// arrangement must differ from the one it settles on when nothing is
    /// refused, or the refusal never bit.
    #[test]
    fn a_numbering_refusal_sends_the_walk_on_and_is_forgotten() {
        let source = crate::construct::tests::looping(&["repeat", "repeat", "break"]);
        let parts = crate::construct::tests::parts_of(&source).expect("the probe projects");
        let Ok(settled) = super::search(&parts.flow, &parts.merges, &parts.topology) else {
            panic!("the probe has a diagram")
        };

        let sweep = Sweep::of(&parts.flow, &parts.topology);
        sweep.unnumbered.set(1);
        let mut state = State {
            placed: vec![false; parts.topology.vertices.len()],
            frontier: Vec::new(),
        };
        let Ok(found) = sweep.walk(
            &mut state,
            &mut Vec::new(),
            &mut std::collections::BTreeSet::new(),
            &mut super::Deepest::default(),
        ) else {
            panic!("the walk goes on past a numbering refusal")
        };
        assert_eq!(
            sweep.unnumbered.get(),
            0,
            "the refusal should have been used"
        );
        assert_ne!(
            found.rank, settled.rank,
            "the refused sequence should not be the one it answers with"
        );
    }

    /// Both directions, over flat and nested loop bodies alike, drawable and
    /// not: a sequence the reduced walk finds is one the whole space finds,
    /// and a topology the reduced walk refuses has none.
    ///
    /// This is the test of the two reductions and of nothing else. It takes
    /// the same steps the production walk takes, and drops only the choice of
    /// one ready vertex and the memory of refused states.
    ///
    /// It compares against the walk itself, not against `construct`: the
    /// preferred search runs first there and would answer for every shape it
    /// happens to draw, which is exactly the set where an unsound reduction
    /// would stay hidden.
    #[test]
    fn the_reduced_walk_agrees_with_the_whole_state_space() {
        let mut drawn = 0;
        let mut refused = 0;
        for source in crate::construct::tests::decision_cases() {
            // A flow another rule rejects has no topology to arrange, and
            // realizability never had a say in it.
            let Some(parts) = crate::construct::tests::parts_of(&source) else {
                continue;
            };
            let accepted = super::search(&parts.flow, &parts.merges, &parts.topology).is_ok();
            if accepted {
                drawn += 1;
            } else {
                refused += 1;
            }
            let sweep = Sweep::of(&parts.flow, &parts.topology);
            let mut state = State {
                placed: vec![false; parts.topology.vertices.len()],
                frontier: Vec::new(),
            };
            assert_eq!(
                whole_space(&sweep, &mut state, &mut Vec::new()),
                accepted,
                "{source}: the walk and the whole state space disagree"
            );
        }
        assert!(
            drawn >= 20 && refused >= 4,
            "the cases should cover both outcomes: {drawn} drawn, {refused} refused"
        );
    }
}
