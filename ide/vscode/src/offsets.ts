// Offsets in a text as VS Code counts them, apart from VS Code so that tests
// reach them.

/**
 * A saved file's text as VS Code shows it, in which the page counts offsets:
 * without a byte order mark, and with mixed line endings turned into the kind
 * most of them are, a line feed when neither is.
 */
export function editorText(saved: string): string {
  const text = saved.replace(/^\uFEFF/, "");
  // Most files have one kind of line ending, which stays.
  if (!text.includes("\r") || !/(?<!\r)\n|\r(?!\n)/.test(text)) return text;
  const ends = text.match(/\r\n|\r|\n/g) ?? [];
  const returns = ends.filter((end) => end !== "\n").length;
  return text.replace(/\r\n|\r|\n/g, returns > ends.length / 2 ? "\r\n" : "\n");
}

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
