# Evolutionary Architecture of the Authoring & Rendering Model — Business Requirements

Status: Draft · Type: Business Requirements Document (BRD)

This document states **what** the platform must allow as its authoring and rendering model
evolves over time, and **why**. The goal is an **evolutionary architecture**: one that
supports *guided, incremental change across multiple dimensions* so that both the **surface
syntax** and the **underlying logic and model** can evolve without big-bang rewrites and
without breaking existing content (Ford, Parsons, Kua & Sadalage, *Building Evolutionary
Architectures*).

Content-type, syntax, and namespace switching — together with versioning — are the **seams**
that make this evolution possible. They are the mechanism, not the goal. Legacy XSLT
coexistence is the **first, current-focus use case** of that mechanism, not the end state.

This is a business/architecture-level document. It does not prescribe data structures, parser
internals, dispatch algorithms, or authoring syntax; those belong in the design and
acceptance-criteria documents under *Related documents and references*.

## 1. Purpose

The platform must be able to grow: new authoring syntaxes, new processing logic, and new
content models will be introduced over years, while documents authored against older
generations keep working. This BRD defines the business requirements for evolving the model
in a controlled, observable, reversible way, and for letting multiple generations coexist
during transitions.

## 2. Background and problem statement

Authoring/rendering models tend to ossify: a single syntax and a single processing engine get
locked in, so change becomes a high-risk rewrite. Three forces make that unacceptable here:

- **The model will change on multiple, independent axes.** The *surface syntax*, the
  *underlying logic/semantics*, and the *content/data model* do not move in lockstep. A change
  to one must not force a change to the others.
- **Older content must keep working.** Documents authored against an earlier generation must
  continue to render while newer generations are adopted incrementally.
- **Legacy models must coexist and then retire on schedule.** Existing legacy XSLT templates
  are the immediate example: they must run alongside new CEM-ML content, isolated from it, and
  be retired deliberately rather than abandoned or frozen forever.

Without explicit evolutionary requirements, mixing generations risks misinterpretation,
breakage on upgrade, permanent forks of the engine, and an inability to measure whether a
change preserved the properties that matter.

## 3. Definitions

Business/architecture definitions, not technical specifications.

Evolutionary-architecture concepts:

- **Evolutionary architecture** — an architecture that supports *guided, incremental change*
  as a first principle, across multiple dimensions.
- **Guided change / fitness function** — an objective, ideally automatable measure of how well
  the system exhibits a desired characteristic (for example "older content still renders").
  Fitness functions guide and protect evolution.
- **Incremental change** — change delivered in small steps that can be verified and shipped
  independently, rather than as a single large migration.
- **Appropriate coupling** — keeping the parts that must evolve independently loosely coupled,
  so a change in one does not ripple into others. The seams below are the coupling boundaries.
- **Parallel change (expand → migrate → contract)** — evolving a contract by first adding the
  new form alongside the old, migrating consumers, then removing the old form, so no single
  step breaks a consumer.

The seams (the evolution axes):

- **Content type** — the *kind* of content a region holds (for example CEM-ML, HTML, SVG, CSS,
  JavaScript, an ES module, or XSLT). It determines how a region is interpreted and what it is
  capable of. Roughly the "model + logic" axis.
- **Syntax** — the *surface form* the content is written in. One content type may be
  expressible in more than one syntax. A syntax is governed by a schema. The "syntax" axis.
- **Namespace** — a named, stable identity that scopes a region and binds it to a content type,
  its syntax, and its version; it also provides prefixes, a default, and aliases.
- **Version** — an independent, comparable identifier for a content type, syntax, or model,
  used to decide compatibility between a document and a processor.
- **Region / subtree** — a bounded portion of a document to which a content type, syntax, and
  version apply; regions may be nested and may differ from their neighbors.

## 4. Objectives

- Let the surface syntax, the underlying logic, and the content model evolve **independently**.
- Let multiple **generations/versions coexist** in one document or system during a transition.
- Make change **guided and observable** through objective fitness functions.
- Keep change **incremental and reversible**, never a big-bang rewrite or a one-way door.
- Provide a controlled, opt-in path for **legacy coexistence and retirement** (current focus:
  XSLT).

## 5. Scope

**In scope**

- Independent evolution of syntax, logic, and model, and the coexistence of their versions.
- Declarative, author-controlled switching of the active content type/syntax for a region.
- Compatibility expectations between documents and processors across versions.
- Fitness functions that guard the evolutionary properties.
- Non-conflicting coexistence and scheduled retirement of legacy models (XSLT first).

**Out of scope**

- The internal mechanisms that implement these requirements (parsers, identity records,
  dispatch rules, caching, transport).
- The exact authoring syntax and directives.
- Migration tooling and conversion utilities (covered by the migration documents).

## 6. Business requirements

"Shall" = mandatory; "should" = recommended.

### 6.1 Evolvability (architecture-level)

- **BR-EV-1** The model **shall** support guided, incremental change across multiple
  dimensions — surface syntax, underlying logic, and content/data model — without requiring a
  big-bang rewrite.
- **BR-EV-2** Each dimension **shall** be able to evolve **independently**: a change to syntax
  shall not force a change to the model or logic, and vice versa.
- **BR-EV-3** Multiple generations/versions of a content kind **shall** be able to coexist in
  one document or system during a transition.
- **BR-EV-4** The architecture **shall not** lock in a single syntax or model prematurely;
  hard-to-reverse decisions shall be deferrable to the last responsible moment.
- **BR-EV-5** Hard-to-reverse changes to a shared contract **should** follow a parallel-change
  pattern (expand → migrate → contract) so that no single step breaks existing content.

### 6.2 Content type

- **BR-CT-1** The platform **shall** treat the content type of each region as a first-class,
  identifiable kind that determines how the region is interpreted and what it can do.
- **BR-CT-2** A single document **shall** be able to contain multiple content types, scoped to
  regions, rather than a single document-wide type.
- **BR-CT-3** Each content type **shall** be independently versioned.
- **BR-CT-4** When processing crosses from one content type into an embedded region of another,
  the surrounding content type **shall** resume unchanged when the embedded region ends.

### 6.3 Syntax

- **BR-SY-1** The platform **shall** distinguish a content type (the kind of result) from the
  syntax (the surface form authored).
- **BR-SY-2** One content type **shall** be allowed to be authored in more than one syntax,
  producing the same kind of result, so syntax can evolve without changing the model.
- **BR-SY-3** Each syntax **shall** be governed by a schema that carries its own version.

### 6.4 Namespace

- **BR-NS-1** A namespace **shall** provide author-facing scoping conveniences: prefixes, a
  default, and tag/attribute aliases within the region it governs.
- **BR-NS-2** A namespace **shall** be the binding point that associates a content type, its
  syntax, and its version with a region.
- **BR-NS-3** Namespace bindings **shall** be nestable, so different regions of one document
  may use different content types, syntaxes, and versions at the same time, isolated from one
  another (appropriate coupling).
- **BR-NS-4** A namespace **shall** be identified by a stable identity that is durable across
  documents and tools.

### 6.5 Versioning and compatibility

- **BR-VC-1** Version compatibility between a document and a processor **shall** be negotiable:
  a document may request a specific version or a compatible range, and the outcome (accepted,
  upgraded-compatible, or rejected) **shall** be deterministic.
- **BR-VC-2** A compatible update (no breaking change) **shall** load existing content without
  author changes; an incompatible update **shall** be reported clearly rather than silently
  misinterpreted.
- **BR-VC-3** When a processor encounters content using a **newer** feature it does not
  understand, the handling **shall** be defined and predictable (for example: ignore, degrade,
  or reject) rather than undefined.
- **BR-VC-4** Each region's content type, syntax, and version **shall** be locally scoped, so a
  version change to one region does not invalidate sibling regions.

### 6.6 Guided change and fitness functions

- **BR-FF-1** The desired evolutionary characteristics **shall** be expressed as objective
  fitness functions — at minimum: prior-generation content still renders; version negotiation
  is deterministic; and no region is interpreted by another content type's processor.
- **BR-FF-2** Fitness functions **shall** be automatable and run as verification gates, so any
  change that preserves or violates a characteristic is detected before release.

### 6.7 Legacy coexistence (current focus: XSLT)

XSLT is the first application of the evolutionary model and a representative legacy case, not
the end goal.

- **BR-CO-1** The platform **shall** support embedding a legacy model (initially XSLT) and the
  current model (CEM-ML) within the same document in a non-conflicting manner.
- **BR-CO-2** Embedded legacy content **shall** remain pinned to its own content type and
  version and **shall not** be affected by future evolution of the current model.
- **BR-CO-3** Legacy support **shall** be explicit and opt-in, never a hidden default, so it
  can be inventoried and retired on a controlled schedule.
- **BR-CO-4** Neighboring legacy and current regions **shall not** be interpreted by each
  other's processing model.

## 7. Use cases

- **Evolve the syntax under a stable model.** A clearer surface syntax is introduced for an
  existing content type. Existing documents in the old syntax keep rendering; new documents use
  the new syntax; both map to the same model. (BR-SY-2, BR-EV-2)
- **Evolve the logic/model under a stable syntax.** The semantics or capabilities of a content
  type advance to a new version. Compatible documents load unchanged; incompatible ones are
  reported; older documents may pin the older version. (BR-VC-1..4, BR-EV-3)
- **Coexist with legacy XSLT (current focus).** A team migrating XSLT generators to CEM-ML
  keeps both in one document during transition. The XSLT regions are isolated and pinned to
  their own version, unaffected when CEM-ML advances, and are retired one at a time on schedule.
  (BR-CO-1..4)

## 8. Assumptions and constraints

- Content types, syntaxes, and models each carry comparable, independent version identities.
- Mixing generations is intentional and declared by the author, not inferred by guessing.
- Legacy support (XSLT) is a time-bounded compatibility path, not a long-term target.
- Evolution is governed by automatable fitness functions wired into existing verification gates.

## 9. Success criteria

Expressed as fitness functions wherever possible:

- A document mixing the current model and a legacy model renders both correctly, with neither
  corrupting the other. *(guards BR-CO-1, BR-CO-4)*
- Advancing the current model to a new major version requires no edits to embedded legacy
  regions, and prior-generation documents still render. *(guards BR-EV-3, BR-CO-2)*
- A compatible syntax/model update loads existing documents with no author changes; an
  incompatible one is reported, not silently misrendered. *(guards BR-VC-2)*
- Syntax can be changed for a content type without changing the model, and the model can be
  versioned without changing the syntax. *(guards BR-EV-2, BR-SY-2)*
- Legacy regions are explicitly marked and therefore discoverable for planned retirement.
  *(guards BR-CO-3)*

## 10. Open questions

The decisions required before this model can be committed are tracked as an immediate working
item in [`todo.md`](todo.md) (§ *Active — Evolutionary Architecture of the Authoring/Rendering
Model*). They cover: ratifying the evolution axes; defining and wiring fitness functions;
switching granularity (whole-template vs namespace-scoped); the cross-axis version-negotiation
policy; the parallel-change migration pattern; forward-compatibility/tolerant-processing
behavior; legacy (XSLT) retirement criteria; and the scope of dimensions the model governs.

## 11. Related documents and references

The technical realization of these requirements lives in the design and acceptance-criteria
documents; this BRD intentionally omits those details.

Internal:

- `cem-ml-syntax.md` — content-type, syntax, and namespace concepts in CEM-ML.
- `cem-ml-ac.md` — acceptance criteria for scoping, namespace dispatch, and versioning.
- `custom-element-bridge-template-policy.md` — current template-level routing of CEM-ML vs
  legacy content.
- `custom-element-template-migration-options.md` — options and schedule for migrating legacy
  XSLT.

External:

- N. Ford, R. Parsons, P. Kua, P. Sadalage, *Building Evolutionary Architectures* (O'Reilly) —
  guided incremental change, fitness functions, appropriate coupling.
- D. Sato, *Parallel Change* (expand/contract) — evolving contracts without breaking consumers.
