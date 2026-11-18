// Offsets in a text as VS Code counts them, apart from VS Code so that tests
// reach them.

/**
 * The offset, in UTF-16 code units, of each position in `text`, given by its
 * line and character, a line ending at a line feed, a carriage return, or
 * both. A position past its line stands at the line's end, and one past the
 * last line at the text's, as a VS Code document places it. The lines are
 * found once, for any number of positions.
 */
export function offsets(
  text: string,
): (line: number, character: number) => number {
  return coordinates(text).offsetAt;
}

/** Bidirectional UTF-16 positions for offsets in the same snapshot. */
export function coordinates(text: string) {
  const starts = [0];
  const ends: number[] = [];
  for (const end of text.matchAll(/\r\n|\r|\n/g)) {
    ends.push(end.index);
    starts.push(end.index + end[0].length);
  }
  ends.push(text.length);
  return {
    offsetAt: (line: number, character: number) =>
      line < starts.length
        ? Math.min(starts[line] + character, ends[line])
        : text.length,
    positionAt(offset: number) {
      let line = 0;
      let after = starts.length;
      while (line + 1 < after) {
        const middle = Math.floor((line + after) / 2);
        if (starts[middle] <= offset) line = middle;
        else after = middle;
      }
      return {
        line,
        character: Math.max(0, Math.min(offset, ends[line]) - starts[line]),
      };
    },
  };
}
