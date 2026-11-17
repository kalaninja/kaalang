import assert from "node:assert";
import test from "node:test";

import { innermost, request } from "../hotspots.js";

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
const hotspots = [boundary, node, wire];

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

test("a call asks for its callee's definition", () => {
  assert.deepEqual(request(node), { type: "definition", offset: 30 });
});

test("a wire name asks for its value's type", () => {
  assert.deepEqual(request(wire), { type: "typeDefinition", offset: 50 });
});

test("any other node asks to reveal its code", () => {
  assert.deepEqual(request(boundary), { type: "reveal", start: 0, end: 90 });
});
