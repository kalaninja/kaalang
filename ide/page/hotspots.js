// What a navigating click on a diagram means: the hotspot it lands on, and what
// the page asks the host for it.

/** @typedef {import('./engine.js').Hotspot} Hotspot */

/**
 * The hotspot a point in the SVG's user units lands on: the smallest holding
 * it, since a cycle boundary's holds its body's.
 *
 * @param {Hotspot[]} hotspots
 * @param {number} x
 * @param {number} y
 * @returns {Hotspot | null}
 */
export function innermost(hotspots, x, y) {
  let found = null;
  for (const hotspot of hotspots) {
    const holds =
      hotspot.left <= x &&
      x <= hotspot.right &&
      hotspot.top <= y &&
      y <= hotspot.bottom;
    if (holds && (found === null || area(hotspot) < area(found))) {
      found = hotspot;
    }
  }
  return found;
}

/** @param {Hotspot} hotspot */
function area(hotspot) {
  return (hotspot.right - hotspot.left) * (hotspot.bottom - hotspot.top);
}

/**
 * What a navigating click on `hotspot` asks of the host: a call goes to its
 * callee's definition, a wire name to its value's type, any other node to its
 * block's code.
 *
 * @param {Hotspot} hotspot
 */
export function request(hotspot) {
  switch (hotspot.kind) {
    case "call":
      return { type: "definition", offset: hotspot.calleeStart };
    case "wire":
      return { type: "typeDefinition", offset: hotspot.start };
    default:
      return { type: "reveal", start: hotspot.start, end: hotspot.end };
  }
}
