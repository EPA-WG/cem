# Todo

This file is the authoritative checklist for active execution work.
Product/module sequencing lives in [roadmap.md](../roadmap.md), and future
wishlist work lives in [wishlist.md](wishlist.md).

The [2026-10-07 snapshot](archive/todo-snapshot-2026-10-07.md) preserves the completed checklists,
adopted decisions and verification evidence removed in this cleanup. Its open
items are historical context; only this file governs active work. Earlier
history remains in the [2026-09-27 snapshot](archive/todo-snapshot-2026-09-27.md)
and [follow-up log](archive/todo-completed-2026-09-27.md).

## AST node reference implementation

The accepted [node references design](cem-ql-cem-ml-node-references-design.md)
governs this work. Parsing retains expressions and never executes reference
selection on load. Consumers supply runtime contexts, scope grants and bounded
evaluation at their lifecycle stage; context roots need no IDs.

Core reference syntax, retained owners, lexical capture, bounded resolution,
supported schema consumers, codecs, query/reload/transport contracts and native
host/syntax adapters are implemented. See the [consumer verification matrix](reference-consumer-verification.md)
and [archived implementation history](archive/todo-snapshot-2026-10-07.md#ast-node-reference-implementation).
General datatype/function composition and conditional typed prelude payloads
remain open below. Further cem-element consumer work is tracked in its
[consumer list](#deferred-cem-element-reference-consumption).

### 5. Integrate schema validation and construct reuse

Completed prerequisites and verification: [archived checklist](archive/todo-snapshot-2026-10-07.md#5-integrate-schema-validation-and-construct-reuse).

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

- [ ] Specify typed prelude reference slots only if that source capability is
      requested. Current directive payloads are literal text; native reference and
      expression slots already belong to schema-element/host attribute forms. Keep
      this separate from the adopted enclosed host-bound child syntax.
      Scenarios for later design verification: a literal query constructor is not an
      authored AST reference slot; consumers retain original source occurrences and
      do not manufacture attributes or reparse a second CEM tree.

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

Completed prerequisites and verification: [archived checklist](archive/todo-snapshot-2026-10-07.md#reference-adoption-dependency-binding-and-readiness).

- [x] Integrate explicit datatype compilation snapshots with package readiness
      and source-owner checks. Fixtures preserve available sources for inspection,
      retry the same retained candidate and keep schema/converters/artifacts active
      together while a selected capability is incomplete. Automatic attribute type
      consumption remains in the separate compiler integration action below.
      Scenarios for later design verification: complete selection cannot activate an
      unavailable execution contract; pending lookup is distinct from a complete empty result.

## General datatype compilation design

Separate workstream: preserve all adopted contracts here, but keep execution,
equality and enumeration decisions outside the core reference implementation.
Whitespace-token `@values` authoring is adopted; richer retained constants remain
deferred in this workstream. No new constant syntax is adopted.

The [adopted datatype decisions](archive/todo-snapshot-2026-10-07.md#general-datatype-compilation-design)
remain requirements for the open work below. The [temporary proposal](cem-datatype-compilation-proposal.md)
gathers those contracts; completed registry/source/dependency foundations are
archived, while executable datatype consumption remains guarded.

- [ ] Design explicit datatype declaration overrides as separate future work:
      specify replaced declaration identity, authorization, dependency rebinding
      and coordinated activation before admitting syntax or implementation.
      Scenarios for later design verification: an application can intentionally
      replace a contract without import order silently weakening restrictions;
      existing native selections retain identity unless explicit rebinding is chosen.

- [ ] Deferred: define compatible replacement of an inherited lexical preparer
      before supporting derived capability replacement. Specify original base
      admission checks, typed representation compatibility and source identity;
      the current compiler requires inheritance and rejects replacement.
      Scenarios for later design verification: a child preparer cannot admit
      conversion-only forms rejected by its base or discard original provenance.

- [ ] Specify list conversion adapters and explicit canonical serialization, then
      add fixture actions for ordered items, duplicates and retained item provenance
      before implementing conversion or changing scalar API result types.
      Scenarios for later design verification: typed results preserve item identity;
      serialization does not overwrite authored lexical input or clone native nodes.

- [x] Adopt fixed datatype validation input roles `value`, `datatype` and
      `candidate` (2026-10-08), with candidate requirements declared per
      registered capability. Do not map arbitrary parameter names to these roles.
      Validation returns acceptance plus diagnostics; conversion stays separate.
      Scenarios for later design verification: typed value and original candidate
      remain distinct; a missing required candidate is unavailable, not invalid.

- [x] Adopt optional `candidate` as a zero-or-one native node sequence and a
      dedicated typed validation result with required boolean `accepted` and
      sequence `diagnostics` (2026-10-08). Empty diagnostics are valid; missing or
      mistyped fields fail execution. Missing a required candidate is unavailable.
      Scenarios for later design verification: absent candidates need no fabricated
      node; validity remains independent of diagnostic severity.

- [x] Adopt ordinary CEM-QL record construction under the dedicated validation
      result contract, with static checks where possible and mandatory consumer
      boundary checks (2026-10-08). Constructors are optional; execution authority
      remains with registered capabilities and retained behavior/owner identity.
      Make candidate cardinality visible to compilation through registered signatures.
      Scenarios for later design verification: a matching legacy record grants no
      execution authority; an empty candidate value does not change function arity.

- [x] Add native signature/invocation fixtures for the explicit registered profile:
      reject renamed/inferred roles,
      retain original datatype/candidate owners, and cover source-less rules with and without a
      required candidate. Check optional candidates with zero, one and multiple nodes.
      Check source-declaration compatibility with registered zero-or-one/exactly-one
      candidate cardinality, independently of optional-argument syntax.
      Keep executable native attribute `@type` guarded.
      Scenarios for later design verification: role naming supplies no scope grants;
      scalar, list and node values preserve their registered representations.

- [x] Implement an explicit registered datatype validation
      adapter that checks retained behavior/owner identity, required input bindings
      and kind-specific representation, distinct from diagnostic-only behaviors.
      Native and inline query implementations use the same checked result adapter.
      Scenarios for later design verification: original candidate/type handles remain
      native; unavailable execution differs from invalid data; conversion stays separate.

- [x] Implement effective `min-items`/`max-items` sequence bounds and explicit
      registered tokenizer contracts (2026-10-08). Preserve absent/empty input,
      inherited and registered nonempty restrictions, original lexical spans and
      separator admission. Tokenization stays separate from item conversion.
      Scenarios for later design verification: an item base does not require an item;
      empty conversion manufactures neither a default item nor a null value.

- [x] Add native fixtures for list bounds and registered tokenization: absent versus
      empty input, nonempty registered bounds, ordered duplicate tokens, Unicode
      source spans, malformed custom spans, cancellation and bounded work. Add
      descriptor fixtures for node/scalar base compatibility, inherited restrictions,
      exact validation registration, incomplete dependencies and original owners.
      Add package fixtures for preserving the active schema, converters and artifacts
      during incomplete replacement, attributing invalid declarations to their
      original sources, and rejecting stale owner snapshots.
      Scenarios for later design verification: tokenization does not convert items;
      native target subtrees retain authored references; ready selection alone
      cannot activate an unavailable implementation.

- [x] Implement effective node-kind metamodel admission and base compatibility,
      connecting native sequence validation to explicit datatype descriptors.
      Verify lexical-facet rejection and unchanged descendant references through
      that integration. Automatic native attribute type activation stays guarded.
      Scenarios for later design verification: native validation never stringifies
      targets or expands descendant references implicitly; scope bounds still apply.

Verification for these three slices (2026-10-08): 131 focused native tests cover
source/registry/traversal contracts, sequence tokenization, descriptor/result
validation, retained behaviors and package/region lifecycles. The schema package
Nx verification includes its CLI build prerequisite. Automatic native attribute
`@type` activation remains guarded; conversion and equality adapters are completed below.

- [x] Adopt the shared diagnostic value contract (2026-10-08): accept existing
      native diagnostics and checked CEM-QL records with required singleton `code`,
      `severity` and `message`, plus optional zero-or-one original native `source`.
      Preserve native metadata; derive new attribution from the source node or
      available invocation input attribution. Malformed diagnostics fail invocation.
      Scenarios for later design verification: no diagnostic requires a fabricated
      candidate; existing source maps survive construction and consumption.

- [x] Adopt shared value-contract declarations for reusable result/diagnostic
      records and sequence cardinality (2026-10-08), compiled without invoking
      user datatype rules. Keep field presence distinct from sequence cardinality.
      Scenarios for later design verification: a required diagnostics field may be
      empty; multiple behaviors reuse one diagnostic shape without copying it.

- [x] Adopt rejection of undeclared value-contract record fields by default, with
      an explicit schema option allowing extensions (2026-10-08). Keep declared
      field constraints effective and preserve existing native diagnostic metadata.
      Scenarios for later design verification: `soruce` fails a closed diagnostic
      contract; an open contract still rejects an invalid declared `source` value.

- [x] Implement the shared value-contract declaration surface and compiler:
      `value-contracts`/`value-contract`/`value-field`, primitive representation kinds,
      QName-bound nested record contracts, independent presence/cardinality and
      explicit `allow-extra`. Retain original declarations and owner-checked captured
      names; reject unknown/cyclic types and unsupported source forms under bounds.
      Scenarios for later design verification: unknown contracts do not become
      untyped fallbacks; compiling shape metadata never invokes user datatype rules.

- [x] Implement and verify the first shared value-contract compiler and CEM-QL
      result adapter. Fifteen native fixtures cover schema-driven fields and
      vocabularies, QName binding, declaration identity, invalid/cyclic dependencies,
      bounds, closed/extended records, missing/empty fields, native node attribution,
      malformed results, native/record parity, one-read native snapshots and
      preservation of execution-side reports. The shipped schema validates the new
      vocabulary and declares its result/diagnostic shapes.
      Verification: all 15 new fixtures, 46 CEM-ML regression tests, the native
      diagnostic fixture and 22 recovery tests pass (84 total); the schema package's
      Nx `verify` target passes, including its CLI build prerequisite.
      Scenarios for later design verification: consumers cannot acquire execution
      authority by returning a matching record; retained sources outlive evaluation.

- [x] Verify primitive record fields reject native node/record scalar accessors
      while admitting typed native atomic values; require the declared representation
      without implicit conversion.

- [x] Declare diagnostic fields and severity vocabulary in the schema-owned
      metamodel and implement the shared native/record consumer. It returns existing
      shared report diagnostics with original attribution, preserves native metadata
      and diagnoses invalid source values without fallback or ID lookup.
      Scenarios for later design verification: both representations retain source
      attribution through reporting; severity never substitutes for `accepted`,
      while host reporting and abort policies still apply.

- [x] Expose severity on the existing native diagnostic query view. The native
      query fixture covers all four severities, field enumeration and retained
      diagnostic metadata/source maps inside a constructed result record. The
      fixture and 22 existing query/template/XSLT recovery tests pass. This is
      query access only; datatype rule execution stays guarded.
      Scenarios for later design verification: reading severity does not emit a
      diagnostic, infer acceptance or replace the retained diagnostic with a record.

- [x] Implement validation result metamodel admission and structural result checking
      without invoking the unfinished datatype compiler. Enforce exactly one result,
      required fields, singleton boolean acceptance, typed diagnostic sequences and
      retained attribution. Keep execution failures separate from rejected values.
      Scenarios for later design verification: lifecycle incompleteness cannot
      masquerade as success; diagnostic severity does not determine acceptance.

- [x] Connect CEM-QL-constructed records and native diagnostic values to the same
      explicit result consumer, with parity and snapshot fixtures. Retain the
      original result and owners; matching fields grant no behavior registration.
      Scenarios for later design verification: constructors are optional; result
      construction leaves original candidate and datatype handles intact.

- [x] Implement registered datatype input signature checking and authorized behavior
      dispatch before invocation. Add fixtures for missing required candidates,
      registered zero-or-one/exactly-one cardinality, kind-specific value inputs,
      retained owners and cumulative rejection across effective restrictions.
      Scenarios for later design verification: a matching legacy result never grants
      execution authority; later acceptance cannot erase an earlier rejection.

- [x] Add 15 native fixtures for the registered validation adapter: exact behavior/owner
      registration, fixed input roles and source cardinalities, native datatype and
      candidate retention, scalar/list/node representations, missing candidates,
      cumulative acceptance, bounded/control-aware execution, query/native parity,
      pending/failed outcomes and explicit diagnostic-only acceptance mappings.
      Verification: 15 new adapter fixtures, 48 query/result/native-function recovery
      tests, 46 CEM-ML regressions and 27 retained/region/namespace behavior tests
      pass (136 tests total). The schema package's Nx `verify` target passes,
      including its CLI build prerequisite.
      Scenarios for later design verification: equal names in another owner confer
      no authority; an empty candidate occupies its argument; later acceptance
      cannot hide incomplete execution or earlier rejection.

- [x] Implement explicit diagnostic-sequence compatibility with a registered
      acceptance mapping (codes, severities, or empty sequence). Native/query fixtures
      cover malformed/unavailable outputs, retained attribution and independence
      from previously emitted reports. Legacy object formats require a host bridge.
      Scenarios for later design verification: diagnostic-only results cannot imply
      acceptance until the selected registered mapping is applied.

- [x] Integrate retained validation registrations with explicit datatype descriptors
      and package readiness/ownership checks (2026-10-08). Adopt separate conversion,
      scalar equality and token-constant interpretation signatures with typed
      outcomes; their executable adapters remain the actions below.
      Scenarios for later design verification: registration grants no scope access;
      an unrelated vendor type cannot borrow a built-in implementation by name.

Next implementation order: shipped validation/conversion parity, then remaining
metamodel support and attribute type consumer integration. Implementations and
fixtures follow the adopted capability signatures; automatic attribute type
activation depends on those tasks.

- [ ] Establish full validation/conversion parity for shipped datatype families;
      add fixture actions for lexical, URI/semver/media-type/path, list, grammar
      and symbolic-reference implementations before adding those implementations.
      Scenarios for later design verification: unsupported conversion stays explicit;
      a primitive predicate alone does not prove a complete conversion contract.

- [x] Add shipped-contract parity fixtures for all 18 original declarations,
      lexical and sequence validation, primitive normalization, wide integers,
      unsupported grammar/conversion and explicit native validator registration.
      Scenarios for later design verification: native validation never converts;
      symbolic values never resolve nodes; unavailable grammar cannot report success.

- [x] Expose shared shipped lexical contracts and explicitly registered native
      validation adapters, reusing existing predicates and scalar conversion rules.
      Scenarios for later design verification: URI validation preserves its current
      absolute-URI rule; the broader declaration prose grants no new semantics.

Verification (2026-10-08): 127 tests pass across shipped inventory/contracts,
datatype validation, shared value contracts and schema-package lifecycle. The three
native shipped-validator tests pass again after final list-policy review; final
schema-package Nx verification and its CLI build prerequisite pass.

- [x] Add native scalar conversion fixtures for normalization, wide integer
      identity, checked source/query results, typed validation, constant preparation,
      exact integer equality, retained source maps, cancellation and cumulative bounds.
      Scenarios for later design verification: metadata lookalikes cannot forge an
      integer; a decimal query projection cannot change the retained datatype.

- [x] Bridge shipped string/boolean/integer/number conversion into exact native
      datatype registrations. Preserve wide integer identity across the existing
      retained typed scalar view, input/output signature checks and result adapters;
      never narrow to `i64` or silently change an integer contract to decimal.
      Implemented with separate constant-interpreter registrations and exact retained
      integer query comparison; checked results retain the shared immutable scalar view.
      Scenarios for later design verification: an integer outside `i64` remains an
      integer; normalization runs once; validation-only use never calls conversion.

Verification (2026-10-08): 188 tests pass across datatype validation/results,
attribute conversion, template rendering and schema-package lifecycle, including
six native scalar fixture groups. Schema-package Nx verification and its CLI build
prerequisite pass.

- [x] Add shipped lexical/list conversion fixtures for exact descriptor registration,
      original item validation, nonempty bounds, ordered duplicate UTF-8 token spans,
      retained maps, cancellation/limits and unavailable-tokenizer package retries.
      Scenarios for later design verification: lexical normalization only trims;
      symbolic strings gain no lookup authority; decoded spans are not source offsets.

- [x] Complete native shipped list/item/tokenizer registrations and explicitly
      designed lexical/URI/semver/media-type/path/symbolic conversion capabilities.
      Existing unsupported conversion must remain explicit until each output and
      normalization contract is adopted; no default string conversion.
      Implemented with descriptor-selected tokenization, original item validation,
      trim-only lexical conversion and separate scalar constant interpretation.
      End-to-end descriptor and package readiness fixtures pass.
      Scenarios for later design verification: name lists remain nonempty, tokens
      preserve order and source spans, and symbolic values confer no lookup authority.

Verification (2026-10-08): 200 tests pass across shipped inventory/contracts,
native datatype/result validation, attribute conversion, template rendering and
package lifecycle. Schema-package Nx verification and its CLI build pass. The five
lexical/list fixture groups also verify descriptor tokenization and item bounds.

- [x] Add content-model grammar fixtures for shipped source inventory, malformed
      alternatives/groups/postfixes, Unicode names, controlled byte/token/depth limits,
      explicit registration/readiness, conversion and token constant preparation;
      pin absolute-URI compatibility.
      Scenarios for later design verification: unavailable parsing cannot accept text;
      grouped CSS alternatives survive; validation does not gain document matching authority.

- [x] Specify and register the bounded shipped content-model grammar consumer,
      including explicit conversion and token constant preparation. Correct generic
      URI prose to the existing absolute-URI contract without broadening admission.
      Malformed grammar, absent registration, overridden bounds and compatibility
      fixtures cover the adopted consumer. Full document matching and relative-URI
      extensions remain deferred actions in roadmap.md.
      Scenarios for later design verification: absent grammar cannot validate arbitrary
      text; URI policy changes never arise incidentally from conversion migration.

Verification (2026-10-08): the 207-test regression suite and six focused grammar
checks pass, including the added mid-scan cancellation/decoded-offset case.
Schema-package Nx verification and its CLI build prerequisite pass. Parser syntax
is covered against every shipped `@children` expression; automatic attribute type
activation and full document matching remain outside this completed consumer slice.

- [x] Add datatype name-catalog fixtures for forward local references, original
      per-declaration aliases, explicit exports, duplicate/ambiguous names, pending
      bindings, foreign metamodel names, bounded collection and denied scope crossings.
      Scenarios for later design verification: importing a public name never grants
      access; an exporter keeps its own dependency aliases; failed catalog replacement
      leaves the previous snapshot usable and never falls back to legacy bindings.

- [x] Implement an immutable namespace/export catalog over explicitly collected
      original datatype sources and lifecycle-supplied lexical environments. Wire
      literal dependency lookup through it without changing native selection or grants.
      Scenarios for later design verification: equal names in independent scopes
      stay independent; pending aliases and unexported targets cannot activate a type.

- [x] Add source-discovery fixtures for literal QName namespace capture/reload,
      forward declarations, source-position rebinding, explicit `uses` aliases,
      conflicting aliases, bounded/malformed collections and package readiness retries.
      Scenarios for later design verification: unavailable metadata never borrows a
      later binding; incomplete discovery preserves the last active package.

- [x] Discover literal schema datatype collections and original lexical bindings,
      install their catalog through an opt-in package compilation path and preserve
      pending native collection/namespace slots without guessing their results.
      Scenarios for later design verification: source discovery grants no exports,
      runtime contexts or implementation authority; declarations keep original owners.

- [x] Add native datatype-discovery fixtures for original owner/alias retention,
      explicit exports and crossing grants, selected `uses`, empty versus pending
      results, wrong targets, cycles, retained query diagnostics and shared bounds.
      Scenarios for later design verification: repeated collection slots cannot
      replenish work; incomplete discovery never replaces the active package.

- [x] Add bounded native selection for schema `types`/`uses` collection slots to
      source discovery, preserving each selected declaration's original schema and
      captured aliases. Collection slots share request/destination traversal bounds;
      package retries retain the candidate and last complete active package.
      Scenarios for later design verification: an unresolved collection never
      disappears from readiness; vendor selection needs a crossing grant, and QName
      exposure still needs an explicit export. Descendant references remain authored.

Verification (2026-10-08): 118 native datatype, declaration-reference and package
lifecycle tests pass; Nx schema-package verification passes with its CLI build.

- [x] Add XML literal-namespace fixtures for whole-start-tag scope, child rebinding
      and sibling restoration, default/implicit bindings, original owners and source
      spans, folded expression attributes, reload compatibility and native discovery.
      Scenarios for later design verification: declaration order within an XML start
      tag cannot change QName values; discarded nodes cannot leak metadata to reused IDs.

- [x] Add literal attribute namespace capture to imported XML schema sources before
      enabling their automatic QName discovery. Missing metadata stays pending;
      legacy reload sidecars remain readable without inventing bindings.
      Scenarios for later design verification: CEM and XML use the same native catalog,
      and later namespace bindings never reinterpret earlier literal QNames.

Verification (2026-10-08): 210 native import, lexical capture/reload, declaration
reference and datatype tests pass; Nx schema-package verification and CLI build pass.

- [x] Add original shipped-descriptor fixtures for all 18 declarations, cross-kind
      bases, list items, typed enumerations, capability absence and consumer parity;
      add dedicated native attribute-type admission and pending-slot fixtures.
      Scenarios for later design verification: source names do not grant authority;
      a native type slot remains incomplete until its original contract is bound.

- [x] Add and implement original literal/native attribute type-slot binding fixtures:
      one exact compiled descriptor, explicit lexical binding, original scopes,
      directed crossing grants, request limits, pending query context, incomplete
      compilation and replacement-owner isolation. Binding alone cannot activate
      the model or clear native-type readiness.
      Scenarios for later design verification: matching names in a replacement
      owner cannot satisfy an old slot; a lexical binding grants no scope crossing.

- [x] Decide literal attribute validation input preparation before automatic
      descriptor activation. Adopted 2026-10-08: separately registered lexical
      preparation inherited from the original datatype, preserving authored
      text/source; incomplete preparation remains incomplete. Contract recorded in
      the [datatype proposal](cem-datatype-compilation-proposal.md#adopted-lexical-preparation-for-attribute-validation).
      Scenarios for later design verification: boolean validation must not start
      accepting conversion-only `1`/`0`; integer `003` retains its source spelling;
      list tokenization does not invoke an item converter.

- [x] Add lexical-preparation fixtures before implementation: exact registration
      and inherited identity, absent/unavailable capabilities, typed output checks,
      shipped lexical admission, original text/token provenance, no converter calls,
      rejection versus incomplete execution, cancellation and cumulative limits.
      Add package replacement coverage for an unavailable selected preparer.
      Scenarios for later design verification: prepared integer values keep their
      authored spelling; boolean presence remains compatible; ordered list items
      and duplicate tokens retain their original source attribution.

- [x] Implement registered lexical preparation and effective descriptor selection
      under the adopted contract, including scalar/list preparation and original
      implementation inheritance. Preserve existing typed/native validation paths
      and apply every effective datatype restriction after successful preparation.
      Completed 2026-10-08: explicit scalar and composed list APIs; automatic
      attribute-local facets and model activation remain separate actions below.
      Scenarios for later design verification: missing required preparation cannot
      activate automatic validation or fall back to conversion; derived restrictions
      cannot widen lexical admission; interrupted preparation stays incomplete.

Verification (2026-10-08): 210 datatype, value-contract, attribute-value,
template and package lifecycle tests pass, including eight new preparation fixture
groups and unavailable-preparer replacement/retry. All 18 original shipped
descriptors prepare and validate through explicit registrations. Schema-package
Nx verification and its CLI build prerequisite pass.

- [ ] Add remaining attribute descriptor-consumer fixtures for local facet
      intersection, typed node sequences, input preparation, cancellation and
      package replacement readiness before automatic validation integration.
      Scenarios for later design verification: validation never invokes conversion;
      unresolved binding cannot clear readiness or become an untyped string.

- [x] Complete remaining datatype metamodel admission and end-to-end shipped
      declaration/base/consumer parity using the registered grammar/reference
      capabilities. Add fixtures before wiring literal/native attribute admission.
      Scenarios for later design verification: attribute restrictions cannot widen
      a base; native node contracts retain typed input without scalar extraction.

Verification (2026-10-08): 330 native declaration-reference, datatype,
attribute-value, template and package lifecycle tests pass. The schema-package
Nx verification and CLI build pass. Automatic attribute activation remains guarded.

- [ ] Design whole-list inheritance syntax, dependency roles and compatibility as
      separate future work before admitting derived whole-list declarations.
      Scenarios for later design verification: list-of-list items are not inheritance;
      existing shipped item-base declarations preserve their effective item contracts.

- [x] Implement registered base compatibility and source-attributed validation
      in the bounded dependency compiler. Verify inherited-kind reuse, explicit
      cross-kind permission, incompatible node/scalar bases and unavailable
      registration. Shipped-family migration remains in the parity action above.
      Scenarios for later design verification: a derived restriction retains the
      base implementation identity; incompatible contracts never activate by name.

- [x] Add native converter fixtures for exact source/implementation binding,
      inherited reuse and explicit compatible replacement, absent versus unavailable
      capabilities, one-call execution without fallback, scalar/list/native outputs,
      empty output versus rejection, malformed representations, required candidates,
      cancellation, cumulative bounds and original source/diagnostic attribution.
      Add a package readiness fixture for an unavailable selected converter.
      Scenarios for later design verification: validation never calls a converter;
      converted output still faces every inherited/local restriction; native output
      cannot acquire a broader view or manufacture source nodes.

- [x] Implement effective native converter selection with original implementation
      identity, checked input/output signatures, inherited reuse and explicit
      compatible replacement. Invoke once, then validate all datatype restrictions;
      preserve unavailable selection and validation-only contracts without fallback.
      Scenarios for later design verification: one converter produces the canonical
      value; inherited/local restrictions all inspect it without rewriting or chaining.

Verification (2026-10-08): 64 focused native tests cover descriptor/result
validation, ten conversion fixtures and package lifecycle preservation. The schema
package Nx verification includes the CLI build. Native attribute `@type` remains
guarded; source/query conversion, equality and constant-interpreter profiles are
completed below.

- [x] Add source/query conversion fixtures for closed tagged results, empty versus
      absent output, exact signatures/owners, malformed and failed execution,
      original diagnostics, retained node boundaries, cancellation and bounded output.
      Scenarios for later design verification: authored profiles grant no execution
      authority; query conversion runs once before all effective restrictions.

- [x] Add schema-owned source/query converter bindings and a checked tagged-result
      adapter under the adopted conversion contract. The `datatype-conversion` profile
      checks fixed roles and explicit host registration; source/query execution uses
      the existing one-conversion path and all effective validation restrictions.
      Scenarios for later design verification: empty converted sequences remain
      present values; malformed tags/output never imply rejection or acceptance;
      source profiles grant no implementation authority by themselves.

Verification (2026-10-08): all 110 tests across datatype validation, value contracts
and schema-package lifecycle pass. The 94 datatype tests also pass after final
review changes; schema-package Nx verification and its CLI build prerequisite pass.

- [x] Add native equality/constant fixtures for numeric versus lexical equality,
      original token spans and owners, exact registration, inherited vocabulary
      intersections, unavailable/malformed/rejected preparation, no implicit converter,
      cancellation and cumulative comparison/preparation budgets. Add package retry
      coverage preserving the active package until its vocabulary is prepared.
      Scenarios for later design verification: derived equality cannot reinterpret a
      base vocabulary; missing lifecycle inputs never make constants ready.

- [x] Implement registered scalar equality and enumeration constant interpretation
      (2026-10-08), retaining each inherited vocabulary's original contract and
      implementations. Native compilation requires an explicit lifecycle runtime;
      preparation and validation share cumulative budgets and never invoke conversion.
      Scenarios for later design verification: a derived equality cannot widen a base
      vocabulary; unknown constant interpretation prevents readiness rather than fallback.

- [x] Add source/query enumeration fixtures for exact role/result signatures,
      explicit registration, closed query bindings, retained token owners and spans,
      malformed/failed/pending output, diagnostics, cancellation and shared budgets.
      Scenarios for later design verification: incomplete preparation cannot activate
      a package; inherited vocabularies retain their original equality implementation.

- [x] Add checked schema-owned source/query equality and constant-interpreter
      profiles (2026-10-08), with exact signatures, explicit host registrations,
      typed/tagged results, original owners and shared context/control propagation.
      Constant queries receive token text and the original vocabulary attribute;
      the existing bounded preparation/validation path retains token spans.
      Scenarios for later design verification: a query returning no comparison boolean
      stays incomplete; malformed constants cannot activate a package; no lexical fallback.

Verification (2026-10-08): 118 tests pass across datatype validation, shared value
contracts and schema-package lifecycle, including eight new source/query enumeration
fixture groups. Schema-package Nx verification and its CLI build prerequisite pass.

- [x] Decide enumeration constant authoring: preserve whitespace-token `@values`
      and defer richer retained constants. Adopted 2026-10-08; each token is one
      constant, checked through its original registered datatype contract.
      Scenarios for later design verification: `@values="In progress"` means two
      tokens, never one string constant containing a space; no implicit quoting,
      JSON parsing or `{value | …}` child syntax.

- [ ] Deferred: design richer retained enumeration constant authoring only as a
      separate extension, including original owners, typed scalar admission,
      references, metamodel admission and compatibility with `@values` tokens.
      Add fixture actions before implementation.
      Scenarios for later design verification: whitespace-containing constants
      have an explicit representation; inherited restrictions retain their
      original bindings and never reinterpret existing token vocabularies.

- [x] Implement retained validation descriptors, explicit original-scope lexical
      bindings and a bounded dependency compiler. Preserve dependency sites,
      reference failures, partial sources and original restriction owners.
      Namespace/export discovery and remaining kind/facet consumers stay open above.
      Scenarios for later design verification: cycles and incomplete dependencies
      prevent activation while original declarations stay available for inspection.

- [ ] Integrate the descriptor into literal and native attribute type consumers,
      preserve compatibility through explicit parity checks, and update the
      metamodel's native datatype admission contract before enabling consumption.
      Original-slot binding, dedicated metamodel admission and explicit lexical
      preparation are implemented; automatic lifecycle integration, local facets
      and activation remain open.
      Scenarios for later design verification: referenced type owners and their
      restrictions survive package replacement and independent runtime contexts.

- [ ] Once adopted, merge the proposal into maintained designs and remove the
      temporary proposal, preserving outstanding action items and scenarios.
      Scenarios for later design verification: normative docs distinguish supported
      consumers from implementation phases and genuinely deferred decisions.

## Deferred cem-element reference consumption

Initially deferred on 2026-10-04 until the CEM-ML reference design was complete.
The [consumer mode](cem-element-reference-ids-design.md) is adopted and implemented
on 2026-10-07 for local and explicitly admitted foreign producer relationships. Generic
CEM-QL export remains separate. Explicit source lifecycle evaluation is available
through the native embedding adapter and explicit browser/worker host inputs.
Each invocation supplies original capture, context readiness, effective policies
and directed grants; document ownership and producer placement create no authority.

Completed prerequisites and verification: [archived checklist](archive/todo-snapshot-2026-10-07.md#deferred-cem-element-reference-consumption).

### Next consumer actions

Execution order: continue the remaining consumer actions below while
general datatype execution awaits its separate contracts. Conditional prelude
authoring remains deferred unless explicitly requested.

Adopted 2026-10-07: allow transaction-scoped prepared placements. The host names
all producers and revisions, retains the private preparation snapshot and activates
relationships only after the whole group commits. Ordinary publication and
serialized resume hints cannot confer this readiness or authority.

- [x] Implement the bounded native placement-admission snapshot and export API
      from the [placement design](cem-element-granted-placements-design.md).
      Add native fixtures before implementation for both grant requirements,
      unique placement selection, producer-owned ID reservations, request/destination
      work bounds and stale/revoked revisions. Add real WASM worker/fallback cases.
      Scenarios for later design verification: a matching foreign ID grants
      nothing; local ambiguity stays ambiguous; an unready or revoked admission
      returns no replacement plan and does not mutate another producer's AST.
      Completed 2026-10-07: eight native admission fixtures and committed/prepared
      real-WASM cases in both heaps; the maintained gate passes 336 checks.

- [x] Implement the host placement registry/coordinator and transactional browser
      publication using native admission tokens. Add browser fixtures before
      implementation for same-root readiness, ID reservation conflicts, nested
      producer readiness, replacement races, revocation and disconnect cleanup.
      Scenarios for later design verification: changed dependencies invalidate
      preparation; coordinated producers publish all relationships together;
      revocation clears dependent routes immediately without transferring context.
      Completed 2026-10-07: five authority unit fixtures and four browser stories
      cover copied snapshots, partial revocation, activation rollback, nested
      prepared producers, conflicts, stale publication, activation-time revocation
      with safe retry and worker/fallback cleanup.

- [x] Extend placement admission to SSR and hydration with fresh host authority.
      Add retained Node/Edge/browser fixtures before implementation for coordinated
      SSR publication, profile/revision handoff, reconnect and stale resume hints.
      Scenarios for later design verification: serialized placement descriptors
      grant no authority; hydration enables routes only after live re-admission;
      stable producer identity does not make an uncommitted revision ready.
      Completed 2026-10-07: retained SSR group fixtures and worker/fallback
      hydration/reconnect stories pass; all 56 Node SSR and 12 Edge browser checks
      pass with the fixed Recommendation profile and authority-free resume hints.

- [x] Implement explicit versioned ARIA export profile selection after native and
      worker/fallback/SSR profile fixtures, using the [reviewed contract](cem-element-aria-reference-profile.md).
      Document a browser/assistive-technology compatibility matrix before enabling
      the experimental draft profile; preserve the Recommendation default.
      Scenarios for later design verification: one/many/empty/repeated targets
      obey the selected profile; profile mismatch requires rerender; unknown
      profiles fail preparation; authored reference order/identity remain intact.
      Fixture work: add native cardinality/identity cases, real WASM worker and
      fallback export cases, processing identity checks, SSR profile handoff and
      browser hydration mismatch coverage before implementing each layer.
      Completed 2026-10-07: exact pinned default/experimental options propagate
      through native export, workers, fallback, cache/revision identity and SSR;
      unknown profiles reject and hydration mismatch rerenders. Native cardinality,
      retained placement metadata, real-WASM heaps and focused browser/SSR checks
      pass. The published matrix separates DOM evidence from pending manual AT work.

- [x] Extend the accepted focus/geometry contract to native dialog/task/tooltip
      surface adapters and invocation-captured pointer/selection geometry after
      their semantic lifecycle is implemented. Add native/browser fixtures before
      each adapter; reuse typed reference consumption and shared role checks.
      Scenarios for later design verification: modal `none` preserves native
      focus entry; pointer geometry supplies no return-focus destination; missing
      invocation geometry rejects opening rather than borrowing another context.
      Adopted implementation scope 2026-10-07: shared native-surface lifecycle
      and reference adapters first; component migration remains separate.
      Fixture work: add native dialog/tooltip reference slots, immutable pointer
      and selection capture/logical fitting units, and browser plays for native modal/nonmodal/
      popover ownership, typed focus/return/boundary endpoints, context rejection,
      cancel versus forced close, missing geometry, tooltip interest and cleanup.
      Fixture work: verify that a completed Tab/outside interaction cannot set a
      later native close reason, changed overflow releases prior size constraints,
      and changing a tooltip profile releases owned description bindings.
      Completed 2026-10-07: eager `native-surface` shared capability/controller,
      native modal/nonmodal/popover ownership and tooltip interest lifecycle use
      the existing typed role adapters. Immutable invocation geometry and logical
      fitting units, native role slots and five browser stories cover the scope;
      both worker and fallback retain modal owners/focus across rerender. Further
      convenience/body lifecycle delivery and component migration stay separate.
      Verification: maintained native consumer gate 339 checks across 36 suites
      plus real WASM workers/fallback; runtime units 600, focused browser 30,
      Node SSR 58 and Edge browser 13. Build, typecheck and lint pass (two existing
      lint warnings). Manual AT interoperability is not covered by these checks.

### Scenarios for later design verification

- A consumer uses the referenced node's declared meaning to generate a native
  relationship, preserving a target element's explicit ID or generating one.
- Repeated template instances resolve references against their own supplied
  contexts and produced elements. Verify intended scope crossings and the
  specified local-name compatibility without a document-wide CEM ID lookup.

## Interaction design implementation: cem-action

The [accepted interaction design](cem-interaction-design.md) takes precedence over conflicting implementation. Deliver one component at a time; this first step owns the action invoker, not popup/menu/dialog lifecycle.

Completed prerequisites and verification: [archived checklist](archive/todo-snapshot-2026-10-07.md#interaction-design-implementation-cem-action).

- [x] Follow-up fixture: preserve unrelated anonymous demo slice state when native
      popup/modal activation is followed by a theme-switch rerender. The earlier
      combined sequence reset the layout-choice demo; retain the full sequence
      as a regression check.
      Fixture work: extend the source/installed action-gallery check across the
      full native surface sequence and add a focused worker/fallback runtime
      story asserting retained demo/declaration/instance identity and slice state.
      Scenarios for later design verification: native visibility/focus changes
      do not replace unrelated anonymous owners; theme projection preserves an
      edited slice without replaying its authored default.
      Completed 2026-10-07: the current runtime passes the extended action gallery
      from source and isolated package archives without a runtime change. The
      colocated theme-switch story verifies worker/fallback slice state and exact
      demo/declaration/instance/group identity after popover and modal commands,
      then Dark/Native/Light rerenders. All 30 focused theme-switch/action/native-
      surface browser plays, source/installed action checks, typecheck and lint
      pass; the lint warnings are existing baseline warnings.

## Autocomplete and suggestions design for CEM inputs

Reference: [legacy autocomplete 0.0.39](https://unpkg.com/@epa-wg/custom-element-dist@0.0.39/src/material/components/autocomplete.html).

- [x] Audit and map the source autocomplete API and examples, including its `input`/`menu` slots, explicit data/option values, text-as-value fallback, grouped suggestions, filtering and selected-value behavior; distinguish preserved contracts from extensions.
      Completed 2026-10-07: [source audit](cem-autocomplete-source-audit.md)
      inventories all 27 attributes, both slots and all seven demos against the
      pinned 0.0.39 source and dependencies. The local declaration and demos are
      byte-identical. Filtering and option commit are documented intents without
      implementation in the inspected declarations; parity markers and the
      frozen standalone product are separate evidence. Explicit option values
      are a compatibility case to verify, not a demonstrated source example.

- [x] Design reusable suggestions composition that attaches to existing `cem-field`, `cem-text-field` and applicable CEM input controls through their actual native input; preserve one form owner, naming, value/input/change events, validation, disabled/readonly behavior and normal editing. Decide textarea applicability explicitly.
      Completed 2026-10-07: [adopted composition design](cem-suggestions-composition-design.md).
      Attach through the field's shared editor provider, retain one form/value
      owner, and commit stored values with labels in the suggestion presentation.
      The first custom-listbox profile admits native text inputs without `list`;
      additional native profiles and label/submission conversion are separate
      extensions. Textarea is explicitly excluded from this whole-value profile.
      This closes composition design only; no runtime attachment is implemented.
      Rationale: the [source audit](cem-autocomplete-source-audit.md#value-filtering-and-compatibility-boundaries)
      separates stored-value/display-label intent from the frozen product.
      Scenarios for later design verification: the additional numeric profile
      keeps stored values with word labels input-compatible; an edit to empty
      survives refresh and rerender; one named field contributes one form value.

- [x] Evaluate reuse of `cem-dropdown` for autocomplete/suggestions: compare direct dropdown composition with reuse of its shared popup capability/controller for visibility, placement, collision handling and dismissal. Keep focus on the editable input, use combobox/listbox semantics rather than menu semantics, and avoid duplicate triggers, focus restoration handlers or form controls; record the selected composition and tradeoffs before implementation.
      Completed 2026-10-07: [adopted popup-service design](cem-suggestions-popup-design.md).
      Reuse shared geometry/native surface services through an explicit suggestions
      profile, using a manual native popover and one dismissal region spanning
      editor/listbox. Dropdown's button/link trigger, focus and ARIA policy remain
      separate. Neutral services and a listbox delegate require implementation;
      the current native adapter admits only dialog/tooltip kinds.
      Follow the [composition ownership contract](cem-suggestions-composition-design.md#composition-and-ownership);
      verify editor support rather than inheriting button-trigger or menu focus rules.
      Scenarios for later design verification: caret clicks do not toggle/dismiss;
      nested Escape closes only suggestions; refits preserve input identity and
      selection; independent surface leases do not overwrite one another.

- [x] Specify declarative attachment/slots and the shared cem-elements capability for filtering, active suggestion, commit/cancel and popup lifecycle. Reuse dropdown popup geometry/dismissal without menu roles: editable combobox + listbox/options, input focus retained with aria-activedescendant and explicit controls/expanded relationships.
      Completed 2026-10-07: [adopted attachment/capability design](cem-suggestions-attachment-design.md).
      Select non-form `cem-suggestions` with shared `suggestions` capability,
      exclusive editor slot or typed/local `editor-for`, native options input or
      inert options template, and row/group label templates inside owned shells.
      Local contains/prefix matching, external query-dependent filtering and an
      unfiltered mode have explicit ownership and truthful autocomplete semantics.
      Native adapters distinguish explicit empty values, text fallback and labels;
      sources remain native nodes, separate from their produced row placements.
      This closes API design only; none of these new exports is implemented.
      Apply the [manual listbox surface and service boundary](cem-suggestions-popup-design.md#shared-service-boundary).
      Identify editor/provider/surface roles explicitly; require one visibility
      controller and one semantic owner without a generated dropdown button.
      Record filtering ownership and compatibility for legacy `data` values and
      native options, including explicit empty values, text fallback and labels
      that differ from text. Do not silently inherit the frozen product's
      upstream-filtering or computed-label value fallback.
      Scenarios for later design verification: grouped filtering hides empty
      groups without changing retained option identity; clearing or replacing
      suggestions removes stale active-descendant relationships; filtering does
      not transfer a row ID to another source; conflicting endpoints/claims do
      not select a mounting-order winner; external/none modes expose list/none.

- [x] Define ArrowUp/Down, Enter commit, Escape cancel/dismiss, Tab, pointer selection, IME composition, native text editing, empty/no-match and hidden/disabled suggestion behavior; distinguish display label from committed value and define free-text versus constrained selection.
      Completed 2026-10-07 in the [interaction/selection design](cem-suggestions-interaction-design.md).
      Adopt manual preview and explicit stored-value commit; focus/edit opening
      has no character threshold, unavailable rows are skipped, and close never
      clears or accepts text. Define cancellable replacement beforeinput,
      synchronous field-owner notifications with reentrancy/revision guards,
      public bubbling change reconciliation distinct from raw capture traces,
      native history without an undo-entry promise, shared IME/held-press routing
      and opt-in commit-provenance validity through a leased field contributor.
      Preserve [value/form handoff](cem-suggestions-composition-design.md#commit-through-the-existing-form-owner).
      Apply the [attachment state and accessibility handoff](cem-suggestions-attachment-design.md#accessibility-and-lifecycle-handoff).
      Set preview/aria-selected behavior explicitly, without copying source
      selected flags; retain one commit/close route and guard stale query revisions.
      Scenarios for later design verification: event observers see one coherent
      native/host/submitted value; commit followed by blur has no duplicate public
      bubbling change while raw ancestor capture is reported separately;
      a later real edit still changes normally; option replacement has an explicit
      tested undo/redo policy; Enter without a commit preserves form behavior.
      Follow [close routing](cem-suggestions-popup-design.md#focus-and-close-routing):
      one handled Escape cannot also close a parent dialog; pointer activation,
      focus-leave and scrolling must not compete with outside dismissal.

- [x] Define static and externally loaded suggestion data at the retained CEM tree boundary, with optional asynchronous loading, stale-response handling, loading/error feedback and accessible announcements; keep UI behavior out of components, galleries and application JavaScript.
      Completed 2026-10-07 in the [source-data lifecycle design](cem-suggestions-data-design.md).
      Adopt atomic retained snapshots, duplicate values with distinct identity,
      explicit readiness/query-revision coordination, page versus vocabulary
      proof lifetime, shared loader/native query ownership, cancellation and
      current-session polite status feedback. Public coordination names are
      planned contracts; outbound native view publication is an implementation
      prerequisite, not a component-local data event/record API.
      Apply the [retained source adapters and filtering boundary](cem-suggestions-attachment-design.md#retained-source-adapters).
      Define duplicate-value policy, source replacement/invalid-revision recovery,
      external result/query-revision association, request cancellation and native
      query/loader bindings; no options URL, record normalizer or second filter.
      Define committed-proof lifetime separately from filtered result pages:
      retain proof through display filtering, distinguish authoritative source
      invalidation/revocation, and never infer it by value equality. Integrate
      settled IME queries and generation guards with loading/status feedback.
      Scenarios for later design verification: a late result from a dismissed
      opening generation cannot reopen the popup; refresh preserves the field's
      value and focus; source readiness never overwrites field-owned `busy`.
      A committed candidate absent from a filtered page is not invalidated merely
      by absence; authoritative revocation releases proof without rewriting text.

- [x] Finalize and implement the retained native source/view transport for
      suggestions before exporting its public capability. Adopted 2026-10-07:
      a separate CEMB-backed capability-session channel retains executable
      source capture/lifecycle contexts, with CEMV for the derived presentation
      view.
      Existing native CEMV export explicitly
      rejects executable source references; no copied records or premature deep
      expansion of descendant references may substitute for retained ownership.
      Fixture action: add native source/derived-view tests before transport code,
      then worker/fallback, grant, bounds, query revision and disposal cases.
      Scenarios for later design verification: label/content templates retain
      original scopes and native source edges; original descendant references
      survive; stale views release ownership and cannot regain authority on resume.
      Fixture action: add Rust native-session cases before implementation for
      original identity, retained descendants, independent frames, directed
      grants, incomplete selections and shared request/destination bounds; add
      WASM and real browser worker/fallback lifecycle cases for session ownership,
      derived CEMV projection, native label frames, cancellation and disposal.
      Fixture action: verify completed native namespace nodes can be inserted in
      label bodies with their namespaced attributes and original content, without
      a portable-value round trip or implicit descendant reference evaluation.
      Completed 2026-10-07: native immutable source selections, selected-forest
      namespace completion, label inputs and guarded CEMV presentation exports
      use shared lifecycle contexts/grants/bounds. Protocol v16 transports live
      session preparation/view/render/release separately from snapshot values;
      worker loss requires fresh authority. Transient leases reject stale query
      revisions, cleanup cancels/disposes retained owners, and root ownership is
      isolated. Verified native/WASM and worker/fallback/cancellation fixtures;
      suggestion-specific adapters/view publication/bindings remain the next item.
      Verification: 65 focused Rust checks, the standalone WASM session fixture,
      57 processing unit checks and four browser plays pass. cem-elements build,
      typecheck and lint pass, with two existing non-null-assertion warnings.

- [x] Implement the native suggestions source/text/filter consumer over retained
      capability sessions before binding the public suggestions declaration.
      Adapt canonical CEM options, HTML data, and HTML option/group roots by
      expanded name; preserve original owners, ordered groups/content, duplicate
      values and explicit empty values. Reject mixed families, repeated native
      identity, malformed declarations and unavailable owning axes atomically.
      Require materialized scalar value/label/alt attributes; prepare dynamic
      scalars upstream and retain descendant references for explicit consumers.
      Pin full default Unicode case folding and keep local filtering native.
      Provide immutable native row/group views and native label frames through
      the existing session channel, with scalar/CEMV presentation exports only.
      Fixture action: add Rust adapter/view contracts before implementation,
      then real WASM and browser worker/fallback cases for text/value distinctions,
      Unicode/filter agreement, source identity, retained descendant references,
      all-disabled/no-match states, bounds, diagnostics and stale/disposed leases.
      Fixture action: verify the packaged WASM and npm runtime include the pinned
      Unicode data's byte-identical copyright and permission notice.
      Scenarios for later design verification: equal values never merge source
      identities; filtering keeps the complete source-order row sequence; label
      frames retain original content/capture; invalid revisions cannot publish
      a partial plan or fall back to another source family.
      Completed 2026-10-07: retained native plans, immutable row/group views and
      label frames are available through protocol v17. Full default Unicode
      17.0.0 folding and materialized scalar admission are adopted; live views
      retain source/content edges and only explicit presentation projections
      export CEMV. The reference-consumer gate passes 354 native checks including
      nine adapter fixtures and its three real WASM lanes. Five browser plays
      across worker/fallback and 57 processing unit checks pass. Package
      build, clean npm import/Unicode notice, typecheck and lint pass; lint keeps
      the two existing non-null-assertion warnings. Component publication and
      outbound native bindings remain in the runtime action below.

- [x] Extract shared surface geometry leases before the suggestions semantic
      delegate. Isolate observations and fitting by actual panel under a shared
      host; reject competing geometry owners and fence queued callbacks, reset
      and disposal by the current lease. Restore only owned styles/placement
      claims, preserving newer authored state. Keep native dialog/tooltip
      registrations bound to their original semantic profile.
      Fixture action: add browser cases before implementation for independent
      panels/anchors, observer disposal and reacquisition, stale fitting/release,
      authored style priorities, profile conflicts and exact native visibility;
      rerun existing native-surface, dropdown and logical-placement contracts.
      Scenarios for later design verification: one surface's cleanup never
      removes another surface's observers; queued work cannot regain ownership;
      source and packaged runtimes cannot claim the same panel concurrently;
      suggestions adds its own focus/dismissal delegate to the shared geometry.
      Completed 2026-10-07: each panel has an exclusive transient lease shared
      across runtime copies, isolated observations and coalesced viewport work.
      Reset/disposal fence pending updates; style/placement cleanup preserves
      authored values and individual shorthand longhand priorities. Native
      dialog/tooltip adapters consume the lease and keep their original kind.
      Thirteen browser plays including existing native-surface, typed relationship
      and dropdown regressions, plus 25 logical/declaration unit checks pass.
      Build, typecheck, lint and fresh cache-bypassed npm package verification
      pass; the two existing lint warnings remain. Visibility/dismissal and
      native publication are the next separate prerequisites below.

- [x] Extract the shared native visibility/dismissal session service and add the
      manual-listbox semantic delegate using the completed geometry leases.
      Require one admitted editor/surface relationship and explicit readiness;
      preserve editor focus, inspect actual native visibility, reject external
      opening without admission and invalidate sessions with ancestor lifetime.
      Keep the existing dialog/tooltip/menu profiles on their own focus/close
      rules. Shared services must not filter sources or commit field values.
      Fixture action: add exact-owner, direct native open/hide, stale generation,
      independent surfaces, ancestor teardown and editor/listbox dismissal-region
      browser contracts before the delegate; include native veto/reentrancy,
      runtime-copy ownership, unsupported editor/list conflicts and pointer
      sequences that outlive an opening; rerun native profile regressions.
      Scenarios for later design verification: geometry cannot reopen a dismissed
      session; opening/refits preserve the existing editor/caret; an invalid
      anchor or zero-size surface never exposes expanded state; nested Escape
      and native parent teardown follow one close route without focus restoration.
      Completed 2026-10-07: shared transient owner registration and actual native
      visibility observation now serve the existing dialog/tooltip adapters and
      the package-private manual-listbox delegate. The delegate requires a
      verified text-editor provider lease, explicit consumer lifecycle admission
      and readiness, and captured ancestor lifetimes. Generation checks fence
      prepared openings and pointer sequences; native hide drops visibility
      claims synchronously. Unsupported editors/native list conflicts, failed
      geometry, ancestor teardown and authority/provider changes close without
      refocusing or value writes. Geometry supplies a bounded editor inline-size
      minimum. Seven new browser plays plus existing provider, native-surface,
      geometry, typed-reference and dropdown regressions pass (22 plays total),
      including trusted Escape inside a modal parent. All 25 logical/declaration
      unit checks, build, typecheck, lint and fresh npm package verification pass;
      the two existing lint warnings remain. Editor ARIA/placement publication,
      filtering/row activation and the public suggestions capability remain in
      the separate actions below.

- [x] Publish the retained suggestions view into component native frames and
      outbound bindings, then coordinate editor-provider attributes and original
      source-to-row placement claims through host-issued admissions in both
      directions. Reserve the native slice against authored writes, keep source
      identity distinct from output IDs, and reacquire all authority on resume.
      Fixture action: add native-frame contracts first and browser worker/fallback
      cases for native bindings, independent consumers, conflicting editor claims,
      stale row mappings, invalidated grants and coordinated producer revisions.
      - [x] Add native contracts for injecting the retained suggestions view into
        a fresh consumer frame, preserving source/content identity and rejecting
        authored writes to its reserved slice/alias before publication. Check
        independent consumer queries and atomic malformed-envelope rejection.
      - [x] Add provider-owned attribute lease contracts before implementation:
        atomic role/autocomplete conflicts, preserved unrelated controls tokens,
        current focused/visible row admission, stale revisions, newer authored
        writes, competing leases and reset/rebind/disconnect cleanup. Share one
        verified editor lease with the manual-listbox delegate.
      - [x] Complete cross-component live-view publication over original-owner
        routing. Add worker/fallback contracts before
        integrating native outbound bindings and the two-way row placement grants.
        - [x] Retain one immutable native suggestions publication on its original
          processing owner and route admitted consumer-frame rendering there.
          Fixture action: add Rust contracts before implementation for native view
          identity/source retention, independent consumer frames, reserved names,
          release/reuse and bounds; then worker/fallback browser cases for directed
          consumer leases, realm copies, stale in-flight work, independent release
          and owner loss.
        - [x] Integrate the routed publication leases with component outbound
          bindings and two-way original source-to-row placement admissions.
          Fixture action: exercise normal compiled component frames with one live
          owner plus materialized inputs, reserved-slice conflicts, stale in-flight
          work and owner loss in worker/fallback modes.
          - [x] Route canonical component render/diff frames through the original
            publication owner, retaining consumer compilation/module context,
            materialized native inputs, CSS/DOM publication and revision fences.
          - [x] Connect original source-to-row registrations and grants in both
            directions before activating provider attributes or commits. Keep
            source identities distinct from constructed shells and generated IDs.
            Decision adopted: the bridge exposes opaque handles limited to
            admitted native rows; retained source documents stay private.
            - [x] Fixture: verify native row handle/source identity, bounded
              scalar commit preparation, eligibility across query publications,
              source replacement and publication revocation before implementation;
              fence queued row preparation against publication release in engine tests.
            - [x] Fixture: verify explicit grants in both directions, exact
              provider/row placement mappings, conflicting claims, filtering,
              disconnected targets and fresh resume authority in worker/fallback.
            - [x] Complete automatic source-to-constructed-row mapping. Decision
              adopted 2026-10-08: return bounded placement metadata from native
              rendering, checked against the current publication and committed output.
              IDs and positional matching do not establish native identity.
              - [x] Fixture: native row annotations retain exact current-view identity,
                reject scalar/copied/foreign/duplicate rows and bound metadata;
                worker/fallback rendering maps only committed nodes, revokes stale
                frames and excludes placement authority from durable snapshots.
              - [x] Fixture: connect the shared controller to native-rendered
                placements in both worker/fallback modes, retaining the slotted
                editor while source/query renders replace row admissions.
            Scenarios for later design verification: independent views of a shared
            source, filtered/removed rows, conflicting claims and fresh resume grants.
      Decision adopted: each consumer frame admits one live publication owner;
      another live owner is a diagnostic. Ordinary materialized native inputs
      remain supported. Multi-owner execution requires a separate design.
      Decision adopted: retain live views on their original processing owner and
      route consuming jobs there. CEMV remains a materialized presentation channel;
      it cannot export a live view or restore consumer authority on resume.
      Progress 2026-10-07: native consumer-frame injection retains original
      source/content edges and rejects reserved aliases/slices. The provider owns
      transient ARIA leases with atomic conflicts, exact endpoint checks, editor
      revision fences and cleanup that preserves newer authored writes. Manual
      listbox visibility can borrow the same verified lease. These APIs require
      a trusted current-publication hook; original source-to-row placement
      admissions now activate provider claims through exact opaque native rows;
      automatic constructed-shell mapping is delivered; production composition
      remains open pending the local host-authorization adapter below.
      Owner-routed immutable publications and independent consumer-frame leases
      are delivered in worker/fallback modes. Copied/serialized bindings, stale
      in-flight results and post-loss authority fail closed. Native publication
      count/control-byte budgets include retired keys; release frees active view
      capacity without allowing identity reuse. Protocol v20 adds bounded native
      row controls and synchronous publication/source owner-loss revocation.
      Normal component frame routing on the original owner preserves materialized inputs and
      rejects unavailable owners, reserved-slice conflicts and stale results.
      Verification: native singleton projection contracts, worker/fallback component
      update and provider/browser regressions, unit checks, typecheck and lint pass.
      Bridge/controller verification: 52 focused Rust contracts, 72 unit checks,
      25 browser checks, typecheck and clean package verification pass. Lint has
      only the two existing non-null-assertion warnings.
      Progress 2026-10-08: `suggestion-row` consumes an exact native row or an
      already-evaluated reference to it in the current immutable publication.
      Bounded render metadata carries its mapping through protocol v21, while the
      annotation is removed from DOM/CEMV output. The runtime captures exact
      committed elements and expires mappings on newer renders, failed updates,
      disconnect and source/publication loss. No mapping authority enters snapshots.
      Mapping verification: 53 focused Rust contracts, 72 unit checks, 16 browser
      checks, typecheck, lint (two existing warnings) and fresh package build/import
      verification pass. Production composition awaits the adopted local adapter below.
      Scenarios for later design verification: separately produced editor and
      listbox relationships activate together; filtering retains original row
      identities; a stale view cannot restore an active descendant or authority;
      releasing one claim preserves authored and unrelated ARIA relationships.

- [ ] Implement the suggestions controller/capability and declarative field
      composition over the completed editor-provider and native source contracts.
      Use the visibility/dismissal and native-publication prerequisites above.
      - [x] Add native `editor-for` singleton projection/grant contracts and the
        shared exact-provider endpoint adapter. Fixture action: verify typed
        references, scoped local names, direct editor slots, ambiguity, unsupported
        controls and reference observation; endpoint lookup never issues grants.
      - [x] Implement the adopted source-session/query-publication lifetime split
        before wiring the controller: retain the source session across queries
        and put query freshness on immutable publications/leases.
        Fixture action: verify original source and row identity across native
        query views and worker/fallback publications; reject stale consumer
        results without releasing the reusable source session.
        Scenarios for later design verification: filtering preserves original
        source identity, stale publications cannot activate/commit, source
        replacement and owner loss revoke affected leases independently of queries.
      - [x] Add the shared controller over admitted native row handles, exact
        editor leases and manual-listbox sessions. Fixture action: verify clamped
        keyboard navigation, no automatic commit, beforeinput veto, provenance,
        reset/IME cleanup, stale queries and pointer cancellation before wiring
        the declarative capability.
      - [x] Fixture: wire the shared declarative capability through an explicit
        trusted host preparation hook; reject missing authority, endpoint changes
        and copied/resumed row handles without creating component-local behavior.
      - [x] Decide local host authorization for production composition.
        Adopted 2026-10-08: explicit runtime opt-in authorizes native capture of
        local slotted options and exact local editor/listbox/row grants. Foreign
        source crossings retain explicit grants; slots alone confer none. See
        [local host authorization](cem-suggestions-attachment-design.md#local-host-authorization).
        Scenarios for later design verification: an unconfigured host stays
        inactive; local opt-in cannot admit unrelated foreign sources; revocation
        and resume require current authority.
      - [x] Implement the shared local suggestions authorization adapter before
        production composition: expose a revocable host-only runtime opt-in,
        capture the one local inert options template at the native CEM boundary,
        retain its session across queries, and use exact provider/rendered-row
        identities to issue property-specific placement grants in both directions.
        Preserve explicit host hooks; diagnose input conflicts instead of falling
        back or extending local admission to external endpoints/foreign references.
        Fence asynchronous preparation and release affected sessions/publications,
        grants and provider claims on revocation, replacement, disconnect or loss.
        - [x] Fixture: verify bounded native XML capture with retained namespaces
          and original source identity, plus denied cross-source selection.
        - [x] Fixture: verify worker/fallback local opt-in, query reuse, source
          replacement, explicit-input conflicts, revocation and fresh resume.
        - [x] Fixture: verify worker/fallback XML import byte bounds, rejection
          of injected grant metadata, ready-empty local sources and isolation
          of provider endpoints belonging to a different runtime; verify owner
          loss cannot regain authority through a markup/configuration mutation; pending
          or failed readiness must not activate rows as a ready-empty source. Verify
          readiness recovery reacquires an expired editor lease even when the
          editor/provider pointers are unchanged after parent reconciliation. Verify
          conflicting live editor claims remain suspended rather than choosing
          a winner by reconnecting one controller.
        Fixture action: add native capture/grant cases first for retained lexical
        scopes, bounds and denied foreign references; then worker/fallback browser
        cases for unconfigured/local opt-in hosts, exact endpoint admission,
        conflicting slots/explicit inputs, query reuse, source replacement,
        revocation during preparation and fresh collapsed admission after resume.
        Delivered 2026-10-08: host-only `localSuggestions` opt-in and synchronous
        revocation, original inert payload capture through bounded native XML
        ingress (protocol v22), reusable source sessions, canonical owner/frame
        compilation and exact two-way placement grants. Source mutations,
        readiness, controller generations, expired editor connections and owner
        loss fence stale work; competing live claims remain suspended.
        Verification: 54 focused Rust contracts, 85 unit checks, 24 browser
        checks, typecheck and fresh package build/import verification pass.
        Lint has only the two existing non-null-assertion warnings. Production
        XHTML composition and the full field/device acceptance matrix stay open.
        Scenarios for later design verification: local composition needs no
        per-attachment host preparation code after opt-in; copying attributes,
        IDs or saved state cannot confer authority; stale work cannot restore
        revoked relationships or commit proof, and unrelated attachments survive.
      - [x] Assemble the production XHTML declaration with native options inputs,
        row/group label frames, rendered placement metadata, loading/failure/empty
        feedback and accessible label/description handoff. Depend on the local
        host-authorization adapter above; do not derive grants from authored IDs or row positions.
        - [x] Implement the prerequisite shared feedback lifecycle: expose current
          focused-session readiness/count, fence stale preparation and dismissal,
          and bind declaration-owned plain-text status and listbox busy state.
          Fixture: verify pending/failed/empty/all-disabled/count feedback,
          unchanged-text deduplication, editor focus/IME/availability, cancellation
          during preparation and clearing on revocation/disconnect in both owners;
          verify declaration-local message changes and invalid status targets.
          Delivered 2026-10-08: immutable presentation feedback, focused-session
          qualification, stale-request/dismissal fences and a shared adapter for
          a declaration-owned plain-text status region and listbox busy state.
          Localized message attributes remain declaration-owned; source handles
          and grants never enter the feedback snapshot. Preview does not rewrite
          unchanged status text, and field help/error/busy remain field-owned.
          Verification: 26 worker/fallback browser checks, 85 unit checks,
          typecheck and fresh package build/import verification pass. Lint retains
          its two existing non-null-assertion warnings. Screen-reader announcement
          interoperability stays in the full acceptance matrix below.
          Scenarios for later design verification: feedback never overwrites
          field help/error/busy, previews produce no repeated announcements, and
          stale requests cannot announce after Escape, blur or authority loss.
        - [x] Wire captured native option/group-label template inputs into the
          canonical component frame before completing the production declaration.
          Preserve original template scopes and reject interactive label output;
          do not recompile copied template strings with another source's bindings.
          Fixture: verify worker/fallback label templates, explicit-label/default
          precedence, stored-value hints, group names and denied template scopes.
          - [x] Fixture: add native bounded label-output admission cases before
            wiring templates; reject interactive descendants, roles/IDs/focus
            overrides, executable attributes and template-owned host/style writes.
            Verify original source attribution, native-content projection and
            node/depth/byte bounds without evaluating authored descendants;
            cover worker/fallback row/group rejection and reusable sessions after
            failed label rendering.
            Delivered 2026-10-08: native row/group rendering admits passive
            HTML/SVG output after bounded native-content projection and rejects
            interaction ownership, executable attributes, host writes and
            template-owned styles/module mappings. Invalid source nodes retain
            attribution; rejection leaves the source session usable.
            Verification: 55 focused Rust checks, 15 worker/fallback browser
            checks, typecheck and fresh package build/import verification pass.
            Scenarios for later design verification: captured template scope
            failures cannot fall back to consumer bindings, and production
            default/custom labels pass admission before attachment activation.
          - [x] Decide the execution context of local HTML CEM-ML label templates
            before canonical captured-template wiring. Adopted 2026-10-08: isolated
            source-owned compilation with captured namespaces, the explicit
            native `suggestion`/`group` parameter and standard library; do not
            inherit the consuming declaration's function/binding closure.
            Preserve original captured context for native module templates.
            Scenarios for later design verification: consumer-local functions
            cannot become ambient authority; equal template bytes from different
            sources retain distinct owners and namespace bindings.
          - [x] Fixture: verify original XML label captures, namespace completion,
            parameter isolation, bounded publication-wide output and original row
            identity in Rust first; then verify local slot invalidation and custom
            labels through worker/fallback canonical component rendering.
        - [x] Fixture: add the production declaration's colocated plays for both
          field providers, grouped/custom labels, stored-value hints, source
          conflicts, localized feedback, keyboard commit and field ownership.
          Add linked property/gallery pages and source/built browser coverage
          alongside the declaration; keep full device/accessibility acceptance
          in the dedicated matrix below.
        Delivered 2026-10-08: captured local label templates retain their original
        native owners and namespace contexts, compile with isolated explicit
        row/group parameters, and publish passive native `labelContent` atomically
        under shared output and retained-session budgets. Source/template edits
        invalidate leases in worker and fallback execution. The production
        `cem-suggestions` XHTML declaration composes both field providers, grouped
        rows, stored-value hints and localized status without component JavaScript.
        Linked property/gallery pages cover five themes and six states; source
        and isolated installed-package browser checks pass.
        Verification: 58 focused Rust checks, 19 browser plays, 607 unit checks,
        typecheck, fresh runtime package/import and page builds pass. Lint retains
        two existing warnings. The aggregate declarative gate remains blocked by
        the existing `cem-theme-switch` story imports; catalog verification and
        full device/accessibility/source-example parity remain below.
        Scenarios for later design verification: grouped/dynamic sources in both
        field types, slot conflicts, native source readiness, independent instances
        and fresh authority after resume.
      Inventory and add applicable native
      metadata/constraint forwarding on both field declarations before promising
      the legacy surface. Add focused fixture actions before implementation.
      Use the completed shared geometry leases, visibility/dismissal services
      and manual-listbox delegate, and preserve current
      dropdown/dialog/tooltip behavior with regression fixtures. Admit no source-
      or component-local popup controller or broadened button endpoint shortcut.
      Deliver native `editor-for` consumption/markers, coordinated editor claim
      leases and placement admissions in both directions, native options bindings,
      source/text/filter adapters with pinned Unicode folding, the reserved
      transient native view, label-template input frames and source-to-row mappings.
      Keep source identity distinct from DOM relationship targets; exclude live
      capability views/leases from durable resume authority.
      Implement the [interaction provider contract](cem-suggestions-interaction-design.md#provider-commit-and-event-contract):
      cancellable replacement beforeinput, coherent synchronous notifications,
      reentrancy guards and revision-aware native change checkpoints. Add shared
      composition/terminal-press and handled Enter/Escape routing to the field's
      implicit-submit path; ordinary editing remains native. Add leased validity
      contributors and commit-provenance tracking before require-selection.
      Deliver one row-activation/dismissal state machine and verify a shared
      focus-preserving adapter per supported pointer type/browser before claiming
      touch/pen support; preserve scrolling, never refocus after stale blur.
      Fixture action: add editor-provider stories before the transaction changes,
      covering exact owned-control admission, synchronous event/FormData/slice
      coherence, beforeinput veto/reentrancy, equal-string cause notifications,
      reset/restore/rebind, competing leases, merged validity and IME terminal
      keys in the shared deferred-submit route. Add retained native source/view
      fixtures before its worker/fallback/binding extension.
      Fixture action: verify coexisting runtime module copies share transient
      provider/key identities while leases remain bound to the original instance.
      Fixture action: extend both fields' native-attribute stories before adding
      scalar constraint/editing-hint forwarding; verify removal, native pattern
      validity and preserved form/name/reset ownership.
      Progress 2026-10-07: shared provider transactions, mutation-cause subscriptions,
      exclusive control leases, independent validity leases, change checkpoints,
      composition/held-press routing and fresh leases after disconnect/rebind are
      implemented. Both fields forward native scalar constraints/editing hints
      and list. The shared native source/view transport, atomic source/text/filter
      adapters and native row/group label frames are delivered. Shared native
      visibility/dismissal sessions and the manual-listbox semantic delegate are
      delivered with explicit lifecycle admission/readiness hooks. Native
      suggestions component outbound bindings and source sessions reusable across
      queries are delivered. Original source-to-row placement claims, the shared
      controller and the registered declarative capability with an explicit trusted
      preparation hook are delivered. Worker/fallback cases verify keyboard
      clamping, pending-query dismissal, IME cleanup, commit veto/provenance, exact
      endpoint changes and native mouse activation/drag cancellation. Physical
      touch/pen, the device/accessibility acceptance matrix and source-example parity
      remain open; do not mark this parent complete from these shared slices.
      Production local composition, captured labels and feedback were delivered
      2026-10-08.
      Reserved native consumer-frame publication and provider-owned attribute
      leases now integrate with current two-way placement grants.
      Scenarios for later design verification: submission and validity update
      before commit observers; stale render work cannot restore an older edit;
      the authored reset default survives commits; listeners and ARIA claims are
      released without patching another producer's owned output.
      Beforeinput veto leaves text untouched; superseding callbacks do not emit
      stale change; author custom/native/selection validity compose without one
      claim clearing another; IME terminal keys do not submit or select.

- [ ] Fixture: add shared native contract cases first, then browser stories applying the same suggestions to cem-field and cem-text-field (and any accepted additional input), covering forms, dynamic suggestions, grouped filtering, focus, IME, independent controls and cancellation.
      - [x] Add production both-field acceptance plays for native mouse/touch/pen
        activation, native touch movement/cancel, wheel scrolling and stale row
        rejection; first verify the shared focus-preserving row adapter in
        worker/fallback execution.
        Scenarios for later design verification: pointerdown stays native for
        touch/pen, scrolling never commits, compatibility focus suppression is
        row-specific and blur never leads to a refocus or stale commit.
      - [x] Add production both-field acceptance plays for independent grouped
        filtering, disabled/read-only changes, type/list conflict recovery, source
        replacement, beforeinput veto and coherent raw/public input/change/FormData
        ownership.
        Scenarios for later design verification: one attachment's revision never
        mutates another; equal text alone cannot establish selection provenance.
      - [x] Fixture: verify observer-driven suggestions synchronization consumes
        committed parent render frames in concurrent source/installed-package
        galleries; retain current endpoint identity through profile recovery and
        fence queued synchronization after disconnect/reconnect.
        Scenarios for later design verification: native label/row publication
        never starts from an intermediate CSS/DOM frame or restores a retired
        capability connection.
      - [x] Add Chromium browser-input composition settlement and terminal-key
        fixtures through the real editor; record this as protocol-driven browser
        evidence and retain physical keyboard/IME and assistive-technology runs
        as an explicit release checklist.
        Scenarios for later design verification: composition settles from actual
        editor text, its terminal Enter cannot select or submit, and a fresh
        deliberate key remains available after keyup.
      Delivered 2026-10-08: shared row activation now admits mouse/touch/pen
      while preserving native touch/pen pointerdown. Exact-row compatibility
      mousedown suppresses focus loss, and normal implicit capture release after
      completed pointerup preserves the click admission. Cancel/drag/scroll,
      source loss and blur still invalidate it. The capability observes type/list
      and availability changes so unsupported profiles recover with fresh leases.
      Observer-driven synchronization waits for the committed parent CSS/DOM
      frame and fences reconnect callbacks. Scalar pending/failure readiness
      withdraws the prior view immediately and preserves focused feedback while
      its render settles. Concurrent source/installed-package galleries verify
      that row publication does not overlap an intermediate parent frame.
      Both production fields have colocated plays for independent grouped filters,
      source replacement, type/list/input conflicts, commit veto, raw/public
      event coherence, native pointer taps/cancel/drag/wheel and protocol-driven
      composition settlement. Verification: 38 browser plays across controller,
      production declaration, manual listbox, geometry and native-surface suites,
      plus 607 unit checks, typecheck, lint (two existing warnings) and fresh
      cache-bypassed package build/import verification pass. Source and isolated
      installed-package property/gallery browser verification also passes.
      The [acceptance evidence](../packages/cem-components/docs/suggestions-acceptance.md)
      records the automated boundary and remaining release runs. This parent
      remains open for physical and host-integrated cases below.
      - [ ] Complete physical touch/pen, real keyboard/IME, screen-reader and
        mobile viewport runs using the [acceptance record](../packages/cem-components/docs/suggestions-acceptance.md).
        Record browser/OS/device/AT versions and exact outcomes. The automated
        touch gesture does not scroll a plain overflow-box control in this
        environment; a wheel-scroll pass does not substitute for touch-pan.
        Preparation 2026-10-08: acceptance record now provides a repeatable
        per-device/field/version/evidence template and plain-scroll baseline
        procedure. Actual hardware, OS IME and AT observations remain pending.
        Scenarios for later design verification: tap preserves editor/caret,
        pan remains scrolling, terminal IME keys neither select nor submit, and
        active-row/status announcements preserve the field's label/help.
      - [x] Add host-integrated both-field acceptance fixtures for explicit
        external native sources and trusted ancestor-surface lifetimes, covering
        nested Escape, modal containment, parent teardown and resume. Existing
        shared service tests do not establish production local-slot admission
        inside a foreign ancestor surface.
        Delivered 2026-10-08: both production fields and the production
        suggestions template consume external CEMB source owners through trusted
        host hooks in worker/fallback execution. Modal parent lifetimes cover
        nested Escape/held repeats, commit/FormData coherence, parent close,
        expired-lifetime rejection after reopen, fresh host admission and parent
        teardown. Source preparation must settle before a fresh deliberate key;
        pending keys do not replay. The production and shared controller/listbox/
        geometry/native-surface suites pass all 39 browser plays; typecheck and
        lint pass (two existing warnings).
        Scenarios for later design verification: adjacency cannot confer parent
        lifetime authority and a closed ancestor cannot revive an old child.
      Progress 2026-10-07: editor-provider transaction/admission/reentrancy stories
      and both fields' native forwarding/pattern stories are delivered. Browser
      checks distinguish raw capture/public bubbling, cover a later native edit,
      captured composition-terminal Enter and expired leases; they do not claim
      device IME or full suggestions acceptance coverage.
      Verified 2026-10-07: 22 focused provider/form/native-surface/field browser
      plays and 19 declaration-contract unit checks pass; cem-elements typecheck
      and lint pass (two existing non-null-assertion warnings). The components
      declarative gate fails on existing theme-switch story imports; an unchanged
      HEAD archive reproduces the identical failure. Full suggestions and its
      remaining matrix stay open at capability/declaration delivery.
      Progress 2026-10-07: native session identity/namespace/label/export cases,
      worker/fallback grant/bound/revision/disposal cases and cancellation after
      import are delivered. The source channel uses the adopted CEMB boundary;
      no transport decision remains pending. These shared transport checks do not
      substitute for the full suggestions/browser/native input acceptance matrix.
      Include [composition acceptance scenarios](cem-suggestions-composition-design.md#delivery-and-verification):
      one submission owner, supported text editors, unsupported type/list/role
      conflicts, independent sessions over a shared native source, disable/readonly
      during an open session and fresh editor/placement admission after resume.
      Add [popup verification scenarios](cem-suggestions-popup-design.md#delivery-gaps-and-verification):
      native visibility/ARIA agreement, editor clicks and pointer scrolling,
      nested Escape/modal containment/ancestor teardown, RTL/vertical and viewport/
      container fitting, lost anchor/boundary closure, observer/style cleanup,
      stale callbacks and unchanged dropdown/dialog/tooltip profiles.
      Add [attachment acceptance cases](cem-suggestions-attachment-design.md):
      slot/external binding on both fields, conflicting editor/source inputs and
      competing claims, native option label/text differences and empty values,
      adjacent text nodes/comments/ASCII versus non-ASCII whitespace, mixed-family
      rejection, label-only templates, Unicode filter agreement, external/none
      autocomplete semantics and full-row filtering without identity transfer.
      Add [interaction acceptance cases](cem-suggestions-interaction-design.md#alternatives-and-verification):
      no auto-first/blur/Tab commit, clamped/disabled navigation, exact preview
      ARIA, Enter veto/no-selection/held repeats, nested Escape and native caret
      editing. Exercise native final IME keys/input ordering and captured deferred
      submit ownership, rather than treating synthetic keys as IME evidence.
      Verify pointer mouse/touch/pen focus, pan/scroll/cancel, stale and accessibility
      activation; no panel-wide cancellation or refocus-after-blur workaround.
      Trace raw capture and public bubbling input/change separately through commit,
      blur, later edits, reentrancy and equal-string setter/reset/restore causes.
      Check optional/required empty values, initial/restored constrained text,
      equal-value distinct sources, custom/native/selection error precedence and
      native undo/redo/caret results with coherent field/submission state.

- [x] Fixture: complete suggestions source-example and public-contract parity across the linked property playground and gallery: map all source examples, verify forced colors and keyboard/pointer use, and finish catalog verification.
      Delivered 2026-10-08: linked pages, five-theme/six-state matrix, public
      attribute/slot and accessibility inventories, package exports, and source/
      generated/isolated-installed-package browser checks for the first local
      profile. The seven source legends now map to executable gallery examples
      with numeric editing explicitly separated from deferred numeric selection.
      Native option edge cases, grouped filtering, retained edits, keyboard/mouse
      commits, original focus/FormData and forced-colors preview checks pass from
      source and isolated archives. Shared setup fixes the theme-switch import
      violation; the aggregate declarative gate, 53-component catalog build,
      package/catalog verification, 27 theme-switch/suggestions browser plays,
      typecheck and lint (two existing warnings) all pass.
      - [x] Fixture: add seven legend-addressed source-intent samples and native
        value/label edge cases; verify editing, grouped filtering, keyboard and
        mouse commit, and forced-colors active-row visibility from source and
        isolated packages. Keep numeric selection explicitly deferred.
        Scenarios for later design verification: explicit empty values remain
        distinct from missing values, native text fallback differs from an
        explicit label, and filtering hides empty groups without transferring
        row identity.
      Map all [seven audited source examples](cem-autocomplete-source-audit.md#map-of-the-seven-examples),
      add explicit native option values and label/text edge cases, and distinguish
      example-intent coverage from the existing static material parity markers.
      Report first-profile coverage honestly: numeric suggestions remain pending
      the additional native profile; bare numeric field editing is not autocomplete
      selection evidence.

- [x] Design additional native suggestion profiles for search/email/tel/url and
      number inputs, evaluating `list`/datalist integration without duplicate
      popups or loss of native roles/constraints. Separately assess an explicit
      field-owner label/submission conversion contract and a multiline/token
      completion profile if those extensions are needed; no implicit type or
      value-contract conversion is admitted by the first text-input profile.
      Adopted 2026-10-08: [native profile design](cem-suggestions-native-profiles-design.md)
      selects explicit browser-owned datalist presentation, with original native
      sources and provider-owned list claims. Custom text listbox remains default.
      The design is complete; runtime/profile acceptance is tracked below.
      Scenarios for later design verification: the numeric source example commits
      `1` rather than `One`; native browser routes remain singular; a future
      multiline profile preserves newline/caret/undo and accessible feedback.
      Separately design full suggestion surface-provider adoption if needed;
      the first profile accepts label templates only. Scenarios for later design
      verification: an adopted listbox has one visibility/semantic owner, valid
      row mappings and fresh foreign placement authority, without menu behavior.

- [x] Implement the [native-datalist profile](cem-suggestions-native-profiles-design.md)
      source projection: retain original owners/scopes, consume bounded ungrouped
      materialized sets, preserve value/label rules, omit unavailable/empty rows,
      and reject grouped/rich-label/query-dependent inputs with diagnostics.
      Fixture action: native Rust cases for numeric values versus word labels,
      missing/empty native values, duplicates, mixed families and scope/reference
      bounds. Keep the WASM control-envelope fixture above the source-preparation
      byte floor so it exercises the intended control bound.
      Scenarios for later design verification: projection does not infer
      committed source identity or silently flatten groups; numeric value is `1`.
      Delivered 2026-10-08: Rust sessions cache a distinct native datalist view
      and bind it directly into the reserved suggestions frame. Original source
      owners/scopes remain accessible, while ordered value/label projections
      omit hidden/disabled/empty rows and attribute empty-value warnings. Groups,
      captured label templates and explicit query/filter/selection controls are
      rejected. CEMV export rejects the live view; scalar extraction is explicit.
      Source/work/depth/byte bounds, grant denial, independent contexts and atomic
      frame failures pass alongside the existing native consumer regressions:
      41 focused Rust checks, the WASM build and native session bridge checks pass.
      Browser/provider wiring remains in the following implementation items.

- [x] Add provider-owned datalist claims and native `list` placement admission:
      retain the original input, enforce exact current grants/unique target IDs,
      preserve authored relationships and newer author writes, and stage options
      before claiming the relationship. Fixture action: shared provider cases for
      conflicts, competing profiles, source readiness/replacement, revision loss,
      disposal and resume. Scenarios for later design verification: only one route
      owns the editor; revoked or serialized IDs cannot reestablish authority.
  - [x] Add shared browser fixtures for directed datalist grants, staged option
        replacement, same-ID author writes, exclusive profiles, revision loss,
        disconnect/resume and supported native input types.
      Delivered 2026-10-08: `lease.datalist` writes only the original input's
      `list`, using an opaque `prepareDatalist` admission with both directed
      grants and exact staged option identities. Revocation, competing claims,
      source/type/revision changes and disconnect expire authority; newer author
      writes, including the same ID, survive release. Source producers own option
      cleanup and refresh the coordinator on readiness changes. Four new browser
      cases and the existing provider/listbox suites pass (25 checks), alongside
      607 unit tests, type checking and lint (two existing warnings). Declarative
      profile wiring and publication transport remain the next item.

- [x] Add native datalist publication fixtures at the Rust/WASM/worker boundary
      and shared/production browser fixtures for profile transitions, native
      values, readiness, original field identity, disposal and unsupported controls.
      Record browser selection attempts and the outstanding physical-device/AT
      acceptance matrix without treating DOM association as selection evidence.

- [x] Integrate explicit profile selection into the declarative attachment,
      accepting text/search/tel/url/single-email/number under the native profile;
      keep custom listbox as the omitted default. Expose the native projection
      through WASM/worker session transport with source retention, attributed
      diagnostics, immutable publication lifetimes and control parsing; never
      export/reload the live view as a portable substitute. Reject email-multiple,
      require-selection/vocabulary proof, custom filter and surface/label APIs
      in the native profile. Fixture action: both-field browser and source/package
      cases for native roles, constraints, FormData/events, original focus,
      unsupported configuration recovery and profile changes. Scenarios for later
      design verification: browser edits do not become native-row proof; no
      custom popup, key interception or implicit type conversion is installed.
      Delivered 2026-10-08: explicit local `native-datalist` and default/explicit
      `listbox` select separate declaration branches. Native options render from
      immutable Rust publications through the original worker/fallback owner;
      strict empty controls, attributed diagnostics and native source retention
      survive that route, while row-proof APIs reject the native profile. Local
      attachment reuses host policy and exact instance-specific directed grants,
      fences late work, clears old options on source withdrawal and preserves
      native editing through resets, conflicts and profile changes. Both-field,
      concurrent-ID, six-type, worker/fallback and source-recovery fixtures pass:
      42 focused Rust, the WASM bridge, 36 browser and 608 unit checks, typecheck,
      lint (two existing warnings), the declarative architecture gate, clean
      package verification and the 53-component catalog build.

- [ ] Record supported-browser native datalist selection, accessibility and device
      evidence before claiming additional-profile acceptance. Fixture action:
      real numeric selection stores/submits `1` rather than One, email/URL
      sanitization, range/step mismatches, browser filtering, source withdrawal,
      physical IME and mobile keyboard/AT runs. Scenarios for later design
      verification: a connected list alone is not selection evidence; unsupported
      native UI preserves ordinary editing without a custom popup fallback.
      Progress 2026-10-08: recorded the automated matrix and reproducible native
      picker attempt in `packages/cem-components/docs/suggestions-acceptance.md`.
      Linux HeadlessChrome 148 Arrow Down/Enter stepped the numeric input to `0`;
      it did not establish selecting One as `1`. Native value/constraint and
      email/URL sanitization tests pass. This item remains open for actual
      headful native picker selection/filtering, physical IME/mobile keyboard and
      screen-reader evidence on supported browser/device combinations.

  - [x] Add a reproducible headful browser acceptance probe to the playground
        verifier for both field providers, source and isolated packages. Record
        picker requests, native event/value/FormData observations, accessibility
        snapshots, filtering attempts and withdrawal; keep actual selection and
        physical IME/AT acceptance separate from these observations.
        Delivered 2026-10-08: headed Chromium 148.0.7778.96 and Firefox 150.0.2
        passed eight field/type/package observations each on WSLg. Native numeric
        Arrow Down/Enter produced `0`, not demonstrated option selection; text
        remained empty. Both retained value/FormData through source withdrawal
        and preserved editing. The acceptance notes link raw reports and the
        repeatable command. WebKit launch is blocked by missing host libraries.
        Parent acceptance stays open for visible picker selection/filtering,
        supported-browser/device coverage, physical IME/mobile and spoken AT.

- [ ] Design field-owned label/submission conversion, multiline/email-multiple
      token completion and full surface-provider adoption only when requested.
      Carry explicit value/provenance, selection/history, native semantics and
      relationship grants into each separate contract. Scenarios for later design
      verification: no label rewrites a numeric value; token edits preserve
      caret/separators/composition, and one provider owns each adopted surface.

- [x] Assess an explicit field-owned undoable replacement adapter before
      promising an option commit is one native undo entry. Keep the first
      profile's [native history policy](cem-suggestions-interaction-design.md#undo-and-redo)
      and report supported-browser results; do not add a component-local history
      stack, deprecated editing command or replacement input as a workaround.
      Scenarios for later design verification: immediate undo after commit,
      later native typing then undo/redo, caret/selection restoration and
      submission/validity coherence; restored strings never infer source identity.
      Delivered 2026-10-08: assessed value assignment, range replacement, input
      notifications and EditContext against the original-editor contract; retain
      the current native-history limitation and expose no undoable adapter yet.
      Headed Chromium/Firefox passed 20 source/package scenarios with real typing,
      undo/redo and plain-input controls. Immediate undo of scripted replacements
      did not restore the pre-commit prefix; later history retained coherent value/FormData/validity
      and never restored source proof. Browser differences, raw traces and
      reproduction commands are in the suggestions acceptance notes. Future
      adapter admission/acceptance criteria are in the interaction design;
      physical/device and WebKit coverage remains open in the acceptance matrix.

  - [x] Add source/package native history probes for both field providers and
        plain-input controls. Use real browser typing/undo/redo, compare direct
        value and range replacement, and record selection, events, submission,
        validity and proof invalidation without imposing browser undo grouping.

- [x] Design non-key platform close-request support for manual suggestions
      surfaces before claiming Back/dismiss-gesture integration. Evaluate native
      CloseWatcher activation/grouping against parent dialogs/popovers and provide
      one close route without a competing keyboard watcher or local behavior.
      Scenarios for later design verification: first Escape closes only the child;
      a platform close request follows its declared scope; async opening does not
      silently group an independently dismissible child with its parent.
      Delivered 2026-10-08: adopted explicit host-coordinated admission in the
      popup design. Reserve one watcher during a deliberate opening activation,
      bind it to pending/visible session lifetime, and dismiss through the shared
      route. Keep the existing Escape/IME press owner; decline platform admission
      when independent parent grouping cannot be established. Implementation and
      device support remain gated by the following actions.

  - [x] Implement shared platform-close admission and watcher lifecycle for the
        manual-listbox session. Fixture action: pending cancellation before
        publication, generation fencing, source/parent/authority loss, disposal,
        reentrant close and fresh reopen. Scenarios for later design verification:
        no stale completion revives a canceled intent; no duplicate watcher or
        close notification; native datalist acquires no watcher.
        Delivered 2026-10-08: shared host coordinator reserves on a fresh trusted
        arrow opening, tracks event consumption and explicit ancestor authority,
        and releases on query replacement, revocation, DOM loss and teardown.
        Suggestions own the reservation before async row readiness; canceled
        intent cannot reopen, including synchronous admission revocation.
  - [x] Integrate admitted watchers with the existing editor Escape/IME route and
        test native grouping with modal dialogs and auto/hint popovers. Fixture
        action: separately activated child, same-activation/unmanaged-parent
        refusal, held Escape and unsupported API. Scenarios for later design
        verification: first Escape closes only the child; late async readiness
        creates no new group; direct requestClose tests do not prove UA grouping.
        Delivered 2026-10-08: both production field providers preserve existing
        Escape/IME/held-press routing. Browser tests cover actual UA close
        processing under modal/auto/hint parents, separate activations, denied
        or consumed interactions, unmanaged ancestors and unsupported APIs.
        All 29 focused controller/manual-listbox/native-surface/browser tests pass.
  - [ ] Record supported-device Back/dismiss-gesture evidence before advertising
        platform close support. Scenarios for later design verification: one
        accepted platform request dismisses the admitted child, preserves text
        and parent state, and releases the watcher so normal navigation resumes.
        Progress 2026-10-08: acceptance notes distinguish Chromium protocol-driven
        close/grouping evidence from physical Back/gesture runs and specify a
        host-enabled device procedure. No physical mobile/device session is
        available here; this item stays open. The ordinary local gallery does
        not supply platform-close authority and is not a substitute test host.

## Restore the components declarative verification gate

- [x] Move the theme-switch runtime/scope/action-declaration setup to the shared
      Storybook test/setup boundary, removing forbidden cross-component/runtime
      imports from its colocated story; rerun verify-declarative and its native
      lifecycle/theme fixtures. Do not loosen the gate to admit component behavior.
      Delivered 2026-10-08: isolated runtime/scope/declaration preparation and
      source dependency loading use the shared Storybook setup boundary. The
      colocated story keeps only permitted imports; no gate relaxation or
      component behavior was added. Declarative verification and all 27
      theme-switch/suggestions lifecycle plays pass.
      Evidence 2026-10-07: the current tree and unchanged HEAD both reject
      `cem-elements/src/index.js` and `cem-action.xhtml?raw` imports in
      `cem-theme-switch.stories.ts`. Scenarios for later design verification:
      theme rerenders preserve declaration/instance/group identity and action
      routing while the per-component authoring rule remains enforced.

## Preserve runtime package verification invalidation

- [x] Fix cem-elements verify-package cache invalidation after runtime build
      outputs change. Fixture action: add or change a packaged runtime module,
      then prove the package check reruns against the new dist contents.
      Evidence 2026-10-07: after the geometry build added three dist files, the
      normal target replayed a cached 183-file result; cache-bypassed verification
      ran against the new 186-file package and passed. Scenarios for later design
      verification: a changed emitted runtime file or required packaged notice
      invalidates verification; an unchanged build may reuse its verified result.
      Reconfirmed 2026-10-08: the normal target rebuilt the runtime but replayed
      the old 183-file verification result; cache-bypassed verification checked
      the current 213-file package and passed.
      Delivered 2026-10-08: changed the dependency-output filter from `dist/**`
      to `**/*`, including transitive WASM comparison inputs, pinned source notice,
      metadata/README/license files and Node/npm versions. Six normal Nx runs
      proved added and changed compiled modules invalidate verification, unchanged
      output hits cache, and fixture removal restores the original hash/output.
      Counts were 219 baseline, 222 with the temporary module, and 219 restored;
      both module versions had different verification hashes. The package README
      records reproduction and links the machine-readable run evidence.

## Respect action states in cem-icon-button

Completed prerequisites and verification: [archived checklist](archive/todo-snapshot-2026-10-07.md#respect-action-states-in-cem-icon-button).

- [x] Run browser state/paint checks and full icon-button source/package gallery
      verification. Delivered 2026-10-08: all 20 icon/icon-button browser stories
      pass, including five-theme state precedence, held pointer/Space, geometry
      and native form behavior. The nested icon reflects its declared `normal`
      default; disabled controls use the adopted disabled action tokens. Corrected
      the stale fixture expectations without changing component CSS or token
      ownership. Both property pages, both galleries and release-bundle checks
      pass from source and isolated package archives through `--icons-only`.
      The unfiltered playground gate still fails the separately tracked menu-item
      attribute inventory; this result does not claim that full gate passes.

## Compose cem-icon-button from cem-icon

Completed prerequisites and verification: [archived checklist](archive/todo-snapshot-2026-10-07.md#compose-cem-icon-button-from-cem-icon).

- [x] Fixture: add an `--icons-only` playground verification mode reusing the
      existing icon/icon-button property, gallery and release-bundle checks for
      both source and installed packages. Scenarios for later design verification:
      unrelated component failures must not prevent focused evidence, and the
      unfiltered gate must retain all its checks.
      Delivered 2026-10-08: focused verification passes in both locations. The
      icon property preview uses live `label` updates and authored rich-content
      examples, respecting captured payload ownership. The module-image example
      uses one explicit `str:concat` expression. Fixture expectations now match
      the gallery's regrouped examples and authored colors; rich-content checks
      find the meaningful Details link, and bundle checks distinguish the sample
      and stylesheet from shared gallery navigation.
- [x] Rerun focused icon/icon-button browser stories and source/installed-package
      playground and bundle checks. Delivered 2026-10-08: 20 stories pass;
      `verify-playgrounds --args=--icons-only` passes both property playgrounds,
      both complete galleries, and bundle loading/metadata/style deduplication in
      source and isolated npm archives. Declarative verification, builds, package
      verification (151 files) and lint (zero errors, 95 existing warnings) pass.
      Command: `yarn nx run cem-elements:test -- packages/cem-components/src/components/cem-icon-button/cem-icon-button.stories.ts packages/cem-components/src/components/cem-icon/cem-icon.stories.ts`.

## Convert five icon gallery demos to CEM-ML

Completed prerequisites and verification: [archived checklist](archive/todo-snapshot-2026-10-07.md#convert-five-icon-gallery-demos-to-cem-ml).

- [x] Formatter follow-up: diagnose and fix repeated tabular CEM formatting
      adding blank lines to text-bearing nodes with a minimal native fixture.
      Delivered 2026-10-08: text-only layout ignores formatting whitespace;
      mixed-content text is fenced at inserted layout boundaries, and leading
      significant whitespace remains inside text fences. Native fixtures assert
      stable second/third output and unchanged inline, nested, multiline and
      fenced text values. Writer tests and all six CEM target conversion tests
      pass; the rebuilt CLI produces identical first/second tabular output.
      Scenarios for later design verification: formatting layout must not become
      text on reparse; significant text whitespace must not be trimmed.

## Remove fixture-only gallery IDs

Completed prerequisites and verification: [archived checklist](archive/todo-snapshot-2026-10-07.md#remove-fixture-only-gallery-ids).

- [x] Run source/installed-package browser checks. Delivered 2026-10-08:
      all source checks completed in the unfiltered verifier; the complete
      installed leg passes with `--installed-only`. Navigation, galleries,
      property controls, native input forwarding, suggestions, bundle loading and
      pending theme gradients are covered. The explicit package retry keeps
      every installed check and the default combined gate remains unchanged.

## Organize cem-icon gallery examples

Completed prerequisites and verification: [archived checklist](archive/todo-snapshot-2026-10-07.md#organize-cem-icon-gallery-examples).

- [x] Run browser gallery verification. Source and installed icon galleries
      pass the focused 2026-10-08 gate recorded above.

## Fix duplicate Font Awesome glyphs in cem-icon

Completed prerequisites and verification: [archived checklist](archive/todo-snapshot-2026-10-07.md#fix-duplicate-font-awesome-glyphs-in-cem-icon).

- [x] Run icon stories and source/installed-package galleries. The 2026-10-08
      focused run above passes 20 stories and both package locations.

## Align cem-icon with the label principle

Completed prerequisites and verification: [archived checklist](archive/todo-snapshot-2026-10-07.md#align-cem-icon-with-the-label-principle).

- [x] Rerun icon stories after removing `name`: covered by the 20 passing
      icon/icon-button stories recorded above.

- [x] Complete source/installed-package playground verification, then commit and push.
      Delivered 2026-10-08: browser/Git access is available; source and isolated
      npm package checks pass as recorded above. Refreshed playground builds pass.

- [x] Align `cem-icon-button` with the shared label principle. Its declaration
      forwards visible fallback `label` to `cem-icon` and separate `aria-label`
      to the glyph/control; the focused stories and package galleries above pass.

## Native verification follow-ups

- [x] Reconcile two CLI query source-contract tests with the current dispatch
      boundary and verify the legacy dropdown/menu pages' external canonical
      declarations instead of requiring removed local templates.
      Delivered 2026-10-08: assertions cover both shared-runner entry points,
      retaining the execution boundary while allowing explicit result exports.
      The manifest verifies canonical declaration links and nonempty native
      templates. Full CEM-ML library verification: 2,082 passed, two ignored.
      Scenarios for later design verification: query execution still uses the
      common typed runner and host policy; the legacy manifest points to a real
      retained template.

## Browser failures observed during placement verification

- [x] Fixture: support installed-only playground verification without weakening
      the default source-plus-installed gate. Wait for the action inventory
      section before comparing its full attribute set. The first full installed
      run observed an empty set; isolated source/installed action verification
      passed. Preserve readiness diagnostics if it recurs; do not claim a runtime
      cause from this single observation. Scenarios for later design verification:
      explicit package retries run all installed checks and report their scope.
      Delivered 2026-10-08: the installed-only run passes every package check;
      isolated action checks also pass in both source and installed locations.

- [x] Fixture: complete native input attribute inventories and live forwarding
      examples in the field/text-field galleries, plus a suggestions profile
      example. Scope the suggestions property interaction to its named Letter
      combobox so the numeric datalist example does not make it ambiguous.
      The full gate first reached `cem-field` and reported missing `min`.
      Scenarios for later design verification: min/max/step and text constraints
      reach native controls; datalist IDs serve real relationships; both source
      and installed galleries use the same admitted public attributes.
      Delivered 2026-10-08: all source/installed gallery inventories and native
      constraint/datalist assertions pass; suggestions pass in both locations.

- [x] Add a native template diagnostic regression for adjacent expression text
      in a quoted attribute (`@image="{$path}{$image}"`), and preserve
      source-attributed diagnostics through the browser.
      Delivered 2026-10-08: AC-T-7 already admits multiple AVT spans. Whole-value
      classification now uses balanced expression boundaries; quoted composite
      spans concatenate, escaped braces remain literal, and native unquoted
      slots retain their own syntax. Invalid and unterminated expressions report
      the owning attribute frame instead of silently falling back to literal text.
      Fixture scope: native adjacent/nested/escaped spans and invalid/unterminated
      attribute frames; a focused browser WASM story checks values and diagnostic
      attribution at the rendering boundary. All 123 CEM-QL library tests pass
      (four ignored), as do 82 template integration tests and the browser story.
      Scenarios for later design verification: malformed expression input must
      not silently remove an attribute; supported concatenation retains its value.

- [x] Reconcile the `cem-menu-item` property/gallery attribute inventory with
      its canonical declaration, then rerun the full `verify-playgrounds` gate.
      Delivered 2026-10-08: the inventory and live navigation/name example now
      include `aria-label` and `href`. Source and installed control checks pass.
      Scenarios for later design verification: every admitted public attribute
      has a documented live example in source and installed pages; exercise
      live link/button switching and accessible-name updates. Capture index
      navigation readiness on failure; focused icon
      verification does not count as a pass of this full gate.

- [x] Fixture: diagnose the workflow gallery's missing `cem-text-field input`
      after its sample readiness check. Preserve declaration/load diagnostics
      before changing readiness or component behavior.
      Delivered 2026-10-08: timeout capture showed an unregistered text-field tag
      and no declaration errors. The page omitted its canonical declaration;
      adding that declaration fixes the focused workflow browser story. Failure
      output now retains registration and declaration diagnostics; waits and
      component behavior are unchanged.
      Scenarios for later design verification: a ready demo marker must not hide
      incomplete nested declarations; native input readiness must remain distinct
      from initial sample publication.

- [x] Fixture: capture the form-isolation story's rendered-control readiness under
      aggregate browser load before replacing its single-frame wait. FormData
      lacked `visible=ok` once in the 417-case run; both focused current/baseline
      runs pass all nine isolation cases. Preserve inert island assertions.
      Delivered 2026-10-08: the traced aggregate run reproduces zero live
      controls while rendering is pending at the single-frame assertion;
      rendering settles 10 ms later without diagnostics. Await the instance's
      owning runtime settlement and assert exactly one submitted live value.
      All 19 focused isolation/HTTP/location/cell-override stories pass.
      Normal aggregate verification passes 484/486, including this form case;
      module-URL settlement and suggestions pointer failures remain below.
      Counts-only lifecycle tracing remains opt-in. Evidence:
      [form lifecycle capture](browser-stabilization-review.tmp.md#form-submission-lifecycle-capture-and-recurrence-follow-up-2026-10-08).
      Scenarios for later design verification: the live control submits once after
      actual render completion; captured controls stay disconnected and absent
      from FormData. Do not attribute the recurrence to references without evidence.

- [x] Fixture: trace the aggregate runtime slot failures before changing waits:
      `Legacy Named Slot Payload Parity` misses h3 and `Slot Projection Render
      Loop` misses div.card in the 2026-10-08 traced run. Capture the owning
      runtime's render state at the original assertion; preserve slot identity,
      ordering and projection assertions.
      Delivered 2026-10-08: both failures recur with pending instance renders
      and no diagnostics at their single-frame checks. Both fixtures now await
      every instance through their owning runtime. All 90 focused runtime,
      module-URL and suggestions stories pass; opt-in lifecycle checkpoints stay.
      Final normal aggregate verification passes all 487 stories in 101 files.
      Scenarios for later design verification: asynchronous publication must
      settle before lookup, while projection updates retain their original nodes.

- [x] Fixture: attribute `Module URL Settlement / Siblings Commit Together`
      observing zero of four resolver entries under aggregate load. Capture
      declaration/compile/render state before changing its polling deadline;
      preserve the controlled sibling-release and atomic publication assertions.
      The normal aggregate rerun also fails this case (484/486 overall).
      Follow-up: the 16-test control, 90-test traced control and two further
      aggregate traces do not reproduce it. Added opt-in resolver-count and
      owning-render diagnostics on failure; discovery and sibling-release
      deadlines/assertions remain unchanged. Resume correction on captured
      recurrence rather than inferring a cause from passing controls.
      The next normal aggregate reproduces zero resolvers again without tracing;
      failure capture now always includes resolver count, settlement and codes.
      Final aggregate verification passes 487/487. The original discovery wait
      remains unchanged pending a failure with the new capture.
      Delivered 2026-10-08: normal-run capture shows zero resolvers, pending
      render settlement and no diagnostics at the one-second deadline; all four
      resolvers appear 85 ms later. Await the controlled resolver-admission
      signal, then assert exactly four entries, within the existing story limit.
      All sibling-release, stale/disconnected batch and failure assertions stay.
      Final normal aggregate verification passes all 487 stories in 101 files.
      Scenarios for later design verification: startup latency and stale sibling
      completion must remain distinguishable.

- [ ] Fixture: investigate aggregate-only data-table/set-URL story deadlines
      and the two native suggestions pointer failures recorded on 2026-10-08.
      Establish isolated controls, then capture the failing step under load.
      Normal aggregate follow-up passes the data-table/set-URL and pointer-tap
      cases; pan cancellation/outside focus still fails. Prioritize that
      reproduced pointer-state failure; passing repeats do not attribute the
      other three deadlines.
      Do not increase global deadlines or relax focus/pointer expectations.
      Fixture scope: split the serial native pan/cancel/outside-focus matrix
      into independently scheduled cem-field and cem-text-field stories, each
      retaining every gesture assertion and its own unchanged 30-second limit.
      Pan-opening correction 2026-10-08: the original one-second check
      observes retained focus/nodes but no current row publication; failure-only
      observation captures current rows and an open popup 6.23 seconds later.
      The fixture now requires both conditions within a ten-second interaction
      budget, retaining the 30-second story limit and all gesture assertions.
      All 90 focused stories pass. Historical tap/data-table/set-URL deadlines
      still need separate attribution.
      After splitting the providers, the final normal aggregate passes 487/487;
      both complete gesture sequences pass within their separate original limits.
      Scenarios for later design verification: all authored cases finish within
      the story budget; native taps, pan cancellation and outside focus keep
      their distinct contracts.

- [x] Fixture: capture the newly observed retained stylesheet
      `Payload Css Disconnect And Failure` zero-versus-one assertion and
      isolation `Declaration Element Renders No Visible Content` and
      `Data Island Content Stays Out Of The Accessibility Tree` failures
      before changing readiness. Both appear during the 2026-10-08 traced
      aggregate investigation; retain exact style ownership and inert declaration
      assertions. Evidence:
      [slot and suggestions follow-up](browser-stabilization-review.tmp.md#slot-and-suggestions-readiness-follow-up-2026-10-08).
      Scenarios for later design verification: style publication/disposal and
      declaration inertness remain distinct from produced-instance readiness.
      Delivered 2026-10-08: both isolation cases reproduce pending rendering
      with no diagnostics before their assertions; await their owning runtime.
      Payload stylesheet readiness reproduces a first-read deadline, with the
      controlled reader called 602 ms later. Both readiness/hydration and
      disconnect/failure fixtures now await their reader's first-call signal,
      preserving exact counts and all publication, cancellation and style checks.
      All 28 focused stories pass, including the bundle-source-context file
      that had a transient setup-module fetch failure during investigation.
      Final normal aggregate verification passes all 487 stories in 101 files.
      Historical post-settlement missing-style attribution remains separate.

## Immediate: Legacy Demo Case Coverage

Completed prerequisites and verification: [archived checklist](archive/todo-snapshot-2026-10-07.md#immediate-legacy-demo-case-coverage).

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
      Follow-up 2026-10-08: after refreshing stale packaged WASM, all 32
      stock probes pass (eight concurrent pages across four batches, both helper
      and real-component paths). Warning publication takes 5.65–9.35 seconds
      during an overlapping traced aggregate suite. No historical stock failure
      recurs; retain its diagnostic target and existing deadline.

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
        Audit reconciliation 2026-10-08: the later thirteen-file source audit
        already completed the static inventory and corrected the reproduced
        string-demo race; see
        [current boundary inventory](browser-stabilization-review.tmp.md#reconciliation-of-the-remaining-readiness-inventory).
        Remaining work is traced HTTP/location recurrence attribution, not
        another wholesale migration of frame waits. Preserve exact per-card
        output and resource predicates when a supported correction is made.

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

### Native-surface delivery follow-ups

- [ ] Verify both pinned ARIA profiles in the documented browser/assistive-
      technology matrix. Record accessibility relation exposure, ordered/repeated
      details targets and error-message announcements with actual versioned
      environments; keep untested combinations explicit and the draft opt-in.
      Scenarios for later design verification: single-relation accessibility APIs
      may expose only the first target; exported DOM sequences must stay intact.

- [ ] Implement shared surface body preparation and `materialize` retain/dispose
      lifecycle before using those conveniences in migrated task declarations.
      Add fixtures first for native owner stability, preparation cancellation,
      stale completion, retained drafts and disposal after closing/exit cleanup.
      Scenarios for later design verification: invisible preparation does not
      expand launchers; default-open does not replay; disposal does not lose
      separately retained application draft state.

- [ ] Implement shared nested menu-to-task focus relay and generated/slotted
      launcher/close conveniences before claiming full interaction acceptance.
      Add native/browser fixtures first for context capture before menu closure,
      one task entry, stable launcher return, outside destination preservation
      and canceled preparation. Reuse eager `native-surface` rather than adding
      component/application-local behavior.
      Scenarios for later design verification: invocation geometry survives
      ancestor hiding without transferring context authority or replaying commands.

- [ ] Inventory and migrate legacy dialog/tooltip component APIs to XHTML/CEM-ML
      declarations using shared `native-surface`; keep frozen behavior modules
      unchanged until each owner is migrated. Add colocated browser plays and
      linked property playground/gallery cases before each cutover, including
      names, forms, modal/nonmodal visibility, interest, reference roles and cleanup.
      Scenarios for later design verification: literal local-name conveniences
      remain scoped, while cross-producer targets require fresh placement grants.

### Remaining declarative UI migration

Completed prerequisites and verification: [archived checklist](archive/todo-snapshot-2026-10-07.md#remaining-declarative-ui-migration).

- [ ] Migrate every legacy `cem-components` member into its own
      `src/components/<cem-tag>/<cem-tag>.xhtml` folder with embedded,
      once-per-declaration scope-contract CEM-token `<style>` and colocated
      CSF Next `<cem-tag>.stories.ts` `play` tests, moving missing reusable behavior into
      `cem-elements`, until both migration targets are zero.

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

- [ ] Investigate runtime suite instability: the action-cutover full run passed
      311/312 stories; `Payload Css Readiness And Hydration` expected one style
      but observed zero. Its complete 12-story file passed on isolated rerun.
      Preserve the readiness assertion and reproduce before changing behavior.

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


## Finish retained working-tree edits

- [x] Reconcile the uncommitted dropdown gallery formatting/overlay experiment
      with the adopted five-separator and reserved-clearance examples. Restore
      bounded image geometry, rebuild pages, and run existing menu/dropdown
      verification in source, generated and installed packages.
      Scenarios for later design verification: initially open panels remain
      within their demo region; exactly five real separators retain hit-testing,
      hover and reveal-on-close behavior across themes and narrow layouts.
- [x] Remove the obsolete `.reference-edit.py` migration helper after verifying
      its tokenizer, projection schema, inspection and design edits are already
      incorporated; do not reapply its superseded design text.
      Completed 2026-10-08: helper effects are present in the tokenizer test,
      projection schema/inspection and adopted reference design; removed the
      untracked helper without replaying it. The gallery experiment diverged
      from the completed overlay requirements in the
      [archived checklist](archive/todo-snapshot-2026-10-07.md#dropdown-sibling-filler-demonstration).
      Preserved readable formatting and restored the five fillers, image bounds
      and overlay clearance. `verify-menu-dropdown` and its build prerequisites
      pass across source/generated/installed pages, five themes, narrow layouts,
      overlay hit-testing, scrolling and forced colors.
