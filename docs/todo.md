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

Implementation checkpoint (2026-10-05): retained syntax/AST identity, lexical
capture, unary construction, explicit bounded resolution, supported schema
reuse/validation consumers, source codecs and native query ingress are implemented.
This does not close native datatype/function composition, specialized schema and
namespace property selection. Native CLI, low-level WASM and Node/browser worker
clients now support explicit host-owned replacement-grant setup.
XML attribute expression recognition, enclosed child override syntax, public
query/transport contracts and `cem-element` ID projection remain separate decisions
or later consumer work. Their actionable items and verification scenarios remain
below or in the roadmap; the implementation is not universal consumer support.

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
- [x] Complete reference expression-slot integration with retained expression
      artifacts, source-position bindings, caller-supplied runtime context, and
      typed attribute slots. Declaration metadata alone does not implement
      runtime evaluation or specialized scope linkage.
- [x] Specify effective scope-schema policy for mandatory, warning, and ignore
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

Historical audit baseline (2026-10-04, superseded by the closure below):
the initial implementation retains standalone
CEM/XML references and implements strict unary `#` construction and CEMB v3
graph edges. The legacy `dom:reference` helper keeps its separate preservation
behavior. Saved `context` handles are lexical origins, and optional `targets`
are representation choices. Scope-schema disposition and bound composition
are implemented for explicit unresolved facts. Shared chain evaluation,
per-link runtime application/cardinality, expression artifact linkage, typed attribute slots,
and schema construct reuse remain incomplete. The parser-backed target view now
preserves absent unevaluated targets separately from resolved-empty targets,
including field enumeration. Existing ID-backed name slots are not a `#` resolver.

Foundations at that baseline: schema contract declarations, schema-sourced depth/work
defaults and validated inherited overrides, numeric graph-label validation,
XML query-payload provenance, and neutral/inherited unresolved-link policy.
These do not claim an implemented chain resolver or full scope-reference integration.

Audit closure (2026-10-05): sections 1 and 4 now have implemented contracts
and verified coverage. `reference_resolution.rs` covers bounded traversal,
cycles, repeated/mixed targets, source preservation and per-link dispositions;
`reference_unresolved_policy.rs` and `reference_traversal_policy.rs` cover
schema policy composition. `reference_runtime_resolution.rs` proves independent
`datadom` executions, retained result ownership after host disposal, delayed
readiness, resolved-empty versus unresolved, and unchanged authored targets.
`lexical_scope_handoff.rs` and the QL `schema_declaration_references.rs` cover
source-position bindings, explicit crossings, retained compilation artifacts
and standalone/typed attribute lifecycle consumption. The ML schema declaration
fixtures enforce complete-result cardinality separately from unresolved policy;
partial selections never become an accepted empty or successful count.
The detailed checked actions in sections 2 and 5 record implementation evidence.
Deferred syntax and consumer decisions remain in their own actionable items.

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
- [x] Fixture: verify closed CEM standalone/native attribute and aliased XML
      reference slots through the explicit lifecycle host. Retain delimiter-rich
      query payloads and forward targets; check pending-to-ready execution with
      a counted native capability, independent outcomes and unchanged AST edges.
      Verify malformed query diagnostics appear only after context preparation
      and map to original payload coordinates. XML attributes remain literals.
      Scenarios for later design verification: frame closure captures source
      bindings without executing a capability or requiring an authored root ID.
- [x] Fixture: protect braces inside CEM-QL block/line comments in standalone
      and native attribute slots, including CRLF and comment-looking strings;
      assert native/tree-sitter parity and unterminated comment/slot errors.
      Scenarios for later design verification: query comment syntax cannot close
      host syntax; preserving a payload does not declare its query grammar valid.
- [x] Align native and editor query-island scanners with CEM-QL `/* */` and
      `//` comment boundaries while retaining existing opaque payload support.
- [x] Fixture: extend CEM/XML parsing parity coverage for standalone and
      attribute expression slots, namespace aliases, forward expressions,
      query strings/comments containing delimiters, malformed expressions,
      and source locations. Prove loading and context closure never execute
      target selection or require an authored context-root ID.
- [x] Align tokenizer/event normalization, parser/builders, XML import, lexical
      grammar, and tree-sitter parity with `{#...}` and `<cem:expr>#...</cem:expr>`.
      Retain the reference AST kind and expression artifact/source linkage in
      every supported slot instead of stringifying or inserting target children.
      Completed: existing native attribute/source-map, lexical capture and editor
      fixtures cover the adopted slots; two new closed-slot lifecycle fixtures
      verify delayed capability execution and mapped malformed-query diagnostics
      across CEM standalone/attribute and aliased XML CDATA forms. Native and
      tree-sitter scanners now also protect CEM-QL block/line comments, including
      CRLF, without validating opaque query payloads. XML attribute expression
      recognition remains the separate deferred design action below.
      Verification: 25 tokenizer unit checks, 25 reference/provenance/capture/editor
      fixtures and both new closed-slot lifecycle fixtures pass. The full native
      `cem_ml_transform_cem_ql:test` and `cem_ql:test` Nx targets pass.
- [x] Fixture: retain sibling-position scope transitions in the explicit QL
      lifecycle host. Verify earlier occurrences and the boundary keep their scope,
      following siblings/descendants inherit it, nested overrides restore on exit,
      same-ID vendor owners stay distinct, and crossings require directed grants.
      Reject unknown owners, foreign scope handles, non-sibling boundaries and
      conflicting repeat handoffs at the same retained boundary.
      Scenarios for later design verification: current schema/namespace context
      boundaries can supply an effective scope without repeated root markers;
      loading still performs no evaluation and later declarations cannot rebind earlier uses.
- [x] Fixture: verify sibling handoff on imported XML with semantic comments/
      whitespace and folded reference payloads, plus explicit child overrides.
      Retain earlier compiled artifacts and prove registration never evaluates sources.
      Scenarios for later design verification: parser-frame closure preserves original
      source order and binding identity; specialized import frames can use the same handoff.
- [x] Add the caller-supplied retained-boundary scope handoff under those fixtures.
      Keep specialized parser-frame integration separate and retain source owners.
      Scenarios for later design verification: source order follows retained structural
      siblings, including imported sources, rather than arena IDs or global ID lookup.
      Verification: all 26 native QL schema-reference fixtures pass, including
      three new sibling-default, invalid-boundary and imported-source cases.
      Full `cem_ql:test` and `cem_ml_transform_cem_ql:test` Nx targets pass.
      The broader parser/schema-frame integration actions below remain open.
- [x] Audit the existing parser/schema-frame completion path before automatic
      runtime handoff. `CemSchemaMachine` applies namespace declarations while
      consuming attributes/directives, inherits child contexts, and restores parents
      on close. Schema switches apply in host/wrapping scopes; no-body schema
      switches also update the parent for following siblings. Its outcome retains
      remaining open frames and diagnostics, not a history of completed lexical
      associations. The package QL bridge currently receives a caller-prepared host;
      it does not feed these specialized frame transitions into the retained source.
      Scenarios for later design verification: frame completion does not imply query
      evaluation, and the effective occurrence binding must survive frame closure.
      Verification: all 41 native schema-machine fixtures pass.
- [x] Decide whether lexical namespace/schema binding changes automatically create
      reference relationship boundaries. Adopted the recommended separation on
      2026-10-05: retain source-position lexical snapshots separately; preserve the
      caller's relationship scope unless an explicit boundary is declared. Effective
      schema policies and traversal limits still apply. Existing `register_scope`
      continues to establish an explicit relationship boundary.
      Scenarios for later design verification: changing a namespace default cannot
      accidentally grant a crossing or silently impose a new relationship boundary;
      lexical rebinding and schema policy changes remain observable independently.
- [x] Fixture: verify distinct lexical lifecycle contexts/artifacts within one explicit
      relationship boundary, shared directed grants without granting reverse crossings,
      independent pending/context replacement and effective child traversal policy.
      Verify explicit same-owner boundaries still need grants, foreign handles reject,
      and retained occurrence/target owners never change.
      Scenarios for later design verification: lexical rebinding neither changes the
      permission boundary nor resets traversal accounting or replaces earlier artifacts.
- [x] Add explicit lexical-snapshot registration to the QL retained consumer host,
      separate from `register_scope` relationship-boundary registration. Apply grants
      to relationship identity; keep compilation/context and policy on lexical identity.
      Scenarios for later design verification: frame handoffs can preserve source-position
      environments while callers continue to define permitted relationship crossings.
      Verification: all 28 native schema-reference fixtures pass, including two
      new lexical-context/artifact/grant and effective-policy fixtures. Both stricter
      child work limits and request-wide work limits apply within a shared boundary.
      Full `cem_ql:test` and `cem_ml_transform_cem_ql:test` Nx targets pass.
      Completed parser-frame association capture and automatic handoff remain open below.
- [x] Fixture: share one normalized stream between lexical frame tracking and the
      AST builder. Retain immutable snapshots at standalone/native attribute slots;
      verify later namespace rebinding, host/wrapping/sibling schema defaults and
      restoration after closure, pending schema selectors, original source spans,
      unevaluated targets, and one final diagnostic notification even after repeated EOF.
      Scenarios for later design verification: frame capture precedes the incoming
      event's effects; no second AST, source reparse, query execution or event history is needed.
- [x] Add opt-in lexical snapshot/stream observation to the schema machine under
      those fixtures; forward original normalized events unchanged to the builder.
      Keep retained associations in caller-owned metadata and report final diagnostics.
      Scenarios for later design verification: completed snapshots survive parser
      teardown; stream completion alone does not imply valid schema selection or readiness.
      Verification: three new native capture fixtures, all 41 schema-machine tests,
      and 18 existing node-reference, expression-source and tree-sitter parity tests
      pass. Automatic retained-handle association and QL lifecycle handoff remain open below.
- [x] Fixture: build owner-checked lexical associations directly from builder node
      identities, including standalone and native attribute expressions, rebinding,
      nested restoration, sibling schema switches and pending selectors. Reject an
      equal node ID from another source owner; retain diagnostics after closure.
      Scenarios for later design verification: equal source coordinates cannot alias
      occurrences; capture neither reparses nor evaluates or copies the original AST.
- [x] Add an opt-in scoped-document build using the normalized stream and checked
      original node identities; retain only surviving expression occurrences and
      machine diagnostics. Keep lifecycle context/policy/grant preparation separate.
      Scenarios for later design verification: temporary folded payload nodes never
      escape as associations, and owner lifetime exceeds parser frame lifetime.
      Verification: all five lexical capture fixtures pass, including two new
      owner/slot and schema/namespace restoration cases. The 20 builder, 41 machine,
      and 18 existing reference/source/parity tests pass. Imported-source association
      and lifecycle preparation of QL contexts, policies and grants remain open.
- [x] Fixture: hand captured original expression occurrences to QL through a shared
      AST tree view. Verify lifecycle-selected contexts, independent pending snapshots,
      native general attribute expression hooks, original owners/artifacts,
      existing explicit boundaries/grants, and stricter
      effective policy limits. Reject foreign owners and repeat attachment before
      invoking preparation or changing host state.
      Scenarios for later design verification: parser closure never runs a selector;
      attachment preserves the nearest caller-defined relationship boundary.
- [x] Add an explicit captured-scope lifecycle handoff and a shared-owner tree
      constructor under those fixtures. Let the caller prepare each runtime context
      and effective policy from its saved lexical snapshot; expose resulting handles
      for later readiness updates. Never inherit runtime inputs or grant crossings.
      Scenarios for later design verification: preparation completes before attaching
      scopes, and source targets remain untouched across separate executions.
      Verification: four new handoff fixtures and all 28 existing QL schema-reference
      fixtures pass, including general attribute lifecycle hooks, late-conflict
      preflight, owner/context isolation, directed grants and both limit budgets.
      All 23 focused CEM-ML capture/reference/source/parity tests pass. Full
      `cem_ql:test` and `cem_ml_transform_cem_ql:test` Nx targets pass.
      Imported-source capture and automatic engine/package preparation remain open.
- [x] Fixture: extend captured associations to imported XML namespace aliases and
      standalone `cem:expr` occurrences, host/wrapping/sibling schema switches,
      source-mapped entity/CDATA payloads and nested restoration. Feed the original
      imported owner into the QL handoff; verify pending inputs and directed grants.
      Scenarios for later design verification: specialized import frames retain
      decoded source provenance without reparsing, target execution or new XML
      attribute-expression syntax (that recognition decision remains deferred).
- [x] Integrate lexical capture with the specialized XML import path under those
      fixtures, sharing the original immutable owner and explicit QL preparation.
      Scenarios for later design verification: namespace aliases are resolved by
      the import owner while later lifecycle evaluation keeps source-position bindings.
      Verification: two new native XML capture fixtures and one new imported QL
      handoff fixture pass; all five QL handoff fixtures preserve pending selection,
      original owners, directed grants and expression payload provenance.
      All 25 focused CEM-ML capture/reference/source/parity tests and 41 machine
      tests pass. Full `cem_ql:test` and `cem_ml_transform_cem_ql:test` Nx targets pass.
      Automatic engine/package loading and consumer preparation remain open below.
- [x] Fixture: package loading retains captured bindings beside the same source AST
      and exposes them to the installed compiler's lifecycle preparation. Verify
      unchanged reads reuse both owners, changed bindings produce a new candidate,
      unready selection preserves the active package, and later preparation completes
      that candidate without reparsing or mutating reference targets. Reuse its
      parsed source for successful descriptor extraction; preserve metadata-error
      and malformed-source retention behavior.
      Scenarios for later design verification: cached source metadata is independent
      of current runtime inputs; source-only loading cannot resolve references.
- [x] Wire retained package source capture into compilation requests under that
      fixture, preserving existing compiler callbacks and publication gates. Provide
      registry inspection of the captured source; prepare contexts/policies/grants
      through the existing explicit handoff at each lifecycle invocation.
      Scenarios for later design verification: machine diagnostics remain inspectable
      and source revision changes replace tree and binding metadata together.
      Verification: the native package lifecycle fixture covers closure, shared owners,
      changed bindings, pending/ready refresh and malformed-source preservation.
      All 12 package lifecycle fixtures and the full `cem_ml_transform_cem_ql:test`
      Nx target pass; 59 focused CEM-ML package tests and 33 registry tests pass.
      Ordinary engine CEM/XML input capture and lifecycle preparation remain below.
- [x] Fixture: ordinary CEM input validation hands captured directive/root and
      child namespace bindings to its installed lifecycle stage on the original AST.
      Verify restoration after child closure, distinct runtime selections, pending
      context, explicit grants and unchanged authored reference targets.
      Scenarios for later design verification: closed child frames do not lose their
      saved bindings; XML requires the specialized import handoff separately.
- [x] Capture CEM input bindings during AST construction and expose them on
      `InputValidationRequest`, sharing the pipeline AST owner and retaining existing
      diagnostic/version-pin behavior. Keep runtime context preparation explicit.
      Scenarios for later design verification: source-only requests remain pending;
      source capture does not infer grants or schema readiness.
      Verification: all 11 retained behavior/input-stage fixtures pass, including the
      new root/child/restored binding and pending-context fixture. All 207 engine
      tests, 22 focused root-scope checks and the full `cem_ml_transform_cem_ql:test`
      Nx target pass. The pipeline shares its original AST owner with saved metadata;
      CEM schema-machine and builder consume one stream without reparsing.
- [x] Fixture: connect specialized XML reference capture to an explicitly installed
      engine validation lifecycle stage, preserving namespace aliases, entity/CDATA
      source attribution and original imported owners. Verify pending contexts and
      directed grants while XML attribute values remain literal.
      Scenarios for later design verification: ordinary native XML source validation
      still uses its dedicated adapter; CEM reference consumption waits for a ready
      consuming schema and explicit runtime preparation.
- [x] Fixture: XML runtime-stage failures preserve native duplicate-attribute
      diagnostics and foreign-owner attribution, and a failure with warning-only
      diagnostics gains the existing hard stage-failure diagnostic.
      Scenarios for later design verification: source diagnostics are projected
      against XML bytes; foreign runtime diagnostics keep their own coordinates.
- [x] Wire that XML lifecycle handoff through the existing specialized import and
      retained source projection, preserving adapter diagnostics and avoiding a
      second source parse or synthetic reference arenas.
      Scenarios for later design verification: imported expression metadata and
      effective lexical bindings must travel together to the lifecycle consumer.
      Verification: both native XML engine handoff fixtures pass for already-parsed
      generic XML and custom-schema XML. They cover aliases, entity/CDATA mappings,
      literal attributes, original owners, pending/denied links and preserved failure
      provenance. The full `cem_ml_transform_cem_ql:test` Nx target and all 207 engine
      tests pass. After restricting admission to ordinary XML adapters, both handoff
      fixtures and all 30 focused engine validation tests pass again.
- [x] Fixture: exercise completed namespace/schema associations through ordinary
      native engine input loading and consumer preparation, including directive/host/wrapping/sibling forms,
      inherited defaults, subsequent rebinding, nested restoration and pending schema
      selection. Verify original owners, grants, effective policies and budgets
      using the adopted lexical/relationship identity rule and explicit CEM/XML handoffs.
      Scenarios for later design verification: identical expression source compiled
      under different lexical bindings stays distinct; explicit vendor crossings remain
      explicit; later declarations do not replace earlier occurrence bindings.
- [x] Fixture: CEM and XML engine handoffs keep request-wide depth/work accounting
      across a chain entering another captured lexical scope. Apply stricter
      destination limits from its effective schema without creating a new relationship
      boundary, and preserve denial until the runtime supplies a directed grant.
      Scenarios for later design verification: looser destination budgets cannot reset
      request work; reference chains can cross lexical scopes within one boundary.
- [x] Wire completed specialized frame associations into ordinary native engine
      input loading and consumer preparation using the explicit retained-host handoff.
      Keep occurrence binding retention,
      runtime-provided evaluation inputs, schema dependency readiness and directed
      crossing permissions separate. Do not infer runtime inputs from a saved context
      node or execute schema `select` expressions during parse/closure.
      Scenarios for later design verification: closed parser frames remain inspectable
      through retained metadata while consumer evaluation waits for its lifecycle inputs.
      Verification: both engine scope-matrix fixtures pass through CEM and XML input
      loading. They exercise directive/host/wrapping/sibling schema forms, inherited
      bindings, rebinding/restoration, pending-to-ready preparation and independent
      contexts on one original owner. Schema-derived destination work limits and
      request work/depth limits remain effective across lexical changes; vendor
      grants remain explicit. Existing lifecycle wiring needs no further production
      change. The full `cem_ml_transform_cem_ql:test` Nx target passes.
- [x] Fixture: verify inherited scope-property defaults, source-position
      bindings and shadowing, restoration after existing schema/namespace child
      scopes, and equal IDs in separate vendor scopes. Cover explicit permitted
      crossings without a document-wide ID scan or repeated root markers.
      Verification: a third engine scope fixture passes through CEM and XML. It
      checks default/prefixed namespace inheritance, schema defaults, named inline
      declaration visibility only after closure, child shadowing and parent
      restoration. Two vendor owners expose equal authored IDs and equal arena
      indices; each still needs its own directed grant. Source roots have no IDs
      and reference target fields remain unevaluated. The full
      `cem_ml_transform_cem_ql:test` Nx target passes.
- [x] Fixture: named inline-schema bindings expose their original declaring AST
      node through owner-checked retained metadata. Verify declaration closure,
      inherited/child-shadowed/restored bindings, foreign-owner rejection and
      original declaration identity in both CEM and XML imports. Cover empty
      declarations and metadata-only observation without a builder source ID.
      Scenarios for later design verification: source-position bindings cannot be
      recovered by matching names or byte offsets against a different arena;
      a later same-name declaration must not rebind an earlier occurrence.
- [x] Fixture: prepare engine scope defaults from owner-checked inline-schema
      source handles in both CEM and XML, preserving inheritance, shadowing,
      parent restoration and explicit vendor grants.
      Scenarios for later design verification: lexical preparation uses original
      declaration identity without interpreting byte ranges as arena addresses.
- [x] Align retained lexical/scope information and existing specialized frames
      with the shared reference contract by attaching owner-checked source handles
      for named inline-schema declarations through builder/import node identities.
      Completed: CEM builder feedback and XML import identities populate the
      original declaration node at closure. Owner-checked `inline_schema` lookup
      returns a native `SchemaDeclarationNode` from each saved binding, preserving
      inheritance, child shadowing, parent restoration and earlier bindings across
      later same-name declarations. Metadata-only observation leaves the node
      absent. Consumers use existing context boundaries without treating the
      containing-node handle as the runtime evaluator environment. Ten capture
      fixtures, 41 schema-machine tests, seven scoping tests and the full
      `cem_ml_transform_cem_ql:test` Nx target pass. Engine defaults now prepare
      through these native handles while vendor grants remain explicit.
      Use current scope forms; do not implement the deferred enclosed child override
      spelling.
      Scenarios for later design verification: preparing a named scope default
      retains its source owner without copying an AST or supplying runtime inputs.

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
- [x] Fixture: exercise the lifecycle query adapter with XML, JSON, YAML and
      CSV owners. Verify native reference construction, ordered/repeated targets,
      nested and empty references, source-owner retention, authored XML reference
      preservation and literal reference-looking data without evaluation.
      Scenarios for later design verification: query preparation and inspection
      must not resolve authored links or serialize a document into records.
- [x] Fixture: migrate CEM-document artifact ingress to native retained nodes;
      verify unary construction over original source references and attributes,
      source owner/provenance, stable identity across repeated wrapping of one
      AST allocation, literal compatibility projection and collection
      child artifacts without implicit evaluation or copied AST arenas.
      Scenarios for later design verification: flat native navigation is the
      default; callers request the former array-wrapped record view explicitly.
- [x] Migrate CEM-document artifact ingress to the shared native tree view and
      expose the old record projection through an explicit Rust compatibility
      function (user decision, 2026-10-05). Keep public reference type/access
      spelling deferred. Verify existing native-artifact consumers and reject
      invalid retained source without fallback to record projection.
- [x] Verify the shared typed bridge used by `cem_ml_transform_cem_ql` exposes
      references without format-specific evaluation or DOM/JSON substitutes.
      Treat current generic-node and target-view interfaces as implementation
      choices, not adoption of the deferred public reference type/access API.
      Completed: lifecycle ingress and CEM-document artifacts share native query
      nodes. Two lifecycle fixtures cover XML/JSON/YAML/CSV; artifact fixtures
      cover unary construction over original references/attributes, repeated
      projection identity and explicit record compatibility. Source identity now
      follows the original AST allocation. Migrated transform fixtures use
      `input.kind`; old record-shaped metadata binding is no longer implicit.
      The full `cem_ml_transform_cem_ql:test` and `cem_ql:test` Nx targets pass.

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
- [x] Specify the shared evaluation entry point and result ownership using the
      existing CEM-QL expression mechanism. The caller supplies context and
      lifecycle timing; retain source-position lexical meaning. Distinguish
      unevaluated source from pending, resolved, unresolved, and invalid runtime
      outcomes without requiring writeback onto the authored reference.
- [x] Fixture: evaluate one retained template against two independent supplied
      `datadom` trees, including evaluation after inputs become available.
      Verify independent outcomes, distinct resolved-empty and unresolved
      results, and unchanged authored expressions/edges without context IDs.
- [x] Fixture: cover deep reference chains, mixed node/reference targets,
      repeated targets, self-reference, cycles, scope crossings, and traversal
      limits within one lexical scope. Verify mandatory failure, warning, and
      ignored unresolved links with source provenance, plus separate empty
      cardinality checks. Ignoring a link must not fabricate a target or remove
      an authored edge.
- [x] Implement shared explicit evaluation and consumer-requested chain
      resolution under effective scope-schema policy, with cycle detection and
      bounded work. Preserve target order/multiplicity and the authored graph.
      Do not add a scheduler, subscriptions, automatic parse-time evaluation,
      URL/ID lookup, or element-to-ID projection.

Verification (2026-10-05): the focused audit reruns all tests in five ML
suites (`reference_resolution`, `reference_unresolved_policy`,
`reference_traversal_policy`, `reference_schema_contracts`,
`schema_declaration_references`) and three QL suites
(`reference_runtime_resolution`, `lexical_scope_handoff`,
`schema_declaration_references`). All 124 tests pass (87 ML, 37 QL); this closure changes documentation
only and adds no runtime behavior or public syntax.

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
- [x] Fixture: compile retained schema references through production CEM-QL
      contexts; verify original source allocation, lexical aliases, pending/empty/
      invalid outcomes (including malformed native constructor results),
      independent contexts, nested references and denied/granted
      scope crossings and foreign-host scope handles. Retained declarations
      survive dropping query owners.
- [x] Implement the native `CemQlSchemaDeclarationHost` compilation stage with
      caller-provided lifecycle contexts, original lexical owners, effective
      subtree scopes and explicit directed crossing grants. Consume the outer
      source constructor; resolve nested references through the shared walker.
      Verification: Nx `cem_ql:test` passes, including five native consumer
      fixtures. 32 CEM-ML declaration/reference and import/XPath fixtures pass.
- [x] Fixture: exercise explicit package loading through the CEM-QL compiler
      hook: source-only Pending, runtime Pending, later completion with the same
      source allocation, independent contexts, rejected replacement preservation,
      changed-source retention, and original query errors. Verify schema,
      converters and artifacts switch together without implicit grants.
- [x] Fixture: reject empty/warning-only compiler failures, foreign model
      identities and invalid traversal bounds without publishing a candidate;
      retain source-attributed errors and the last active package.
- [x] Fixture: load a reference-bearing manifest through an engine validation
      request and enforce the reused declaration on the input; preserve the
      caller context while compiling the enriched runtime snapshot.
- [x] Supply explicit lifecycle evaluation for package declaration references
      through the production consumer host, allowing completion and activation
      of the same source without automatic parser evaluation or a new scheduler.
      The explicit `EngineContext.schema_package_compiler` hook invokes a fresh
      native host before coordinated publication; source arenas are retained by
      resolved URI and byte revision. Source-only loading stays pending.
      Verification: Nx bridge tests pass (115 tests); the additional engine
      request fixture passes. All 41 focused CEM-ML reference/schema integration
      tests pass. Nx CEM-ML reports 2069 passes, two ignored and the known legacy
      dropdown selector failure (`template#cem-dropdown` not found).
      Scenarios for later design verification: one retained candidate completes
      when dependencies arrive; independent lifecycle contexts do not share
      selected targets; incomplete replacements preserve the active package.
- [x] Adapt native CEM-QL declaration views to original retained declaration
      owners for the production schema host. Do not serialize or clone source
      ASTs to manufacture target arenas; preserve lexical aliases and boundaries.
- [x] Fixture: reuse retained attribute declarations in multiple schemas; verify
      scalar validation, source ownership, authored collection order, zero/many
      selections, source-only pending state, kind/name failures, and partial
      unresolved results under every disposition.
- [x] Fixture: follow native CEM-QL attribute-reference chains with directed
      crossing grants and stricter destination limits; retain the final owner
      after the query host is dropped. Validate reused attribute constraints
      through manifest-driven engine package compilation.
- [x] Fixture: verify referenced attribute diagnostic dependencies are checked
      in the assembled consumer, retain the target declaration's source map,
      and accept explicitly supplied dependencies without implicit schema import.
- [x] Extend explicit schema compilation to `{attributes}` declaration references
      with shared scope resolution, original owners, existing attribute contracts
      and coordinated package readiness. Keep dependency declarations explicit.
      Verification: all 46 focused CEM-ML reference/schema tests and six native
      declaration-consumer fixtures pass. Nx bridge tests pass (117 tests); the
      final compact representation also passes focused engine package validation.
      Nx CEM-ML reports 2069 passes, two ignored and the known legacy dropdown
      selector failure (`template#cem-dropdown` not found).
      Scenarios for later design verification: a common attribute validates two
      schemas; an incomplete attribute link prevents package activation; lexical
      diagnostic strings require declarations in the consuming schema.
- [x] Define the existing schema validation/composition sites that consume typed
      node references, their target-kind/cardinality constraints, and required
      evaluation phase. Keep their construct-use and recursive-composition
      rules consumer-owned while applying the shared scope resolution policy.
      The accepted reference design inventories implemented collection consumers,
      existing scalar contracts and the remaining explicit integration phases.
- [x] Fixture: reuse diagnostic and behavior declarations from retained owners;
      cover zero/many selections, kind/key failures, local overrides, original
      lexical aliases, unresolved links and dependent attribute checks.
- [x] Fixture: pending behavior/diagnostic collections defer missing dependent
      lookups until complete; resolved-empty collections then report missing
      dependencies. Preserve invalid declaration and known binding errors.
- [x] Fixture: preserve declaring diagnostic aliases, original external binding
      errors during incomplete local selection, and reused inline function
      behavior metadata and source maps without synthesizing source nodes.
- [x] Fixture: bind native diagnostic/behavior selections from different retained
      owners only after directed grants; complete a pending package with reused
      dependencies and publish schema, converter and artifacts together.
- [x] Fixture: retain partial behavior bindings under every unresolved policy;
      defer only missing dependencies, preserve original evaluator errors even
      when they share compiler diagnostic codes, and keep known binding/default
      errors visible during incomplete selection.
- [x] Extend reference consumption to `{behaviors}` and `{diagnostics}` before
      dependent schema checks. Keep named/code-keyed collection order, original
      owners, explicit dependencies and shared scope readiness rules.
      Verification: all 53 focused CEM-ML reference/schema tests and seven native
      declaration-consumer fixtures pass. Nx bridge tests pass (118 tests).
      Nx CEM-ML reports 2069 passes, two ignored and the known legacy dropdown
      selector failure (`template#cem-dropdown` not found).
      Scenarios for later design verification: a reusable attribute names a
      reused diagnostic; a diagnostic names a reused behavior; missing or cyclic
      selections keep the model inactive rather than dropping its dependencies.
- [x] Fixture: reuse constraint declarations in authored order, retain source
      owners and declaring aliases, preserve nested reference-resolution metadata,
      reject wrong kinds/missing kind keys, retain duplicate policy errors, and
      verify lexical behavior binding through native QL scope grants.
      Scenarios for later design verification: candidate policies cannot reset
      the current traversal budget; pending constraints keep the candidate inactive.
- [x] Extend reference consumption to constraint declaration collections after
      diagnostic/behavior reuse, retaining kind keys, lexical aliases, nested
      declarations, policy duplicates and stable runtime traversal budgets.
      Verification: 58 focused ML reference tests, eight native QL declaration
      fixtures and 118 bridge tests pass. Full Nx ML library verification reports
      2069 passes, two ignored and the known legacy dropdown selector failure
      (`template#cem-dropdown` not found).
      Scenarios for later design verification: shared constraints bind local
      behavior names in each consumer and never reset current scope budgets.
- [x] Fixture: compile field-contract references during package refresh; retain
      the active schema, converter and artifact registrations on missing targets,
      and activate the complete candidate once its local element is available.
      Scenarios for later design verification: target errors remain inspectable
      and a rejected candidate never replaces the last complete package.
- [x] Fixture: reuse field contracts in multiple consuming schemas, preserve
      repeated ordered applications and nested choices, retain lexical aliases
      and source owners, reject wrong kinds/missing keys, and check missing local
      targets only after element assembly completes. Cover native scope grants.
      Scenarios for later design verification: empty element selections expose
      missing targets; pending collections preserve original errors and readiness.
- [x] Implement field-contract declaration reference reuse and the approved
      missing-target rule: report a compile error once element assembly is
      complete, defer the lookup while element references are incomplete, and
      apply the same rule to direct and referenced contracts. Preserve required
      keys, repeated ordered applications, source owners, nested choices and
      per-application lexical aliases. Pending diagnostic bindings do not hide
      independent contract errors.
      Verification: 62 focused ML reference tests, nine native QL declaration
      fixtures and 119 bridge tests pass. Full Nx ML library verification reports
      2069 passes, two ignored and the known legacy dropdown selector failure
      (`template#cem-dropdown` not found).
      Scenarios for later design verification: one contract applies in two
      consuming schemas; local targets stay local while declaring aliases retain
      source meaning; invalid targets never disappear as empty selections.
- [x] Fixture: structural reference traversal retains ordered subtree edges,
      active cycle identities, cumulative scope work and depth through terminal
      elements. Cover partial child completeness and sibling restoration.
      Scenarios for later design verification: reference → element → reference
      cycles terminate; destination work does not reset inside selected subtrees.
- [x] Fixture: explicitly validate structural child references with the consuming
      schema without source cloning. Cover multiple/repeated targets, parent
      sequence contracts, subtree attributes, pending vs empty, invalid node kinds,
      source attribution, independent runtimes and native QL scope grants.
      Scenarios for later design verification: incomplete child selections never
      look like missing children; a reused element keeps its original owner.
- [x] Implement explicit validation-input structural reference consumption.
      Parent and selected-subtree checks use the consuming schema. Preserve
      zero/many ordering, original owners and source maps; descend under one
      active reference stack and cumulative request/destination budgets. Expose
      completeness separately from failure, and defer field contracts while
      their child selections are incomplete. Native QL exposes `validate_input`.
      Verification: 68 focused ML reference tests, ten native QL declaration/input
      fixtures and 119 bridge tests pass. Full Nx ML library verification reports
      2069 passes, two ignored and the known legacy dropdown selector failure
      (`template#cem-dropdown` not found).
      Scenarios for later design verification: references cross vendor scopes;
      a selected child matches the parent contract but differs under the two
      subtree schemas; original owners, source maps and request limits survive.
- [x] Fixture: report per-input validation completion without changing diagnostic
      severity/counts. Cover legacy reports, source-only structural references,
      stable multi-input ordering, deferred child contracts/whole-document
      behavior hooks and non-success CLI outcomes for incomplete reports with
      zero violations.
      Scenarios for later design verification: ordinary documents finish; source
      references stay unevaluated and pending validation never appears to pass.
- [x] Add explicit engine validation completion metadata. Validate/check reports
      expose aggregate and per-input completion in stable input order. Keep
      diagnostic counts and severity independent; source-only structural
      references remain pending, child contracts and whole-document hooks defer,
      and CLI validation/check return non-success even with zero violations.
      Legacy reports without metadata retain their existing interpretation.
      Verification: 52 focused completion/reference fixtures, two command-type
      fixtures and the CLI completion fixture pass. Broad CLI verification has
      523 passes and the two scoped-grant override failures recorded below.
      Full Nx ML verification has 2069 passes, two ignored and the known
      legacy dropdown selector failure (`template#cem-dropdown` not found).
      Scenarios for later design verification: incomplete ignored links still
      prevent a completed verdict without becoming synthetic violations.
- [x] Integrate structural reference validation into an explicit runtime-selected
      engine lifecycle hook for parser-backed source validation.
      Propagate the explicit consumer's completion and diagnostics through the
      report metadata; keep parser/load stages unevaluated. Do not make ignore
      policy silently resolve missing children.
      Scenarios for later design verification: pending inputs preserve available
      diagnostics; incomplete ignored links still prevent a completed verdict;
      source-only loading and independent runtime contexts remain unchanged.
- [x] Fixture: retained behavior handoff preserves original owners, declaring
      schemas and repeated placement edges without expanding source arenas.
      Cover pending structural selections, independent structural diagnostics,
      complete behavior violations and legacy evaluators without the new hook.
      Scenarios for later design verification: multi-owner selections preserve
      lexical context; unsupported or deferred behavior never appears complete.
- [x] Fixture: native QL retained behavior handoff uses the registered source
      scopes and directed grants, preserving selected owners and declaring
      schemas. A denied crossing must defer behavior; a granted crossing must
      invoke the new hook without the old whole-document method.
      Scenarios for later design verification: the same AST supports independent
      runtime snapshots without target-list mutation or context IDs.
- [x] Fixture: structural-only and pending validation do not request declaring
      schema lookup; enrich lexical metadata only at the explicit, complete
      retained behavior handoff.
      Scenarios for later design verification: schema lookup does not become an
      implicit loading/evaluation stage for ordinary structural consumers.
- [x] Implement the additive `SchemaBehaviorEvaluator.validate_retained_structure`
      handoff. Expose retained source owners, optional original declaring schemas,
      ordered placement edges and child-selection completeness. Run the new hook
      only after complete structural selection; preserve diagnostic severity and
      distinguish complete behavior violations from pending checks. Unsupported
      legacy evaluators remain incomplete without a whole-document fallback.
      Native QL exposes `validate_input_with_behavior_evaluator`; ordinary
      `validate_document` and structural-only APIs remain compatible.
      Verification: 56 focused ML fixtures, 11 native QL host fixtures and 119
      bridge tests pass. Full Nx ML verification reports 2069 passes, two ignored
      and the known legacy dropdown selector failure.
      Scenarios for later design verification: selected nodes from multiple
      owners retain their function context and source attribution.
- [x] Adopt per-placement behavior checks for repeated retained source nodes.
      Each consumer placement keeps its consumed parent and child context;
      original node/owner identity never deduplicates distinct placements.
      Scenarios for later design verification: the same original node under two
      parents may produce different behavior outcomes without source expansion.
- [x] Fixture: native QL placement views retain original owners and lexical
      schemas, expose consumed parent/child axes and typed original attributes,
      distinguish repeated selections and overlapping arena node IDs, and reject
      incomplete or malformed validation graphs without returning empty axes.
      Scenarios for later design verification: source parents differ from
      consumed parents; text extraction follows selected children; no AST clone,
      record substitute, context ID or reference target-list mutation is needed.
- [x] Fixture: standalone QL selects repeated placements by their different
      consumed parents and extracts text through native views. Distinguish a
      completed empty selection from pending roots even when both graphs are
      empty, and preserve independent query snapshots of the same source nodes.
      Scenarios for later design verification: per-placement identity survives
      selection/set operations while original source identity remains shared.
- [x] Fixture: construct placement views from real native QL host results,
      enforcing context readiness and directed scope grants before query access.
      Resolve the same root reference to a target, an empty selection, then
      pending context without changing the source reference.
      Scenarios for later design verification: pending empty graphs stay
      incomplete after an earlier completed runtime snapshot.
- [x] Implement native per-placement query views for retained validation.
      `RetainedValidationQueryTree` retains original owner handles and consumed
      graph edges; `ValidationPlacementNode` exposes native parent/child axes,
      original typed attributes, source maps and optional declaring schemas.
      Distinct placement/snapshot identities do not create authored or context
      IDs. Constructor checks readiness, owning edges and forest reachability;
      unavailable roots/children never become completed empty axes.
      Scenarios for later design verification: each placement has independent
      query focus and retains its originating schema/source attribution.
- [x] Use placement views in `CemQlSchemaBehaviorEvaluator`. Native select/match
      and function paths operate per consumed placement; candidates retain their
      original owners and source maps. Explicit diagnostic scalar extraction
      preserves the reporting boundary. Keep legacy whole-document APIs compatible.
      Verification: full Nx QL target (857 passes, 9 ignored), full Nx
      bridge target (125 passes), and 56 focused ML reference/completion checks
      pass; the new suites cover six placement and six behavior fixtures.
      Scenarios for later design verification: each occurrence's match/function
      result uses its consumed parent/child context and original lexical schema.
- [x] Choose the engine runtime handoff: retain the parsed owner and invoke an
      optional caller-provided per-input runtime stage.
      Scenarios for later design verification: runtime snapshots retain source
      ownership and avoid evaluating references during parse/load.
- [x] Integrate the explicit runtime validation hook into validate/check for
      parser-backed inputs with ready consuming models. `InputValidationStage`
      receives the original parsed owner retained as a native tree, the model,
      effective reference policy, root scope and optional behavior evaluator.
      Its outcomes replace source-only model checks and propagate completion and
      diagnostics independently. Preserve other pipeline checks; parse/load and
      ordinary query registration never invoke the optional stage.
      Verification: 61 focused ML lifecycle/reference checks, 17 native QL
      host/placement checks, and the full Nx bridge target (126 passes) pass.
      Full Nx ML verification has 2069 passes, two ignored and the existing
      dropdown selector failure (`template#cem-dropdown` not found).
      Scenarios for later design verification: pending snapshots stay incomplete;
      warning-only preparation failures receive a hard stage diagnostic; original
      vendor URI/coordinates never inherit the input document's line index.
- [x] Fixture: JSON/YAML/CSV validate and check supply their retained lifecycle
      owners through the shared native import to an explicit runtime stage.
      Resolve a retained template against independent inputs; verify pending
      context, source ownership, literal reference lookalikes, malformed import
      rejection and original foreign diagnostic coordinates. Parse/load must not
      invoke the stage, and missing or incomplete models must not admit it.
      Scenarios for later design verification: consuming data schemas use the
      same native tree as query ingress without reparsing or JSON record handles.
- [x] Extend runtime-stage validation to XML/JSON/YAML/CSV lifecycle adapters
      that finish source validation without a CEM parse. Supply their source owner
      through the shared typed import boundary; never reparse imported data or
      pass format-specific parser ASTs/JSON records into the runtime consumer.
      Scenarios for later design verification: imported XML/JSON/YAML/CSV sources
      and CEM sources share native ownership, completion and provenance contracts.
      Completed: JSON/YAML/CSV use `import::retain_lifecycle` on the loaded
      native AST, with no parser retry or record runtime values. Three native
      bridge fixtures cover validate/check, parse-only isolation, independent
      ready/pending contexts, literal lookalikes, malformed source rejection,
      missing/incomplete model admission and original foreign diagnostics.
      Other specialized validators retain their existing paths.
      Verification: the full `cem_ml_transform_cem_ql:test` Nx target passes,
      alongside nine focused ML runtime/completion fixtures and the existing
      two XML lifecycle fixtures.
- [x] Adopt retained behavior function candidate typing: require
      explicit `node` candidate parameters on the retained path while preserving
      existing `object` signatures on the legacy whole-document path.
      Do not copy a retained candidate into a JSON/object record.
      Scenarios for later design verification: legacy candidate property paths
      remain compatible in their declared mode; typed nodes carry original owner,
      consumed placement context and source maps through function execution.
- [x] Fixture: retained schema behaviors select each placement independently,
      extract native candidate parent/name/attribute fields in function bodies,
      reject object candidate parameters and node-valued diagnostic output,
      preserve legacy object execution, and distinguish pending/empty stages.
      Scenarios for later design verification: repeated source nodes under two
      consumed parents produce separate attributed diagnostics without AST copies.
- [x] Fixture: native QL host behavior validation enforces directed scope grants
      and context readiness, preserves original source maps, and checks repeated
      placements across owners with overlapping arena IDs.
      Scenarios for later design verification: denied or pending inputs never run
      behavior; a completed empty result remains distinct and leaves sources intact.
- [x] Fixture: retained function signature checks run even for empty selections,
      and selectors returning original attributes are rejected as non-elements.
      Scenarios for later design verification: invalid consumer contracts cannot
      pass merely because a runtime snapshot currently selects no candidates.
- [x] Fixture: engine runtime validation replaces source-only model checks,
      invokes the stage once per input, retains independent source owners, and
      commits completion in stable input order separately from violations.
      Scenarios for later design verification: legacy behavior hooks never run
      before retained checks; pending inputs with no violations stay incomplete.
- [x] Fixture: runtime stage preparation failures preserve diagnostics and
      remain incomplete; missing hard failures receive an explicit stage failure.
      Preserve diagnostics attributed to other source URIs and coordinates.
      Scenarios for later design verification: warning-only setup failure cannot
      appear successful; vendor source coordinates never use input source bytes.
- [x] Fixture: pending schema models do not invoke runtime input validation.
      Scenarios for later design verification: blocked schema declarations retain
      their existing diagnostics and completion gate without evaluating inputs.
- [x] Fixture: installing a runtime validation stage never evaluates parse/load
      requests, and malformed schema traversal bounds prevent stage execution.
      Scenarios for later design verification: runtime registration does not
      change source-only lifecycle behavior or bypass schema-owned limits.
- [x] Fixture: engine validate/check invokes native QL retained behavior using
      a fresh per-input host, typed node candidates and directed crossing grants.
      Preserve vendor URI/line coordinates and repeated placement diagnostics.
      Scenarios for later design verification: pending contexts defer behavior;
      independent input owners keep source reference target lists untouched.
- [x] Choose a general retained native value slot for CEM AST attributes,
      preserving text compatibility and leaving binding/evaluation to consumers.
      Scenarios for later design verification: native attribute references survive
      codec/inspection without becoming strings or runtime record substitutes.
- [x] Implement the native attribute slot across source retention, owning graph
      edges, codec/projection validation, inspection and native value handoff.
      Verification: 90 focused ML integration checks, three native writer/validator
      unit checks and the full Nx bridge target (127 passes) pass. Broad Nx ML
      verification reports 2070 passes and the existing dropdown template failure.
      Scenarios for later design verification: parsing never evaluates references;
      old attributes/codecs remain compatible; expression nodes retain provenance.
- [x] Prepare native attribute slot lexing: retain `AttributeValueSyntax` on
      source tokens and share the quote/comment-aware brace scanner between
      unquoted attribute spans and standalone expressions. XML/HTML literal
      token behavior remains unchanged; native event/AST handoff is now implemented.
      Verification: 145 focused ML tokenizer/event/builder/highlight/reference
      checks and the full Nx bridge target (126 passes) pass.
      Scenarios for later design verification: lexer metadata is propagated into
      the native AST slot rather than reconstructing intent from value text.
- [x] Adopt CEM-ML unquoted brace expressions first; preserve XML attribute
      literals until an explicit XML expression contract is designed.
      Scenarios for later design verification: quoted lookalikes stay literal.
- [ ] Design XML attribute expression recognition and compatibility before
      enabling native value construction for XML attributes.
      Scenarios for later design verification: quoted XML `{#nodes}` remains
      literal unless the eventual explicit contract chooses expression intent.
- [x] Fixture: AST/DOM projection validators accept native attribute values,
      reject malformed reference target IDs and support binary versions 1 and 2.
      Scenarios for later design verification: explicit exports retain node kinds;
      old envelopes remain valid while new native edges use versioned layouts.
- [x] Fixture: typed inspection and canonical formatting preserve native attribute
      expressions and quoted lookalikes; markup export requires consumer resolution.
      Scenarios for later design verification: inspect/format never evaluate values
      or silently export unresolved native references as empty attributes.
- [x] Fixture: native query attribute views expose the original owning arena's
      value nodes without resolving references or serializing runtime records.
      Scenarios for later design verification: literal `.value` stays scalar;
      native `.value` and `.valueNodes` retain typed source identity.
- [x] Fixture: retain native attribute references and general expressions with
      owning value edges, original source ranges, lexical context and no targets.
      Verify quoted lookalikes and authored-ID compatibility without evaluation.
      Scenarios for later design verification: native values are authoritative;
      expression-valued IDs never enter the literal ID table.
- [x] Fixture: round-trip native attribute values through binary persistence,
      share the original retained owner and reject invalid owning value edges.
      Scenarios for later design verification: legacy binaries retain literal
      attributes; native values do not become XPath attribute children.
- [x] Fixture: distinguish quoted/bare CEM attribute literals from unquoted
      brace expression spans, preserving original lexical source ranges.
      Scenarios for later design verification: identical text can have different
      literal/expression intent; boolean attributes retain their existing form.
- [x] Fixture: query strings, escaped/doubled quotes, nested comments and record
      braces never prematurely close attribute spans; following attributes and
      standalone reference expressions retain their boundaries.
      Scenarios for later design verification: tokenizer boundary recognition
      agrees between attribute and standalone expression slots without evaluation.
- [x] Fixture: malformed attribute expressions retain their source body and
      report the missing boundary; XML braced attribute text stays literal under
      its existing contract while XML expression design remains deferred.
      Scenarios for later design verification: malformed comments/strings cannot
      turn later source text into an apparently complete expression value.
- [x] Adopt explicit node-valued attribute contracts; primitive contracts retain
      literal inputs and never automatically extract values from reference targets.
      Scenarios for later design verification: retention never chooses conversion;
      string constraints cannot silently turn native references into empty text.
- [x] Implement the independent native attribute contract guards: recognize node
      types, reject literal node inputs and native primitive inputs, and retain
      pending readiness without inventing empty scalar values.
      Scenarios for later design verification: required-attribute and presence
      checks still run; value-dependent checks and behavior wait for consumption.
- [x] Fixture: primitive or untyped attribute contracts reject native expressions
      without evaluation or scalar empty-value/facet diagnostics.
      Scenarios for later design verification: quoted lookalikes remain literal;
      original attribute source maps and schema-owned type diagnostics survive.
- [x] Fixture: node-valued contracts reject literal/boolean inputs, scalar
      conversion and textual schema defaults; native values remain retained.
      Scenarios for later design verification: an authored string never becomes
      a native constructor merely because its contract expects a node.
- [x] Fixture: retained input validation stays incomplete for native attribute
      references and general expressions without evaluating them; preserve
      available structural checks and attribute presence conditions.
      Scenarios for later design verification: native attributes cannot satisfy
      readiness through absent text, trigger defaults or look absent to guards.
- [x] Fixture: source-only validation defers behavior for native attribute values
      and keeps literal documents on their existing completed validation path.
      Scenarios for later design verification: both reference and generic query
      expressions require an explicit lifecycle consumer, including vendor nodes.
- [x] Adopt exactly one node-valued attribute target by default; explicit
      `itemCount`, `minItems` or `maxItems` facets replace the default envelope.
      Scenarios for later design verification: resolved-empty differs from pending;
      repeated targets retain order and identity; count failures are complete.
- [x] Implement an explicit bounded native attribute reference consumer, retaining
      source/target owners and using schema-owned cardinality diagnostics.
      Scenarios for later design verification: no source mutation, string
      extraction, ID generation or implicit behavior execution occurs.
- [x] Update incompatible primitive count-facet diagnostic expectations to
      include the newly allowed node types in `expectedType`.
      Scenarios for later design verification: primitive type mismatches remain
      schema errors; advertised allowed types agree with the count consumer.
- [x] Fixture: native attribute reference chains preserve owner identity and
      enforce singleton defaults and explicit count envelopes, including empty.
      Scenarios for later design verification: repeated targets count separately;
      malformed count facets remain schema compilation errors.
- [x] Fixture: native attribute pending, ignored, denied, cyclic and work-limited
      resolutions remain incomplete; available prefixes do not trigger count checks.
      Scenarios for later design verification: host scopes/grants and destination
      limits remain enforced by the shared resolver.
- [x] Fixture: node count diagnostics preserve schema-owned severity/message
      and the original attribute source; primitive/general-expression slots do
      not invoke the reference host.
      Scenarios for later design verification: warning count violations can be
      complete; unsupported expression consumption remains pending.
- [x] Adopt resolved native nodes through the existing retained behavior
      attribute `.value`; original source handles retain authored references.
      Scenarios for later design verification: concurrent consumers never
      overwrite shared targets or turn node values into scalar records.
- [x] Adopt compiler rejection of lexical `values` constraints for node types;
      native target validation belongs to node behaviors and cardinality facets.
      Scenarios for later design verification: incompatible lexical contracts
      cannot trigger reference evaluation or automatic primitive extraction.
- [x] Fixture: a caller-supplied model with unconsumed lexical facets keeps
      attribute validation incomplete while still applying available count checks.
      Normal schema compilation rejects those incompatible node facets.
      Scenarios for later design verification: complete target selection and
      complete contract validation remain independently observable.
- [x] Adopt selected-subtree query access for consumed attribute targets, with
      explicit grants required for broader original parent/sibling/root access.
      Scenarios for later design verification: navigation never silently widens
      scope access; original source handles remain available to trusted consumers.
- [x] Retain references authored inside selected target subtrees as native nodes
      until an explicit consumer resolves them; resolve the attribute chain only.
      Scenarios for later design verification: query navigation does not execute
      nested expressions or rewrite the target's authored child structure.
- [x] Implement a bounded, grant-checked snapshot of original target subtree
      handles, and a native query view whose axes stay within that snapshot.
      Scenarios for later design verification: parent/context/target access cannot
      bypass the selected boundary, including saved non-owning reference links.
- [x] Fixture: lexical `values` on a node contract is a schema error before
      host evaluation, while literal primitive enums and node counts remain valid.
      Scenarios for later design verification: diagnostics retain the schema's
      original attribute source and explain the incompatible native contract.
- [x] Fixture: native attributes in authored and referenced validation placements
      consume once, preserve target owners, and share the enclosing request's
      active identities and work/scope budgets in selected structural subtrees.
      Scenarios for later design verification: attribute incompleteness cannot
      erase available structural/presence checks or fake a resolved-empty value.
- [x] Fixture: query targets keep source ownership and authored descendant refs,
      expose consumed attribute `.value`, and block external parent/sibling/root,
      context and saved target links; capture stays bounded and grant checked.
      Scenarios for later design verification: repeated target occurrences retain
      identity/order and independent consumers never overwrite source targets.
- [x] Fixture: corrupt target owning edges are rejected before query access;
      missing handles, duplicate ownership and non-attribute attribute edges
      cannot masquerade as complete authorized subtrees.
      Scenarios for later design verification: bounded capture also limits
      malformed cycles; valid repeated reference selections remain distinct.
- [x] Fixture: retained behavior and runtime validation read consumed node values
      through `.value` and native attribute conveniences without scalar coercion.
      Scenarios for later design verification: pending inputs defer behavior;
      legacy primitive attributes retain existing binding and source attribution.
- [x] Fixture: explicitly re-reference consumed native query views through the
      production host; retain original owners and registered scopes rather than
      manufacturing synthetic arenas or inheriting the requesting scope.
      Scenarios for later design verification: renewed consumption still checks
      directed grants and never writes targets into the shared source reference.
- [x] Adopt an explicit lifecycle expression hook for general native attribute
      expressions; consume returned nodes/references under the node contract and
      shared bounds, without implicit parser evaluation or scalar extraction.
      Scenarios for later design verification: absent context stays pending;
      general expressions and reference constructors remain distinct source nodes.
- [x] Fixture: general native attribute expressions evaluate only through the
      explicit lifecycle hook; verify missing context, original parse/type errors,
      zero/many/repeated node results, returned reference chains and invalid scalars.
      Scenarios for later design verification: hook results retain source owners;
      reference-to-reference resolution follows existing effective scope policy.
- [x] Fixture: composite slots share cumulative work and destination limits with
      enclosing structural selections; verify authored order, pending branches,
      mixed expression/reference values and original-source query access.
      Scenarios for later design verification: no per-value accounting resets;
      complete selection is distinct from available partial target sequences.
- [x] Fixture: malformed native source-slot ownership is rejected before query
      access; missing and repeated owning handles cannot produce a complete value.
      Scenarios for later design verification: malformed slots remain bounded;
      repeated selected target occurrences remain valid and distinct.
- [x] Fixture: the engine runtime stage consumes general native attribute
      expressions alongside references, preserves input attribution, and defers
      behavior for pending inputs without expanding authored descendant links.
      Scenarios for later design verification: the caller controls evaluation
      time/context, and every source arena remains immutable across consumers.
- [x] Add the lifecycle expression hook and native result validation before
      completing generic attribute slots; preserve original expression diagnostics,
      lexical context and retained target ownership.
      Scenarios for later design verification: empty/multiple results obey the
      authored count envelope; scalar and foreign-host results cannot become nodes.
- [x] Support composite native attribute slots through one shared request rather
      than separate per-value budgets; generic expression evaluation needs an
      explicit expression consumer. The standalone reference-slot API stays
      chain-only; retained placement validation supplies the broader consumer.
      Scenarios for later design verification: zero, repeated and mixed native
      values preserve order, ownership and bounded traversal accounting.
      Verification: all 71 focused ML tests and all 130 bridge tests pass; the
      full Nx CEM-QL target passes. The final native behavior fixture also
      preserves returned expression-shaped targets without reevaluation. Nx ML
      reports 2072 library passes, two ignored and only the known legacy dropdown
      template failure (`template#cem-dropdown` not found).
- [x] Integrate the attribute consumer with retained validation placements and
      behavior access using the selected-subtree query boundary. References in
      selected structural subtrees share the enclosing request's active identities
      and budgets rather than restart traversal accounting.
      Scenarios for later design verification: mixed structural/attribute cycles
      and repeated scope crossings remain bounded; consumers see original nodes.
- [x] Complete engine runtime native attribute validation by invoking the
      retained placement consumer after query-access boundaries are implemented.
      Supply lifecycle context and keep completeness separate from schema-owned
      diagnostic disposition.
      Scenarios for later design verification: unresolved mandatory/warning/ignored
      links remain incomplete; native target owners and lexical scopes survive;
      literal and primitive contracts keep their existing behavior.
      Verification: all 74 focused ML fixtures, the full Nx CEM-QL target and
      all 129 bridge tests pass. Nx ML reports 2072 library passes, two ignored
      and only the known legacy dropdown-template failure
      (`template#cem-dropdown` not found). The existing QL literal-attribute
      fixture now includes empty native value edges.
- [x] Adopt explicit native references in element `@base`, selecting exactly one
      named element declaration while keeping literal QName bases compatible.
      Scenarios for later design verification: inheritance retains existing local
      override rules, lexical aliases and original declaration owners.
- [x] Fixture: source-only native element bases remain pending instead of silently
      compiling a complete model without inheritance; reject generic/composite
      native base slots before evaluation and retain original source diagnostics.
      Scenarios for later design verification: pending bases prevent package
      activation while local declaration properties remain available for inspection.
- [x] Retain native base readiness and source diagnostics during source-only schema
      compilation; no query context or evaluation should be requested at this stage.
      Scenarios for later design verification: literal QName bases stay completed,
      and both pending and hard-invalid base candidates remain inspectable.
- [x] Adopt a dedicated schema-owned `element-base` datatype for QName/native
      alternatives; literal QName behavior stays compatible and native targets
      are checked by the compiler's named element-declaration consumer.
      Scenarios for later design verification: generic primitive contracts still
      reject native values; bases do not require a new general union mechanism.
- [x] Fixture: native bases inherit original declarations and their lexical QName
      aliases, preserve local override rules and source-only pending state, and
      retain owners after the host is dropped.
      Scenarios for later design verification: two derived declarations can share
      one base; dependent attribute/behavior declarations remain explicit.
- [x] Fixture: reject empty/multiple/wrong-kind/unnamed native base targets, while
      pending/unresolved/denied/cyclic base chains remain bounded and inspectable.
      Scenarios for later design verification: incomplete selections never masquerade
      as complete empty bases, and invalid candidates cannot replace active packages.
- [x] Fixture: native base chains share request and stricter destination limits,
      including bases inside referenced element collections.
      Scenarios for later design verification: inheritance cannot restart accounting;
      unrelated ready models keep existing QName semantics.
- [x] Fixture: production CEM-QL native bases preserve lexical context, require
      explicit scope grants and share bounds inside referenced collections.
      Scenarios for later design verification: retained owners survive host release;
      a destination context evaluates its own nested base.
- [x] Fixture: the schema metamodel admits native element bases and existing
      literal QName/wildcard bases, while other scalar sites reject native bases.
      Scenarios for later design verification: malformed native bases are rejected
      by compilation, not interpreted as general attribute expressions.
- [x] Fixture: native base package candidates cannot activate before evaluation;
      an incomplete replacement preserves the complete active package.
      Scenarios for later design verification: schema, converter and artifact
      activation remains coordinated with inheritance readiness.
- [x] Fixture: nested base cardinality errors point to the failing base constructor
      in its original document rather than the enclosing collection reference.
      Scenarios for later design verification: original maps and expressions survive
      nested compilation across explicitly granted scopes.
- [x] Implement bounded native base compilation, original lexical alias handling,
      the metamodel datatype contract and coordinated package readiness integration.
      Scenarios for later design verification: no source AST clones, implicit schema
      imports, generic expression evaluation or public root IDs are introduced.
      Verification: 78 focused ML fixtures pass; full Nx QL and bridge targets
      pass (131 bridge tests). Full Nx ML reports 2072 library passes, two ignored
      and only the known legacy `template#cem-dropdown` failure. Native inheritance
      preserves pending candidates, original target owners and nested error maps.
- [x] Adopt attribute declaration `@type={#datatype}` selecting exactly one named
      schema `{type}` declaration, preserving its original owner and lexical scope;
      keep literal type names compatible. Executable datatype forms remain a
      separate decision before enabling consumption.
      Scenarios for later design verification: named declarations are not copied
      into source ASTs; reference evaluation belongs to compilation lifecycle.
- [x] Fixture: native attribute type slots stay pending during source-only and
      explicit compilation until their datatype consumer exists, without invoking
      the type reference host; literal and absent type contracts stay compatible.
      Scenarios for later design verification: an unconsumed native type cannot
      activate a silently untyped model; local metadata remains inspectable.
- [x] Fixture: generic native attribute type expressions and composite slots are
      source-attributed schema errors under the explicit-reference contract.
      Scenarios for later design verification: malformed slots cannot activate
      candidates regardless of unresolved-link diagnostic disposition.
- [x] Fixture: selected attribute declarations retain pending native type slots
      and their original constructor metadata, without evaluating the type link.
      Scenarios for later design verification: referenced attribute reuse does not
      bypass candidate readiness or inherit the consuming schema's lexical scope.
- [x] Fixture: production CEM-QL selected attribute declarations keep native types
      pending without an executable datatype consumer, retaining original owners.
      Scenarios for later design verification: a collection's complete selection
      cannot imply that its selected declaration's datatype has been consumed.
- [x] Fixture: an unconsumed native attribute type in a replacement package keeps
      its candidate inspectable and preserves the active schema/converter/artifacts.
      Scenarios for later design verification: missing datatype context cannot
      downgrade a typed attribute to an untyped, publishable declaration.
- [x] Fixture: pending native type metadata defers type-dependent facet/default
      checks while retaining independent malformed facet and local values errors;
      existing serialized literal attribute metadata stays compatible.
      Scenarios for later design verification: an unknown datatype cannot prove a
      facet inapplicable, but a malformed pattern remains a known schema error.
- [x] Fixture: standalone attribute conversion rejects pending native type
      models in both base and restriction contracts; caller-supplied pending
      attribute metadata also blocks model readiness. Pending metadata survives
      explicit serialization while old literal metadata defaults to no pending type.
      Scenarios for later design verification: no scalar/string fallback bypasses
      datatype readiness when a consumer extracts an individual attribute model.
- [x] Guard source-only and selected native attribute type readiness before
      executable datatype consumption is implemented; retain available metadata.
      Scenarios for later design verification: incomplete replacement candidates
      preserve the last complete package under existing activation rules.
      Verification: 91 focused ML reference/value tests, 154 schema model unit
      tests, 34 production QL tests and 111 bridge/package tests pass against the
      final guard. General datatype execution remains disabled pending its design.
- [x] Choose general datatype compilation design before enabling native `@type`
      consumption; do not adopt a limited built-in-base/values-only consumer.
      Scenarios for later design verification: all authored `kind`, `rule`, base
      and restriction fields acquire an explicit supported meaning.
- [ ] Adopt the general datatype compilation design and enable bounded singleton
      attribute type consumption only after its executable contracts are chosen.
      Draft: [general datatype compilation proposal](cem-datatype-compilation-proposal.md).
      Scenarios for later design verification: unsupported declarations cannot
      silently lose rules; source owners, lexical scope and budgets survive reuse.
- [ ] Define native datatype and function scalar composition contracts before
      extending their existing string-valued sites. Element `@base` is already
      implemented under the dedicated singleton `element-base` contract above;
      it is not blocked by these remaining consumer contracts.
      Scenarios for later design verification: singleton composition rejects
      multiple targets; recursive validation stays bounded; existing aliases
      remain compatible without making scalar strings implicit constructors.
- [x] Fixture: validate referenced constructs and reuse one declaration from
      multiple schema sites without cloning it into the source tree. Cover
      invalid target kinds, unresolved links under each disposition, source
      attribution, independent scopes, and bounded recursive relationships.
      Completed: ML declaration fixtures cover reuse across schemas, wrong kinds,
      all unresolved dispositions, readiness and bounded cycles; QL fixtures cover
      original-owner handoff, independent contexts, scope grants and destination
      limits. These run in the package suites recorded in section 7.
      Scenarios for later design verification: supported declaration collections
      and validation inputs retain their original source arenas at every reuse;
      new scalar sites require separate target-kind/cardinality contracts.
- [x] Integrate shared reference evaluation into supported declaration collections,
      singleton element bases, structural validation inputs and native attribute
      validation. Preserve source-position capture and explicit scope handoff
      alongside specialized schema/namespace records, without a mandatory common
      storage struct or public root IDs.
      Completed: `CemQlSchemaDeclarationHost`, the retained structural/attribute
      consumers and optional engine lifecycle stage use the shared bounded path.
      Specialized scope establishment remains caller-owned; capture/handoff does
      not execute a namespace/schema selector during parsing.
      Scenarios for later design verification: lexical snapshots and runtime
      relationship boundaries remain distinct, with directed crossing grants.
- [x] Audit specialized scope-property reference adoption before implementation.
      Completed (2026-10-06): namespace bindings retain URI/context identities;
      `SchemaSource::Select` retains selector text; lexical capture and explicit
      subtree/following-sibling handoff are implemented. None of these executes
      native property selection. The subsequent cardinality decision and native
      consumers below complete that implementation; see the follow-up audit for
      current coverage and remaining integrations.
      Scenarios for later design verification: distinguish lexical shadowing
      from a selector's ordered target sequence; retained QNames and earlier
      bindings remain unchanged by later consumer completion.
- [x] Decide schema scope-selector multi-target behavior before implementing
      consumption. Adopted (2026-10-06) through the instruction to continue with
      the recommendation: preserve innermost lexical declaration lookup, then
      require exactly one completed target after bounded reference-chain
      resolution. An arbitrary query sequence is not ranked by lexical depth.
      This reconciles the older stack-design innermost-match wording; native
      selector evaluation was implemented in the subsequent preparation/runtime
      tasks after target contracts were adopted.
      Scenarios for later design verification: nested same-name declarations
      retain lexical shadowing; deliberate zero/multiple target selections cannot
      establish an active schema; pending resolution cannot establish a scope.
- [x] Fixture: implement original-owner schema scope target admission for a
      schema-language declaration or a named core wrapper with exactly one direct
      schema-language declaration child. Cover correct expanded namespaces,
      empty/multiple/nested-only declaration children, unnamed wrappers, unavailable
      source-position name metadata and retained descendant reference nodes. Verified
      with three focused native tests and 77 declaration/resolution regressions.
      Scenarios for later design verification: admission returns source handles
      without compiling/evaluating or choosing effective scope; runtime consumption
      still requires cardinality, grants, budgets and dependency readiness.
- [x] Decide the native schema scope target shape. Adopted (2026-10-06) through
      the instruction to continue with the recommendation: admit a schema-language
      declaration directly, or a named core `cem:schema` wrapper containing exactly
      one direct schema-language declaration child. Match expanded namespaces,
      not prefix spelling; the wrapper has a nonempty literal core `cem:name`.
      Wrapper bodies are not implicitly compiled as schema fragments.
      Scenarios for later design verification: wrapper selection retains its
      original owner and declaration identity; zero/multiple declaration children
      cannot silently pick the first schema; unrelated subtrees and similarly
      named elements from other namespaces cannot become schema targets.
- [x] Decide readiness behavior for an explicit pending/invalid schema override.
      Adopted (2026-10-06): keep its governed region incomplete until a usable
      schema is available; do not validate it under the inherited schema. This
      is distinct from the separately adopted last-complete package replacement
      policy.
      Scenarios for later design verification: delayed dependencies never yield a
      falsely complete validation result; invalid overrides do not mutate source
      QNames or earlier bindings; region activation follows the chosen policy.
- [x] Fixture: compile an admitted schema scope declaration from its exact source
      handle before lifecycle activation. Retain the original arena and lexical
      bindings; exclude unrelated schema roots and declaration-reference sites
      from compilation instead of picking the first schema in the document.
      Cover selected collection resolution, pending bases/native types, retained
      lexical aliases, source-attributed errors and unchanged legacy compilation.
      Verified with four admission/selected-slot unit tests, 79 reference integration
      tests and 154 document-model regressions; scope activation remains separate.
      Scenarios for later design verification: selecting a later schema compiles
      that declaration only; unresolved declaration dependencies and compilation
      diagnostics retain original source attribution and keep the governed region
      incomplete until its explicit override is usable.
- [x] Fixture: hand captured name metadata to the registered CEM-QL owner and
      prepare schema scopes through bounded native-reference selection, singleton
      target admission and exact compilation. Cover unregistered/foreign owners,
      pending requesting/selected-declaration contexts and name metadata,
      zero/multiple targets, wrong namespaces,
      pending dependencies, hard compile errors, work limits and directed grants.
      Verified with 43 native CEM-QL preparation/handoff/declaration/runtime tests.
      Scenarios for later design verification: preparation retains original owners
      and diagnostics, cannot activate or use an inherited fallback, and retries
      consume current inputs with unchanged authored names.
- [x] Fixture: validate explicitly selected original structural roots under a
      supplied model without copying an arena or validating unrelated source.
      Cover retained target owners/provenance, pending/limited references, inactive
      models and invalid/duplicate root requests before evaluation. Exercise the
      same original-root boundary through the CEM-QL consumer host. Verified with
      82 native CEM-ML and 43 CEM-QL reference/consumer regressions.
      Scenarios for later design verification: empty valid regions stay distinct
      from unavailable regions; source siblings outside the selected roots cannot
      supply violations or consume runtime evaluation work.
- [x] Decide host child-sequence ownership at a child schema override. Adopted
      2026-10-06 by continuing with the recommendation: enclosing-schema host
      attributes/direct child-sequence checks plus child-schema descendant
      validation. An incomplete child keeps overall validation incomplete.
      Scenarios for later design verification: parent child-count/relationship
      checks are attributed to the chosen schema; child readiness cannot produce
      false complete parent validation or silently use inherited descendants.
- [x] Fixture: validate caller-declared child schema regions over original source
      handles in one retained traversal. Keep host attributes/child-sequence rules
      on the enclosing model; use the child model for descendants and restore the
      enclosing model for siblings. Cover authored and referenced boundaries,
      pending/invalid overrides without inherited evaluation, native attributes,
      parent diagnostics and shared traversal budgets under selected subtrees.
      Verified with native retained-region and preparation fixtures, including
      nested restoration, wildcard boundaries, empty enclosing models and descriptor
      preflight. CEM-QL accepts explicit preparation snapshots; automatic property
      recognition and runtime scope/policy installation remain below.
      Scenarios for later design verification: per-placement models do not copy
      source arenas; pending child bodies cannot appear empty to parent counts;
      the CEM-QL adapter accepts prepared models only when their readiness gate passes.
- [x] Fixture: decode existing host schema-source/selector attributes from original
      handles and captured expanded names, then prepare literal and native selectors
      through the shared bounded CEM-QL resolver and exact schema admission. Cover
      canonical unqualified controls, namespace aliases/rebinding, mutual exclusion,
      URI retention without query handling, pending contexts, singleton failures,
      directed grants, nested chains and source-attributed selector errors. Keep
      wrapping/sibling forms, runtime policy installation and behavior dispatch as
      separate lifecycle work. Verified with 141 native reference/consumer tests,
      including retained imported XML control decoding without attribute retyping
      and preservation of diagnostics attributed to foreign original sources.
      Scenarios for later design verification: decoding never evaluates or copies
      source; literal and native selectors share cardinality/readiness rules; URI
      sources cannot become query text; an unavailable explicit host override cannot
      silently validate its body under the inherited schema.
- [x] Decide validation ownership for recognized `schema-select` / `schema-src`
      host attributes before full host-region wiring. Adopted 2026-10-06 by
      continuing the recommendation: shared schema-control validation, with
      ordinary host attributes validated by the enclosing application schema.
      Source controls remain authored attributes; their shape/selector readiness
      cannot be delegated to each application's string/node declaration.
      Scenarios for later design verification: native and literal control forms
      cannot spuriously fail an application string/node contract; unrelated foreign
      attributes remain ordinary data; control validation still checks exclusivity,
      value shape and readiness without losing original source handles.
- [x] Fixture: validate shared host schema-control contracts over original source
      handles and connect prepared control snapshots to retained region validation.
      Exclude recognized control attributes from application type/unknown-attribute
      checks and ordinary native-attribute consumption; retain source and presence
      metadata. Cover pending/invalid selectors and URI loading without fallback,
      nested/sibling restoration, referenced hosts, ordinary attribute violations,
      foreign lookalikes, conflicting controls and repeat validation with new inputs.
      Implemented original-owner control contracts and explicit host-region snapshots.
      Verified with 148 native reference/consumer tests and 154 document-model
      regressions (302 tests), including equal node IDs in distinct arena owners.
      Scenarios for later design verification: an invalid control cannot hide behind
      an application declaration; control exemption is owner-checked metadata rather
      than a copied/filtered AST; pending bodies cannot appear empty to host counts;
      preparation diagnostics preserve selector attribution in selected placements.
- [x] Fixture: discover and prepare host schema controls on entry to the retained
      validation walk, including original hosts and hosts selected through references.
      Prepare each original host once per invocation, retain inspection snapshots,
      and skip blocked descendants and unrelated roots without a discovery rewalk
      or a second evaluation of structural references. Cover nested restoration,
      repeated selections, empty enclosing models, pending names/contexts, retries,
      denied/limited selections and selector diagnostics over original owners.
      Retain authored attribute target subtrees without discovering their controls.
      Implemented on-entry scheduling and per-call inspection snapshots, sharing
      immutable compiled models while keeping source arenas and placements original.
      Verified with 157 native reference/consumer tests and 154 document-model
      regressions (311 tests), including foreign-host metadata rejection.
      Scenarios for later design verification: discovery never scans inactive or
      unselected source; structural budgets and active reference identities survive
      selector preparation; preparation uses current explicit lifecycle contexts,
      adds no grants, and never persists runtime targets or invented scope IDs.
- [x] Fixture: prepare child runtime inputs from a ready original host/schema
      snapshot and an explicit caller context. Derive the child reference policy
      from its enclosing host policy, retain missing-context readiness and original
      policy errors, and reject foreign host preparations. Cover inherited bounds/
      diagnostics, child overrides, malformed declarations, pending/invalid selectors,
      absent contexts, distinct caller inputs and unchanged existing expression scopes.
      Implemented immutable runtime-input snapshots; custom diagnostic definitions
      inherit intact. Verified with 174 native policy/reference/consumer tests.
      Scenarios for later design verification: preparation does not execute selectors,
      allocate/assign scopes, create grants or mutate source; missing caller context
      does not silently inherit runtime inputs; invalid policy cannot become usable
      by inheriting the enclosing policy; compiled models and source owners stay shared.
- [x] Decide how selected child policies combine with already-captured occurrence
      policies before installing runtime inputs. Adopted 2026-10-06: preserve explicit
      local occurrence overrides while applying child policy to inherited settings.
      Track declared/inherited provenance rather than comparing effective values.
      Existing complete-policy handoffs retain their explicitly supplied policies;
      provenance-aware handoff supplies local declarations or explicit inheritance.
      Scenarios for later design verification: source-position bindings and runtime
      contexts remain caller-owned; local precedence is deliberate; a handoff never
      resets an active request's work/depth budget or creates crossing authority.
- [x] Fixture: retain explicit/inherited policy provenance in lexical runtime
      scopes and expose a provenance-aware captured occurrence handoff. Record schema
      declaration source maps or caller origin per declared depth/work/disposition;
      apply only declared local settings to a new enclosing child policy. Preserve
      legacy complete-policy handoff and original context/crossing semantics. Cover
      explicit values equal to defaults, partial inheritance, neutral resets,
      malformed declarations and immutable enclosing records/source nodes.
      Implemented per-setting origins, lexical parent metadata and partial-policy
      registration/capture handoff; existing complete policies stay explicit.
      Scenarios for later design verification: child defaults cannot erase local
      declarations or mistake equal values for inheritance; missing occurrence
      context stays pending; provenance is metadata, not a copied AST or context ID.
- [x] Fixture: register ready child runtime frames and replay captured local policy
      declarations onto fresh occurrence frames in lexical order. Require explicit
      occurrence context, preserve its pending state, retain the original frames and
      relationship boundary, and leave source assignment to explicit lifecycle handoff.
      Cover inherited facets, explicit defaults, nested declaration origins, bounded
      evaluation after explicit assignment, foreign frames and independent roots.
      Implemented fresh frame registration and ordered declaration replay; native
      consumption verifies pending contexts, effective policies and request work caps.
      Scenarios for later design verification: scope registration never grants a
      crossing or rewrites captured frames; missing context cannot borrow a ready
      child context; replay preserves request limits; unrelated roots need an explicit
      handoff rather than an inferred shared relationship.
- [x] Fixture: hand off entered native resolver handles before effective-scope
      activation, after work and existing edge checks. Verify one root preparation,
      rejected roots, denied/exhausted targets, post-handoff edge authorization and
      fresh destination limits. Restore invocation bindings after a handoff error
      or caller unwind, retaining earlier frame handles for inspection.
      Implemented an optional native lifecycle handoff with reauthorization after
      scope changes, plus invocation restoration on rejection and caller unwind.
      Scenarios for later design verification: preparation preserves original node
      kind/identity and ownership, never grants authority, and cannot replenish
      active request/ancestor budgets or invoke contexts on rejected descendants.
- [x] Fixture: connect prepared runtime frames to automatic host body/occurrence
      binding and combined region readiness through an explicit caller context hook.
      Assign fresh frames while preserving source-position bindings/local provenance,
      restore invocation mappings, and block missing contexts/invalid policy without
      inherited or previously active body fallback. Cover nested/sibling restoration,
      selected hosts, repeat calls with new inputs, inherited/declared facets and
      request/destination budgets while preparing frames during selected traversal.
      Implemented explicit runtime-region validation, lazy local frame replay and
      invocation-local source assignments. Native fixtures also verify enclosing
      host attributes, body general expressions and authored target descendants.
      Verified with 195 native resolver, policy, schema and retained consumer tests.
      Scenarios for later design verification: fresh frame registration precedes
      body consumption; original records/source names remain immutable; invocation
      cleanup cannot invalidate retained result inspection or leak old contexts into
      retries; body handoff never creates grants or resets work accounting.
- [x] Decide behavior selector execution across child consuming-model boundaries.
      Adopted 2026-10-06: retain navigation across the authorized forest and filter
      execution to placements governed by the behavior's consuming model. Returned
      placements governed by another model do not invalidate selection; invalid
      source/stage handles and non-element selection remain errors.
      Scenarios for later design verification: relationship checks keep consumed
      parent/child navigation, while selector navigation cannot apply another
      region's behavior contract to a candidate.
- [x] Fixture: verify behavior-region admission and completion independently of
      candidate count: reject invalid/repeated placement domains and non-node
      selection, compile empty ready child contracts, defer incomplete forests,
      retain unsupported-evaluator incompleteness without compatibility fallback,
      and restore invocation assignments if a region evaluator unwinds.
      Verified admission, unsupported completion, empty-contract compilation and
      consumer-unwind restoration in native region/retained behavior fixtures.
      Scenarios for later design verification: missing executable support cannot
      become an empty pass; compilation does not depend on matching candidates;
      failed behavior execution cannot leak body scopes into the next invocation.
- [x] Fixture: connect retained behavior validation to each placement's consuming
      model and active runtime frame. Retain original declaring/consuming contexts,
      native candidate nodes and already-consumed attribute values, and execute
      before invocation scope restoration. Cover nested/sibling models, repeated
      selected sources, model-specific behaviors/diagnostics and blocked regions.
      Implemented per-model placement routing over the complete authorized forest,
      native region behavior evaluation and explicit runtime-stage integration.
      Ready empty models compile once; incomplete forests defer behavior. Verified
      with 215 native policy, reference, retained consumer and behavior tests.
      Scenarios for later design verification: behaviors use the actual consuming
      model without losing original source ownership; pending regions cannot pass
      or borrow an enclosing behavior contract; behavior dispatch does not resolve
      structural/attribute references a second time or reset active traversal limits.
- [x] Fixture: activate established wrapping schema `src`/`select` controls
      through shared body-control decoding and the entered-boundary runtime
      scheduler. Cover original captured namespace names, literal/native selectors,
      nested restoration, empty explicit bodies, pending contexts and URI loading,
      recognized control ownership, original CEM/XML body-form capture (including
      empty wrapping versus self-closing forms), repeated reference-selected
      wrappers and native per-model behavior dispatch. Preserve original handles, local policy overrides
      and active traversal budgets; leave no-body sibling/prelude scheduling separate.
      Completed on 2026-10-06: 232 native reference, lexical capture, schema
      consumer and retained behavior tests pass, including eight new fixtures.
      Scenarios for later design verification: a named declaration is not a switch;
      foreign schema/select lookalikes remain ordinary data; unavailable explicit
      overrides never borrow inherited models or contexts; consumed wrappers retain
      enclosing contracts and descendants use only the selected model.
- [x] Fixture: decode no-body sibling schema controls through the shared original
      source contract before wiring activation. Expose body versus following extent,
      retain literal/native selectors and URI sources, require original source-form
      metadata, preserve host/body APIs and namespace distinctions, and reject
      conflicts or malformed slots without evaluating queries or installing scopes.
      Completed on 2026-10-06 with pure extent/slot/namespace fixtures and a guard
      rejecting following contracts supplied to legacy explicit body-region APIs.
      Scenarios for later design verification: switch decoding does not choose which
      model governs the switch node; named declarations without a source do not
      switch; repeated original selectors remain execution-local consumer requests.
- [x] Fixture: preserve invalid-target diagnostics when a structural reference host
      cannot supply the original retained target handle after resolution. Model
      routing must not panic or borrow a selected scope for an unsupported target.
      Verified the missing-metadata panic before the fix and attributed invalid
      admission afterward with the original-node fixture.
      Scenarios for later design verification: malformed consumer handoffs remain
      attributed hard errors and do not bypass original-node admission.
- [x] Adopt following-switch activation after the control node (2026-10-06):
      the control node keeps the enclosing model/context and the selected scope
      governs subsequent siblings and descendants until the containing scope ends.
      Align the design table with the established original-source handoff.
      Scenarios for later design verification: selectors and ordinary control-node
      attributes use the preceding scope; earlier siblings cannot acquire later inputs.
- [x] Fixture: activate no-body sibling controls at entered original boundaries.
      Cover literal/native selectors, enclosing switch-node contracts, original sibling
      order, nested and body-region restoration, pending regions without fallback,
      repeated reference-selected subtrees, directed grants, policy limits and
      per-consuming-model behavior dispatch. Keep prelude and URI loading separate.
      Completed on 2026-10-06: 242 focused native tests pass, including ten new
      control, runtime, compatibility and behavior fixtures. CEM/XML retries restore
      assignments and consume fresh explicit contexts without source target writeback.
      Scenarios for later design verification: active traversal accounting survives
      switches; an incomplete following region cannot borrow prior contexts/models;
      retries restore original assignments and use fresh caller inputs; declaring
      owners and repeated placements retain their provenance.
- [x] Fixture: capture original document `@schema` directive form and decode
      existing literal URI/selector payloads through the shared following contract.
      Cover default namespace independence, original source handles, missing form,
      empty/conflicting/repeated sources and compatibility with body-only APIs.
      Scenarios for later design verification: opaque directive text is not a typed
      reference slot; decoding does not install scopes or synthesize AST attributes.
- [x] Fixture: preserve the consuming model for reference-selected targets whose
      original enclosing region differs from their consumed placement. Apply entered
      following controls along actual owning child edges within selected subtrees;
      retain original destination contexts and traversal limits independently.
      Scenarios for later design verification: an outer source default cannot replace
      a child consumer contract; authored controls inside a selected subtree still
      govern its following siblings without moving or cloning source nodes.
- [x] Fixture: activate entered literal schema preludes through existing retained
      runtime stages. Cover document scopes and nested body restoration, enclosing
      selector contexts,
      pending inputs and URI readiness, fresh invocation retries and native
      consuming-model behavior domains. Existing following-region regressions retain
      coverage for repeated selected subtrees, authorization and traversal limits.
      Scenarios for later design verification: a prelude starts after its original
      source control and ends with its block; unresolved sources never borrow an
      inherited model/context or mutate captured lexical records.
- [x] Fixture: connect existing document-prelude schema controls to the
      entered-boundary runtime scheduler. Reuse captured original boundaries and
      the shared admission/readiness/frame/behavior stages, preserve earlier source
      names and local policy provenance, and restore enclosing/following regions.
      Cover ready and pending literal selectors/native query results, nested
      restoration, original owners and explicit caller contexts. Keep the enclosed
      child-reference syntax decision deferred. Completed literal document controls
      on 2026-10-06 using original text payload occurrences and shared following
      stages. Verified with 246 focused native reference, schema, lexical-capture
      and retained behavior tests, including four new fixtures. Source capture remains passive; block parsing is deferred as recorded below.
      Scenarios for later design verification: control forms share the same reference
      consumer contracts without introducing a second AST projection; pending
      overrides cannot borrow inherited or previously active models/contexts; lexical
      names and active traversal budgets survive boundary changes.
- [x] Decide block-directive recognition before changing source parsing. The syntax
      guide describes `@schema` block-prelude shorthand, but the tokenizer currently
      retains it inside block content as ordinary text. Adopted on 2026-10-06:
      defer block parsing and retain its current text behavior until line-position
      recognition, literal-text escaping and compatibility are specified.
      Scenarios for later design verification: block content is not silently
      reclassified; nested directives end with the containing scope; existing host
      and schema-element controls remain available without a new delimiter.
- [ ] Specify block-directive recognition and literal-text compatibility before
      enabling the documented block-prelude shorthand. Cover line positions,
      escaped directive-looking text, comments, nested restoration and existing
      content behavior. Keep host/schema-element controls available meanwhile.
      Scenarios for later design verification: defaults do not reclassify literal
      content; blocked bodies do not execute directives; source provenance remains
      original and earlier namespace bindings are not renamed.
- [x] Fixture: hand off explicit schema URI loader outcomes for an original control
      and exact authored URI. Cover pending/unavailable/invalid/complete states,
      original target handles, no query parsing or I/O on handoff, directed grants,
      bounded chains, cardinality, admission, source names, context readiness and
      incomplete/hard-invalid declaration compilation. Implemented on 2026-10-06
      with four new preparation fixtures and verified in 253 focused native tests.
      Scenarios for later design verification: loading does not grant crossings or
      activate scopes; the loader owns relative URI/public-part selection; stale
      completed snapshots are inspectable but cannot replace a current pending load.
- [x] Fixture: activate ready URI-backed host/body/following/prelude regions through
      the established invocation stages. Cover fresh load retries, failed/pending
      resources without fallback, nesting, consuming-model behavior and restoration
      of original owners, scopes and authored target slots. Implemented on
      2026-10-06 with two runtime fixtures and a retained behavior fixture;
      verified in 253 focused native tests.
      Scenarios for later design verification: the same URI in another control or
      host cannot borrow a load result; URI ingestion retains request/destination
      caps and declaration compilation readiness; source-only results never activate.
- [x] Decide the byte loader's default target-selection contract for schema URIs
      without a public fragment. Select exactly one direct schema declaration or
      named schema wrapper, rejecting zero/multiple candidates.
      Adopted on 2026-10-06: select exactly one direct target. Public fragment
      interpretation remains loader-owned.
      Scenarios for later design verification: an assembly with several schema
      declarations cannot select an arbitrary first declaration; unrelated content
      cannot become a scope target; CEM-QL does not implement URL/ID lookup.
- [x] Fixture: import CEM/XML schema bytes once with retained lexical metadata;
      prepare policy-controlled URI requests and exact singleton direct target
      selection. Cover explicit public-part callbacks, redirects, MIME/byte bounds,
      denial/substitution, abort, malformed input and original owner/source retention.
      Implemented on 2026-10-06 with five native loader/import fixtures; verified
      in 267 focused native reference, import, schema and behavior tests.
      Scenarios for later design verification: native import never reserializes an
      AST; public part callbacks cannot return a different owner; zero/multiple
      direct targets cannot activate; transport policy does not grant scope access.
- [x] Fixture: bind resolver completions to original control/URI generations.
      Reject stale/foreign completions before import or context preparation; cover
      pending retries, I/O/import failure, destination context readiness, explicit
      lexical handoff and grant-controlled invocation activation. Implemented on
      2026-10-06 with three native bridge fixtures; verified in 267 focused tests.
      Manual publication/removal also supersedes outstanding generation tickets.
      Scenarios for later design verification: a late response cannot replace a
      newer load; completion never evaluates declarations; original target handles
      and authored expressions survive imports and subsequent region execution.
- [x] Fixture: connect resolver/resource lifecycle outcomes and shared retained
      CEM import to the implemented schema URI handoff under the adopted direct
      singleton target default. Retain original loaded owners, lexical metadata and
      explicit contexts; preserve loader policies, abort/error states and directed grants.
      Cover relative URIs, public exports, fresh revisions and per-control retries.
      Implemented explicit begin/read/import/complete APIs on 2026-10-06 and
      verified in 267 focused tests. Automatic engine queue dispatch remains below.
      Scenarios for later design verification: byte completion alone does not
      activate a schema; public IDs do not authorize scope crossings; pending
      imports or missing metadata never borrow a prior model or caller context.
- [x] Fixture: add opt-in resumable native input validation sessions. Yield typed
      schema resource requests from CPU stages; dispatch reads on the engine I/O
      executor and resume the retained session on CPU without holding a CPU slot
      during I/O. Keep synchronous stages compatible. Adopted on 2026-10-06.
      Cover multiple rounds, original owner identity, stable diagnostic provenance,
      failed reads, cancellation, memory accounting and invalid/no-progress yields.
      Implemented on 2026-10-06 with eight native engine fixtures, including CPU
      availability during I/O and cumulative payload retention across rounds.
      Scenarios for later design verification: one CPU worker can serve another
      document while a session awaits I/O; pending sessions retain original owners;
      cancellation prevents import/resume; resource accounting never resets on retry.
- [x] Fixture: provide a reusable retained CEM-QL validation session over the
      engine suspension boundary. Discover URI loads from entered region reports,
      retain original control generations, prepare caller-supplied destination
      contexts/local bindings/grants and retry the same source after completion.
      Cover nested controls entering in successive rounds, unavailable contexts,
      denied crossings and failed reads without inherited fallback. Implemented
      on 2026-10-06 with two bridge fixtures covering CEM/XML input and schema
      owners, declaration references, child-policy inheritance and explicit MIME hints.
      Scenarios for later design verification: blocked descendants are not fetched;
      byte completion alone does not activate; no callback synthesizes context IDs;
      original owners and unresolved authored reference nodes survive every retry.
- [x] Fixture: connect entered URI-backed schema regions to the engine external
      resource queue using native request/generation/import APIs. Schedule only
      entered controls, hand off explicit destination/local contexts and directed
      grants, and retry region readiness after queue completion. Preserve loader
      operation cancellation and bounds; retain prior original owners only under
      the operation's existing lifetime/memory contract. Implemented on 2026-10-06
      through the opt-in session contract and reusable CEM-QL session. Native
      coordinator reads run on I/O and imports/readiness retries run on CPU.
      Verified in 327 focused native reference, import, engine, scheduler and
      retained-behavior tests, including ten new resumable-session fixtures.
      Scenarios for later design verification: earlier/unentered controls are not
      fetched; late replies cannot restore canceled/replaced generations; current
      pending dependencies block model behavior without inherited fallback; retries
      preserve lexical names, source owners and request/destination traversal caps.
- [x] Fixture: broaden resumable queued schema loading across wrapping controls,
      following sibling switches, document preludes and reference-selected foreign
      subtrees. Include explicit public exports and multi-request batches with
      stable completion order under existing queue/overflow policies. Reuse the
      session's native controls, original-owner URI bases and generation checks.
      Implemented on 2026-10-06 with eight bridge fixtures covering CEM/XML,
      public exports, foreign owners, failed peers and block/spill/reject queues.
      Verified with 336 focused native reference, scope, lifecycle and engine tests.
      Scenarios for later design verification: a foreign placement resolves relative
      URIs against its authored owner; public parts do not grant crossings; following
      switches stay enclosing; blocked controls are not fetched; peer failures never
      hide pending regions or overwrite original diagnostics.
- [x] Fixture: preserve original owner URIs and authored coordinates for structural
      errors in queued reference-selected foreign subtrees. Cover required and
      unknown attributes and parent/child relationships across owners; never infer
      owners from document-local source IDs or effective schema frames.
      Include direct CEM-QL structural-consumer fixtures with a foreign effective
      scope, imported event coordinates and already-attributed diagnostics.
      Implemented on 2026-10-06 through the retained structural diagnostic hook,
      with direct CEM-QL and queued CEM/XML foreign-owner fixtures. Original builder
      positions and imported event coordinates supply reporting locations; source
      stacks and existing explicit attribution remain intact.
      Scenarios for later design verification: schema loading/retries do not stamp
      the caller URI onto foreign errors; relationship errors belong to the child;
      native source frames and existing explicit diagnostic attribution survive.
- [ ] Specify typed prelude reference slots only if that source capability is
      requested. Current directive payloads are literal text; native reference and
      expression slots already belong to schema-element/host attribute forms. Keep
      this separate from the deferred enclosed child-reference syntax decision.
      Scenarios for later design verification: a literal query constructor is not an
      authored AST reference slot; consumers retain original source occurrences and
      do not manufacture attributes or reparse a second CEM tree.
- [x] Fixture: integrate bounded schema-scope selection, target admission and exact
      declaration compilation into lifecycle region readiness. Require one completed
      target and a complete model without hard compilation diagnostics before
      activation; pending/invalid explicit overrides keep their region incomplete.
      Supply original source-position name metadata and keep literal forms compatible.
      Shared control ownership, explicit prepared host-region validation and
      on-entry discovery/preparation of original and selected hosts are implemented.
      Runtime-input preparation, policy provenance, fresh frame registration and
      automatic entered-body/occurrence binding with combined readiness and invocation
      restoration and per-consuming-model retained behavior dispatch are implemented.
      Wrapping body controls now use original CEM/XML source forms and the same
      runtime handoff. No-body sibling controls now activate after the original
      control, using explicit runtime inputs and original owning positions. Existing
      literal document-prelude controls now use the same following stage.
      Retained URI loader-result preparation and controlled native byte loading/import
      are implemented, with the direct singleton URI target default adopted.
      Opt-in native engine resource dispatch, retained imports and entered-region
      readiness retries are implemented on 2026-10-06. Block parsing stays deferred.
      Reuse the implemented preparation, loader and region APIs; do not add an AST projection or choose
      the deferred enclosed child-reference syntax as part of this wiring.
      Scenarios for later design verification: no inherited fallback hides an invalid
      override; retries keep earlier lexical bindings; leaving a completed child
      override restores the enclosing schema; zero/multiple targets never activate.
- [x] Fixture: capture immutable CEM element/attribute expanded-name metadata at
      each authored name event and associate it with original builder IDs. Expose
      owner-checked lookup and imported XML names through the same capture handle.
      Cover prefix/default rebinding, child restoration, boolean/native attributes,
      unresolved prefixes, folded expressions and rejection of another arena owner.
      Verified with CEM/XML capture fixtures and admission/declaration-resolution
      regressions. Lookup supplies namespace metadata, not runtime schema readiness.
      Scenarios for later design verification: schema admission consumes real source
      namespace identities without rewriting nodes or applying a later binding;
      XML namespace and attribute rules stay those of its shared importer.
- [x] Specify namespace target-kind/readiness contracts independently of the
      implemented schema preparation handoff. Distinguish inline schema wrappers,
      schema-language declarations and namespace binding/context identities;
      do not treat context record IDs as AST node IDs or silently extract a URI
      from an arbitrary selected node. Preserve existing literal forms.
      Adopted on 2026-10-06: namespace selection admits only explicit namespace
      bindings/declarations; schema declarations are not namespace targets. Require
      one completed target with original-owner binding metadata and an available
      URI before activation. Preserve empty default-namespace resets and the
      destination's own prefix designation. Keep enclosed syntax deferred.
      Scenarios for later design verification: wrong target kinds cannot establish
      a scope; original owners and source provenance survive consumption; namespace
      completion does not retroactively rename already expanded source QNames.
- [x] Fixture: capture original namespace declaration nodes and parser binding
      metadata through CEM/XML source events. Admit only owner-checked explicit
      declarations, preserving source positions, rebinding, default alias/reset,
      child restoration and decoded XML values. Reject schema/data lookalikes and
      declarations with no completed binding; never infer targets from source IDs.
      Include given root namespace defaults without synthesized source declarations.
      Implemented on 2026-10-06 with six native fixtures and 218 focused namespace,
      reference, scope and lifecycle regressions. The retained admission API uses
      original parser/import binding records; `@default ""` now clears its default
      instead of copying the prior blank-name binding. Pending declaration/use
      capture, completed query views and namespace property execution remain separate.
      Scenarios for later design verification: selecting an earlier declaration
      keeps its earlier URI; sibling contexts can reuse local binding IDs without
      merging targets; XML namespace attributes remain available by source handles
      even when omitted from the normalized query tree; no target admission evaluates
      a reference, authorizes a crossing or rewrites captured QNames.
- [x] Decide the completion boundary for pending namespace-property references.
      Namespace QNames currently expand during parsing, while native references
      are evaluated at consumer lifecycle stages. Choose whether to retain the
      pending source-position binding identity and defer dependent QName completion,
      or require the namespace target to be ready during parsing. Adopted on
      2026-10-06: retain the original pending binding identity and complete dependent
      QNames when that same binding resolves. Completed names stay fixed. Completion
      belongs to an explicit consumer lifecycle, not loading or later lexical rebinding.
      Scenarios for later design verification: completing the same pending binding
      is distinct from adopting a later declaration; completed names remain fixed;
      retry never borrows a later alias or an inherited URI; root contexts need no IDs.
- [x] Decide how completed pending QNames are exposed to CEM-QL consumers.
      Adopted on 2026-10-06: expose completed expanded names through a per-execution
      native query view, retaining the original AST and fixed completed source names.
      Current captured-name handoff does not yet change the registered query tree.
      Scenarios for later design verification: authored source inspection stays
      available; completion in one execution cannot alter another query's names.
- [x] Fixture: capture pending namespace declaration/use associations and implement
      independent completion views over one original owner, then expose them through
      the adopted native query view. Adopted on 2026-10-06: require completed
      namespace dependencies in the selected query subtree before executable
      resolved-name view construction. Authored source inspection stays available.
      Cover inherited masks, default aliases of pending declarations, literal resets,
      independent executions, original-owner checks and selected-subtree boundaries.
      Scenarios for later design verification: no source arena or completed name is
      rewritten; independent executions can resolve one pending declaration differently;
      QName lookup never substitutes a later alias; missing completion remains pending.
- [x] Fixture: exercise per-execution namespace query views in
      `namespace_name_views`: completed field/axis navigation, unchanged authored
      source inspection, independent execution identities, native `#` operands,
      retained source metadata and rejected owner mismatches.
      Scenarios for later design verification: selected roots cannot navigate to
      incomplete siblings; reference descendants remain authored native nodes.
- [x] Fixture: add bounded CEM-QL namespace selection over original declaration
      handles in `namespace_scope_preparation` after adopting the completion boundary.
      Reuse request/destination
      limits, directed grants, singleton admission and pending/unresolved outcomes;
      consume original captured binding metadata without schema URI extraction.
      Implemented: `attach_captured_namespaces` and `prepare_namespace_scope`
      retain original owners and expose metadata/binding/context readiness apart
      from graph failures and singleton target admission. XML namespace attributes
      keep original handles and decoded binding values. Selected pending declaration
      value consumption remains part of namespace-property lifecycle activation.
      Scenarios for later design verification: zero/multiple targets do not activate;
      denied crossings retain source diagnostics; missing runtime input stays pending;
      independent consumers can select different namespace declarations from one source.
- [x] Fixture: integrate ready namespace completion views into explicit lexical
      handoff in `namespace_lexical_handoff` and `pending_namespace_names`.
      Retain original occurrence snapshots
      and selected-subtree boundaries; supply completed prefix bindings only to the
      execution that resolved their original declarations. Preserve source metadata
      and fixed completed names without replacing the original registered arena.
      Implemented: `lexical_snapshot` and
      `attach_completed_namespace_lexical_scopes` preflight every selected
      occurrence and its pending prefixes before preparation or mutation. Ready
      URI overlays retain original declaration handles and saved scalar snapshots;
      contexts and local policy overrides remain caller supplied. General attribute
      expression wrappers preserve their intrinsic native names; their captured
      namespace dependencies still must complete. Unselected occurrences, existing
      relationship grants and original captured names remain unchanged.
      Scenarios for later design verification: unavailable namespace dependencies
      block executable name views; retries never borrow later bindings; independent
      executions sharing one source can expose different completed namespaces.
- [x] Decide captured-expression namespace readiness for completion handoff.
      A selected forest may have no dependent QName but its expression occurrences
      can retain pending prefix bindings. Require every captured pending binding
      before constructing a namespace-aware context (recommended), or preserve
      unused pending bindings and check them only on namespace-sensitive access.
      Adopted on 2026-10-06 through the user's instruction to continue recommended:
      require every captured pending binding before preparing a namespace-aware
      expression context, even when the selected forest has no dependent QName.
      Scenarios for later design verification: pending declaration selectors can
      run in their original pre-declaration context; unused prefix dependencies
      cannot be mistaken for completed namespaces or borrow inherited/later URIs.
- [x] Fixture: add immutable declaration binding lookup to namespace completion
      metadata in `pending_namespace_names`. Resolve original pending aliases,
      expose completed literal/native URI values and retain individual pending
      outcomes without constructing expression contexts or changing source records.
      Scenarios for later design verification: a pending binding outside the selected
      forest can be a captured dependency; literal resets remain ready; arbitrary
      nodes and foreign owners cannot masquerade as binding declarations.
- [x] Fixture: integrate explicit ready namespace completion views into shared
      native query ingress in `namespace_query_ingress`. Keep authored source
      inspection available, require the
      matching original source owner and preserve selected-subtree boundaries.
      Supply completion at the consumer lifecycle stage without inventing grants
      or evaluating declarations during ordinary loading/inspection.
      Implemented: `QuerySourceOwner::NamespaceCompleted` carries the matching
      original retained tree and completion through shared preparation;
      `run_query_with_source_owner` retains shared query controls without loading
      or reparsing input bytes. The QL adapter projects selected roots in order,
      preserving empty forests, native reference targets and authored source
      access. Independent completions keep fixed names and original metadata;
      owner mismatch rejects before query compilation. Captured-expression
      execution still requires its separate lexical handoff. XPath/CSS retain
      their existing lifecycle admission, cancellation and result budgets remain
      active, and ordinary `run_query` preserves loading/capture behavior.
      Scenarios for later design verification: the same source supports independent
      executions; missing dependencies cannot expose guessed expanded names; native
      references and source metadata retain their original owner through ingress.
- [x] Define and implement native reference consumption by specialized schema and
      namespace scope properties after their target-kind, cardinality and readiness
      contracts are specified. Reuse existing capture/handoff and bounded outcomes;
      preserve established literal selectors and source-position binding semantics.
      The exact enclosed child override syntax remains deferred in the roadmap.
      Implemented for established forms: schema controls support literal/native
      selectors, URI loader handoff, singleton target admission, exact compilation
      and retained runtime regions; native namespace attributes support bounded
      value consumption, singleton binding admission, explicit completion/activation
      and shared query ingress. Completed-name handoff into schema consumers,
      completed-declaration publication and automatic namespace coordination remain
      separately tracked below; this does not claim those integrations are complete.
      Scenarios for later design verification: pending scope selection cannot
      silently adopt a later binding; invalid property targets cannot establish a
      scope; completed child overrides restore enclosing defaults on exit.
- [x] Fixture: consume original native namespace-property value slots at an
      explicit lifecycle stage in `native_namespace_properties` and
      `namespace_property_preparation`. Reuse bounded graph traversal and original
      occurrence contexts, require one ready namespace declaration target, and
      retain pending/invalid outcomes before producing completion metadata.
      Implemented: `decode_native_namespace_property` retains the original
      declaration, destination prefix and owning reference/general-expression
      slot; `prepare_namespace_property` evaluates that slot through one bounded
      traversal and the existing lifecycle expression hook. Namespace admission
      reuses the same selection without resetting bounds. Reports preserve
      property/metadata issues, missing inputs, pending target bindings, source
      diagnostics and singleton failures. Ready results can supply name completion
      under the original declaration ID; selection results are never cached or
      written to source. Literal/default-alias forms remain unchanged and selected
      expression-looking nodes are not implicitly executed. Ready-result activation
      is implemented below; publication/consumption of pending selected namespace
      declarations remains separate work.
      Scenarios for later design verification: pending declaration selectors use
      pre-declaration bindings; invalid cardinality and denied crossings cannot
      supply a URI; repeated consumers never write source targets or share results.
- [x] Integrate prepared namespace-property results with lifecycle activation
      and selected-name completion. Preserve source-position dependencies, fixed
      completed QNames, child shadowing/restoration and pending governed regions;
      reuse explicit lexical handoff and native query ingress for execution.
      Implemented: `activate_namespace_properties` consumes ready original property
      reports and creates this execution's `NamespaceNameCompletion`. Owner,
      duplicate declaration, original property shape, selected-name readiness and
      occurrence assignment/dependency checks finish before callbacks or mutation.
      The callback receives completion metadata for native context bindings and
      supplies runtime readiness/local overrides through existing lexical handoff.
      Missing callback inputs remain pending. Selector-only roots can activate
      before their governed children; already assigned selectors stay excluded
      from later roots. The same completion feeds shared query ingress without
      reevaluating selectors, changing ASTs or manufacturing relationship grants.
      Scenarios for later design verification: earlier and later declarations do
      not rebind each other's saved uses; missing runtime input stays pending;
      child exit restores enclosing defaults without moving original source nodes.
- [x] Fixture: add ready-result namespace-property activation preflight in
      `namespace_property_activation` and connect it to shared query ingress in
      `namespace_query_ingress`. Verify multiple original declarations, missing
      results, duplicate/foreign properties, pending retries and assignment
      conflicts before callbacks or context mutation. Keep completed views
      execution-local, and exercise child selector dependencies and restoration.
      Scenarios for later design verification: incomplete results never install
      partial contexts; selectors retain pre-declaration bindings; activation
      neither restarts traversal nor grants a crossing.
- [x] Fixture: verify namespace-property lifecycle activation through shared
      native query ingress and lexical completion handoff after those integrations.
      Cover independently completed executions, pending retries, pre-declaration
      selectors, general attribute expressions, default aliases/resets and nested
      child restoration. Assert original owners, fixed names, source maps and
      request/destination traversal bounds through the full lifecycle.
      Implemented: `namespace_property_activation` covers missing/unused pending
      dependencies, invalid/foreign/duplicate reports, assignment conflicts and
      retry without partial contexts. `namespace_query_ingress` consumes multiple
      prepared properties, stages child selectors with their original dependencies,
      retains original default aliases and literal resets, restores enclosing
      bindings and exposes independently completed native executions. Pending
      source/target contexts, directed crossings and request/destination work/depth
      failures reject before activation; ready results remain usable without
      selector re-evaluation. Source owners, metadata and fixed names stay original.
      Block-prelude aliases use the established normalized producer fixture;
      this does not extend inline body directive syntax.
      Scenarios for later design verification: unavailable bindings block only
      their governed execution; retries reuse original dependencies; granting a
      crossing does not implicitly complete or activate a namespace property.
- [x] Audit schema and namespace scope-property reference adoption after
      lifecycle integration. Verify coherent original-owner, readiness, bounds,
      grants and diagnostic contracts across native values and existing literal
      conveniences; update the design coverage and remaining action items.
      Keep enclosed child-override syntax deferred in the roadmap.
      Completed: the reference design's scope-property audit now maps original-owner,
      value-form, traversal, target-kind, readiness, activation and lifetime contracts
      to their existing fixtures. Corrected stale implementation-status claims.
      Namespace activation is an explicit execution-host handoff, whereas schema
      runtime validation restores invocation assignments. Query name completion
      does not yet reach schema admission/control discovery or structural input
      attribute-contract lookup; recorded that integration gap separately below.
      Completed namespace declaration reuse and automatic coordination remain open.
      Verification: 92 existing native scope-preparation, runtime-region, namespace
      preparation/activation/handoff, region-behavior and query-ingress tests pass.
      Scenarios for later design verification: supported consumers share native
      reference contracts without rewriting ASTs or inventing context IDs;
      remaining deferred work is distinguishable from implemented behavior.
- [ ] Fixture and implement execution-specific completed-name handoff into schema
      consumers. Route completed expanded names through original owner/node handles
      for target admission, schema control discovery, structural attribute-contract
      lookup and retained behavior views. Preserve fixed names, source inspection,
      selected-forest boundaries and consuming-model ownership; missing dependencies
      must keep the affected region incomplete. Keep original captured tables and
      AST records immutable and independent executions isolated.
      Admission/control handoff implemented: `with_completed_namespace_names`
      supplies one registered owner's selected completion only during the callback;
      `consuming_expanded_name` uses it for schema admission and established control
      discovery, while `captured_expanded_name` stays immutable. Nested same-owner
      invocation temporarily replaces the active forest; other owners retain theirs.
      Return/error/unwind restore previous completion. Structural attribute lookup
      now uses the shared input-name hook on owning/selected placements; pending
      names defer shallow presence/field checks. Retained behavior name projections
      remain the separate unchecked task below.
      Scenarios for later design verification: a namespace-completed schema target
      can be admitted without using its raw lexical prefix; completed core aliases
      identify controls while foreign aliases remain data; names outside a selected
      completion cannot borrow its bindings; repeated source reuse keeps distinct
      execution names and original source maps.
- [x] Fixture: add invocation-scoped completed-name admission and control discovery
      in `namespace_schema_names`. Supply ready completion over a registered original
      owner, preserve the captured-name getter, and restore completion on return,
      error and unwind. Exercise schema target kinds, pending/outside names, core
      versus foreign control aliases, wrapper/following forms, owner mismatch,
      independent/nested executions and directed grants/traversal bounds.
      Verification: 102 focused native capture, name-handoff, scope preparation,
      runtime-region, namespace activation/handoff, behavior and query-ingress
      tests pass. Retained behavior projection migration remains open below.
      Scenarios for later design verification: callback name completion supplies
      no runtime inputs, crossing authority or declaration readiness; selected
      completions cannot rewrite fixed names or leak into later invocations.
- [x] Fixture: retain original schema body/following producer form while its QName
      is pending in `pending_namespace_names`. Do not infer target kind from a
      lexical prefix; completed core names can later consume the form while foreign
      names remain ordinary data.
      Scenarios for later design verification: an empty explicit body stays wrapping;
      a no-body control stays following after its original binding completes.
- [x] Route completed names through structural input attribute-contract lookup
      after admission/control handoff. Cover original owning and reference-selected
      placements, unavailable names, consuming-model boundaries and bounded native
      attributes without rewriting ASTs or changing lexical datatype behavior.
      Completed: `input_expanded_name` carries invocation metadata into owning and
      reference-selected native attribute eligibility and shallow literal typing.
      Original values/diagnostic owners and local-name conveniences stay intact.
      Missing element/attribute names defer presence/field checks and keep validation
      incomplete. Runtime child hosts forward names and restore original contexts;
      selected attributes share their enclosing reference's grants and work budget.
      Scenarios for later design verification: the same raw prefix can complete to
      an allowed or foreign URI in separate executions; unavailable names cannot
      masquerade as complete validation or use another completion's forest.
- [x] Fixture: cover completed structural attribute names in `namespace_schema_names`.
      Test literal typing, native attribute selection on owning and referenced
      placements, incomplete/outside names, child consuming models and unchanged
      directed grants/work limits. Preserve original source handles and diagnostics.
      Completed: six native fixtures cover literal typing, native slot readiness,
      excluded forests, selected owner reuse, bounded crossings, child model contracts,
      runtime body context restoration and deferred presence checks.
      Verification: 255 focused native capture, namespace, declaration, input validation,
      runtime schema, behavior and query-ingress tests pass.
      Scenarios for later design verification: pending names do not evaluate native
      slots or invent unknown/missing-attribute violations; independent invocations
      can admit or reject the same authored attribute through different namespace URIs.
- [x] Fixture: retain intrinsic CEM namespace-declaration header names in
      `lexical_scope_capture`. Match the completed-name view's XMLNS namespace
      without requiring an authored `xmlns` binding or resolving unrelated unbound
      prefixes. Preserve literal header readiness in engine input validation.
      Completed: original CEM capture recognizes reserved namespace attributes as
      XMLNS names. Ordinary prefixed attributes/elements still require a binding;
      the existing engine retained-attribute fixture remains complete for ready input.
      Scenarios for later design verification: `@xmlns:prefix` is a ready declaration
      name while an unbound element/ordinary attribute stays unavailable.
- [ ] Expose completed names through retained validation/behavior views after
      structural handoff. Preserve original `source` inspection and consumed
      attribute values; keep placement identity and model ownership intact.
      Scenarios for later design verification: behavior navigation sees this
      execution's names; source metadata and authored descendant references remain
      unchanged when the same original node appears in multiple placements.
- [ ] Define per-execution publication and reuse of completed native namespace
      declaration results. Selecting a pending declaration currently remains
      `TargetBindingNotReady`, even after another consumer completed its value.
      Specify how the lifecycle exposes those results to bounded target admission
      without caching them on the source or restarting traversal/dependency budgets;
      decide any additional value-consumption stage before implementation.
      Scenarios for later design verification: a completed declaration can be
      reused only in the execution that supplied its result; an incomplete target
      never borrows inherited/later URIs or another execution's completion.
- [ ] Wire the explicit namespace preparation/activation stages into an opt-in
      document lifecycle coordinator after completed-declaration publication is
      specified. Use caller-provided runtime inputs and original lexical dependencies
      to stage selectors, retry pending regions and hand matching ready views to
      validation/query consumers. Preserve ordinary parse/load/inspect behavior.
      Scenarios for later design verification: waiting releases execution work;
      unrelated ready regions stay consumable; separate instances never share
      mutable namespace results, contexts or source targets.

- [x] Decide the authority source for command/WASM replacement grants before
      exposing them. The native CLI has explicit caller arguments; run configs
      and virtual command request JSON can arrive as data. Adopted on 2026-10-05:
      explicit CLI grant arguments and embedding-host
      `EngineContext` supply authority; loading a config or parsing a request
      must not manufacture a grant. Existing command preparation clones the host
      context and already preserves its scoped grants.
      Scenarios for later design verification: a config or request naming the
      expected owner cannot authorize itself; a trusted host grant remains exact
      across command preparation; standalone CLI intent is expressed separately
      from `--schema-package` manifest inclusion.
- [x] Fixture: cover explicit CLI/host grant handoff, no-grant
      rejection, wrong package/expected origin/replacement URI, built-in versus
      manifest ownership and unchanged active package after rejection. Verify
      config/request admission follows the selected authority contract. Add the
      fixtures before implementation and preserve the existing override outputs.
      Scenarios for later design verification: grants select one expected owner
      and incoming manifest, never a wildcard or inferred manifest-list privilege.
- [x] Integrate scoped replacement grants into native CLI and common command
      context handoff. Do not infer grants from package URI lists or manifest
      contents. Verify explicit caller and embedding-host boundaries.
      Scenarios for later design verification: manifest inclusion does not grant
      authority; expected-owner or incoming-URI mismatches preserve the active
      package; native consumer grants survive the boundary without widening scope.

- [x] Fixture: add native trusted-host configuration decoding and WASM admission
      cases before exposing replacement grants. Cover all exact origin variants,
      unknown fields/kinds, empty identities, omitted setup, host context handoff
      and rejection before callbacks. Verify the generated Node/browser API types.
      Scenarios for later design verification: per-execution host setup neither
      persists into later executions nor grants authority from request/config data.
- [x] Add low-level JavaScript WASM embedding-host configuration for scoped
      replacement grants, mapped into a fresh trusted `EngineContext` before
      callbacks and command preparation. Never populate grants from parsed argv,
      virtual requests or run-config data.
      Scenarios for later design verification: exact package/origin/URI identities
      survive handoff; omitted setup supplies no grants; invalid host control JSON
      fails before callbacks; host setup does not leak between executions.
      Completed (2026-10-06): optional final `host_configuration_json` on
      `executeCommandServiceV1` strictly decodes named JSON objects, rejects
      duplicate/unknown fields and empty identities, and maps exact origins into
      a fresh execution context. It does not load manifests. All 49 native
      command-boundary tests, two TypeScript projection tests and eight WASM
      runtime tests pass. Nx `@epa-wg/cem-ml:verify` passes generated browser/Node
      types, ABI and integrity checks. Worker-client forwarding remains below.
- [x] Fixture: verify constructor host configuration through both worker clients.
      Snapshot before asynchronous startup, mutate the caller's options, and
      confirm the original setup survives repeated executions while a new client
      receives its own snapshot. Request/execute-option lookalikes cannot replace
      setup. Cover the Node service wrapper and public Node/browser option types.
      Scenarios for later design verification: malformed configured authority
      fails before callbacks; existing cancellation and worker-failure paths retain
      their behavior; omitted constructor setup remains compatible.
- [x] Forward trusted host configuration through constructor-owned Node/browser
      worker-client options. Snapshot host setup separately from execution requests,
      transport it through the worker bridge and pass it to the low-level executor.
      Add worker and type fixtures before implementation.
      Scenarios for later design verification: requests/configs cannot override
      host authority; distinct clients retain distinct grant snapshots; callbacks,
      cancellation and worker restarts preserve the exact configured boundary;
      mismatches retain the active package and report attributed diagnostics.
      Completed (2026-10-06): clients capture `hostConfiguration` as JSON before
      asynchronous startup and send it outside request data to the WASM executor.
      The Node service wrapper forwards the same option. Node/browser fixtures
      prove caller mutation cannot replace existing setup, new clients receive
      independent snapshots and request/execute-option lookalikes supply no
      authority. Nx `@epa-wg/cem-ml-cli:verify` passes worker suites, browser
      fixtures, cancellation/failure regressions and public API type checks;
      package lint also passes. The Node suite passes all 19 tests.
      Worker failures close the client; recreation explicitly captures new setup.

- [x] Migrate CLI external CEMT package override fixtures to the scoped grant
      consumer API. Prior broad CLI verification reported
      `convert_schema_package_option_loads_external_cemt_output_artifacts` and
      `convert_cemt_profile_ambiguity_reports_and_explicit_selectors_resolve`
      failing with `cem.schema_package.replacement_not_authorized`. Both now pass
      with exact native CLI grants. Four denied replacement cases preserve the
      active built-in output while reporting the rejection diagnostic.
      Scenarios for later design verification: explicit grants authorize the
      expected owner/origin; listing a manifest alone remains insufficient.

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
- [x] Fixture: inspect a parser-backed cyclic reference graph through retained
      native nodes, AST-vocabulary rows and DOM diagnostics. Verify owning children
      remain unchanged, target order/multiplicity and empty-versus-absent metadata
      survive, and source CEM/XML exports retain expressions without graph expansion.
      Scenarios for later design verification: inspection never evaluates a link;
      source text export cannot claim to transport runtime graph edges.
- [x] Fixture: reload a CEMB source reference and evaluate it against fresh native
      data contexts after delayed readiness. Verify independent original owners,
      stable source provenance and unchanged codec payload after consumption,
      without serializing contexts, grants or live bindings.
      Scenarios for later design verification: callers reattach runtime metadata
      explicitly after reload; lexical snapshot transport remains a separate contract.
- [x] Fixture: extend current CEMB round trips and version compatibility coverage
      for unevaluated source, resolved-empty and ordered/repeated graph edges,
      reference-to-reference edges, self-reference, cycles, source provenance,
      and invalid node handles. Reload source for later evaluation with a newly
      supplied runtime context without serializing a live environment.
- [x] Align AST codec, native graph integration, formatter, XML convention
      export/import, and typed AST/DOM/event inspection with retained reference
      identity and source semantics. Inspection must not follow chains or
      silently turn references into text or structural target subtrees.
      Keep optional runtime outcomes distinct from source-only export.
- [x] Record the supported AST graph round-trip boundary and remaining native
      output artifact gaps. Do not select cyclic native-output expansion,
      pending-outcome transport, or new transport contracts in this stage;
      those decisions remain in the roadmap.

Verification (2026-10-05): parser-backed cyclic graphs preserve order, repeated
and reference-node targets, self-reference, empty/absent targets and provenance
through CEMB versions 3 and 4. Existing fixtures retain v2 ordinary-node support
and reject invalid handles/reference tags. New ML coverage checks inert retained
node/AST-row/DOM inspection and source-only CEM/XML/event exports; two QL fixtures
check native target handles after reload and fresh-context evaluation without
payload writeback. All 43 focused tests pass: 22 ML reference/source/inspection,
nine AST codec unit tests and 12 QL construction/runtime tests. The supported
boundaries are documented in the design.
Native output-value graph and pending-outcome transport remain deferred in the
roadmap; this closure adopts no new transport or public query access syntax.

### 7. Verify and document completion

- [x] Fixture: CLI queries construct native references over CEM/XML/JSON/YAML/CSV
      inputs without authored IDs, preserve duplicate selection and inspect malformed
      authored reference source without evaluating it. Reject URL/ID string operands
      with attributed query diagnostics; verify supplied native data never triggers
      resource resolver reads during reference construction.
      Scenarios for later design verification: resource/import owns URI fetching;
      reference consumers receive nodes and decide when retained source is evaluated.
- [x] Decide CEM source ingress for CLI/shared queries: the previous preparation
      contract required `Arc<LoadedInputAstStream>` and rejected CEM
      inputs with `cem.query.input_model_unsupported`, even though CEM-document
      artifacts already used the shared native view. Adopted: admit the
      original parsed CEM owner with captured lexical bindings; preserve literal
      external imports and keep runtime evaluation explicit. User adopted shared
      ingress migration on 2026-10-05.
      Scenarios for later design verification: CEM source queries retain the original
      parser arena and source-position bindings without fabricated lifecycle ASTs,
      reparsing into an external format or automatic authored-reference evaluation.
- [x] Fixture and implementation after that decision: extend the query preparation
      owner contract and adapter admission for retained CEM inputs. Cover CEM CLI
      construction/selection, lexical capture, original owner identity, inert malformed
      reference slots and coexistence with existing external lifecycle owners.
      Scenarios for later design verification: no implicit record fallback or authored
      context IDs; unsupported source owners retain an explicit admission diagnostic.
      Completed: `QuerySourceOwner` admits original lifecycle storage or the CEM
      parser arena/capture. CEM-QL retains the same native query tree and exposes
      saved snapshots to later explicit consumers. External adapters preserve
      their ownership; CSS selector/XPath retain lifecycle admission. Four focused
      query bridge fixtures and all 16 CLI query fixtures pass, including namespace
      handoff, duplicate targets, inert malformed source and zero resolver reads.
      Verification: the full Nx query bridge target passes 148 tests; CLI query
      integration passes 16 tests; focused CEM-ML owner/capture/reference/schema,
      CSS selector and grammar parity checks pass 33 tests.
- [x] Fixture and fix: investigate generic native member lowering for an inferred
      unary-reference value in a local declaration. The module `declare let refs =
      #(input, input)` followed by `refs.targets` previously dispatched `targets`
      as a function and reported `cem.ql.unknown_function`; the legacy
      `dom:reference` result uses generic native field lookup successfully.
      Reconcile implementation-level inference/dispatch without adopting the deferred
      public reference type/target-access syntax or introducing implicit evaluation.
      Cover expression/module locals, parenthesized construction, missing fields,
      explicit unknown calls and prefixed names in native fixtures; add a CLI
      module regression that projects duplicate original CEM input targets.
      Scenarios for later design verification: generic metadata access remains
      consistent across source nodes, unary constructions and legacy helper results;
      invalid member access never masquerades as a user function invocation.
      Completed: native bare-field inference now defers its result type to the
      existing runtime view when no registered function matches. Explicit calls
      and prefixed names retain function checks. Expression/module fixtures retain
      duplicate original target identity, and missing fields remain empty. All
      34 focused query tests and 17 CLI query fixtures pass. The full Nx
      `cem_ql:test` target passes 880 tests with nine profiling fixtures ignored.
- [x] Fixture: finish CLI transformation fixture migration to the adopted native
      document view. Use `input.kind` for original document metadata instead of the
      legacy record-shaped `datadom.attributes.kind` convenience. Cover stdout, branched
      exports, local/custom-resolver globs and recursive globs with unchanged output.
      Scenarios for later design verification: template and query inputs share
      native navigation without retaining implicit record metadata wrappers.
      Completed: all six previously failing transformation tests pass through
      the original native `input` root, including both branches and recursive
      local/custom-resolver glob exports.
- [x] Fixture: align CLI binary projection header validation with the shared
      version-1/version-2 contract. Cover all three emitted projection kinds,
      legacy version admission, unsupported versions and kind mismatches.
      Scenarios for later design verification: header admission does not imply
      full payload decoding or evaluation of retained references.
- [x] Audit the three CLI binary-projection validation failures found by broad
      verification: AST, DOM and event source validation return a hard violation
      for emitted binary fixtures. Inspect attributed diagnostics and distinguish
      prior codec/schema gaps from grant handoff before choosing a repair.
      Scenarios for later design verification: an emitted source artifact must
      validate under its declared schema/version without runtime reference evaluation.
      Completed: CLI header validators retained a version-1-only check while the
      shared validators already admitted versions 1 and 2. Aligned all three CLI
      paths, retaining magic/kind/schema/content-type checks. Emitted version-2
      source cases and legacy/unknown-version header regression cases pass.
- [x] Fixture: add focused Rust integration cases through the existing CLI/query
      and schema paths for retained references and explicit supplied-context
      evaluation. Confirm URL resolution stays at the resource/import boundary
      and no projected IDs or browser behavior are needed for CEM-ML references.
- [x] Fixture: repair stale CEM-native-template invalid examples. Use `==`
      before legacy `and`, preserving the intended boolean-operator diagnostic.
      Use an import without required `src` for the missing-required-attribute
      case, since anonymous templates are permitted by the current schema.
      Scenarios for later design verification: each invalid fixture isolates
      its expected diagnostic and retains attributed expression-slot details.
      Completed (2026-10-06): all three focused CLI integration tests pass,
      including `schema_owned_cem_native_template_examples_validate_through_cli`,
      expression-slot attribution and built-in manifest validation. Nx
      `cem_ml_schema_package_cem_native_template_v1:samples2readme` and its CLI
      build prerequisite pass; generated README examples match the repaired sources.
- [x] Run affected Rust package, schema consistency, grammar parity, codec,
      and focused CLI checks through the appropriate Nx targets. Record results
      and remaining failures; browser/WASM integration follows a green native
      path only where the changed shared boundary requires it.
      Verification checkpoint (2026-10-05): full Nx bridge passes 148 tests;
      CEM-QL passes 880 tests with nine profiling fixtures ignored; CLI query
      integration passes 17 tests. Refreshed Nx CEM-ML prerequisites validate
      15 observability cases, two command-type tests and CLI schema artifacts.
      Its library run passes 2072 tests with two ignored and reproduces the known
      missing `template#cem-dropdown` failure. Because that stops integration
      execution, all 121 focused reference/declaration/policy/lexical/inspection
      and native/editor parity integration tests were run separately and pass.
      Codec unit cases pass within the library run. The broad gate remains red
      for the existing dropdown failure. The two CLI override failures were
      subsequently resolved by explicit scoped grant integration above.
      Scoped grant follow-through (2026-10-05): all 529 native CLI library tests
      and 47 core command-boundary tests pass. This includes strict CLI grant
      parsing, host/config/request boundaries, both migrated external override
      fixtures, unchanged active output on denied replacements, native template
      input navigation and version-1/version-2 projection header admission.
      The full Nx CLI run encountered an initial fixture compile error (fixed
      and covered by the passing library run) and the CEM-native-template schema
      example fixture failures subsequently repaired above. It was stopped after more than
      40 minutes of prerequisites; the full Nx gate is not claimed green.
- [x] Update public syntax/package/API documentation and initial implementation
      notes to match verified behavior. Mark only verified action items complete,
      record any unsupported cases, and retain deferred decisions and their
      verification scenarios without claiming full consumer support.
      Completed: schema contracts, CEM-QL/bridge/CLI package guidance and the
      maintained design document describe retained ownership, lexical handoff,
      explicit consumption, codec/export boundaries and checked query admission.
      The 2026-10-05 audit separates verified mechanisms and supported consumers
      from deferred property/scalar APIs; it preserves the associated scenarios.

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

## Reference adoption: dependency binding and readiness

General datatype execution, conversion, equality and enumeration authoring are a
separate workstream below. Its adopted decisions remain effective. They do not
block core reference adoption; executable native attribute `@type` specifically
remains guarded until that compiler is ready.

- [x] Fixture: bind literal datatype dependencies through an explicit declaring-scope
      lookup hook, preserving the original QName and field; verify mixed native/literal
      chains, same names in independent scopes, cycles, request/destination limits,
      crossing grants, pending versus empty/multiple targets and original provenance.
      Include imported declarations with conflicting local names, mixed-link depth
      limits and unchanged authored attributes after selection.
      Scenarios for later design verification: lookup uses the original declaring
      scope, never the importing consumer's aliases; no synthetic Reference AST is made.
- [x] Implement literal dependency lookup under those fixtures as a consumer-owned
      symbolic link over the original field, sharing the native traversal's accounting,
      active-link detection and grants. An absent lookup adapter remains pending.
      Scenarios for later design verification: a QName is never stripped into an
      implicit built-in fallback; lookup availability does not establish execution readiness.
      Verification: 13 dependency traversal fixtures and 92 adjacent source/registry/
      inventory/native-value/declaration-reference tests pass (105 total).
- [x] Fixture: retain compiled CEM-QL expression artifacts per original source
      occurrence and effective scope for standalone references and general attribute
      slots. Verify pending contexts do not compile, repeated evaluation reuses only
      compiled artifacts, context replacement invalidates them, equal source text in
      child scopes remains independent, malformed source cannot cache a valid artifact,
      and namespace-aliased XML/CDATA references evaluate with the same typed context.
      Verify a changing registered capability runs for each evaluation while the
      compiled artifact remains shared and selected target identities change.
      Scenarios for later design verification: cached compilation never caches targets,
      fabricates a runtime context ID or mutates the authored reference graph.
- [x] Integrate retained compiled source artifacts into the CEM-QL lifecycle host;
      keep compilation tied to the original occurrence and invalidate scope entries
      when caller-supplied context is replaced. Preserve explicit evaluation timing.
      Scenarios for later design verification: changed binding types/capabilities cannot
      reuse stale compilation; original handles remain available for artifact inspection.
      Verification: all 18 CEM-QL schema-reference fixtures pass, including five new
      artifact lifecycle/parity cases and changed static binding-type rejection. Full
      `cem_ql:test` Nx verification passes 862 tests (nine ignored); full
      `cem_ml_transform_cem_ql:test` passes 132 tests. No source targets are cached.
- [x] Fixture: verify expression compiler/evaluator diagnostics retain the original
      standalone/attribute occurrence and document source, including leading whitespace,
      Unicode and XML text/CDATA payload boundaries. Add precise source-position
      linkage without treating a query-relative offset as a document-relative offset.
      Scenarios for later design verification: equal expressions in separate occurrences
      report their own source; decoded XML entities preserve their authored provenance.
      Include legacy/source-only ASTs without mapping or source text, runtime-native
      failures, and already-attributed foreign diagnostics without rewriting them.
- [x] Fixture: retain expression-to-authored source segments in CEM parser and XML
      import source maps, including trim offsets, split text/CDATA, decoded entities,
      CRLF normalization and CEMB round trips. Verify mapped spans crossing payload
      boundaries stay discontiguous and missing mapping never fabricates precision.
      Scenarios for later design verification: source interpretation stays in the
      import layer; consumers use format-neutral retained mapping metadata.
- [x] Integrate original-occurrence diagnostic/source-position linkage into the retained
      expression lifecycle adapter under those fixtures; preserve compiler diagnostics
      and keep runtime binding selection separate from authored context handles.
      Scenarios for later design verification: malformed expressions remain invalid
      without losing source spans or emitting diagnostics against a different occurrence.
      Verification: all 23 CEM-QL schema-reference fixtures pass, including five new
      diagnostic attribution cases and the static type-error assertion. Ninety focused
      CEM-ML tokenizer/parser/codec/native-slot/source-mapping tests pass, including
      three new retained mapping and binary fixtures. Full `cem_ql:test` and
      `cem_ml_transform_cem_ql:test` Nx targets pass; no deferred syntax is enabled.
- [x] Fixture: strengthen tree-sitter/native parity to compare expression bodies and
      native attribute slots separately from literal lookalikes. Cover quoted/doubled/
      escaped delimiters, nested comments and records, empty spans, malformed closure,
      following siblings, exact source spans and incremental edits inside query strings.
      Scenarios for later design verification: editor parsing never ends a query at a
      quoted/commented brace; quoted and bare `#` text never gains native expression kind.
- [x] Fixture: tokenize TextMate reference and attribute spans with quoted/doubled/
      escaped braces, nested comments and records, and following siblings; keep
      quoted reference lookalikes literal and preserve query scope through line breaks.
      Scenarios for later design verification: editor highlighting keeps host closing
      braces distinct from braces belonging to opaque query source.
- [x] Align tree-sitter's opaque query-span scanner, generated parser/build integration,
      lexical EBNF and TextMate reference highlighting under those fixtures. Preserve
      the existing CEM/XML attribute boundary and deferred child override spelling.
      Scenarios for later design verification: all canonical fixture projections remain
      compatible; the generated parser binds the scanner, and native build/cache
      inputs include it.
      Verification: five native/editor parity tests pass, including all canonical
      fixtures; 25 native tokenizer tests and 14 TextMate expression/attribute cases
      pass. Parser regeneration succeeds through `cem_ml:tree-sitter:generate`.
- [x] Audit and finish the remaining retained reference expression-slot integration
      and CEM/XML parity from sections 1–2 above. Identify existing coverage first;
      add focused native fixture actions for actual gaps before implementation.
      Scenarios for later design verification: standalone and attribute references
      preserve expression artifacts and source-position bindings; loading never
      evaluates targets or requires an authored runtime-context ID.
- [ ] Integrate dependency selection outcomes with schema consumer readiness when
      the separate datatype compiler supplies its executable contracts. Add readiness
      fixtures before integration; preserve available sources for inspection and the
      last complete active package while replacement dependencies remain incomplete.
      Scenarios for later design verification: complete selection cannot activate an
      unavailable execution contract; pending lookup is distinct from a complete empty result.

## General datatype compilation design

Separate workstream: preserve all adopted contracts here, but keep execution,
equality and enumeration decisions outside the core reference implementation.
The enumeration authoring recommendation remains undecided and deferred to this
workstream; no new constant syntax is adopted by the reference work.

- [x] Draft the general datatype compiler representation, dependency lifecycle,
      kind inventory, migration boundary and implementation sequence in
      [the temporary proposal](cem-datatype-compilation-proposal.md).
      Scenarios for later design verification: one compiled descriptor serves
      literal and native selection without cloning authored declarations.
- [x] Adopt explicitly registered schema-owned executable rule implementations;
      preserve shipped rule prose as descriptions of known built-in contracts.
      Do not introduce an executable grammar language for `@rule`.
      Scenarios for later design verification: unknown implementations cannot
      activate; matching a prose description or local type name grants no primitive.
- [x] Fixture: retain and inventory the 18 generic CEM-ML datatype declarations,
      including lexical/scalar/list/grammar/reference kinds, list item bases and
      prose versus named rules, through original AST owners.
      Scenarios for later design verification: the compiler must not treat a list
      item base as scalar inheritance or a symbolic reference as an AST constructor.
- [x] Fixture: verify that current scalar conversion produces values accepted by
      the same schema validator, and record which shipped datatype families still
      require native conversion implementations rather than a string fallback.
      Scenarios for later design verification: canonical primitive outputs preserve
      whitespace/normalization semantics; unsupported native families stay explicit.
      Verification: all three inventory/parity fixtures, nine native value contract
      tests and 65 declaration-reference tests pass (77 total). No datatype rule
      binding or general compilation is enabled by this design inventory.
- [x] Adopt explicit native `@rule={#behavior}` selecting one named registered
      datatype-compatible behavior; preserve literal rule descriptions and do
      not introduce a separate `@behavior` binding field.
      Scenarios for later design verification: named datatype-compatible behaviors
      retain their original owner/scope; prose never becomes an implicit reference.
- [x] Adopt separate validation/conversion roles: `@rule` reports acceptance or
      diagnostics; explicitly requested conversion uses a separate registered
      capability, and rule results cannot replace the typed value.
      Scenarios for later design verification: conversion happens once; inherited
      restrictions cannot rewrite the final value or mutate an authored source.
- [x] Adopt datatype registry collision errors for distinct same-name declarations
      within one lexical schema. Reusing one original declaration from multiple
      attribute sites remains valid; ordinary duplicates do not authorize overrides.
      Scenarios for later design verification: equal local names in separate scopes
      remain independent; source order cannot silently weaken a type restriction.
- [x] Adopt intersecting scalar restrictions: base, derived and attribute-local
      constraints all hold; conversion occurs once and restrictions cannot widen
      the base contract or replace its result.
      Scenarios for later design verification: disjoint restrictions admit no values;
      statically provable contradictions report their original source locations.
- [ ] Design explicit datatype declaration overrides as separate future work:
      specify replaced declaration identity, authorization, dependency rebinding
      and coordinated activation before admitting syntax or implementation.
      Scenarios for later design verification: an application can intentionally
      replace a contract without import order silently weakening restrictions;
      existing native selections retain identity unless explicit rebinding is chosen.
- [x] Adopt canonical list conversion as ordered typed items with retained authored
      lexical source. Lexical output is explicit serialization; preserve shipped
      whitespace list validation and keep conversion disabled until its adapter exists.
      Scenarios for later design verification: item order and duplicates survive;
      list item bases do not become scalar inheritance; validation does not convert.
- [x] Adopt rejection of `values` on list datatype declarations; use item datatype
      restrictions and registered list rules. Preserve attribute-local `values`
      compatibility separately; no sequence-enumeration shorthand is admitted.
      Scenarios for later design verification: item restrictions and whole-sequence
      rules remain distinct; sequence enumeration is never inferred from a string.
- [ ] Specify list conversion adapters and explicit canonical serialization, then
      add fixture actions for ordered items, duplicates and retained item provenance
      before implementing conversion or changing scalar API result types.
      Scenarios for later design verification: typed results preserve item identity;
      serialization does not overwrite authored lexical input or clone native nodes.
- [x] Adopt datatype rule results with explicit acceptance plus attributed diagnostics.
      Require all effective restrictions to accept; keep pending/unavailable and
      failed execution separate from completed validation results.
      Scenarios for later design verification: warning severity does not accidentally
      accept an invalid value; malformed results never become successful validation.
- [ ] Specify a registered datatype validation
      adapter that checks retained behavior/owner identity, required input bindings
      and kind-specific representation, distinct from diagnostic-only behaviors.
      Add fixture actions before implementing signature checking or invocation.
      Scenarios for later design verification: original candidate/type handles remain
      native; unavailable execution differs from invalid data; conversion stays separate.
- [x] Adopt general list emptiness: permit zero items unless an effective registered
      contract/cardinality/rule forbids it. Preserve shipped name-list nonempty
      behavior; an absent attribute remains distinct from an empty supplied list.
      Scenarios for later design verification: an empty result collection has a clear
      contract; representation alone does not erase the shipped nonempty restriction.
- [ ] Specify custom list cardinality admission and registered tokenizer contracts,
      then add fixture actions for absent versus empty input, inherited nonempty
      restrictions and invalid lexical tokens before implementing adapters.
      Scenarios for later design verification: an item base does not require an item;
      empty conversion manufactures neither a default item nor a null value.
- [x] Adopt a dedicated `node` datatype kind for reusable custom retained-node
      contracts. Preserve symbolic-reference kinds and existing built-in node
      contracts separately; reject lexical facets and retain original target handles.
      Scenarios for later design verification: target handles retain original owners;
      lexical facets never apply to nodes; reference resolution remains consumer-owned.
- [x] Adopt node datatype rule invocation once for the complete ordered target
      sequence, including a complete empty selection. Keep the original candidate
      distinct from targets and incomplete resolution distinct from an empty result.
      Scenarios for later design verification: reusable contracts can check target
      relationships; empty selections and cardinality retain explicit outcomes.
- [ ] Specify node-kind metamodel admission, base compatibility and native sequence
      signatures; add fixture actions for retained targets, lexical-facet rejection
      and unchanged descendant reference nodes before implementing the node adapter.
      Scenarios for later design verification: native validation never stringifies
      targets or expands descendant references implicitly; scope bounds still apply.
- [x] Adopt per-capability candidate requirements for standalone datatype consumers.
      Capabilities without a required candidate may accept source-less values;
      capabilities requiring a native candidate remain unavailable when it is absent.
      Scenarios for later design verification: a source-less scalar consumer never
      fabricates an AST or candidate record; required native inputs remain unavailable.
- [ ] Specify concrete validation result bindings and an explicit compatibility
      adapter for diagnostic-only behaviors, with a declared acceptance mapping.
      Add fixture actions for acceptance/diagnostic independence, cumulative rejection,
      missing-result failures and retained source attribution before implementation.
      Scenarios for later design verification: an accepted rule cannot erase another
      restriction's rejection; lifecycle incompleteness cannot masquerade as success.
- [ ] Specify the registered implementation identity, typed input/result signature,
      package ownership checks and compilation/validation/conversion adapters.
      Scenarios for later design verification: registration grants no scope access;
      an unrelated vendor type cannot borrow a built-in implementation by name.
- [ ] Establish full validation/conversion parity for shipped datatype families;
      add fixture actions for lexical, URI/semver/media-type/path, list, grammar
      and symbolic-reference implementations before adding those implementations.
      Scenarios for later design verification: unsupported conversion stays explicit;
      a primitive predicate alone does not prove a complete conversion contract.
- [ ] Complete datatype namespace/export API and metamodel admission, registered
      tokenization policies, and grammar/reference kind contracts.
      Scenarios for later design verification: attribute restrictions cannot widen
      a base; native node contracts retain typed input without scalar extraction.
- [x] Fixture: verify retained datatype registry collection without authored scope IDs:
      repeated original declaration reuse, distinct same-name collision provenance,
      independent lexical scopes, same node addresses in different owners, original
      owner lifetime and malformed declaration rejection. No executable kind is enabled.
      Scenarios for later design verification: native identity remains independent of
      local-name lookup; a collision preserves both sources and never replaces a binding.
- [x] Implement the retained datatype registry collection foundation under these
      fixtures, with caller-supplied lexical scope handles and original declaration
      lookup. Keep dependency compilation and native type execution guarded.
      Scenarios for later design verification: registry membership grants no scope
      access or built-in implementation; collected declarations do not claim readiness.
      Verification: six registry fixtures, three datatype inventory fixtures, nine
      native value contract tests and 65 declaration-reference tests pass (83 total).
- [x] Adopt omitted datatype `kind` inheritance from a resolved base for derived
      types, with explicit `kind=list` for item-base semantics. Pending bases
      remain pending; whole-list inheritance needs its separate dependency contract.
      Scenarios for later design verification: forward/pending bases never default
      to strings; a list item base is never mistaken for scalar inheritance.
- [x] Fixture: retain datatype source descriptors with original lexical scope and
      declaration handles, native base/rule attribute nodes, unknown authored fields,
      omitted versus empty kind and last-authored attribute precedence. Verify owners
      survive registry/source-owner release without cloning declarations.
      Scenarios for later design verification: kind inference sees authored fields
      accurately; unsupported fields remain visible to later compilation diagnostics.
- [x] Implement retained datatype source descriptors under these fixtures; do not
      interpret kind/base/rule or claim executable readiness during collection.
      Scenarios for later design verification: native dependency attributes remain
      original nodes until the bounded lifecycle compiler consumes them.
      Verification: four source descriptor fixtures plus the 83 adjacent registry,
      inventory, value and declaration-reference tests pass (87 total).
- [x] Preserve existing list `base` item semantics and defer whole-list inheritance
      until separate explicit syntax is designed. Do not infer a list inheritance
      edge or reinterpret an item base from a missing kind.
      Scenarios for later design verification: a list of lists is distinct from a
      derived list; source compatibility never silently changes a dependency's role.
- [ ] Design whole-list inheritance syntax, dependency roles and compatibility as
      separate future work before admitting derived whole-list declarations.
      Scenarios for later design verification: list-of-list items are not inheritance;
      existing shipped item-base declarations preserve their effective item contracts.
- [x] Fixture: classify retained datatype source plans for explicit and omitted kinds,
      list item versus inherited base roles, descriptive versus native rules, malformed
      native fields and rejected list/node lexical values with original attribution.
      Scenarios for later design verification: classification never executes prose,
      resolves a native reference or labels an incomplete datatype ready.
- [x] Implement source classification plans under these fixtures, retaining dependency
      attributes and references for later shared bounded traversal. Keep native datatype
      consumption guarded and do not create a new per-field traversal budget.
      Scenarios for later design verification: explicit kind alone does not erase
      unresolved base/rule dependencies; unsupported source fields remain visible.
      Verification: five plan fixtures plus the 87 adjacent source, registry,
      inventory, native value and declaration-reference tests pass (92 total).
- [x] Adopt inherited base compatibility declared by registered contracts, including
      explicit cross-kind relationships. Matching representation alone is insufficient;
      preserve conversion once and keep list item edges distinct from inheritance.
      Scenarios for later design verification: shipped grammar/string and symbolic
      reference/qualified-name bases remain valid through explicit registered contracts;
      a matching representation alone never grants a new primitive or scope access.
- [ ] Specify the registered base compatibility contract and its source-attributed
      validation; add fixture actions for inherited-kind reuse, permitted shipped
      cross-kind bases, incompatible bases and unavailable registration before wiring
      the bounded dependency compiler.
      Scenarios for later design verification: a derived restriction retains the
      base implementation identity; incompatible contracts never activate by name.
- [x] Adopt inherited converter selection: inherit the base converter unless a
      compatible derived converter is explicitly registered. Select and invoke one
      converter, then validate all effective restrictions without a hidden pipeline.
      Scenarios for later design verification: restrictions cannot replace a value;
      unavailable selected converters never trigger silent fallback to a base converter.
- [ ] Implement effective converter selection with original implementation identity
      and registered output compatibility; add fixture actions for inherited reuse,
      explicit replacement, unavailable selected capability and validation-only types
      before wiring executable conversion. Never fall back after selected failure.
      Scenarios for later design verification: one converter produces the canonical
      value; inherited/local restrictions all inspect it without rewriting or chaining.
- [x] Implement shared datatype dependency traversal over original owning containers
      and base/rule fields, preserving per-field singleton/target diagnostics. Literal
      QName lookup now uses an explicit original-scope host hook under the same charged
      and authorized walk; see the reference binding fixtures above. Executable compiler
      adapters and registered capability admission remain separate pending work.
      Scenarios for later design verification: containment never bypasses a grant;
      native and literal dependencies share accounting without synthetic Reference ASTs.
- [x] Adopt registered datatype equality for scalar declaration `values`. Preserve
      existing attribute-local vocabulary behavior until its migration is explicit;
      equality neither rewrites the input nor implicitly invokes conversion.
      Scenarios for later design verification: integer 003 versus 3 has a defined
      outcome; equality never rewrites the candidate or implicitly invokes conversion.
- [x] Fixture: traverse native datatype base/rule dependencies in one retained forest:
      preserve field grouping/target handles, detect cycles, share request/destination
      limits, enforce grants, distinguish pending from empty/multiple/wrong targets,
      and retain deferred literal lookup or missing lexical source context.
      Scenarios for later design verification: hard target errors survive unresolved
      disposition; no dependency starts a new budget or becomes executable by collection.
- [x] Implement bounded native datatype dependency traversal using the existing
      consumer-container resolver. Keep literal QName lookup and registered signature/
      equality/conversion execution deferred until their adapters are implemented.
      Scenarios for later design verification: original fields supply error provenance;
      a partial native forest never appears to be a ready general datatype descriptor.
      Verification: nine native dependency fixtures and the 92 adjacent datatype/
      native value/declaration-reference tests pass (101 total).
- [ ] Implement registered scalar equality and enumeration constant interpretation,
      preserving each inherited restriction's original contract and binding. Add
      fixture actions before adapters for numeric/string equality, unavailable equality
      and inherited restriction provenance; do not rewrite input during comparison.
      Scenarios for later design verification: a derived equality cannot widen a base
      vocabulary; unknown constant interpretation prevents readiness rather than fallback.
- [ ] Decide enumeration constant authoring: preserve whitespace-token `values` and
      defer richer retained constants (recommended), or design retained constant
      declarations now with source/reference/metamodel compatibility.
      Scenarios for later design verification: one string containing spaces is never
      silently split into a different claimed enum constant; no implicit JSON parsing.
- [ ] Implement retained datatype descriptors and lexical name binding, then a
      bounded dependency compiler and kind consumers under the adopted contracts.
      Add focused native fixture items before implementing each slice.
      Scenarios for later design verification: cycles and incomplete dependencies
      prevent activation while original declarations stay available for inspection.
- [ ] Integrate the descriptor into literal and native attribute type consumers,
      preserve compatibility through explicit parity checks, and update the
      metamodel's native datatype admission contract before enabling consumption.
      Scenarios for later design verification: referenced type owners and their
      restrictions survive package replacement and independent runtime contexts.
- [ ] Once adopted, merge the proposal into maintained designs and remove the
      temporary proposal, preserving outstanding action items and scenarios.
      Scenarios for later design verification: normative docs distinguish supported
      consumers from implementation phases and genuinely deferred decisions.

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
