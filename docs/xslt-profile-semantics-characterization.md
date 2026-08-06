# XSLT Formatter and Colorizer Profile Characterization

Status: recommended profile policy accepted, implemented, and covered by the
native XSLT output contracts.

## Native boundary

The production owner is `XsltStylesheetAst`, not `XsltDocumentAst`. It retains
the package-owned `XmlDocumentAst` event stream, XSLT facts, version, source
media identity, ranges, maps, and line ending. Output passes a borrowed
`XmlFamilyDocumentCemtSubjectRef::Xslt` evaluator view to the package CEMT
formatter. No JSON value, serializer DTO, source reparse, or replacement XML
tree exists between the lifecycle AST and the materialized writer-token stream.

The characterization fixture covers:

- the XSLT namespace and stylesheet module root;
- namespace bindings and an explicit extension-element namespace;
- `match`, `test`, and `select` XPath-bearing attributes;
- attribute value templates on literal-result and extension elements;
- `xsl:text`, literal result elements, comments, CDATA, and extension content;
- the separate legacy custom-element alias boundary.

Tests prove that each native XML event retains a non-empty source range and map,
that event lexemes reconstruct the source exactly, and that XPath/AVT/text/CDATA
lexemes survive the current output path unchanged. Legacy fragments are
accepted only through an explicit compatibility alias; the standard XSLT
identity rejects them as a non-stylesheet root.

The fixture also exposed and closed a cross-language validation leak: the CLI's
generic CEM-QL embedding pass previously interpreted XSLT AVTs such as
`{$label}` as CEM-QL content expressions when an XSLT input defaulted to the CEM
tokenizer format. Explicit standard or compatibility XSLT content/schema
identity now leaves those expressions exclusively in the XSLT AST. A focused
CEM-QL bridge test and schema-owned CLI example gate enforce that ownership.

## Characterized formatter behavior

Before this implementation, the public profiles were `compact`, `pretty`, and
`tabular`, while the pretty artifact resolved to internal metadata value
`xml.pretty`. All three profiles:

- emit two generated, unmapped formatter marker/decision tokens;
- then emit one source-mapped writer token for each XML event;
- copy each complete event lexeme without token-level markup decomposition;
- produce byte-identical lexical stylesheet output;
- differ only in formatter profile and `lexical-lossless-*` layout metadata.

The shared typed `xml_event_markup_tokens` helper splits element
delimiters, names, whitespace, attribute names, equals signs, and attribute
values with token-level ranges.

## Implemented formatter and colorizer behavior

XSLT now selects the shared borrowed XML-family token and safe-layout view from
its native `XsltStylesheetAst`; the XSLT package still owns all formatting and
color policy. The formatter implements distinct `structural-compact`,
`structural-pretty`, and `attribute-tabular` layouts. It treats `xsl:text`,
mixed/non-whitespace content, CDATA/entity content, and every non-XSLT namespace
subtree (including no-namespace literal result elements) as lexical islands.
XPath and AVT attribute-value tokens therefore retain their exact bytes.

The emitted formatter profile is `pretty`; the public CEMT function keeps the
registry's canonical `xml.pretty` selector as the required compatibility alias
and passes `pretty` to the package helper. Configured indentation, line ending,
and tab-size metadata are honored. Lexical markup tokens retain token-level
source maps and output origins; formatter-generated whitespace is explicitly
unmapped.

The terminal, HTML, and `md` CEMT wrappers attach typed style overlays to the
same formatted owner. XML markup token roles distinguish punctuation, element
names, attribute names, and attribute values. Event roles remain the baseline
for declarations, processing instructions, text, CDATA/entities, and comments.

Terminal and HTML have explicit writer output selections. The `md` profile
records a span/class overlay, while the shared output-color selector writes
plain text. Tests verify that terminal, HTML, and Markdown-visible text matches
the uncolored writer output.

## Accepted decision

The characterization found no pre-existing package rule for the following
behavior, so implementation paused until the recommended policy below was
accepted:

1. Whether `compact` removes only structural whitespace or also normalizes
   markup spacing.
2. Whether `pretty` uses one element event per line, how comments and processing
   instructions attach, and whether its canonical metadata remains
   `xml.pretty` or becomes `pretty`.
3. Whether `tabular` means one attribute per line, aligned attributes, or a
   stylesheet-specific declaration/template table.
4. Which scopes are indivisible lexical islands beyond the required XPath,
   AVT, `xsl:text`, mixed text, CDATA/entity, and foreign-namespace content.
5. Whether XSLT token roles distinguish XSLT instruction names, literal-result
   names, extension names, namespace declarations, XPath attributes, and AVTs.
6. Whether `md` remains a style-overlay alias with plain output or gains a
   first-class Markdown writer target.

## Accepted policy

Use the established SVG/MathML structural formatter as the mechanical baseline,
with XSLT-owned policy:

- `compact`: remove only structural inter-element whitespace; retain all
  lexical token bytes and add no generated layout.
- `pretty`: insert configured line ending plus depth-based indentation at safe
  element/comment/PI boundaries.
- `tabular`: use the pretty event layout and put each attribute on its own
  depth-plus-one line; do not align with generated padding.
- Treat `xsl:text`, any mixed/non-whitespace text scope, CDATA/entity content,
  and every non-XSLT namespace subtree as indivisible lexical islands.
- Preserve complete attribute-value tokens, so XPath and AVT bytes are never
  rewritten before the independent XPath-fusion work lands.
- Normalize the public and artifact profile name to `pretty`, retaining
  `xml.pretty` only as a compatibility alias if required.
- Keep XML token roles as the stable baseline and add XSLT-specific instruction,
  literal-result, extension, XPath-attribute, and AVT roles only if their public
  palette contract is explicitly accepted.
- Treat `md` as typed overlay metadata with plain writer output for this slice;
  schedule Markdown encoding separately rather than silently inventing it.

The accepted policy changes observable bytes, source-map segmentation, profile
identity, and color-role contracts. Focused contracts cover those changes, the
schema-package fixture covers lexical islands, and the full package/core/CLI
gates prevent drift across the native pipeline.
