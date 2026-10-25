# Catalog README and declared capability audit, 2026-10-25

- [x] Give each project a README with typical uses, exact content type/schema
  identities and configuration IDs, expected artifacts, provenance boundaries
  and deliberate failures/repairs where applicable.
- [x] Review every authored and generated catalog README against its resources,
  native fixtures and known limits without changing saved project identities.
- [x] Add deterministic documentation/capability checks and negative fixtures
  for missing documentation, dangling references and unavailable operations.
  Separate registered selectors, declared inputs, dependencies and expected
  reference artifacts from actual execution evidence.
- [x] Regenerate catalog bundles and verify README integrity, native command
  fixtures, worker transport, deployed readonly/saved/offline access and draft
  retention. Preserve shared declarative controls and credential recovery refusal.

## Instructions and coverage

All **22 authored project READMEs** now name their real configuration IDs and
non-documentation MIME/schema identities. The instructions distinguish source
syntax checks, schema-instance validation, query execution, graph builds and
explicit exports. Expected fixtures do not become admitted source formats.
Repairs preserve original sources, native diagnostics and actual location limits.

[Schema examples](../../examples/projects/schema-examples.md) and the legacy
Feature Tour seed share a new ordinary Markdown resource. Both retain their
existing configuration/source identities and now contain **80 resources**;
the five portable workbenches still contribute **14 resources**. The
[starter](../../examples/projects/starter.md) adds documentation after `source`
without adding configurations or changing its `html-v1:source` preview choice.
Repository exports sort resources by path; this order is distinct from the
authored starter order and default preview selection.

The existing native build graph deploys the README asset. Catalog metadata pins
its source hash, resource descriptor and cache URL; the shared Feature Tour
loader verifies descriptor agreement and SHA-256 before publishing owned bytes.
The starter and Schema examples carry documentation in their normal archives.
No documentation screen, source parser or new UI behavior is introduced.

The [generated coverage document](../studio-catalog-coverage.md) and ignored JSON
report account for **282 authored source resources**, **203 authored
configurations** and **208 documented workflows**, including five generated
portable workbenches. Seventeen previously missing inventory rows are documented.
The selectable catalog has **23 projects**, **240 declared configurations** and
**565 stored resources**; the starter adds two resources and no configurations.
The legacy seed is not counted twice.

The native registry supplies **32 packages** and **69 package-qualified selector
claims** (63 distinct strings). Primary input declarations represent 40 of those
claims; the remaining 29 are listed as gaps. Some are output/compiled formats or
deliberately refused entity/DTD inputs. This is neither an activation allowlist
nor a promise of uniform conformance or UI execution. `run-check` uses
`validate --config`; native `check` and capability negotiation remain separate.

## Deterministic audit and native limits

`verify:catalog-audit` checks named manifest/bundle/documentation metadata,
unique identities, exact inline configuration/MIME/schema tokens, source file
existence, primary/dependency/expected-fixture references, generated command
coverage and complete workflow inventory tuples. README bytes and hashes must
match the authored sources and generated archives. Registered selector gaps and
control/domain schemas outside the registry are reported without inventing a
second source registry. Native command/data parsing stays in Rust.

The **23 audit tests** include missing/empty/wrong-identity documentation,
misleading ID/MIME/schema suffixes, missing files, dangling inputs/dependencies,
binary/graph goldens, unavailable operations, stale/missing/duplicate workflows
and duplicate generated command coverage. Positive cases preserve explicit MIME
parameters, generic MIME with an explicit schema, control schemas outside the
registry and the distinction between entry kind and argv command.

Known limits remain explicit: Login's older generic CEM identity can report hard
runtime-binding diagnostics despite exit zero; Parse/Inspect coordinate output
does not supply owned artifact-source navigation; AST debug JSON does not yet
round-trip through its registered source shape; projection input adapters remain
output-only and binary validators check headers. Binary Studio resources remain
readonly and non-runnable with exact-byte downloads. These native follow-ups are
not closed by documentation or by declared configuration counts.

## Verification

- The deterministic audit passes for all 22 authored projects, 203 authored
  configurations, 208 inventory rows and 32 native package records.
- All **55 Feature Tour browser tests** pass, including generated README
  loading, corrupt-byte/metadata refusal and the starter's actual default preview.
  Existing native worker command/repair coverage remains intact.
- The native Feature Tour verifier passes its authored-project checks, five
  portable workbenches and **32 generated schema-package validations**. Login
  retains its existing separate browser cohorts and generic-source limits;
  these results are not a clean execution claim for every declared configuration.
- Deployed documentation acceptance passes all **23 original README views**
  online and offline, exact bytes/hashes/identities, readonly controls, URL/tree
  selection and matching window/page titles. Create retains the two-resource
  starter and its source preview. Two saved Schema examples copies have separate
  roots and revisions; saved edits and an unsaved Unicode README draft survive
  catalog browsing and offline reload without changing originals or saved bytes.
  Verification uses isolated browser storage, not the user's session.
- The existing empty/entered-credential checkpoint regression passes on worker
  and fallback runtimes. Studio type checking and focused authored-source lint
  pass. The output verifier checks the real deployed README and its cache/hash
  metadata, with **218 declared assets / 333 files**.
- Two clean static builds match with aggregate SHA-256
  `389a931aabbbed454d60539f4171101a366515514ce2cb3395c368115308192a`.
  The final file inventory and every byte hash match that report. Deployed
  verification runs the configured Node script directly for live progress,
  without changing its assertions or timeouts.

UI remains declarative in the existing shared components. No native grammar,
theme or component behavior changes are required. Entered passwords and one-time
codes continue to block recovery until explicitly cleared; automatic
whole-Studio recovery remains disabled. Saved projects/drafts are not cleared.
Generated screenshots, Vitest attachments, reports and build outputs stay out of
Git. Full catalog configuration/navigation/clone/build/preview/offline acceptance
and the listed native functional gaps remain active TODOs.
