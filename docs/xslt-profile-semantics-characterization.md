# XSLT Formatter and Colorizer Profile Characterization

Status: characterization complete; profile semantics require an explicit
decision before implementation.

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

## Current formatter behavior

The public profiles are `compact`, `pretty`, and `tabular`. The pretty artifact
currently resolves to internal metadata value `xml.pretty`. All three profiles:

- emit two generated, unmapped formatter marker/decision tokens;
- then emit one source-mapped writer token for each XML event;
- copy each complete event lexeme without token-level markup decomposition;
- produce byte-identical lexical stylesheet output;
- differ only in formatter profile and `lexical-lossless-*` layout metadata.

The shared typed `xml_event_markup_tokens` helper already splits element
delimiters, names, whitespace, attribute names, equals signs, and attribute
values with token-level ranges. The borrowed XML-family evaluator currently
exposes those markup tokens and layout flags only for SVG and MathML. XSLT does
not yet select that helper, so element attributes are colored as part of one
`syntax.name` event token rather than by token role.

## Current colorizer behavior

The terminal, HTML, and `md` CEMT wrappers attach typed style overlays to the
same formatted owner. Roles are currently event-level: declaration and
processing-instruction use `syntax.keyword`; element events use `syntax.name`;
text uses `syntax.text`; CDATA and entity references use `syntax.string`; and
comments use `syntax.comment`.

Terminal and HTML have explicit writer output selections. The `md` profile
currently records a span/class overlay, but the shared output-color selector
has no Markdown target and therefore writes plain text. “Markdown parity” is
not yet defined as either a typed-overlay contract or a Markdown-encoded writer
surface.

## Decision required

The package README explicitly says stylesheet-aware reflow is not yet defined,
and no existing fixture selects rules for the following behavior:

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

## Recommended policy

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

Implementation must not start until this policy—or an alternative—is accepted,
because adopting it changes observable bytes, source-map segmentation, profile
identity, and color-role contracts.
