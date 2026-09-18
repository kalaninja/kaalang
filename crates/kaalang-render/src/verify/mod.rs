//! Verification of renderer-proposed arrangements.

use std::collections::{BTreeMap, BTreeSet};

use kaalang_compiler::topology::{Connection, Topology, Vertex};
use kaalang_compiler::{Arrangement, ArrangementChecks, Flow};

mod loop_block;

/// Reusable verification context for renderer-proposed arrangements.
///
/// The flow and topology must come from the same compiler analysis and
/// projection. Construction caches facts shared by every candidate.
pub struct ArrangementVerifier<'a> {
    checks: ArrangementChecks<'a>,
    topology: &'a Topology,
    bodies: loop_block::Bodies,
    body_vertices: BTreeMap<usize, BTreeSet<Vertex>>,
}

impl<'a> ArrangementVerifier<'a> {
    /// Prepares to verify arrangements for one analyzed and projected flow.
    #[must_use]
    pub fn new(flow: &'a Flow, topology: &'a Topology) -> Self {
        let body_vertices = topology
            .loops
            .iter()
            .map(|loop_| loop_.header)
            .chain(
                topology
                    .loop_boundaries
                    .iter()
                    .map(|boundary| boundary.header),
            )
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(|header| (header, topology.body_vertices(flow, header)))
            .collect();
        Self {
            checks: ArrangementChecks::new(flow, topology),
            topology,
            bodies: loop_block::Bodies::of(flow, topology),
            body_vertices,
        }
    }

    /// Checks a complete arrangement proposed for rendering.
    ///
    /// Missing or inconsistent arrangement references produce an error rather
    /// than being indexed.
    ///
    /// # Errors
    ///
    /// Returns the first placement, boundary or common geometry rule the
    /// arrangement breaks.
    pub fn verify(&self, arrangement: &Arrangement) -> Result<(), String> {
        self.checks
            .verify_placement(arrangement, |edge| self.bodies.completes_a_boundary(edge))?;
        self.checks.verify_geometry(arrangement, |geometry| {
            self.bodies.verify(self.topology, geometry)
        })
    }

    /// Normalizes and verifies one renderer candidate.
    ///
    /// Taking ownership lets a caller retain its current arrangement until this
    /// returns `Ok`, so a rejected candidate cannot change the fallback.
    ///
    /// # Errors
    ///
    /// Returns the first placement, boundary or common geometry rule the
    /// candidate breaks.
    pub fn normalize(&self, arrangement: Arrangement) -> Result<Arrangement, String> {
        self.checks
            .verify_placement(&arrangement, |edge| self.bodies.completes_a_boundary(edge))?;
        self.checks.normalize_geometry(arrangement, |geometry| {
            self.bodies.verify(self.topology, geometry)
        })
    }

    /// Whether this relation may rise beside a cycle whose boundary it leaves.
    #[must_use]
    pub fn may_rise_beside(&self, arrangement: &Arrangement, edge: &Connection) -> bool {
        self.bodies.may_rise_beside(arrangement, edge)
    }

    /// Vertices drawn inside the cycle body beginning at `header`.
    ///
    /// Returns an empty set when `header` does not identify a cycle in this topology.
    #[must_use]
    pub fn body_vertices(&self, header: usize) -> BTreeSet<Vertex> {
        self.body_vertices.get(&header).cloned().unwrap_or_default()
    }
}
