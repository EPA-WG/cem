# Indent Size vs Tab Size

> **Scope:** This file is a decision note and rationale archive. AI agents,
> automation, and implementation work must not treat it as active project
> instructions, acceptance criteria, or formatter requirements. Use `CLAUDE.md`,
> `docs/todo.md`, package-level README files, and package-level AC documents for
> executable guidance. If this note conflicts with those sources, those sources
> win.

## Chosen page parameters
* `indent` - 4 characters
* `tab-size` - 8 characters
* `wrapColumn` - 100 characters

This note separates two settings that are easy to conflate:

- `indent`: the structural step used to show block nesting.
- `tabSize`: the visual width assumption used when rendering a literal tab
  character (`\t`).

They answer different questions. `indent` asks, "How far should the next block
begin?" `tabSize` asks, "Where is the next tab stop on this line?"

## Line Length Context

The older 80-column code convention has several overlapping roots. IBM's
punched cards had 80 columns, and the DEC VT100 exposed an 80-character line
mode. The 80-column limit also remains close to accessibility and prose
readability guidance: WCAG's AAA visual-presentation guidance requires a
mechanism to make blocks of text no wider than 80 characters, while typography
sources commonly recommend body text around 50-75 characters or, more broadly,
45-90 characters.

The newer 100-120 column range is common in some coding ecosystems, but it is
not a universal replacement for 80:

- PHP PSR-12 defines 120 as a soft limit and still says lines should preferably
  be no longer than 80.
- Google Java Style, Android Kotlin Style, and rustfmt use 100.
- ktlint varies by profile: its current `ktlint_official` profile uses a wider
  `max_line_length`, its `android_studio` profile uses 100, and its
  `intellij_idea` profile leaves the rule off.
- Prettier defaults to 80 and explicitly treats print width as a formatting
  target, not a hard maximum.
- Python PEP 8 keeps 79 as the standard-library limit and allows teams to move
  toward 99 when they agree that wider lines improve readability.

So the extra 40 characters in a 120-column budget are not automatically
justified. They are useful when the language or domain naturally produces wide
type signatures, namespaces, fluent calls, schema paths, or generated names.
They are not a good substitute for managing nesting depth.

## The Indentation Budget

Indentation is a scope outline. It should make block starts easy to scan; it is
not meant to reserve room for words.

A useful way to evaluate a line-length policy is to subtract visible structural
indentation from the available text area. For five visible structural levels,
the remaining payload width is:

|       | Indent Unit | Indent Cost | Remaining at 80 | Remaining at 120 |
|-------|-------------|-------------| --------------- | ---------------- |
| [ ]   | 2 spaces    | 10          | 70              | 110              |
| [X]   | 4 spaces    | 20          | 60              | 100              |
| [ ]   | 8 columns   | 40          | 40              | 80               |

This is the strongest argument against using an 8-column indent unit for code
structure. With five levels of visible nesting, an 8-column indent consumes the
entire 40-character difference between 80 and 120. A 4-space indent leaves about
60 characters at an 80-column limit, which is already inside the comfortable
range for prose-like text and usually enough for readable code.

The "five levels" example should be read as a worst-case acceptable visible stack, not as a
goal:

1. module, package, class, or top-level declaration, when the language visibly
   indents it;
2. function or method;
3. first control-flow block;
4. second nested control-flow block;
5. third nested control-flow block.

Three nested control-flow blocks are already a warning sign. Linters and code
quality tools often model this directly: ESLint has `max-depth` for nested
blocks, and Sonar rules flag deeply nested control flow as a maintainability
smell. The practical target is:

- 0-1 nested control-flow levels: easy to scan;
- 2 nested levels: acceptable when the conditions are simple;
- 3 nested levels: borderline; prefer guard clauses, extraction, or flatter
  data flow;
- 4+ nested levels: usually a design smell.

## What `tabSize` Actually Means

A literal tab does not mean "insert N spaces". It means "advance to the next tab
stop". With `tabSize = 8`, stops are at columns 0, 8, 16, 24, and so on. With
`tabSize = 4`, stops are at 0, 4, 8, 12, and so on.

That makes tabs useful for coarse field alignment:

```text
name\tcount\tstatus
ada\t12\tready
linus\t7\tblocked
```

But alignment only works when the producer and renderer agree on tab stops. If
one viewer renders tabs at 8 and another renders them at 4, columns shift.
Longer field values also overflow their slot and push following text to a later
stop. For strict tabular output, compute display widths and pad explicitly. For
loose readable output, tabs can be acceptable when the format declares the
`tabSize` assumption.

The historical 8-column tab convention is real: DEC terminals exposed "set
8-column tabs", GNU `expand` defaults to 8-column tab stops, and editors such as
Emacs also document 8-column tab stops as their default. The common explanation
that 8 matches typical English word length is plausible as a mnemonic, but it
should not be treated as the historical reason. Running prose does heavily favor
short words, but word-length distributions vary by corpus and dictionary. The
more reliable operational fact is simpler: an 8-column tab grid divides an
80-column screen into ten coarse fields.

## Project Decision

For CEM formatter options:

- `indent` defaults to four spaces.
- `tabSize` defaults to `8`.
- `wrapColumn` defaults to `100` when a readable formatter performs wrapping.
- `indent` is a generic formatter option and must preserve exact whitespace,
  including spaces or literal tabs.
- `tabSize` is a generic formatter option and must be a positive integer.
- `wrapColumn` is a generic formatter option and must be a positive integer
  when it is active.
- formatters that emit literal tabs must carry the active `tabSize` through
  metadata and previews.
- visual alignment profiles should not use `compact`; `compact` should minimize
  optional spacing and remain the safest profile for interchange output.

The default `tabSize = 8` follows the historical terminal tab-stop convention.
Modern code and structured data can request a denser alignment grid explicitly,
and output metadata makes the assumption visible to preview renderers.

## Practical Rules

Use `indent` for scope:

```text
function
    condition
        action
```

Use `tabSize` only when literal tabs are part of a visual presentation contract:

```text
field\tvalue\tcomment
```

Prefer these defaults:

- code and CEM-readable formatters: 4-space indent;
- body text: around 60-75 characters per line when possible;
- hard code limit: 80, 100, or 120 depending on ecosystem, with 120 treated as
  a soft ceiling rather than a target;
- maximum nesting: keep control flow at 0-2 nested levels in ordinary code and
  treat 3 as a refactoring prompt.

## References

- IBM, "The punched card": https://www.ibm.com/history/punched-card
- DEC VT100 User Guide, 80/132 column mode:
  https://www.vintagecomputer.net/digital/VT100/vt100_manual/chapter1.html
- W3C WCAG 2.2, Understanding SC 1.4.8 Visual Presentation:
  https://www.w3.org/WAI/WCAG22/Understanding/visual-presentation
- Baymard, "Readability: The Optimal Line Length":
  https://baymard.com/blog/line-length-readability
- Butterick's Practical Typography, "Line length":
  https://practicaltypography.com/line-length.html
- PHP-FIG PSR-12, line length:
  https://www.php-fig.org/psr/psr-12/
- Prettier options, print width and tab width:
  https://prettier.io/docs/options
- Python PEP 8, maximum line length:
  https://peps.python.org/pep-0008/#maximum-line-length
- Google Java Style Guide, column limit and block indentation:
  https://google.github.io/styleguide/javaguide.html
- Android Kotlin Style Guide, column limit and indentation:
  https://developer.android.com/kotlin/style-guide
- ktlint standard rules, `max_line_length` profiles:
  https://ktlint.github.io/ktlint/1.8.0/rules/standard/
- rustfmt configuration, `max_width`:
  https://rust-lang.github.io/rustfmt/
- GNU Coreutils `expand`, default tab stops:
  https://www.gnu.org/software/coreutils/manual/html_node/expand-invocation.html
- DEC VT320 User Guide, tab setup:
  https://vt100.net/docs/vt320-uu/chapter4.html
- GNU Emacs Manual, tab stops:
  https://www.gnu.org/software/emacs/manual/html_node/emacs/Tab-Stops.html
- ESLint `max-depth`:
  https://eslint.org/docs/latest/rules/max-depth
- Sonar rule S134, deeply nested control flow:
  https://rules.sonarsource.com/go/RSPEC-134
