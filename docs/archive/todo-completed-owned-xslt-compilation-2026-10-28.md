# Original-owned XSLT compilation — 2026-10-28

## Contracts

- [x] Reproduce source-loading/compiler grammar disagreement with a native fixture.
- [x] Carry the immutable native parser contract through shared adapter compilation,
  import discovery, host-name resolution and every resolved stylesheet module.
- [x] Reconcile the stale native bootstrap pin with the trusted producer while
  retaining strict runtime/pack equality and registry validation.
- [x] Make the verifier's malformed-CSV rejection probe request the existing
  strict fail level; preserve its exact native diagnostic assertion.
- [x] Verify complete/external native cohorts, physical-runtime WASM package
  replay, Studio optional graph execution online/offline and deployed demos.
- [x] Run final isolated Login recovery before commit/push, preserving credential
  refusal and all existing budgets/deadlines.

## Cause and repair

Native loading and engine XSLT preflight already used the retained original schema
model. The CEM-QL bundle compiler, namespace/name resolver and source-import API
reparsed with the contextless built-in catalog. The new engine fixture reproduces
this before the repair: an original schema treats an attribute as literal, then
compilation incorrectly applies the built-in AVT grammar and reports a fatal
unclosed-expression error. The same fallback prevented optional-profile execution
after original source admission.

The shared template-adapter trait now offers `compile_with_context`, defaulting
to the existing compile method for other adapters. The XSLT adapter selects the
current model's ready original-owned contract and passes it to the compiler's
explicit native entry points. The parser keeps its compiled grammar/diagnostic
snapshot and original source owner; later mutable inspection declarations cannot
replace that snapshot. No schema is reconstructed from JSON or parser ASTs.

The existing WASM bundle/component/import ABI selects the same contract from its
physical runtime package owner. Parser authority is absent from scalar component
options and portable bundles. Missing/incomplete optional packages and inactive
native models refuse new compilation; complete-profile contextless defaults are
preserved. Public compilation functions without a supplied contract still refuse
missing optional originals.

Discovery defers only the native external-URI fact while exact import edges are
resolved. Classification uses its native fact identity, so an original schema's
diagnostic-code rename does not accidentally block valid module preflight. Every
other hard diagnostic blocks discovery. Full compilation revalidates exact module
edges and native diagnostics; document I/O is not enabled.

Original stylesheet URI/hash/byte-length manifests, dependency edges, XSLT-owned
XPath source ranges and native typed programs survive portable bundle reload.
Generated CEMT remains a separately identified source for generated literals.
Source/closure, module count/hash, parameter and execution limits remain intact.

The first optional deployment rebuild correctly refused a runtime/pack pin
mismatch. The last pin update was `eca54d20`; seven later schema/formatter source
changes account for the exact +3,152 emitted bytes: AST/events projection schemas,
the generic CEM-ML schema, CSS-selector and XPath formatters, and Markdown schema/
formatter. No catalog sources change in this repair. The trusted native producer
confirms the 32-package registry is unchanged. The bootstrap now pins digest
`3d473c3297afecdc96fe4133faad8bd7d8149124a60465f5a5b333ef8c39a749`,
69,075 index bytes, 200 sources and 1,347,296 source bytes. Runtime/pack equality
and source-digest admission checks remain strict; no downloaded sidecar grants
authority.

The optional verifier also assumed default CSV Parse selected a failing exit for
an unclosed quote. Native CLI probes show default `--fail-level parse` reports
the original diagnostic with exit 0 and no AST artifact, while `--fail-level
strict` returns exit 1 with the same diagnostic and no artifact. The rejection
fixture now explicitly requests strict policy. Valid default Parse coverage and
the exact unclosed-quote assertion remain. The success-without-publication
behavior masks that native diagnostic in Studio and remains a separate TODO;
native exit policy and shared output guards are unchanged by this repair.

## Verification

- The original native engine fixture fails before editing with
  `cem.xslt.avt_unclosed_expression`; after the repair it produces the intended
  HTML using the admitted original grammar.
- Complete native XSLT bundle/component/lowering/matching/output/view/focus/parser
  cohort: 58/58 pass.
- Existing native adapter migration, XPath-call and focus/reload cohort: 7/7 pass.
- Original parser/import/name/diagnostic/provenance/limit fixtures: 3/3 pass in
  complete and external-source profiles. Actual package-admission engine fixtures
  verify a fresh original owner on independent contexts and an imported module
  whose external-URI diagnostic has an authored name.
- Engine actual-admission/import/fresh-context cases: 2/2 pass in each profile.
- Complete Node/browser common ABI tests: 28/28 pass; all 13 public CEM-QL WASM
  scripts and default distribution integrity checks pass. The complete-profile
  XSLT control/import/native-output WASM regression passes after correcting the
  test's component handle field to the existing `artifactId` ABI.
- Complete and optional Studio builds pass after refreshing the producer-backed
  bootstrap. The default runtime bytes are unchanged from the verified API cohort
  across that rebuild (CEM-QL 23,959,193 bytes; CEM-ML 24,639,934 bytes).
- Fresh physical WASM acceptance passes: default controls/imports/native output,
  plus first/fresh optional memories with missing/incomplete/disposed-authority
  refusal, original-source admission/replay, exact module hashes and namespaced
  parameter controls. Native pack verification checks all 200 originals, 205
  offline files and independent WASM reimports.
- Default Studio XSLT configurations/builds and cross-query Login semantic/source
  provenance browser cases: 3/3 pass. Studio package lint and the new WASM test's
  authored script lint pass.
- The new `@epa-wg/cem-ml:test:xslt-parser-contracts` target passes with its
  default/optional build dependencies cached and actual behavioral cases run.
- Full optional Studio verifier passes: native XSLT graph output online and
  disconnected (`nativeXsltGraph: true` in both reports), normal parsing and
  formatter/colorizer demand for Markdown/SCSS/XSLT/Relax NG, selected-resource
  admission, both factory pins, all 205 pack files, bidirectional cache isolation,
  cold offline command execution, source replay across invocation, command worker,
  rendering worker and fallback handoffs, and credential/stale/retirement refusal.
  The final successful run executes the target's exact verifier command directly
  after correcting its strict-policy rejection fixture; the earlier wrapper run
  exposed that obsolete assumption.
- Deployed acceptance passes: 898 managed/total files and all hashes, 333 Studio
  files, paired runtime bytes, shared initialization/isolated memories, native
  render/highlighting, Studio URL selection/saved copies/previews/offline builds,
  all ten unchanged theme generators, 120 canonical playground pages and the live
  tree gallery. All four default deployed WASM files remain below 25 MiB.
- Final isolated Login recovery passes 2/2 worker/fallback cases: three recoveries,
  twelve edits and two credential refusals per runtime, with complete workspace
  checkpoints inside the unchanged 4 MiB group budget. Existing allocation/resume
  deadlines and disabled automatic recovery remain intact.

Studio remains declarative; no application/component parser or JavaScript UI
behavior is introduced. Entered passwords/one-time codes still block recovery
until explicitly cleared, automatic recovery remains disabled, and the 4 MiB
group budget, 32,768-unit witness bound and allocation/resume deadlines remain.
Generated screenshots and Vitest attachments remain excluded from Git.
