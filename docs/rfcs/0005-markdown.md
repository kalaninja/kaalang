# RFC 0005: Markdown

- Status: accepted
- Language: [RFC 0001: kaalang Language](0001-language.md)
- Visual language: [RFC 0002: kaalang Visual Language](0002-visual-language.md)
- Renderer: [RFC 0003: kaalang SVG Renderer](0003-svg-renderer.md)

## 1. Motivation and scope

Descriptions sometimes need to emphasize a condition, distinguish a code name
from prose, strike out an obsolete term, show indices, quote a statement, write
a formula, or call out a phrase. Today every character is displayed literally.
Add bold, italic, strikethrough, code, superscript, subscript, quoted text,
rendered TeX formulas, underline, highlight, and palette text colors.

This restricted notation does not turn descriptions into Markdown documents. It
adds no headings, tables, lists, fenced code blocks, links, images, general
HTML, arbitrary font size, or arbitrary CSS. Formatting changes presentation
only; it cannot affect execution, wire identity, branching, or diagram topology.

For example:

```rust
#[kaalang]
fn clamp_negative(value: i32) -> i32 {
    #[action("Keep **nonnegative** values; replace *negative* `value` with ~~the input~~ zero.")]
    let result = |value| value.max(0);

    |result| return result;
}
```

The action displays those four phrases with their respective effects, without
the surrounding delimiters. The generated Rust is the same as with a plain
description.

## 2. Where formatting applies

Interpret the supported Markdown notation in descriptions wherever they are
displayed. Source-derived and generated labels remain literal.

Descriptions remain ordinary Rust string literals. First decode the Rust
literal; then interpret its value as inline text. Ordinary and raw literals with
the same value produce the same display. No attribute, renderer option, or CLI
flag enables the feature: it applies to descriptions by default.

## 3. Notation

| Source text                      | Effect                                |
| -------------------------------- | ------------------------------------- |
| `**text**` or `__text__`         | Bold                                  |
| `*text*` or `_text_`             | Italic                                |
| `~~text~~`                       | Strikethrough                         |
| `` `text` ``                     | Inline code in a monospace font       |
| `^text^`                         | Superscript                           |
| `~text~`                         | Subscript                             |
| `> text`                         | Quoted line                           |
| `$formula$`                      | Inline TeX formula                    |
| `$$formula$$`                    | TeX formula using display math layout |
| `<u>text</u>`                    | Underline                             |
| `<mark>text</mark>`              | Highlight with a pale background      |
| `<color name="red">text</color>` | Palette text color                    |

The color names are `red`, `green`, `blue`, `purple`, and `muted`. Tag names,
attribute spelling, quotes, and color names are exact and lowercase. No other
attributes or spacing are accepted. Tags may nest with one another and with
supported Markdown; a matching closing tag is required on the same authored
line. For example, `<mark>**important**</mark>` highlights bold text. An
unrecognized or unpaired tag stays literal. A backslash can keep its opening `<`
literal.

For example, `x^2^` displays as x² and `H~2~O` displays as H₂O. These effects
raise or lower the authored text; they do not change letter case or substitute
Unicode superscript and subscript characters. They work with text, not only
digits. Single tildes delimit subscripts; double tildes delimit strikethrough.

A quoted line drops its `>` marker and displays its contents in italic between
typographic quotation marks. Formatting inside the quote remains active. A quote
occupies the authored line; a multi-line quote repeats `>` on each line.

Formula delimiters disappear. Parse the contents as TeX math and render them as
STIX Two Math glyph outlines. `$x^2$` therefore uses mathematical glyphs and
placement rather than the superscript text effect. `$$...$$` selects display
math layout, but it remains inside the description's current display line; it
does not create a document-level block. A formula is one wrapping unit. If its
TeX is invalid or unsupported, display the complete source including dollar
delimiters literally.

### 3.1 Delimiters and combinations

Emphasis and code follow [CommonMark](https://spec.commonmark.org/).
Strikethrough, superscript, and subscript extend this notation with the
delimiters in §3. Single `^` and `~` pairs normally delimit text at word
boundaries. Index notation also permits a pair in otherwise plain text when at
least one outside neighbor is alphanumeric and the enclosed text is nonempty and
has no leading or trailing whitespace. This makes `x^2^` and `H~2~O` useful.
Escaped markers and code spans remain literal. The three supported tag pairs
denote text effects, not HTML content.

Supported effects may combine. For example, `***text***` is bold italic, and
`**very *important* text**` adds italic to the bold word `important`.
Underscores follow Markdown emphasis rules, which leave `some_identifier`
literal. Source-derived labels remain outside Markdown interpretation
altogether.

For nested superscripts or subscripts, the innermost position wins; nesting does
not repeatedly shrink the font or accumulate baseline shifts.

Unmatched delimiters remain text. Invalid TeX and formatting introduce no syntax
errors in otherwise valid descriptions. Unsupported constructs are preserved
literally under §3.3.

### 3.2 Escapes, code, and formulas

Use CommonMark escape rules. Outside code, a backslash can escape ASCII
punctuation, including formatting markers. A backslash before an ordinary letter
remains literal.

The following Rust literals have the same value and display literal
`**important**`:

```rust
"\\*\\*important\\*\\*"
r"\*\*important\*\*"
```

Code spans use matching backtick runs and CommonMark whitespace handling. Longer
backtick delimiters allow literal backticks inside code. Code contents have no
nested Markdown interpretation, syntax highlighting, or evaluation.

Formulas use matching `$` or `$$` delimiters. Inline formulas must be nonempty
and have no leading or trailing ASCII whitespace; display formulas may have that
whitespace. Backslash escapes can keep dollar signs literal. Within a recognized
formula, backslashes belong to TeX commands and no Markdown formatting applies.
Formulas do not evaluate code or load external resources.

Character references follow CommonMark: outside code and formulas, `&amp;`
displays as `&`; escaping the ampersand or putting the reference in code keeps
it literal. Character references alone do not enable HTML.

### 3.3 Lines and unsupported constructs

Interpret each authored line independently. A newline in the decoded Rust string
remains an explicit line break, including empty lines. Spans cannot cross those
newlines. Automatic wrapping does not introduce another parsing boundary.

Formatting applies to an ordinary CommonMark paragraph or a single block quote
around one paragraph. If a line instead forms other document structure, display
that entire line literally. For example, `# **Warning**` stays exactly
`# **Warning**`, without becoming a heading. A nonempty authored line must not
disappear; a link reference definition, for example, stays literal.

Within a paragraph, links, images, and HTML other than the three exact tag pairs
above are literal source spans. Copy the complete original span once, including
its contents, punctuation, and delimiters, rather than interpreting its
children. Thus `[**label**](url)` remains exactly that text. Supported
formatting elsewhere on the line still applies. Unsupported extensions,
including tables, add no interpretation of their own.

Preserve ordinary spacing, including leading and trailing whitespace, using
source ranges where interpretation omits it. Only recognized inline syntax may
transform text: delimiters, escapes, character references, and code span
whitespace. Do not add smart punctuation or prose normalization. Interpret lines
without their newline separators, so trailing spaces or a final backslash
introduce no additional Markdown line break.

Automatic wrapping may split a styled span across display lines; each part
retains its effects. Wrapping and shortening split text only between Unicode
grapheme clusters, including inside code and across adjacent spans. A rendered
formula remains an indivisible unit.

## 4. Presentation and accessibility

Measure and wrap the interpreted text before drawing it. Delimiters that become
formatting consume no width; literal markers still do. Measurement must account
for each span's presentation, including bold weight, italic overhang, monospace
advances, and the size and baseline shift of superscripts and subscripts. A
formula supplies its own exact width, ascent, and descent from the math layout.

Retain each label's existing base size and alignment. Uncolored spans keep the
label's existing color. Bold must be visibly stronger than that label's base
weight, including labels that already use heavier text. Italic changes font
style, strikethrough draws a line through the text, and code uses a semibold
system monospace stack without a separate box or background. Superscripts and
subscripts use a smaller font and shift above or below the surrounding baseline.
Their exact scale and offset are renderer presentation choices. Quoted text uses
the same italic effect with visible quotation marks. Underline adds a line
beneath the text, and highlight paints a pale background behind its measured
span without changing the text color. Palette colors use fixed, dark SVG colors
chosen to remain legible on white, the node fills, and the highlight background.
These effects may surround formulas: their paths take the selected text color,
and highlight and underline are drawn around the formula's measured bounds. An
explicit TeX color overrides a surrounding palette color within its math scope;
uncolored formula content inherits the surrounding color. Formulas use the
description's base font size and align their math baseline with surrounding
text. Subsequent plain text returns to the base size, weight, and baseline. Use
the existing line height as a minimum and grow it from formula metrics when
required. Effects must not make text escape its measured bounds or overlap
adjacent content.

Formatting may change pixel dimensions and spacing. It must not change the
verified arrangement's rows, columns, branch order, connections, or cycle
structure. Existing geometry and label-clearance checks still apply.

The expanded-cycle caption keeps its existing two-line limit and may shorten or
disappear when space is insufficient. Shorten the interpreted spans at grapheme
boundaries, retaining their effects, and append an unstyled ellipsis measured in
the caption's base style. Never truncate the markup string and interpret the
prefix again: that could turn a valid opening delimiter into visible
punctuation.

Tooltips and the accessible diagram description use the full plain-text
projection of the interpreted description. Remove bold, italic, strikethrough,
and code delimiters, resolve escapes and character references, and retain code
contents and literal fallback text. Remove supported tag delimiters while
retaining their contents. Represent superscripts as `^(text)` and subscripts as
`_(text)` so their position is not lost: `x^2^` becomes `x^(2)`, and `H~2~O`
becomes `H_(2)O`. Keep struck-out words, typographic quotation marks, and the
TeX body of each rendered formula in that projection. It must remain complete
even when a cycle caption is shortened or omitted. Styling has no execution
meaning and must not be the only way an author communicates a condition or a
branch's meaning.

SVG output remains deterministic and standalone. Serialize effects with
renderer-owned styles on `<tspan>` elements and formulas as nested SVG
containing glyph paths and rules. Do not introduce HTML, `foreignObject`,
JavaScript, external resources, or user-supplied CSS. Escape every text span,
tooltip, and accessible description as XML, and do not insert whitespace between
adjacent spans. Validate XML-incompatible characters before formatting so no
effect can hide invalid input. Validate interpreted text as well, including
characters introduced by numeric references, before serializing it.

## 5. Semantic and processing boundary

Keep the original decoded descriptions for nonempty-description checks,
diagnostics, arrangement checks, and Rust lowering. Formatting applies only to
presentation and introduces no new compiler error. A formula that cannot be
parsed or laid out, uses unsupported features, or requires unavailable glyphs
remains literal as defined in §3, rather than causing a renderer error.

Interpret each authored line into styled spans for text, code, formulas, and the
three paired style tags. Track active effects across balanced delimiters and
flatten combinations for measurement. Preserve source ranges for literal
fallback and spacing. Apply the intraword-index exception in §3.1 only to
remaining plain spans. Do not generate HTML as an intermediate format.

Parse and lay out each valid formula once. Keep its box metrics and rendered
output with its span, and keep that span atomic during wrapping. Scale the
formula to the label's font size during SVG serialization; position adjacent
text using measured advances without browser scripting.

Preserve description provenance so source-derived and generated labels remain
literal. Derive visible text and its full plain-text projection from the same
spans. Carry those spans through measurement, wrapping, shortening, and
serialization; do not reparse in the serializer or strip markers after layout.

Every renderer must follow the same notation and literal fallback rules.

## 6. Compatibility and RFC integration

This is a presentation change for existing descriptions containing recognized
marker pairs, escapes, or character references. For example, a description
containing `**urgent**` previously showed asterisks and will now show bold text.
Escape those markers to keep them visible. Raw Rust strings bypass Rust escapes,
not this notation. Descriptions without recognized notation keep their existing
presentation. Source-derived labels and generated Rust remain unchanged.

Earlier accepted RFCs remain unchanged. This RFC supersedes the following
provisions only within the scope of description formatting:

- [RFC 0002 §6](0002-visual-language.md#6-wires-and-labels): the requirement to
  display exact authored text now permits the description interpretation defined
  here, while preserving the resulting content.
- [RFC 0002 §4.8](0002-visual-language.md#48-cycle) and
  [RFC 0003 §3](0003-svg-renderer.md#3-svg-output): tooltips and accessibility
  retain the full plain-text projection defined in §4 of this RFC, including
  when a caption is shortened or omitted. Description measurement and display
  use the same styled spans as defined in §§4–5.

The requirement for nonempty Rust description strings in
[RFC 0001 §3](0001-language.md#3-block-statements) and all other provisions of
the earlier RFCs remain in force.

## 7. Acceptance criteria

- Every supported effect, combination, escape, quote, formula, literal fallback,
  and explicit newline follows §3.
- Wrapping, shortening, formula placement, accessibility, and XML validity meet
  §4, including when several effects occur in one description.
- A flow using the complete notation retains the same Rust behavior as its
  plain-text equivalent.

## 8. Color limits

The five color tag names have renderer-owned definitions. Color tags with other
names, color codes, or CSS declarations remain literal fallback. A color tag
affects only its contents, including formulas; text after the closing tag
returns to the enclosing or base color.

Within formulas, support the math renderer's TeX color commands, including
`\color`, `\textcolor`, `\colorbox`, `\fcolorbox`, and `\definecolor`. TeX color
names and models follow that renderer's rules. A color definition alone does not
color any glyph; it takes effect when a later command uses the defined name.
Unknown names and unsupported models leave the complete formula literal.
