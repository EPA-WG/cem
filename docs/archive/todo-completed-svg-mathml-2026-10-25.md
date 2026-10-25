# SVG and MathML projects, 2026-10-25

- [x] Add ordinary [SVG diagram](../../examples/projects/svg-diagram/README.md)
  and [MathML formula](../../examples/projects/mathml-formula/README.md) projects
  under Documents and UI and Queries and transformations. Include nine and ten
  source resources respectively, with six SVG and seven MathML configurations.
- [x] Demonstrate native inspection, validation, namespace-aware XPath queries,
  exact same-schema formatting, deliberate namespace failure/repair and an
  explicit HTML build entry. Keep Content MathML validation separate from the
  rendered Presentation MathML expression.
- [x] Add failing native import regressions before retaining the SVG/MathML
  family owners through the shared CEM tree. Preserve source bytes, namespaces,
  media profiles, original ranges, policy facts and XML import bounds.
- [x] Add failing shared artifact-preview stories before admitting bounded
  static SVG and Presentation MathML. Preserve geometry, text and accessibility
  names while refusing execution, integration escapes and new resource grants.
- [x] Add native authored-configuration and worker tests for exact artifacts,
  original diagnostics, repair, source provenance and independent copies.
- [x] Admit the three MathML text media identities to the existing saved-source
  policy with failing regressions first, preserving explicit command selection,
  valid UTF-8 admission and binary/invalid/oversized-source read-only bounds.
- [x] Verify deployed catalog navigation, native commands and repair, exact
  outputs, saved edits, visible geometry, provenance, failed-build retention,
  independent copies and offline reopening without clearing user data.
- [x] Regenerate the normal portable project archives and update the checked
  content-type inventory, retained-import contract and shared preview docs.

## Native source and output contracts

Exact SVG and all three registered MathML media identities resolve before the
generic `+xml` path. The shared import reuses XML semantic projection while
retaining the original family lifecycle owner. Dedicated validators reapply the
existing family facts without reparsing or weakening their severity. SVG hard
policy refusals and MathML profile/root/namespace checks remain intact; external
MathML annotation diagnostics retain their existing warning severity.

The default CEM AST inspector and XPath queries now see the real namespaced
tree, original URIs and byte positions. Explicit JSON inspection remains the
existing compatibility projection. The native adapters preserve existing XML
byte, event, node, depth and string limits, and release their source owner when
the final retained tree is disposed.

Each query compares seven semantic values. The SVG source includes accessible
names, literal geometry and `currentColor`; the Presentation MathML source uses
a fraction and a superscript. The separate Content MathML resource demonstrates
source validation without implying algebra evaluation or Content-to-Presentation
conversion. Neither native family validator claims complete vocabulary,
geometry, expression-arity or rendering conformance.

The native graphs apply explicit CEMT templates to retained SVG/MathML roots,
construct HTML wrappers and omit the source XML declaration. The outputs are
`generated/diagram.html` (1,467 bytes) and `generated/formula.html` (735 bytes).
Same-schema pretty outputs are 1,011 and 375 bytes respectively. Exact expected
HTML retains the native writer's final three spaces, with narrow Git whitespace
exceptions for those two fixtures only. This is authored template processing,
not a universal foreign-format-to-HTML converter.

Both deliberate failures omit the required root namespace and report one error,
zero warnings at line 2, column 1, original UTF-8 byte offset 39. Repairing the
same saved resource restores the valid primary source exactly. Invalid graph
inputs cannot publish a new artifact. Native provenance keeps partial original
source and template boundaries, including after Unicode edits, without claiming
arbitrary computed character lineage.

## Shared artifact preview

The generic implementation belongs in `cem-elements`; Studio and the examples
add no UI JavaScript. The viewer uses its inert output DOM as the rendering
boundary and preserves the admitted foreign namespaces. It is not a source
parser or substitute for native import and validation.

The SVG subset includes basic shapes, paths, groups and text. The MathML subset
includes presentation tokens, rows, fractions, roots, scripts and over/under
layouts. Attribute allowlists preserve accessibility names, static geometry and
literal colors. Scripts, handlers, animation, arbitrary styles, URL attributes,
paint servers, unknown foreign nodes and annotations remain refused. Mixed
namespace children and integration-point element children cannot escape into
the HTML asset policy. The existing script-denied, network-denied sandbox/CSP,
relative HTML/CSS/image handling, cleanup and checkpoint contracts remain intact.

Native validation and browser preview admission are separate policies. A source
accepted by a native family validator does not automatically grant its entire
vocabulary or any resource access to the preview.

Deployed acceptance also found that Studio's existing text-resource service
omitted MathML media types, leaving saved formula sources read-only. It now
admits the three exact registered MathML identities after media-essence
normalization. Existing UTF-8, local source-kind and source-size checks remain
in force; selecting a file alone still does not infer an executable command.
This changes resource loading, not application UI composition.

## Verification

The native retained-import, XHTML compatibility, common XPath and authored
project cohorts pass **26 tests**:

```sh
cargo test --locked -p cem-ml --test svg_mathml_import --test xhtml_import \
  --test xpath_cem_tree --target-dir dist/target/cem_ml_cli
cargo test -p cem-ml-cli --test svg_mathml_projects \
  --target-dir dist/target/cem_ml_cli
```

Six initial retained-import cases fail before the adapter repair. The final
seven cases also prove all four media identities in default CEM inspection.
Three CLI cases cover every authored configuration, exact outputs, original
diagnostics and repair, refusal before publication, partial original provenance,
Unicode edits and independent builds.

The focused saved-resource workbench cohort passes **10 browser cases**: three
MathML aliases with mixed-case/parameterized media types exercise UTF-8 editing,
save and explicit native Inspect execution; seven cases preserve invalid UTF-8,
oversized sources and binary media as exact read-only downloads. The three
admission cases fail before the service-policy fix. Authored lint passes.

The shared artifact browser suite passes **5 cases**, including both new foreign
content cases that fail before preview admission is implemented. Existing
relative navigation, CSS/image loading, cleanup and checkpoint behavior passes.
The adjacent shared live-preview lifecycle/checkpoint suite passes **8 cases**.
Focused shared lint has zero errors; the two new stories introduce no warnings.

The rebuilt native CLI, command-service WASM, rendering WASM and runtime packages
pass their builds. All **29 Studio Feature Tour browser tests pass**, including
the new native family commands and graph workflows. Studio and shared runtime
typechecks pass using the exact native-generated CLI schema input. The known
standalone preparation gap remains a separate TODO; the temporary input is
removed after verification.

Authored Studio `src`/`scripts` lint passes. The broad package lint still reports
the already-recorded 238 errors and five warnings in generated schema-asset
WASM glue; its scope repair remains in the native/release follow-ups. No authored
lint rule is relaxed and generated glue is not edited.

Native Feature Tour verification passes all **32 schema examples** and the
authored project configurations. The checked inventory contains **69 workflow
mappings**. Output verification passes 218 declared assets and 319 files;
deterministic builds match SHA-256
`5571bc02cd588743f43e5b495476f480a09e1f7128fc01a739c9cf2bbb30de0e`.

The dedicated `@epa-wg/cem-studio:verify:svg-mathml` gate passes with all 25
prerequisites. All **14 evidence flags for each project** are true in
`dist/reports/cem-studio/svg-mathml-projects.json`: catalog/read-only originals,
native commands, exact formatting, namespace queries, repair, explicit static
entry, foreign namespace, visible geometry, native provenance, unsaved-source
guard, last-good output after a failed build, independent copies and offline use.
The saved-copy tests change source geometry/tokens and template captions through
the shared editor and controls; original catalog sources remain unchanged.

## Preserved policies and next work

Acceptance uses isolated browser storage and temporary project copies. Existing
saved projects and drafts are not cleared. Original catalog resources remain
read-only and unchanged. Generated screenshots, Vitest attachments, reports and
binaries remain outside Git.

Entered passwords and one-time codes still block recovery until explicitly
cleared. Automatic whole-Studio recovery remains disabled. No theme dimensions,
component registration authority or credential policy changes are introduced.

Continue with CSS/SCSS/selectors, CEM-QL/XPath data workflows, then CEM
transformation/XSLT projects in the active TODO.
