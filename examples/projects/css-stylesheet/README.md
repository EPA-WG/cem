# CSS stylesheet

Open **Examples → Content types and schemas → Documents and UI → CSS stylesheet**.
The same ordinary project appears under **Queries and transformations**. Clone
it to edit saved source and build resources independently of the catalog original.
Typical uses are reviewing a stylesheet's lexical and semantic structure,
repairing syntax, preserving formatting and publishing a small styled document.

## Source identities and configurations

`styles/cards.css` uses `text/css` and schema
`https://cem.dev/ns/data/css/1`. The native loader retains its CSS tokens,
comments, lexemes, original bytes and source ranges. Because this source contains
`@scope`, its native entry mode is `scoped-style-block`. The semantic CEM tree
and the lexical inspection vocabulary are separate views of the same source.

| Configuration | Typical use | Expected result |
| --- | --- | --- |
| Inspect native CSS source | Review lossless native CSS source/token fields | CSS document identity, entry mode and original byte length |
| Inspect retained CSS rules | Review semantic rules, declarations and selectors | Native CSS-namespace nodes with original byte ranges |
| Validate the scoped CSS source | Check native syntax and source policy | Zero errors and warnings |
| Format CSS (pretty profile) | Export conservative native CSS formatting | Exact bytes in `expected/cards.css` |
| Repair the unclosed CSS layer | Locate and repair a missing closing delimiter | `cem.css.invalid_token`, then zero errors after repair |
| Build and preview the scoped stylesheet | Build one HTML entry and its stylesheet asset | `generated/cards.html` and `generated/cards.css` |

The CSS package's current `compact`, `pretty` and `tabular` profiles preserve
conservative lexical boundaries. The name `pretty` does not promise that existing
layout, strings, comments, functions, custom properties or nested blocks are
rewritten. The example's formatted CSS is exactly 241 UTF-8 bytes.

## Scoped output and saved edits

The stylesheet uses an `examples` layer and an ordinary browser
`@scope (.catalog-card)` rule. The heading inside the card is teal; the outside
heading remains unaffected by that rule. Card padding is `0.5rem`, with a
`currentColor` border. This demonstrates native handling and browser rendering
of authored CSS; it is not DCE host-selector compilation, Shadow DOM isolation
or a complete cascade evaluator.

In an editable copy, change `rgb(24, 92, 128)` to `rgb(128, 48, 96)`, save, then
rerun the formatter and build. The stylesheet artifact changes and the card's
heading changes color. Change the card padding independently to compare native
source structure with browser geometry. Unsaved drafts do not replace persisted
command inputs.

`graphs/cards.cem` imports admitted HTML from `data/preview.html` and calls the
native CEMT wrapper `templates/cards.cemt`. The wrapper constructs the document
head and its relative `cards.css` link while copying the retained `main` subtree.
The source HTML deliberately contains no external stylesheet reference: directly
loading an HTML source with a link requires an explicit native resource policy.
The output link addresses a declared artifact in the immutable build tree.

A second graph import loads the actual CSS source with its own media/schema
identity and exports native `text/css`. It does not mark that source opaque,
relabel SCSS as CSS, or use a browser parser as the source implementation. The
entry artifact is explicitly `generated/cards.html`; the stylesheet is its
companion `generated/cards.css`. The expected HTML is exactly 563 bytes, including
the native writer's final layout spaces.

Run **Build project** to use the shared artifact preview. It resolves only
admitted build-tree assets and keeps example styles inside the sandboxed output
frame. Source colors and stylesheet edits do not change Studio's theme or
storage keys. No scripts, network fonts or sample-specific UI code are present.

To inspect source provenance, run **Build and preview the scoped stylesheet**,
open **Commands and results → Results → Provenance**, and choose a native
source-map row. The HTML wrapper has template origins, copied document content
has source boundaries, and exported CSS retains native CSS source ranges where
mapped. Generated layout does not acquire invented character lineage.

Native build, from this project directory:

```sh
cem-ml transform --config graphs/cards.cem --config-content-type application/cem \
  --config-schema https://cem.dev/ns/cli/transform-config/1 --scope-budget queueSize=1024
```

## Failure and repair

`failures/unclosed-rule.css` omits the final `}` closing the layer. It is separate
from the graph input. Run **Repair the unclosed CSS layer**: native validation
reports one `cem.css.invalid_token` at line 2, column 17, original byte offset 68.
Append the missing closing brace and newline, save and rerun. The repaired bytes
match the valid stylesheet exactly, with zero errors and warnings.

A malformed saved graph input must be repaired before publishing a successful
new build. Studio keeps the last successful output when a build fails; source
and failed-attempt diagnostics remain available separately.

## Limits and independent offline copies

Native validation covers the documented CSS syntax, encoding and resource-policy
facts, not full property/value conformance, browser support or cascade results.
Unknown at-rules and vendor/custom-property token streams can remain preserved
without an execution claim. Parsing, inspection and formatting do not fetch
imports, URLs or fonts. Native `@import` and URL policy refusals remain distinct
from the shared artifact viewer's bounded resolution of declared local assets.
See the [CSS package](../../../packages/cem_ml/schema-packages/css/v1/README.md).

After runtime and project resources are cached, saved copies can reopen,
validate, format and rebuild offline. Sources, commands, templates and drafts
remain independent of the read-only original and other copies. Existing entered
credential recovery refusal and disabled automatic whole-Studio recovery are
unchanged.
