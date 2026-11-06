# Original payload and canonical native island preparation — 2026-11-05

`cem_ql::ssr_island::prepare_inline_island` now consumes the existing prepared
inline DCE, a caller-supplied instance identity and the original operation
control. It captures ordinary source children or the contents of one sole
meaningful unmarked HTML payload template, then constructs the canonical inert
marked template and `0.2.0` namespace-aware context root. All nine domain parts
exist once in schema order. The prepared object retains the source page,
declaration/template owner, original payload nodes, envelope handle, native
context/template and shared input bindings.

The source page remains unchanged and the SSR envelope remains
`Partial(Incomplete)` with zero render passes and no hydration readiness.
Template insertion into a derived page AST, declaration evaluation, rendering,
settling, completion/adoption proof, command lowering and publication are later
work. This increment prepares an island; it does not emit an SSR page or add a
wire codec. Initial native hydration fields are `version`, `instanceId` and
`outputTarget=light-dom`, not a completed render/adoption record.

`instance_island` is extracted from the existing compiled component receiver.
It owns the same canonical namespaces, topology/domain admission, attribute and
DOMStringMap conversion, slot indexing, sensitive-input/reserved-alias rules
and input ceilings. The old `application_runtime::island` path remains a reexport
and its explicit CEMV byte ingress remains intact. Native SSR uses direct retained
input, without serializing/reloading the island or enabling the experimental
component runtime. The canonical optional HTML envelope marker now admits exactly
`data-cem-explicit-payload=true`; arbitrary marker values remain invalid.

Generated structure uses the existing common constructed CEM value owner and its
explicit retained source derivation anchor. It does not claim physical authored
line numbers. Original payload occurrences are `CemValueNode::Reference` values;
their targets retain the exact original CEM tree/HTML owner, IDs, native query
identity, source parents, order and maps. The envelope handle remains available
for the later page applier. No payload subtree is copied, external syntax parsed
in the coordinator, JSON DOM introduced, or parallel island/slot model created.

Original node provenance follows retained source-boundary frames before falling
back to document metadata. A deployed `document_uri` therefore does not rename
an authored source. Document/base accessors and relative URL resolution keep
their separate deployed meanings. Transformed origins remain intact, and
unmapped input fails native island admission rather than acquiring invented
source frames. The shared constructed-output API remains opt-in: ordinary output
still has no derivation anchor or fabricated provenance.

Host attribute and dataset entries retain encounter order and literal values;
the existing internal render-scope attribute is omitted. Slot arrays preserve
direct root encounter order, without flattening nested descendants into named
slots or changing source parents. Empty slices, resources, form, validation and
event sections initialize the passive state. Native bindings reuse the existing
semantic domain views and input frame, preserving literal strings and safe
`constructor`/`__proto__` names without a JavaScript object handoff.

Meaningful-child classification matches browser `String.trim`, including BOM and
NBSP but excluding NEL. Ordinary comments make an unmarked envelope ambiguous;
render-boundary comments do not. Mixed/multiple envelopes fail with
`cem-element.instance_payload_mixed`. A marked island selects resume refusal
before capture, leaving derived siblings untouched. This first profile accepts
plain HTML envelopes, optionally `type=text/html`. CEM source payloads, foreign
payload domains, extra envelope adapters/headers and instance CSS remain explicit
refusals until shared native admission exists for them. Scripts/resources retain
the existing strict import policy, and sensitive controls/attributes remain
refused by the shared domain/payload rules.

The retained native context closure is bounded to 16,384 expanded value visits,
depth 64 and 1 MiB of accounted lexical/map/provenance/node storage. Reference
targets count toward expanded work, while original source-parent links are not
traversed into the surrounding page. Caller memory and logical-stack caps can
only lower those bounds. The same operation is checked during capture, closure
validation and domain binding, with cancellation/completed-scope codes retained.
No fresh timer replaces the caller's deadline. These checks are not total-heap
accounting for already-retained source owners or a complete settling coordinator.

Twelve native cases failed before implementation: the missing capture path and
the existing source/document provenance conflation. The expanded suite adds
transformed/unmapped origin, caller-limit and shared browser-vector coverage.
A separate failing regression demonstrated that caller memory/stack ceilings
must reduce the native bounds before the implementation was corrected.

Focused checks use `cem_ql:test --test=ssr_island` and
`cem_ql:test --lib --args=instance_island`. The explicit
`cem_ql:test:ssr-island:stripped` target runs the capture, earlier preparation and
shared island suites with actual `--no-default-features`.
`cem_ql:test:ssr-island:component` checks the isolated compiled receiver's reexport
and input behavior with `--no-default-features --features component-runtime`.
`cem_ql:test:ssr-island:browser` compares ten passive DOM observations with the
same native test-control fixtures, with authored JavaScript disabled and external
requests refused. It does not execute SSR or verify full browser hydration.

Exact results and code/artifact hashes are retained in the
[verification receipt](ssr-island-preparation-2026-11-05.json). Verification uses
`CARGO_INCREMENTAL=0`, no development/test debug symbols, bounded Cargo jobs and
`--skip-nx-cache` to avoid the earlier disk exhaustion. Default features and the
production WASM size budget are unchanged. No full-workspace gate is claimed.
A superseded WASM optimizer was stopped after a source-only constant cleanup;
the final production build is rerun from the final source before verification.

The next step uses the prepared native bindings and retained template to produce
and apply the shared render plan outside the island, preserving namespaces,
source owners and render boundaries. Additional payload/CSS/resource admission
and bounded settling follow before a native completion proof may permit browser
adoption. There is no public Site/UI, theme/CSS, npm command ABI or deployment
change. The default core WASM is rebuilt for verification; npm packages and the
frozen presentation are not repackaged.

- [x] Capture: **15/15** in default and actual stripped profiles.
- [x] Shared native island **13/13** and earlier preparation **13/13** in both
      profiles; isolated component receiver **17/17** with actual component features.
- [x] Common import **24/24**, native import provenance **7/7**, query provenance
      **2/2**, template source URI **2/2** and renderer **28/28**.
- [x] Chromium 148 passive capture observations **10/10**, without authored scripts
      or external requests; native fixture observations match.
- [x] Focused formatting/JavaScript lint and native Clippy pass, with existing
      warnings elsewhere and none in the touched island/output-view/tree modules.
- [x] Default core WASM **24,179,963 bytes**, below **26,214,400 bytes**; public
      descriptors match the prior build (28 imports, 171 exports). Existing
      template-cache and retained declaration/instance CSS consumer checks pass.
- [x] Frozen presentation remains **17,321,262 bytes**, SHA-256
      `bbcb37da0a36491e5c021a5cf0c67304cc41f001a98247fc5c101ed6c45a9864`.
