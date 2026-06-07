# AC-P-6 Promotion — NVDL-style namespace dispatch (draft)

Status: Draft proposal for [`cem-ml-ac.md`](cem-ml-ac.md) · Resolves: [`todo.md`](todo.md) OQ-3
residual · BRD: [`content-type-switch.md`](content-type-switch.md) §6.8 (BR-SW-2)

## 1. Context and goal

[`cem-ml-ac.md`](cem-ml-ac.md) currently carries AC-P-6 as a single Tier-C placeholder:

> **AC-P-6 [C] MUST** dispatch between schemas mid-document via NVDL-style rules (see References).

The BRD [`content-type-switch.md`](content-type-switch.md) makes the **interior selection**
layer (BR-SW-2) the load-bearing mechanism: inside a CEM-ML region the content type/schema is
selected **directly** (an explicit `cem:schema` form, already normative under AC-F-2) or
**indirectly**, resolved from **namespace metadata**. The indirect path is exactly NVDL-style
dispatch, and it is the part AC-P-6 leaves undetailed. OQ-3's residual is to promote/detail
AC-P-6 and add acceptance criteria for an embedded `xsl:`-style content type.

This draft:

1. replaces the AC-P-6 one-liner with a structured, normative set (AC-P-6.1–AC-P-6.7);
2. adds an embedded XSLT content-type worked case (AC-P-6.8–AC-P-6.9);
3. proposes a **tier promotion**: carve a static, locally-resolved dispatch **core** down to
   Tier B, keeping plugin-invoking / externally-loaded full NVDL at Tier C;
4. specifies verification fixtures and the matching G-NVDL (§16.4) updates.

The prerequisites are already normative, which is what makes promotion feasible: AC-P-4
context scopes with identity `{ schemaUri, contentType, namespaceUri }`; AC-P-5 nesting; AC-P-7
source-map stacks; AC-F-2 explicit schema switching + the NVDL-composition rule; AC-F-4
parent-owned content-type handoffs; AC-I-2 content-type switching; AC-V-9..AC-V-13 schema
version identity (§3.1); AC-F-8 document-format version identity; AC-CC-1/AC-CC-3 cache and
policy stamps; Layer 5 `HandoffRecord { content_type, schema_id, source_span,
inherited_context, return_condition }`.

## 2. Proposed replacement for AC-P-6 (normative)

Replace the single AC-P-6 line with the following. Sub-criteria keep referenceable decimal IDs;
tier tags are per sub-criterion (see §3 for the promotion rationale).

**AC-P-6 [B/C] MUST** dispatch the active content type and schema for a region by namespace,
mid-document, via NVDL-style rules. Namespace-driven (indirect) selection composes with the
explicit AC-F-2 forms; where both apply on one boundary, NVDL applies first and the explicit
form layers within its scope (per the existing AC-F-2 *Composition with NVDL* rule).

- **AC-P-6.1 [B] Dispatch model — namespace metadata.** A resolved namespace identity MUST
  carry **namespace metadata** `{ contentType, schemaUri, schemaVersion }`, extending the AC-P-4
  scope identity. Selecting a content type/schema for a region is either:
  - **direct** — an explicit AC-F-2 `cem:schema` declaration/switch/attribute form; or
  - **indirect** — derived from the namespace metadata of the region's active namespace
    binding (the binding supplies content type, schema, and version without a separate schema
    declaration). The indirect path is the NVDL dispatch governed by this AC.
  A namespace with no resolvable metadata and no explicit form is governed by AC-P-6.7.
  The source of namespace metadata, its trust boundary, and its cache identity are the open
  decision D-4.

- **AC-P-6.2 [B] Two-layer boundary.** AC-P-6 governs **interior** dispatch only. The
  host-surface ingestion boundary — `<template>` / `<script>` `lang`/`type` on an HTML host —
  is **not** AC-P-6; it is the AC-F-4 / AC-I-2 host handoff from the HTML content type into a
  CEM-ML content type, owned by the HTML parser and the cem-element runtime. AC-P-6 begins once
  a region is inside the CEM-ML model. The two layers MUST compose: a host-ingested CEM-ML
  region MAY contain interior dispatched regions of other content types.

- **AC-P-6.3 [B core / C full] Rule form and modes.** Dispatch rules MUST be scoped and
  resolved innermost-first (consistent with AC-F-2 identifier resolution and AC-P-10 rebinding).
  Tier B MUST support the **static modes**: *attach* (validate/interpret the region under the
  dispatched schema/content type), *allow* (accept without validation as inert foreign content
  unless the parent schema explicitly defines pass-through/rendering semantics), and *reject*
  (diagnose and refuse). Tier C adds the **dynamic modes**: *unwrap*/*cascade* (re-dispatch nested
  namespaces) and plugin-invoking attach (a dispatched schema that owns an AC-T-4 transform or
  AC-PL plugin chain).

- **AC-P-6.4 [B] Scope identity and isolation.** Each dispatched region MUST open an AC-P-4
  context scope with its own `{ schemaUri, contentType, namespaceUri }`, nest per AC-P-5, and be
  **isolated**: tokens/constructs of one dispatched namespace MUST NOT be interpreted by another
  region's content type or schema, and a child scope's diagnostics relax/hide only within parent
  override bounds. (Realizes BRD BR-NS-3 / BR-CO-4.)

- **AC-P-6.5 [B] Per-namespace version resolution.** The schema dispatched for a namespace MUST
  resolve its embedded SemVer per AC-V-9..AC-V-13. A version segment in the namespace URI (for
  example the `/1` tail of `https://cem.dev/ns/core/1`) is a **MAJOR constraint** resolved with
  the AC-F-8 / AC-V-2 / AC-V-3 model: same-MAJOR equal-or-higher MINOR/PATCH loads (forgiving);
  an unsupported MAJOR aborts or routes to a legacy/compat handler (strict). Each dispatched
  namespace versions on its own axis; a MAJOR change to one MUST NOT force a change to another.
  (Realizes BRD BR-VC-5/6 at the namespace axis.)
  For external standards that do not publish SemVer-compatible identities, dispatch MUST use a
  CEM-owned adapter SemVer line or a documented native-version mapping; this is the open
  decision D-6.

- **AC-P-6.6 [B] Source-map and handoff continuity.** A dispatched region MUST be modeled as a
  Layer-5 parent-owned handoff `HandoffRecord { content_type, schema_id, source_span,
  inherited_context, return_condition }`: the child parser MUST NOT consume past the
  parent-owned return condition, and the host content type MUST resume on return. Source-map
  stacks MUST span the dispatch boundary per AC-P-7 (origin-first, byte-range identity).

- **AC-P-6.7 [B] Diagnostics and unknown-namespace policy.** Diagnostics MUST originate in the
  dispatched scope and bubble to the nearest schema-declared or context-root boundary per
  AC-P-4. When a region's namespace resolves to no metadata, no explicit schema, and no rule,
  the effective scope policy MUST select one **defined** behavior — `reject`, `allow`
  (unvalidated foreign content), or `ignore` (drop with a report event) — with a documented
  default; the outcome MUST be deterministic. (This is the namespace-axis instance of BRD
  BR-VC-3 / OQ-6.) Dispatched schema sets participate in the AC-CC-1 cache hash and AC-CC-3
  policy stamp; a host missing a dispatched schema MUST fail with `cem.cc.policy_mismatch`.
  `allow` and `ignore` are non-execution modes unless a separate handler is explicitly selected.

## 3. Worked content type: embedded XSLT (`xsl:`)

XSLT is the first concrete foreign/legacy content type dispatched interiorly, and the interior
counterpart to the host-layer `<template lang="custom-element-v0">` bridge.

- **AC-P-6.8 [B] XSLT region dispatch and isolation.** The namespace
  `http://www.w3.org/1999/XSL/Transform` (conventionally `xsl:`) MUST be dispatchable as an
  embedded content type per AC-P-6.1–AC-P-6.7. The CEM-ML parser MUST treat the dispatched
  `xsl:` subtree as foreign content: it opens a Layer-5 handoff, does **not** interpret XSLT
  constructs as CEM-ML, and the surrounding CEM-ML content type resumes on return. The `xsl:`
  content type carries its **own** version, pinned independently; a MAJOR bump of the CEM-ML
  core MUST leave the dispatched `xsl:` region's expanded names and version unchanged. (Realizes
  BRD BR-CO-2 / BR-CO-4.) XSLT dispatch is explicit opt-in: the host must provide namespace
  metadata or an effective scope-policy rule for the XSLT namespace. A schema for validation MAY
  be attached (the RELAX-NG schema for XSLT in References); if no validation schema is attached,
  the region may be accepted only under the effective AC-P-6.7 `allow` policy and remains inert
  unless AC-P-6.9 selects an execution handler. The source of the XSLT compatibility version
  (`xsl:stylesheet/@version`, namespace URI, or CEM-owned adapter version) is D-6.

- **AC-P-6.9 [C / decision] XSLT execution binding.** *Executing* the dispatched XSLT (running
  the transform) is out of scope for the CEM-ML parser and MUST be delegated by scope policy to
  an explicit handler: a real XSLT processor, the `custom-element-v0` legacy bridge, or a
  documented non-goal that diagnoses unsupported execution. The choice is the open decision in
  §6 and is bounded by [`custom-element-template-migration-options.md`](custom-element-template-migration-options.md)
  (Option A vs B). Dispatch + isolation + version-pinning (AC-P-6.8) do **not** depend on this
  decision.

## 4. Tier and gate placement — the promotion

Today AC-P-6 is wholly `[C]` under **G-NVDL** (§16.4), which depends on G-PLUG and G-EXT.
That coupling is only required for the **dynamic** path (plugin-invoking dispatch, externally
loaded schemas). The **static core** — namespace→`{ contentType, schemaUri, schemaVersion }`
dispatch over **locally available** (embedded or same-document) schemas, with attach/allow/
reject modes — needs none of it.

**Proposal:** stage G-NVDL into two entries.

- **G-NVDL-CORE [Tier B]** — opens AC-P-6.1, .2, .4, .5, .6, .7, the static modes of .3, and
  the dispatch/isolation/version of .8. Prerequisites: AC-P-4, AC-P-5, AC-P-7 (Tier A);
  AC-F-4 generalized parent-owned foreign-content handoff (promote the Tier-A HTML-only handoff
  to a general local handoff at Tier B); AC-I-2 content-type switching (Tier B); AC-V-9..V-13,
  AC-F-8 (version identity); AC-CC-1/AC-CC-3 (cache/policy stamp). **Does not depend on G-PLUG
  or G-EXT** while schemas are local. This is the slice the BRD's interior switching needs.
- **G-NVDL-FULL [Tier C]** — the existing G-NVDL: dynamic modes of AC-P-6.3 (unwrap/cascade,
  plugin-invoking attach), AC-P-6.9 XSLT execution, externally loaded dispatched schemas.
  Keeps the existing G-PLUG / G-EXT dependencies.

This re-tiering changes the §16.1 gate dependency graph and therefore requires maintainer
sign-off. **Conservative fallback:** if re-tiering is rejected, keep all of AC-P-6 at Tier C
under the single G-NVDL but adopt the AC-P-6.1–6.9 detailing as-is (detail-only, no tier
change) — the OQ-3 architecture is still resolved by the BRD §6.8 two-layer model regardless.

## 5. Verification (proposed AC-P-V additions)

- **AC-P-V-2 — indirect dispatch from namespace metadata.** A fixture declares a namespace whose
  metadata binds a content type + schema, emits a region in that namespace with **no** explicit
  `cem:schema` form, and the parser attaches the correct schema and content type, with source-map
  frames spanning the boundary per AC-P-7.
- **AC-P-V-3 — isolation.** A document interleaves two dispatched namespaces; constructs valid in
  one are inert/foreign in the other; neither parser interprets the other's tokens; diagnostics
  attach to the originating scope per AC-P-4.
- **AC-P-V-4 — embedded XSLT version-pinning.** A document embeds an `xsl:` region inside CEM-ML;
  bumping the CEM-ML core MAJOR leaves the `xsl:` region's expanded names and resolved version
  unchanged, and the CEM-ML parser emits no XSLT-construct interpretation. (Guards BR-CO-2.)
- **AC-P-V-5 — per-namespace version negotiation.** Forgiving load (same MAJOR, higher MINOR) and
  strict reject (unsupported MAJOR → version diagnostic) both observed per dispatched namespace.
- **AC-P-V-6 — unknown-namespace policy determinism.** The same unresolved-namespace region
  yields `reject` / `allow` / `ignore` strictly per the effective scope policy, with the
  documented default when unset.
- **AC-P-V-7 — legacy XSLT explicit opt-in.** A fixture containing an `xsl:` subtree without
  namespace metadata or an explicit scope-policy rule follows the AC-P-6.7 unknown-namespace
  default; adding an explicit XSLT dispatch rule opens an isolated XSLT handoff without CEM-ML
  interpretation or execution.
- **AC-P-V-8 — direct/indirect conflict.** A fixture where namespace metadata dispatches one
  schema/content type and an explicit `cem:schema` form requests an incompatible one produces
  the documented D-5 outcome with a deterministic diagnostic or allowed layering behavior.
- Existing G-NVDL entry/exit fixtures (two-then-three namespace dispatches; AC-CC-1 reuse across
  hosts; `cem.cc.policy_mismatch` on a missing dispatched schema) remain and now map to
  AC-P-6.1–6.7.

## 6. G-NVDL (§16.4) update

Update the gate entry to: split into G-NVDL-CORE (Tier B) and G-NVDL-FULL (Tier C) per §4 above;
list **Gated ACs** as AC-P-6.1–6.9 (CORE: .1,.2,.4,.5,.6,.7, static .3, .8; FULL: dynamic .3,
.9, external-schema dispatch); keep the existing prerequisite list for FULL, and the reduced
prerequisite list (no G-PLUG/G-EXT) for CORE; keep the entry/exit fixtures, adding AC-P-V-2..V-8.

## 7. Open decisions

- **D-1 (re-tiering).** Accept the G-NVDL-CORE Tier-B split, or keep AC-P-6 detail-only at Tier C?
- **D-2 (unknown-namespace default).** Default of AC-P-6.7 — `reject` (safe, strict) vs `allow`
  (tolerant). This is the namespace-axis half of OQ-6; the per-feature ignore-vs-degrade default
  should be decided consistently here.
- **D-3 (XSLT execution, AC-P-6.9).** Real XSLT processor, `custom-element-v0` bridge, or
  documented non-goal — bounded by the Option A/B migration decision.
- **D-4 (namespace metadata authority).** Decide whether namespace metadata is supplied by inline
  schema descriptors, a local registry, package manifests, an external registry, or a composed
  lookup chain; define trust/resource-policy checks, offline pinning, and AC-CC-1/AC-CC-3
  cache/policy identity inputs.
- **D-5 (direct-vs-indirect conflict policy).** AC-F-2 says namespace dispatch applies first and
  explicit forms layer within its scope; decide whether an explicit form may change content type
  inside an indirectly dispatched namespace, or whether incompatible direct/indirect selections
  diagnose and reject.
- **D-6 (external standard version mapping).** Decide how XSLT/native external-standard versions
  map to the platform's SemVer axes: `xsl:stylesheet/@version`, namespace URI, a CEM-owned XSLT
  adapter version, or a pair of native-version constraint plus adapter SemVer.

## 8. References

- NVDL — Namespace-based Validation Dispatching Language (ISO/IEC 19757-4):
  <https://en.wikipedia.org/wiki/Namespace-based_Validation_Dispatching_Language>,
  <https://nvdl.oxygenxml.com/> (already in `cem-ml-ac.md` §References).
- RELAX-NG schema for XSLT: <https://qt4cg.org/specifications/xslt-40/schema-for-xslt40.rnc>
  (already referenced) — candidate validation schema for the AC-P-6.8 `xsl:` content type.
- [`cem-ml-ac.md`](cem-ml-ac.md) — AC-P-4/5/7/10, AC-F-2/F-4/F-8, AC-I-2, AC-V-9..V-13,
  AC-CC-1/CC-3, §16.4 G-NVDL, Layer 5 handoff.
- [`content-type-switch.md`](content-type-switch.md) — BRD (BR-SW-2, BR-NS-2, BR-VC-5/6, BR-CO-2/4).
- [`custom-element-template-migration-options.md`](custom-element-template-migration-options.md)
  — XSLT Option A/B, bounds AC-P-6.9.
