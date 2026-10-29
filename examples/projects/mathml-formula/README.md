# MathML formula

Inspect a mathematical formula and display it on a page. Compare presentation and content MathML, query the formula structure and repair a missing namespace.

## Essential files

- [data/formula.mml](data/formula.mml) — A formula expressed as presentation MathML.
- [data/content.mathml](data/content.mathml) — A formula expressed as content MathML.
- [queries/formula.xpath](queries/formula.xpath) — Reads the formula structure.
- [graphs/formula.cem](graphs/formula.cem) — Builds a page containing the formula.
- [failures/missing-namespace.mml](failures/missing-namespace.mml) — A formula with a missing namespace to repair.

## Using the example

Open **Examples → Content types and schemas → Documents and UI → MathML formula**,
or find the same project under **Queries and transformations**. Clone it to make
an editable saved project. It demonstrates native source handling and a shared
HTML preview of a small Presentation MathML expression, with an independent
Content MathML source-validation example.

## Source identities

- `data/formula.mml` uses `application/mathml-presentation+xml` and schema
  `https://cem.dev/ns/data/mathml/1`. Its `math` root is in
  `http://www.w3.org/1998/Math/MathML`, with an accessible label and block display.
- The expression is the fraction `(x² + 1) / 2`, authored with `mfrac`, `mrow`,
  `msup`, `mi`, `mo` and `mn`. The browser lays out these presentation elements.
- `data/content.mathml` uses `application/mathml-content+xml` with the same schema.
  It describes `x + 1` using `apply`, `plus`, `ci` and `cn`. It is a separate
  validation resource, not the rendered expression's backing source.

The primary `application/mathml+xml` identity supports the generic mixed profile;
the presentation and content aliases select their corresponding expression
profiles. This example does not translate Content MathML into Presentation
MathML, evaluate algebra or claim complete MathML Core conformance.

The other declared resource roles in [example.json](example.json) are:

| Resources | Content type | Schema identity |
| --- | --- | --- |
| `queries/formula.xpath` | `application/vnd.cem.xpath` | `https://cem.dev/ns/query/xpath/1` |
| `templates/formula.cemt` | `application/vnd.cem.template+cem` | `https://cem.dev/ns/template/cem-native/1` |
| `graphs/formula.cem` | `application/cem` | `https://cem.dev/ns/cli/transform-config/1` |
| `expected/summary.json` | `application/json` | `https://cem.dev/ns/data/json/1` |
| `expected/formula.html` | `text/html` | `https://cem.dev/ns/data/html/1` |

The malformed and expected formatted presentation resources keep the presentation
identity above. These declarations do not imply support for every standalone
query/template/configuration source operation.

## Configurations

| Configuration | Typical use | Expected result |
| --- | --- | --- |
| `inspect-mathml` — Inspect retained MathML formula | Review native structure and original source ranges | Namespaced fraction and power elements |
| `validate-presentation` — Validate Presentation MathML | Check XML and supported presentation-profile facts | Zero errors and warnings |
| `validate-content` — Validate Content MathML source | Check the separate content-profile example | Zero errors and warnings |
| `query-mathml` — Query the MathML formula structure | Read a namespace-aware structural summary | Seven strings matching `expected/summary.json` |
| `format-mathml` — Format Presentation MathML (pretty) | Export readable same-schema source | Exact bytes in `expected/formula.mml` |
| `repair-mathml-namespace` — Repair the MathML namespace | Diagnose missing MathML document identity | `cem.mathml.namespace_missing`, then zero errors after repair |
| `build-html` — Build and preview the formula | Construct HTML containing the retained expression | `generated/formula.html`, matching `expected/formula.html` |

The query binds `m=http://www.w3.org/1998/Math/MathML` and selects expanded
element names. The binding belongs to execution; standalone XPath source
validation currently does not inherit caller namespace bindings, and standalone
XPath Parse/Inspect/format are not configurations here. The query reports:

```text
label=x squared plus one, divided by two
fractions=1
powers=1
variable=x
exponent=2
constant=1
denominator=2
```

## Save, build and follow provenance

In `data/formula.mml`, change `<mi>x</mi>` to `<mi>y</mi>` and update the accessible
label to describe the changed expression. Save and rerun the query and build.
The visible variable and query summary change. Editing the denominator or power
likewise changes authored structure; native source validation does not prove a
mathematical identity. Unsaved drafts leave persisted execution inputs unchanged.

`graphs/formula.cem` imports the presentation resource with its proper family
owner. `templates/formula.cemt` constructs a heading/caption and inserts the
retained `math` element and its children into HTML, omitting the XML declaration.
The entry artifact is explicitly `generated/formula.html`; Content MathML is not
an input to this build. No JavaScript reparses, draws or typesets the expression.

Select the project and choose **Build project** for the shared output preview.
The preview admits a bounded inert presentation vocabulary, including this
fraction and power. It does not admit arbitrary annotations, integration points,
external references, scripts or CSS. Browser layout and assistive-technology
support remain distinct from native source validation.

For source navigation, run **Build and preview the formula**, open **Commands and
results → Results → Provenance**, then select a **Source-map provenance** row.
The copied expression retains original native source boundaries where mapped;
the wrapper retains template origins. Constructed text and synthetic indentation
do not acquire arbitrary character-by-character input lineage.
`inspect-mathml` prints source coordinates but the current Parse/Inspect command
publication supplies no owned artifact-source references to the Provenance panel.
Use the build's actual output maps or native diagnostics for source navigation.

Native build, from this project directory:

```sh
cem-ml transform --config graphs/formula.cem --config-content-type application/cem \
  --config-schema https://cem.dev/ns/cli/transform-config/1 --scope-budget queueSize=1024
```

## Failure and repair

Run **Repair the MathML namespace** on
`failures/missing-namespace.mml`, which is separate from the graph inputs. Its
well-formed XML root lacks the MathML namespace. The native validator reports one
`cem.mathml.namespace_missing` at line 2, column 1, original byte offset 39.
Replace the opening `<math ` with
`<math xmlns="http://www.w3.org/1998/Math/MathML" `, save and rerun. The error
clears and the repaired bytes match the valid presentation source.

## Limits and independent offline projects

Native validation covers XML facts, MathML root/namespace, the selected media
profile and documented expression/policy checks. It does not validate every
MathML element, expression arity, operator, mathematical type or rendering rule.
An unsupported `profile` MIME parameter warns; the registered alias retains its
default profile. External annotation policy diagnostics retain their documented
warning severity without granting a resource resolver. This example uses no
annotations, external URLs or foreign content. See the
[MathML package](../../../packages/cem_ml/schema-packages/mathml/v1/README.md).

The native same-schema formatter preserves mathematical token text and sensitive
content while laying out structural elements. Generated whitespace is unmapped.
HTML preview is an explicitly authored presentation output, rather than an
automatic universal format conversion.

After runtime and source resources are cached, each saved copy can reopen and
rebuild offline. Its sources, templates, configurations and drafts remain
independent of the catalog original and other copies. There are no component
scripts, external fonts or example storage keys. Entered passwords and one-time
codes elsewhere block recovery until explicitly cleared; automatic whole-Studio
recovery remains disabled.
