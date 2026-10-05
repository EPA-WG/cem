# Todo

This file is the authoritative checklist for active execution work.
Product/module sequencing lives in [`../roadmap.md`](../roadmap.md), future
wishlist work lives in [`wishlist.md`](wishlist.md), and completed execution
history is preserved under [`archive/`](archive/). The
[2026-09-27 snapshot](archive/todo-snapshot-2026-09-27.md) preserves completed
items and the context referenced by older progress notes. Later completions are
recorded in the [follow-up log](archive/todo-completed-2026-09-27.md).

## AST node reference implementation

The accepted [node references design](cem-ql-cem-ml-node-references-design.md) governs this work. Begin with schema contracts; parsing retains expressions and never executes reference selection on load.

The 2026-10-04 adoption defines implicit scope defaults, runtime-supplied
evaluation context, consumer reference-chain resolution, and scope-schema
diagnostic policy. Design adoption does not mark implementation complete.

Implementation plan recorded on 2026-10-04. Execute the stages in order, with
the contract fixtures before the corresponding implementation changes. The
workspace contains initial parser, operator, codec, schema, and fixture work;
review and reuse it rather than assuming a fresh implementation. All items
remain open until their adopted contract and verification evidence are met.

### 1. Audit current work and define schema contracts

- [x] Fixture: verify schema-owned standard reference traversal limits and
      scope overrides, including inheritance and rejection of zero, malformed,
      or overflowing bounds. Keep these bounds separate from lexical scope
      depth and native output-artifact limits.
- [x] Implement schema-sourced reference traversal defaults and validated
      effective-scope overrides for explicit consumer resolution. User chose
      overridable standard defaults on 2026-10-04; use depth 128 and work
      100000, matching the existing native value capacity baseline while
      retaining a separate reference-resolution policy.
- [x] Fixture: verify the three reference schema packages declare retained
      source, runtime-supplied context, non-owning ordered graph edges, and
      consumer-requested chain resolution separately. Verify optional projection
      context/target metadata does not require a runtime context ID.
- [x] Compare the initial reference changes and existing fixtures in `cem_ml`,
      `cem_ql`, and `cem_ml_transform_cem_ql` against the accepted design.
      Record covered behavior, gaps, and compatibility obligations, including
      saved containing-node handles, optional source target lists, the legacy
      `dom:reference` helper, and existing schema reference-resolution behavior.
- [x] Declare core schema-owned reference source, node/operator, and graph-edge
      contracts in the CEM-ML, CEM-QL, and AST projection schema packages.
      Define retained expression/provenance and lexical semantics separately
      from runtime evaluation context and outcomes. Update package contract
      documentation without inventing deferred public query type/access syntax.
- [ ] Complete reference expression-slot integration with retained expression
      artifacts, source-position bindings, caller-supplied runtime context, and
      typed attribute slots. Declaration metadata alone does not implement
      runtime evaluation or specialized scope linkage.
- [ ] Specify effective scope-schema policy for mandatory, warning, and ignore
      dispositions, empty-result cardinality, permitted scope crossings,
      reference traversal depth/work limits and numeric defaults, and cycle or
      limit outcomes. Keep lexical scope depth distinct from chain depth.
      Present unresolved policy choices for decision before implementing them.
- [x] Decide the fallback when no effective schema declares unresolved-link
      disposition. User adopted the recommended neutral consumer-owned outcome
      on 2026-10-04; explicit inherited mandatory, warning or ignore rules take
      precedence. Neutral outcomes neither emit a diagnostic nor imply success.
- [x] Fixture: verify neutral fallback, inherited mandatory/warning/ignore
      dispositions and explicit neutral child overrides. Reject unknown/missing
      values, duplicate declarations and incompatible diagnostic severity.
      Preserve unresolved facts and source provenance under every disposition.
      Verify depth/work limits and disposition compose into one effective
      policy without partially applying malformed or conflicting overrides.
- [x] Implement schema-owned unresolved-link disposition declarations and
      validated effective-scope overrides. Reuse schema diagnostic definitions;
      apply policy only to explicitly supplied unresolved facts, without query
      execution, graph mutation or empty-result cardinality inference.
- [x] Fixture: verify schema declarations and policy validation, including
      malformed or conflicting policies and inherited policy. Keep reference
      failures as neutral facts until the effective schema determines their
      diagnostic disposition; invalid expression/operand types remain invalid.
      Verification: five unresolved-policy, two traversal-policy, two schema
      contract and ten retained-reference fixtures pass. These validate policy
      and explicit fact treatment; they do not implement chain traversal.

Audit findings (2026-10-04): the initial implementation retains standalone
CEM/XML references and implements strict unary `#` construction and CEMB v3
graph edges. The legacy `dom:reference` helper keeps its separate preservation
behavior. Saved `context` handles are lexical origins, and optional `targets`
are representation choices. Scope-schema disposition and bound composition
are implemented for explicit unresolved facts. Shared chain evaluation,
per-link runtime application/cardinality, expression artifact linkage, typed attribute slots,
and schema construct reuse remain incomplete. The parser-backed target view now
preserves absent unevaluated targets separately from resolved-empty targets,
including field enumeration. Existing ID-backed name slots are not a `#` resolver.

Completed foundations: schema contract declarations, schema-sourced depth/work
defaults and validated inherited overrides, numeric graph-label validation,
XML query-payload provenance, and neutral/inherited unresolved-link policy.
These do not claim an implemented chain resolver or full scope-reference integration.

Verification (2026-10-04): 19 focused CEM-ML reference/policy tests pass.
The previous construction slice passed eight CEM-QL reference tests, all 99
native query-adapter tests, and the full CEM-QL Nx test target (829 tests).
The broad CEM-ML target has 2067 passing library tests,
two ignored tests, and one preexisting failure:
`legacy_custom_element::tests::material_manifest_primary_templates_convert_under_engine_subset`
cannot find `template#cem-dropdown`. This failure was reproduced before the
changes above; keep its gallery/template investigation separate from reference
implementation. `git diff --check` passes.

### 2. Retain reference syntax and lexical scope

- [x] Fixture: preserve query-payload source spans when XML text or CDATA is
      folded into a reference node, including namespace aliases and following
      siblings. Verify foreign `expr` elements remain ordinary elements.
- [ ] Fixture: extend CEM/XML parsing parity coverage for standalone and
      attribute expression slots, namespace aliases, forward expressions,
      query strings/comments containing delimiters, malformed expressions,
      and source locations. Prove loading and context closure never execute
      target selection or require an authored context-root ID.
- [ ] Align tokenizer/event normalization, parser/builders, XML import, lexical
      grammar, and tree-sitter parity with `{#...}` and `<cem:expr>#...</cem:expr>`.
      Retain the reference AST kind and expression artifact/source linkage in
      every supported slot instead of stringifying or inserting target children.
- [ ] Fixture: verify inherited scope-property defaults, source-position
      bindings and shadowing, restoration after existing schema/namespace child
      scopes, and equal IDs in separate vendor scopes. Cover explicit permitted
      crossings without a document-wide ID scan or repeated root markers.
- [ ] Align retained lexical/scope information and existing specialized frames
      with the shared reference contract. Resolve required lexical relationships
      at their existing context boundaries without treating a containing-node
      handle as the runtime evaluator environment. Use current scope forms;
      do not implement the deferred enclosed child override spelling.

### 3. Complete node-valued query construction

- [x] Fixture: construct references to all retained AST node kinds, including
      raw text, errors and retained reference occurrences. Verify repeated and
      mixed targets, absent versus empty source-reference targets, independent
      same-content document owners, and dynamic rejection of scalar operands.
      Preserve existing `data:read(...).id` selection keys across fresh reads
      while distinguishing their runtime node-owner identities.
- [x] Align the imported typed query view with retained reference fields and
      distinguish independently imported document owners from source content
      fingerprints. Keep authored graph edges separate from structural children.
      Importable source kinds use the native retained-tree view; explicitly
      supplied typed error nodes remain eligible operands without changing the
      semantic import boundary's rejection of source errors. Query-local
      memoization may reuse one import; independent executions have distinct
      owners. Eight query reference fixtures, focused import/view tests, and
      all six viewer-selection compatibility tests pass. Full CEM-QL native
      verification through Nx passes 829 tests. The initial broad run exposed
      stable `.id` selection-key compatibility; that behavior is preserved
      separately from runtime owner identity and covered by the rerun.
- [x] Fixture: distinguish an unevaluated reference's absent targets from a
      resolved empty target list in the parser-backed typed query view. Verify
      repeated cyclic edges remain non-owning and are not followed by inspection.
- [x] Preserve absent versus resolved-empty targets in the existing typed
      query bridge without adding public target-access syntax or executing the
      authored reference expression.
- [x] Correct the initial precedence fixture to inspect typed reference targets
      through the runtime view rather than assuming an adopted `.targets`
      query operation. Public query target-access spelling remains deferred.
- [x] Fixture: verify unary precedence/grouping and strict operands for every
      supported AST node kind, attribute nodes, empty sequences, ordered and
      repeated targets, mixed ordinary/reference nodes, and `##nodes`. Reject
      statically and dynamically supplied scalars, including strings that look
      like IDs, URLs, or query source; construction must not follow chains.
- [x] Align query parsing, type checking, IR lowering, and evaluation with the
      schema contracts. Construct non-owning reference edges through retained
      typed CEM node views, preserving target order, multiplicity, owning
      document identity, and reference occurrence identity. Keep legacy helper
      compatibility explicit without changing `#` into implicit dereferencing.
- [ ] Verify the shared typed bridge used by `cem_ml_transform_cem_ql` exposes
      references without format-specific evaluation or DOM/JSON substitutes.
      Treat current generic-node and target-view interfaces as implementation
      choices, not adoption of the deferred public reference type/access API.

### 4. Add explicit runtime evaluation and bounded chain resolution

- [x] Decide scope-crossing budgets: apply request and destination limits.
      Destination limits constrain their subtree; repeated entries never reset
      work accounting within the request (user decision, 2026-10-04).
- [x] Fixture: verify destination depth/work limits, cumulative work on repeated
      entry, pruning an exhausted subtree while preserving enclosing siblings,
      and stricter request caps despite more permissive destination limits.
- [x] Implement host-owned opaque runtime scope keys and cumulative per-scope
      budgets alongside the request cap, without authored context/root IDs.
- [x] Decide cycle/limit outcome treatment before shared traversal. User adopted
      incomplete unresolved outcomes governed by scope disposition on 2026-10-04.
      Cycles and limits always stop the affected traversal, preserve the graph,
      and keep the outcome incomplete even under neutral or ignore disposition.
- [x] Fixture: verify iterative shared traversal of typed reference graphs,
      preserving order and repeated targets. Cover branch-local cycles and
      depth limits, execution-wide work exhaustion, per-link dispositions,
      explicit denied crossings, pending/invalid/empty outcomes and deep chains
      without recursive stack growth. Reuse one source with independent runtime
      evaluations and preserve the authored targets and source provenance.
      Verify constructed reference occurrences require neither a saved arena
      handle nor a captured runtime context handle. Verification: 30 focused
      CEM-ML tests and two CEM-QL runtime consumer fixtures pass.
- [x] Fixture: connect the shared host-driven resolver to the existing CEM-QL
      expression mechanism in a consumer fixture. Evaluate the same retained
      `{#datadom}` source against independent supplied native data trees, with
      delayed readiness, empty selections and invalid scalar operands, without
      writing results onto the source or adding authored data-root IDs.
- [x] Implement an explicit host-driven shared resolver over retained typed
      nodes. The host supplies evaluation timing/context, occurrence metadata,
      effective per-link disposition and explicit edge permission. Keep request
      and cumulative scope budgets, execution-local results, and immutable sources;
      do not infer scopes from document owners or authored IDs.
- [ ] Specify the shared evaluation entry point and result ownership using the
      existing CEM-QL expression mechanism. The caller supplies context and
      lifecycle timing; retain source-position lexical meaning. Distinguish
      unevaluated source from pending, resolved, unresolved, and invalid runtime
      outcomes without requiring writeback onto the authored reference.
- [ ] Fixture: evaluate one retained template against two independent supplied
      `datadom` trees, including evaluation after inputs become available.
      Verify independent outcomes, distinct resolved-empty and unresolved
      results, and unchanged authored expressions/edges without context IDs.
- [ ] Fixture: cover deep reference chains, mixed node/reference targets,
      repeated targets, self-reference, cycles, scope crossings, and traversal
      limits within one lexical scope. Verify mandatory failure, warning, and
      ignored unresolved links with source provenance, plus separate empty
      cardinality checks. Ignoring a link must not fabricate a target or remove
      an authored edge.
- [ ] Implement shared explicit evaluation and consumer-requested chain
      resolution under effective scope-schema policy, with cycle detection and
      bounded work. Preserve target order/multiplicity and the authored graph.
      Do not add a scheduler, subscriptions, automatic parse-time evaluation,
      URL/ID lookup, or element-to-ID projection.

### 5. Integrate schema validation and construct reuse

- [x] Decide the first schema consumer: declaration reuse in collections such
      as `{elements}` during schema compilation (user decision, 2026-10-04).
      Preserve existing `@base`/`{uses}` behavior. Implementation remains open.
      Scenarios for later design verification: two sites reuse one declaration
      without source cloning; wrong target kinds/cardinality diagnose at the
      reference; existing base/alias fixtures remain compatible.
- [x] Decide incomplete declaration selection treatment before integration:
      retain available declarations with an explicit incomplete model outcome
      (user adopted recommendation, 2026-10-04). Mandatory failures and invalid
      expressions still fail compilation. Scenarios for later design
      verification: one valid declaration plus an unavailable dependency under neutral/warning/ignore remains visibly
      incomplete; mandatory/invalid results cannot become successful models;
      pending compilation remains distinct from an empty declaration selection.
- [x] Fixture: compile typed references in `{elements}` with ordered zero/many
      declaration targets, repeated selections, wrong target kinds/missing names,
      independent source owners and original lexical `@base`/`{uses}` aliases.
      Verify no source cloning/writeback and collection order compatibility.
- [x] Fixture: retain available declarations and explicit incomplete outcomes
      under all dispositions; preserve pending, empty and invalid distinctions,
      diagnostic attribution, denied crossings, cycles and bounded work.
- [x] Fixture: compile through a host with runtime-constructed typed references
      outside a saved AST arena. Verify shared depth/work accounting, native
      occurrence metadata without context IDs, and retained declaration owners.
- [x] Implement explicit host-driven `{elements}` declaration compilation and
      per-site retained outcomes. Compile direct references without an evaluator
      as pending; use declaring lexical aliases for referenced declarations.
      Verification: seven declaration-consumer fixtures plus 30 existing reference
      tests and two CEM-QL runtime fixtures pass. Nx `cem_ml:test` reports 2067
      unit tests passing, two ignored, and the existing dropdown-template
      failure (`material_manifest_primary_templates_convert_under_engine_subset`).
      Each collection site makes one bounded resolution request; whole-document
      compilation and query-instruction budgets remain separate concerns.
- [x] Decide package activation for incomplete compilation: retain models for
      inspection but keep them inactive for final validation (user adopted the
      recommendation, 2026-10-04). Neutral and ignore may be incomplete without
      diagnostics; readiness must not be inferred from diagnostics alone.
      Scenarios for later design verification: incomplete neutral/ignored models
      never appear ready accidentally; pending sources remain distinct from
      invalid schemas; completing inputs permits activation without source edits.
- [x] Fixture: block registry lookup, built-in fallback and direct final
      validation for pending/unresolved/invalid models, including neutral and
      ignore without link diagnostics. Preserve inspection and original facts;
      completing the same source permits ready registration and validation.
- [x] Fixture: load an incomplete or hard-invalid schema package without publishing its schema,
      converter routes or artifacts. Preserve the inactive model for inspection;
      retain converter-only package compatibility and ready package activation.
- [x] Gate initial package publication, registry identity resolution and final
      validation on complete reference outcomes. Package publication also retains
      existing hard compile-error checks; legacy structural projections keep
      their existing handling of behavior delegated to other consumers. Preserve inspection, original link dispositions and diagnostics;
      direct blocked validation reports a consumer-operation readiness error.
      Inactive explicit models must not fall back to built-ins. Ready converter-
      only packages retain their existing behavior.
      Verification: 41 reference/declaration integration tests, one package
      readiness unit fixture and two CEM-QL runtime fixtures pass. Nx `cem_ml:test`
      reports 2068 passing, two ignored and only the existing dropdown-template
      failure; native-template/transform/XPath projection compatibility is preserved.
- [x] Decide coordinated refresh of an already-active schema package: retain
      the last complete package while inspecting an incomplete replacement,
      then switch schema, converter routes and artifacts together (user adopted
      recommendation, 2026-10-04). Staged publication is implemented below.
      Scenarios for later design verification: incomplete replacements cannot
      mix old converter routes with a new schema; rejected candidates preserve
      coherent registrations; a complete replacement switches every owned part;
      candidate inspection remains separate from active validation identity.
- [x] Decide replacement ownership: require the same manifest origin or an
      explicit grant for a different origin/built-in override (user decision,
      2026-10-04). Matching local package IDs alone must not authorize replacement.
- [x] Decide explicit grant semantics: manifest inclusion alone does not grant
      replacement authority; require a separate scoped runtime grant naming the
      expected package and origin (user adopted recommendation, 2026-10-04).
      Existing override consumers need migration. Scenarios for later design
      verification: equal local IDs across vendors do not authorize replacement;
      same-origin refresh works; authorized built-in overrides remain supported;
      mismatched expected ownership fails without changing active registrations.
- [x] Separate latest candidate inspection from last-complete active model
      identity lookup. Incomplete replacements preserve active model/schema/
      converter/artifact registrations. Initial inactive candidates still block
      built-in fallback. Registry-level ready model replacement is explicit;
      coordinated whole-package ready switching is implemented below.
- [x] Fixture: verify same-origin package refresh removes stale schemas, lookup
      indexes, converter routes and artifacts; conflicting ownership, wrong grants
      and converter collisions preserve all active registrations. Verify scoped
      cross-origin and built-in overrides, ready empty projections and
      incomplete-candidate inspection.
- [x] Implement origin tracking, scoped replacement grants and coordinated ready
      refresh on staged registries. Reject conflicting ownership without changing
      active registrations; switch schema, model, converter routes and artifacts
      together. Migrate existing native override fixtures to scoped grants.
      Runtime ownership uses exact resolved manifest URIs; direct registry writes
      become Untracked. Grants name package, expected owner and incoming URI.
      Verification: coordinated-refresh/readiness fixtures and 11 declaration
      integration tests pass. Nx `cem_ml:test` reports 2069 passing, two ignored
      and only the existing dropdown-template failure.
      Scenarios for later design verification: URI aliases do not imply authority;
      incomplete and rejected candidates stay inspectable; stale schema identity
      indexes and routes are removed only on complete publication.
- [x] Fixture: retain a last complete model for active identity lookup while
      inspecting an incomplete replacement. Verify final validation uses the
      active model, later completion switches the model, and initial inactive
      candidates still block built-in fallback.
- [ ] Supply explicit lifecycle evaluation for package declaration references
      through the production consumer host, allowing completion and activation
      of the same source without automatic parser evaluation or a new scheduler.
- [ ] Adapt native CEM-QL declaration views to original retained declaration
      owners for the production schema host. Do not serialize or clone source
      ASTs to manufacture target arenas; preserve lexical aliases and boundaries.
- [ ] Define the existing schema validation/composition sites that consume typed
      node references, their target-kind/cardinality constraints, and required
      evaluation phase. Keep their construct-use and recursive-composition
      rules consumer-owned while applying the shared scope resolution policy.
- [ ] Fixture: validate referenced constructs and reuse one declaration from
      multiple schema sites without cloning it into the source tree. Cover
      invalid target kinds, unresolved links under each disposition, source
      attribution, independent scopes, and bounded recursive relationships.
- [ ] Integrate the shared reference evaluation path into those schema sites
      and specialized scope records. Preserve existing schema/namespace
      behavior and typed import boundaries; do not replace specialized storage
      with a mandatory common struct or impose public root IDs.

- [ ] Integrate scoped replacement grants into command/WASM consumer APIs when
      override support is added. Do not infer grants from package URI lists or
      manifest contents. Add native boundary fixtures before extending those APIs.
      Scenarios for later design verification: manifest inclusion does not grant
      authority; expected-owner or incoming-URI mismatches preserve the active
      package; native consumer grants survive the boundary without widening scope.

### 6. Preserve references through codecs and inspection

- [x] Fixture: verify CEMB reference states, repeated graph edges and source
      provenance without structural expansion; reject invalid context/target
      handles. Read version 2 ordinary nodes and reject version 3 reference
      tags in a version 2 payload.
- [x] Enforce version-specific reference tags in the CEMB decoder while
      preserving version 2 read compatibility and version 3 graph retention.
      Verification: all ten `node_references` fixtures and nine existing
      `ast::tests` pass, including the version-tag regression that failed
      before the decoder fix.
- [ ] Fixture: extend current CEMB round trips and version compatibility coverage
      for unevaluated source, resolved-empty and ordered/repeated graph edges,
      reference-to-reference edges, self-reference, cycles, source provenance,
      and invalid node handles. Reload source for later evaluation with a newly
      supplied runtime context without serializing a live environment.
- [ ] Align AST codec, native graph integration, formatter, XML convention
      export/import, and typed AST/DOM/event inspection with retained reference
      identity and source semantics. Inspection must not follow chains or
      silently turn references into text or structural target subtrees.
      Keep optional runtime outcomes distinct from source-only export.
- [ ] Record the supported AST graph round-trip boundary and remaining native
      output artifact gaps. Do not select cyclic native-output expansion,
      pending-outcome transport, or new transport contracts in this stage;
      those decisions remain in the roadmap.

### 7. Verify and document completion

- [ ] Fixture: add focused Rust integration cases through the existing CLI/query
      and schema paths for retained references and explicit supplied-context
      evaluation. Confirm URL resolution stays at the resource/import boundary
      and no projected IDs or browser behavior are needed for CEM-ML references.
- [ ] Run affected Rust package, schema consistency, grammar parity, codec,
      and focused CLI checks through the appropriate Nx targets. Record results
      and remaining failures; browser/WASM integration follows a green native
      path only where the changed shared boundary requires it.
- [ ] Update public syntax/package/API documentation and initial implementation
      notes to match verified behavior. Mark only verified action items complete,
      record any unsupported cases, and retain deferred decisions and their
      verification scenarios without claiming full consumer support.

The [enclosed child override syntax](../roadmap.md#deferred-cem-reference-syntax-decision)
and [public query/transport contracts](../roadmap.md#deferred-cem-reference-query-and-transport-contracts)
remain deferred. Stages above implement the accepted contract using existing
forms and internal interfaces; if a specific step requires one of those
decisions, resolve it explicitly before that step rather than guessing.
The [cem-element reference mode](#deferred-cem-element-reference-consumption),
interaction attribute API, and ID generation/extraction remain subsequent
consumer work.

### Scenarios for later design verification

These scenarios are preserved from the adopted proposal for future design and
implementation verification. They do not claim executable test coverage.

- A scope default supplies the same effective property to many nodes without
  repeated references or an authored root ID.
- Two vendor scopes contain equal ID strings without accidental cross-scope
  selection; an explicit boundary contract permits an intended relationship.
- A URL selects a publicly exposed part of an external document without
  introducing URL resolution into CEM-QL.
- One retained template reference is evaluated during two transformations with
  different supplied `datadom` contexts and without mutating a shared target
  list on the authored occurrence.
- A consumer deeply resolves a chain of reference nodes within its effective
  scope policy while preserving the authored graph. A cycle or traversal limit
  stops resolution even when every link remains in the same lexical scope.
- Unresolved links follow mandatory, warning, or ignore requirements declared
  by the applicable scope schema; a resolved empty selection is handled
  separately by the schema's cardinality rule.
- Schema constructs are declared once and referenced by composition and
  validation consumers without copying them into the source tree.

## Deferred cem-element reference consumption

Deferred on 2026-10-04 until the CEM-ML reference design is complete. The adopted
[consumer responsibilities](cem-ql-cem-ml-node-references-design.md#consumer-responsibilities-and-examples)
establish ownership; adoption does not choose or implement the consumer mode.

- [ ] Specify `cem-element` treatment of references: detect reference nodes in
      templates and provide a transformation mode that resolves element
      references to the produced elements' IDs, preserving explicit IDs and
      generating IDs when needed. Decide the interaction attribute API,
      explicit cross-scope relationships, and compatibility with local-name
      conveniences in this consumer work. The runtime supplies evaluation
      context and timing; ID generation and extraction remain consumer-owned.
- [ ] After that consumer contract is specified, implement and verify the
      reference-to-ID mode in the shared `cem-elements` transformation path.
      Add actionable fixture items before creating fixtures.

### Scenarios for later design verification

- A consumer uses the referenced node's declared meaning to generate a native
  relationship, preserving a target element's explicit ID or generating one.
- Repeated template instances resolve references against their own supplied
  contexts and produced elements. Verify intended scope crossings and the
  specified local-name compatibility without a document-wide CEM ID lookup.

## CEM-QL / CEM-ML node references design

The 2026-10-03 items below record adoption history. The 2026-10-04 revision
supersedes their references to an open preservation alternative and their
consumer-reference API assumptions; deferred implementation items remain open.

- [x] Adopt the reference scope and consumer responsibilities proposal on
      2026-10-04 into the reference, syntax, and interaction designs: implicit
      defaults and enclosed override semantics, scoped IDs and URL part access,
      runtime context without mandatory IDs or source target lists, bounded
      consumer chain resolution, and schema-controlled unresolved links. Record
      deferred syntax, query/transport, and `cem-element` work with verification
      scenarios beside roadmap/todo action items; remove the temporary proposal.

- [x] Write a standalone proposal for a reference AST node targeting other node(s) through CEM-QL, including `#` and `{#datadom.attributes.commandfor}`. Define initial parsing/finalization semantics without a template/datadom-only scope. Keep all consumer behavior, CEMT/import IDs, cem-element/browser projection, and AST re-evaluation outside scope. See [the proposal](cem-ql-cem-ml-node-references-design.md); promoted to an accepted design; remaining open decisions are identified explicitly.
- [x] Adopt # prefix-unary precedence at level 9, with tighter postfix/type/dot operations, looser infix operations, and right-to-left prefix nesting (2026-10-03). Type and existing-reference semantics remain proposed; no implementation is authorized by this documentation change.

- [x] Adopt standalone `{#...}` reference expressions and XML `<cem:expr>#...</cem:expr>` semantic parity (2026-10-03). Do not introduce a separate `cem:reference` vocabulary. Remaining reference type/resolution semantics remain under discussion; syntax is not implemented.
- [x] Adopt the common contextual-reference resolution model (2026-10-03): reuse existing scope/binding associations and source-position semantics, distinguish binding resolution from target availability, and keep specialized constraints without a new independent context registry. Concrete representation and initial-evaluation boundary details remain under discussion.
- [x] Adopt context closure as the definite boundary for AST-specific reference resolution (2026-10-03). Retain expressions/dependencies when required context is not yet supplied; later explicit consumer evaluation is possible but its mechanics remain outside this proposal.
- [x] Adopt any AST node kind as a reference target, including reference nodes, with self-reference and cycles permitted as graph edges (2026-10-03). AST loading retains expressions; context closure resolves contextual bindings without automatically executing reference queries. Explicit evaluation and consumer mechanics are separate. Reference-value preservation versus reference-node targeting remains open.
- [x] Adopt reuse of existing AST node/owning-graph identity and graph serialization for reference occurrences and target edges (2026-10-03). Do not introduce a separate global reference ID scheme or require persistent identity across reparses; query comparison syntax remains separate.
- [x] Adopt strict node-valued reference construction and distinguish attribute nodes from their scalar values (2026-10-03). `#` performs no implicit ID/name/selector lookup or expression execution on strings; additional scalar interpretation requires a separate explicit contract. Existing-reference preservation remains proposed.
- [x] Promote the node-reference proposal to [accepted design](cem-ql-cem-ml-node-references-design.md) on 2026-10-03. Preserve the adopted semantics, remove superseded automatic-query-evaluation wording, and retain unresolved type/storage/reference-preservation choices as explicit open decisions. Keep the former proposal path as a redirect.

## Interaction design implementation: cem-action

The [accepted interaction design](cem-interaction-design.md) takes precedence over conflicting implementation. Deliver one component at a time; this first step owns the action invoker, not popup/menu/dialog lifecycle.

- [x] Add generic action-command wiring in cem-elements and consume it from the cem-action declaration: native invoker attributes, explicit local/ID references, descriptor defaults, invocation metadata, conflict diagnostics, and cleanup.
- [x] Fixture: add colocated action stories for real native popover/modal commands, trusted keyboard single activation, local-scope isolation, dynamic rebinding, custom-command source/context, conflicting routes, disabled controls, and native submit preservation.
- [x] Update action playground/gallery and public documentation; verify declarations, runtime unit/type checks and focused browser stories through Nx.

Validation (2026-10-03): 21 action Chromium stories and 588 runtime unit tests pass. Runtime/component builds, declarative verification, lint (two existing warnings), style contract, and `verify-playgrounds --args=--action-only` pass, including source and isolated-package galleries. Full playground verification still fails the existing cem-menu-item attribute inventory (`aria-label`/`href`), reproduced with the original action gallery. Surface/theme composition below remains unresolved; do not claim complete interaction lifecycle coverage.
- [ ] Follow-up fixture: preserve unrelated anonymous demo slice state when native popup/modal activation is followed by a theme-switch rerender. The original gallery passes that check; the combined native-surface sequence currently resets the layout-choice demo.
## Interaction design specification

- [x] Convert the adopted popup/action/menu/dialog proposal into a concrete design with public naming, native-owner lowering, recursive submenu wiring, local references, focus/context, materialization, diagnostics, and an acceptance matrix. See [the design](cem-interaction-design.md); implementation and migration are separate work.
- [x] Promote [the interaction design](cem-interaction-design.md) to accepted status on 2026-10-03. Its names and contracts govern future implementation; the temporary proposal is a research archive.

## Dropdown sibling filler demonstration

- [x] Add exactly five visible `<hr/>` filler siblings to each dropdown gallery example, including theme and RTL containers, to demonstrate overlay paint over surrounding DOM.
- [x] Fixture: verify five fillers and hit-test the open panel where it overlaps a filler; verify native pointer hover above the sibling and that closing reveals the same filler, across five themes and source/generated/installed galleries. `verify-menu-dropdown` passes, including narrow layouts, scrolling and forced colors.

## Dropdown initial stacking and legacy styling

- [x] Adopt the legacy dropdown panel stacking level and attached shape; retain CSS anchoring for initially open panels instead of moving offscreen examples into the viewport.
- [x] Fixture: verify initial-open panels remain attached to their triggers and within reserved gallery regions before interaction, and verify declaration-owned z-index and shape across source/generated/installed galleries. `verify-menu-dropdown`, 588 runtime unit tests and runtime lint pass.

## Dropdown scroll anchoring

- [x] Restore legacy relative-container/absolute-panel CSS anchoring for dropdowns while retaining generic collision handling when opened or resized.
- [x] Fixture: verify popup coordinate conversion and open panels scrolling with their triggers beyond the viewport edge in source/generated/installed galleries and nested scrolling containers. `cem-elements:test:unit` passes 588 tests; runtime lint and `verify-menu-dropdown` pass, including five themes, narrow layouts and forced colors.

## Dropdown demo overlay clearance

- [x] Reserve theme-sized visual space around dropdown gallery examples and the property preview so overlays do not cover source viewers, descriptions or adjacent demos; keep room for nested and RTL panels.
- [x] Fixture: check open gallery panels and property previews against their reserved demo region in source/generated/installed previews, including all five themes and narrow viewports; rebuild playgrounds. `verify-menu-dropdown` passes with desktop and 390px clearance checks and forced-colors coverage.

## Autocomplete and suggestions design for CEM inputs

Reference: [legacy autocomplete 0.0.39](https://unpkg.com/@epa-wg/custom-element-dist@0.0.39/src/material/components/autocomplete.html).

- [ ] Audit and map the source autocomplete API and examples, including its `input`/`menu` slots, explicit data/option values, text-as-value fallback, grouped suggestions, filtering and selected-value behavior; distinguish preserved contracts from extensions.
- [ ] Design reusable suggestions composition that attaches to existing `cem-field`, `cem-text-field` and applicable CEM input controls through their actual native input; preserve one form owner, naming, value/input/change events, validation, disabled/readonly behavior and normal editing. Decide textarea applicability explicitly.
- [ ] Evaluate reuse of `cem-dropdown` for autocomplete/suggestions: compare direct dropdown composition with reuse of its shared popup capability/controller for visibility, placement, collision handling and dismissal. Keep focus on the editable input, use combobox/listbox semantics rather than menu semantics, and avoid duplicate triggers, focus restoration handlers or form controls; record the selected composition and tradeoffs before implementation.
- [ ] Specify declarative attachment/slots and the shared cem-elements capability for filtering, active suggestion, commit/cancel and popup lifecycle. Reuse dropdown popup geometry/dismissal without menu roles: editable combobox + listbox/options, input focus retained with aria-activedescendant and explicit controls/expanded relationships.
- [ ] Define ArrowUp/Down, Enter commit, Escape cancel/dismiss, Tab, pointer selection, IME composition, native text editing, empty/no-match and hidden/disabled suggestion behavior; distinguish display label from committed value and define free-text versus constrained selection.
- [ ] Define static and externally loaded suggestion data at the retained CEM tree boundary, with optional asynchronous loading, stale-response handling, loading/error feedback and accessible announcements; keep UI behavior out of components, galleries and application JavaScript.
- [ ] Fixture: add shared native contract cases first, then browser stories applying the same suggestions to cem-field and cem-text-field (and any accepted additional input), covering forms, dynamic suggestions, grouped filtering, focus, IME, independent controls and cancellation.
- [ ] Fixture: plan linked property playgrounds and galleries mapping source examples, all five themes, forced colors, keyboard/pointer use and source/generated/installed-package checks; update public attribute/slot inventory, accessibility contracts, catalog and exports with implementation.

## Source dropdown and menu composition

- [x] Adopt version 0.0.39 dropdown source API (label, open, base/default slots) in a canonical declaration with shared popup lifecycle, positioning and focus.
- [x] Share popup mechanics between dropdown and nested menus; verify root dropdown menus dismiss together and restore the correct trigger.
- [x] Fixture: add browser stories for projected base, arbitrary content, external open changes, pointer/keyboard activation, nested menus, dismissal, disabled controls and viewport collisions.
- [x] Fixture: add property playground and gallery mapping all source examples, five themes, forced colors and nested dropdown/menu composition; verify generated pages and package exports.
- [x] Update indexes, catalog sources and public attribute/slot contracts; run build, style, declarative and browser checks.

Validation: dropdown/menu browser stories passed, including multilevel Escape
restoration, native activation and complete-chain dismissal. Source, generated
and isolated-package galleries passed in all five themes and forced colors,
including viewport bounds. Build, catalog, declarative/style and package gates
passed. The shared controller owns popup geometry and focusable-entry selection.

## Source menu and nested interaction

- [x] Implement the persistent wrapping cem-menu declaration, direction/justify defaults, native navigation, item paint, tokens, labelling and disabled behavior.
- [x] Add generic cem-elements composite navigation, focus, submenu dismissal and collision-aware positioning capabilities; consume them declaratively and extend menu-item links/submenu slots.
- [x] Fixture: add shared navigation contract tests before implementation, then browser stories for native and composite input, nested focus/restoration/dismissal, dynamic removal, independent siblings and empty/disabled menus.
- [x] Fixture: add property playground and gallery covering source examples, token overrides, five themes, forced colors, multilevel submenus and RTL; verify source/generated pages.
- [x] Update indexes, public contracts/inventory, catalog inputs, package exports and legacy source mapping; run Nx runtime/component/build/declarative/style checks.

Validation: 587 runtime unit tests and 11 component browser stories passed.
Runtime/component builds, lint (two existing runtime warnings), declarative/style,
state-matrix, catalog, package and source/isolated gallery-navigation checks passed.
All six source/generated/installed menu/dropdown gallery checks passed across
five themes and forced colors. Submenu owner href/expanded compatibility,
dynamic removal/re-enabling and empty/disabled-only panel focus are covered.

## Gallery heading weight

- [x] Add a theme-owned heading weight endpoint backed by strong voice and consume it in the shared gallery navbar.
- [x] Fixture: verify generated token exports and rendered heading weights across all five themes, including independent consumer overrides; build component galleries.

Validation: theme/component builds and manifest coverage (511/511) passed.
Public token exports and source/generated gallery H1 weights match all five
themes; consumer heading overrides leave UI text weight unchanged.

## Typography size scale

- [x] Apply the 10/12/14/16/24/34/48px scale to canonical typography tokens and matching documented CSS values, retaining rem units and theme invariance.
- [x] Fixture: rebuild theme CSS and token exports; verify the seven generated size values and token manifest.

Validation: theme build passed; all seven generated CSS/public token values
match the scale. All 510 manifest tokens are covered, with no gaps.

## Input gallery CEM-ML examples

- [x] Convert form validation/submission/reset, presence/hidden/class and validation-message examples to native tabular CEM-ML in all three input galleries.
- [x] Fixture: verify native round-trip structure and component build for the converted examples.

Validation: all nine native XML structure round-trips passed. Component build
and Chromium checks in all three source/generated galleries passed, including
form ownership, required validation, reset, presence attributes and messages.

## Filled and outlined input backgrounds

- [x] Verify Material's filled/outlined background distinction; add a theme-owned filled input background token and attribute-dependent transparent override for all three canonical input controls.
- [x] Fixture: cover empty/populated default/fallback/outline backgrounds, state changes and live restoration in all five themes; add clear standalone marker slot examples to each gallery and verify builds/tests.

Validation: all 24 colocated Chromium tests passed across five themes. Theme
and component builds, token manifest and style-contract checks passed. Source
and generated galleries passed background and standalone marker slot checks.

## Customize required markers

- [x] Add required-marker attribute/slot overrides to all three input controls, retaining required presence and slot-over-attribute precedence.
- [x] Fixture: verify live/default/empty attribute values, slot markup precedence and hidden optional markers; update galleries, playgrounds and public API docs and run browser/build checks.

Validation: all 24 colocated Chromium tests passed. Component build and
style/declarative/state-matrix gates passed. Source and generated galleries and
property playgrounds passed marker override and visibility checks.

## Required markers for input controls

- [x] Add the declarative required star to cem-field, cem-text-field and cem-textarea labels, with inherited label color and native required accessibility semantics.
- [x] Fixture: cover presence values, optional controls, projected labels, validation, disabled/readonly/busy states and independence from indicator appearance in all five themes; add gallery use cases and verify the build.

Validation: all 21 colocated Chromium tests passed. Component build,
declarative/state-matrix gates and style contract passed; all three source and
generated gallery marker samples were checked in Chromium.

## Indicator appearance examples

- [x] Add focused underline/outline/default/fallback examples to cem-field, cem-text-field and cem-textarea galleries using the implemented API.
- [x] Fixture: verify live indicator geometry, retained control/value and unchanged layout across all five themes; build and check source/generated gallery samples.
- [x] Review local theme documentation for the separate required marker and record an implementation proposal.

Validation: 18 colocated Chromium tests passed; component build and
declarative/state-matrix gates passed. All three source and generated gallery
indicator samples render with the expected boundary geometry. Required-marker
proposal recorded in the field controls contract; marker behavior is deferred.

## Resting indicators for text inputs

- [x] Validate the Material filled-field resting underline and apply the existing theme boundary/anchor tokens to cem-field, cem-text-field and cem-textarea.
- [x] Fixture: extend colocated ThemeStates/InteractionPaint plays for empty and populated resting controls in all five themes and both indicator appearances; verify state precedence and native borders remain unchanged.

Validation: all 18 colocated Chromium story tests passed (three components, five
themes, resting empty/populated values, hover/focus and state precedence).
Component build, declarative/state-matrix gates and style contract passed.

## Shared gallery navigation

- [x] Add reusable declarative gallery navigation with Playground link, page H1 (component name and page goal) and a disabled theme icon placeholder; use it on every component/gallery page and package the index/navigation.
- [x] Fixture: verify shared navigation on source and packaged galleries and component pages, including accessible controls and working index links.

Validation: playground build and focused browser navigation checks passed for all
20 source pages and their isolated-package copies. The broader playground suite
currently times out in the icon playground content-projection check.

## Icon-button action and icon sizing

- [x] Reuse action size tokens while forwarding the same size to cem-icon.
- [x] Add dedicated sizing examples and a colocated fixture for both modes, inheritance and live size changes; run available checks.

Validation: native sizing-sample renders, composition renders, style contract,
Storybook syntax and Nx builds/gates passed. Browser geometry fixture added;
execution remains pending under the session’s browser restriction.

## Convert icon-button gallery source syntax

- [x] Convert all 16 gallery demo sources to CEM-ML with the native tabular formatter; verify XML round trips and rendered element/attribute/text parity with the original HTML.

## Align cem-icon-button with the action API

- [x] Replace kind/quiet with cem-action variants and their complete state paint
      in both modes; reuse bend classes and native button/form attributes.
- [x] Fixture: cover variant changes, native submit/reset/form ownership and
      form overrides; retain icon sizing, state and link behavior.
- [x] Migrate focused samples, property controls, theme matrices, documentation
      and verification expectations; build and run available native/style checks.

Validation: declarative/state-matrix gates, bundle/playground builds, style-token
checks, 576 state renders, 33 composition renders, and 30 variant/form renders
passed. Browser execution of the new stories remains pending because Chromium
requires host permission unavailable in this session.

## Respect action states in cem-icon-button

- [x] Add selected/selectable and pending semantics with action/theme paint,
      persistent selection, disabled precedence, contrast contours, independent
      keyboard focus, reduced motion and forced colors. Remove expanded from
      the component API, ARIA forwarding, playground and gallery.
- [x] Fixture: add colocated ActionStates coverage for both native control modes,
      live state changes, presence semantics and combined states. Update the
      state-matrix evidence and gallery reduced-motion/forced-color checks.
- [x] Add focused selection/pending samples, property controls and five-theme
      matrix rows. Describe action-theme support in the component, contract,
      playground and gallery. All eleven focused samples and attribute
      inventories match in source and built pages.
- [x] Native rendering passes 576 action-state combinations, 33 icon composition
      cases and 18 page-template cases. Declarative and state-matrix gates,
      stylesheet contract, bundle and playground builds pass.
- [ ] Run the new browser state/paint checks and full source/package gallery
      verification when browser execution is permitted. Restricted session
      permissions still prevent that verification.

## Focused cem-icon-button samples

- [x] Replace the All implemented attributes sample in gallery and playground
      with individual command, activation, navigation, disabled, selection, pending,
      kind, variant, class and visibility examples.
- [x] Fixture: add browser checks using meaningful demo/attribute selectors;
      verify all eleven samples and value combinations in source and built pages.
      Navigation includes absent, empty, fragment, relative and absolute href;
      disabled and selection cover both modes, and kinds/variants cover defaults
      and every documented value. Browser execution remains pending below.

## Use image as the sole cem-icon-button source

- [x] Remove icon/name aliases and migrate component consumers, documentation,
      playground controls and gallery examples to image.
- [x] Fixture: cover image updates, empty/absent sources and ignored removed
      aliases in button and link modes; update gallery verification selectors.
- [x] Declarative verification, bundle/playground builds and 33 native WASM
      composition cases pass. All 18 playground/gallery template renders have
      zero diagnostics. Source and built attribute inventories match; browser
      reruns remain pending under restricted session permissions (see below).

## Compose cem-icon-button from cem-icon

- [x] Embed the canonical icon and relay its public attributes and payload;
      retain button/link activation, disabled state and expansion. Source aliases
      were subsequently removed in favor of image (see above).
- [x] Fixture: cover nested icon rendering, live attribute forwarding, source
      precedence, label/payload, sizes and both control modes in colocated stories.
- [x] Document embedding and attribute ownership in the declaration, contract,
      playground and gallery. Gallery descriptions explain only the current API.
- [x] Verify declarative architecture, bundle and playground builds, attribute
      inventories, and bundle dependency paths. All 33 native WASM composition
      cases and 12 playground/gallery template renders pass without diagnostics.
- [ ] Rerun focused icon/icon-button browser stories and source/installed-package
      playground and bundle checks when browser execution is permitted. The first
      browser run passed 12 of 15 tests; its failures exposed empty-source
      forwarding and a style assertion that included the nested declaration.
      Both are corrected; native rendering confirms empty-source preservation.
      Browser rerun is pending after the session changed to restricted permissions.
      Command: `yarn nx run cem-elements:test -- packages/cem-components/src/components/cem-icon-button/cem-icon-button.stories.ts packages/cem-components/src/components/cem-icon/cem-icon.stories.ts`.

## Convert five icon gallery demos to CEM-ML

- [x] Convert inline SVG, hidden presence, sizes, Material Icons, and module
      image samples with the native tabular formatter; preserve all demo metadata.
- [x] Verify source values, attribute presence, and embedded module-loader
      semantics through native round trips, then rebuild the gallery.
      All five pass the demo WASM rendering API with zero diagnostics and
      preserve rendered elements, attributes, and meaningful text. Browser
      interaction verification remains subject to the session restriction.
- [ ] Formatter follow-up: repeated tabular CEM formatting adds blank lines to
      text-bearing nodes. Keep the first native conversion output for this task;
      diagnose formatter idempotence separately with a minimal native fixture.

## Remove fixture-only gallery IDs

- [x] Remove test-only component IDs across all nine galleries; preserve native
      form owners, help/error associations, page styling, and fragment destinations.
- [x] Fixture: replace gallery ID selectors with existing public attributes
      scoped to demos; verify removed IDs are not referenced and rebuild galleries.
      Audited all nine source/built galleries: removed 62 example IDs and 420
      test-only sample markers. Native form/help/error/fragment references and
      all other example content are preserved. Declarative and script syntax
      validation pass. Remaining command-example ID selectors belong to property
      playgrounds, whose markup is unchanged by this gallery cleanup.
- [ ] Run source/installed-package browser checks when execution is available.
- [x] Record the gallery authoring principle in CLAUDE.md and the demo protocol.

## Syntax conversion helper and icon example

- [x] Add a project syntax conversion helper and route explicit conversion
      requests through dedicated context in CLAUDE.md; require native tabular formatting.
- [x] Convert the Decorative and accessible glyphs demo to CEM-ML and verify
      that its five icon cases retain their attributes and source values.
      Native XML round trip and tabular idempotence pass; gallery rebuild passes.
      The helper also passed skill validation and query-module formatting.
      Browser verification remains pending under the existing session restriction.

## Gallery demo headings and descriptions

- [x] Move individual icon case headings/descriptions to demo attributes, remove
      single-demo sections, and preserve IDs, legacy cases, and resource links.
- [x] Update gallery verification selectors for the demo-owned case IDs and
      record the convention in the component development protocol.

## Organize cem-icon gallery examples

- [x] Replace the mixed source/naming/size/content example with focused label,
      accessible-name, inline/external image, and visibility sections. Preserve
      the seven legacy groups and five-theme size/direction/source matrix.
- [x] Fixture: verify separate inline and external image examples, their source
      URLs, inline image decoding, and hidden presence in source/package galleries.
- [ ] Run browser gallery verification when session permissions allow it.

## Fix duplicate Font Awesome glyphs in cem-icon

- [x] Remove the source-derived wrapper class, which leaked `fa-*` classes and
      generated a second glyph in the surrounding text font.
- [x] Fixture: assert Font Awesome classes occur only on the glyph in icon
      stories, and verify the gallery wrapper has no generated `::before` content.
- [ ] Run icon stories and source/installed-package galleries when browser
      execution is available; current session restrictions still apply.

## Align cem-icon with the label principle

- [x] Document visible label fallback, payload override, and separate aria-label naming.
- [x] Fixture: cover label/payload combinations, live independent naming updates,
      rich links, decorative glyphs, and empty sources in stories and source/package previews.
- [x] Preserve all seven legacy gallery groups, migrate accessible names, and expose both labels in the playground.
- [x] Remove the `name` glyph alias and playground control; migrate glyphs to
      `image` and the recycling example to visible `label` text.
- [x] Fixture: update source/live-attribute coverage to select glyphs through
      `image` and restore `circle` when it is removed.
- [ ] Rerun icon stories after removing `name` (browser execution requires host permissions).
- [x] Before removing `name`, verify all six icon stories, declarative validation, package contents,
      style contract, and lint (zero errors, 48 existing warnings).
- [ ] Complete source/installed-package playground verification, then commit and push.
      The prior playground run was interrupted by the session environment change;
      `.git` is now read-only and network access is restricted. Package checks
      passed before that change; the rerun hit `spawnSync npm EPERM`. Stop the
      restricted reruns and resume these checks in a browser-capable environment
      with Git write access.
- [ ] Next: align `cem-icon-button` with the shared label principle; its current
      accessibility-only `label` remains an inconsistency outside this change.

## Completed: Complete cem-icon legacy examples

- [x] Audit the seven use-case groups in custom-element-dist 0.0.39 against
      the canonical gallery; restore all source collections, direction/size,
      module-resolved and external images, resource links, and inherited colors.
- [x] Fixture: verify Font Awesome families, color inheritance, and all legacy
      gallery groups, including the packaged import-map image asset.
- [x] Record the versioned comparison in the icon contract and demo protocol;
      verify stories, source/package galleries and package contents, commit and push.

The versioned audit covers all seven legacy groups, all 51 literal image values,
and the dynamically module-resolved logo. Five icon stories pass. Full source
and isolated-package playground/gallery/bundle checks pass, including exact
source collections, glyph sizes, inherited colors, and the shipped logo asset.
Package verification passes with 130 files; lint has no errors and 48 existing
warnings. The component implementation already supports these use cases.

## Completed: Introduce canonical cem-icon

- [x] Decision: approve theme-owned icon sizing proposed in
      `packages/cem-components/docs/components-css-exceptions.md` (CEM-CSS-003).
- [x] Migrate the frozen registry icon to a canonical XHTML declaration; retain
      `name` and decorative/labeled accessibility, and support legacy `image`
      sources, size, direction, and projected content.
- [x] Fixture: add colocated stories for source classification, live attribute
      changes, accessible naming, slot content, sizes, hidden state, and themes.
- [x] Add the property playground and mandatory Full examples and variation
      matrix, description and related-component links; register the gallery in
      the component index and ship both pages in the package.
- [x] Update migration inventory, consumers, documentation, and bundle checks;
      assess reuse by cem-icon-button while preserving its public behavior.
- [x] Verify stories, declarative architecture, source/installed playgrounds,
      package, style contract and lint; commit and push.

All three icon stories and nine legacy primitive tests pass. Source and isolated
package playgrounds, all eight galleries, bundle loading, package contents,
declarative architecture, catalog, theme/style contract, and Material parity
pass. Lint has no errors and 48 existing warnings. The approved standalone
sizes are theme-owned; icon-button retains its existing glyph rendering and
size contract. No runtime or application JavaScript behavior was added.

## Completed: Rename action loading to pending

- [x] Rename the canonical `cem-action` attribute and action state to `pending`;
      update theme/component docs, examples, playgrounds, stories and state coverage.
- [x] Verify all 17 action stories, declarative architecture, state matrix and catalog.
- [x] Verify source and installed-package playgrounds and published package contents.

All 17 action stories, catalog/state-matrix checks, package build and source/installed
playgrounds pass. The public action attribute is now `pending="true"`.

## Completed: Extra-light red destructive loading stripe

- [x] Use the palette extra-light red (`--cem-color-red-xl`, `#ffb4ab`) for
      the destructive light contrast outline; preserve dark mode and normal fills.
- [x] Fixture: verify the exact extra-light endpoint and retained motion,
      then run browser/package checks, commit and push.

All 17 action stories pass, plus source/installed-package playgrounds with
visible motion, reduced-motion and forced-colors checks. Package, style, material
parity and lint pass (no errors; 48 existing warnings).

## Completed: Brighter light contrast loading outlines

- [x] Decision: use 40% white for light contrast outline stripes only; preserve
      ordinary fills and dark mode (user approved option 1).
- [x] Fixture: every light contrast intent uses the brighter endpoint, with
      stripe separation above 3:1; normal fills and dark contours retain their colors.
- [x] Update theme recipes and documentation, verify, commit and push.

All 17 action stories pass, including the explicit light-outline exception and
restoration of ordinary fills after switching themes. Source and installed-package
playgrounds pass visible-frame motion, reduced-motion and forced-colors checks.
Package, style, material parity and lint pass (48 existing warnings, no errors).

## Completed: Sharper loading gradient and light contrast motion

- [x] Apply the approved 20% hold / 60% blend / 20% hold pattern to all
      action loading gradients and theme previews, preserving endpoints/timing.
- [x] Fixture: verify solid color holds and actual painted motion for every
      intent in light/dark contrast playgrounds; investigate reported static light mode.
- [x] Verify browser/package checks, commit and push.

Chromium showed moving pixels for all five intents in both contrast modes before
and after the change; a stopped light-mode animation was not reproduced. The new
solid color holds make stripe boundaries sharper without changing the palette,
text contrast, timing or reduced-motion behavior. All 17 action stories and
source/installed-package playground checks pass, including actual frame comparisons,
plus package, style, material parity and lint (no errors; 48 existing warnings).

## Completed: Stronger loading stripe contrast

- [x] Fixture: assert stripe separation above 1.8:1 and text contrast of at least
      4.5:1 for every intent in light/dark, retaining normal/contrast parity.
- [x] Add coherent theme-owned stripe endpoints across intents, update action
      and theme preview, verify accessibility fallbacks, commit and push.

Measured light/dark stripe separation: 1.83:1–2.72:1; minimum text contrast:
4.58:1. All 17 action stories pass, including gradient midpoint contrast and
normal/contrast parity. Source/package playgrounds, theme preview, package, style,
material parity and declarative checks pass. Lint has no errors (48 existing
warnings). Native system-palette stripe endpoints retain their previous behavior.

## Completed: Intent-colored pending animation

- [x] Fixture: verify every action intent has matching loading colors and motion
      in light/contrast-light and dark/contrast-dark, including live switching.
- [x] Update theme recipes, canonical action and generated preview; verify, commit
      and push. Preserve contrast contour geometry and accessibility fallbacks.

Verification: all 17 canonical action stories; source and installed-package
playgrounds (including reduced motion and forced colors); package, style contract,
material parity and declarative architecture gates. Existing per-control normal
fill color overrides remain covered.

## Completed: Shared demo property layout

- [x] Put property fieldsets in a wrapping flex container and stack their options
      vertically using shared `cem-components/demo.css` classes in every property
      playground; include the shared stylesheet in published packages.

## Completed: Canonical field controls

- [x] Fixture: both fields remain borderless at rest, hover, focus, readonly,
      invalid, busy and disabled, including themes and forced colors; hover
      retains a visible theme-owned indicator.

- [x] Audit canonical prototypes for both fields; identify the form-reset contract gap.
- [x] Decision: approve an opt-in shared form-control capability in `cem-elements`
      before resuming migration. See [the migration gate](field-controls-migration-gate.md).
      User approved the recommended shared capability.
- [x] Fixture: prove shared form reset, one submission owner, native validity,
      fieldset disabling and state restoration before component migration.
- [x] Fixture: preserve native Enter submission, default-submit cancellation,
      disabled submitters and multiple-control suppression in the shared capability.
- [x] Implement and verify the shared form-control capability.
- [x] Migrate `cem-field` and `cem-text-field` with explicit boolean-presence
      semantics, full attribute coverage and canonical source playgrounds.
- [x] Fixture: verify field attributes, boolean presence, input/value events, form reset,
      retained busy state, scoped indicator paint and source/package playgrounds.
- [x] Compare remaining legacy state failures after the migration.

Ten canonical field stories and six shared form/generic-input stories pass.
Primitive tests pass (9/9). Legacy state tests retain the 4-pass/15-fail baseline;
workflow tests pass 13/14, with the remaining failure at legacy checkbox required
presence. Field assertions pass. Source/package playgrounds, package, style,
state-matrix and forced-color checks pass. See the
[field contract](../packages/cem-components/docs/field-controls-contract.md).

## Completed: Required component galleries

- [x] Require a full examples and variation matrix gallery for every component,
      linked from its property playground and shipped in the package.
- [x] Add missing galleries for all six introduced canonical components.
- [x] Fixture: verify select outline styles compile without diagnostics and the
      gallery applies the outline appearance token.
- [x] Fixture: verify every canonical playground links its gallery, renders real
      examples and theme/state variations, and works from source and package archives.

All seven canonical galleries pass source and isolated-package browser checks.
Package verification passes (119 packed files), and all 12 select stories pass.
The select outline selector now stays within the scoped-CSS specificity limit.

## Completed: Component demo index

- [x] Define `packages/cem-components/index.html` as the required component listing
      in the demo protocol, with exact component names linking to galleries.
- [x] List all seven canonical components alphabetically and verify their labels
      and local gallery targets.

## Completed: Gallery introductions and index destinations

- [x] Point component index entries directly to their full examples galleries.
- [x] Require and add a brief component description and relationships paragraph
      immediately after each gallery heading, with links to related galleries.
- [x] Verify all seven index entries, introduction placement, related links,
      and generated gallery pages.

## Completed: Icon-button legacy link parity

- [x] Decision: extend `cem-icon-button` to cover legacy `cem-icon-link`, including navigation.
- [x] Fixture: icon source selection, empty/removal and precedence, native link
      activation, disabled navigation, button/link switching, slot naming and state paint.
- [x] Implement declarative link mode, icon renderers and legacy kind compatibility.
- [x] Update the property playground, full gallery, contract and legacy transition notice.
- [x] Verify stories and source/package demos, then commit and push.

Eight icon-button stories pass, including five-theme link state paint.
Source/package playgrounds, package, style and parity checks pass.
Lint passes with 48 existing warnings and no errors.

## Completed: Canonical textarea

- [x] Migrate `cem-textarea` using the shared form-control capability, with scoped
      indicator CSS, a canonical source/package playground and its required gallery.
- [x] Fixture: cover multiline editing, reset, boolean presence, validation,
      retained focus/selection and forced colors; compare remaining legacy failures.

Eight textarea stories pass, covering multiline editing and whitespace, native
constraints, boolean presence, external form ownership, disabled fieldsets,
reset, Enter behavior, retained selection, and all five theme modes. Source and
isolated-package playgrounds, all nine galleries, bundle loading, package
contents, declarative architecture, catalog/state matrix, style, forced colors,
and Material parity pass. Lint has no errors and 48 existing warnings. No shared
runtime change was needed. The final multiline property editor was also checked
from source and isolated package archives after its update.

Legacy comparison: states remain 4 passing / 15 failing; workflows remain
13 passing / 1 failing (checkbox required presence). Textarea workflows pass.
The primitive suite passes all nine tests in an isolated rerun. During concurrent
package work, one run failed to render eight primitive fixtures; the rerun logged
Vite dependency reoptimization. A packaged-page preview also timed out once
during concurrent package work, then passed in the focused source/archive run.
The cause is not established; keep browser startup investigation open and avoid
rebuilding shared package artifacts during archive verification.

## Completed: Native radio selection feedback

- [x] Decision: radios use their native dot for selection; checkbox and switch
      styling stays unchanged. Radio focus, invalid and pending outlines follow
      the circular native control (user approved).
- [x] Fixture: checked radios have no extra selection shadow; circular focus,
      invalid and pending feedback remains visible across themes and forced colors.
- [x] Update shared styles and contracts, verify and commit/push.

## Completed: Canonical icon-button and menu-item migration

- [x] Migrate `cem-icon-button` and `cem-menu-item` into per-component XHTML,
      using explicit disabled presence and removing their legacy ownership.
- [x] Fixture: colocated stories for every implemented attribute, native
      pointer/keyboard activation and disabled behavior; companion playgrounds.
- [x] Compare the complete legacy state failure inventory after migration;
      verify source/package output, commit and push.

Verification and the remaining 4-pass/15-fail legacy state inventory are recorded
in [command component migration](command-component-migration.md).

## Completed: Batch renders after event bursts

- [x] Decision: let bubbling and related synchronous events finish before one
      asynchronous render of their accumulated state (user instruction).
- [x] Fixture: nested/bubbling events, microtask follow-ups, host attributes and
      behavior updates produce one render; retain latest event metadata and
      make `whenRenderSettled` wait for the queued work.
- [x] Fixture: disconnected queued instances do not render; reconnected
      instances render current state and stale in-flight results stay rejected.
- [x] Update runtime scheduling and the repeated-click contract, verify and
      commit/push. Continue owner migration after this shared capability.

The [legacy investigation](legacy-action-state-investigation.md) remains the
owner-failure baseline. Event payloads must be captured while `currentTarget` is
available; batching concerns rendering, not discarding later events.

All 348 runtime browser stories and 585 unit tests pass. Source/package
playgrounds pass; the legacy state baseline is unchanged. See
[event render batching](event-render-batching.md).

## Completed: Release XHTML bundle and source context

- [x] Fixture: characterize relative module URLs in individual and combined
      fragment sources, including retained `xml:base`.
- [x] Decision: preserve original source-base metadata in the shared loader
      (user approved the recommended approach).
- [x] Fixture: native source-base chain validation and browser fragment parity,
      invalid metadata, source identity, dependencies and styles.

The generated `components.xhtml` bundle and its playground now ship alongside
canonical source. The shared loader preserves original resource bases through
`xml:base`, with native validation. Source and isolated-package checks pass;
see [delivery evidence](component-bundle-source-context.md).

## Completed: Component galleries and module-URL startup settlement

- [x] Decision: batch sibling module-URL results before rendering, matching
      the DOM fallback (approved by the user on 2026-09-28).
- [x] Fixture: reproduce partial URL publication with controlled resolver
      completion; batch results while retaining stale-render/disconnection
      guards, successful siblings and failure diagnostics.
- [x] Verify the owned action attribute gallery and select companion from
      source and isolated packages, including concurrent gallery startup.

The action gallery covers all 20 explicitly implemented attributes. The select
companion and listbox scrolling fix are complete. Verification and remaining
scope are recorded in the [completion log](archive/todo-completed-2026-09-27.md#component-galleries-and-module-url-settlement)
and [startup review](browser-stabilization-review.tmp.md#gallery-module-url-settlement-recurrence).
The release XHTML bundle is also complete; remaining migration work is below.

## Completed: Selected actions and state-specific input indicators

- [x] Replace blanket input shadows with state-specific native-control feedback; revise shared/theme contracts.
- [x] Restore controlled selected action state, zebra and accessible selectable-button semantics.
- [x] Replace Expanded in the playground with Selected and add a container-owned choice example.
- [x] Fixture: resting inputs/labels, focus, checked/mixed, invalid, pending and forced-colors feedback.
    - [x] Verify native control CSS states directly across theme modes so legacy
          boolean-binding failures cannot hide feedback regressions; preserve
          the separate component behavior assertions and baseline repair task.
- [x] Fixture: selected action presence/removal, no automatic toggling, accessibility and combined states across modes.
- [x] Verify source/package playgrounds, theme and component gates; record completion and commit/push.

Browser and package verification now runs in the unrestricted session. Native
CSS checks cover all ten controls across five theme modes and forced colors;
source and isolated-package playgrounds pass. The follow-up fixes missing
pending feedback on date/time inputs and scopes the icon-spacing fixture to its
intended sample. No theme token definitions were removed.

The legacy state suite remains 3 passing / 18 failing, with exactly the same
failed test names as the documented baseline. Its boolean-binding failures
still block later component assertions; direct CSS coverage does not close that
separate repair task. See [completion evidence](archive/todo-completed-2026-09-27.md#selected-actions-and-state-specific-input-indicators).

## Completed: Action size roles and content compositions

- [x] Document size roles, independent content layout, native navigation semantics and compact-only hit expansion in the dimensions specification.
- [x] Add the theme-owned xx-large minimum profile and allow action labels to wrap within constrained layouts.
- [x] Fixture: cover xx-large inheritance/overrides, compact-only hit borders and growing image/title content in action stories.
- [x] Add working inline, text, icon, image-choice and hero-choice playground samples; verify responsive source and installed-package pages.
- [x] Run theme and component checks and record evidence.

Verification: 15 action stories, theme generation and fresh Phase 13 checks,
component package/style/declarative checks, typecheck, lint and responsive
source/isolated-package playgrounds pass. See [completion evidence](archive/todo-completed-2026-09-27.md#action-size-roles-and-content-compositions).

## Completed: Playground undefined size and contrast contract

- [x] Fixture: Size radios expose Undefined, small, medium, large and x-large; Undefined removes the attribute in source and packaged playgrounds.
- [x] Resolve pending-gradient versus contrast-outline policy: user selected animated loading zebra with collapsed fills.
- [x] Generate contrast action fills/ink and implement zebra state feedback from the canonical theme.
- [x] Fixture: verify contrast-light/dark default, hover, active, focus, disabled and loading, including theme switching and accessible state feedback.

Verification: 14 action stories, theme generator/manifest checks, component
package/style/declarative checks, typecheck, lint and source/isolated-package
playgrounds pass. See [completion evidence](archive/todo-completed-2026-09-27.md#undefined-size-and-contrast-action-contours).

## Completed: Action dimensions and compact hit areas

- [x] Document optional theme size profiles and conditional native hit areas in the dimensions, controls and coupling specifications.
- [x] Generate reusable size profiles and preserve their public token exports.
- [x] Apply inherited/explicit sizes to action and reserve compact hit areas in layout.
- [x] Fixture: verify size inheritance/removal, both-axis targets, compact edge activation, disabled/form behavior, spacing, stretching, bends and pending/focus paint.
- [x] Add size radios and layout examples to the property playground; verify source and installed-package pages.
- [x] Finish theme generator verification, record evidence, commit and push.

Verification: 13 action browser stories, source/isolated-package playgrounds,
component package/style checks and typecheck pass. Lint passes with existing
non-null assertion warnings. Theme generation, token exports, manifest validation
and the Phase 13 browser verifier pass. See [completion evidence](archive/todo-completed-2026-09-27.md#action-dimensions-and-compact-hit-areas).

## Completed: Theme switch component and playground integration

- [x] Add a reusable `cem-theme-switch` component with visible Light / Dark /
      Native choices and a separate Contrast toggle. Author a canonical XHTML
      definition with colocated CSF stories, following the shared component
      authoring and public CSS ownership rules.
- [x] Native disables the Contrast toggle; the operating system owns native
      contrast. Light and Dark map to the existing contrast variants.
- [x] Add the shared theme switch to component playground pages, starting with
      the action playground, and make it part of the pattern for future
      playgrounds. Apply the selected theme to the page, including its live
      preview, while preserving the component's current property selections.
- [x] Fixture: verify mouse and keyboard operation, accessible labels and state,
      all supported mode/contrast combinations, retained preview state, and
      source/packaged playground loading through page-level module maps.

### Theme switch integration finding

- [x] Adopt generic nested custom-element ownership: preserve self-rendered
      output while updating authored attributes and child inputs. Components
      that consume those inputs expose subsequent updates through their API.
- [x] Fixture: preserve custom-element output and self-added attributes during
      full renders and worker patches; verify authored input updates/removals
      and nested slice/resource binding ownership.
- [x] Verify source/installed-package playgrounds after the runtime fix, then
      commit and push the theme-switch integration.

Verification: 332 browser stories and 585 runtime unit tests pass; component
package, style, source/installed playground checks, typecheck and lint pass.
See [completion evidence](archive/todo-completed-2026-09-27.md#theme-switch-and-nested-custom-element-ownership).

## Immediate: Destructive pending dark-mode stripes

- [x] Fixture: darken the destructive pending stripe in dark and contrast-dark themes; verify endpoint separation, readable text, animation and browser preview.

## Immediate: Destructive pending stripe contrast

- [x] Fixture: lighten destructive pending stripes through the canonical theme, preserve readable text and motion across modes, and preview in the browser.

## Immediate: Public CSS property ownership

- [x] Record public component CSS property ownership and the mandatory theme/component conflict decision.
- [x] Fixture: action consumes existing intent/state theme tokens directly; verify container inheritance, instance overrides, restoration and state behavior.
- [x] Audit select indicator aliases and popup stacking ownership: adopt existing theme indicators; promote public popup stacking into D4 (user decisions).
- [x] Fixture: generate/export --cem-select-popup-z-index from D4; verify inherited/instance popup stacking and pointer access.
- [x] Fixture: remove redundant select focus/selection color aliases and verify theme overrides across underline/outline, pending, invalid, disabled, focused and expanded states (21 action/select stories pass).
- [x] Preserve all Markdown-generated theme variables; retain select's five existing state/geometry calculations. Record optional unused-code optimization for consumer builds in the roadmap, never CEM release pruning.
- [x] Verify source and packaged playgrounds after theme ownership migration; record evidence.
- [x] Commit and push public CSS ownership and theme adoption changes.

## Immediate: Legacy Demo Case Coverage

- [x] Remove redundant action forced-color gradient/shadow resets and consolidate
      bend-radius binding; verify action states and source/package accessibility fallbacks.

- [x] Fixture: canonical action and select respect native host hidden states,
      initial/live visibility, keyboard exclusion, retained control/state and
      case-insensitive until-found semantics. Document the shared convention,
      remove the legacy invisible demo and verify source/package galleries.

- [x] Align the legacy custom-element package license and all eight Material
      demo footers with the repository MIT license; clarify historical records.
      Verified matching license text, package metadata and footer references.

- [x] Keep the action variations matrix limited to intent and shape; remove its
      disabled column, retain interaction-state examples, and verify both galleries.

- [x] Remove the unsupported action alignment demo and redundant icon-example
      labels; retain the fallback-label example and verify the source/package gallery.

- [x] Fixture: disabled loading actions retain pending animation while native
      disabled semantics block activation; verify all intents, submit workflow,
      loading completion and reduced-motion/forced-colors package previews.

- [x] Fixture: restore canonical pending gradients (45-degree tilt, seamless
      two-second loop) in theme specs, token generation, generator previews and
      cem-action. Verify intent colors, movement across cycles, disabled/loading
      toggles, reduced motion, forced colors, and source/package previews.

- [x] Fixture: action loading transitions once into intent pending colors;
      verify all five intents, live toggling, disabled precedence, stable geometry,
      hover/keyboard-focus replay, and reduced-motion behavior in the playground.

- [x] Reset the native button border for every `cem-action` intent; verify
      action stories and source/package playgrounds retain zebra focus and
      the forced-colors outline.

The local `~/aWork/custom-element/demo/` comparison found that current-gallery
verification is not a one-to-one legacy case audit. In particular, anonymous
whole-file XSLT loading and XSLT selected by `file.xhtml#id` initially lacked
gallery cases; the existing `embedded-xsl` fixture actually contains CEM-ML.

- [ ] Stabilize browser startup waits under parallel Storybook load, after
      DATA-CELL-MATCH-1: the unchanged scoped-CSS and legacy icon-link stories
      intermittently exhaust their short frame/two-second waits, although
      isolated and earlier full runs pass. Prefer declaration/render readiness
      over incidental delays. Also diagnose the source-loaded cell-overrides
      stock sample's initial-warning timeout seen once in the full gallery;
      six focused standalone/source-loaded runs passed without reproducing it.
      Establish its cause before treating it as the same startup issue.
      2026-09-21: scoped-CSS, hex-grid and legacy parity helpers now wait on
      rendered output/style/image conditions within the 30-second story budget,
      replacing frame-count and two-second cutoffs. All 29 cases in the five
      affected story files pass together and in the full parallel suite. The
      table's invalid-input alert also needed its existing ten-second interaction
      budget under parallel load. Final full result: 179 passed, three separate
      CEM-QL/local-storage failures tracked below. Stock-warning checks pass in
      both focused gallery modes; its earlier intermittent cause remains open.
      After native storage migration, all 190 browser stories and the full
      gallery (28 standalone pages, 34 source-loaded documents) pass. The
      original intermittent stock-warning cause still needs investigation.
      Follow-up 2026-09-21: all 198 current stories and 24 concurrent startup
      probes (12 gallery-helper / 12 real demo-component, with three declaration
      release schedules) pass. No lost markers, missing mounts or diagnostics
      reproduce. The user selected [continued investigation](browser-stabilization-review.tmp.md#startup-follow-up-decision-continue-investigation);
      keep this item active before the authored-sample coverage inventory.
      Sequencing update 2026-09-23: the user approved beginning that inventory
      while historical timeout attribution remains open on traced recurrence.
      This supersedes the earlier stabilization-first order, not its open
      investigations. The inventory follow-up below records the accepted work.
      Follow-up 2026-09-24: the cell-overrides audit and real source-harness
      component pass their focused stories and all 31 standalone / 37
      source-loaded gallery documents. The referrer matrix's scalar-URL frame
      timeout recurred in both final aggregate runs and a dedicated full
      Storybook rerun (219/220 each), temporarily blocking aggregate sign-off.
      The specific referrer fixture below now attributes and corrects it.
  - [ ] Audit remaining frame-count readiness helpers and aggregate-count
        predicates in demo stories. Check the authored instance inventory and
        available lifecycle signals before migrating each affected fixture;
        keep historical stock-timeout attribution separate from fixture fixes.
        Prioritize the newly observed failures: external-src declaration loading
        waits 120 frames for its first button; NPM-version samples wait 200
        frames for the default selection; location samples wait 180 frames for
        both readers. Capture declaration/render/worker state before attributing
        these failures or changing waits. Normal coverage passes, but that alone
        does not establish their failure cause under combined load.
        Progress 2026-09-22: opt-in lifecycle and frame-wait traces reproduce
        external-src's 120-frame timeout during pending rendering after clean
        declaration settlement. Its render settles 49 ms later without errors;
        later assertions remain unverified. Propose lifecycle-based readiness
        within the existing story limit. NPM/location pass this repeat and still
        require captured failure state before attribution; see the attribution
        section linked above. No waits changed in this investigation.
        The hook-eligibility validation also observes two declaration-source
        retry stories failing their default 1,000 ms `waitFor` after declaration
        settlement. Their four-story isolated run and a subsequent full suite
        pass; capture render/worker state on recurrence before attributing the
        failure or migrating these waits. Do not change retry/cache behavior
        based on a missing paragraph at a polling deadline.
        Follow-up 2026-09-22: external-src and retry fixtures now await existing
        lifecycle settlement (completed item below). The broader audit remains
        open for NPM/location failure attribution and other aggregate/frame
        predicates; the thirteen-file inventory is recorded in the
        [readiness audit](browser-stabilization-review.tmp.md#declaration-and-retry-readiness-audit).
        Follow-up 2026-09-23: focused overlapping load reproduces NPM's
        premature startup wait twice. Awaiting existing initial settlement
        before the unchanged HTTP predicates passes the same workload, both
        204-test full suites and all 160 stock probes. Location passes under
        focused load; its failure and the broader inventory remain open. See
        [NPM/location evidence](browser-stabilization-review.tmp.md#npm-and-location-readiness-under-overlapping-load).
    - [ ] Capture HTTP's newly observed 300-frame initial article-count
          timeout with declaration/render/worker tracing before choosing its
          correction. It failed once in the storage diagnostic workload;
          that run did not observe HTTP lifecycle state or verify its later
          resource assertions.
          Follow-up 2026-09-23: two traced 16-file runs and a focused five-file
          startup workload pass 26/26, 26/26 and 15/15, with 96/96 stock probes.
          The timeout remains unattributed; no wait or runtime change is made.
          Permanent opt-in tracing records lifecycle state and per-card article
          counts on startup success/failure, rethrowing the original error.
          Resume attribution on a traced recurrence. The native storage
          diagnostic follow-up below is complete; see
          [HTTP observation](browser-stabilization-review.tmp.md#http-startup-observation-without-a-reproduced-failure).
      - [ ] On a traced HTTP failure, distinguish declaration/library startup,
            current rendering and future HTTP completion before changing the
            fixture. If startup polling is premature, use existing lifecycle
            settlement before checking each authored instance; preserve the
            resource assertions and 30-second story deadline.
## Declarative UI Architecture Correction

### Remaining declarative UI migration

- [x] Fixture: restore the action preview's native hover, active, keyboard-focus
      and disabled states with CEM tokens. Verify native disabled suppression,
      stable geometry, and state paint in source/dist/installed-package previews.
      Check each zebra stripe against the focused element's current colors;
      inherited root-resolved ring recipes must not erase the focus stripe.
    - [x] Fixture: give each action sample a consistent theme intent and add
          a matrix covering all five intents and sharp/smooth/round/disabled
          controls. Verify each intent's actual default/hover/active/disabled paint.
    - [x] Fixture: retain runtime host-attribute counts during legacy template
          conversion instead of folding them against the static source document.
          Preserve static document count/sum evaluation and verify native lowering
          before rebuilding WASM and checking disabled browser behavior.
      Validation: all 74 native converter tests and the custom-element lint gate
      pass. Source, dist and installed-package checks cover nine cards / 52
      buttons, all five intents, native input states, and zebra across five theme
      modes. DevTools confirms keyboard focus and native disabled controls.
      See [state and variation evidence](archive/todo-completed-2026-09-27.md#action-preview-states-variations-and-zebra).

- [x] Fixture: verify the legacy action demo's actual sharp, smooth and round
      corner radii against generated CEM Shape/Controls tokens, including live
      Bend changes and source/dist/installed-package theme delivery. Keep styles
      declaration-owned and preserve page-level module-map loading.
      Checks cover multiline shape overrides and compact/forgiving sizes.
      See [curvature evidence](archive/todo-completed-2026-09-27.md#action-theme-curvature).

- [ ] Migrate every legacy `cem-components` member into its own
      `src/components/<cem-tag>/<cem-tag>.xhtml` folder with embedded,
      once-per-declaration scope-contract CEM-token `<style>` and colocated
      CSF Next `<cem-tag>.stories.ts` `play` tests, moving missing reusable behavior into
      `cem-elements`, until both migration targets are zero.
    - [x] Fixture: a pointer click on button content followed by a Space click
          keeps the slice value but refreshes native target metadata; confirm
          that holding Space does not publish another click.
    - [x] Fixture: isolate the icon-button/menu-item disabled bindings in the
          unchanged state suite, compare failed test names, and characterize
          repeated click payloads before changing the active-state contract.
    - [ ] Investigate the legacy component suite failures before claiming a
          green package gate: the cutover trial reports 80 passing / 49 failing;
          empty disabled state and collapsible navigation failures reproduce
          with committed legacy sources. Establish the remaining baseline and
          repair shared semantics or migrate owners without weakening tests.
          CSS closure comparison (2026-09-27): rebuilt commit `5f8e7172` and
          the native-CSS default both produce 81 passing / 48 failing tests,
          with the same 48 test names. No new component failures; the package
          gate remains red. Rebuild `cem-elements` before comparing this suite,
          because these tests import its packaged output.
          Follow-up 2026-09-27: the unchanged suite again reports 81 passing /
          48 failing. The [failure inventory](cem-components-baseline-2026-09-27.md)
          records each failed test, the boolean-presence mismatch, and the
          separate datepicker template-brace defect. Owner repairs remain open.
        - [x] Fixture: pin native template attribute-presence semantics for
              absent, empty, `"false"`, and `"true"` host values. Verify the
              explicit presence expression used by canonical components and
              distinguish it from legacy value/truthiness bindings.
        - [x] Fixture: repair the shared substrate harness's required-input
              declaration using explicit attribute presence. Preserve native
              form-validity/reset assertions and verify absent, empty,
              `"false"`, and `"true"` values plus live removal without replacing
              the native input. Compare the full legacy failure inventory.
              The full harness gate passes. The aggregate suite is now
              82 passing / 47 failing, with only this failure removed and no
              new failing test names. See the
              [repair evidence](archive/todo-completed-2026-09-27.md#shared-harness-required-input-repair).
    - [x] Fixture: load the legacy material action demo through page-level
          module maps and repository-owned runtime/demo modules, matching the
          `cem-elements` demo convention. Verify source and packaged asset
          loading and the IDE preview URL through DevTools when connected.
          Review the archive inventory against native CSS runtime additions,
          excluding browser-story adapters from the vendored package.
          Validation: source, dist and clean installed-package browser checks
          pass, as does the package lint target and its dependencies. DevTools
          verifies all seven cards, 29 buttons, successful local JS/WASM loads,
          no console errors, and the Bend interaction at the IDE preview URL.
          See [closure evidence](archive/todo-completed-2026-09-27.md#action-demo-local-preview).
        - [x] Fixture: preserve the adapter smoke test's native declaration and
              implicit-instance scope checks across CSS emitter whitespace,
              retaining all computed-style and isolation assertions.
    - [x] Fixture: verify canonical action loading in workflow/demo and Site
          search/interactive consumers, including packaged XHTML asset delivery.
    - [x] Fixture: cover explicit submit/reset, required-input validation,
          submitter name/value and form overrides, external form ownership,
          disabled suppression and cancellation, while preserving default
          non-submitting command behavior.
    - [x] Fixture: migrate action unit coverage to colocated CSF Next plays,
          retaining exact state reflection, slot/fallback labels, native click,
          keyboard/disabled behavior, node identity, hover/active token pairs
          and declaration-owned style installation.
    - [x] Remove the legacy action registry member/global styles, migrate
          affected consumers to canonical XHTML, and update package/catalog/
          style/state evidence before reducing the migration inventory.
          Completed: 2 canonical / 47 legacy components. See
          [cutover evidence](archive/todo-completed-2026-09-27.md#canonical-action-cutover).
- [ ] Investigate runtime suite instability: the action-cutover full run passed
      311/312 stories; `Payload Css Readiness And Hydration` expected one style
      but observed zero. Its complete 12-story file passed on isolated rerun.
      Preserve the readiness assertion and reproduce before changing behavior.
- [x] Mark the old Material action page as legacy, link its canonical CEM
      component, and document the same transition step for other Material pages.
- [x] Fixture: a held native mouse click on a select option keeps combobox
      focus and the popup open until release, then commits once; outside clicks
      close the popup without taking focus back. Verify every
      action playground choice with a real press/release delay.
- [x] Fixture: action playground enumerated options use visible, labeled radio
      groups; verify initial selection, mouse and keyboard changes, exclusivity
      and retained selection after other property edits in source/package previews.
- [x] Implement the accepted [component development pattern](component-development-pattern.md)
      first for `cem-action`, then apply it to `cem-select` and future components.
    - [x] Fixture: event bindings read the live string value of form-associated
          custom controls through `$target.value`, value aliases, defaults and
          serialized event targets; retain native input behavior and reject
          non-form/object-valued targets.
    - [x] Fixture: companion property form drives one canonical action instance
          across intent, bend, label, type, disabled, loading and expanded;
          native hover/active/keyboard focus remain usable.
    - [x] Fixture: ship the full action gallery and its icon declaration with
          the component playground, using native import-map rewriting; verify
          isolated archive loading without the legacy package installed.
    - [x] Fixture: source-only view fetches the canonical XHTML without executing
          another declaration; link the full demo and verify IDE/module-map and
          packaged preview loading. Update the legacy Material page to link
          the new playground when available.
    - [x] Fixture: route the action playground gallery link to the owned
          cem-components gallery in source and isolated-package previews.
    - [x] Fixture: cover every explicitly implemented action attribute in the
          owned gallery, including size, selected/selectable and native form
          overrides; verify inventory and behavior in source/package previews.
    - [x] Fixture: apply the companion property playground/source-view pattern
          to `cem-select`, including source and isolated package preview checks.
        - [x] Fixture: keep the page scroll position when persistent select
              examples mount; reveal active options within their own listbox
              during initial rendering and keyboard navigation.
    - [x] Fixture: characterize individual/bundled resource bases; record the
          [source-context decision](component-bundle-source-context.md) above.
    - [x] Fixture: generate the release XHTML bundle through the CEM AST pipeline
          and compare individual versus `#ID` loading, relative dependencies,
          style ownership and duplicate registration; ship source and playground.
- [ ] Migrate remaining Studio and Site visible DOM construction, UI listeners,
      and state projection to XHTML/CEM-ML, retaining JavaScript only for non-UI
      services and host adapters.

## Deferred Roadmap Work

The Edge/SSR host fixtures belong to Phase 3.5 after the browser substrate is
stable. Moving `@epa-wg/custom-element` into the monorepo and deciding final
legacy XSLT preservation belong to Phase 3.6. Swift/Xcode plus Kotlin/Compose
compile gates remain Phase 8. Live Figma UI Kit and prototype work is deferred
until final [Phases 10 and 11](../roadmap.md#phase-10---figma-ui-kit). Their
checklists live only in the roadmap. Finish and verify `cem-elements` and the
declarative `cem-components` migration first, then the remaining non-Figma phases
through Phase 9 release governance.
