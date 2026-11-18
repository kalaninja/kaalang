import assert from "node:assert";
import test from "node:test";

import { carryingAt, innermost, request, wires } from "../hotspots.js";

/** A cycle boundary around a node and a wire name in its body. */
const boundary = {
  kind: "node",
  left: 0,
  top: 0,
  right: 400,
  bottom: 300,
  start: 0,
  end: 90,
};
const node = {
  kind: "call",
  left: 40,
  top: 40,
  right: 200,
  bottom: 100,
  start: 10,
  end: 40,
  calleeStart: 30,
  calleeEnd: 33,
};
const wire = {
  kind: "wire",
  left: 120,
  top: 120,
  right: 160,
  bottom: 134,
  start: 50,
  end: 55,
};
/**
 * Two connections sharing a rail inside the cycle, below the node, along
 * y = 200; a third along y = 220, near enough that their strips overlap; and
 * the merge where the first two meet the third.
 */
const left = {
  kind: "connection",
  left: 28,
  top: 188,
  right: 212,
  bottom: 212,
  ribbon: [0, 1],
};
const right = {
  kind: "connection",
  left: 108,
  top: 188,
  right: 312,
  bottom: 212,
  ribbon: [2, 3],
};
const below = {
  kind: "connection",
  left: 28,
  top: 208,
  right: 212,
  bottom: 232,
  ribbon: [3],
};
const merge = {
  kind: "merge",
  left: 188,
  top: 188,
  right: 212,
  bottom: 212,
  ribbon: [1],
};
const hotspots = [boundary, node, wire, left, right, below, merge];
/** The diagram's wires, which ribbons list by position. */
const listed = [
  { name: "total", start: 60, end: 65 },
  { name: "count", start: 20, end: 25 },
  { name: "total", start: 70, end: 75 },
  { name: "rest", start: 80, end: 84 },
];

test("a click on a node inside a cycle takes the node", () => {
  assert.equal(innermost(hotspots, 100, 70), node);
});

test("a click on a wire name inside a cycle takes the wire name", () => {
  assert.equal(innermost(hotspots, 140, 127), wire);
});

test("a click inside a cycle off its body takes the cycle", () => {
  assert.equal(innermost(hotspots, 300, 250), boundary);
});

test("a click outside every hotspot takes nothing", () => {
  assert.equal(innermost(hotspots, 500, 10), null);
});

test("a navigating click on a connection inside a cycle takes the cycle", () => {
  assert.equal(innermost(hotspots, 60, 200), boundary);
});

test("a click on a connection lists its ribbon, in text order", () => {
  assert.deepEqual(
    wires(carryingAt(hotspots, 60, 200), listed).map((wire) => wire.name),
    ["count", "total"],
  );
});

test("a click on a shared rail lists each wire there once, with its first binding", () => {
  assert.deepEqual(wires(carryingAt(hotspots, 150, 200), listed), [
    { name: "count", start: 20, end: 25 },
    { name: "total", start: 60, end: 65 },
    { name: "rest", start: 80, end: 84 },
  ]);
});

test("a click between two connections lists the nearer one's ribbon", () => {
  assert.deepEqual(carryingAt(hotspots, 60, 209), [left]);
  assert.deepEqual(carryingAt(hotspots, 60, 211), [below]);
});

test("a click by a corner lists the connection whose segment runs nearest", () => {
  // `down` turns into `across` at (100, 100); `under` runs 20 below `across`.
  const down = {
    kind: "connection",
    left: 88,
    top: 38,
    right: 112,
    bottom: 112,
    ribbon: [0],
  };
  const across = {
    kind: "connection",
    left: 88,
    top: 88,
    right: 212,
    bottom: 112,
    ribbon: [0],
  };
  const under = {
    kind: "connection",
    left: 38,
    top: 108,
    right: 212,
    bottom: 132,
    ribbon: [1],
  };
  assert.deepEqual(carryingAt([down, across, under], 100, 111), [under]);
  assert.deepEqual(carryingAt([down, across, under], 101, 109), [down, across]);
});

test("a click near a merge lists the merge's ribbon, not its connections'", () => {
  assert.deepEqual(carryingAt(hotspots, 195, 205), [merge]);
});

test("a navigating click on a merge takes the cycle around it", () => {
  assert.equal(innermost(hotspots, 200, 200), boundary);
});

test("a click off every connection lists nothing", () => {
  assert.deepEqual(carryingAt(hotspots, 100, 70), []);
});

test("a call asks for its callee's definition", () => {
  assert.deepEqual(request(node), { type: "definition", offset: 30 });
});

test("a wire name asks for its value's type", () => {
  assert.deepEqual(request(wire), { type: "typeDefinition", offset: 50 });
});

test("any other node asks to reveal its code", () => {
  assert.deepEqual(request(boundary), { type: "reveal", start: 0, end: 90 });
});
