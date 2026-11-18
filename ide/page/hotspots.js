// What a click on a diagram means: under the modifier, the place a hotspot
// leads to and what the page asks the host for it; without, the ribbon of the
// merge or of the connections under the pointer.

/** @typedef {import('./engine.js').Hotspot} Hotspot */
/** @typedef {import('./engine.js').Wire} Wire */

/**
 * The hotspot a navigating click at a point in the SVG's user units lands on:
 * the smallest holding it, since a cycle boundary's holds its body's. A
 * connection or a merge names no one place to go to, so neither is taken.
 *
 * @param {Hotspot[]} hotspots
 * @param {number} x
 * @param {number} y
 * @returns {Hotspot | null}
 */
export function innermost(hotspots, x, y) {
  let found = null;
  for (const hotspot of hotspots) {
    if (
      !carries(hotspot) &&
      holds(hotspot, x, y) &&
      (found === null || area(hotspot) < area(found))
    ) {
      found = hotspot;
    }
  }
  return found;
}

/**
 * The hotspots whose ribbons a plain click at a point in the SVG's user units
 * shows: the nearest merge, if one holds the point, else the connections whose
 * line runs nearest, since neighbouring strips overlap. Connections that share
 * a rail run equally near, so a click there shows them all.
 *
 * @param {Hotspot[]} hotspots
 * @param {number} x
 * @param {number} y
 * @returns {Hotspot[]}
 */
export function carryingAt(hotspots, x, y) {
  const held = hotspots.filter(
    (hotspot) => carries(hotspot) && holds(hotspot, x, y),
  );
  const merges = held.filter((hotspot) => hotspot.kind === "merge");
  const candidates = merges.length > 0 ? merges : held;
  /**
   * How far the point lies from a merge's centre or a strip's segment, along
   * the farther axis. Each reaches as far across as it is half wide.
   */
  const distance = ({ left, top, right, bottom }) => {
    const reach = Math.min(right - left, bottom - top) / 2;
    return Math.max(
      left + reach - x,
      x - right + reach,
      top + reach - y,
      y - bottom + reach,
    );
  };
  const nearest = Math.min(...candidates.map(distance));
  return candidates.filter((hotspot) => distance(hotspot) === nearest);
}

/**
 * The wires of the ribbons of `carrying`, looked up in the diagram's `listed`
 * wires, each name once, as a merge makes one wire of its producers: with its
 * first binding, in text order.
 *
 * @param {Hotspot[]} carrying
 * @param {Wire[]} listed
 */
export function wires(carrying, listed) {
  const first = new Map();
  for (const position of carrying.flatMap((hotspot) => hotspot.ribbon)) {
    const wire = listed[position];
    const known = first.get(wire.name);
    if (known === undefined || wire.start < known.start) {
      first.set(wire.name, wire);
    }
  }
  return [...first.values()].sort((a, b) => a.start - b.start);
}

/** Whether `hotspot` carries a ribbon: a connection's strip or a merge. */
function carries(hotspot) {
  return hotspot.kind === "connection" || hotspot.kind === "merge";
}

/** @param {Hotspot} hotspot */
function holds(hotspot, x, y) {
  return (
    hotspot.left <= x &&
    x <= hotspot.right &&
    hotspot.top <= y &&
    y <= hotspot.bottom
  );
}

/** @param {Hotspot} hotspot */
function area(hotspot) {
  return (hotspot.right - hotspot.left) * (hotspot.bottom - hotspot.top);
}

/**
 * What a navigating click on `hotspot` asks of the host: a call goes to its
 * callee's definition, a wire name to its value's type, a path in a card or a
 * relationship's target to its definition, any other node to its block's code,
 * and an incoming card's header to its type's name. Each names the text its
 * offsets count in, even if the editor has switched documents since the click.
 *
 * @param {Hotspot} hotspot
 * @param {string[]} uris
 * @param {string[]} [texts] Each card's source snapshot, for a jump outside the focus.
 */
export function request(hotspot, uris, texts) {
  const source = hotspot.source ?? 0;
  const file = { uri: uris[source] };
  if (source > 0 && texts) file.text = texts[source];
  switch (hotspot.kind) {
    case "call":
      return { type: "definition", ...file, offset: hotspot.calleeStart };
    case "wire":
      return { type: "typeDefinition", ...file, offset: hotspot.start };
    case "path":
    case "target":
      return { type: "definition", ...file, offset: hotspot.start };
    default:
      return {
        type: "reveal",
        ...file,
        start: hotspot.start,
        end: hotspot.end,
      };
  }
}
