# CEM-ML ↔ XML / HTML Cross-Surface Conversion

**Status:** Tier A normative for the CEM-ML → HTML projection direction
and for HTML/XML → canonical CEM-ML reverse serialization. Canonical
CEM-ML plus HTML/XML parity tokenizers lower into the shared event model
today; reverse serialization uses the canonical formatter and records a
`ContentTypeTransform` source-map boundary.

This document fixes the *exact* per-construct conversion rules so all
three surfaces lower into the same schema event stream and AST, with
source-map traceability preserved through the boundary.

## 1. Namespaces

| CEM-ML                            | XML / HTML parity              | Notes |
| --------------------------------- | ------------------------------ | ----- |
| `@ns cem = "https://cem.dev/ns/core/1"` directive | `xmlns:cem="https://cem.dev/ns/core/1"` on the document root | Prefix binding. Lowers to a `NamespaceBinding` record in `NsContext`. |
| `@default html`                   | `xmlns="http://www.w3.org/1999/xhtml"` | Default (blank) prefix binding. |
| `cem:screen` attribute            | `cem:screen` attribute         | Identity. The schema-qualified attribute name is preserved verbatim. |
| Unprefixed element under `@default html` | Unprefixed HTML element | Both resolve to the HTML namespace at their source position. |

**Conversion rule:** every namespace binding is keyed on
`(NamespaceName, NamespaceUri, ByteRange of declaration)`. Repeated
binding names rebind from the source position of the new declaration;
previously resolved nodes retain their original expanded name
(`cem-ml-ac.md` AC-P-10).

## 2. Default Namespace Changes

Default namespace can be rebound mid-document on either surface:

```cem
@default html

{section |
  {svg @xmlns="http://www.w3.org/2000/svg" |
    {path @d="M2 8h12"}
  }
  {input @id=email}
}
```

```xml
<section xmlns="http://www.w3.org/1999/xhtml">
  <svg xmlns="http://www.w3.org/2000/svg">
    <path d="M2 8h12"/>
  </svg>
  <input id="email"/>
</section>
```

**Conversion rule:** the XML attribute `xmlns` and the CEM-ML directive
`@default` lower to the same blank-prefix binding event. Rebinding
inside a subtree (e.g. SVG) is local to that subtree on both surfaces.

## 3. Comments

| CEM-ML            | XML / HTML       | Notes |
| ----------------- | ---------------- | ----- |
| `// line` (to newline) | n/a         | CEM-ML-only line comment. Lowers to `Trivia(Comment)`. |
| `/* block */`     | `<!-- block -->` | Block comments are bi-directional. |
| Inside rich content (triple backticks) | Inside `<![CDATA[ ... ]]>` or raw `<script>` | Preserved verbatim; the schema event stream records a single content scalar. |

**Conversion rule:** the schema event stream emits exactly one
`Trivia(Comment)` event per comment regardless of surface. Line comments
project to `/* ... */` on output when converting CEM-ML → XML/HTML.

## 4. Whitespace

| Position                                | Preservation |
| --------------------------------------- | ------------ |
| Between attributes in a node header     | Dropped — both surfaces accept any whitespace between attributes. |
| Between sibling element children        | Preserved — the event stream emits `Trivia(Whitespace)` events with byte ranges. |
| Inside text content                     | Preserved verbatim. |
| Inside rich content / CDATA / raw text  | Preserved verbatim. |

The light-DOM renderer (`cem_ml::interpreter::light_dom`) currently
collapses inter-element whitespace into the surrounding text; the
authoritative event stream still carries the byte range so the parity
direction can reproduce the original spacing when needed.

## 5. Typed Scopes (Anonymous Scopes)

| CEM-ML                            | XML / HTML parity              |
| --------------------------------- | ------------------------------ |
| `{@type="text/html" \| ...}`       | `<cem:scope type="text/html">...</cem:scope>` |
| `{@type="application/json" \| ...}` | `<cem:scope type="application/json">...</cem:scope>` |

**Conversion rule:** an anonymous CEM-ML scope is a parser/content-type
boundary, not a semantic node. On the XML/HTML side, `<cem:scope>` is a
namespaced wrapper element introduced by the parity tokenizer. The
schema machine sees a `ModeSwitch` event with a `HandoffRecord`
regardless of surface (see
[`../../../docs/cem-ml-syntax.md`](../../../docs/cem-ml-syntax.md)
§"Content-Type Handoffs Stay Schema-Owned").

## 6. Rich Content

| CEM-ML                | XML parity         | HTML parity              |
| --------------------- | ------------------ | ------------------------ |
| ```` ```...``` ```` (triple-backtick) | `<![CDATA[ ... ]]>` | Raw text inside `<script>`, `<style>`, or an explicit `<cem:raw>` wrapper |

**Conversion rule:** the body bytes survive verbatim. The schema event
stream emits a single `Value(Text)` event whose byte range covers the
fenced body (excluding the opening/closing delimiters). Round-trip
identity holds on the *body bytes*; the choice of fence (triple
backtick vs CDATA vs `<script>`) is surface-determined.

## 7. `$` Expression Nodes

| CEM-ML            | XML / HTML parity                                 |
| ----------------- | ------------------------------------------------- |
| `{$ expr}`        | `<cem:expr>expr</cem:expr>`                       |
| `{$ \| expr}`      | `<cem:expr>expr</cem:expr>` (explicit boundary form is informational only) |

**Conversion rule:** the expression body is passed through to cem-ql as
opaque content. The schema event stream emits an `OpenScope` named `$`
followed by a single `Value(Text)` carrying the body, then `CloseScope`.
The XML parity surface uses a reserved `cem:expr` element so the body
stays inside the same namespace as the rest of the CEM annotations.

## 8. Attribute cem-ql Spans

| CEM-ML                                | XML / HTML parity                       |
| ------------------------------------- | --------------------------------------- |
| `{button @disabled={.busy} \| Save}`   | `<button disabled="{.busy}">Save</button>` |
| `{button @label="Hello {.name}" \| Save}` | `<button label="Hello {.name}">Save</button>` |

**Conversion rule:** attribute-value `{...}` cem-ql spans are
identifiable lexically on both surfaces. The tokenizer/normalizer
records the span as the attribute's value, wrapped in literal `{...}`
braces so the cem-ql layer can identify and parse it without
re-scanning. Literal `{` / `}` characters in a non-template attribute
value are escaped as `{{` / `}}` on both surfaces.

## 9. Source Maps

Every cross-surface conversion preserves source-map identity:

- **Byte ranges** on the originating side stay attached to the lowered
  event / token / AST node.
- **Frames** are origin-first per `cem-ml-stack-design-impl.md` §2.2,
  so the original `CemTokenizer` / `HtmlTokenizer` / `XmlTokenizer`
  frame survives a conversion.
- **Cross-surface conversions** push an additional
  `TransformKind::ContentTypeTransform { content_type }` frame onto the
  output, recording the conversion boundary. The Rust enum for this
  variant already exists in
  [`../src/source_map.rs`](../src/source_map.rs).

## 10. Conversion Test Matrix

Tier A runs both the projection direction CEM-ML → HTML via the
light-DOM renderer and the reverse serialization direction HTML/XML →
canonical CEM-ML via the formatter. The integration tests are:

| Test | Direction | Location |
| ---- | --------- | -------- |
| `every_canonical_fixture_matches_snapshot` | `.cem` → light-DOM HTML byte-identical | `packages/cem_ml/tests/transform_snapshots.rs` |
| `every_canonical_fixture_runs_through_every_layer` | `.cem` → events → AST → render | `packages/cem_ml/tests/end_to_end.rs` |
| `canonical_cem_projection_preserves_schema_event_identity` | `.cem` → events → render → re-tokenize → events | `packages/cem_ml/tests/cross_surface_projection.rs` |
| `every_html_parity_fixture_serializes_to_canonical_cem_ml` | HTML → canonical `.cem` byte-stable after reformat | `packages/cem_ml/tests/reverse_conversion.rs` |
| `namespace_rebinding_xml_fixture_serializes_to_canonical_cem_ml` | XML → canonical `.cem` byte-stable after reformat | `packages/cem_ml/tests/reverse_conversion.rs` |

HTML parity fixtures already lower through the shared event stream in
`packages/cem_ml/tests/fixture_pair.rs`. XML parity currently covers the
namespace-rebinding fixture in
`packages/cem_ml/tests/namespace_rebinding_fixtures.rs`. The CLI/library
conversion surface exposes reverse serialization as `convert
--from-format html|xml --to-format cem`.

## 11. Non-Lossless Constructs

Conversion is lossless for the constructs in §§1–9 when the source
preserves enough byte-range identity to round-trip. The following are
**not** byte-stable across a round trip and require an opinionated
canonical form:

- Attribute *order* — the formatter normalizes to `(namespace,
  local_name)` regardless of original surface.
- Quote style — `"..."` is canonical; both surfaces accept `'...'`.
- Inter-attribute whitespace — collapsed.
- Inter-sibling whitespace — preserved as a single `Trivia(Whitespace)`
  event but normalized when rendered.

Authors who need byte-stable round trips should run the formatter
(`cem_ml::formatter::format_source`) on the source surface before
conversion; the formatter is idempotent across all five canonical
fixtures
(`packages/cem_ml/src/formatter.rs::every_canonical_fixture_formats_idempotently`).
