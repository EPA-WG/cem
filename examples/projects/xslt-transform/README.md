# XSLT transformation

This ordinary Studio project compares bounded native XSLT with the related
**CEM transformation** project. Both own the same UTF-8 XML order document and
produce the same explicit HTML entry. Open **Content types and schemas → Queries
and transformations**, select this project, then clone it to edit.

## Resources and configurations

- `orders-source` — `data/orders.xml`, `application/xml`,
  `https://cem.dev/ns/data/xml/1`: three orders, ready/draft status and totals.
- `report-template` — `templates/report.xsl`, `application/xslt+xml`,
  `https://cem.dev/ns/transform/xslt/1`: a named `report` template with a declared
  `minimum` parameter, native XPath selection and literal report construction.
- `page-xslt` — `templates/page.xsl`, the same standard XSLT identity: a named
  `page` template with `minimum` and `heading` parameters for direct HTML output.
- `page-template` — `templates/page.cemt`,
  `application/vnd.cem.template+cem`, `https://cem.dev/ns/template/cem-native/1`:
  the declarative CEM-native view of the report model.
- `report-graph` — `graphs/report.cem`, `application/cem`,
  `https://cem.dev/ns/cli/transform-config/1`: explicit XSLT report and CEM view
  stages, graph parameters, and the HTML preview entry.
- `invalid-template` and `invalid-orders` are independent failure resources.
  Exact expected HTML and native source-format output are project resources too.

| Configuration | Typical use |
| --- | --- |
| `validate-template` | Validate the original standard XSLT source contract. |
| `inspect-template` | Inspect its native source tree and original source ranges. |
| `format-template` | Apply native tabular source formatting, including its current XML reflow. |
| `validate-graph` | Check explicit stage identities, parameters and export declarations. |
| `inspect-graph` | Inspect the native graph source and its XSLT-to-CEM stage boundary. |
| `repair-template` | Restore the missing stylesheet version declaration. |
| `transform-html` | Render the original retained XML directly through the named XSLT page template. |
| `build-report` | Filter the XML through XSLT, then render the constructed report through CEM. |

## Native boundaries and supported subset

The graph's first stage runs named XSLT `report` on the original retained XML
owner. Native XPath selects ready orders whose totals are at least thirty, then
XSLT constructs a report tree. The intermediate stage declares `application/cem`
and the CEM-ML schema: it passes a constructed CEM-tree sequence directly to the
CEM-native view. `generated/index.html` is the explicit final entry and contains
`O-101` and `O-103` under **Ready orders**.

A constructed transform tree is not an original retained XML/XDM owner. The
current downstream XPath/XSLT path evaluator requires such an owner, so this
project intentionally demonstrates the supported mixed XSLT-to-CEM graph rather
than pretending an encoded XML string or JSON AST is a native node. No execution
handoff serializes and reparses the model. The CEM project separately demonstrates
an application JSON artifact with its explicit unmapped-writer boundary.

`transform-html` applies `templates/page.xsl` directly to the original XML with
explicit `minimum=30` and `heading=Ready orders` parameters. It produces the same
list-item content as the graph; native formatting whitespace differs. Each path
has its own byte-exact fixture. The graph's HTML output spans/sidecar identify
original `templates/page.cemt` ranges. They are template provenance, not invented
character mappings from computed results into source data.

Standard `application/xslt+xml` selects the native bounded XSLT adapter. Neither
browser `XSLTProcessor` nor the `custom-element-xslt` compatibility dialect is
used. This is not a complete XSLT 3.0 conformance claim. The example uses named
templates, declared literal parameters, native XPath paths/predicates, `for-each`,
`value-of`, literal result elements and attribute value templates. External
stylesheet imports, arbitrary URI access, extension functions and unlisted
instructions are not granted by the example.

Default/complete runtime execution is covered. The opt-in optional-schema WASM
profile still refuses executable XSLT graphs where original parser/name-resolution
contracts are not available to bundle compilation. Loading schema source assets
alone does not prove executable XSLT activation. Keep that refusal until the
separate native contract follow-up is repaired; do not substitute a browser engine.

## Failure and repair

`repair-template` validates `failures/report.xsl`, whose stylesheet root lacks
`version="3.0"`. Native validation reports one error,
`cem.xslt.version_missing`, at line 1, column 1, byte 0. Restore the version
attribute to recover the exact valid report source and validate again. This
source check is independent from execution.

Replacing saved `data/orders.xml` with malformed `failures/orders.xml` makes the
build fail without publishing a new entry. Studio retains the last successful
preview until valid XML is saved and rebuilt. Read-only catalog failures remain
unchanged when a saved copy is repaired.

## Saved changes and independent copies

Change graph minimum `@value=30` to `@value=45` to retain only `O-101`; change the
XML customer `Café Noord` to `Café Haven`; change `@name=ul` to `@name=ol` in the CEM
view; or change graph heading **Ready orders** to **Dispatch queue**. These exercise
independent source, template and parameter inputs. Direct XSLT has its own
explicit command parameters, so editing the graph does not silently edit that
command configuration.

Unsaved edits do not replace persisted inputs. Saved independent copies rebuild
offline through Studio's shared source editor, commands and preview controls.
No sample-specific UI JavaScript, browser query parser or serialized AST handoff
is introduced. Existing saved projects and drafts are preserved.

Entered passwords and one-time codes still block recovery until explicitly
cleared. Automatic whole-Studio recovery stays disabled.
