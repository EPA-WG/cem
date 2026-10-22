## Project builds and artifact previews

- [x] Add native rooted folder imports and mixed static-asset publication before the site example. Fixture: verify legacy unrooted bindings, native root authoring, root/nested glob parity across filesystem and manifest providers, Windows folder addresses, retained source identities and folder bindings through native conversion, exact bytes/media types, deterministic paths, missing/outside-root/collision diagnostics and bounded traversal.
- [x] Fixture: run a rooted native folder/static build through Studio's ordinary saved-project command and immutable artifact preview; verify nested CSS/images, binary bytes, unchanged source selection and failed-run retention without adding application UI JavaScript.
- [x] Fixture: keep the Node filesystem discovery adapter consistent with native zero-directory recursive matching and bounded traversal; verify root/nested matches and preserved file URI roots (including encoded directory names), an ordinary rooted graph through the Node service, and rejection before deep traversal.

- [x] Record the project-level build/preview design for single CEM-ML documents, transformation graphs and folder-based sites; keep editor selection independent of the preview configuration/run/artifact.
- [x] Model project build configurations, admitted run snapshots and bounded artifact manifests using native output identities and provenance. Fixture: cover a one-document HTML build and Supplier HTML/JSON/YAML outputs, including exact selected-artifact copy/download.
- [x] Fixture: verify shared multi-output reads, aggregate/count limits, duplicate/missing publications and exact digests before admitting an artifact set; retain failed-output diagnostics, literal YAML preview and binary download policy.
- [x] Compose an independent project preview region with build configuration and artifact selection, preserving existing file/folder listings and source editor selection. Fixture: verify URL selection, saved-revision Build, stale/failed/cancelled attempts and last-successful preview ownership.
- [x] Fixture: verify shared three-region workspace geometry, retained source/preview owners, inherited spacing/themes and forced colors; exercise sample and saved-project builds without mutating sample storage or user drafts.
- [x] Fixture and shared runtime repair: preserve exact trailing newlines through native plaintext editing and subsequent Studio build/theme updates; do not serialize the browser's terminal caret placeholder as source.
- [x] Extend shared artifact-tree preview loading for relative HTML navigation, CSS/images and nested paths within one immutable run; raise required cem-elements JavaScript before implementation. Fixture: verify run isolation, missing assets, disposal and atomic output-set replacement.
- [x] Fixture: verify native retained-CSS resource remapping (imports/conditions, escaped URLs, image-set, inline declarations, UTF-8 and unsupported grammar) before wiring the shared browser artifact loader.
- [x] Fixture: verify artifact-preview component lifecycle, script/network isolation, nested asset bases, fragment/link events, run replacement/revocation and Studio artifact URL/source-selection ownership.
- [x] Fixture: verify relative CSS/images and nested output selection in source and isolated-package galleries, plus the deployed Studio artifact frame after a native project build.
- [x] Fixture: verify shared native YAML scanner-to-byte source spans for Unicode scalars/collections and sparse coordinate lookup for long values before embedding metadata; retain authored source slices through the common import.
- [x] Fixture: verify opt-in native YAML front matter, typed metadata and formatter round trips, unchanged ordinary Markdown, UTF-8/CRLF coordinates, invalid/unclosed/oversized/deep/duplicate metadata and lifecycle diagnostics.
- [x] Fixture: verify native CLI Markdown sidecars retain the conversion frame, exact original source boundaries and distinct file identities.
- [x] Fixture: preserve native sourced atomic values during interpolation, alongside layout expression frames, and leave synthetic values without invented origins.
- [x] Fixture: build two rooted Markdown sources through native HTML recovery and CEMT layout/partial composition with explicit site/navigation inputs; verify distinct original source URIs, source ranges, indexed output-span lookup, unchanged token origins despite coincident byte lengths, and absence of invented lineage.
- [x] Fixture: verify the same metadata/layout/provenance build through Studio's ordinary project service, including failed metadata validation retaining the previous successful preview.
- [x] Define supported native site metadata/front-matter and layout/navigation inputs before the complete site example. Fixture: preserve content/partial source frames and verify distinct Markdown source-URI provenance through folder imports, conversion and layout composition.
- [x] Add the ordinary Markdown + CEM-ML site example already planned in [the roadmap](../roadmap.md#studio-example-markdown--cem-ml-site-project), using native folder-tree/metadata/render/export semantics and the same project workspace: home, section, two articles, shared layouts/partials, data, assets/static, configurations and expected outputs.
- [x] Fixture: run the authored site project through the native manifest and filesystem CLI paths; compare four HTML expectations and exact static bytes, check deterministic output paths/content/partial provenance, article/shared-layout edits, invalid metadata and missing dependencies before browser integration.
- [x] Fixture: open the catalog site in the existing Studio workspace, build and navigate nested links/CSS/images without changing source selection, edit/save/fail/repair, export/reimport independent copies and reopen/build offline through the deployed navigation gate.
- [x] Fixture and delivery repair: admit the shared artifact loader's owned image/stylesheet blob URLs in Studio's parent CSP; verify actual nested asset rendering under emitted headers while keeping script/worker blob execution and inline style attributes denied.
- [x] Fixture: verify article/layout edits, intentional validation failure/repair, static assets, multiple outputs and provenance in the site project, including portable independent copies and offline reopen/runs.

- [x] Let a named build configuration declare its entry artifact explicitly; persist it in portable projects, keep URL artifact selection authoritative, and report a missing entry without guessing among nested index documents.
- [x] Fixture: verify optional entry-artifact CEM/JSON normalization and deterministic round trips, Unicode/space paths, rejection of noncanonical paths and incompatible configuration kinds, and schema/native diagnostic parity before browser wiring.
- [x] Fixture: verify declarative saved-project entry editing, revision/undo/import/copy preservation, default/URL/manual precedence, absent outputs, legacy defaults, configuration changes and stale in-flight admission; run the Markdown site's configured entry through deployed offline reopening without an artifact URL.
- [x] Add bounded pinned successful build snapshots: preserve exact bytes, input/configuration versions, digests and provenance for offline reopening, while keeping current builds and source selection independent.
- [x] Fixture: verify atomic bounded pin persistence, exact bytes/digests and provenance, corrupt/incomplete rejection, project scoping and deletion without changing source revisions or drafts.
- [x] Fixture: verify declarative pin/current selection, retained current builds, relative assets and exact transfers, late-operation disposal and aborted repository opens/reads, expanded-toolbar containment and actual preview clicks, and offline reopening of a pinned Markdown site without rebuilding.
- [x] Compare pinned/current builds by artifact paths, additions/removals, exact digests and source/configuration versions, with native provenance available for changed outputs.
- [x] Fixture: compare bounded admitted run manifests by path and stable input identity, covering additions/removals, binary digests, media/provenance-only changes, reordered control keys, configuration versions and project isolation.
- [x] Fixture: compare the edited Markdown site against a verified pin through declarative controls, preserve current/source/draft ownership, show native source ranges and verify offline comparison and clearing/deletion.
- [x] Specify and implement conservative native source/template dependency identities and failing-first invalidation fixtures before enabling reuse, following [the caching roadmap](../roadmap.md#312-reuse-unchanged-render-content-and-stylesheets); include imported closures, effective inputs, engine/policy and URL/scope context, keeping opaque dependencies uncacheable.
- [x] Fixture: prove native dependency identity determinism, invalidation of every declared axis, transitive imports, URI/type/schema/provenance context, explicit input/freshness revisions, opaque/pending/released rejection, bounded admission and runtime/owner generation guards before enabling cache hits.
- [x] Fixture: compile a real native template closure through the identity entrypoint, verify imported edits/options/policy invalidation and render/source-frame parity, and withhold identities for invalid/incomplete closures without changing the existing compiler or binary transport.
- [x] Fixture repair: supply the existing source_uri option in the older template-render test literal so the broader native gate compiles, preserving all behavioral assertions.
- [x] Extend existing shared native parsed/compiled artifact caches with bounded leases/eviction and runtime-replacement invalidation; prove cached/uncached semantic and provenance parity before enabling hits, without a Studio-specific cache.
- [x] Fixture: verify native content-cache byte/count limits, LRU and sidecar ownership, immutable deduplicated leases, pinned-capacity backpressure, atomic failed admission/replacement, corruption rejection, compacted input buffers and runtime/drop invalidation, fenced late admission/lookups and cross-thread retired lease release with bytes still charged.
- [x] Fixture: reload real compiled CEM templates through bounded native cache leases in dev/prod modes; compare cached/uncached rendering, diagnostics, source/output ranges and policy/ownership behavior before enabling any browser fast path.
- [x] Fixture: exercise checked native compilation hit/miss counts, imported byte/hash and contract tampering, fresh resolver/URI/options/mode invalidation, bounded eviction, dev/prod provenance parity, and cancelled/superseded/replaced-owner publication; exercise the real WASM closure path and shared processing handle isolation.
- [x] Wire checked dependency identities, fresh resolver/provider context and publication admission into shared native compilation consumers and WASM processing handles; verify real hit/miss paths and stale/cancelled/superseded rejection before enabling browser compile shortcuts. Keep this ahead of DOM/style reuse and use the bounded native store rather than another app cache.
- [x] Fixture: force browser garbage collection while a stylesheet connection remains live; prove activated preparation handles and transferred commit arrays are collectible, cancelled handles drop candidates, and repeated replacements, late cancellation, reentrant adoption and scope disposal preserve current styles in worker/fallback modes.
- [x] Repair stylesheet previous-connection retention in shared cem-elements: release replaced/cancelled owners without retaining linked installation history, while preserving current theme/context behavior.
- [x] Reuse unchanged native-validated stylesheet installations in shared cem-elements; preserve style nodes, CSSOM and animation state while refreshing imports and transferring current lifecycle ownership.
- [x] Fixture: prove unchanged instance/declaration CSS has no installation mutations and preserves CSSOM identity; verify sibling additions/removals/order, changed native identities/context/import bytes, pending/failed/cancelled publication, DOM repair and worker/fallback cleanup without retaining previous generations.
- [x] Fixture: verify custom-element authored inputs refresh changed provenance without replacing unchanged nodes.
- [x] Fixture: verify unchanged direct/patch projections preserve edited text, textarea, checkbox, select and contenteditable nodes, focus and selection; changed bindings and complete owner/revision identities invalidate reuse; consumed prepared transactions release their snapshots.
- [x] Retain unchanged committed DOM in shared cem-elements using complete render/input identities; preserve focus, selection, edited controls and form state while changed inputs reconcile normally. Stylesheet installation reuse is covered above.
- [x] Fixture: verify repeated edits, theme/context changes, cancellation and runtime replacement release the correct DOM/style owners with bounded retention, while unchanged output preserves control/focus identity.

- [x] Fixture: verify native conditional/repeated root output and provenance, then browser direct/patch root insertion/removal/reordering and empty output with retained edits, focus/selection, cancellation, corruption recovery and restricted-preview rejection.
- [x] Fixture: reorder produced CEM roots with native and fallback DOM moves; prove current edits, stylesheet loads and consumer owners survive, while real removal and cross-parent moves still disconnect/reacquire.
- [x] Fixture: isolate declaration-import CSSOM retention on ordinary data updates to distinguish existing import behavior from root movement.
- [x] Extend shared render-plan publication to incremental root insertion/removal/reordering, including an empty result, while retaining compatible sibling DOM and its edits. Cover native output and browser admission before changing the patch protocol; root-count changes currently use scope fallback.
- [x] Fixture: verify native branch-selective binding reads (including direct record projection), missing/opaque/provider freshness, failed/cancelled evaluation, query/provenance identity, bounded receipts and owner/generation changes before any evaluation reuse.
- [x] Add native selective query dependency/read tracking and provider freshness contracts before enabling pure render-evaluation memoization; opaque inputs keep evaluating, with event/command/resource work admitted independently.
- [x] Fixture: aggregate native CEM-ML branch/loop/named-template/declaration reads with output/provenance parity; invalidate every provider freshness axis, policy and template/contract change; bridge validated native snapshots and reject incomplete/opaque closures; reject opaque callbacks, inherited runtime scopes, failed/cancelled/repaired attempts and bounded metadata without changing ordinary rendering.
- [x] Aggregate native query read receipts across CEM-ML renders and establish host/provider freshness contracts for data, attributes, payload, slices, resources, form, validation, events and refresh; opaque inputs must continue evaluating.
- [x] Fixture: prove real native pure-render hits, output/type/provenance independence, every provider/runtime/owner/revision guard, cancelled and reentrant admission, always-run event/command/resource admission, opaque/reference fallback, bounded capacity/eviction and released native owners.
- [x] Admit opt-in bounded native pure render-evaluation reuse only with complete current receipts and runtime ownership; fixture branch changes, provider invalidation, cancellation and independent event/command/resource execution.
- [x] Fixture: derive retained JSON frame freshness from actual native-parsed inputs; verify hits, changed values with unchanged revision strings, resource/native capability fallback, native reset/disposal, repeated resource/host updates, and stale/cancelled publication in real WASM plus worker/fallback processing.
- [x] Fixture: preserve nested component disabled-state repair when revisiting an earlier admitted pure body after a failed form submission.
- [x] Wire guarded native pure-render reuse into shared retained/WASM processing with explicit pinned provider snapshots and always-run event/command/resource admission; keep incomplete/native-reference cases uncached, alert before adding shared cem-elements JS, and verify worker/fallback publication ownership before enabling it for Studio.
- [x] Fixture: verify native allocator live/peak accounting, independent linear capacity, validated soft/hard/headroom policy, bounded concurrent reservations, refusal/cancellation/drop release, reconfiguration fencing and cooperative memory-pressure failure; exercise real WASM compile/render/command admission and post-pressure disposal.
- [x] Implement shared per-runtime allocator/linear-memory measurements, configurable admission budgets and recovery headroom with cooperative hard-pressure checks; keep browser/JS heap coverage explicit and disposal available under pressure.
- [x] Fixture: reject unacknowledged/mixed-owner checkpoints, stale generations, unsafe counters, oversized or cancelled candidates; preserve the last host-owned checkpoint across failed replacement and WASM reinitialization. Verify exclusive recovery headroom; keep transport sealing separate from semantic resume admission.
- [x] Define and implement portable committed checkpoints for between-render recovery, with headroom, admission/cancellation rules and native fixtures. Keep state outside the runtime being replaced; never serialize pointers or browser AST substitutes.
- [x] Fixture: select a newly imported project before repository notifications refresh the workbench inventory; retain unknown-target rejection and the previous active workbench. Repair inventory refresh ordering before URL selection admission.
- [x] Fixture: investigate the full-check Markdown portable-copy artifact URI failure, expose actual URI diagnostics, and verify isolated/full-suite identity assertions without weakening them.
- [x] Keep the Studio shell grid within a 320px viewport during graph authoring; verify the existing horizontal-overflow and keyboard accessibility assertions without hiding overflow.
- [x] Restore the declarative Operation output viewer alongside the independent project preview using shared cem-media-preview; verify Login sandbox/source/live output and avoid duplicate standalone preview mounts.
- [x] Fixture: verify the deployed navigator shell theme trigger, contextual navigation actions, status semantics, inherited token overrides and project-layout geometry in all themes/forced colors; retain all existing Login live-view, save/reload and offline assertions.
- [x] Repair the full Studio release/lifecycle verifier for the committed navigator shell: remove assumptions about the removed Install control, retain equivalent current-control theme/layout checks and all long-session Login assertions, then rerun the complete check before runtime replacement is enabled. The existing shell browser fixture explicitly requires Install to be absent.
- [x] Fixture: coordinate every active owner and outstanding admission before checkpoint capture; reject dirty/failed/removed/mixed-policy owners, stale async seals, oversized islands and counter/owner overflow. Coalesce capture requests without queueing or replaying input; preserve last-good export while denying stale recovery.
- [x] Fixture: exercise checkpoint publication through real worker/fallback DOM/CSS rendering, held stylesheet loads/failures, nested owners, immediate edits and mutation records, privacy refusal, disconnect and policy disposal. Verify complete inert data-island export and native sealing without enabling runtime replacement.
- [x] Wire opt-in committed-checkpoint capture to all-owner DOM/CSS publication and immediate pending-input/owner/policy invalidation for portable declarative owners. Require complete whole-island privacy consent and drained host admissions; coalesce captures without buffering/replaying input, refuse unsupported state for the entire domain, and preserve ordinary rendering and last-good export.
- [x] Fixture: enforce checkpoint traversal limits for nested inert templates in a different document realm; use native DOM template identity instead of the caller window constructor.
- [x] Fixture: preserve accepted input ordinals across a produced instance disconnect/reconnect; retain only weak instance counters and never reconnect an obsolete publication handle.
- [x] Fixture: validate bounded portable control state natively before browser capture: UTF-16 selections, disabled/unchecked/multiple-select edits, duplicate identities, password/file/unknown fields and invalid shapes. Verify real worker/fallback and plaintext-editor capture, immediate property-only changes, composition refusal, form-domain ownership and major-version fencing against older generic hydration.
- [x] Fixture: journal host operations before delivery; retain unknown outcomes on synchronous/async failure, stale completion and owner removal, bound journal growth without dropping actions, and never replay. Journal storage/location writes before their browser effect and refuse orphaned outcomes after owner removal; verify location writers without trigger tokens are also refused. Admit only routed host-command owners while preserving existing authority checks.
- [x] Fixture: retain complete fixed, read-only URL resource snapshots with an explicit read-again disposition; validate the existing native resource-state domain and reject live/ambient reads, location writers, storage/HTTP/repository resources until their complete admission contracts exist.
- [x] Extend checkpoint portability to native control edits, the shared plaintext editor and fixed read-only URL state, with explicit host/storage/location operation outcomes. Keep unsupported owners blocking the entire domain; validate controls natively and fence checkpoint-only form state from generic hydration.
- [x] Fixture: validate bounded checkpoint runtime metadata natively: capability tags, terminal resource lifecycle/revisions/cursors, read/resubscribe policies, consumed implicit/tokened writers, credential URLs, exact binary HTTP source bytes/digests, duplicate identities, malformed/oversized documents and normalized output limits. Verify canonical resource/form domains and fresh native import before browser admission.
- [x] Fixture: capture shared form/checkable controls and form-associated choice/select editors (selection, preview, range, defaults and typeahead), live/ambient location, local/native JSON storage, HTTP retained documents, repository queries/status and subscriptions through worker/fallback rendering. Reject composing/leased capabilities, pending resources, private/credential state, nonportable structured-clone values and oversized snapshots; detect changes during asynchronous exports and preserve ordinary behavior without replaying writers.
- [x] Fixture: allocate non-reusable journal identities, explicitly reconcile unknown outcomes under current runtime/generation/policy authority, stage terminal retirement and compact only after a successful all-owner native seal. Cover orphan retirement, failures/cancellation/races, stale handles, mixed policies, overflow, bounded repeated retirement and no replay.
- [x] Extend checkpoint portability to shared form-associated controls (field, select and autocomplete), checkable controls, settled HTTP/repository/storage and live/ambient resource state, consumed writer suppression metadata and explicit journal reconciliation/retirement. Validate metadata and original source bytes natively, preserve credential/lease/pending-state boundaries and never replay actions.
- [x] Fixture: validate non-form widget checkpoint state natively (tree/tabs/navigation/menu, closed popup/surface and focus anchors), including malformed state, pending sessions and unknown capabilities. Verify production declarations in worker/fallback rendering and retain all-owner refusal for unsupported states.
- [x] Fixture: validate bounded provider/placement reacquisition contracts against the complete acknowledged owner/revision set. Reject missing/stale endpoints, duplicate/conflicting relationships, unsupported properties, authority tokens and transient sessions; keep contracts inert and unsupported live owners blocking capture.
- [x] Define native/domain non-form widget and provider/placement checkpoint contracts: stable tree/tab/menu navigation, owner-local focus, closed settled surfaces, and bounded acknowledged dependency intents requiring fresh authority. Keep unsupported live owners blocking the entire domain; declarative grid layout uses ordinary payload/control/focus capture.
- [x] Fixture: capture settled active suggestions and exactly owned provider leases/claims with original XML/CEMB provenance, source-order row and committed-proof identities, query/feedback/visibility state and fresh dependency receipts. Refuse pending queries, gestures, composition, foreign claims, lost source authority and nonportable inputs through worker/fallback paths.
- [x] Fixture: capture committed foreign placement routes from the authority registry with original bounded source bundles and exact owner/revision/node dependencies. Reject forged markers, revoked/stale grants, absent producers and prepared transactions before/after asynchronous native sealing.
- [x] Fixture: validate settled open surface invocation/return-focus contracts and closed dialog return values natively; capture owned endpoint identities and retain refusal for asynchronous preparation/closing, gestures, context handles and retained deferred-body projections. Preserve normal widget interaction and full Studio acceptance.
- [x] Extend capture admission to settled active listbox suggestions/editor providers and committed foreign placements, with bounded original source provenance, private query/selection/proof state and fresh authority intent/revocation. Capture settled open popup/dialog invocation endpoints and closed return values; refuse pending work, gestures and unaccounted authority for the entire domain. Semantic authority reacquisition remains part of resume.
- [x] Fixture: validate and capture settled idle/failed/unavailable suggestion source intent natively and through worker/fallback declarations. Retain original sources and provider/query revisions without stale rows/proofs or retrying actions; refuse pending work, composition, revoked host policy and unknown provider claims.
- [x] Fixture: snapshot original native-session source bytes and selectors before asynchronous preparation; caller mutations during the pending job must affect neither native execution nor checkpoint provenance in worker/fallback modes.
- [x] Fixture: capture native datalist source/target and exclusive provider claim contracts in worker/fallback modes. Preserve native input types and edits, require fresh relationship acquisition, refuse in-flight preparation and unobservable focused UA picker state, and verify source mutation, policy revocation and foreign claims.
- [x] Extend settled widget/source capture to inactive listbox states and native datalist providers, including failed local sources, exact attributes/datalist/empty claims and subscription accounting. Preserve original source intent without obsolete row proofs, keep unobservable focused native pickers blocking capture and copy eligible source input before asynchronous submission.
- [x] Fixture: validate retained inactive listbox proofs with their exact original source and ordinal natively; capture worker/fallback readonly handoffs without losing required-selection validity, require fresh ready publication on re-enable, preserve refusal for unobservable read-only validation messages after revocation and capture after repair, and reject mismatched values, revoked proofs, malformed provenance and authority serialization.
- [x] Fixture: validate settled submenu parent/launcher/open-panel endpoint contracts natively; capture worker/fallback menu navigation and completed menu-to-dialog handoffs with complete dependency receipts, while refusing held gestures, queued task activation, missing endpoints and in-flight preparation.
- [x] Extend shared checkpoint capture to retained inactive listbox proofs with exact original provenance and settled submenu/completed task handoffs with complete current endpoint receipts. Require ready feedback before reclaiming a publication after availability changes; retain gesture, preparation, unknown-authority and unobservable native validation refusals.
- [x] Fixture: validate original-source native publication checkpoints for non-editor declarative consumers before browser admission; reject malformed provenance/configuration, duplicate or stale row identities, unknown profiles, authority fields and aggregate byte overflow. Verify worker/fallback rendering, source revocation, host policy/currentness changes and fresh native reimport.
- [x] Fixture: exercise external-source suggestions with independently owned form providers through production declarations in worker/fallback modes: ready/failed/inactive provenance, exact provider/placement subscriptions, ordinal proofs, source/query revocation and repair, asynchronous sealing races and foreign leases. Keep standalone unaccounted delegates blocking the whole domain.
- [x] Extend shared external-source listbox/provider capture and source-backed declarative frames with native-validated original publication provenance, exact row identities and fresh source/provider reacquisition intent. Refuse opaque/revoked inputs, stale asynchronous seals and unaccounted delegates.
- [x] Fixture: validate standalone manual-listbox ownership and deferred-body scalar contracts natively; reject forged endpoints, authority fields, invalid modes, duplicate projection identities and oversized records before browser admission.
- [x] Fixture: capture opted-in standalone delegates and current native deferred-body projections in worker/fallback rendering; cover provider conflicts, pending/revoked lifecycle, edited controls, empty/disposed bodies, stale template projection, foreign DOM and asynchronous sealing races without losing last-good bytes.
- [x] Extend shared checkpoint capture to opted-in standalone manual-listbox ownership and current native deferred-body projections, with native scalar validation, exact provider/endpoint receipts, rematerialization intent and ordinary edited-control/focus capture. Refuse stale projections, unknown ownership and pending work without disposing edits.
- [x] Fixture: validate bounded original retained projection records, paired dynamic ranges and nested owner endpoints natively; reject malformed topology, duplicate identities, opaque authority, missing roots/owners, unknown formats and aggregate projection/source overflow.
- [x] Fixture: capture stale retained, nested produced/deferred and dynamic body projections in worker/fallback modes; preserve independent child edits/focus, reject foreign mutations and incomplete/pending child owners, and preserve last-good bytes across async sealing races and disposal/reopen.
- [x] Fixture: preserve ordinary deferred-surface opening with authored comments around the body template; refuse unaccounted owner comments only during checkpoint capture.
- [x] Extend native/shared checkpoint capture to immutable original retained projections, paired dynamic ranges and independently owned nested produced/deferred bodies. Preserve original native publication identity, edits/focus, current child dependency receipts and next-open template replacement behavior; retain whole-domain refusal for opaque/private/unknown state.
- [x] Fixture: validate standalone native-datalist source/ownership records natively, rejecting authority fields, malformed endpoints, unsafe revisions, picker restoration, opaque input and aggregate source overflow.
- [x] Fixture: exercise standalone datalist attachments in worker/fallback modes, including exact provider/target receipts, unchanged edits, focused pickers, competing leases, source/placement revocation, foreign options, disposal/replacement with unchanged native DOM from an earlier revision, and asynchronous sealing races.
- [x] Fixture: reject an actual native attribute capsule on a standalone datalist root before granting portable ownership, even when scalar option witnesses are intact.
- [x] Refresh the eight stale curated component-search heading references against the generated native documentation; keep production fragment verification intact.
- [x] Repair the published transform-config reference to the Markdown metadata contract, whose local README is not a deployed Site route; rerun production link verification.
- [x] Repair Site’s missing shared checkpoint/preview/stylesheet resource declarations and verify production search and interactive delivery with the current runtime.
- [x] Declare the shared standalone datalist adapter in Studio and Site source/destination deployment maps; verify closed module graphs and clean consumer packaging.
- [x] Add shared standalone datalist ownership checkpoint adapters with private provider accounting, original committed native options and source provenance, and unfocused reset/reacquisition intent.
- [x] Fixture: validate complete submenu geometry/focus policy records natively, including default/none/target roles, inherited boundary ownership and fresh-layout intent; reject malformed endpoints, unknown modes, authority fields and pixel restoration.
- [x] Fixture: capture explicit submenu anchor/boundary/entry-focus/return-focus policies in worker/fallback modes; cover closed/open and inherited/typed/scoped references, focus/edits, geometry invalidation, foreign ownership, revocation, disposal and stale sealing without losing last-good bytes.
- [x] Extend shared submenu checkpoints with explicit policy/endpoint receipts and read-only settled geometry proofs; retain fresh authority/layout/focus admission and whole-domain refusal for unsupported state.
- [x] Fixture: validate portable retained-projection native attribute/source-value artifacts natively, preserving typed graphs, provenance and shared artifact indices; reject corruption, mismatched hashes/bindings, private fields, sensitive content and aggregate overflow.
- [x] Fixture: capture native attribute/source-value capsules in original retained bodies in worker/fallback modes, including changed/current templates, native metadata replacement, nested owners, preserved edits/focus, async seal races, disposal and unsupported capsules without losing last-good bytes; reimport in independent WASM modules.
- [x] Extend native/shared retained projection checkpoints with bounded opaque CEMV envelopes and private committed capsule witnesses, keeping original native source semantics and fresh ownership admission; never derive capsules from rendered strings or decode native trees in JavaScript.
- [x] Fixture: validate native invocation context capsules and preparation control frames/additional attribute and slice bindings before extending capture; reject corruption, authority fields, malformed wrappers, duplicates, privacy violations and aggregate overflow.
- [x] Fixture: capture native context and nonempty preparation inputs/additional bindings in worker/fallback modes, including immutable invocation/submission copies, independent native reimport, revocation, stale seals and whole-domain refusal without lost edits.
- [x] Extend shared native session/surface checkpoints with bounded original preparation control-frame bytes and native-validated opaque CEMV capsules, preserving explicit limits and default resolution without reviving authority.
- [x] Fixture: enforce retained preparation limits on encoded CEMV bytes and the combined additional-binding payload, including binary envelope overhead.
- [x] Fixture: validate explicit source traversal depth/work and unresolved policies natively, preserving omitted versus explicit defaults and rejecting invalid/unsupported policy records without granting crossings.
- [x] Fixture: validate same-domain cross-runtime endpoint receipts with complete owner revisions natively; reject missing, mixed-domain, stale, forged and authority-bearing endpoints.
- [x] Fixture: capture custom source policies and same-coordinator cross-runtime geometry/focus, provider/delegate and placement endpoints in worker/fallback modes; cover edits/focus, input/publication races, revocation, foreign domains, copied markers, moved targets, duplicate hosts, disposal and stale seals without losing last-good bytes.
- [x] Fixture: validate explicit custom source policy and complete cross-runtime dependency receipts in an independently instantiated WASM module, preserving exact originals and rejecting stale/authority-bearing records.
- [x] Extend shared source policy and coordinator-owned endpoint adapters with native validation and exact publication ownership; keep physical runtime discovery/replacement and resume separate.
- [x] Complete remaining widget/source checkpoint adapters: custom CEMB source policies and cross-runtime endpoints within explicitly shared coordinator domains. Keep unsupported owners blocking the complete domain and add native-first fixtures before extending admission.
- [x] Fixture: bind publications to privately issued worker-slot/fallback-module generations; reject omitted/foreign clients, held host-only work, disposed generations and stale asynchronous seals.
- [x] Fixture: account for direct retained native resources and validate bounded complete runtime/client/owner receipts natively and in an independent WASM module.
- [x] Fixture: refuse independently created checkpoint ledgers claiming the same actual Worker/WASM memory so duplicate framework copies cannot claim partial coverage.
- [x] Fixture: worker loss during retained-source cleanup must retire the old generation without poisoning shared fallback coverage; reject native session results that lose their execution generation before acknowledgement.
- [x] Bind checkpoint domains to actual shared worker/fallback runtime generations and every managed native client, including standalone hosts/helpers, native source/document capsules and host work outside instance rendering; preserve generation, publication and cleanup fences.
- [x] Fixture: pressure-close actual worker/fallback generations; reject fresh native work while queued work, cleanup and host stylesheet I/O drain; cover deadlines, cancellation, revocation, cross-generation moves, late replies and stale/copyable drain evidence without losing last-good checkpoints.
- [x] Fixture: refuse direct XPath-library cache acquisition/reuse and synchronous source-view/XSLT construction after pressure closure; keep asynchronous acquisition in the drain until completion/disposal.
- [x] Fixture: enroll transient helper work atomically with its pending lease and notify peers only at enrollment/retirement; preserve pressure-race cleanup and complete drain coverage without redundant publication invalidations.
- [x] Fixture: exercise production worker and fallback pressure closure with real native memory budgets, cleanup and complete checkpoint coverage; retain the full Studio long-session gate and independent WASM pressure/headroom checks.
- [x] Add pressure-triggered native admission closure, drain/timeouts and independently verified complete client coverage before permitting runtime replacement. Cover separate command/native runtimes and unmanaged embedding boundaries explicitly; retain the complete Studio long-session gate.
- [x] Fixture: close managed CLI main-thread and worker native admission independently from rendering; retain command callback waits, artifact handles and resumable hosts as checkpoint barriers; refuse copied/stale/foreign drain evidence, timeout revival and raw-generated-WASM authority.
- [x] Fixture: exercise real Node and browser command workers under native pressure, retain cleanup/cancellation, reject fresh execution without worker failure/replay, and preserve Studio's isolated full-release gate.
- [x] Add closed client/admission adapters for separately instantiated command-service/native heaps and define the explicit unmanaged-embedding boundary before whole-runtime replacement; never treat shared render coverage as authority for another heap.
- [x] Fixture: native-first census validation requires every linked command/reference registry, distinguishes live resources from rebuildable caches/retired journals, and fences same-count mutations and unpolled async commands.
- [x] Fixture: original embedding leases bind actual WASM memory/Worker identities; require complete current ownership consent and refuse omitted/copied/foreign/released leases, stale native inventories, and changes during capture.
- [x] Fixture: exercise raw retained documents/templates/reference sources/callback hooks and command bridges through real Node/browser WASM; preserve legacy managed-only checkpoints, cleanup, Studio's full lifecycle and isolated saved projects/drafts.
- [x] Add authoritative native command/resource registry inventories and opt-in embedding ownership before admitting command-heap replacement; keep raw generated-WASM aliases explicitly unmanaged and never promote managed-only witnesses to complete physical coverage.
- [x] Fixture: native-first admission closure refuses every ordinary generated-WASM entrypoint, accounts for synchronous/unpolled asynchronous work, retains cleanup and rejects open/busy/stale/incomplete inventories under one native validation lock.
- [x] Fixture: coordinated close/revalidation requires original complete embedding leases and current original witnesses; reject copies, foreign/changed owners, raw cleanup races, same-count native changes, stale replies, timeouts and worker loss without enabling replacement.
- [x] Fixture: real Node/browser aliases cannot start native work after coordinated closure; admitted callbacks settle and cleanup remains available. Preserve legacy inventory-only capture, independent heaps, saved projects/drafts and the full Studio release/navigation/Site gates.
- [x] Add coordinated raw/native admission closure and atomic inventory-revision/embedding-lease revalidation before replacement; unregistered aliases remain unsupported and worker snapshot currentness alone cannot authorize retirement.
- [x] Fixture: native retirement permits freeze cleanup only after a current complete closed inventory; stale cleanup cancels preparation, original permit disposal reopens cleanup, the held permit remains an inventory root, and copied scalar confirmations cannot release the barrier.
- [x] Fixture: host checkpoint/journal handoff requires current sealed bytes, terminal outcomes and unchanged publication; preserve last-good bytes on stale input, failed retirement and unknown actions; bound independent byte copies, reject shared/oversized storage and avoid caller slice hooks.
- [x] Fixture: real Node/browser worker retirement requires original complete ownership and native confirmation; cancel on handoff/owner changes, raw cleanup, timeout or loss, retain cleanup after abort, and never restart or replay actions. Run full Studio/navigation/Site acceptance in isolated contexts.
- [x] Fixture: strict ES2022 package consumers can typecheck generated native retirement classes with their declared disposable-symbol library, without weakening TypeScript checks.
- [x] Bind fresh native/ownership confirmation to actual command-worker generation retirement and checkpoint/journal handoff; stale cleanup must cancel retirement, and no scalar witness grants replacement authority. Alert before introducing the shared browser lifecycle capability.
- [x] Fixture: shared rendering retirement requires original closed drains, every pooled client/source/owner, current checkpoint/journal bytes and native freeze confirmation; refuse copied/stale/omitted evidence, changed owners, timeout and worker loss without fallback/replay.
- [x] Fixture: real rendering workers release owned sessions/documents/templates before the native zero-root barrier, retain host bytes and DOM edits/focus, and refuse unmanaged roots; shared main-thread/fallback retirement is explicitly unsupported without an exclusive disposal boundary.
- [x] Extend the original native retirement barrier and current checkpoint/journal handoff to shared rendering workers; resolve fallback/main-thread generation ownership explicitly, require complete closed client/source/owner coverage, and keep unsupported physical retirement blocked.
- [x] Fixture: independently instantiated fallback bindings own distinct WASM memories; native zero-root disposal releases owned references, fences saved aliases/initializers, handles finalizers safely, and refuses live roots or initialization races without reviving a generation.
- [x] Fixture: shared render/query/XPath helpers use the opted-in owned fallback generation; require complete original embedding leases and no managed peers/resources at empty disposal, reject copied/foreign/released/racing consent, preserve legacy heaps and package/deployment consumers, and keep active-owner checkpoint retirement blocked.
- [x] Fixture: refuse adversarial ownership-consent arrays without invoking caller iterators, methods or accessors; reject reentrant disposal and consent changes during property inspection before closing admission.
- [x] Fixture: ship the shared native facade and owned factory through Site source/destination maps; verify byte-identical deployment assets and rerun production/search/interactive delivery after the missing-facade failure.
- [x] Isolate shared fallback WASM generations behind an exclusive module/memory disposal boundary, including standalone native helpers and original embedding ownership; retain explicit refusal while raw aliases or peer clients cannot be covered. Alert before adding the shared lifecycle capability.
- [x] Fixture: owned fallback retirement requires original complete embedding consent bound to the original closed drain, complete clients/sources and current checkpoint/terminal journal bytes; refuse primitive tokens (including numeric zero with no ownership claims), copies, omitted peers, changed consent, newer input, expired cleanup/freeze deadlines and reentrant preparation before releasing references.
- [x] Fixture: terminal fallback cleanup releases only covered native resources in one synchronous window; unrelated ledger mutations, missing cleanup adapters, native blockers and final-freeze ownership changes fence retirement and preserve host bytes without replay or replacement.
- [x] Fixture: real Chromium active fallback retirement across multiple hosts/scopes preserves stylesheet installations, edited/focused controls and portable resource/checkpoint/journal bytes, keeps legacy WASM independent, refuses native blockers and unknown operations, and retains full Studio/navigation/Site acceptance in isolated contexts.
- [x] Bind owned fallback retirement to current checkpoint/journal handoff and complete client/source/embedding consent; terminally clean up owned resources, freeze the native inventory and release the exact generation while preserving host bytes and edited DOM on failure. Keep legacy fallback retirement unsupported.
- [x] Fixture: original generation participants and retirement receipts form one bounded complete allocation group; reject copies, omitted/duplicate peers, reused receipts, array inspection reentrancy, stale candidates and foreign physical identities before atomic admission.
- [x] Fixture: failed or expired fresh allocation revokes every candidate, including late initialization; original aliases stay retired and independent checkpoint/journal bytes survive caller mutation without automatic resume or action replay. Keep admitted/revoked coordinator handles free of strong candidate/physical-memory references and reject in-flight rendering requests on explicit revocation.
- [x] Fixture: real WASM/Chromium and command-worker fresh allocation covers owned fallback, rendering and command workers in coordinated groups, with native headroom guards and preserved edited DOM; retain Studio/navigation/Site acceptance.
- [x] Fixture: preserve Site's source/destination runtime path convention for the coordinator facade and vendor in both search and interactive delivery; assert adjacency to native-runtime and verify byte-identical production assets, published documentation links, search and interaction.
- [x] Coordinate fresh native/WASM generation allocation and admission across shared runtimes without retargeting old aliases or permitting partial multi-heap replacement. Require original complete ownership and retained checkpoint/journal handoff; alert before adding the shared browser lifecycle capability.
- [x] Fixture: original fresh admissions and pre-retirement owner resume tickets require complete physical/owner coverage and exact private checkpoint bytes; refuse copies, omissions, reuse, inspection/probe reentrancy, changed policy/input, unknown operations and unsupported adapters.
- [x] Fixture: staged owner/source/widget preparation stays inactive until every peer is current; timeout, cancellation, newer edits, explicit revocation and late completion fence before abort/disposal and discard every candidate without replay or lost host bytes. Verify actual fresh WASM source reimport and preserved Chromium focus/selection.
- [x] Add the shared owner-resume transaction and original-admission binding, with bounded preparation, fresh authority gates and complete rollback; preserve explicit refusal for runtime owners without semantic adapters.
- [x] Fixture: fresh generation-bound render bindings isolate native helpers, caches, XPath libraries and client/resource ledgers; copied runtime handles, retired aliases and revoked processing work refuse without fallback.
- [x] Fixture: fresh processing reimports opaque canonical islands and original external source bytes through native validation/import before evaluation; staged work stays inactive and cancellation cleans imported resources.
- [x] Fixture: restored consumed writers suppress every external write until a newer accepted input, preserving consumed tokens across ordinary rerenders and rejecting stale/invalid event sequences. Verify real worker/fallback processing and retained DOM edits/focus in Chromium.
- [x] Provide original-generation processing bindings and a shared native checkpoint/source preparation helper with consumed-write suppression; keep semantic DCE activation separate.
- [x] Fixture: renewed publication requires an original active resume context from the same coordinator/owner and a fresh native client for its admitted generation; reject copies, cross-domain contexts, live predecessor clients, duplicate binding and stale/revoked authority while retaining accepted event counters and journals.
- [x] Fixture: built-in plain retained CEM-ML owners prepare canonical islands, sources, compilation and consumed writes without live DOM mutation or early publication; refuse unsupported capabilities/providers/placements and newer property-only edits, declarations or policies before admission.
- [x] Fixture: real worker/owned-fallback built-in owner resume preserves canonical slice/control values, retained DOM edits/focus/selection and styles, renews complete native receipts and supports another capture/retirement, safely renews stylesheet sources without replacing installed nodes, and refuses overlapping/timed-out read-only recovery; validate source refresh and new accepted-input evaluation without replay, explicitly refuse legacy storage/location owners, and cover native recovery validation/sealing after ordinary admission closes.
- [x] Wire built-in CEM owners to the shared fresh processing/source preparation helper, restoring canonical island bindings and renewed publication before first evaluation; keep unsupported capabilities explicitly refused.
- [x] Fixture: establish the shared editor contract for Studio using the `packages/cem-demo-element/demo/playground.html` source pane as the reference; preserve exact authored text (including tabs, Unicode and trailing newlines), editable invalid/incomplete source, native undo, IME composition, focus/caret/selection and scroll through coloring, unrelated renders and file switches. Refuse stale asynchronous results after newer edits/file selection; keep native work bounded, cancellable and bound to the current generation.
- [x] Fixture: native lossless highlighting bounds request/source bytes and paint ranges; verify tabs, Unicode, CRLF/trailing lines, malformed/incomplete HTML/CEM-ML/CSS, nested languages and unsupported content types without formatting. Shared worker/fallback paint jobs obey cancellation, pressure and original runtime generations; CSS paint preserves native undo/IME/selection/scroll, multiple editors and cleanup.
- [x] Fixture: retain and emit terminal `::highlight(custom-ident)` through the native CSS selector AST, preserving identifier case/escapes, specificity and scopes; reject invalid arguments, unsupported function/placement and query selection. Verify editor scoped CSS has no diagnostics.
- [x] Extract/reuse the playground's native source-coloring presentation through shared cem-elements support and enhance declarative cem-source-editor in cem-components. Preserve its value/input/selection API and CEM theme/forced-color behavior. Keep coloring separate from source-changing formatting, which remains an explicit command. Alert before adding shared JavaScript; keep Studio UI and component composition declarative.
- [ ] Adopt the upgraded cem-source-editor in Studio and reuse it in the demo playground; preserve project/resource identity, URL-driven selection, drafts, Save/Discard, Parse/Inspect/Format, diagnostics/provenance navigation and independent build previews. Retain sample read-only behavior and offline package delivery.
- [ ] Fixture: verify the shared colored editor in its demo/gallery and Studio's Login/Supplier/Markdown projects, including draft persistence, explicit formatting and stale-result cancellation. Treat colored spans as disposable presentation: checkpoint authoritative text and UTF-16 selection, then regenerate decoration on the fresh runtime as part of the following plaintext/widget resume work.
- [ ] Diagnose the existing `ActiveSuggestionsProviderAndProofCapture` worker/fallback checkpoint story timeouts without relaxing assertions or deadlines. Both 30-second timeouts reproduce on committed `08394d00` with independently rebuilt original WASM; 56 other checkpoint stories pass. Keep Studio/editor adoption ahead of this recovery fixture work.
- [ ] Implement built-in form/plaintext/choice and non-form/provider/placement widget resume adapters with fresh authority, renewed publication and preserved live DOM edits/focus/selection.
- [ ] Resume acknowledged checkpoint owners, sources and widget state on fresh generations without lost edits, stale handles or repeated external actions; retain full Studio long-session acceptance and explicit refusal of unknown outcomes. Alert before adding the shared browser lifecycle capability.
- [ ] Expose recovery status declaratively through shared cem-components and integrate Studio as a consumer, keeping app-specific UI declarative.

Editor sequencing decision (2026-10-21): schedule playground-style source editing immediately after built-in owner/fresh-processing wiring and before form/plaintext/widget resume adapters. Studio already uses the shared plaintext contenteditable component; the playground additionally applies native formatted/colored source markup on Render. Reuse that native presentation capability through the shared component rather than introducing a Studio-specific editor controller. Establish the upgraded editor's durable text/selection contract before implementing its recovery adapter, so the adapter and long-session acceptance exercise the intended editor. Source-preserving highlighting is distinct from explicit formatting. The remaining recovery milestones retain their order. The shared source-preserving coloring upgrade is implemented before Studio/playground adoption and widget recovery adapters.

Shared source coloring (2026-10-21): `cem-source-editor` now uses the playground's native lossless HTML/CEM-ML/CSS semantic roles through a bounded `sourceHighlightPaintV1` presentation DTO (UTF-16 offsets/roles, no AST/token-stream handoff). Shared `cem-elements` worker/fallback jobs use protocol v28, the original native client/generation and ordinary admission/draining, 80 ms debouncing, cancellation and a five-second deadline; worker loss refuses paint without replay/fallback. Native request/source/range limits are 2 MiB / 256 KiB UTF-8 / 16,384 ranges. The existing source-to-HTML formatter remains unchanged. CSS Custom Highlights paint the existing native editing subtree without inserting markup or changing text, undo, selection, composition or scroll. Source/language/resource changes, unreported DOM edits, disconnection and authority loss fence stale results. Native CSS import/emission now retains terminal `::highlight(custom-ident)` with decoded case-sensitive identifiers, original ranges and type specificity; invalid arguments/placements and selector-query use remain refused. Public attributes are `content-type` (default `text/plain`), `highlight=auto|off` and optional `source-id`; unsupported languages/browser APIs or unavailable native processing retain plain editing. Source-owned capability identity is v2. Formatting and widget recovery remain explicit separate work.

CEM audit: component markup, bindings and scoped styles remain XHTML/CEM-ML; browser editing/paint ownership is generic shared `cem-elements` functionality, with no Studio or component implementation JavaScript. Syntax uses inherited canonical trust/enthusiasm/creativity/calm foregrounds and script typography, with CanvasText/GrayText forced colors and no new public token defaults or hardcoded palette. All nine painted semantic roles meet 4.5:1 contrast in all five shipped themes, and inherited overrides remain effective. The source/packaged property playground and gallery exercise supported/nested/incomplete source, explicit opt-out and unsupported-language fallback. Studio and the demo playground still need resource-language/identity wiring and shared-editor adoption; language services and additional native lossless language adapters remain future work.

Validation: all 79 selected Rust highlighting/CSS import/emission/nesting/query cases, 748 shared runtime unit tests, eight focused worker/fallback/editor browser plays, source and isolated-package editor galleries (including forced colors/disabled text), real WASM memory/headroom checks, runtime TypeScript, declarative verification, component/runtime lint (zero errors), catalog/state matrix and clean runtime package import pass. Full frozen-source Studio release acceptance passes all 52 tasks (43 cached), including 104 workbench cases, Login/Supplier failure/repair/live view, two-project lifecycle/accessibility, offline commands and clean consumers; deployment remains deterministic at 202 assets/297 files. Site passes all 27 tasks (20 cached). A concurrent Studio rebuild initially collided with the navigation reader; a workbench case also exceeded its unchanged 15-second deadline under load, then passed with the current editor and committed baseline in 13.15/13.19 seconds and all 104 cases passed on the complete rerun. The final release gate passes without relaxing assertions/timeouts. Navigation passes all 26 tasks (23 cached), covering URL/history, trees, Create/Clone/Delete, drafts, themes, responsive layout and offline Supplier/Markdown builds. Two broader active-suggestions checkpoint stories remain baseline failures as recorded above. All 3,488 frozen source/fixture/contract/module-map files stay unchanged through acceptance (only this todo evidence excluded). Isolated browser contexts preserve saved projects/drafts; generated screenshots and Vitest attachments remain ignored and excluded.

Design: [Project builds and artifact previews](cem-studio.md#project-builds-and-artifact-previews). Run manifests, independent preview selection and shared relative artifact loading are implemented. The ordinary Markdown site, explicit entry artifacts, bounded pinned snapshots and pin/current comparisons are implemented. Native identity/invalidation, bounded artifact storage/leases and checked native/WASM compilation consumers and stylesheet ownership/installation reuse are established; fresh-output DOM reuse and transaction cleanup are established. Incremental root topology, native query read receipts and render-wide provider freshness aggregation are now implemented; guarded pure-render reuse now reaches shared retained WASM/worker processing for fully owned JSON inputs. Per-runtime allocator/linear-capacity accounting, guarded admission budgets and recovery headroom are established. Bounded portable committed-checkpoint transport and host-owned session storage are now implemented. Opt-in host-domain publication/capture wiring is implemented for complete portable declarative owners. Native control/shared plaintext-editor capture, fixed URL snapshots and bounded external-operation outcomes are implemented. Native-validated form/choice capability state, settled resource source/reacquisition policies and explicit terminal journal retirement are implemented. Stable non-form widget capture and native provider/placement dependency contracts are implemented. Settled active listbox/provider/placement capture and open popup/dialog contracts are implemented. Native invocation context and nonempty preparation/additional materialized attribute and slice bindings now have native-validated portable capture with immutable input witnesses. Explicit CEMB source policies and cross-runtime checkpoint endpoints now work within shared coordinator domains. Actual shared worker/fallback generation binding and complete managed-native-client/source/resource coverage now guard capture. Pressure-triggered ordinary admission closure, bounded draining/timeouts and private closed-domain client/resource/source coverage checks are now established in shared runtime support. Managed CLI admission/checkpoint adapters now cover separately linked command heaps, with explicit refusal of whole-heap authority for generated-WASM aliases. Authoritative native registry inventories, opt-in embedding ownership and coordinated closure/revalidation now guard explicit command-worker retirement with current host checkpoint/journal handoff. Shared rendering-worker retirement now reuses the original native barrier after complete managed-owner cleanup and current host checkpoint/journal handoff. Opt-in fallback factories now isolate native memory and bindings, route shared standalone helpers to the selected instance and permit empty-generation disposal with complete original embedding consent. Active owned-fallback retirement now binds complete embedding/client/source consent and current checkpoint/journal handoff to terminal native cleanup and guarded reference release. Legacy fallback remains unsupported. Explicit complete-group fresh-generation allocation/admission now works across shared runtimes. Semantic resume and declarative status remain before automatic between-render recovery.


Built-in owner bindings and renewed publication (2026-10-21): the announced shared JS lifecycle extension now prepares plain retained CEM-ML owners behind the complete original resume gate. Native control/runtime validation, canonical compilation and approved HTTP source reimport finish while inactive. The existing DOM island adapter uses XMLSerializer for strict native XML ingress of namespace-qualified parts, HTML void payload and Unicode; original sealed bytes remain the transaction authority, and no source AST crosses JSON. Host-only witnesses detect newer control properties, declaration text (including template child text), source bytes/policies and input changes before admission. Explicit `CemResumedElementOwner.resume()` then renews publication against that same coordinator/owner context and exact fresh native client before evaluating. Event counters, journals, canonical slices, control nodes, focus, selection and installed styles survive. HTTP refresh and stylesheet sources use renewed ownership; retired source/scope references are released. Copies, foreign owners, live predecessors, duplicate/revoked authority, changed input/declarations/policies and unsupported capability/provider/placement or legacy storage/location owners refuse. Bounded generation-bound read-only controls retain drain/freeze barriers and deadlines without changing managed publication revisions; native inventory remains independently revalidated. Another capture/retirement succeeds after fresh ordinary admission closes. Old aliases stay retired, no effects replay, and automatic Studio recovery remains disabled.

Validation/CEM audit: 744 shared runtime unit cases (77 files), 78 Chromium checkpoint/stylesheet plays, 56 native admission/inventory/memory/checkpoint cases, 11 generation cases and all three independent real-WASM ownership/memory/checkpoint probes pass. Runtime build/TypeScript, clean package verification (334 files) and lint (zero errors; 557 warnings) pass; unchanged CLI build, all 52 CLI cases, browser and lint targets reuse valid Nx cache. Full frozen-source Studio release passes 52 tasks (26 cached), including 104 workbench cases, 32 native examples, deployed two-project lifecycle/accessibility, Login semantic/live/format comparisons, Supplier failure/repair, offline commands and clean consumers. Deterministic deployment contains 202 declared assets/297 files (sha256:d6e0acc05550993c28a37a5115dfa5e65e0e0d2fb35492f6ec58106ebde648da). Navigation passes 26 tasks (24 cached), covering URL/history, trees, CRUD, drafts, themes, responsive layout and offline Supplier/Markdown builds. Site passes 27 tasks (20 cached), including deterministic, production, search and interactive delivery. All 3,487 frozen tracked/unignored source files remain unchanged through final gates; only todo evidence is exempt. Lifecycle code remains reusable cem-elements support, UI remains declarative, and theme inheritance/forced colors remain verified. Isolated contexts preserve saved projects/drafts; generated screenshots and Vitest attachments remain ignored/excluded. The shared playground-style editor upgrade is next, followed by widget/legacy-resource resume and declarative recovery status.

Fresh processing/source preparation (2026-10-21): the shared JS lifecycle capability was announced before implementation. Privately issued fresh worker/fallback facades now provide explicit processing hosts without retargeting existing roots or the selected fallback. Render helper caches are shared per physical heap; XPath libraries and client/resource ledgers bind to that same heap. Fresh roots share worker job sequencing, while copies and fresh-worker failure refuse without fallback. Protocol v26 adds a named native checkpoint-validation/import operation. The shared `prepareCemCheckpointProcessing` helper validates named control DTOs and imports/releases the opaque original island through the native importer, reimports explicitly policy-approved original document bytes under new logical handles, and compiles cloned canonical authoring inputs while the semantic owner gate remains closed. It exposes no evaluation host or source handles until complete transaction admission. A newer accepted input during preparation now refuses before import. Revocation cleans source/engine resources before physical disposal and clears allocated host source/compile maps. Restored consumed tokens plus an accepted-input barrier suppress implicit, tokened and newly exposed external writes before journal/effect admission; renders and refreshes do not count as input. Invalid/regressing sequences and out-of-bounds writer identities refuse.

Boundary and CEM audit: this is reusable cem-elements processing preparation for trusted semantic adapters, not built-in DCE activation or automatic Studio recovery. Built-in canonical island binding, widget authority and renewed publication remain open. All source syntax/validation/import remains native; JavaScript reads only named lifecycle control DTOs, opaque source bytes and existing render-plan contracts. No app/component UI JS, DOM restoration from projections, CSS defaults, public token changes, external subscription or action replay was introduced. Browser fixtures preserve live edits, focus and selection and use isolated sessions without changing saved projects/drafts. Studio/Site receive explicit source/destination module-map entries only. The failing-first helper factory and newer-input cases now pass, along with all 741 unit cases, 56 selected native admission/inventory/memory/checkpoint cases, all 56 checkpoint Chromium plays, 11 common generation controls and the three real-WASM probes. Shared build/types/package verification (334 files) and lint (zero errors; 538 warnings) pass. All 52 CLI control cases and build/lint/browser targets pass using valid Nx cache evidence for unchanged command-runtime inputs. Full Studio release acceptance passes all 52 tasks (26 cached), including 104 workbench cases, 32 native schema-package examples, deployed two-project lifecycle/accessibility, Login/Supplier failure/repair, all 23 offline configurations, theme inheritance/forced colors and clean consumers. Studio deployment is deterministic across 202 declared assets/297 files (sha256:e7018f0832ab5bff0900de46fae6ae1ac04ebe7df822136d0d432b7b21cd6d73). Navigation passes all 26 tasks (24 cached), including URL/history, model trees, Create/Clone/Delete, drafts, themes, responsive layout and offline Supplier/Markdown builds with saved edits, intentional failure/repair and portable independent copies. Site passes all 27 tasks (20 cached), including determinism, production, search and interactive asset delivery. Both final validation processes exit zero. All 20 frozen implementation/fixture/map/contract hashes and the exact 21-path delivery set remain unchanged apart from this TODO evidence. Whitespace, focused Rustfmt and screenshot/Vitest-attachment exclusions pass. Built-in DCE island binding, source/widget activation, renewed publication and automatic Studio recovery remain open. Exploratory missing resource-envelope/host-binding fixture failures are excluded from acceptance.

Explicit owner-resume transaction (2026-10-21): the shared lifecycle extension was announced before implementation. Original pre-retirement publication tickets now bind complete trusted semantic adapters, native-sealed opaque islands, current privacy/input/policy witnesses and terminal journals to original complete fresh-generation admissions. Private retirement bytes guard resume independently of public exports. One inactive owner gate covers bounded reversible preparation; fresh runtime/owner probes and host witnesses are rechecked before atomic admission without activation callbacks. Timeout, changed host state, external revocation and late completion discard prepared owners and revoke the whole fresh group. Native work is fenced before owner cleanup; an explicit disposal/release allowlist keeps source cleanup available until physical fallback disposal. Claims lock before freshness probes; regression tests exposed and repaired reentrant double claims and allocation-abort ordering. Superseded/input-invalidated/policy-invalidated tickets erase private bytes and cancel pending preparation. Completed attempts drop old adapters/witnesses and zero then release private ticket buffers. The real-WASM Chromium fixture uses a deliberately small trusted-host adapter to validate portable edits/tree state, import original source bytes on fresh memory, retain consumed-writer intent and preserve DOM/focus/selection; a newer property-only edit refuses the complete transaction without rollback. Built-in DCE owners still explicitly refuse resume: fresh processing contexts, canonical island/source integration, all built-in widget/provider/placement adapters and renewed publication remain open work. No automatic Studio recovery is enabled.

CEM audit: imperative lifecycle work stays in reusable cem-elements and the shared generation coordinator. No application/component UI JavaScript, CSS/token defaults, alternate AST representation, persistence migration or external-action replay is added. Studio/Site receive only explicit delivery-map entries for the shared module. Native semantic validation/import is reused; the transaction treats island text as opaque. Existing theme inheritance, forced colors, edits and accessibility remain acceptance requirements. Browser acceptance uses isolated iframes/contexts and preserves saved projects/drafts; generated screenshots and Vitest attachments remain ignored. Validation: all 11 shared allocation/admission control cases (including forced-GC reachability), all 732 runtime unit cases (75 files), all 56 checkpoint Chromium plays, all 55 selected native admission/inventory/memory/checkpoint contracts, the three real-WASM ownership/memory/checkpoint probes, all 52 CLI cases and the real CLI Chromium suite pass. Shared build, strict types, lint (zero errors; 529 warnings), clean package import/asset verification (331 files), CLI build and lint pass. Full Studio release acceptance passes all 52 tasks (24 cached), including 104 workbench browser cases, all 32 native schema-package examples, deployed two-project lifecycle/accessibility, Login/Supplier failure/repair, offline commands and clean consumers. Deterministic deployment has 201 declared assets and 296 files (sha256:3ce1ae1e51a1addcf64bf66ee6df152c1105cdeb5fc63a6eaaa019e94985b1da). Navigation passes all 26 tasks (24 cached), including URL/history, trees, CRUD, drafts, themes, responsive layout and offline Supplier/Markdown builds. Site passes all 27 tasks (20 cached), including determinism, asset admission/byte identity, production, search and interactive delivery. Theme inheritance and forced colors remain verified. Final implementation/fixture/map/contract hashes remain frozen; whitespace and generated-artifact exclusions pass. Exploratory missing-sealer fixture failure and the interrupted pre-buffer-release gate are excluded from final acceptance. Built-in DCE owner/source/widget restoration, renewed publication and automatic Studio recovery remain open.


Coordinated fresh-generation allocation/admission (2026-10-21): the reusable lifecycle extension was announced before implementation. Original physical participants and complete successful retirement receipts bind one bounded group (16 heaps, four MiB of independent checkpoint/journal bytes, a positive deadline of at most five seconds). Copies, omissions, duplicate peers, reused receipts, reentrant inspection and foreign/previous physical identities cannot admit a partial group. Owned fallback factories and dedicated rendering/browser/Node command workers initialize privately with native headroom guards; one synchronous host gate admits the complete set. Old aliases and the selected global fallback stay fenced. Abort and late-initializer cleanup preserve host bytes; explicit revocation rejects in-flight rendering requests before transport detachment, without fallback or replay. Only bound controls and weak identity history survive admission/revocation; candidate objects and strong identity sets are released. Forced-GC regression and a real owned-adapter diagnostic verify original and discarded memory reachability with host handles retained, without promising production GC/RSS timing. Same-realm vendored coordinator copies share original capabilities. Legacy/raw heaps remain unsupported; owner/source/widget restoration and automatic Studio recovery remain subsequent work. Site runtime maps now keep both helpers beside the native facade in search and interactive delivery, with topology/byte checks and a published repository documentation link.

Validation and CEM audit: all 8 allocation-control cases, 718 shared runtime unit cases (including 27 transport cases), 52 CLI cases, 55 checkpoint Chromium stories, real Chromium CLI coverage, 55 native inventory/memory/checkpoint contracts, three real-WASM probes and the owned-adapter reachability diagnostic pass. Strict types, runtime packaging and CLI lint pass; runtime lint reports zero errors and 526 warnings. Full Studio release acceptance passes all 52 tasks (28 cached), including 104 workbench cases, 32 native examples, isolated project lifecycle/accessibility, Login and Supplier failure/repair, offline commands and clean consumers. Deterministic deployment contains 200 assets/295 files (sha256: 0a42a38d5ffb7d1c9e041da33150cf7faf865e067d06ee5f682608c069cf571a). Navigation passes 26 tasks (24 cached); Site passes 27 tasks (20 cached), including production links, search, interaction, deterministic output and asset delivery. Shared lifecycle JavaScript stays in runtime/SDK support; no Studio/component UI JavaScript, CSS defaults or new public token families are introduced. Theme inheritance, forced colors, installed styles, edited controls, focus and selection remain verified. Saved projects/drafts are preserved through isolated acceptance contexts. Frozen implementation hashes, whitespace and generated-artifact exclusions are checked before staging.

Active owned fallback retirement and checkpoint/journal handoff (2026-10-21): the shared lifecycle extension was announced before implementation. Original complete embedding leases issue an opaque consent token, bound to the actual owned memory, ownership revision and original closed drain. The existing publication coordinator now retires that generation only with complete current client/source coverage and acknowledged checkpoint/terminal journal bytes. Each processing host registers its original synchronous engine cleanup adapter. A private cleanup window suppresses only covered resource deletions; unrelated mutations retain invalidation. Post-cleanup host callbacks run only under the original native zero-root freeze, with private ledger/consent and monotonic deadline checks repeated afterward. Native blockers, missing peers/adapters, copied evidence, newer input, changed ownership and expired cleanup/freeze deadlines cannot authorize release. The endpoint's callback scope is separate from initialization's memory/module locals. Success releases owned factory/domain references and fences aliases, retaining independent bounded checkpoint/journal bytes, stylesheet installations, edits, focus and selection. Failure after cleanup quarantines the managed generation and preserves last-good host bytes/DOM, without fallback, replay, replacement or a claim of synchronous GC/RSS reclamation. Legacy fallback remains explicitly unsupported. The next steps are fresh-generation coordination, semantic resume and declarative recovery status.

CEM audit: lifecycle behavior stays in reusable cem-elements, reusing the existing native census/freeze and processing-engine cleanup. Native transformation semantics, component/Studio UI composition, theme definitions and persistence are unchanged. No app-local UI JavaScript, CSS/token defaults or parallel state serializer is added. Real Chromium fixtures cover two fallback hosts/scopes, retained HTTP source policies, native stylesheet leases, portable checkpoint/journal bytes, unknown outcomes, native blockers, ownership changes and newer input while preserving legacy WASM independence. Callback, deadline and primitive-consent regressions were written failing-first; primitive pseudo-tokens (including zero with no embedding claims) now refuse before closing admission. Full release acceptance uses isolated contexts and preserves saved projects/drafts. Generated screenshots and Vitest attachments remain ignored. Validation: all 714 runtime unit cases (74 files), all 55 checkpoint Chromium plays, all 55 selected native admission/inventory/memory/checkpoint contracts and all three real-WASM ownership/memory/checkpoint probes pass. Runtime build, TypeScript, lint (zero errors; 525 warnings) and clean shared-package verification (323 files) pass. Full Studio release acceptance passes all 52 tasks (28 cached), including 104 workbench browser cases, all 32 native examples, the deployed two-project lifecycle/accessibility session, Login/Supplier failure/repair, offline commands and clean consumers. Deterministic deployment contains 196 declared assets and 291 files. Navigation passes all 26 tasks (24 cached), including URL/history, trees, CRUD, drafts, inherited themes, responsive layout and offline Supplier/Markdown builds. Site passes all 27 tasks (20 cached), including determinism, native asset delivery, production, search and interactive checks. Theme inheritance and forced colors remain verified. Final implementation hashes, whitespace and generated-artifact exclusion checks pass. Interrupted pre-primitive-fix release work and exploratory fixture failures are excluded from acceptance; the final frozen-source cohorts all exit zero. No replacement/resume or automatic Studio recovery is enabled.


Owned fallback generation isolation (2026-10-21): the new shared lifecycle capability was announced before implementation. An opt-in, selected before first shared native use, creates a private lexical wasm-bindgen factory with its own WASM memory, instance/module, views and finalizers. Shared render/query/XPath helpers route to that fixed instance; raw legacy WASM stays separate. The public controller exposes no raw memory or native permit. Complete original embedding leases and an empty managed client/resource domain are required for disposal. Copied/foreign/released/omitted/duplicate consent, concurrent disposal, initialization races, ownership changes during input inspection, drain or native freeze, reentrant disposal, and hidden native roots refuse release. Consent reads bounded own data slots without invoking caller array methods, iterators or accessors; failing-first fixtures reproduced overridden-method authority and getter/inspection side effects before repair. The original complete native census and RAII freeze permit guard synchronous owned-reference release; host memory/telemetry references are cleared too. Saved aliases and initializers cannot revive or retarget a retired instance, and failed initialization is terminal. Closure failures retain cleanup under closed admission. Release reports owned references only, without claiming synchronous browser GC or RSS reclamation. Active fallback checkpoint/journal retirement, replacement/resume and automatic recovery remain disabled; their transaction is the next item. The factory and its declarations ship in shared package/deployment assets. Site search acceptance exposed its omitted native facade; both Site runtime maps now ship the facade/factory, with explicit required-asset admission and existing byte-identity/production/search/interactive checks retained.

CEM audit: reusable imperative behavior remains in cem-elements and generated shared native bindings/build tooling. Native transformation semantics and the original retirement barrier are unchanged. Studio receives deployment map entries only; components and Studio UI remain declarative, without new UI JavaScript, token defaults, CSS or persistence changes. Chromium verifies preserved light DOM, edited form controls, focus and selection after empty disposal. Full acceptance uses isolated contexts to preserve saved projects/drafts. Generated screenshots, Vitest attachments and build outputs remain ignored. Validation: all 55 selected native admission/inventory/memory/checkpoint contracts, all three real-WASM ownership/memory/checkpoint probes, all 704 shared runtime unit cases (74 files), and all 54 checkpoint Chromium plays pass. Shared build, TypeScript, lint (zero errors; 522 warnings) and clean npm packaging (323 files, byte-identical native assets and self-contained declaration paths) pass. Full Studio release acceptance passes all 52 tasks (28 cached), including 104 workbench browser cases, the two-project lifecycle/accessibility session, Login/Supplier failure/repair, offline commands and clean consumers. Deterministic Studio deployment has 196 declared assets and 291 files. Navigation passes all 26 tasks (24 cached), including URL/history, trees, CRUD, drafts, inherited themes, responsive layout and offline Supplier/Markdown builds. After the Site-only asset repair, Site passes all 27 tasks (20 cached), including determinism, required native/factory asset admission, byte identity, production routes, search and interactive delivery. Theme inheritance and forced colors remain verified. Shared runtime sources stayed frozen through final acceptance; the subsequent Site-only map/verifier repair was independently frozen and verified. Final source hashes, whitespace and generated-artifact exclusion checks pass. Interrupted pre-hardening Studio and missing-asset Site runs are excluded from acceptance. Next: active owned-fallback checkpoint/journal retirement, coordinated replacement and semantic resume, then declarative recovery status with Studio integration.


Shared rendering retirement and fallback ownership boundary (2026-10-21): the shared lifecycle capability was announced before implementation. The publication coordinator now closes one actual pooled rendering worker, drains admitted work and stylesheet I/O, closes native admission, and retires only with complete original client/source/owner coverage and a current sealed checkpoint with terminal journal outcomes. It retains independent bounded host bytes before terminal cleanup of owned native render resources, reuses the existing complete native census/original RAII freeze permit, and rechecks current ownership and publication immediately before exact worker termination. Unmanaged native roots, missing/copied/stale witnesses, newer input, malformed confirmation, timeout and worker loss cannot authorize retirement. Failed preparation fences old handles and preserves host checkpoint bytes and edited DOM; explicit closure cannot migrate into fallback or replay work. Ordinary worker failures retain their established fallback behavior, verified by a browser regression failure and repair. Shared main-thread/fallback retirement rejects before changing admission or checkpoint state because the current shared module has no exclusive disposal boundary. Multi-generation publication domains refuse partial closure. No replacement/resume or automatic Studio recovery is enabled. Generated permit types stay private so package declarations remain self-contained. Implementation stays in shared cem-elements; Studio and component UI remain declarative, with no styling defaults or token changes. Validation passes all 55 selected native contracts, 694 unit cases, all 53 checkpoint browser plays, both real-WASM memory/checkpoint probes, shared build/TypeScript and lint (zero errors; 519 warnings), and the clean shared-package import check. Full Studio release acceptance passes 52 tasks (26 cached), including two-project lifecycle, Login/Supplier failure/repair, offline command delivery and clean consumers; deterministic deployment remains 194 assets/289 files. Navigation passes 26 tasks (24 cached), including URL/history, trees, CRUD, drafts, inherited themes, responsive layout and offline Supplier/Markdown builds. Site passes 27 tasks (20 cached), including production, search, interactive delivery and docs. Theme inheritance and forced colors remain verified. Isolated acceptance preserves saved projects/drafts; frozen implementation hashes, whitespace and generated-artifact exclusion checks pass. Next: exclusive fallback generation ownership/isolation, coordinated replacement and semantic resume, then declarative recovery status with Studio integration.

Command-worker retirement and checkpoint/journal handoff (2026-10-21): the shared browser lifecycle addition was announced before implementation. Shared Rust now prepares a non-cloneable RAII retirement permit only at a fresh complete closed zero-blocker inventory revision. Stale cleanup cancels preparation; the held permit freezes ordinary work and cleanup and remains visible as one live native-entries root. Its scalar confirmation cannot release it or grant replacement authority. Dropping the original permit advances the revision and restores cleanup under closed admission. Shared cem-elements checkpoint-store/publication handoffs require exact current sealed bytes, unchanged publication/policy/owners and terminal operation outcomes; pending/replaced captures, input, disposal and unknown effects refuse handoff without losing last-good exports. Existing explicit reconciliation and journal retirement policies are reused. Browser/Node command clients retain an independent opaque host byte copy (at most 1 MiB), require original complete embedding leases and the original coordinated witness across native preparation, then check handoff/ownership and request termination in one synchronous host turn. Changed owners/publications, raw same-count cleanup, copies, malformed confirmation, timeout and transport loss cannot revive authority; abort preserves cleanup, while uncertain abort fences recovery. Host checkpoint/journal bytes survive physical worker termination; native artifact tombstones disappear with the retired heap, and external actions are never replayed. Metadata retains false replacement authority. Strict ES2022 consumers exposed wasm-bindgen's disposable-symbol library dependency; both native packaging paths now declare it without weakening type checks. Native-first failure/repair fixtures and all 55 selected Rust cases pass; all 50 CLI cases, real Node/browser retirement and raw-alias checks, strict public declarations, CLI lint, 683 shared unit cases and shared runtime TypeScript pass. Rebuilt-WASM memory/headroom and independent committed-checkpoint probes pass, as does low-level package ABI/integrity verification. Full frozen-source Studio release acceptance passes 52 tasks (22 cached), including two-project lifecycle/accessibility, Login live view, Supplier failure/repair, offline execution and clean consumers. Deployment remains deterministic at 194 assets/289 files. Navigation passes 26 tasks (24 cached), including URL/history, trees, CRUD, drafts, themes, responsive layout and offline Supplier/Markdown builds. Site passes 27 tasks (20 cached), including production/search/interactive and docs delivery. Implementation stays in shared Rust, CLI and cem-elements; Studio remains declarative. Existing inherited themes/forced colors remain verified, isolated acceptance preserves saved projects/drafts, and generated screenshots/Vitest attachments remain ignored. Shared rendering/fallback retirement, generation replacement/semantic resume and declarative recovery status are the next steps.

Coordinated raw/native closure and revalidation (2026-10-21): shared Rust now guards every ordinary generated WASM entrypoint with bounded RAII admission, including synchronous work and async commands before the first poll. Explicit closure is monotonic for the exact linked memory. Original admitted work can settle; cancellation/disposal, existing resource settlement, cache release and bounded checkpoint controls remain available. Native entry permits supply the twenty-ninth inventory root. Full native validation rejects open/busy admission before registry collection, then checks idle closed state, a complete zero-blocker inventory and its exact current revision under one native admission lock. Shared browser/Node SDK close and revalidation APIs require current complete original embedding leases and original coordinated witnesses; monotonic host ownership receipts must remain unchanged across the native confirmation instant. Copies, changed owners, stale native replies, raw same-count cleanup mutations, timeouts and worker loss cannot revive evidence. Legacy managed/inventory-only capture behavior remains available. Worker synchronous currentness stays host-only; async revalidation checks otherwise invisible native changes. All witnesses retain managed-cli-only scope and false replacement authority. Actual retirement must consume fresh confirmation and hand off portable checkpoints/journals; initialization, direct memory mutation and unregistered/Rust-native callers remain explicit unsupported replacement boundaries. No Studio/component UI JavaScript, theme defaults, persistence migration, generation replacement/resume or action replay is introduced. Native 53-case, CLI 42-case, Chromium raw-alias/ownership/revision checks, strict public SDK declarations, low-level package verification, CLI lint and 680 shared unit cases pass. Rebuilt-WASM memory/headroom and independent committed-checkpoint probes pass. Full frozen-source Studio release acceptance passes 52 tasks (24 cached), including isolated two-project lifecycle/accessibility, Login live view, Supplier failure/repair, offline workers and clean-consumer packaging; deterministic deployment contains 194 assets/289 files. Navigation passes 26 tasks (24 cached), covering URL/history, trees, CRUD, drafts, themes, responsive layout and offline Supplier/Markdown site builds. Site passes 27 tasks (20 cached), including production, search, interactive delivery and documentation links. Theme inheritance/forced colors remain verified, and isolated acceptance preserves saved projects/drafts. Frozen source hashes, staged whitespace and artifact-exclusion checks pass.

Native inventories and embedding ownership (2026-10-21): the shared linked Rust runtime now supplies an exact 28-root counter-only inventory across command and reference registries, active bridges/reservations, callback hooks and caches. Native revisions conservatively fence same-count mutations, including retained interior state; async command admission enrolls before the first poll and retains its permit through callbacks. Live roots block inventory-backed acknowledgement while reconstructible cache entries and terminal artifact journals remain visible separately. Shared CLI APIs accept complete original opt-in embedding leases tied to actual WASM memory/Worker identity, with bounded owner/list admission, copy/foreign/omission/release refusal and asynchronous membership fences. Main authoring currentness re-reads the native revision; worker currentness covers parent activity/ownership and its native report remains point-in-time evidence. Default legacy capture behavior is preserved. Every proof keeps managed-cli-only scope and replacementAllowed false: unregistered raw callers, arbitrary initializers, coordinated raw closure, atomic fresh native revalidation and semantic resume remain outside this milestone. Studio receives only the shared module deployment mapping; no component/app UI JavaScript, theme defaults, persistence migration, heap replacement or action replay is added. Validation: 50 native inventory/memory/checkpoint tests, 8 low-level WASM-package cases, 37 CLI cases, real Chromium worker inventory/pressure/ownership checks, strict public API declarations, CLI lint and 680 shared runtime unit cases pass. Rebuilt-WASM headroom and independent-module committed-checkpoint probes pass. Full Studio release acceptance passes all 52 tasks (22 cached), including isolated two-project lifecycle/accessibility, Login live view, Supplier failure/repair, offline command delivery and clean consumer packaging; deployment contains 194 assets/289 files. Navigation passes all 26 tasks (24 cached), covering URL/history, model trees, CRUD, drafts, themes, responsive layout and offline Supplier/Markdown site builds. Site passes all 27 tasks (20 cached), including documentation links, production, search and interactive delivery. Theme inheritance/forced colors remain verified; saved projects and drafts are preserved through isolated acceptance contexts. Frozen source hashes and staged whitespace/artifact checks pass.

Separate command admission and checkpoint adapters (2026-10-21): shared CLI runtime support now wraps every shipped browser/Node command-authoring, execution, fallback and resumable-operation native entrypoint. Exact generated telemetry-function identity shares a ledger per linked module; workers and rendering remain independent domains. Native pressure or unavailable bounded telemetry permanently closes fresh admission. Asynchronous discovery/callbacks and command terminal publication stay enrolled; artifact counts and resumable hosts refuse capture until native disposal acknowledges them. Cleanup/cancellation and operation settlement remain available without pressure-triggered restart, fallback or replay. Bounded close/drain endpoints and original private host witnesses reject pending callbacks, retained handles, copies, foreign clients, cleanup mutations, uncertain results, overflow and timeout revival. Every witness declares managed-cli-only scope and replacementAllowed false: raw generated exports/initializers remain an explicit unmanaged boundary requiring authoritative inventories and embedding ownership before replacement. Studio receives shared runtime deployment assets only; its command consumer remains declarative. The next implementation step is native registry/embedding ownership admission, followed by coordinated replacement/resume (alert before introducing the shared browser lifecycle capability) and declarative recovery status.


Command-adapter validation: all 31 CLI tests pass, including 11 new admission/checkpoint cases, real Node pressure, held callbacks, partial/idempotent artifact cleanup, main-thread authoring pressure, duplicate module evaluation, bounded admission/waiters, automatic deadlines and stale/copied/foreign witnesses. Production Chromium passes the existing worker-pool/command matrix plus new client drain and real native-pressure/artifact cases, with no worker restart or replay. Strict public API declarations, package build/integrity verification, CLI lint and diff checks pass. All 680 shared-runtime unit cases and 45 native memory/checkpoint cases pass. The full Studio release gate passes all 52 tasks (31 cached), including 32 native examples, the complete deployed two-project lifecycle/accessibility session, online/offline worker commands, deterministic deployment (193 assets/288 files), themes/forced colors and clean-consumer packaging. Navigation passes all 26 tasks (24 cached), including URL/history, trees, CRUD, drafts and offline Supplier/Markdown-site failure/repair and independent copies. Site passes all 27 tasks (20 cached), including determinism, production, search and interactive delivery. Its production heading check caught a skipped heading in the new CLI documentation; the guide headings and published checkpoint link are corrected with assertions retained. Frozen runtime/fixture/module-map hashes remain unchanged through the accepted gates; all final processes exit zero. No Studio/component UI JavaScript, persistence migration, runtime replacement/resume or action replay is added. Isolated browser contexts preserve saved projects/drafts, and generated screenshots/Vitest attachments remain ignored and excluded.


Pressure-triggered native closure and bounded drains (2026-10-21): shared cem-elements samples the existing native memory-status DTO before/after asynchronous support work. Native soft/hard pressure or unavailable telemetry closes ordinary admission permanently for the actual worker/fallback generation; versioned worker notifications close every pooled host together. Fresh requests probe closure before host ownership/queue mutation; a helper refused between client enrollment and work admission always retires its transient client. Admitted queued/running/superseded work stays enrolled until completion or pressure refusal, without pressure-triggered fallback/replay. Direct XPath-library acquisition/reuse joins the same admission/drain boundary, and synchronous native source views/XSLT construction check closure. Native session/publication/document cleanup, stylesheet continuation/release, cancellation and disposal retain ordinary ownership and budget checks. Host stylesheet I/O remains a drain obligation between requests. Pressure starts an automatic five-second deadline; bounded explicit waits support cancellation, at most 16 waiters, and positive timeouts up to 60 seconds. Timeout permanently fences capture; late completion can clean up without reviving evidence. Transient helper clients enroll their pending lease atomically and retire it with their membership, avoiding redundant peer publication invalidations. Privately branded frozen drain witnesses bind to exact physical generation/revision, coalescing transient-client retirement before completion. Independent closed-coverage checks require current original source proofs and complete native client/resource/owner coverage; copied/duplicate/stale proofs and unknown retained state refuse the domain. Native control validation/sealing can still use bounded recovery headroom. Last-good checkpoint bytes never authorize rollback. Separate command-service/native heaps and generated-WASM bypasses remain explicit unsupported replacement boundaries, with their closed adapters next. No Studio/component UI JavaScript, theme defaults, persistence migration, runtime replacement/resume or action replay is introduced. Browser pressure acceptance uses disposable realms to preserve saved projects/drafts and avoid poisoning the runner's fallback module.


Pressure-closure validation: regressions first reproduced direct XPath-cache reuse after closure and a transient-client leak when pressure arrived between enrollment and admission; both now pass. All 45 selected native memory/checkpoint cases and real independent-module WASM memory/headroom/checkpoint probes pass. All 680 unit cases (73 files), runtime build, package typecheck, lint (warnings only) and diff checks pass. The final frozen-source browser cohort passes all 155 cases across 22 files, including production worker/fallback closure, cleanup of retained source sessions, independent complete coverage, preserved draft/focus/selection, capture under recovery headroom and refusal to roll older bytes back over a newer edit after closure. The complete Studio release gate passes all 52 tasks (28 cached), including the deployed two-project lifecycle/accessibility session, 32 native examples, deterministic deployment (191 assets/286 files), native worker commands online/offline, theme/forced-color/offline checks and clean-consumer packaging. Navigation passes all 26 tasks (24 cached), including URL/history, model trees, CRUD, drafts and offline Supplier/Markdown-site builds. Site passes all 27 tasks (20 cached), including determinism, production/search and interactive delivery. Final processes exit zero and source hashes remain unchanged. Existing active-suggestions cases exceeded their 30-second limit during concurrent exploratory gates; unchanged HEAD cases also take 25–27 seconds. The accepted sequential run retains their assertions and timeouts. Intermediate diagnostic/A-B sources and interrupted exploratory runs are excluded; all production edits are restored and verified. No runtime replacement/resume or action replay is enabled. Saved projects/drafts remain untouched in isolated contexts; generated screenshots and Vitest attachments remain ignored and excluded.


Physical runtime generations and native client coverage (2026-10-20): shared cem-elements issues private client witnesses for actual Worker objects and the initialized fallback WASM memory. Pooled roots share a generation; all fallback roots share their actual module generation across documents/pools. Worker loss revokes coverage before source callbacks, and moving a client to fallback requires a new owner publication. Native job results/session leases pin their execution generation; old-worker cleanup and retired models cannot poison or impersonate the fallback generation. A realm-wide weak ownership fence refuses duplicate framework ledgers claiming the same Worker/memory. Coordinator labels remain logical session identities. Every processing host, live native consumer and standalone asynchronous support/query call participates, including host-only work. Pending stylesheet I/O, queued/running jobs, superseded work and cleanup remain barriers. Original session and HTTP-document capsules supply exact private resource witnesses; a copied marker or matching instance ID cannot cover hidden native state. Source hosts join the physical coverage through exported source capsules, with shared owners allowed across multiple clients/heaps. Binding revocation retires consumers only after their in-flight work settles. Direct retained native templates/documents/sessions, cold XSLT wrappers and XPath libraries remain blockers until adopted or disposed. Native cleanup intent survives queued/disposed races until release acknowledges it; actual uncertain cleanup revokes coverage, and duplicate disposal observes the same completion. Inactive local source intent carries the retained session's own capsule witness after preparation; unused models cannot hide behind XML intent. No omitted client or partial owner subset can be sealed as complete. Native cem-ml validates the bounded inert runtime/client/owner receipt; all source/model semantics remain native. Source-witness bookkeeping reuses current owner inspection without caching semantic admission. No component/Studio UI JavaScript, theme defaults, persistence migration, runtime replacement/resume or action replay is introduced; isolated acceptance preserves saved projects/drafts. Independently instantiated native command heaps and generated-WASM bypasses require their own closed client boundaries before replacement.


Physical-generation validation: the native-first receipt fixture rejected the new format before implementation. All 55 selected native checkpoint/import cases and 665 unit cases pass, including actual pooled-client coverage, immutable host document admission, copied witnesses, duplicate physical ledgers, pending host-only stylesheet I/O, stale asynchronous seals, failed cleanup and native result acknowledgement after worker loss. Three production Chromium plays cover actual worker/fallback client membership, opted-out consumers, source/document models claiming existing instance IDs, direct retained resources, worker-generation transitions, native-source loss/reacquisition and healthy shared fallback coverage after old-worker cleanup. The final frozen-source 16-file browser cohort passes all 124 cases; all 25 typed-control and DOM/pure-render reuse cases pass, with existing assertions retained. Real independent-module WASM validates complete inert coverage and original source reimport. Builds, typechecking, lint (warnings only), focused Rustfmt and diff checks pass. Final Studio release verification passes all 52 tasks (25 cached), including 104 workbench cases, the complete deployed two-project lifecycle/accessibility session, 32 native examples, deterministic deployment (191 assets/286 files), worker commands online/offline, theme/forced-color/offline checks and clean-consumer packaging. Navigation passes all 26 tasks (24 cached), including URL/history, model trees, CRUD, drafts, themes, responsive layout and offline Supplier/Markdown-site failure/repair and independent copies. Site passes all 27 tasks (20 cached), including deterministic deployment, production/search and interactive delivery. All final processes exit zero and source hashes remain unchanged. Exploratory/interrupted runs while tightening capsule, disposal, duplicate-ledger and generation-acknowledgement guards are excluded from acceptance. Saved projects/drafts remain untouched in isolated contexts; screenshots and Vitest attachments remain ignored and excluded.


Custom source policies and cross-runtime checkpoint endpoints (2026-10-20): native cem-ml validates optional per-source depth/work and unresolved-reference overrides while retaining omission versus explicit defaults. Bounds use positive WASM-width integers; only standard unresolved dispositions are portable, and XML import overrides remain unsupported. Source bytes, lexical metadata, preparation inputs and directed crossing intent remain complete and require fresh admission. Shared cem-elements copies policies before asynchronous preparation and transports opaque CEMB/CEMV without interpreting native trees. Separate CemElementRuntime instances can now resolve endpoints through an explicitly shared coordinator's exact host registrations. Private renderer node/instance witnesses reject copied markers and moved nodes; foreign/unregistered boundaries and disposed owners remain fenced. Provider/delegate, placement and geometry/focus receipts still require complete matching owner publications. Settled acknowledgement retries handle peer-publication races, including manual relationships, without recursively inspecting endpoint owners or renewing old frames automatically after policy changes. Stale seals preserve last-good bytes; revocation erases them. Physical runtime discovery, all-native-client coverage and replacement/resume remain separate. No component/Studio UI JavaScript, theme defaults, persistence migration or action replay is introduced; acceptance uses isolated contexts and preserves user projects/drafts.

Source-policy/endpoint validation: the native-first policy fixture failed at the unsupported format boundary before implementation. Unit regressions reproduced missed peer acknowledgements and policy-revocation renewal before the retry guards were fixed. All 54 selected native checkpoint/import cases, 651 unit cases, runtime/WASM builds, typechecking, lint (warnings only), focused Rustfmt and diff checks pass. Six new browser plays exercise worker/fallback clients, exact custom policies, immutable preparation, provider/delegate and placement ownership, geometry/focus, preserved edits/caret, copied markers, moved nodes, duplicate hosts, foreign domains, disposal, revocation and stale seals. The independent-module WASM probe validates exact policy/source bytes and complete dependency receipts, reimports original CEMB and rejects invalid policy/authority/stale records. The final frozen-source 16-file Chromium cohort passes 119/121 cases; only the two documented suggestions-controller failures remain (trusted ancestor visibility and missing committed native row map), with assertions unchanged. All 25 typed-control and DOM/pure-render reuse cases pass. An early release attempt stopped for revocation hardening, and browser setup against its incomplete generated package, are excluded from acceptance; the native package was rebuilt before the final browser/release runs. Final Studio verification passes all 52 tasks (18 cached), including 104 workbench and 30 live-preview cases, native examples, deterministic deployment (190 assets/285 files), the complete deployed two-project lifecycle/accessibility session, theme/forced-color/offline checks, worker commands and clean-consumer packaging. Navigation passes all 26 tasks (24 cached), including URL/history, model trees, CRUD, drafts and offline Supplier/Markdown-site failure/repair and independent copies. Site passes all 27 tasks (20 cached), including deterministic output (269 files), production content/search and interactive delivery. All three final processes exit zero. Source hashes remain unchanged throughout the final checks; generated screenshots and Vitest attachments remain ignored and excluded.


Portable retained native attribute/source-value capsules (2026-10-20): reusable lifecycle JavaScript remains in shared cem-elements; native cem-ml owns CEMV decoding, typed contracts, source provenance, integrity/hash/binding checks and privacy/budget admission. Retained renderer output carries one shared artifact table and per-attribute root indices. Original native bytes and materialized source graphs survive current template/reference-source changes; original CEMB inputs remain in the separate owner source contract. One bounded weak byte witness per published buffer prevents an unchanged projected string or claimed hash from masking mutation. Mount/export compare shared buffers once per pass; no native tree is decoded in JavaScript. Retained raw and produced-host input capsules keep their original witnesses, while nested owners retain independent models, controls and receipt endpoints. Unsupported metadata replacement, corruption, sensitive values, credential provenance, private authority, unused/duplicate artifacts and aggregate overflow refuse the entire domain and preserve last-good bytes. Ordinary disposal/rematerialization reacquires current inputs. Executable/live references, native contexts, unsupported bindings/custom source policies and cross-runtime endpoints remain fenced. No component/Studio UI JavaScript, CSS/theme defaults, persistence migration, automatic restart/resume or action replay is introduced. Saved projects and drafts stay untouched in isolated acceptance contexts; screenshots and Vitest attachments remain ignored.

Native context/preparation validation: the native-first positive fixture failed at the unsupported context/preparation boundary; a separate native fixture reproduced missing encoded-overhead and combined-binding byte limits before their repair. All 51 selected checkpoint/import cases, 650 unit cases, runtime/WASM builds, typechecking, lint (warnings only), focused Rustfmt and diff checks pass. Five new worker/fallback browser plays cover immutable asynchronous input copies, original control frames and limits, shared attribute/wrapped/raw-slice artifacts, fresh-session rendering, changed/corrupted context refusal, preserved edits/caret/focus, stale seals, revocation and disposal. The independent-module real-WASM probe validates/imports native context and preparation capsules and rejects encoded-limit violations. The final frozen-source, rebuilt-WASM 16-file Chromium cohort passes 113/115 cases; only the two documented suggestions-controller failures remain (trusted ancestor visibility and missing committed native row map), with assertions unchanged. All 25 typed-control and DOM/pure-render reuse cases pass. An initial full release attempt was interrupted to harden encoded byte limits and is excluded from acceptance. Final Studio release verification passes all 52 tasks (18 cached), including 104 workbench and 30 live-preview cases, native examples, deterministic deployment (190 assets/285 files), the deployed two-project lifecycle/accessibility session, theme/forced-color/offline acceptance, online/offline worker commands and clean-consumer packaging. Navigation passes all 26 tasks (24 cached), including URL/history, model trees, CRUD, drafts and offline Supplier/Markdown-site failure/repair and independent copies. Site passes all 27 tasks (20 cached), including determinism (269 files), production content/search and interactive delivery. All three final processes exit zero. Generated screenshots and Vitest attachments remain ignored and excluded.

Retained native capsule validation: the native-first positive fixture failed at the unsupported projection-envelope boundary before implementation. All 47 selected checkpoint/import cases, 646 unit cases, runtime build/typecheck/lint (existing warnings), worker/fallback native capsule plays with independent nested owners and the independent-module real-WASM typed-value/source import probe pass. The final frozen-source, rebuilt-WASM 16-file Chromium cohort passes 108/110 cases; only the two documented suggestions-controller failures remain (trusted ancestor visibility and missing committed native row map), with assertions unchanged. An initial active-suggestions capture timeout does not recur in the final serial run. All 25 typed input/select and DOM/pure-render reuse cases pass. Full Studio release verification passes all 52 tasks (17 cached): 104 workbench and 30 live-preview cases, 32 native schema-package examples, deterministic deployment (190 assets/285 files), deployed two-project lifecycle/accessibility, theme/forced-color/offline acceptance, online/offline worker commands and clean-consumer packaging. Navigation passes all 26 tasks (24 cached), including URL/history, model trees, CRUD, drafts, responsive themes and offline Supplier/Markdown-site builds. Site passes all 27 tasks (20 cached), including deterministic output (269 files), production content/search and interactive delivery. All three processes exit zero. Focused Rustfmt and diff checks pass; generated screenshots and Vitest attachments remain ignored and excluded.


Explicit submenu geometry and focus policies (2026-10-20): shared cem-elements now captures invoker/viewport/auto/none or current owned target endpoints, inherited boundary ownership, close/freeze anchor-loss intent and an unconditional recompute-on-resume policy. Native cem-ml validates complete role contracts, endpoints and bounded identities and refuses unknown modes, private authority and pixel restoration. Logical parent/launcher/panel proofs use the private composite render identity; host policy proofs keep explicit anchor/boundary/entry-focus/return-focus dependencies distinct. Current native typed and scoped references retain their original source and acknowledged owner revisions. Read-only capture compares the last successful placement against current anchor/boundary/panel/viewport/containing-block geometry, direction, ownership, style claims and pending observer work. It never fits, focuses or opens during export. Frozen stale output, missing/ambiguous/unowned targets, disabled/inert focus targets, foreign geometry owners and mutation during async sealing refuse the complete domain and preserve last-good bytes. Repaired ordinary placement reacquires eligibility. Closed targets remain structural intent pending fresh visibility/focus admission. No component/Studio UI JavaScript, CSS/theme defaults, persisted project migration, automatic runtime restart or action replay is introduced; saved projects/drafts remain untouched in isolated acceptance contexts.

Explicit submenu focused validation: the native-first fixture initially failed at the unsupported policy boundary; all 44 checkpoint/import cases now pass. Shared runtime build and TypeScript checks, lint (existing warnings), all 646 unit cases, six worker/fallback submenu browser plays and independent-module real-WASM policy portability pass. The new plays cover inherited and native typed references, open/closed and none policies, exact dependency receipts, preserved edits/caret/focus, anchor/boundary/style invalidation, foreign ownership, stale seals, revocation and disposal. The serial 16-file Chromium cohort passes 106/108 cases; only the two previously documented suggestions-controller failures remain (trusted ancestor visibility and missing committed native row map), with assertions unchanged. All 21 isolated production menu/dropdown/theme-switch and renderer-reuse cases pass. An initial 19-file parallel run also hit two resource timeouts and a nested-dropdown focus failure; those cases pass serially/in isolation, without source or assertion changes. Final full Studio release verification passes all 52 tasks (17 cached), including 104 workbench cases, 30 live-preview cases, native example validation, deterministic deployment (190 assets/285 files), the deployed two-project lifecycle/accessibility session, theme/forced-color/offline acceptance, online/offline worker commands and clean-consumer packaging. Navigation passes all 26 tasks (24 cached), including URL/history, model trees, CRUD, drafts, responsive themes and offline Supplier/Markdown-site builds. Site passes all 27 tasks (20 cached), including deterministic output (269 files), production content/search and interactive delivery. All three processes exit zero. Generated screenshots and Vitest attachments stay ignored and excluded.


Standalone datalist ownership (2026-10-20): reusable lifecycle JavaScript was announced and stays in shared cem-elements; strict original-source/endpoint validation stays in native cem-ml. An opted-in standalone attachment owns its exact provider lease, subscriptions and target placement, while capture reads the actual committed native binding and private renderer attribute/text/topology witnesses. The portable resource record holds original bounded XML/CEMB, editor-host/target endpoints, provider revision and attached/unavailable reset/reacquisition intent. Private generated IDs, native handles, callbacks and picker/row-selection authority are excluded. Unchanged native option DOM can retain its original render revision while the current committed binding proves freshness; source replacement reacquires fresh authority without losing field edits/caret. Focused/composing editors, pending/revoked/opaque sources, foreign or authored relationships, competing leases/listeners and mutated or replaced targets (including native attribute capsules on the root or options) refuse the complete domain and preserve last-good bytes. Failed setup and disposal release exact accounting; generic owners join revision-neutral settled publication. No component/Studio UI JavaScript, theme/CSS changes, automatic capture/restart, storage migration or action replay is introduced. Saved projects and drafts remain untouched.

Standalone datalist validation: the native-first ownership fixture failed before implementation, then the final 42 checkpoint/import cases passed. Build, lint (existing warnings only), 646 unit cases, both new worker/fallback datalist plays, 10 renderer reuse cases and the independent-module real-WASM portability probe passed. The 16-file Chromium cohort passed 102/104 cases; only the two previously documented suggestions-controller failures remain (trusted ancestor visibility and missing committed row map), with assertions unchanged. Full Studio release passed all 52 tasks (39 cached), including 104 workbench cases, 30 live-preview cases, 32 native examples, deterministic deployment (190 assets/285 files), two-project lifecycle/accessibility, online/offline worker delivery and clean-consumer packaging. Navigation passed all 26 tasks (24 cached). Site passed all 27 verification tasks (20 cached), including production links, search, interactive delivery and deterministic output (269 files). Deployment inventories now include the shared adapter in both consumers; Site’s nine older missing checkpoint/preview/stylesheet declarations, one unpublished local documentation target and eight stale component-search heading references were repaired without weakening verification. Generated screenshots and Vitest attachments remain ignored and excluded.

Original retained projections and nested/dynamic bodies (2026-10-20): shared lifecycle JavaScript was announced and stays in cem-elements; native cem-ml validates a named flat renderer-output checkpoint transport. Immutable original attributes/text/topology and instance/artifact/revision/policy identity come from private committed renderer records at mounting, not DOM serialization or authored input reconstruction. Original bodies survive later template changes, including conditional removal and empty output, with separate current template inputs. A reuseOnOpen flag retains the original/current template-node relationship so ordinary opening still replaces an obsolete template attachment. Native validation bounds entries/depth/attributes, ordered roots, unique IDs, dynamic marker pairing, owner endpoints and the aggregate original-source/projection budget. Opaque native capsules, password/file types and sensitive autofill are refused even in historical original output. Nested produced hosts remain independent model/control/focus/body owners and join the complete current receipt graph; parent inspection skips their owned DOM. Missing/pending/composing/foreign owners, counterfeit hosts/markers and mutated raw projections still refuse the whole domain. Renderer targeted fragment topology and replacement bookkeeping now tracks current template records, and derived cloning preserves private markers, text, child records and immutable provenance. No component/Studio UI JavaScript, theme/CSS changes, persistence migration, automatic capture/restart or action replay is introduced. Saved projects and drafts are untouched.

Retained/nested focused verification: new native cases were observed failing before implementation (projection shape, origin, reuse policy and private output exclusions), then all 41 focused native checkpoint/import cases passed. Build, 646 unit cases (70 files), lint (warnings only) and independent-module real-WASM checkpoint validation pass. The final 16-file Chromium cohort reports 100 passes, including all 31 checkpoint and two native-session cases, and the same two documented suggestions-controller failures (ProductionFieldsExternalSourcesAndTrustedAncestorResume and PlatformCloseCancelsPendingAndPreservesEditorRouting). Ten additional renderer metadata/direct-patch/focus/identity cases pass. New worker/fallback fixtures cover independent nested fields/dialogs, retained edits/selection/focus through conditional removal, composing/pending refusal, missing/counterfeit hosts, revision dependencies, sealing races and teardown. A native renderer/range transport fixture verifies original output after current changes, caller mutation isolation, private marker corruption, template replacement/reuse disposition, disposal/reopen and historical private-input refusal. Existing baseline assertions are unchanged. An ordinary-opening regression with authored owner comments was observed failing and repaired: unaccounted comments refuse capture without changing normal surface validity/materialization. The final 16-file cohort and unit/lint checks were repeated after that repair with the same pass/baseline counts. The completed final 52-task Studio release gate passes (28 valid cached dependencies), including all 104 workbench cases, deployed two-project lifecycle/accessibility, 32 native schema examples, online/offline CLI/WASM workers, deterministic deployment (189 declared assets, 284 files) and clean package-consumer acceptance. The dedicated 26-task navigation gate passes (24 valid cached dependencies): URL/history, model trees, CRUD, draft guards, themes/responsive layout and offline Supplier/Markdown builds, including saved edits, failure/repair and portable independent copies. Generated screenshots and Vitest attachments remain ignored and excluded.

Standalone delegates and deferred bodies (2026-10-20): reusable JavaScript was announced before implementation and stays in shared cem-elements, with strict scalar/identity validation in native cem-ml. A standalone manual-listbox lifecycle opts in explicitly; private exact lease ownership, tagged subscriptions, current provider revision and panel/editor dependency receipts support inert reset/reacquisition intent. The generic declarative owner now participates in the shared revision-neutral interaction-settled publication path. Raw delegates without opt-in and direct unaccounted datalist claims still block the whole domain. Native deferred-body contracts retain current template/root identities, retain/dispose mode and materialization state. Private committed renderer attribute, text and child records provide equality evidence even after targeted native patches invalidate the full-plan reuse cache. Clone identity and native capsules remain owned by the original projection. Current raw bodies retain input edits, selection and focus across capture/unrelated rerenders; never-materialized, empty and disposed bodies are explicit. Foreign/template mutations, stale retained output after source changes, untracked/dynamic boundaries, nested produced components and contenteditable bodies remain refused. Capture never disposes or resets edits to repair a stale body; ordinary dispose/reopen supplies a fresh admitted projection. No component/Studio UI JavaScript, theme/CSS changes, persistence migration, automatic capture/restart or action replay is introduced. Saved projects and drafts remain untouched.

Standalone/body focused verification: the new native contract fixture failed before implementation, then all 40 focused native checkpoint/import cases passed. Build, 646 unit cases (70 files), lint (existing warnings) and real-WASM checkpoint validation pass. The final 15-file Chromium cohort reports 92 passes, including all 29 checkpoint and two native-session cases, and the same two previously documented suggestions-controller failures (ProductionFieldsExternalSourcesAndTrustedAncestorResume and PlatformCloseCancelsPendingAndPreservesEditorRouting). New worker/fallback plays cover standalone opt-in/teardown, foreign leases, pending attempts, revoked lifecycle and async sealing, edited retained controls, unchanged rerenders/focus, forged inert templates, foreign body mutations, stale template refusal, disposal/rematerialization, empty bodies and mutation during async sealing with last-good bytes preserved. Baseline failing assertions remain unchanged. The complete 52-task Studio release gate passes (16 valid cached dependencies), including all 104 workbench cases, deployed two-project lifecycle/accessibility acceptance, 32 native schema examples, online/offline CLI/WASM workers, deterministic deployment (189 declared assets, 284 files) and clean package-consumer verification. The dedicated 26-task navigation gate passes (24 valid cached dependencies): URL/history, model trees, CRUD, draft guards, themes/responsive layout and offline Supplier/Markdown builds, including saved edits, failure/repair and portable independent copies. Generated screenshots and Vitest attachments remain ignored and excluded.

External providers and native publication consumers (2026-10-20): shared cem-elements now captures opt-in host-admitted native listbox frames, including frames alongside stateful suggestions, with original XML/CEMB, query/publication configuration and exact native ordinal/render-node identities. cem-ml validates these inert contracts against the same aggregate source budget as other resources. External suggestions use production declarations and independently owned scoped form providers; source-intent export returns a fresh DTO without querying or retrying. Fresh imports require new native identities and authority. Forged row markers, pending queries, revoked or opaque source inputs, changed admission during sealing and foreign leases refuse the complete domain and preserve last-good bytes. Raw standalone delegates and unattached datalist publications remain unsupported. Shared runtime/native modules contain the implementation; no Studio/component UI JavaScript, automatic recovery, storage migration or action replay is introduced. Saved projects and drafts are untouched.

External-adapter focused verification: the new native fixture failed before implementation, then all 39 focused native checkpoint/import cases passed. All 646 unit cases across 70 files, build, lint (existing warnings) and the real-WASM independent-module checkpoint script pass. The final 15-file Chromium cohort has 90 passes, including all 27 checkpoint and two native-session cases in worker/fallback modes, and the same two previously documented suggestions-controller baseline failures (ProductionFieldsExternalSourcesAndTrustedAncestorResume and PlatformCloseCancelsPendingAndPreservesEditorRouting). Existing failing assertions remain unchanged. The complete 52-task Studio release gate passes (17 valid cached dependencies), including all 104 workbench cases, deployed two-project lifecycle/accessibility, 32 native schema examples, online/offline CLI/WASM worker execution, deterministic deployment (189 declared assets, 284 files) and clean package-consumer acceptance. The dedicated 26-task navigation gate passes (24 valid cached dependencies): URL/history, model trees, CRUD, draft guards, themes/responsive layout and offline Supplier/Markdown builds, including saved edits, failure/repair and portable independent copies. Generated screenshots and Vitest attachments remain ignored and excluded.

Inactive/native-datalist adapters (2026-10-20): new lifecycle JavaScript was announced before implementation and remains in shared cem-elements, with strict native source/lifecycle validation in cem-ml. Separate inactive contracts retain original XML/selectors/labels, profile, provider/query revisions, query and feedback intent for settled idle/failed/unavailable states, with zero row count and no obsolete row/proof fields. Native datalist contracts retain original source, exact provider revision, target render-node identity, attached/unavailable claim and reset-on-resume presentation intent. The exact shared lease now accounts for the placement coordinator subscription as well as attachment subscriptions. Native failed sources release claims/subscriptions; source intent is captured without retrying unchanged invalid options, and source repair acquires fresh native ownership. Provider accounting release is idempotent. Native picker state cannot be observed, so a focused native-datalist editor conservatively blocks the whole domain. Pending queries/preparation/readiness, composition, foreign claims and revoked host policy remain blocking. Eligible native-session inputs and execution receive independent copies before asynchronous submission; mutating caller buffers/selectors during a pending job changes neither native output nor checkpoint provenance. No app/component UI JavaScript, new public attributes, CSS/token defaults, storage migration, action replay, automatic Studio capture or runtime replacement is introduced. Saved projects/drafts remain untouched.

Adapter focused verification: the new native fixture first failed before implementation. All 36 native cases, 646 unit cases across 70 files, 21 checkpoint Chromium stories and 8 related source/datalist/placement browser cases pass. Worker/fallback tests cover inactive provider transitions, nonempty owned validity, foreign subscriptions/validity owners, failed source/query preservation, composition and pending refusal, exact original bytes during asynchronous preparation, text/email/number edits, readonly and focused picker fences, source mutation, invalid-option failure/repair, authored list conflicts and policy revocation. Equal-byte replacement during a failed source handover is rejected as a stale asynchronous export and preserves the last-good bytes; changed source input is refused before export. The existing publication barrier supplies both fences, without a new runtime workaround. Shared build/lint and independent-module real-WASM checks pass. The broader 26-file cohort reports 182 passes and the same two previously recorded suggestions-controller baseline failures; no additional failure appears. The complete final 52-task Studio release gate passes with serialized scheduling (37 valid dependency results reused), including all 104 workbench cases, 32 schema examples, Login/Supplier edit/failure/repair/Parse/Inspect/graph/provenance/live-view cycles, two-project lifecycle and accessibility, online/offline CLI workers, theme inheritance/forced colors and clean package-consumer verification. Deployment and determinism retain 189 assets and 284 files. The initial run was intentionally interrupted to add the handover regression assertion; native/package/workbench checks already completed there were reused validly. The dedicated 26-task deployed navigation gate also passes (24 valid dependency results reused): URL/history, model/physical trees, folder listings, Create/Clone/Delete, draft guards, themes/responsive layout and Supplier/Markdown portable/offline builds. Generated screenshots and Vitest attachments remain ignored and excluded.

Retained inactive proofs and settled menu handoffs (2026-10-20): shared cem-elements keeps exact original native source provenance on private source identities and exports only bounded ordinal/value revalidation intent. Read-only and repaired ready states preserve a valid required-selection proof without serializing handles or leases. The production fixture exposed that re-enabling a previously prepared editor could claim an idle publication; currentness now requires ready feedback and a fresh publication. Source mutation revokes the proof and restores required-selection validity. Read-only inputs with customError but no observable validation message still refuse the whole domain; re-enabling exposes the exact message and permits capture after repair. Settled submenu parent/launcher/open-panel endpoints join complete current owner/revision receipts; completed dialog handoffs retain source, logical parent and return destination without replaying the command. Queued leaf activations, held gestures, missing endpoints, preparation and explicit menu geometry/focus policies remain fenced. Native cem-ml owns scalar/source validation and ordinary XML/CEMB import. No component/Studio UI JavaScript, theme/CSS changes, persistence migration, automatic capture/restart or action replay is introduced. Existing saved projects and drafts are untouched.

Verification for retained proofs/menu handoffs: both new native contract fixtures were observed failing first, then all 38 focused native checkpoint/import tests passed. All 646 shared unit tests (70 files), build, lint (existing warnings) and the real-WASM checkpoint script pass. Twenty-five focused Chromium checkpoint/native-session cases pass in worker/fallback modes. The broader 12-file surface/menu/datalist/suggestions run, including the final last-good-byte assertions, has 77 passes and only the two previously documented suggestions baseline failures (ProductionFieldsExternalSourcesAndTrustedAncestorResume and PlatformCloseCancelsPendingAndPreservesEditorRouting); their assertions remain unchanged. The complete 52-task Studio release gate passes (17 valid cached dependencies), including all 104 workbench browser tests, deployed long-session two-project lifecycle/accessibility acceptance, 32 native examples, online/offline worker execution and clean package-consumer acceptance. Deterministic deployment has 189 declared assets and 284 files. The dedicated 26-task navigation gate passes (24 valid cached dependencies): URL/history, model trees, Create/Clone/Delete, draft guards, themes/responsive layout and offline Supplier/Markdown builds, including saved edits, failure/repair and portable independent copies. Generated screenshots and Vitest attachments remain ignored.

Active interaction checkpoint capture (2026-10-20): reusable lifecycle JavaScript remains entirely in cem-elements, with strict scalar/source validation and XML/CEMB import in cem-ml. Ready listbox suggestions and exactly accounted form providers retain query/publication configuration, source-order active/committed identities, row proof intent, scroll/visibility, provider revision/validity and terminal duplicate-change suppression. Committed foreign placements retain immutable invocation source bytes, bindings, routes and exact current owner/revision/node dependencies. Passive reference inputs are also captured. Settled open/closed popup and dialog state retains invocation/return-focus intent, modality and return values. All authority is reacquired on future semantic resume; leases, grants, handles and proof objects are never serialized. Unknown claims/subscriptions, pending queries, gestures/composition, stale endpoints, prepared transactions, retained bodies and unsupported source/widget states still refuse the complete domain. Revision-neutral inspection and settled adapter acknowledgements preserve the existing source/placement publication without causing a new render. Studio deployment maps include the shared runtime module; no app/component UI JavaScript, CSS/token changes, storage migration, action replay, automatic Studio capture or runtime replacement is introduced. Saved projects and drafts remain untouched.

Active interaction verification: 34 native cases, all 645 unit cases across 70 files, all 17 checkpoint Chromium stories and the independent-module real-WASM checkpoint script pass. Worker/fallback fixtures cover equal-value source-order proofs, fresh native source admission, exact provider claims, pending queries, composition/gestures, placement revocation/forged markers, asynchronous dependency changes, settled open surfaces and closed return values. Source snapshot fixtures prove later caller byte/selector mutations cannot rewrite captured invocation provenance. Shared runtime build and lint pass; production XHTML imports are explicitly fixture inputs rather than runtime dependencies. The 25-file broader Chromium cohort reports 174 passes and two failures in suggestions-controller (trusted-ancestor visibility and platform-close row publication); both failures reproduce with the exact committed HEAD JavaScript runtime sources, unchanged assertions and an isolated rerun. These existing failures remain tracked separately from checkpoint acceptance.

The complete final 52-task Studio release gate passes with serialized scheduling (28 valid dependency results reused), including all 104 workbench cases, 32 native schema examples, repeated Login/Supplier editing, validation failure/repair, persisted Parse/Inspect, graph edits/undo/redo, provenance/live views, online/offline workers, two-project lifecycle, accessibility, themes/forced colors and clean package-consumer verification. A preceding parallel aggregate run timed out once in the graph-creation/reset workbench case; the preceding 104-case run and final isolated 104-case run pass with unchanged assertions and deadlines. No cause or runtime fix is attributed to that timeout. The release verifier caught a missing new shared-module deployment edge; both explicit maps now include it, and deployment/determinism verify 189 assets and 284 files. The final 26-task navigation gate also passes (23 valid dependency results reused): URL/history, model/physical trees, folder listings, Create/Clone/Delete, draft guards, themes/responsive layout and Supplier/Markdown portable/offline builds. Generated screenshots and Vitest attachments remain ignored and excluded.

Non-form checkpoint contracts (2026-10-19): shared lifecycle JavaScript was announced before implementation and remains in cem-elements. The native runtime control envelope now validates stable tree roving/typeahead state, tab focus/count, closed-menu active-node/typeahead state, navigation and owner-local host/render-node focus. Canonical attributes/slices retain selection and expansion; caches, generated IDs and listeners remain derived. Declarative cem-grid uses ordinary payload/control/focus capture rather than inventing an ARIA grid controller. Popup capture requires an internal trigger and closed surface. Native surface capture requires consumed default initialization and a closed settled controller; open/preparing/closing sessions, hover/exit timers, invocation/return-focus/task handoffs, nonempty dialog return values and retained deferred-body projections block the complete domain. Keyboard and surface lifecycle events join the existing input/focus publication fences. Checkpoint-only generic hydration remains refused.

Native acknowledged receipts now accept bounded inert editor-provider/placement dependency intents with owner/revision/node endpoints and required reacquisition disposition. Sealing checks complete current owner membership, unique routes and exclusive editor consumers, supported relationship properties and strict fields; obsolete tokens, sessions and grants are not portable. Trusted-host registrations are copied, fenced on endpoint revision/currentness and runtime/generation/policy authority, and checked again after asynchronous export/sealing. Owner removal blocks dependencies; policy updates revoke intent until explicit removal/replacement. These contracts do not override active suggestions/provider-lease or foreign-placement exporter refusal. Complete private source/query/selection/proof adapters, fresh grant acquisition/revocation and transient-session state remain separate follow-up work. No component/app UI JavaScript, CSS/token defaults, storage migration, action replay, actual runtime replacement or automatic Studio capture is introduced. Saved projects/drafts remain untouched and generated screenshots/Vitest attachments stay ignored.

Non-form contract focused verification: 30 native cases pass, including strict non-form/focus state and complete relationship receipts, stale/absent endpoints, conflicting routes, unknown authority fields and bounds. All 642 shared unit tests pass, including policy/endpoint removal and asynchronous authority invalidation. All 135 Chromium cases across 17 shared runtime/production component story files pass, including all 14 checkpoint stories in worker/fallback modes. Production tree, tabs, navigation, grid, dropdown, dialog and menu declarations exercise focus/edits and surface refusal/re-admission. The tree fixture correction supplied its required public label; assertions were retained. Real independent WASM modules validate the new contracts, stale endpoint rejection and existing headroom/source/journal boundaries. Theme and forced-color component behavior is covered by the unchanged production regression fixtures. The complete 52-task Studio release gate passes (17 dependency results reused valid Nx cache output), including all 104 workbench tests, 32 native schema-package examples, repeated Login/Supplier editing, validation failure/repair, persisted source Parse/Inspect, graph editing/undo/redo, provenance and live preview, online/offline worker execution, two-project lifecycle and accessibility, theme inheritance/forced colors and clean package-consumer verification. Deterministic deployment retains 188 declared assets and 283 files. The dedicated 26-task deployed navigation gate also passes (24 valid dependency results reused): URL/history, model and physical trees, folder listings, Create/Clone/Delete, draft guards, themes/responsive layout, Supplier and Markdown site saved edits, intentional failure/repair, portable independent copies and offline rebuilds. Existing saved projects/drafts remain untouched; no generated screenshots or Vitest attachments are committed.

Runtime portability and journal retirement (2026-10-19): shared JavaScript lifecycle work was announced before implementation and remains in cem-elements. Native cem_ml::checkpoint_runtime_state validates bounded private form/choice state, terminal resource revisions/cursors, read/resubscribe policies, original HTTP source envelopes with native SHA-256, and consumed implicit/tokened writers. Capability state lives in the form domain; source/reacquisition state stays in resources, with prototype hydration fencing unchanged. Fields, checkboxes, selects, autocomplete and source editing reuse their production declarations in browser fixtures. Derived option caches/generated IDs/listeners are rebuilt from canonical payload and fresh host permissions, rather than serialized as browser ASTs or authority. Live/ambient URL, local/native JSON storage, repository/status and settled HTTP GET/HEAD snapshots retain their existing exact state; pending operations, provider leases/claims, credentials, unsupported host values and oversized payloads block the entire domain. Explicit write suppression metadata is a future semantic-resume obligation; ordinary resource behavior remains unchanged, and capture performs no extra requests or writes.

Journal IDs now use a private coordinator nonce and monotonic counter; callers cannot supply/reuse retired identities, and no tombstone history grows. Trusted handlers receive only an optional scalar checkpointOperationId. Current runtime/generation/policy authority can explicitly reconcile unknown outcomes to acknowledged/not-applied after independently establishing finality. Terminal retirement is staged and compacted only when a fresh all-owner native seal commits. Orphans remain blocking until resolved/staged; errors, cancellation and stale captures preserve records and intent. Policy changes revoke retirement intent; obsolete acknowledgements cannot mutate retired records. The 256-entry ceiling still blocks recovery after an unrecordable effect; retirement cannot repair an already-overflowed journal. Remaining non-form capability/provider/placement admission and actual runtime-domain mapping are explicit next steps. No Studio/component UI JavaScript, public component attributes, markup, paint, CSS/token defaults or storage migrations are added. Automatic Studio capture/restart stays disabled; isolated fixtures preserve saved projects/drafts, and generated screenshots/Vitest attachments remain ignored.

Runtime portability verification: failing-first native cases covered the absent validator/outcome variant, mixed consumed writer history and choice-state admission before implementation. Scalar-boundary fixtures exposed oversized keys and hidden/symbol properties that would otherwise be omitted; descriptor checks now refuse hidden/symbol/accessor state without invoking getters. All 28 focused native cases pass, including independent native source reimport and form/resource domain validation. All 640 shared unit tests pass, with atomic failed/stale/cancelled retirement, current-policy reconciliation, orphan handling and 300 repeated bounded retirements without ID reuse or replay. The 90-case Chromium regression covers checkpoint/host/form/stylesheet/island behavior plus production source-editor, field, checkbox, select and autocomplete; the final 12 checkpoint stories also pass after scalar admission tightening. Real WASM validation passes under recovery headroom, verifies native source digests, imports bytes in a fresh module and admits terminal not-applied outcomes while preserving native receipts and external host bytes. The complete 52-task Studio release gate passes (17 dependency tasks reused valid Nx cache results), including all 104 workbench tests, 32 native schema examples, all twenty Login and three Supplier configurations, live input, validation failure/repair, provenance, inherited themes/forced colors, 320px accessibility, static-worker online/offline execution and clean package-consumer acceptance. Deterministic deployment retains 188 declared assets and 283 files. The dedicated deployed navigation gate also passes URL/history, model/physical trees, Create/Clone/Delete, draft guards, theme retention, responsive layout and offline Supplier/Markdown site builds, including saved edits, intentional validation failure/repair and portable independent copies. Existing saved projects and drafts remain untouched; generated screenshots and Vitest attachments remain excluded.


Portable edits and effect outcomes (2026-10-19): reusable JavaScript remains in cem-elements; native validation belongs in cem-ml. Checkpoints now retain owned unbound input/textarea/select/button edits, UTF-16 selection/focus, checked/indeterminate, disabled/read-only, options and readable custom validity, plus the production shared plaintext editor. Composition, deferred edits, password/file/credential fields, hidden custom-validity messages, arbitrary contenteditable and unsupported capabilities refuse the whole domain. Property-only edits invalidate publication. A bounded explicit scalar control DTO is validated natively before form-domain export; checkpoint root version 1.0.0-checkpoint.1 fences older generic hydration, and current generic adoption requires future semantic resume. Fixed non-live URL readers preserve canonical state with read-again dispositions. Trusted shared command delivery and storage/location writes journal opaque unknown outcomes before effects and acknowledge successful completion without replay. Disposal/policy changes fence late outcomes; orphan records and journal overflow block recovery without suppressing ordinary actions. Async owner exporters pin membership before awaiting. A failing-first browser case exposed location writers without trigger tokens passing an empty-map guard; all location-writer owners are now refused until consumed writer state has a complete resume contract. Other resource portability, safe journal retirement and actual physical runtime coverage remain pending. No Studio/component UI JavaScript, markup, paint or token defaults are added; existing themes and forced-color behavior are unchanged. Automatic Studio capture/restart remains disabled; isolated fixtures preserve user projects and drafts, and generated screenshots/Vitest attachments stay ignored.


Portable-state verification: all 18 native control/checkpoint/data-island cases, 633 shared unit tests and 51 Chromium checks pass, alongside the real-WASM headroom/receipt/control-validation fixture. Browser assertions include actual worker/fallback rendering, production source editing, Unicode selection, composition, custom-validity privacy refusal, generic-hydration major-version fencing, Promise rejection/disposal, fixed URL read-again state, successful storage outcomes, rejected location effects and orphan refusal. Existing form-control, host-view, retained-stylesheet, source-editor and data-island assertions remain intact. The anonymous-declaration case passed on the focused rerun and full rerun without source/assertion changes after one two-frame timing failure. The complete 52-task Studio release gate passes (19 dependency tasks reused valid Nx cache results), including all 104 workbench checks, 32 native schema examples, the Login/Supplier lifecycle, real live input, intentional validation failure/repair, graph provenance, inherited themes/forced colors, 320px accessibility, static-worker online/offline commands and clean package-consumer acceptance. Deterministic delivery contains 188 declared assets and 283 files. The dedicated deployed navigation gate also passes URL/history, physical/model trees, Create/Clone/Delete, draft guards, theme retention, responsive layout and offline Supplier/Markdown site builds, including saved site edits, intentional failure/repair and portable independent copies. Existing sessions, saved projects and drafts are preserved; generated screenshots and Vitest attachments remain ignored.


Shared publication checkpoints (2026-10-19): shared cem-elements JavaScript was announced before implementation. CemCheckpointPublicationCoordinator now tracks all active owners in an explicit trusted host domain, successful DOM/CSS publication and stable form revisions, pending/superseded admission drain, event ordinals and immediate mutation/input/policy invalidation. Full canonical inert islands are exported only under whole-owner consent; native binary values remain opaque. Bounded coalescing retains no input payload queue and performs no action replay. Native sealing and the existing two-slot host store reject stale captures; policy changes erase previous private bytes. Unsupported live resources, behavior capabilities, host actions and unbound control edits block the whole domain instead of losing state. Overflow disables capture without breaking ordinary rendering. This opt-in host domain does not discover physical heaps or establish replacement authority, and Studio automatic capture/restart remains disabled. Shared control/resource/capability portability, an external-operation journal and real worker/fallback runtime coverage remain explicit next items. Studio deployment maps include the shared module; no Studio/component UI JavaScript, UI paint, theme defaults or persistence changes are added.

Checkpoint verification: the new coordinator fixture failed first with the absent API, then six coordinator cases plus four existing host-store cases passed. All 630 shared unit tests pass. Thirty focused Chromium cases pass across checkpoint publication, retained stylesheets and data-island isolation. The iframe fixture failed first because a caller-realm instanceof check skipped nested template content; DOM-native template traversal now enforces the same node/depth limits across documents. A separate failing-first reconnect fixture exposed an ordinal reset; weak instance counters now preserve accepted input sequence across disconnection while stale publication handles stay revoked. Real native sealing verifies actual exported islands in worker/fallback render fixtures; the separate real-WASM transport fixture passes headroom, native receipt checks, independently owned host bytes and fresh-module re-admission. Native sealing/serialization semantics are unchanged; this work adds shared browser publication/lifecycle admission around them. The complete 52-task Studio release gate passes (27 valid dependency tasks reused Nx cache results), including all 104 workbench tests, 32 native samples, all twenty Login and three Supplier configurations, real live input, validation failure/repair, graph provenance, offline isolation, inherited themes/forced colors, 320px accessibility and clean package-consumer acceptance. Deterministic deployment includes 187 declared assets and 282 files. The dedicated deployed navigation gate passes URL/history, physical trees, Create/Clone/Delete, draft guards, themes/responsive layout, Supplier provenance and offline builds, plus Markdown saved edits, failure/repair, portable independent copies and offline rebuild. Automatic checkpoint capture/replacement remains disabled in Studio. Existing browser sessions, saved projects and drafts are untouched; generated screenshots and Vitest attachments remain ignored.

Current Studio delivery verifier (2026-10-19): the verifier follows the navigator shell's shared theme trigger, contextual actions, footer status and three-region project layout. Removed Install/storage controls stay absent. Current URL navigation drives fixture selection; the separate deployed navigator gate covers physical tree/Clone interaction. Removed archive/search/Trash/Restore/structural controls are exercised through their existing public services with the original revision, byte, history and guard assertions retained. Login keeps all twenty configurations and Supplier all three, including native outputs, source colors/CSP, provenance, validation failure/repair, live input and independent offline copies. Results → Operation output now composes shared cem-media-preview declaratively, including the source resource set needed by live CEM; the independent project build preview is unchanged and standalone workbenches avoid duplicate mounts. Existing non-UI selection orchestration refreshes inventory when a committed target arrives before its repository notification. A deterministic delayed-notification fixture covers that race and unknown-target rejection; the Markdown copy assertion retains exact URI ownership and now reports actual URIs. The shell grid uses a bounded column for 320px graph authoring without hiding overflow. No component/app UI JavaScript, new browser capability, theme defaults or automatic runtime replacement is added.

Verification: the complete 52-task Studio release gate passes (35 dependency tasks reused valid Nx cache results), including all 104 workbench browser tests, the full deployed two-project/Login/Supplier lifecycle, accessibility at 320px, inherited themes/forced colors, static-worker online/offline execution, shell delivery, clean package consumer, all 32 native sample checks and deterministic 281-file output (186 declared assets). The deterministic delayed-inventory fixture was observed failing before the ordering repair and passing afterward. The live-view resource-set omission was caught by the preserved ready/input acceptance checks and repaired declaratively. Existing browser sessions, saved projects and drafts are untouched; generated screenshots and Vitest attachments remain ignored. The dedicated deployed navigation gate also passes URL/history, physical tree selection, Create/Clone/Delete, draft guards, theme retention, responsive/forced-color layout, Supplier provenance and offline builds, plus Markdown saved edits, intentional failure/repair, portable independent copies and offline rebuild.

Portable committed checkpoints (2026-10-19): shared native `cem_ml::committed_checkpoint` seals versioned transport against an independently held all-owner commit receipt, including declaration/revision identities, accepted event sequences, privacy policy and acknowledged/unknown external operation outcomes. The envelope carries inert DOM-native island markup; transport sealing never certifies semantic safety, replays an action or substitutes a browser AST. Both JSON inputs/output cap at 1 MiB, owners/operations at 256, and counters at JavaScript-safe integers. Exclusive recovery admission requires ordinary reservations to drain and fits both withheld headroom and the remaining hard budget; it never disables normal render controls. Shared `cem-elements` JavaScript was announced before implementation: a host-owned store preallocates two bounded slots outside WASM, copies acknowledged receipts, fences pending captures on cancellation/input/owner/policy changes, preserves last-good bytes on failure and separates stale work export from current recovery eligibility. Disposal erases/releases buffers. A store replacing a worker belongs in the surviving host. Studio consumes the reusable module through its declarative deployment maps; no Studio/component JavaScript or UI changes were added. Automatic publication/capture wiring, live island export and runtime replacement remain disabled and explicitly tracked next. See [checkpoint contract](committed-checkpoints.md).

Verification: the native failing-first fixture rejected the absent checkpoint/recovery APIs. Six checkpoint cases, six memory-policy/allocator cases and eighteen operation-control regressions pass. All 95 focused shared unit cases, 27 worker/fallback Chromium cases and seven Studio preview cases pass. Real WASM checks prove native sealing under pressure, exact receipt rejection, last-good preservation on admission failure, independent host bytes and re-admission in a second WASM module; existing runtime-memory and retained-render real WASM checks also pass. The deployed import-closure gate initially caught the new export missing from Studio's module maps; both declarative maps were repaired, and output verification now passes with 186 assets / 281 files. All 32 native samples, deterministic output and deployed navigation pass, including URL/history/tree/CRUD/draft/theme/forced-color/layout checks, ordinary Supplier/Markdown builds, saved edits, failure/repair, portable independent copies and offline reopening. No UI/theme defaults changed. The separate full release/lifecycle check was not rerun: its known removed-Install-control mismatch remains tracked immediately below the completed work, and automatic replacement remains disabled. Standalone Rust formatting, diff and generated screenshot/Vitest-attachment ignore checks pass. Existing browser profiles, saved projects and drafts are untouched.

WASM memory budgets and headroom (2026-10-19): shared cem_ml::runtime_memory measures actual requested allocator live/peak bytes through a WASM System-allocator wrapper with allocation-free bookkeeping and separately observes each runtime's linear-memory capacity. Admission conservatively uses the larger observation plus bounded outstanding workspace reservations; these overlapping measurements are not summed or presented as browser/process RSS. Defaults are 768 MiB soft / 1 GiB hard / 64 MiB withheld headroom and at most 64 concurrent operations. Trusted native/WASM configuration validates complete limits and fences reconfiguration while reservations remain. Guarded compile/render, retained CSS/document/native input and command-service boundaries reserve eight times actual input bytes plus 64 KiB before parsing; RAII releases reservations on failure, cancellation completion and drop, including async commands. Soft pressure refuses new work without poisoning operation control; cooperative safe points deliver the existing root memory failure at the hard work ceiling while preserving earlier cancellation/failure. Disposal/cancellation, bounded status/configuration and taking already-produced artifacts remain callable. No allocator-null/trap policy, forced collection, automatic restart/action replay, JS heap estimate, Studio/component JavaScript, new browser AST or theme changes are added. ABI copies, growth between safe points and unguarded low-level APIs remain outside a strict forecast; this is cooperative admission, not an absolute allocation/OOM guarantee. Explicit recovery admission/storage and portable committed checkpoints are next, followed by runtime generation replacement/resume and declarative status.

Verification: the failing-first native fixture rejected the absent accounting/control APIs. Six new policy/allocator cases, eighteen operation-control regressions and 272 focused CEM-QL/query/render/CSS cases pass (four existing ignored). Real WASM checks verify nonzero native allocation measurements, exact exported memory capacity and manual growth, atomic invalid configuration, refusal of compile and previously cached render work under pressure, command refusal before resource/write callbacks, reservation release, template disposal and subsequent admission after restoring limits. Retained-render real WASM checks, 107 focused shared unit cases and 27 worker/fallback Chromium cases pass. Studio workbench (103), shell (11), preview (7), repository/provider/other browser cases and Node worker checks pass, along with all 32 native sample checks and deterministic output. The full 52-task Studio check fails in static-worker/workspace-delivery/shell-delivery before Login assertions: inspectShell still calls getComputedStyle on the removed [data-cem-studio-install] button. The committed shell.browser.spec.js explicitly asserts this control is absent; the verifier mismatch is recorded separately without reintroducing removed UI or weakening assertions. The current deployed navigator and output gates pass URL/history/tree/CRUD/draft/theme/layout checks, ordinary Supplier and Markdown builds, edit/failure/repair, portable copies, pinned comparisons and offline reopening. Full long-session Login acceptance remains blocked by the old delivery verifier; its failure is not counted as passing. New Rust modules/fixtures pass standalone rustfmt; diff and generated screenshot/Vitest-attachment ignore checks pass. Existing browser sessions, saved projects and drafts are untouched.


Shared retained WASM/worker render reuse (2026-10-19): shared Rust now parses owned JSON and computes complete native freshness from the actual input bytes, engine versions and scope policy. Browser digest/revision claims cannot establish equality. One bounded runtime output store serves retained artifacts; owner/instance/artifact identity qualifies each request, and reset/template disposal clears cached output and advances generation. Native/documents/companion/reference capabilities, tagged resource/node handles, opaque computations and unsupported output remain fresh. Shared cem-elements JS was announced before implementation; it pins worker/fallback requests before async preflight, passes internal admission context and rejects superseded/released render completions. Every hit still performs native value/reference projection, current output limits, payload/link handling, host updates, resource lowering, DOM diff and publication checks; event/command admission remains in the normal host path. Optional native hit/evaluation/admission counters are diagnostic only. No Studio/component JavaScript, browser AST substitute, theme defaults, retained DOM/native owners or preceding connection chain is added. Native input proof caps at 1 MiB / 65,536 traversal units; the existing 64-entry / 32 MiB output store and per-output bounds remain. Whole-runtime accounting and portable committed checkpoints are next, before runtime replacement.

Verification: native semantic gates pass 272 cases (four existing ignored). The real WASM fixture proves actual hits, changed values under unchanged context, independent native value exports, fresh stricter output limits, opaque input and explicit native capability fallback, unknown/released owners and reset/disposal. All 107 focused processing/transport/projection unit cases and 27 worker/fallback Chromium cases pass, including request pinning against caller mutation, stale overlap rejection, repeated host/resource updates and live input edits/focus/selection. A failing-first captured-input fixture exposed targeted patches invalidating only physical DOM ancestors: inert payload owners could keep an earlier children receipt and skip restoring disabled attributes. Shared projection now invalidates logical authored owners across template/island boundaries and records the current merged inputs; only weak current-container edges are retained. All seven Studio preview cases, deployed navigation and output gates pass, including URL/history/trees/CRUD/drafts/themes, ordinary Supplier and Markdown builds, entry-setting repair, saved edits, intentional build failure/repair, portable copies and offline pinned/current previews. All 32 native feature-tour examples and the deterministic 280-file build pass. Standalone rustfmt, git diff checks and generated-artifact ignore checks pass. Generated screenshots and Vitest attachments remain ignored; existing browser profiles, saved projects and drafts are untouched.


Guarded native pure-render reuse (2026-10-19): `cem_ql::render_cache::PureRenderCache` now consumes complete render identities and successful pure-render receipts. Its existing ContentCache backend stores only immutable bounded native output bytes with leases/LRU/count-byte admission; no decoded render/native owner cache or parallel unbounded index is added. Lookups are qualified by complete template/query/provenance/contracts/provider/policy identities plus consumer owner/runtime generation. Weak store tickets reject foreign/reset runtime requests before callbacks; requested owner revisions must remain current. Mandatory host admission runs accepted-event/command/resource work on hits as well as misses, while a separate read-only currentness callback validates pinned metadata before/after evaluation/decode and before return. Admission errors, changed snapshots and cancelled scopes return no stale output. New admitted revisions with identical pure inputs can hit; old requests cannot publish under them. Each hit reloads independent owned nodes, atomic streams, contracts, host updates and source frames. Unsupported native/reference-valued output, partially consumed/chain streams, opaque/failed/caught evaluations and unavailable metadata stay uncached. Output payloads cap at 4 MiB / 16,384 traversal units / depth 64; capacity refusal preserves ordinary output. Native binary atoms preserve decimal lexical forms and nonfinite floats. Decode admits temporary bytes against current/ancestor scope memory and checks cancellation; pressure before allocation falls back without poisoning control. Returned-plan heap and whole-runtime memory accounting remain separate work. Native payload/input owners and callbacks are not retained by cache entries or request tickets.

Verification: the failing-first Rust fixture initially rejected the absent cache API. Eleven native cache integration cases and two private output-codec cases now pass; the focused native gate passes 269 cases (four existing ignored library fixtures), covering receipts, default/native evaluation, recovery, rendering and retained CSS. Actual cache hits reduce evaluator invocations, preserve independent output, atomic types, host updates and source spans, and still execute event/command/resource admission. Native-valued attributes remain normal uncached output and release their original owners. Scope pressure falls back without poisoning control; stale/reset/foreign requests and changed/admission-failed/cancelled publication return no cached result. Shared count/byte/LRU refusal preserves successful uncached rendering. WASM builds, 106 processing/transport/projection unit cases, 24 worker/fallback Chromium root/stylesheet cases and all seven Studio preview cases pass. New cache modules/fixtures pass standalone rustfmt; `git diff --check` and generated screenshot/Vitest attachment ignore checks pass. Existing browser state, saved projects and drafts were untouched.

CEM audit: new semantics and serialization stay in shared Rust; no Studio/component JS, browser AST substitute, theme paint/defaults or resource/event suppression is added. The native cache is opt-in: ordinary render APIs and existing browser/worker processing continue evaluating. Shared retained/WASM processing wiring with pinned provider snapshots and mandatory admission is the next explicit item; alert before any shared cem-elements JS capability. Native value identity/provenance is preserved by refusing unsupported payloads, rather than projecting them into anonymous cache-owned values. Store limits do not claim complete process RSS/OOM recovery.

Native render receipt aggregation (2026-10-19): the opt-in shared Rust render boundary now aggregates native query identities, read names, focus usage and invocation counts across declaration selects, attributes, conditionals, loops, variables and named/match templates. Native output, host updates, source/output provenance, operation control and recovery use the ordinary renderer. Every provider axis and an environment stamp must attest pinned pure/fresh values, absence, provenance and access policy. The `RenderFreshness::from_snapshot` bridge validates the existing ready RenderProjection snapshot, including source/import closure, policy, resolver, scope and every input version; opaque/pending/failed/released/compilation-only snapshots do not qualify. Native template/query IR, stylesheets/module maps and receiver/output/value contracts are fingerprinted. All provider axes conservatively invalidate the aggregate; local names alone do not establish external independence. A per-invocation native stamp distinguishes changing loop/call values under the same query. Transient query receipts are consumed into bounded metadata rather than cached separately. Query/aggregate bounds preserve ordinary rendering on refusal; no input/output/native-view owners, captures, callbacks or predecessor chains are retained. Custom template callbacks, inherited captured runtime scopes, opaque queries, diagnostics, cancellation and even caught query failures remain ineligible. No render cache, JS bridge, wire-format change or Studio/component-specific implementation is added.

Verification: failing-first native coverage initially rejected the absent render tracking API. Twelve new integration fixtures plus one inherited-runtime-scope fixture now pass; the full focused native gate passes 256 cases (four existing ignored library cases), including query receipts, evaluator/native callbacks, render recovery, ordinary template output and retained stylesheets. Both native WASM builds through the downstream gate, 106 processing/transport/projection unit cases, 24 worker/fallback Chromium root/stylesheet cases and seven Studio preview cases pass. New modules/fixtures pass standalone rustfmt; `git diff --check` and generated screenshot/Vitest attachment ignore checks pass. Existing browser state, saved projects and drafts were untouched. No cache hit or provider/event/resource suppression is enabled by this change.

CEM audit: semantic evaluation/freshness remains shared native Rust. Studio and components retain declarative behavior with no new browser capability, authored JS, paint or theme defaults. Existing native provider contracts remain authoritative; hosts lacking complete immutable/current freshness continue ordinary evaluation. Theme/forced-color implementation changes are inapplicable. Whole-render matching is only an input check: current runtime ownership and independent event/command/resource/cancellation/publication admission remain required before the next caching step. Finer binding-to-axis invalidation is deliberately deferred until native origin tracking can prove it.

Native evaluation read tracking (2026-10-19): opt-in `cem_ql::eval::dependencies` runs the existing evaluator with the same operation-control safe points and returns an independent native receipt. Actual taken-branch reads include the direct record-field path; whole-binding freshness covers missing fields and provenance. Provider contracts require pure reads plus complete freshness, with opaque defaults, validated native content hashes or owner/generation/revision stamps, and an explicit environment stamp. Matching checks actual IR/source maps/types/resolutions, scope and effective policy. Native MessagePack query encoding preserves nonfinite float identities. Closed sequence/string/number/record operations can qualify; dynamic/native/template calls and DOM/data/resolver/report operations remain ineligible. Failure, cancellation, diagnostics or overflow preserve ordinary evaluation without a receipt. Tracking caps binding IDs at 1,024 and strings/stamps at 1 MiB, with a separate bounded 1 MiB temporary query encoding. Receipts retain no input/output/native-view owners or history; normal evaluation allocates no read set. This establishes the query contract only: render aggregation/provider contracts and admitted bounded pure-render memoization are explicitly next, before runtime recovery. CEM audit: all semantics remain in shared Rust; no Studio/component JavaScript, browser AST bridge, theme defaults or UI changes.

Verification: the native failing-first fixture initially failed because the tracking API was absent. Twelve new read-receipt fixtures now pass; evaluator, native-function, template-render and retained-stylesheet regressions bring native coverage to 233 passed with four existing ignored library tests. The WASM build, 106 processing/transport/projection unit cases, 24 Chromium root/stylesheet cases in worker/fallback modes and seven Studio preview cases pass. Both new Rust files pass standalone rustfmt; `git diff --check` passes and generated screenshot/Vitest attachment paths remain ignored. Theme/forced-color changes are inapplicable to this native-only change. No browser session, saved projects or drafts were altered. Render memoization remains disabled until the next two explicit tasks establish complete render/provider receipts and independent operation admission.

Incremental owned-root publication (2026-10-19): native rendering already emitted the correct conditional/repeated roots and empty plans; the host previously replaced scopes on root-count changes and emitted no operation for an empty replacement. A single isolated reconcileRoots operation now publishes the full fresh root list through existing keyed merge machinery. Root ownership and integrity receipts compare against the last committed element-root list, so deliberate topology changes retain compatible nodes while corrupted roots still recover. Empty output clears owned roots, preserving explicit runtime nodes, bounds and instance/style owners. Available text/comment IDs retain exact root nodes; restored text without in-memory IDs uses range reconciliation. Duplicate root identities and mixed root/targeted transactions reject before DOM mutation. Preview policy and native-placement marker checks inspect all new root payloads.

Same-parent internal moves use the browser's state-preserving move when available, with focus/text-control/editable-selection restoration for fallback insertion. A transient shared weak flag prevents produced elements and their descendants from tearing down live owners during either internal reorder path. It clears synchronously even on failure and retains no state/history. Both stylesheet document observers drain genuine prior records before a move, then exclude only its exact verified removal records; weak listeners hold no prior consumers. This prevents a same-parent move from being mistaken for a disconnect and deleting same-key CSSOM. Actual removal and cross-parent moves still disconnect/reacquire context. Child evaluation, provenance updates and import freshness continue normally. Matching CSSOM and live controls survive the fresh publication; authored child-input equality does not authorize skipping evaluation. Host snapshots and mutation invalidation retain their existing contract, including changed/reverted attributes and native capsule handoffs. Produced-child fixtures check retained stylesheet/CSSOM/control identity through native and fallback reorders, fresh import validation, changed/reverted input propagation and ordinary cross-parent reacquisition. Render engine 1.7.0 and processing host v24 fence the new patch vocabulary; explicitly incompatible engine frames reject before mutation.

Final verification passes 76 Chromium cases (72 across eleven focused files and four direct-patch legacy cases), 106 unit cases across seven processing/transport/projection files, and 68 native cases (55 template rendering and 13 retained stylesheet cases). The runtime TypeScript build and all seven Studio preview cases pass. The 26-task deployed gate passes graph admission (185 declared assets / 280 files), all 32 native schema-package examples, deterministic rebuilding (`9cdba3a69b504f3f8c4728f572da4b9a10fc61c8a91bfeeb152c2bcf9d2d7f50`), URL/history, model trees, CRUD, drafts, themes/forced colors, responsive layout and offline Supplier/Markdown site builds. It includes repeated edits/saves, validation failure/repair and portable independent Markdown copies. A final serialized output check follows the concurrent-build probe: overlapping checks temporarily removed a generated feature-tour file; build/read overlap is stopped and the complete successful gate and final output check are the evidence, without weakened assertions or longer timeouts. `git diff --check` and generated screenshot/attachment ignore checks pass. Failing-first browser fixtures exposed scope replacement, missing empty publication, positional root-text reuse and produced-child resource reacquisition. Native contract coverage checks conditional insertion/removal, repeated root order, empty output and unchanged surviving provenance without changing Rust semantics. Existing storage demo assertions now require retained paragraphs and no spurious scope-recovery diagnostics through object/array/scalar transitions; genuine invalid-JSON diagnostics remain required.

CEM audit: all browser/DOM/lifecycle changes live in reusable cem-elements, with native rendering/resource semantics unchanged. Studio and declarative components add no JavaScript workaround, paint/defaults, schema/storage migration or project/draft mutation. Root receipts store bounded current root IDs under weak range ownership; moves use a transient weak flag, without a new AST cache or retained predecessor chain. Provider/event/command and resource freshness remain on their normal paths. Evidence covers owned-root lifetimes, edits and native/Chromium behavior, not a whole-process RSS/OOM guarantee or physical-device/IME/AT acceptance. Native selective read tracking and between-render WASM recovery remain next; the separately tracked held-worker readiness timeout and strict-schema baseline remain open. Generated screenshots and Vitest attachments remain ignored.


Fresh-output DOM reuse (2026-10-19): direct projection previously compared live editable text with desired text, overwriting user edits even when authored children were unchanged. It now owns one immutable input snapshot shared by subtree receipts, compares complete child output (including native bytes and provenance), and leaves matching live children untouched. Existing weak DOM ownership holds no predecessor/history chain. Changed bindings still update native input/textarea values and editable text. Ordinary select/checkbox edits and focus/selection survive repeated updates; explicit select[value] intentionally retains its existing controlled-every-commit contract.

Reusable ownership now includes tag, instance, template artifact, scope policy, output target and effective ARIA profile. Direct range ownership changes replace content without transferring focus to another owner; patch diffs also fence those changes. Processing verifies retained prior-plan handles against actual artifact/policy, engine, source-map mode and revision. Structured revision encoding prevents delimiter collisions. Fresh data still evaluates even with the same revision string; no data-only, source-only or browser AST evaluation cache is introduced. Prepared patch objects clear frames, targets, bounds and option callbacks after commit/cancel/stale consumption; forced Chromium collection proves callback-owned witnesses are released while consumed handles remain reachable.

Verification: failing-first browser fixtures reproduced overwritten editable text, missing instance/policy invalidation and retained consumed callbacks; the processing fixture reproduced an admitted mismatched prior-plan handle. The final source gate passes 60 Chromium cases across nine files; all 93 selected processing/host/projection unit cases and seven Studio preview browser cases pass, including the runtime TypeScript build. The custom-element provenance fixture also failed against its old diff-only shortcut and now preserves node identity while updating source metadata. The final deployed build passes graph admission (185 declared assets / 280 files), all 32 native schema-package examples and deterministic rebuilding (`f6ccd6d2e688c0455e921b68921627991b78d699e85b962cf577c5b048e51e5d`). The full 26-task deployed Studio gate passes URL/history selection, model trees, Create/Clone/Delete, guarded drafts, themes/forced colors, responsive layout and offline Supplier/Markdown site builds. `git diff --check` and generated screenshot/attachment ignore checks pass. The prior controlled-select assertions remain unchanged. Repeated data edits run through real native worker/fallback and DOM/CEM-ML runtime paths, preserving user-created editable text, native form state and focus/selection under inherited theme changes. Prepared admission covers edits during pending publication, cancellation, stale revisions, target/range replacement and scope recovery. Existing stylesheet generation/context/import/collection and processing host replacement cases supply the complementary owner-lifecycle checks.

CEM audit: all implementation stays in shared cem-elements DOM publication/transport code; native interpretation, evaluation, provider/event/resource freshness, compilation and stylesheet ownership remain on their normal paths. Studio and cem-components add no JavaScript or styling changes, token defaults, storage/schema changes or project/draft mutations. No whole-process RSS bound, skipped evaluation, incremental root-topology support, physical-device/IME/AT acceptance or WASM OOM recovery is claimed. Root-count fallback and the separately tracked held-worker DOM-adoption timeout and strict-schema baseline remain open. Generated screenshots and Vitest attachments remain ignored.


Native-validated stylesheet installation reuse (2026-10-19): unchanged instance publications formerly rewrote the occurrence marker and appended each style again, recreating CSSOM and restarting animations. Declaration ordering also moved unchanged siblings when another native sheet was added. Shared `cem-elements` now keeps matching installed nodes in place and moves only required nodes into managed order. Instance lookup uses current occurrence receipts rather than physical DOM order; native owner/cache/context identity, host/mode and policy qualify reuse. A changed native identity reinstalls even when CSS bytes match. Installed text and managed metadata are checked against newly admitted output and repaired if changed. Declaration sheets retain their existing derived-owner sharing, with minimal ordering and DOM repair. One weakly keyed current receipt per instance node is deleted on owning-generation disposal; no previous-generation or application cache is added.

Native compilation, import closure loading, redirects, response policy and resolution still run through the existing shared processing path before publication. Matching native emissions already share retained native records; the browser change preserves the matching installation while transferring current load/consumer ownership. Unknown or changed source/context is not authorized by text equality. Every imported stylesheet is re-read/revalidated on each attempt; pending, failed and cancelled candidates preserve committed styles. CSS-driven inherited theme changes update computed values without changing CSSOM or the running animation object. Render/data/provider/event work remains on its normal path; this does not implement full DOM reuse or skip expression evaluation.

Verification: failing-first browser fixtures exposed the instance's three unnecessary DOM mutations and the declaration sibling's recreated CSSOM; a separate integrity assertion exposed altered cached declaration text not being restored. All 42 focused browser cases across six files now pass in real Chromium, covering worker/fallback behavior, instance/declaration CSSOM, running animation identity, inherited changes, fresh and changed imports, changed base/native identity with identical emitted bytes, sibling changes/order repair, text/marker repair, transactional failure/cancellation, scope disposal, reentrant retirement and forced collection. The runtime integration checks repeated data edits in DOM and CEM-ML modes, retaining both stylesheet installations and CSSOM while rendered text updates. All 13 native retained-stylesheet cases and 77 processing engine/host/boundary unit cases pass. The runtime TypeScript build and all seven Studio preview browser cases with 19 prerequisites pass. The 26-task deployed Studio gate passes graph admission (185 declared assets / 280 files), all 32 native schema-package examples, deterministic rebuilding (`1492e6b63e4690746c2b4dcdc3c6781569d4a3d7fc9ae51d32284770c1bcc4dd`) and URL/history, trees, CRUD, drafts, themes, responsive layout and offline Supplier/Markdown site builds. The output verifier caught the new shared helper missing from deployment resource maps; explicit source/destination entries now ship it without changing Studio behavior. `git diff --check` passes.

CEM audit for stylesheet reuse: implementation is shared reusable host DOM/lifecycle machinery in `cem-elements`; native CSS parsing, source/import identity, emission and policy stay in Rust. No component- or Studio-specific JavaScript, parallel source/AST cache, paint/token defaults, schema/storage migration or project/draft mutation is added. Managed style nodes remain outside the instance render range and preserve native declaration occurrence order; stylesheet freshness and ownership admission are retained. Existing collection and late-owner fixtures remain green, with receipts retaining no prior generation. Evidence concerns native-validated installations and CSSOM, not whole-process RSS/OOM bounds or full unchanged-DOM/focus/form-state acceptance. The separately tracked baseline held-worker DOM-adoption timeout and strict-schema/device/helper work remain open. Generated screenshots and Vitest attachments remain ignored.


Stylesheet replacement ownership (2026-10-19): an earlier OOM repair already cleared the explicit `previous` pointer, but the new failing-first forced-collection fixture proved a live connection still retained its completed preparation through callbacks/scope observers. Preparations now compare opaque predecessor generation tokens instead of retaining predecessor connections. Activation creates live callbacks and scope observers in separate method scopes and clears transferred commit records before adoption/retirement callbacks. Cancellation clears records, staged load bookkeeping and removed observer closures while preserving committed styles. Live scope disposal and reentrant old-generation abort callbacks still remove exactly the intended connection; late retired-handle release cannot remove the newest styles. No DOM/style caching or evaluation shortcut is enabled by this lifecycle repair.

Verification: all 31 focused browser lifecycle cases across five story files pass, covering registry preparation, native installation, instance styles, worker/fallback parity, forced collection of retired generations, completed preparation handles and transferred arrays, cancelled candidates, context/URL changes, late responses, queued publication, ancestor disposal and reentrant retirement. The new collection fixture failed against the original code before implementation. All 13 native retained-stylesheet cases, 77 processing engine/host/boundary unit cases, and seven Studio preview browser cases pass. Preview's 19 prerequisites rebuild/check the shared runtime TypeScript, native CLI/adapters, components and themes; cached unchanged dependencies are reused. `git diff --check` passes.

The broader six-file browser probe reports 32 passing cases and one pre-existing timeout in `DOM Stylesheet Adoption / Readiness And Native Diagnostics`. The same 30-second timeout reproduces alone with the unchanged `6f1c8dca` registry; sequential execution does not resolve it. Its other two cases pass. This baseline is recorded separately below functional work; no readiness assertions, timeout limits or unrelated runtime behavior were weakened to hide it.

CEM audit for replacement ownership: the repair lives entirely in reusable `cem-elements` lifecycle code. Native CSS parsing, loading, scoping, identity and resource policy remain native and unchanged. Components and Studio retain declarative composition, without app-specific JavaScript, a parallel cache, CSS paint/token defaults or project/draft persistence changes. Context-bound module URLs and computed styles remain valid in both worker and fallback modes. Collection is asserted explicitly in test-mode-only Chromium fixtures, not inferred from timing or exposed as product behavior. Evidence covers these ownership graphs and cleanup, not a whole-process RSS/OOM guarantee or unrelated physical-device/AT acceptance. Generated screenshots and Vitest attachments remain ignored.


Checked native/WASM compilation (2026-10-19): `TemplateCompilationCache` uses the existing bounded `ContentCache`, without an application cache or an unbounded key index. Before lookup it bounds dependency metadata, rehashes actual root/import bytes, and checks parameter/entrypoint contracts, compiler versions, provenance-owning URIs, effective options, mode and newly supplied resolver policy/freshness metadata. Unknown resolver stamps and invalid closures keep ordinary diagnostics and remain uncached. Immutable serialized binaries reload into independent native owners. Capacity refusal remains a successful uncached compilation. Native request tickets distinguish current owner/revision, cancellation and cache runtime replacement; WASM calls execute synchronously, while shared processing checks CEM supersession/disposal after async boundaries and releases rejected owners before publication.

WASM source, closure and binary compilation now use checked lookups. Counter-only diagnostics and cache reset expose actual compiler invocations/hits, residency, charged bytes, eviction and runtime generation. Cache reset leaves already published independent handles alive. New processing CEM owners reach native checks; already retained unchanged handles remain compile-once. CSS/XSLT concurrency and retention behavior is preserved. Provider/event/command evaluation and resource/CSS loading still execute normally; no render or DOM reuse is enabled by a compilation key.

Verification: the initial native fixture failed at the missing API; all 104 selected native cases pass (22 shared cache/identity and 82 CEM-QL cases, including seven checked-compilation cases). The new cached Nx gate builds release WASM and passes the real boundary fixture for hits, imported source corruption, fresh/opaque resolution, independent handles, reset and dev/prod binary parity. All 77 selected processing engine/host/boundary cases pass, including superseded and disposed async result release and separate native closure requests. Studio lint and the rebuilt runtime TypeScript build pass. All seven Studio preview browser cases and their 19 prerequisites pass, including native CLI/adapters, component declaration verification and native theme builds. Focused rustfmt checks, the WASM fixture syntax check and `git diff --check` pass.

CEM audit for compilation: all interpretation, hashing, dependency validation, binary verification and bounded retention stay in shared native crates. Shared `cem-elements` owns transport, handle lifecycle and async publication guards; Studio and declarative components add no JavaScript UI behavior. The binary envelope is unchanged, with closure binaries held internally instead of exported to the source-only registry. No DOM renderer, component paint, token defaults, project/draft persistence or theme semantics change. Cached serialized buffers are bounded; decoded per-consumer IR, compilation scratch, CSS adoption, allocator overhead and process RSS are outside this budget. DOM/style reuse and previous-connection cleanup remain the next functional work; unrelated strict-schema, helper and physical-device/AT acceptance remain separately tracked. Generated screenshots and Vitest attachments remain ignored.


Bounded native artifact storage (2026-10-19): the existing `ContentCache` now has configurable count/byte limits (64 owners / 32 MiB by default), immutable shared leases, successful-read LRU promotion and artifact-owned sidecars. Admission verifies format/body/sidecar integrity, bounds metadata encoding to 1 MiB and compacts incoming payload, policy and provenance buffers. Charges cover compact serialized payload, encoded controls/sidecars and record/key overhead, including retired live owners; decoded IR, compilation scratch, container/allocator/process overhead and unrelated native registries remain processing-admission responsibilities. This extends the shared native store, without a parallel Studio cache.

Admission plans all victims before mutation, never evicts live leases and preserves existing entries/recency on capacity or validation failure. Identical records share one charge; a leased record cannot be replaced with conflicting policy/metadata. Reset advances runtime generation and clears resident lookup state; old leases reject access but remain charged until their final release. Drop also invalidates leases. Async lookups/results use an opaque weak owner ticket captured before work, fencing both generation changes and distinct cache owners whose diagnostic counters happen to match. Tickets retain no store/payload owner, and are not serialized/revived as handles. Counter-only telemetry is bounded and contains no payload/history log.

Cache verification: core lifecycle fixtures were written before the new APIs and initially failed at those native boundaries. All 97 selected native cases pass: legacy content cache (6), bounded cache/leases (11), dependency identities (5), compact templates (6), compiled artifact reload/dev-prod/CSS transport and cache parity (9), template receipt (4), template rendering (54) and serialization (2). Coverage includes count/byte LRU pressure, sidecar ownership, atomic failure/replacement, deduplication, policy/corruption/metadata rejection, cross-thread retired release, stale and cross-owner tickets, and moving genuinely oversized-capacity input buffers into admission. Real cached/uncached compiled-template reloads preserve exact render plans, diagnostics, source maps and output byte ranges in dev/prod modes; changed source context and policy remain rejected. Final CEM-QL WASM release build, all 76 selected processing engine/host/boundary unit cases and all seven Studio preview browser cases pass. The browser target's 19 prerequisites rebuild native CLI/adapters and check declarative components, native themes and runtime TypeScript. Focused rustfmt checks and `git diff --check` pass.

CEM audit for cache ownership: parsing, artifact verification, metadata hashing and leases remain in shared native engine code; the binary artifact wire envelope is unchanged. The store retains immutable serialized native artifacts, with sidecars owned by the same lease, rather than AST/DOM/callback/abort or previous-connection history. No Studio JavaScript, browser capability, component markup, CSS, tokens or project/draft persistence changes were introduced. Existing declarative UI/themes remain on their normal execution path. Generated screenshots and Vitest attachments remain ignored. Production dependency-to-artifact lookup, fresh-context/publication admission and WASM consumer wiring are now the next functional item; no browser compile shortcut, DOM/style reuse, global WASM RSS guarantee or unrelated strict-schema/device acceptance is claimed by this storage step.


Native dependency identities (2026-10-19): `cem_ml::dependency_identity` supplies bounded native snapshots and a versioned BLAKE3 domain for compilation, stylesheet and render identities. Complete source/import closures, original URIs/type/schema, effective options, engine/IR, policy/resolution/scope and provider-owned revisions participate. Render snapshots require explicit data, attributes, payload, slices, resources, form, validation, accepted-event, refresh and evaluation-provider freshness axes. Opaque providers and pending/failed/released closures never produce keys. Canonical graph/maps, typed versions, conflicting/missing identity checks and preflight/encoded metadata limits avoid payload hashing and unbounded records. Limits are 1,024 sources, 4,096 edges, 128 options and 1 MiB metadata. Owner/runtime-generation/revision admission is separate from the portable key.

The opt-in `compile_template_module_closure_with_dependencies` entrypoint runs the existing native compiler once and returns its artifact plus an eligible identity or a small ineligibility reason. Verified nested source hashes, provenance-owning URIs, resolver policy/freshness, effective binding/entrypoint/function-body options and compiler/format versions are covered. Imported declaration/stylesheet order remains an explicit axis because the current compiler appends in preflight order. The receipt preserves ordinary diagnostics/render plans and source/output frames, rejects conflicting source owners and leaves missing resolver stamps opaque. It changes no binary artifact envelope or CLI/WASM/browser protocol. This is an identity/invalidation foundation with no cache hits, leases or skipped evaluation; native cache consumers and host admission wiring remain next.

Identity verification: the nine new native cases were added before their APIs and first failed at the missing native boundaries. All 85 selected native cases pass: dependency identity (5), existing content cache (6), template receipt (4), compact imported bodies (6), artifact reload/dev-prod/CSS transport (8), template rendering (54) and serialization/output ranges (2). Fixtures cover every context/input/freshness axis, nested edits and URI/order changes, canonical binding sets, malformed/opaque/incomplete/ambiguous closures, conflicting provenance owners, encoded-size/source/edge limits and changed owner/runtime/revision admission. CEM-QL WASM release build passes. The processing engine/host-runtime/boundary unit gate passes all 76 cases; the rebuilt Studio preview browser gate passes all seven cases and its 19 prerequisites, including native CLI/adapters, declarative component checks and runtime TypeScript compilation. Focused rustfmt checks and `git diff --check` pass. No full device/AT or strict-schema baseline closure is claimed.


CEM audit for identities: all source interpretation, validation, hashing and template work stays in shared native engine crates. Records contain control fingerprints/revisions, without serialized ASTs, source buffers, callbacks, DOM or retained handle histories. Native tests were introduced before implementation; the broader render fixture needed only its missing existing `source_uri: None` field, with assertions preserved. Studio and components add no JavaScript UI behavior, browser capability, markup, CSS or token defaults. Declarative composition, inherited themes and native CSS ownership remain unchanged. Saved projects/drafts are not touched; generated screenshots and Vitest attachments remain ignored. Existing strict-schema diagnostics and physical-device/helper acceptance remain separately tracked behind functional work.


Build comparison (2026-10-19): selecting a verified pin exposes **Compare pin with current build** against the retained successful current run. Sorted artifact paths report additions/removals/changes, complete SHA-256 digests, media/length changes and native provenance changes separately. Inputs match by stable resource ID with both paths/URIs/revisions/digests; configuration/entry and engine versions are captured on both sides. Canonical portable control comparison ignores object-key ordering and process-local source/handle/source-map identities while preserving native frame order and multiplicity. It compares admitted bounded manifests, reads no additional artifact bytes and memoizes by immutable run ownership.

**Compare artifact** selects one detail independently of the preview, source editor and URL, keeping native frame rendering bounded to one artifact. Both sides expose captured source/output byte ranges without fabricating missing lineage or claiming causality. No current run means no comparison; return/delete/context disposal and pending/failed baseline reads clear or suppress it. Failed/cancelled/stale attempts retain the previous successful current run. Original source bytes are not stored in pins, and comparison neither follows stale ranges into the editor nor saves drafts. Text diffs, portable pin archives and cache hits remain outside this step.

Comparison verification: all 23 repository cases and 103 workbench cases pass. Coverage includes added/removed/binary/media-only/provenance-only outputs, stable input renames, configuration changes, object-key order, bounds/ambiguity/project isolation, exact native ranges/digests, current/source/draft independence, corrupt and late reads, deletion and offline reopening. The real Markdown site reports one changed article and six unchanged artifacts. Runtime/Studio typechecks and Studio lint pass. The 26-task emitted/deterministic/deployed gate passes all 32 native examples, existing CRUD/URL/theme/responsive checks and fresh offline pin/current comparison, independent artifact selection, forced-color interaction and toolbar containment. Output remains 184 declared assets / 279 files; deterministic SHA-256 is `8b64b27e79e288e734bbb5be5c6e1ed189611f41627394f7a90390bc10662944`.

CEM audit for comparison: native commands retain ownership of parsing, builds, exact output and provenance; Studio compares derived control manifests and orchestrates existing selection/persistence services. Visible report composition and interaction bindings remain XHTML/CEM-ML using shared selects, expansions, stacks and existing controls. No new JavaScript browser capability, app DOM renderer, native AST projection, component paint/token defaults, storage/schema migration or extra preview owner was introduced. Existing inherited themes/forced-color controls and bounded toolbar remain intact. Isolated fixtures preserve user projects/drafts; screenshots and Vitest attachments remain ignored, and temporary typecheck source preparation was removed. Shared native dependency identities, bounded reuse and retained DOM/styles are the next functional steps; the separately tracked strict-schema baseline diagnostics remain open.


Pinned builds (2026-10-19): successful complete runs persist exact artifact bytes, lengths/digests, configuration/input versions, execution/admission metadata and native provenance in a separate versioned result store. The small inventory avoids loading artifact bodies for listing/capacity checks. Limits are eight pins / 64 MiB across the repository, with 128 artifacts / 16 MiB output and 1 MiB metadata per pin. Atomic admission rejects capacity/verification failures without eviction or source edits; reopening verifies every digest before publication. Source exports and drafts remain independent.

The declarative **Pinned builds** expansion reuses shared selects, actions, stacks, alerts and metadata disclosure. Current and pinned runs/artifact selections have separate owners. Offline reopening does not build; returning to current or deleting a pin preserves its retained run and selected source. An unreadable pin remains selectable for deletion after output verification fails. Shared artifact preview handles relative links/CSS/images and exact transfers. A bounded toolbar contains expanded controls without covering preview links. Project/disposal generations, abortable pin reads and guarded repository opens prevent late reads or connections from surviving their owner; initial inventory loading follows configuration setup.

Pin verification: all 21 repository cases and 103 workbench cases pass, with the focused fresh-open/cancellation/site cases passing after the initialization repair. Tests cover exact UTF-8/CRLF/binary bytes, metadata/provenance, source/draft independence, wrong-project access, corrupt/incomplete/oversized records, concurrent count/aggregate limits, deletion/space reuse, retained current builds and late ownership. Runtime/Studio typechecks and lint pass. Final emitted/deterministic/deployed gates pass all 32 native schema examples, existing CRUD/URL/themes/responsive checks, seven-artifact site output, ordinary preview clicks and fresh offline pin reopening without a build. Output admits 184 assets / 279 files; deterministic SHA-256 is `14e066ea30b99d6f7cc918111dbf10c199b79e7c58d1886540343484d229a365`.

CEM audit for pins: native commands still own parsing, transformation, exact output and provenance. Studio services own bounded derived persistence, read cancellation and immutable run selection; visible composition/bindings stay in XHTML/CEM-ML using production shared controls. cem-elements owns artifact loading and passive preview rendering. No new browser capability, component paint/token defaults or Studio DOM renderer was introduced. Toolbar geometry/text wrapping preserves inherited themes and forced-color behavior. Native handles/trees/DOM/registry leases are not persisted or revived; historical references remain descriptive. Pins are local derived data, with original input bytes and portable pin archives outside this step. Source projects/drafts remain untouched in isolated fixtures; screenshots/attachments stay ignored and temporary typecheck preparation is removed. Comparison is implemented above; conservative native reuse and the previously tracked strict-definition cleanup remain next.


Explicit entry artifacts (2026-10-19): named conversion/query/transformation/graph configurations retain optional `entryArtifact` (`@entry-artifact` in CEM). Native validation owns the normalized relative-output grammar and compatible kinds; both projections retain Unicode/space paths and reject escapes, schemes, controls and query/fragment suffixes. The path is output metadata, independent of input resource IDs. URL/manual artifact selection wins; otherwise the configured entry wins, with legacy unambiguous selection retained when absent. A missing entry remains unavailable rather than falling back to another index. Captured run metadata and admission/staleness fingerprints include the setting.

Saved-project editing uses the declarative **Build entry settings** expansion, shared field/action/stack controls and existing host-command/project-edit services. Revision guards, validation, clearing, undo/redo, source/command drafts and active-build ownership remain enforced. Samples are read-only; their normal copies retain the setting. The Markdown site now declares `generated/index.html`, so a normal Build opens home without manually selecting an artifact. Portable copies preserve the default; deployed acceptance removes the artifact parameter before offline reopening and verifies the saved default and independent source selection.

Entry verification: all 15 native project cases and 103 Studio workbench cases pass, including rejection/round trips, default/URL/manual precedence, ambiguous legacy indexes, missing outputs, configuration switches, stale in-flight rejection, invalid-save repair, clearing, undo/redo and portable copies. Runtime/Studio typechecks, lint, all 32 native schema examples and emitted/deterministic/deployed/offline gates pass. Emitted output admits 183 assets / 278 files; deterministic SHA-256 is `f7373d0cbe6f9281431a1b33a42b6bc28a60d9dd1369f74d8bb4f0c88b0b62db`. A separate strict schema-definition probe retains the same five pre-existing identifier/path-facet errors on the baseline and current source; the lower-priority follow-up below records them. No additional strict-definition error remains.

CEM audit for entry artifacts: native cem-ml owns validation and CEM/JSON projection; Studio services own captured configuration metadata, selection/history transport and guarded persistence. Visible controls, help, error presentation and bindings are XHTML/CEM-ML using production shared components. Existing spacing, paint, target sizes, inherited themes, responsive geometry and forced-color contracts remain in force, with no new component CSS/token defaults or browser capability. Artifact bytes, digests, provenance and shared passive-preview policy are unchanged. Existing projects/drafts remain untouched in isolated fixtures; screenshots/attachments stay ignored and transient typecheck preparation is removed. Pinned runs, comparisons, dependency reuse and the pre-existing strict-definition cleanup remain separate work.

Ordinary site evidence: **Samples → Amsterdam 2026 → Markdown site** is discovered from `examples/projects/markdown-site/example.json`, with home, an article index, two articles, shared CEM-ML layout/content/chrome modules, explicit site/navigation data, CSS, SVG and a download. Three ordinary configurations validate article metadata, preview its body, or build all seven artifacts. Four exact HTML expectations and original static bytes define the baseline. Relative links work inside the immutable preview tree and when publishing `generated/` as a static root. Saved article/shared-footer edits, invalid metadata with retained success, repair, native content/partial provenance, portable independent copies and offline reopen/rebuild all use existing project services.

Site verification: all 102 Studio workbench cases, four native project/metadata cases, all three filesystem CLI configurations, exact bytes, deterministic reruns and named sidecars pass. The deployed navigation gate passes nested links/CSS/images, forced-color keyboard focus, unchanged source selection, CRUD/drafts/themes/responsive checks, saved edits/failure/repair, portable independent copies and offline Supplier/site execution. All 32 native schema examples, generated catalog checks and Studio lint pass. Emitted output admits 183 assets / 278 files; deterministic rebuilds produce SHA-256 `3eca946113a482b1833f4a6ec52a139643c1701b57597ed67b32f2b97b3ac20f`.

CEM audit for the site project: native import/conversion/export and retained metadata/provenance own semantics; portable CEM-ML templates own composition. Studio reuses shared declarative controls, captured builds and cem-elements artifact loading. Secured deployment first exposed parent CSP denial of the loader's object URLs; the authored response policy now admits image/stylesheet blobs while still denying blob script/worker execution and inline style attributes. No production JavaScript capability, component token or Studio rendering behavior was added. Exported static documents use browser system colors independently of the Studio theme. Existing projects/drafts stay untouched in isolated acceptance contexts, and generated screenshots/attachments remain ignored. YAML opt-in, project-owned relative `base`/navigation data, manual entry-artifact selection and the shared passive-preview limits are explicit; Hugo routing/Go templates, pinned historical runs and incremental reuse remain future work.

Native site metadata evidence: the Markdown content-type parameter `front-matter=yaml` opts into a bounded native YAML mapping. Ordinary Markdown keeps its existing interpretation. Reserved field types, duplicate keys, malformed/unclosed metadata, unsupported tags/aliases and byte/depth/value/token limits are enforced before publication. A retained lexical event preserves the header through all three Markdown formatter profiles. Metadata enters common native CEM data nodes; HTML recovery exposes it and the stable content URI to CEMT. Site data/navigation and layout/partial modules remain explicit graph dependencies. Metadata never initiates template loading or selects output paths.

Provenance evidence: Markdown writer frames name their original resource; indexed recovery preserves whole writer token frames without assuming character alignment from equal byte lengths. Native scalar interpolation retains data origins alongside template frames. YAML scanner positions become original UTF-8 byte ranges through sparse requested-coordinate lookups, including Unicode/CRLF and long scalars, with no per-character coordinate allocation. The native two-article fixture verifies distinct metadata/body source URIs and ranges, imported partials, navigation and failed-build atomicity. Native CLI sidecars retain the conversion frame and exact, distinct file boundaries. Generated/unmapped bytes acquire no invented content lineage.

Verification: native validation and CEM-QL suites, rooted-folder/source-provenance regressions, metadata/formatter/index/CLI cases and the native filesystem smoke pass. All 101 Studio workbench cases pass; final rebuilt-worker metadata and rooted-asset cases also pass. The same saved-project fixture verifies typed metadata, output provenance, validation failure, retained success and repair. Seven rebuilt Node host cases, runtime/Studio typechecks, targeted lint, Markdown package verification and final emitted/deterministic/deployed/offline checks pass. The emitted build admits 183 assets / 277 files, all 32 native schema-package examples pass, and deterministic rebuilds produce SHA-256 `c03d54cc3d41084d8031b4b6b8f89c67d0d6c7163bb26020f0f5aed8f8809215`. Deployed URL/history, model trees, CRUD, drafts, themes, responsive geometry and offline Supplier execution pass.

CEM audit: parsing, typed metadata, coordinates, bounds, conversion, provenance and export belong to shared native cem-ml/cem-ql; package CEMT owns formatting/composition. Studio production JavaScript, declarative controls and the shared preview are reused. Existing syntax roles, theme tokens, component styles and geometry are unchanged. Test resources are raw assets admitted through the ordinary project manifest, without an application data parser or serialized AST handoff. Existing saved projects/drafts remain untouched; generated screenshots/attachments stay ignored and excluded, and transient typecheck preparation is removed. Provenance stays at honest writer-token granularity. The ordinary site project and its editing/portable/offline acceptance are recorded above; Hugo/Go compatibility and automatic metadata routing/scheduling are outside this contract.

Native folder-build evidence: `import @root` exposes folder-relative path/directory bindings while retaining each original source address and the existing unrooted filesystem bindings. Recursive selectors include root files and nested files, and native hydration/lowering share one matcher. Rooted opaque imports publish mixed static trees with per-resource media types and exact bytes, without inheriting parsed-schema claims. Local destinations are canonical before duplicate admission, including an empty `{dir}` on root files. Native traversal checks cancellation and stops at 1,024 matches, depth 64 or 16,384 visited entries; it skips symlink entries. Windows filesystem addresses produce portable bindings without changing source identity. The Node adapter matches zero-directory recursion, bounds its existing candidate discovery and preserves the requested file URI root/encoding when returning concrete resources.

Folder-build verification: 15 focused native fixtures and 65 related native configuration/authoring/lowering/hydration/invocation checks pass. Initial fixtures reproduced missing root matches, wrong bindings, unbounded depth, the unrooted compatibility regression, Windows separators and an empty-directory output collision before repair. The native CLI smoke builds Markdown through the registered converter, publishes five outputs and two source-map sidecars, and preserves static HTML/CSS/binary bytes exactly. All 100 Studio workbench cases and seven emitted Node host cases pass; runtime/Studio typechecks and targeted lint pass. The Studio fixture uses an ordinary saved project with Markdown conversion, an opaque HTML tree, CSS/SVG/binary assets, immutable output bytes/digests, nested preview navigation, retained source selection and last-success ownership on preparation failure. The final native CLI smoke and focused Studio fixture pass against rebuilt packages, and the seven Node cases include actual publication from a directory containing spaces, ampersands and brackets. Emitted output admits 183 assets / 277 files; all 32 native schema-package examples pass. Deterministic rebuilds produce SHA-256 `8baa18841446df37d0532050f6add9dc0b3a5fdf4f271d6a4c41b0753a04fc1a`. Deployed URL/history, model trees, CRUD, drafts, themes, responsive geometry and offline Supplier execution pass. The added Node execution fixture caught raw-path and equivalent-encoding aliases causing repeated discovery; returning the requested URI root repairs both without rebinding identities.

CEM audit for folder builds: native parsing, file selection, bindings, conversion and export stay in cem-ml. The existing Node host owns filesystem I/O discovery; Studio's production JavaScript and declarative UI are unchanged and reuse captured command/output sets and the shared artifact preview. No app-local renderer, component JavaScript, new browser capability, theme token or paint/geometry override was added. Typed content stays in native retained artifacts; opaque publication is an explicit byte boundary. Markdown conversion retains native source-map frames/spans; distinct URI-level provenance through full site layout/partial composition remains in the next fixture, and static bytes claim no parsed character lineage. Existing user projects/drafts are untouched, generated screenshots/attachments remain ignored, and transient CLI typecheck preparation is removed. Native metadata/layout composition and the ordinary site's portable/offline acceptance are recorded above.

Artifact-tree evidence: `cem-media-preview mode=artifact-tree` uses a shared cem-elements registry and loader. The admitted complete output set owns immutable cloned bytes outside UI state; the declarative view receives only a handle and selected path. HTML links, directory indexes, queries/fragments, inline/linked/imported CSS and image URLs resolve within that run, with stylesheet-relative bases and encoded filenames. Query/fragment changes retain the document. Missing selections/assets remain explicit; failed/stale/cancelled attempts keep the previous success, and replacement/release revokes previous mounts and object URLs. Original native output bytes, digests, transfer policy and provenance remain unchanged.

Artifact-tree verification: two native retained-CSS cases pass after the initial missing-API failure, covering imports/conditions, escaped URLs, image-set, inline declarations, UTF-8, exact non-resource source and unsupported grammar. All 99 Studio workbench cases, seven preview cases, 24 shared preview/component/host-view plays and the final two run-admission/navigation cases pass. Runtime/Studio typechecks, three-package lint (warnings only), nine style-verifier cases and the 56-component/479-token audit pass. The targeted media-preview source and isolated-package gallery/property gate verifies nested CSS/SVG images, output selection, inherited themes and forced-color focus; it also repairs this component's older canonical-source path and quoted live/artifact examples. Emitted output admits 183 assets / 277 files, 32 native examples pass, and deterministic rebuilds produce SHA-256 `789cc4e3399d11394261156714ce2ee6d960ba869c4c3b1d565fc91a2a122eb7`. Deployed native builds enter the shared artifact frame, and navigation/CRUD/drafts/themes/responsive/offline checks pass in a fresh browser context. Existing saved projects/drafts are untouched. Screenshots/attachments stay ignored and excluded; temporary CLI typecheck preparation is removed.

CEM audit for artifact loading: required browser JavaScript was raised before implementation and lives in cem-elements; native retained CSS import/remapping stays in cem-ml, including its typed resource roles and complexity bounds. HTML is admitted at the exported-document browser display boundary, with no serialized/reconstructed data AST. Studio owns captured run resources and routing, while all visible controls/state bindings remain XHTML/CEM-ML and the shared host-command bridge forwards only an owning preview's scalar navigation value. Component CSS and public tokens are unchanged; output CSS has its own presentation independently of the Studio theme. The first browser asset fixture caught invalid individual-blob CSP expressions; the repaired loader admits only run-owned URLs and CSP denies network/scripts. Current limits remain 128 files / 16 MiB, 256 KiB selected HTML and the shared CSS import bounds. Fonts/media, responsive srcset, authored data/blob references, pinned historical runs and complete Markdown site layout/metadata composition are future work. Broader gallery canonical-link maintenance is recorded below, after Studio presentation work.

Independent preview evidence: shared `cem-workspace layout=project` keeps navigation, source/folder contents and preview in distinct responsive regions. Build configuration and artifact paths have separate `build`/`artifact` URL parameters, survive source browsing and history, and reopen without pretending an in-memory run was persisted. Named native configurations and derived `html-v1:<resource-id>` document builds use read-only captured repositories; samples are never imported by Build. The service retains one successful run for the current project/configuration, checks fresh repository input/configuration metadata before admission, and rejects late, cancelled or stale attempts. Failed attempts retain their diagnostics beside the last success. Related unsaved drafts mark that preview stale; Build does not save or discard them. Missing artifacts stay explicit. App update admission counts builds as active work.

Verification for this increment: the full 98-case Studio workbench suite and final focused source/preview/CRUD retention cases pass. Eight shared workspace and five shared source-editor plays cover responsive geometry, all five themes, inherited spacing, forced colors, native frame/editor retention, Enter/paste trailing newlines, undo/redo, composition and caret behavior. Runtime/Studio typechecks, three-package lint (existing warnings only), nine style-verifier cases and the 56-component/479-token audit pass. Emitted output verification admits 182 assets / 276 files; 32 native schema examples and deterministic rebuilds pass. Deployed navigation checks cover sample/saved preview selection, exact draft retention through Build and theme changes, bookmark/history, CRUD, three-region geometry and offline Supplier execution. Visual review verifies the meaningful idle status and natural source-editor placement. Screenshots/attachments remain ignored; temporary CLI typecheck schema preparation is removed.

CEM audit for this increment: the shared layout, canonical theme endpoints and existing declarative controls own UI geometry, paint and interactions. Studio owns project/run/routing orchestration without app-local DOM behavior. Native exports and bounded shared output readers remain the data boundary. Browser terminal caret placeholders are normalized in cem-elements, preserving meaningful source newlines and native edit history; the initial deployed fixture and a native Enter fixture reproduced the defect before repair. No component JavaScript, new token family/defaults, serialized AST replacement or sample-storage mutation was introduced. Source commands/results remain independent from project Build. Relative asset loading is completed by the artifact-tree increment above; explicitly pinned historical runs remain future work.

Evidence: run snapshots retain configuration/command revisions and hashes, declared input identities/revisions/hashes, the native admission ledger/policy and engine identity. The shared cem-elements reader verifies the complete output set within 128 artifacts / 16 MiB before admission. Studio publication receipts now preserve declared output URIs instead of replacing them with temporary memory addresses. A declarative selector changes preview, metadata, provenance and exact transfers together without changing source selection; transfer completion cannot label a later selection. YAML is declared text; binary/invalid text/oversized active markup retain download-only policy. Failed publications keep diagnostics and an empty manifest. Regenerated Supplier archives/catalog carry the updated project guide.

Verification: all 94 workbench, 14 example-project, seven preview and eight shared-reader cases pass, including native Supplier and one-document HTML execution; the final focused run verifies selected URI/digest metadata and exact bytes. Runtime/Studio typechecks and two-package lint pass (existing warnings only); Studio output verification admits 182 assets / 276 files. Temporary CLI typecheck schema preparation was removed. Generated screenshots and Vitest attachments remain ignored and excluded.

CEM audit for the run/manifest milestone: UI is XHTML/CEM-ML composed from existing cem-select, layout, table and media-preview components, with their canonical theme/spacing/accessibility contracts. No component/app UI JavaScript, token defaults or native transformation semantics were added. Shared browser output reading belongs to cem-elements; Studio owns captured project/run/selection state. Native source-map references and HTML spans are retained per artifact; constructed JSON/YAML exports do not invent character-level lineage. The later independent-preview increment above implements build/artifact URLs and last-successful ownership; the artifact-tree increment above completes relative loading; pinning remains pending.

## Shared contenteditable source editor (2026-10-18)

- [x] Add a reusable plaintext-editor capability in cem-elements for safe text values, browser-owned editing/undo/IME state, declarative selection and scalar host-command bindings.
- [x] Create declarative cem-source-editor with canonical CEM typography, surfaces, spacing and focus paint; publish property playground, gallery, contracts and catalog entry.
- [x] Use the shared editor for selected Studio files, including sample resources, while retaining project drafts, saving, URL selection and source navigation.
- [x] Fixture: cover escaped source, native multiline edits/undo, retained caret and composition, readonly/disabled state, selection requests, all-theme overrides, forced-color focus and Studio file editing/persistence; run package/runtime/Studio checks.

Evidence: the existing cem-demo-element playground supplied the plaintext-only/innerText precedent. New browser plays first exposed caret loss when authored text reconciliation crossed the native editable subtree; the shared behavior now owns live text while CEM owns its markup and attributes. Fifteen source-editor/form-control/host-view browser plays, 19 declaration contract unit cases and all 91 Studio workbench browser cases pass. Source and isolated-package property/gallery checks cover editing bindings, all five themes, readonly and forced-color keyboard focus. Runtime and Studio typechecks, three-package lint (warnings only), the nine style-verifier tests and 56-component/479-token audit pass. Studio output verification admits 182 assets and 276 files; Studio and Site module maps include the shared capability. Temporary CLI typecheck schema preparation is removed afterward.

Existing-profile verification opens the Login README through its file URL in the shared editor, retains its 14,297 characters as plain source, uses script typography and readonly sample semantics, and preserves existing storage values. Saved project editing, Save/Reload, diagnostics/source selection, draft recovery and navigation guards are covered by the Studio suite. Generated screenshots and Vitest attachments are ignored repository-wide and excluded from commits.

CEM audit: the source editor is XHTML/CEM-ML with scoped canonical script typography, input/palette/stroke/inset/coupling geometry and system-color fallbacks. Browser value, selection, IME and editing ownership belong to cem-elements, with no Studio-local editing JavaScript or public token defaults. Samples remain readonly until cloned; form/command textareas retain their existing roles. Syntax highlighting and language services remain future work.

## Studio navigation polish and existing-session verification (2026-10-18)

- [x] Fixture: verify folder icons on the opened project root and nested directories, leaving file labels and sample/saved-project catalog labels unchanged. The recursive declarative projection prefixes only project/directory names in the files tree; existing names, resource identities and catalog projections remain intact. The navigation/CRUD browser case verifies the rendered root, nested directories and file labels; Studio output verification passes (181 declared assets, 275 files). No UI JavaScript or theme overrides were added.

- [x] Separate the project trees from the opened project's files with a conditional semantic horizontal rule; use canonical separator/stroke tokens and existing navigation gaps. The existing navigation/CRUD browser case and Studio output verification pass.

- [x] Put the selection count before the contextual navigation actions, retaining the CEM coupling guard between controls.
- [x] Define unfilled contextual resting/disabled states in the canonical theme, preserving interaction feedback and inherited overrides.
- [x] Fixture: verify contextual button/link transparency and disabled ink in all five themes, contrast feedback and Studio action ordering/spacing; inspect the existing browser session without clearing storage.

Contextual toolbar evidence: the new surface fixture first reproduced the opaque resting fill. Canonical state-table contextual columns now own transparent resting/disabled backgrounds and 38% disabled surface ink; CSS generation, the selectable reference matrix and exported formula metadata honor the same columns. Other interaction endpoints remain unchanged. All 39 action/icon-button browser plays, the focused Studio navigation/CRUD case, theme Phase 13 checks, the style verifier's nine tests and 55-component/479-token audit, three-package lint and Studio emitted-output verification (181 declared assets, 275 files) pass. Existing Chrome inspection confirms `2 folders` before Create/Clone/Delete, transparent enabled and disabled controls, muted disabled ink, targets above the minimum and an 8px gap. Existing local storage values are unchanged.

CEM audit: contextual paint remains theme-owned and uses existing public endpoints, including their current aliases and inherited overrides. No new component defaults, token family or UI JavaScript was added. The toolbar keeps `max(--cem-gap-related, --cem-coupling-guard-min)` because D2 forbids zero-distance adjacent controls; hover/active/pending, contrast contours, focus/selection indicators and forced-color behavior retain their contracts.

- [x] Move selection actions and count into the Navigation heading using a shared declarative workspace header slot.
- [x] Compact tree rows without reducing operable targets or guard spacing; space project action forms with shared layout tokens.
- [x] Fixture: verify heading ownership, compact row geometry and action-dialog spacing alongside navigation and CRUD behavior.
- [x] Verify layout, trees, folder listings, URL selection and temporary Create/Clone/Delete in the existing browser profile; preserve saved projects and drafts.

Evidence: Chrome DevTools MCP connected to the existing Windows Chrome profile. A separate tab in that profile verified the default Samples root, Amsterdam/Supplier tree activation, child names/descriptions, folder and file URL updates, direct file-URL reload, matching window/page titles, toolbar count, and project/file Create/Clone/Delete. Two uniquely named verification projects were moved to Trash; no pre-existing projects existed at baseline. The original Supplier tab kept its URL, open empty Create dialog and all existing storage values; no draft keys existed before or after. No browser storage, caches or user tabs were cleared. Visual inspection confirmed the toolbar beside the Navigation heading, single-row tree geometry at the 48px target floor, and Create dialog padding/field/action gaps of 16px/16px/8px.

Validation: the full 91-case workbench suite passed during implementation; final focused workbench action/retention checks (2) and shared tree/workspace plays (17) pass. Lint for both packages, Studio typecheck, the 55-component/479-token style contract and emitted Studio output verification (181 declared assets, 275 files) pass.

CEM audit: navigation actions project through the shared workspace navigation-actions slot beside a separate semantic h2. Ordinary tree rows omit the hidden status track while loading rows retain it; minimum target, guard spacing, theme paint and forced-color rules remain intact. Dialog spacing belongs to shared stack/grid owners across projection boundaries. Existing inherited tokens own geometry; no app-local UI behavior, runtime capability, component token default or gallery-only fixture hook was added. The local Windows Node REPL adapter translates mounted-drive workspace URIs in MCP metadata without rewriting code, permissions or arguments.

- [x] Fixture: reproduce stale notices after file create/delete and dialog target loss, retaining external-change and draft guards.
- [x] Follow-up from existing-session verification: clear stale selection/revision notices after admitted structural operations; file deletion currently leaves a missing-selection notice while the parent folder and remaining files work.
- [x] Follow-up from existing-session verification: retain the selected target in project action-dialog titles (currently rendered as “create ·”, “clone ·” or “delete ·”).

Retry repairs: action display data uses `targetName`, respecting the documented CEM-QL `.target` reference-resolution helper. Selection availability/revision notices are derived from the current admitted session and repository records, so local structural refreshes do not leave an obsolete warning. Actual external Trash/Restore and divergent revisions still warn without replacing drafts. New dialog-title and post-create/delete assertions first reproduced the failures; all 91 workbench browser cases, Studio lint/typecheck and emitted-output verification pass after repair. Existing-profile recheck confirms named Create/Clone/Delete dialogs and no missing-selection notice after file deletion. The two recheck projects were moved to Trash; the original tab/dialog and all prior storage values, including the remembered selection, were preserved.

## Studio and engine commit verification (2026-10-16)

- [x] Complete shared icon-control installation and verify native theme selection, focus and storage.
- [x] Preserve command-view ownership of the operation-option slice and verify deployed typed forms; give the existing multi-step browser integration 30 seconds after measuring 17.9 seconds in isolation.
- [x] Repair compatibility packaging for full XHTML imports; verify seven authored samples, all five action intents and disabled transitions, painted geometry, and the locked archive inventory.
- [x] Revalidate native engines, CLI and Studio release gates; commit reviewed source while excluding generated screenshots and Vitest attachments.

Evidence: 2,090 CEM-ML unit tests, 124 CEM-QL unit tests, 13 native project tests and three srcset tests pass (two existing CEM-ML and four CEM-QL tests remain ignored). CLI and full Studio check targets pass, including all 94 Studio browser tests, 32 native and deployed Feature Tour examples, 171 declared deployment assets, 261 deterministic output files, clean consumers and synchronized release evidence. Deployed two-project acceptance covers typed Set/Reset forms, draft recovery, export, Trash/Restore, offline reload, resource rename/move, unchanged consumer bytes, revision guards, undo/redo and automated accessibility. The compatibility archive passes Chromium rendering, interaction/forced-color/geometry and JavaScript/type contracts with 384 locked files; changed scripts pass lint. Staging uses an explicit 205-file source allowlist and excludes all generated screenshots and Vitest attachments.

CEM audit: visible Studio UI remains composed from production declarative controls and theme-owned layout/paint tokens, with the operation selector's local state retained by its command view. Stable dataset IDs and resolver addresses remain independent from mutable display/export paths; path edits preserve consumer bytes. No app-local UI JavaScript, component-local token defaults, browser instrumentation prerequisites or consumer rewriting were added.

## Navigation controls and spacing (2026-10-16)

- [x] Fixture: cover navigation icon links/disclosure, exact aria-current forwarding and live aria-expanded updates, native navigation/disabled behavior, all-theme paint and inherited overrides, layout gaps/insets, matching control typography, keyboard focus and retained projection.
- [x] Add navigation-token paint and ARIA forwarding to cem-icon-button; compose the cem-nav disclosure from it and add shared token-driven layout/padding/gaps while retaining native content support.
- [x] Update bundle and navigation/icon-button property pages and galleries, navigation contracts and component reference; verify browser/package/build/lint and forced-color behavior, inspect the rendered result and record CEM audit.

Evidence: all 41 focused navigation/icon-button/theme-switch/gallery-page browser plays pass. The new controls/ARIA fixtures first reproduced the missing composed disclosure and lost aria-current values. Checks cover all five themes, current/hover/held-active/disabled paint, inherited color and spacing overrides, minimum hit targets, matching typography, trusted glyph clicks, Enter/Space/Tab behavior, retained projection and native-link compatibility. All ten source/package checks pass for the updated nav/icon-button property pages, galleries and bundle. Component build/catalog/package verification passes (53 catalog components, 299 packed files); component/script lint passes with the existing 121 component warnings. The navigation forced-colors gate passes with the composed disclosure. Final real-browser source/package bundle inspection verifies all five page modes, native fragment navigation, exact current-page ARIA, glyph/keyboard disclosure, padding/gaps, narrow LTR/RTL bounds and forced-color current paint; the updated component snapshot is saved with this task's outputs.

CEM audit: declarative shared icon-button composition; navigation uses existing --cem-navigation-item-* endpoints, D1 inset/gap and coupling guards, shared control hit geometry, and inherited nav typography. Native links and their discoverable ARIA-disabled activation guard remain supported; disabled icon links retain their existing URL/tab-stop contract. Explicit ARIA values remain owner-controlled. Compact native CSS nesting stays within existing import/specificity limits, with valid XHTML entity escaping. No new theme defaults/aliases, runtime behavior JavaScript, app-local UI code or gallery-only test attributes were introduced. Previously dirty files outside this task remain unchanged.

## Shared persisted gallery page theme (2026-10-16)

- [x] Implement declarative cem-gallery-page with normalized live page storage, page palette and controlled navigation theme selector; retain independent example keys and fixed theme scopes.
- [x] Migrate every source property/gallery/index/bundle page and publish the shared declaration with generated pages; preserve authored samples and navigation.
- [x] Fixture: cover defaults/saved/invalid themes, all five choices, live/cross-page synchronization, retained editors/forms and independent example themes/keys in worker and fallback runtimes.
- [x] Fixture and fix nested theme palette/state isolation in the shared theme generator so explicit example themes survive every outer page mode, including native.
- [x] Fixture and fix generic button value capture for clicks on nested icon/label content; initialize nested demo declarations through an inert page payload.
- [x] Fixture and use the existing viewport boundary for theme menus in page navigation, including real pointer activation near screen edges.
- [x] Extend whole-inventory source/package checks for theme changes, persistence and scope isolation; update gallery/package verification and docs, run build/lint/browser/package gates and record CEM audit.

Evidence: all 108 source playground/gallery/index/bundle pages and the component overview use the shared owner; all 479 authored sample blocks and 260 fixed theme roots remain byte-identical. All 52 focused browser plays pass, covering storage normalization/rebinding, all five choices, independent examples, retained forms/declarations, worker/fallback execution, nested theme paint/state isolation, descendant button values and viewport menu geometry. The whole source/package inventory passes all 218 page checks; final rebuilt source/package theme and bundle checks also pass. Real-browser checks confirm reload/navigation restoration, live cross-tab synchronization, retained edited preview fields, fixed matrix palettes, narrow navigation and forced-color selection. Theme/component builds, manifest validation, runtime/component/script lint (existing warnings only), runtime typecheck and npm package verification (299 packed files) pass. As in the earlier runtime work, typecheck temporarily materialized the CLI dependency's generated command-schema source from its existing schema and removed it afterward.

CEM audit: declarative page ownership, controlled navigation and existing live text storage; example keys and fixed theme roots stay independent. Canonical source tables own per-theme palette resets and bounded state stripes, preserving inherited brand colors and explicit root overrides. Existing popup viewport geometry and the generic runtime button-value decoder cover nested menu glyph clicks. No component/app UI JavaScript, component-local theme defaults, parallel token families or gallery-only test attributes were added. Unrelated previously dirty files remain unchanged.

## Shared icon alignment (2026-10-16)

- [x] Add shared align=start|center|end with centered defaults, logical inline alignment independent of row/column direction, and icon-button forwarding; start-align theme choices.
- [x] Fixture: verify live alignment and fallback, row/column and LTR/RTL geometry, button/link modes, retained projected content and theme menu selection/clearance.
- [x] Update both property playgrounds, variation galleries and contracts; verify browser stories, source/package demos, builds, lint and theme appearance; record CEM audit.

Evidence: all 35 icon/icon-button/theme-switch/menu/dropdown browser plays pass. Geometry checks cover default/empty/invalid values, live changes, row/column layouts in LTR/RTL, projected labels, icon-only/text-only content, button/link modes and retained controls. Build and lint pass (121 existing warnings). Ten source/package import/gallery checks pass; eight source/packaged icon demos additionally verify 24 live preview changes and all 64 alignment examples. Production-source theme inspection confirms start-aligned glyphs, automatic height and adjacent selection/focus clearance at desktop and narrow viewports, with forced-color selection preserved and scrolling only when viewport height is constrained. CEM audit: declarative shared attribute/CSS composition, contextual theme choices and existing inherited icon/action/coupling tokens; no new token defaults, runtime/component JavaScript or gallery test attributes. Other previously dirty file contents remain unchanged.

## Theme menu icon buttons (2026-10-16)

- [x] Replace theme choices with contextual icon buttons and separate sized Unicode glyphs from their labels.
- [x] Fixture: preserve menu radio semantics, selected indicators, keyboard navigation, persistence and menu clearance; cover icon-button role transitions.
- [x] Verify theme/icon-button browser stories, source/package gallery, build/lint and desktop/compact/forced-color menu appearance; record CEM audit.

Evidence: all 27 theme-switch/icon-button/menu/dropdown browser plays pass, including radio-role transitions, glyph sizing, all five saved themes, retained projection and keyboard/indicator clearance. Component build, lint (121 existing warnings) and source/packaged theme gallery import checks pass. Production-source browser inspection confirms 32px glyphs, automatic 360px menu height with all five choices visible at desktop and 328×420 viewports, adjacent selected/focused ring clearance and forced-color selection borders; a 240px-tall viewport correctly scrolls within its 232px panel. CEM audit: contextual intent and shared icon/action/coupling tokens are retained, with no token defaults, component behavior JavaScript or gallery test attributes. System uses the trigger’s half-disc glyph. Pre-existing dirty contents outside the seven intended files are unchanged.

## Automatic dropdown height (2026-10-16)

- [x] Fixture: reproduce the theme menu's unnecessary vertical scrolling and require all five items to fit when the viewport has room.
- [x] Remove the dropdown's fixed four-row cap, retain the shared viewport/boundary constraint, and document content-driven height.
- [x] Verify menu/dropdown/theme and viewport-boundary behavior, rebuild, lint, and inspect the gallery at normal and constrained viewport heights.

Evidence: the geometry fixture first reproduced 320px of menu content constrained to 192px. Removed both four-row limits from the dropdown source; the existing popup controller still owns viewport/boundary clamping. All 16 focused menu/dropdown/theme/surface-geometry browser plays, component build, source/package gallery acceptance and component lint pass (existing warnings only). Production-source geometry shows all five items at their natural 320px height with no overflow, including a 328×360 viewport; a 240px viewport correctly constrains the panel to 232px and permits scrolling. Selected/adjacent focus rings and forced-color outlines remain visible. The gallery preview was inspected; compact sizing was verified through an isolated fixture loading the same production declarations and theme stylesheet because a later cold-gallery probe timed out waiting for asynchronous theme CSS readiness. CEM audit: no row-count policy or new token/behavior added; existing shared menu spacing and viewport safety remain intact. Unrelated dirty-file contents are preserved.

## Template capability documentation (2026-10-16)

- [x] Document `{template @capability=...}` and equivalent HTML metadata in the form demo, runtime README and loading contract; demonstrate template-owned choice behavior in the existing form-input sample.
- [x] Correct obsolete fragment-import instructions and add component documentation caveats about metadata preservation, explicit overrides and component-specific capability names.
- [x] Fixture: verify all five form cases with the capability on the template, source-loaded and standalone; run focused unit/browser coverage, fixture acceptance and applicable lint/typecheck.

Evidence: the form introduction documents `{template @capability=...}` and equivalent HTML, registration timing, inheritance and explicit empty overrides. The existing fifth sample puts `choice-select` on its template while retaining all five legends and use cases. Runtime/loading docs and component contracts now explain source-owned defaults, copying metadata and the distinct form ownership of native-input wrappers; obsolete repeat-on-fragment guidance is removed. All 8 form source-contract unit checks, 7 form/source-capability browser plays, and standalone/source-loaded form fixture acceptance pass. Both lint targets and runtime typecheck pass; typecheck uses the existing transient CLI-schema preparation and removes it afterward. Twenty added local documentation links resolve. CEM audit: no component behavior, styles, tokens, new demo cases or test-only sample hooks added; form ownership and required intrinsic capabilities remain explicit. Unrelated dirty-file contents are preserved.

## Menu indicator spacing (2026-10-16)

- [x] Fixture: reproduce the theme menu's clipped selected zebra ring; verify popup-edge clearance and item separation with default and inherited spacing.
- [x] Add theme-owned inset and gap spacing to the shared menu composite, respecting the coupling guard; document the shared layout contract.
- [x] Verify menu/theme/dropdown interactions, inherited overrides and forced colors; inspect the updated menu and record the CEM audit.

Evidence: the original popup had zero inset/gap and clipped its selected 6px zebra ring. The shared composite now consumes the control inset and related gap, floored at one coupling guard around the content and two between items (8px/16px by default). A new geometry play reproduces both edge clipping and overlapping adjacent selection/focus rings before the fix, then verifies default spacing, inherited 20px/18px overrides and the safety floor under zero spacing overrides. All 17 menu, menu-item, dropdown and theme-switch plays pass, including native keyboard checks. Source/package theme-switch gallery acceptance, component build and lint pass (existing warnings only). The real gallery's selected ring, last-item scrolling, adjacent keyboard focus and forced-color outlines fit inside their available space; an updated screenshot was inspected. CEM audit: canonical D1/D2 tokens retain ownership and inheritance; no new tokens, component-local defaults, app behavior or test-only gallery attributes were introduced.

## Source-owned declaration capabilities (2026-10-16)

- [x] Fixture: verify capability inheritance from selected templates and their owning declarations, fragment isolation, explicit overrides, invalid capabilities and host-behavior conflicts; exercise real field submission, validation and reset.
- [x] Retain selected-source capability metadata before registration and remove repeated capabilities from the theme-switch gallery imports.
- [x] Run focused registration/form browser regressions, gallery checks, build, lint and typecheck; record the CEM audit.

Evidence: six source-capability browser plays cover actual field/text-field sources, prefix-mapped and relative imports, local/inline templates, custom declaration names, fragment selection and isolation, importer/template/owner precedence, and existing diagnostics. All 28 focused loading/form/registration plays and both source/package theme-switch gallery checks pass. Runtime and component builds, both lint targets and runtime typecheck pass; typecheck uses the existing temporary CLI-schema preparation, removed afterward. The fixture first reproduced the missing behavior before the runtime change. Capability metadata is retained on template clones without modifying cached sources or importer attributes; URL resolution and shared-document fetching remain unchanged. CEM audit: source declarations own reusable form/progress behavior; existing explicit overrides remain supported. No component CSS, tokens, app-local behavior or test-only gallery markup added. Pre-existing dirty-file contents are preserved outside the three intentionally edited files.

## Direct imports across demo pages (2026-10-16)

- [x] Replace resolver-slice declaration imports throughout component playgrounds and shared navigation; consolidate library folder mappings in source and package maps.
- [x] Update declaration-loading examples in the runtime module-URL demo while retaining URL controls used for visible links, images and referrers.
- [x] Fixture: check every component demo's source/packaged registration and gallery navigation; run runtime demo/unit regression coverage, builds and lint, then commit only this migration.

Evidence: 112 demo/navigation files adopt direct source imports, replacing 5,135 indirect declaration references; all six source/package import maps use component folder mappings. Component sample markup, declaration inventories and capabilities are preserved. The new `@epa-wg/cem-components:verify-demo-imports` gate passes all 218 source/generated pages, validates document fragments and page-owned registration, renders gallery navigation and requires one shared bundle-document fetch. All 41 focused browser plays, 32 unit checks, the native prefix-map fixture, and the module-URL standalone/source-loaded acceptance pass. Runtime, playground and bundle builds, architecture checks, both lint targets, verifier lint and runtime typecheck pass; typecheck uses the existing temporary generated CLI-schema preparation and removes it afterward. Existing component-owned duplicate-registration diagnostics were reproduced before and after migration; the verifier checks every page-owned diagnostic and nested acquisition/selection failure without treating those dependency repeats as page import failures. CEM audit: no component behavior, CSS, theme tokens or test-only sample markup added. Unrelated theme, Studio and cleanup work remains outside this commit.

## Direct cem-element module sources (2026-10-16)

- [x] Fixture: verify full XHTML prefix/exact mappings with the shared native resolver, then browser declaration loading with fragments, contextual maps, host resolvers, failed resolution and retries.
- [x] Reuse the cem-module-url resolver for bare declaration sources; split fragments first and retain relative/local source loading and host-loader compatibility.
- [x] Pilot explicit mapped XHTML sources in the theme-switch gallery; verify registration, interactions and document fetches before broader convention adoption.
- [x] Design follow-up: adopt the direct full-path convention across demo pages and shared gallery navigation, authorized on 2026-10-16; preserve component-owned relative declarations.

Evidence: 16 native module-resolution tests, 15 focused unit checks and 30 browser plays pass, including nine new declaration-source plays and the existing URL-control settlement, source retry, relative/nested source, URI policy, module/referrer demo and theme-switch cases. The real source gallery registers all 18 direct imports without declaration diagnostics or browser errors and switches through all five themes. Runtime build and lint pass (16 existing lint warnings). Workspace typecheck passes with the CLI dependency's generated command-schema source temporarily materialized from its existing generated schema, then removed; the unprepared typecheck target still encounters that dependency's missing transient source. The shared navigation keeps its legacy icon-button alias during this pilot. CEM audit: no gallery sample, component behavior, CSS or theme-token changes; existing capabilities and relative dependencies are preserved. Resolution failures are evicted only for declaration imports, preserving URL-control failure settlement across rerenders.

## Shared persistent theme menu (2026-10-15)

- [x] Replace theme radios with contextual sun/moon action and shared dropdown/menu; persist selection with declarative local-storage and default to light.
- [x] Cover selection styling, keyboard dismissal, reload persistence, independent scopes and retained projected controls in shared stories and Studio browser checks.
- [x] Update Studio composition, component galleries/reference/accessibility docs; verify inherited tokens, forced colors and rebuild deployment.

CEM audit: declarative shared action/dropdown/menu composition; contextual intent for trigger and all choices, selected zebra through native radio-menu semantics, existing coupling/typography/layering tokens, no component-local token defaults or app UI JavaScript. Added generic button-value capture and corrected focus transitions/return in shared runtime. Verified 36 shared stories, the final 9 theme/dropdown/reference-lifecycle checks, 11 Studio shell tests, lint/typecheck, component catalog/playground builds and static deployment output. Regular Google Chrome verified all five themes across actual reloads, mobile layout, forced-color selection and inherited contextual overrides. Tooltip: “Change theme”.

## Studio vertical rhythm (2026-10-15)

- [x] Add browser regression coverage for installed shared layout components, theme-driven section/block rhythm and inherited spacing overrides. Reproduced the unregistered cem-stack failure before the fix.
- [x] Register shared stack/grid/text components and compose Studio shell, forms and workbench sections with semantic D1 gaps; remove browser-default paragraph margins through shared text composition.
- [x] Verify Chrome layout at desktop/mobile widths, theme/spacing modes and forced colors; rebuild the running Studio deployment. Passed 30 combinations (390/1280px, all five themes, normal/dense/sparse), forced colors and inherited token overrides. Shell/workbench coverage: 66 tests verified, including the targeted dialog rerun after replacing its paragraph selector with shared text. Lint/typecheck and deployment output pass. CEM audit: shared declarative stack/grid/text own D1 section/block/related gaps and D6 typography; no local spacing constants, token overrides, CSS or JavaScript UI rendering. Existing production components retain coupling and theme behavior.

## Remove retired browser research implementation (2026-10-15)

- [x] Remove browser observation/replay hosts, native evidence/preparation/rebinding research, WASM exports and probe fixtures. Ordinary HTML srcset parsing/validation remains supported.
- [x] Remove retired public exports, build inputs and documentation; clean generated artifacts. CLI/WASM rebuilt without research symbols; stale packaged copies and temporary Chromium archive removed, plus approximately 7.3 GiB of obsolete compiled research tests/probes.
- [x] Verify retained native project/command behavior, browser worker execution and Studio build. Native CEM-ML: 2,089 unit tests, 13 project tests and 3 srcset parser tests pass (2 pre-existing ignored tests). Regular Google Chrome CLI worker/command suite and CLI/Studio lint pass; 50 native renderer tests pass (2 pre-existing ignored tests); Studio typecheck, deterministic deployment, 32 native Feature Tour examples and deployed two-project lifecycle/accessibility acceptance pass in regular Google Chrome. Rename/move command execution retains unchanged consumer content. CEM audit: existing native dataset ownership and declarative UI remain intact; no UI styling or theme-token changes.

## Studio project dataset identity correction (2026-10-15)

- [x] Verify native resource references survive path/name changes, serialization and reload; reject ambiguous references. (13 native project tests pass; the research-only SVG tests were subsequently removed with their implementation.)
- [x] Replace Studio rename rebinding with a project dataset metadata edit; preserve consumer bytes and IDs.
- [x] Verify rename, move, collision, stale revisions, undo/reload and command execution through unchanged references (repository/workbench tests and deployed lifecycle in regular Google Chrome).
- [x] Retire custom Chromium checkout, private origin-provider research, build integration and obsolete Studio requirements. Removed approximately 54 GiB of checkout/build/depot-tools files; the temporary patch/notes archive was also deleted during the research-code removal.
- [x] Verify Studio command-service entry point excludes live DOM/paint observation modules from deployment (171 declared assets; 261 deterministic output files).
- [x] Run native and Studio regression checks and record filesystem reclamation and CEM audit. Passed: 13 native project tests, 76 Studio browser tests (repository/workbench/file-system provider), Studio lint/typecheck, deterministic deployment and deployed two-project lifecycle/accessibility acceptance. CEM audit: native dataset identity owns references; existing declarative UI projects committed metadata; no consumer rewriting, application DOM-rendering path, new styling or theme-token changes. The custom browser checkout and private probe integration are removed.

# Todo

This file is the authoritative checklist for active execution work.
Product/module sequencing lives in [roadmap.md](../roadmap.md), and future
wishlist work lives in [wishlist.md](wishlist.md).

Completed datatype/reference contracts, native-surface capabilities, component
migrations and verification evidence are preserved in the
[2026-10-10 snapshot](archive/todo-snapshot-2026-10-10.md). Its checkbox states are
historical; only the unfinished actions below govern execution. Adopted contracts
remain in their design documents. Earlier history is linked from the snapshot.

Current priority (2026-10-17): **Studio functionality and Amsterdam presentation
samples**. Graph-edge editing and its validation are complete. Make
the [required tutorial checkpoints](declarative-amsterdam-2026/presentation.md#required-tutorial-checkpoints)
available as reproducible Studio samples. Implement functionality needed by
the presentation before unrelated Studio features, device acceptance or helper
work. The [active sample checklist](#amsterdam-presentation-samples-active-studio-priority)
tracks that work. Device/browser/AT acceptance remains open, with its existing
criteria; it does not lead this implementation queue.
The [milestone contract](cem-studio.md#next-delivery-milestone-user-owned-project-workspace)
defines scope and acceptance; its ordered implementation checklist is below.
Studio/Site declarative UI migration is complete. All 54 CEM components are
canonical and the legacy registry/installer is retired.
Use application needs to polish the shared library against Angular Material
functionality: implement useful missing controls in `cem-components` first,
with reusable JavaScript behavior in `cem-elements` and CEM theme styling in
the declarative component. Application JavaScript owns non-UI services only.
Presentation plans and resume context remain preserved below. The required
sample functionality is active; deck, website and rehearsal work stays separate.

Standing requirement: CEM theme principles govern every design decision and
completion review. Check semantic intent, bounded variation, accessibility,
cross-dimension harmony, theme survivability and theme-owned inherited tokens
against the [canonical principles](../packages/cem-theme/src/lib/tokens/index.md#core-cem-principles).
Passing a token-name check alone does not establish semantic conformance.

### Unified example-project workspace: completed 2026-10-17

- [x] Update the accepted Studio design: predefined examples and user-created or
  customized examples are ordinary projects opened through the same workspace.
  Amsterdam identifies catalog content and presentation use, not dedicated UI.
- [x] Add fixtures proving predefined and imported/custom projects share project
  admission, resource editing, command execution, validation, draft guards and
  independent persistence. Remove numbered presentation-page expectations from
  the generated examples and deployed acceptance fixture.
- [x] Replace example-specific startup/navigation with a generic example-project
  catalog and the shared project/resource/configuration workflow. Preserve old
  project identities, saved bytes and portable import/export compatibility.
- [x] Update example generation, docs, handoff and remaining presentation todo
  around project contents and commands; verify native/browser/offline delivery
  and record the shared-component audit before marking the correction complete.

Implementation: generic catalog projects and a neutral starter use hash-checked
portable archives. Home creates independent ordinary projects; startup restores
repository selection without installing a specific example. Login definitions
now live in `examples/projects/login/example.json` with resources, configurations
and README guidance. Amsterdam is collection metadata. The UI exposes
configurations in command targeting, structure editing and search; native v1
`entries`, `entryId` and `apply-command-page` remain compatibility fields.
Existing saved bytes/identities are preserved. Old Feature Tour catalog-only
operation presets require Apply or a fresh Schema examples copy; startup never
rewrites them. Legacy factory/mount exports alias the generic workspace APIs.

Verification: 3 native Login fixtures and all 116 Studio browser tests pass.
The complete 51-task Studio gate passes, including generated native project
validation, shared custom/imported/example lifecycle, README resource access,
all 5 unnumbered Login configurations, deliberate failure/repair and repeat
failure, independent copies, offline deep-link recovery, all 5 portable commands,
32 schema-package examples and clean consumer installation with one runtime.
Two clean builds produce 264 identical files and 171 declared module assets
(aggregate SHA-256
`4febabb624400682faf214ca61d1dd1e33e28330c6bd7f9212ed187846d7b935`).

CEM audit: all visible changes are XHTML/CEM-ML compositions of shared form,
select, action, dialog, tree, alert, list and textarea controls. Catalog/archive
loading remains non-UI host orchestration. No new UI JavaScript capability,
application CSS or token defaults were introduced. Inherited token overrides,
five themes and forced colors pass shell delivery. The combined lifecycle has
69 named accessible controls, valid relationships, visible keyboard focus and
no horizontal overflow at 320/390/1280 pixels. Physical device/AT acceptance
remains separately open. Generated screenshots and Vitest attachments are
excluded from Git.

### Amsterdam presentation samples: active Studio priority

- [x] Locate the existing presentation feature list before choosing sample scope.
  Use [required tutorial checkpoints](declarative-amsterdam-2026/presentation.md#required-tutorial-checkpoints)
  and the [required login demonstration](declarative-amsterdam-2026/presentation-plan.md#required-login-demonstration-and-companion-examples):
  read/edit, validate/fail/repair, typed inspection and formatting, HTML alternate
  input and conversion, supported query/template processing, and a light-DOM
  `cem-element` view. The supplier pipeline is a companion after the core login
  journey. These existing lists remove the need to invent a replacement scope.
- [x] Audit existing login/HTML fixtures and presentation assets against every
  required checkpoint; identify the precise missing Studio functionality first.
  The [Studio sample audit](presentation/declarative-amsterdam-2026-handoff.md#studio-sample-audit-2026-10-17)
  records existing sources, current native validation, the missing sample entry,
  and the separate failure/repair, HTML, query/template and live-view proof gaps.
- [x] Deliver the first availability increment: canonical login CEM-ML and HTML
  resources with native parse/inspect/validate/conversion pages and independently
  editable sample copies. Add native command and browser copy/save/run fixtures
  before implementation; retain the three existing login reference warnings.
  - [x] Freeze four command-page definitions in `fixtures/amsterdam-login/sample.json`
    and add native CLI fixtures for their output, known warnings, a heading edit,
    both form controls and plain saved text. Both native tests pass.
  - [x] Add declarative sample selection and independent project copying, then
    verify copy isolation, guarded opening, Save/reload and selected browser runs.
  - [x] Expose the existing persisted-validation service through a shared
    declarative action; add a browser fixture proving validation preserves an
    unsaved draft and durable source before wiring that action.
  - [x] Add a native invocation fixture proving identical relative command argv
    bind to independent virtual project roots while filesystem behavior remains
    intact; implement native URI working directories and bind Studio's host
    context before enabling the copied presentation command pages.
  - [x] Complete the full Studio delivery gate, including the expanded sample
    cache inventory, combined lifecycle, browser suites and release evidence.
  - Implementation: a generated, hash-checked archive contains the two canonical
    sources and four native command/configuration pages. Home composes sample
    selection/copy actions declaratively; new copies keep resource IDs and
    reference paths while receiving independent project identities and names.
    Native invocation resolves relative argv under a virtual project directory;
    Studio supplies the owning root without rewriting command bytes. The
    persisted-validation action preserves unsaved text and marks differing drafts'
    results stale. Existing shared controls own form, focus, spacing and palettes;
    no new UI JavaScript capability, application CSS or token defaults were added.
  - Verification: 2 native CLI fixtures, all 5 native invocation tests and
    2 focused browser copy/validation checks pass. Deployed four-page acceptance
    passes native parse/inspect/HTML conversion, validation with the existing
    three reference warnings, heading Save/reload, independent second copies
    and offline execution. The full Studio check passes all 51 tasks, including
    112 browser tests, the combined lifecycle and automated AX/reflow checks,
    expanded offline cache inventory, shell delivery and clean consumer release
    evidence. Two clean builds produce 262 identical files with 171 declared
    module assets (aggregate SHA-256
    `68d928dae22117f390308e5153b25dac762e490b2eb0d5df7113d266ba10dfb3`).
    Generated screenshots and Vitest attachments are excluded from Git.
- [x] Deliver the Amsterdam intentional validation failure and repair checkpoint.
  - [x] Freeze the existing teaching schema/package and syntax-valid login with
    missing `@method=post`; add native CLI fixtures for exit status, the exact
    `demo.login.method_required` diagnostic and retained form source span, then
    verify the repaired bytes return zero errors with the three existing warnings.
  - [x] Add browser fixtures for owned schema-package loading and innermost
    retained diagnostic-span selection before wiring validation, parse and
    inspection to the page's schema dependencies.
  - [x] Add the fifth generated sample page and declarative checkpoint guidance;
    verify diagnostic selection, saved repair/reload, reintroduced failure and
    independent copies offline through the deployed presentation fixture.
  - [x] Update sample packaging/release assertions, documentation and handoff;
    run the full Studio delivery gate and record the shared-component audit.
  - [x] Add a public CLI browser-facade fixture for initialized native graph
    authoring, route Studio's existing graph helper through that facade and
    preserve the strict dependency audit. The full gate exposed a pre-existing
    direct runtime import when Nx refreshed the edited service's dependencies.
    The facade fixture and strict audit now pass; the module maps retain only
    Studio's actual CLI import. The final delivery rerun passes on the corrected route.
  - Native evidence: 3 Amsterdam CLI fixtures pass. The deliberate failure is
    syntax-valid, has exactly one `demo.login.method_required` error with the
    retained `{form` span on line 8, and repairs to the exact canonical bytes.
    Both validation states retain three reference warnings. Two initially
    failing browser fixtures now pass for owned package registration, native
    package-identity preservation, parse/inspection, unreadable-schema rejection
    and innermost diagnostic selection. Lint and typecheck pass. The deployed
    five-page fixture passes source selection, persisted-versus-unsaved repair,
    repair/reload, repeat failure/repair, independent copies and offline execution.
    The full 51-task Studio gate passes with 115 browser tests, the strict
    dependency audit, synchronized release evidence and clean consumer install
    with one physical runtime copy. CLI SDK lint and Studio lint/typecheck pass.
    Two clean builds produce 262 identical files and 171 declared module assets
    (aggregate SHA-256
    `3192ec1092f4157da27a9fdaeaf02532dbb82f4d781ee3f08e0a3b5a63a2c3e9`).
    Generated screenshots and Vitest attachments remain excluded from Git.
  - CEM audit: existing shared alert/list/textarea/action compositions own the
    checkpoint guidance, diagnostic activation, focus and editing. Studio only
    projects page descriptions and native byte ranges and orchestrates owned
    resource loading. No new UI JavaScript capability, application CSS, token
    default or component-local behavior is introduced. The native teaching rule
    remains the semantic owner; source-map frames and existing warnings survive.
    Shell delivery verifies inherited token overrides, five themes and forced
    colors; combined lifecycle AX relationships and 320/390/1280 reflow pass.
- [x] Add native fixtures for the agreed HTML/CEM semantic comparison and the
  supported HTML import/query route, then add the resources and configurations to the ordinary Login project.
  Resolve the recorded `cem.xpath.query_input_unsupported` boundary in shared
  native import/query code; keep the XML slide projection distinct from HTML.
  - [x] Add failing native HTML import fixtures for typed-tree namespace/value
    semantics, boolean attributes, entities, original source ownership/ranges,
    foreign content, recovery and bounded input; extend the consumer boundary guard.
  - [x] Add failing native Login query fixtures proving HTML and CEM-ML yield
    the same explicit semantic field sequence, preserve input bytes/provenance,
    and detect meaningful saved edits through the common XPath query path.
  - [x] Add namespace-view fixtures proving XPath consumes original ready literal
    namespace metadata without source mutation, and rejects pending bindings or
    name metadata from another source owner.
  - [x] Register HTML import in the shared native boundary and admit retained
    CEM trees to XPath without format-specific evaluator branches or serialized ASTs.
  - [x] Add ordinary Login query resources/configurations and README comparison
    guidance, then browser/deployed offline fixtures for both inputs and saved edits.
  - [x] Update docs, handoff and delivery evidence; run native and Studio gates,
    audit shared components, then commit/push while excluding generated attachments.
  - Native evidence: 66 selected tests pass: 4 Login CLI fixtures, 17 common
    CLI-query regressions, 23 typed-tree/import-boundary/namespace fixtures and
    22 import-limit/string/XML-view/node-accessor regressions. The real browser
    worker also passes both 21-field contracts, saved heading/method edits and
    original node provenance. Source bytes and native CEM arenas stay unchanged;
    pending namespaces and metadata from another owner are rejected.
  - Delivery evidence: the deployed offline lifecycle passes both complete
    contracts, the saved CEM heading edit, unsaved/saved/restored HTML method
    edits, exact reload, schema failure/repair and fresh independent copies.
    `projectSemanticComparison` is required release evidence. Accessibility
    relationships and 320/390/1280-pixel reflow pass. Two clean builds produce
    264 identical files and 171 declared module assets (aggregate SHA-256
    `6e672889f066b367e599c3b24e0c029a420350650e52f54cb6be50a8abafaa25`).
    The full 51-task Studio check passes with 117 browser tests, lint/typecheck,
    strict dependency verification, synchronized release evidence and a clean
    consumer install with one physical runtime copy. Shell theme inheritance,
    forced colors and offline reload pass. Generated screenshots and Vitest
    attachments remain excluded from Git.
  - CEM audit: the comparison is ordinary Login resources and configurations.
    Existing shared workspace, editor, result and diagnostic components own the
    UI. Native HTML policy stays in the registered parser/common import boundary;
    XPath consumes retained typed nodes and ready same-owner name metadata. No
    Studio UI JavaScript capability, app CSS, token default or serialized AST
    bridge is introduced. Stable resource/configuration IDs remain intact and
    existing project copies retain their saved contents.
- [x] Add native fixtures for equivalent XPath/CEM-QL queries, then add their
  Login project resources/configurations and browser/offline evidence.
  - [x] Add native Login fixtures comparing complete XPath/CEM-QL field values
    for both canonical inputs and meaningful saved source edits; verify retained
    source ownership/ranges and distinguish empty from absent attributes.
  - [x] Add failing native API/CLI fixtures for explicit ready-name admission:
    preserve ordinary authored views and source owners, reject pending/foreign
    namespace capture without evaluation, and preserve reference reload export.
  - [x] Expose that opt-in through shared native query orchestration and the CLI;
    reuse same-owner namespace completion, keeping global defaults unchanged.
  - [x] Add failing native query-budget fixtures and apply authored bounded
    scope policy through shared orchestration, retaining default limits and
    invalid-budget diagnostics rather than raising global limits.
  - [x] Add native validation fixtures for the query module's implicit `input`
    host binding; make valid editable query resources validate while preserving
    rejection of unknown variables and ordinary library-module behavior.
  - [x] Author and natively format the equivalent CEM-QL resource(s), preserving
    explicit namespace/mirror mappings and inert native source semantics.
  - [x] Add ordinary Login resources/configurations using the existing workspace
    and expected contract; keep stable IDs and existing project copies intact.
  - [x] Add native portable-command fixtures for ready-name lowering and query
    source validation, and trusted-host ceiling fixtures proving commands cannot
    raise inherited limits. Compose the existing CEM-QL adapter into the single
    common WASM runtime; keep runtime registration and host authority native.
  - [x] Align namespace-ingress fixtures with full-owner XPath admission and
    stale-input rejection; prove explicit re-preparation restores readiness.
  - [x] Add browser-worker and deployed offline fixtures for both languages,
    saved/unsaved input and query-module edits, and independent copies; update
    docs/handoff, run gates, audit shared components, then commit/push without
    generated attachments.
  - Native evidence: 116 selected tests pass: 5 Login CLI fixtures, 19 common
    query CLI regressions, 7 module-query/source-validation/host-ceiling fixtures,
    11 namespace-ingress fixtures, 53 native command/protocol cases and 21
    import/typed-tree regressions. Both languages match all 21 values across
    canonical sources, heading/method edits, empty/absent `required` and renamed
    namespace prefixes. Source owners/maps and reload exports remain intact;
    pending/foreign capture, stale preparations and host-limit escalation reject.
  - Delivery evidence (2026-10-17): the 51-task Studio check gate passes,
    including all 117 browser cases, lint/typecheck, 32 packaged schema examples,
    online/offline worker execution and a clean consumer with one runtime copy.
    Deployed offline checks edit, validate, rerun, reload and restore the query
    module; both source formats and independent copies retain exact results.
    Two clean builds match across 264 files with aggregate SHA-256
    `168729654d457b267a8ddf91fd41ee377ab7ce431a94ba5235c8305849e76864`.
    Debug-control and stripped common-runtime builds pass initialization,
    rendering and debug-capability assertions. Theme inheritance, container
    overrides, forced colors and offline shell reload pass. Generated
    screenshots and Vitest attachments remain excluded from Git.
  - CEM audit: ordinary project resources/configurations reuse the existing
    declarative workspace, editor, diagnostics and Results controls. No new UI
    JavaScript capability, app CSS or token defaults are introduced. The only
    Studio host change supplies an explicit bounded worker ceiling. Ready names,
    query validation, command lowering and runtime composition remain native
    shared capabilities over retained source owners, without a serialized AST
    bridge. Stable IDs and saved project copies remain intact. Generic query
    source Parse/Inspect and standalone-expression formatting remain explicit
    limits, scheduled after the remaining presentation functionality.
- [x] Add native fixtures for template output, then add its Login project
  resources/configurations and browser/offline evidence.
  - [x] Add native Login template fixtures for complete HTML output from both
    canonical inputs, saved heading/label/method edits, Boolean attribute
    presence, escaped content and unchanged source bytes. Keep source
    declarations inert and name/attribute selection explicit in the template.
  - [x] Author and natively format a shared CEM-native Login view template and
    expected output as ordinary project resources; add transformation
    configurations without changing existing IDs or saved project copies.
  - [x] Add failing native renderer fixtures for constructed HTML void elements,
    foreign namespaces and XML output; fix shared HTML serialization so layout
    children cannot introduce invalid void-element end tags.
  - [x] Repair the packaged DOM projection converter fixture to retain typed
    preludes when copying its source document, then run converter parity
    regressions against the shared HTML writer.
  - [x] Add a failing native template-schema fixture for the documented layout
    whitespace policy; admit the shared annotation through the owning schema
    while preserving unknown-annotation diagnostics and native formatting;
    update emitter fixtures for the newly enumerated core annotation.
  - [x] Add browser-worker and deployed offline fixtures for input/template
    edits, Save/validate, exact reload, baseline restoration and independent
    copies through the existing shared workspace and Results controls. Open
    Results and bring the shared lazy document preview into view before
    inspecting its frame, then return to the source editor.
  - [x] Update project instructions, docs and presentation handoff; verify
    native/browser/delivery gates, record the CEM audit, then commit/push while
    excluding generated screenshots and Vitest attachments.
  - Native evidence: 500 selected tests pass: 5 Amsterdam sample-command
    fixtures, 3 Login template/validation fixtures, 12 HTML/XML serialization
    and whitespace fixtures, 26 renderer regressions and 454 core schema tests.
    Both sources match the complete HTML artifact across heading/label/method
    edits, empty/absent Boolean attributes, escaped text/attributes and renamed
    CEM namespace prefixes. Template edits change only the instruction; malformed
    templates report their source URI and publish no partial HTML. Native tabular
    formatting is stable and template validation reports no diagnostics.
    The template package's manifest/CLI gates and both packaged converter
    compile/parity regressions also pass. The focused browser worker verifies
    the exact artifact and accessible form semantics from both inputs and edits.
  - Focused deployed evidence: all eleven configurations pass offline through
    the common workspace. Source/template edits, exact Save/reload, restored
    baselines, repeated schema failure/repair and independently rendered copies
    pass. The fixture opens Results before inspecting the shared lazy iframe;
    no component visibility or loading behavior is changed for the test.
  - Delivery evidence (2026-10-17): the full 51-task Studio check passes,
    including all 118 browser cases, lint/typecheck, 32 native/deployed schema
    examples, online/offline worker execution, project/template persistence,
    copy isolation and automated accessibility. Theme inheritance, container
    overrides, forced colors and offline shell reload pass. Two clean builds
    match across 264 files with aggregate SHA-256
    `ff2a1011cbc78e5dbcf49833980588385d441186ebc169e392526f8fb4ada8f4`.
    A clean consumer verifies static bootstrap with one runtime copy, and
    synchronized release evidence includes `projectTemplateOutput`. Generated
    screenshots and Vitest attachments remain excluded from Git.
  - CEM audit: ordinary project resources/configurations reuse the declarative
    workspace, editor, diagnostics and shared `cem-media-preview` Results view.
    No new application UI JavaScript, shared UI capability, CSS or token defaults
    are introduced. The native writer owns HTML void-element semantics and output
    ranges while preserving foreign/XML serialization. The owning schema admits
    its documented static whitespace policy and retains unknown-value/annotation
    guards. Typed source owners remain native, without a serialized AST bridge;
    the bounded Login projection does not infer namespace bindings or execute
    source references. Stable IDs and saved project copies remain intact. Login
    graph provenance and the live light-DOM view remain separate work.
- [x] Add a Login transformation-graph resource/configuration using the view
  template; add native and browser/offline fixtures for execution, graph editing
  and generated-output source provenance through the existing graph tools.
  - [x] Add native fixtures for graph-produced HTML, graph/template/source edits,
    output ranges and durable source identities; cover invalid graphs and missing
    dependencies before exposing the project configuration.
  - [x] Add the Login graph and configurations as ordinary project resources,
    reusing its existing view template and expected HTML with stable IDs.
  - [x] Verify generated-output provenance identifies the owning project resource
    and exact UTF-8 range, including valid EOF carets; retain draft/revision guards
    and use existing declarative graph/results controls for navigation.
  - [x] Add browser-worker and deployed offline fixtures for graph editing,
    generated results, source navigation, saved reload and independent copies.
  - [x] Add a disposed-refresh fixture and stop canceled navigation refreshes
    before they can issue follow-up storage queries or reopen IndexedDB.
  - [x] Fix sample-generator dependency ordering for its shared package imports
    and verify it through the existing clean-build/delivery fixtures.
  - [x] Update docs/handoff, run appropriate native/Studio gates, record the CEM
    audit and remaining limits, then commit/push without generated test artifacts.
  - Native evidence: 155 selected tests pass (124 CEM-QL library tests, 14
    URI/serialization/whitespace integration cases, 7 core provenance/codec
    cases and 10 Amsterdam/Login CLI cases; four profiling-only tests remain
    ignored). Fixtures cover both source formats, source/template/graph edits,
    exact UTF-8 ranges after a non-ASCII comment, invalid graphs and missing
    templates without partial HTML publication. Dev artifact hashes distinguish
    source URIs; Prod artifacts strip maps; copied native input retains its own
    URI/range. The canonical graph uses the native tabular formatter.
  - Delivery evidence (2026-10-17): the serial 51-task Studio check passes,
    including all 129 browser cases, lint/typecheck, 32 native/deployed schema
    examples, online/offline worker execution, graph/template persistence,
    source selection, repair/reload and independent project copies. Explicit
    fixtures cover stable resolver addresses after display-path renames,
    Save/Discard/Cancel admission, stale/foreign origins, UTF-8 boundaries,
    valid EOF carets and disposed refreshes without storage reopening.
    The deployed fixture explicitly chooses the retained Diagnostics/Provenance
    tabs and awaits asynchronous provenance navigation. Browser gates run
    serially with their existing timeout limits. Theme inheritance, container
    overrides, forced colors and offline shell reload pass. Two clean builds
    match across 264 files with aggregate SHA-256
    `f79ce8f431d16eb4ce9d95289c8b76609bf1a1da6876a28c2723725cffb71387`.
    Clean consumer installation and synchronized release evidence pass with
    `projectGraphProvenance`. Generated screenshots and Vitest attachments
    remain excluded from Git.
  - CEM audit: the graph and command are ordinary Login resources/configurations.
    Existing declarative graph forms, Results, `cem-list` and the shared source
    editor provide all UI. Studio JavaScript only coordinates native protocol
    projections and guarded repository routing; no new UI JavaScript capability,
    CSS, token defaults or AST serialization bridge is introduced. Resource IDs
    and resolver addresses remain independent of display paths. Native source
    boundaries name actual owners instead of guessing numeric parser IDs into
    paths. Constructed/computed values map to template instructions/expressions;
    arbitrary per-character data lineage is outside this change. The sample
    generator now declares the built packages and source modules it imports.
    Canceled navigation refreshes stop before follow-up storage queries, so a
    disposed workbench cannot reopen IndexedDB through its background refresh.
- [x] Implement the live light-DOM `cem-element` login view and add browser
  evidence that a saved edit visibly changes it. Keep Studio UI declarative;
  add any missing reusable control/capability to the shared packages first.
  - [x] Design and implement the reusable live-preview boundary in shared
    packages, with explicit resource ownership and lifecycle cleanup. Alert the
    user if a new JavaScript UI capability is required in `cem-elements`.
  - [x] Add shared preview fixtures for script/navigation/storage/network denial,
    UTF-8 resources, interactive light DOM, property controls, semantic token
    styling/overrides, replacement/disposal and independent frames.
  - [x] Add the live Login view as ordinary project resources/configurations,
    then prove saved edits, reload and independent offline copies through native
    and browser fixtures. Keep example storage/theme state independent of Studio.
  - Verification: five native Amsterdam sample fixtures, 130 Studio browser
    cases and 16 shared preview/component/form-control plays pass. The deployed
    workspace proves real pointer/keyboard interaction, persisted versus unsaved
    templates, saved HTML data, offline reload, independent copies and the
    unchanged Studio theme key. Normal/forced-color paint, inherited spacing
    overrides and automated accessibility pass. Parent-realm inspection keeps
    Playwright's child-realm listeners out of the script-disabled frame; native
    input is retained and the zero-console-error assertion remains strict.
    The full serial 52-task Studio check passes, including synchronized release
    evidence and clean consumer installation. Runtime lint/build and Studio
    lint/typecheck pass (runtime story non-null assertions remain lint warnings).
    Two clean builds match across 268 files with aggregate SHA-256
    `9f3f1e21b5f4a5c49a09754d29b13ddfc747cd844ac0a17c11af2ae95cb12eee`.
    Generated screenshots and Vitest attachments remain excluded from Git.
  - CEM audit: `cem-media-preview` remains a declarative shared component. Its
    new `live-preview` capability, scope/iframe lifecycle, explicit resource
    admission and cross-realm control handling belong to `cem-elements`; the
    user was alerted before adding this reusable JavaScript capability. Studio
    only transports saved original source/resource text and shared library assets.
    JSON names the transport envelope; native imports retain the data model.
    Declaration CSS is preflighted before mounting; no component-local public
    token defaults or Studio-owned DOM/event handlers are added. Login consumes
    group gaps, container insets and comfort colors, accepts inherited token
    overrides, and owns its light theme separately from the shell. Entered form
    values are transient, and the local action does not submit credentials.
    Scripts, navigation, ambient storage, nested previews and network fallback
    are excluded. Initial live coverage uses native controls and shared text
    fields, action and alert; it does not claim cross-frame parity for every
    library capability. The template is bounded to 256 KiB, resources to 128 and
    16 MiB total, and frame height to 4096 CSS pixels before scrolling.
- [x] Add native CEM-QL source Parse/Inspect adapters. Keep unsupported identities
  explicit instead of falling back to CEM-ML.
  - [x] Add native fixtures for module/expression AST and token events, all Inspect
    views, UTF-8 source ranges, malformed input, identity rejection and cancellation.
    Six native adapter fixtures, six expression API regressions and eight document
    inspection regressions pass. Token events reconstruct the exact source; emoji
    ranges retain original UTF-8 bytes and diagnostic columns use UTF-16. Source
    inspection resolves declared base URIs and honors cancellation/time budgets.
  - [x] Verify that typed parameter source ranges include their annotations.
    The regression reproduced a name-only span; the complete annotated range
    now passes alongside the six native adapter fixtures.
  - [x] Add a native CLI fixture for recoverable Parse diagnostics, explicit
    fail-level rejection and repair; align native CLI and browser exit policies.
    The fixture reproduced unconditional CLI success before the fix; all 33
    CLI parse-related tests now pass. Parse uses the existing report failure
    policy while preserving recoverable output and optional side reports.
  - [x] Add browser fixtures for the Login query source Parse/Inspect path and
    expression diagnostics through the existing shared command transport; verify
    saved-query Parse/Inspect through deployed declarative controls and include
    source-inspection evidence in the release gate.
  - Delivery evidence (2026-10-18): 53 focused native tests and all 131 Studio
    browser cases pass. The complete 52-task Studio check gate passes, including
    saved-query source inspection, live-view interactions, validation repair,
    independent copies, offline reload, automated accessibility, theme inheritance,
    forced colors, 32 schema examples and a clean seven-package consumer with one
    runtime copy. Lint/typecheck and whitespace checks pass. Two clean builds
    match across 268 files / 175 declared assets, aggregate SHA-256
    `be01aac90d947472bf69b18e6b6c2cfdfd948c733955237ff442511f106c803d`.
    An earlier Chromium renderer crash did not recur in the successful full gate.
    Its cause remains unidentified. The deployed verifier now fails promptly on
    page crashes by closing the browser and rejecting pending evaluations.
    Generated screenshots and Vitest attachments remain excluded from Git.
  - CEM audit: source parsing/projection belongs to the shared Rust engine and
    CEM-QL bridge. Native parser owners project directly to the typed CEM writer;
    JSON is only an explicit export. Studio reuses declarative source controls
    and command transport, adding no UI JavaScript capability or styling/token
    defaults. Module/expression identities never fall back to a document parser;
    AST/tree views inspect syntax without binding names, loading imports or
    running queries. Query sources have no DOM; use AST/events and their named
    JSON exports. Standalone-expression formatting remains a separate task.
- [x] Add native standalone-expression formatting without inventing module
  wrappers; preserve exact source identity and diagnostic ranges.
  - [x] Add native formatter fixtures for expression identity, comments/Unicode,
    line endings, idempotence, HTML output, invalid syntax/UTF-8, mismatched
    source/target identities, registered module aliases, cancellation and original
    diagnostic coordinates.
  - [x] Add browser command fixtures for expression formatting, failure and repair
    through the shared Studio command transport; retain module formatter regressions.
  - [x] Add a native CLI regression for MIME-only expression schema inference,
    explicit identity conflicts, syntax failure and repair; verify browser parity.
  - [x] Document the native formatting contract and verify native CLI, browser,
    packaging and release checks.
  - Delivery evidence (2026-10-18): 27 focused native tests and all 132 Studio
    browser cases pass, including MIME-only dispatch, original emoji diagnostic
    coordinates, failure/repair and existing module formatting. A rebuilt CLI
    smoke test passes explicit and MIME-only identity, malformed source and repair.
    The complete 52-task Studio gate passes on retry, including deployed Login
    live view, source inspection, independent copies, offline execution, automated
    accessibility, theme inheritance/forced colors, synchronized release evidence
    and a clean seven-package consumer with one runtime copy. Lint/typecheck and
    whitespace checks pass. Two clean builds match across 268 files / 175 declared
    assets, aggregate SHA-256
    `bd79b5d435a21375b093a56a0bc79e0c95074e93500e274ddbcb7c824324514e`.
    The first full run hit a Chromium dedicated-worker fatal trap during saved
    Login live-view rerendering. The unchanged retry with browser stderr capture
    passed; Nx flags workspace delivery as flaky. The crash's cause remains
    unidentified, and no assertions or browser protections were weakened.
    Generated screenshots and Vitest attachments remain excluded from Git.
  - CEM audit: the shared Rust engine/CLI bridge selects the exact native source
    kind before legacy registry defaults, parses expressions without module
    wrappers and reuses the typed token formatter/colorizer/writer pipeline.
    Writer identity and original source URI/UTF-8 ranges are retained; diagnostics
    use UTF-16 columns. Formatting neither binds names nor evaluates queries,
    and rejects module/expression relabeling. HTML is an explicit presentation
    target. Studio retains its declarative controls and existing command transport;
    no new UI JavaScript capability, application CSS or token defaults are added.
    There is no JSON AST handoff between native formatting stages.
- [x] Record the requested reusable AI element and explicit data/UI context
  exposure in [roadmap Phase 3.3](../roadmap.md#33-ai-elements-and-dataui-context),
  covering Chrome built-in AI and WebMCP with shared-package ownership. Keep its
  implementation deferred behind the current Studio/Amsterdam functionality.
- [x] Research and record OAuth Authorization Code Flow with PKCE and Google,
  Microsoft, GitHub and candidate Atlassian adapters in
  [roadmap Phase 3.4](../roadmap.md#34-oauth-authorization-and-account-backed-persistence),
  including declarative UI/shared JavaScript ownership and provider/backend limits.
  Add Drive, OneDrive and GitHub persistence components, standalone demos and
  subsequent Studio integration to that roadmap; keep implementation behind the
  active presentation functionality. No live account access or credentials added.
- [x] Record proactive WASM memory recovery between committed renders in
  [roadmap Phase 3.1.1](../roadmap.md#311-wasm-memory-recovery-between-renders):
  bounded portable checkpoints, whole-runtime replacement, shared owner coordination,
  stale-result fencing, side-effect reconciliation and restart limits. Implementation
  remains planned in the shared engine/`cem-elements`, with declarative recovery UI.
- [x] Record unchanged render/stylesheet reuse in
  [roadmap Phase 3.1.2](../roadmap.md#312-reuse-unchanged-render-content-and-stylesheets),
  separating compiled native artifacts, stylesheet installations and per-instance
  committed projections, with complete dependency keys, bounded ownership/eviction
  and no prior-connection chains. Optimization remains planned independently of
  repairing the verified retention defect.
- [x] Check the Studio catalog for a Markdown + CEM-ML site with Hugo-inspired
  folders and record the missing ordinary example project in
  [the site roadmap](../roadmap.md#studio-example-markdown--cem-ml-site-project).
  Existing Markdown schema fixtures and the standalone playground do not provide
  this project-level demo. Implementation remains planned after active repair and
  agreed presentation functionality, with shared-package ownership preserved.
- [x] Make the initial presentation sample set available through Studio's existing sample
  catalog, with read-only originals and independently editable project copies.
  Reuse canonical sources, native command/configuration semantics and stable
  project resource identities; implement any blocking Studio feature first.
- [x] Add browser fixtures for choosing/copying the presentation sample, editing,
  validating and running the selected checkpoints, then verify generated sample
  packaging and offline availability with the existing delivery gates.
- [x] Record exact supported sample routes, expected results and remaining
  presentation blockers in the presentation handoff and Studio documentation.

Next presentation functionality:

- [x] Audit the complete Login project against the required tutorial checkpoints;
  identify remaining gaps and deliver the plain-formatting increment through
  the common workspace.
  - [x] Add explicit pretty/tabular Login Convert configurations over the shared
    saved source and document the remaining color/expression presentation gaps.
  - [x] Add native fixtures for both layouts, unchanged input, retained semantic
    fields, plain output, idempotence and saved Unicode heading edits.
  - [x] Add browser fixtures using the packaged configuration argv and deployed
    offline Save/rerun/copy-isolation coverage through existing Studio controls.
  - [x] Verify sample generation, browser/native tests and Studio release checks;
    record the shared-component audit and update the presentation handoff.
  - Audit: read/edit, validation repair, typed inspection, HTML conversion,
    equivalent queries, native templates/graph/provenance and live view already
    have project resources. Login 2.6.0 adds the missing pretty/tabular source
    configurations, bringing the total to fifteen. Source-color output and a
    packaged standalone-expression evaluation remain explicit follow-ups below;
    complete rehearsal/participant delivery is not claimed.
  - Evidence (2026-10-18): the new native fixture first failed on the absent
    configurations, then passes both profiles, idempotence, source preservation,
    Unicode heading edits and the complete 21-field semantic contract. All six
    native Login sample/format tests and all 133 Studio browser cases pass.
    Deployed offline checks prove unsaved versus saved Run, exact restored
    outputs and fresh-copy isolation. The 52-task Studio gate passes on its first
    run, including `projectSourceFormatting` release evidence, live view,
    source inspection, automated accessibility/reflow, inherited theme/forced
    colors, 32 schema examples and a clean seven-package consumer with one runtime
    copy. Lint/typecheck and whitespace checks pass. Two clean builds match across
    268 files / 175 declared assets, aggregate SHA-256
    `d127039d3192495a9640225f8f91b80a1a4aef1d0e70b8e39a3bcda056841a3f`.
    The historical Chromium crash did not recur; its unresolved cause remains
    documented above. Generated screenshots and Vitest attachments are excluded.
  - CEM audit: this increment is authored project data and guidance. Both
    configurations call the existing shared native Convert formatter with exact
    CEM-ML identity and plain output; formatting does not overwrite saved sources.
    The existing declarative Studio workspace and shared controls own all UI.
    No UI JavaScript capability, component CSS or public token default is added.
    The native semantic check reads formatted CEM-ML directly; JSON is only the
    explicitly selected scalar comparison report, with no AST handoff.
- [x] Emit native CEM-ML HTML source colors and expose them as a Login
  presentation configuration. Preserve the approved slide preview independently.
  - [x] Add native fixtures for HTML selectors, plain/terminal regressions, exact
    pretty/tabular text, escaped markup/Unicode, source ranges and HTML identity.
  - [x] Implement HTML presentation in the shared typed writer without reparsing
    formatted source or using a JSON AST handoff; retain layout and provenance.
  - [x] Add a Login colored-source export configuration and browser/deployed
    fixtures for HTML text, inherited palette overrides, saved edits, download
    identity and offline copies.
  - [x] Verify the sandbox denies exported inline styles, matching source-preview
    CSP denials to exact exported style hashes (including restored projections),
    while keeping unknown styles and other console errors fatal.
  - [x] Update docs and verify native/browser behavior and focused deployed export
    checks; record sandbox limits and the unresolved full release blocker below.
  - Shared-component audit: Login 2.7.0 adds **Export Login source colors (HTML)**
    as an ordinary configuration (sixteen total). The shared Rust typed writer
    consumes formatter/color overlays, finishes tabular compaction before HTML
    escaping and retains tab-size decisions and source-frame chains. Attribute
    output retains its existing attribute-name owner. HTML output identity is
    explicit. Existing shared palette styles and inherited custom properties
    are reused; no new UI JavaScript, component CSS or theme defaults are added.
    Studio's existing static HTML preview still blocks styles/scripts; copy and
    download retain source colors. The approved slide artifact stays unchanged.
  - Evidence (2026-10-18): the source-color fixtures first failed on plain output;
    all 3 now pass, including escaped markup/Unicode, exact pretty/tabular text,
    custom tab size/CRLF, retained owner frames and HTML identity. All 7 typed
    pipeline/writer regressions and 11 native Login/sample cases pass. All 134
    Studio browser cases pass, including the packaged argv, inert HTML text and
    inherited palette override. Lint/typecheck and native example verification
    pass. Two clean builds match across 268 files / 175 declared assets, SHA-256
    `9cca99c285fa98ba99ac790ff1268b925274bce09b96cc310754dee4385df29a`.
  - Focused deployed verification also passes: exact HTML download bytes/MIME,
    source text in the actual sandbox, expected style blocking, saved/unsaved
    edits, offline reload and independent customized/fresh Login copies. These
    isolated checks stopped before the live-view regression repaired below.
  - Initial release result, before the following shared-runtime repair: the full
    52-task gate was **not green**. Both Chromium headless
    shell and full Chrome for Testing 148.0.7778.96 reported V8 JavaScript OOM
    (`CALL_AND_RETRY_LAST`) during the Login live-view workflow. The host had ample
    available memory. This does not establish the cause or prove it is the same
    fault as the earlier dedicated-worker crashes. No assertions or browser
    protections were weakened; generated screenshots/attachments remain excluded.
- [x] Diagnose and repair the Login live-view OOM in long Studio sessions, then
  pass `verify:workspace-delivery` and the full Studio release gate. Keep fixes in
  the appropriate shared engine/runtime and preserve declarative Studio UI.
  Historical result at `bd8f8428`: the long-session regression and complete release
  gate passed. A later recurrence is tracked explicitly below; this allocation
  repair does not close that renewed blocker.
  - [x] Profile long sessions and isolate excessive allocation in shared snapshot
    indexing; distinguish cumulative allocations from retained memory.
  - [x] Add large-payload index fixtures for ordering, independent snapshots and
    authored property names; remove quadratic array copying in the shared runtime.
  - [x] Verify repeated Login preview replacement and disposal; include the existing
    live-preview capability in the exact shared capability inventory fixture.
  - [x] Settle save-triggered navigation in the stale-provenance fixture before
    IndexedDB teardown, then rerun the Studio browser gate.
  - [x] Rerun the complete workspace lifecycle and Studio release gate; record
    the result without weakening browser protections or assertions.
  - Allocation evidence (2026-10-18): Chromium sampling identified about 6 GiB of
    cumulative `payloadElementsByAttribute` allocation across four workspace
    interactions. It copied each growing attribute group on every append. Shared
    slot/attribute indexes now append to fresh per-snapshot arrays and use own-key
    records, including authored `constructor`/`__proto__` names.
  - Controlled before/after fixture: ten snapshots of 4,096 inert items, Chromium
    148.0.7778.96, 32 KiB allocation sampling including collected objects. Index
    allocations fell from 3,382,929,504 to 6,480,612 bytes; total sampled allocation
    fell from 4,449,726,044 to 236,840,984 bytes. Elapsed time was 1,162 ms versus
    476 ms. These are sampled cumulative allocations, not retained heap sizes or
    a portable timing budget.
  - Eight saved-source replacements and six unchanged reruns passed in focused
    deployed diagnostics; replaced preview workers closed. The original fatal
    allocation site was not captured, so the profile alone does not prove the
    cause of every historical crash.
  - Shared-component audit: the repair is in `cem-elements`; Studio stays
    declarative. Snapshot ordering/independence and the existing transport shape
    are preserved. No UI capability, styling, palette default or browser-policy
    change is introduced. Both new browser fixtures, all 610 shared unit cases,
    typechecking and all 18 shared live-preview browser cases pass.
  - Historical release evidence at `bd8f8428`: Studio `check` passed all 52 tasks, including all
    134 Studio browser cases, 18 shared preview/index cases, 32 native/deployed
    examples, deterministic packaging, online/offline workers and the clean
    seven-package consumer. The long deployed workspace lifecycle passes Login
    input/action, unsaved/saved replacement, dependency changes, offline reopening,
    independent copies, theme isolation and automated accessibility checks without
    OOM. The stale-provenance fixture awaits its final navigation read before
    closing IndexedDB; no production Studio JavaScript was changed.
    Deterministic output: 268 files / 175 declared assets, SHA-256
    `e50745fcd26231e031df5e3ca927338930bbc8bfdd10410b1a49c420fde1a4b5`.
    Generated screenshots and Vitest attachments remain excluded. Physical-device
    and spoken-AT acceptance remain open behind presentation functionality.
- [x] Package a standalone Login expression resource with evaluation and formatting
  configurations, source-inspection guidance and saved-edit/offline fixtures.
  - [x] Add native fixtures for packaged expression evaluation on HTML/CEM inputs,
    source/dependency edits, formatting identity/idempotence and invalid-source repair.
  - [x] Add the expression resource and evaluation/pretty/tabular configurations to
    the ordinary Login project; document persisted inspection and output boundaries.
  - [x] Add packaged browser-command and deployed workspace fixtures for saved versus
    unsaved expression edits, exact output, offline reload and independent copies.
  - [x] Verify the complete Studio release gate and update presentation evidence.
  - Implementation: Login 2.8.0 has twenty ordinary configurations. One owned
    `queries/summary.cem-ql` expression reports title/required-field count from
    either saved native input; pretty/tabular Convert configurations preserve the
    exact expression identity without a module wrapper. Results do not overwrite
    source. Both current expression profiles preserve authored layout. The project
    README documents resource selection, persisted Parse/Inspect and Unicode edits.
  - Shared-component audit: only sample resources/configurations, documentation
    and fixtures changed. Studio uses its existing declarative workspace, shared
    native adapters and command transport; no production UI JavaScript, new shared
    capability, CSS, palette or browser-policy change was required.
  - Focused evidence (2026-10-18): both new Rust fixtures first failed on the absent
    packaged expression, then passed. All 11 native Login/sample regressions and
    136 Studio browser cases pass, covering HTML/CEM evaluation, input/dependency
    edits, attribute presence, Unicode, formatting idempotence, invalid-syntax
    repair, output identity and expression Parse/Inspect. Rust formatting and
    whitespace checks pass. The full 52-task gate failed in
    `verify:workspace-delivery`: V8 `CALL_AND_RETRY_LAST` during the existing Login
    live-view saved-dependency rerun. New first-copy expression checks had passed;
    independent-copy checks were not reached in that run. Native/deployed example
    verification (32 examples), online/offline workers, shell delivery, lint,
    typecheck and deterministic packaging passed. That release attempt was not green.
    Two clean builds matched 268 files / 175 assets, SHA-256
    `6b9f0847fd42371cfbcdc12c1b0317ed4b445ae76f62f4b6574fe62ea9492a23`.
  - Focused deployed evidence: the expression-only diagnostic extracted from the
    new workspace assertions passes saved/unsaved formatting, persisted Parse/Inspect,
    both evaluations, offline reload and original source/results in an independent
    copy. It uses actual static output and normal browser protections. This isolated
    run does not certify the full lifecycle or replace its failed release report.
  - Release follow-up (2026-10-18): the shared stylesheet retention repair below
    passes the complete 52-task gate, including all expression assertions in the
    full deployed lifecycle, offline reopening and independent copies.
- [x] Diagnose the recurring long-session Login live-view V8 OOM observed after
  `bd8f8428`, repair the shared engine/runtime cause, and pass the complete Studio
  release gate. Preserve the successful payload-index allocation repair and all
  assertions. Focused expression checks cannot substitute for this release gate.
  Evaluate the planned [WASM recovery contract](../roadmap.md#311-wasm-memory-recovery-between-renders)
  with deterministic fixtures as separate resilience work; do not assume that
  WASM recycling resolves the current failure without memory-domain evidence.
  - [x] Add a shared browser regression proving retired stylesheet generations
    become collectible after replacement while the current generation remains live;
    cover worker/fallback CSS and cancelled preparation without losing active styles.
  - [x] Break verified retention paths and compare long-session heap growth before
    and after, then pass shared lifecycle tests and the complete Studio release gate.
  - Diagnosis (2026-10-18): the deployed heap snapshot showed live scope disposal
    listeners retaining a stylesheet preparation closure, then repeated `previous`
    connection/unobserve/release links. Retired abort reasons retained V8 exception
    stacks and publication closures with full snapshots and patch data. This is a
    verified JavaScript retention path; WASM recycling is not required for the fix.
  - Repair: clear the preparation's previous connection on activation/cancellation,
    retaining the retired connection only locally during synchronous cleanup.
    Adoption still happens before old-generation abort notifications. The new
    Chromium collection fixture failed before the change and passes afterward;
    it covers worker/fallback CSS, ten successive replacements and a cancelled
    preparation retained by the caller. Current CSS stays installed and prior
    generations become collectible. All 25 shared-preview/registry cases pass.
  - Heap evidence: a read-only `Runtime.getHeapUsage` monitor on the deployed
    lifecycle observed normal main-thread JavaScript collection from about
    1.14 GB to 135 MB during independent-copy checks, then below 115 MB. No forced
    collection or restart was used in that delivery run. These are sampled JS
    heap readings, not total browser memory, a matched allocation benchmark or
    proof of a general memory bound; the before-fix snapshot establishes the
    retention path and the red/green fixture establishes its collection repair.
  - Audit: shared `cem-elements` lifecycle change only; Studio remains declarative.
    No new capability, native format, token default, browser-policy change or
    production forced collection/restart was added. The complete 52-task Studio
    release gate passes: 136 Studio browser cases, 25 shared preview/registry cases,
    32 native/deployed examples, online/offline workers, shell delivery, deployed
    two-project lifecycle/accessibility and the clean seven-package consumer.
    The previously failing live-view saved-dependency rerun, offline reopening and
    independent-copy expression checks all pass. Two clean builds match 268 files /
    175 assets, SHA-256
    `76fd8ac70d158465538c91bb0152d7a28966b6eae78bc0739f745028aef5cb02`.
    Historical failures above remain recorded; no claim covers every possible OOM.
    Generated screenshots and Vitest attachments remain excluded.
  - Additional shared checks: all 610 unit cases and lint pass. The standalone
    `cem-elements:typecheck` passes after temporarily regenerating the CLI command
    schema from the native build artifact with the existing builder's exact module
    content; the CLI build normally deletes that source after emission. The first
    standalone type-check attempt lacked that prerequisite. The temporary source
    is removed after verification and is not committed.
- [x] After the core Login walkthrough, add the planned supplier-pipeline
  companion as an ordinary example project, reusing native commands and shared UI.
  - [x] Add native supplier fixtures for XML catalog + CSV stock reconciliation,
    explicit product-ID matching, numeric quantities, missing/duplicate records,
    JSON/YAML/HTML outputs, source provenance and failure/repair without partial publication.
  - [x] Add shared native fixtures and implementation for explicit transform-stage
    result identities and bounded constructed generic-data JSON/YAML export, using
    package-owned formatters. Reject unsupported values and duplicate keys; retain
    template provenance without a JSON AST handoff or application-owned serializer.
  - [x] Update the existing native-reference fixture to assert the original source
    frames plus the retained owner's explicit URI boundary; its old raw-stack
    equality conflicts with the current provenance contract exposed by the full suite.
  - [x] Package the supplier resources, native templates/graph, expected outputs,
    README and ordinary configurations in the existing Amsterdam example catalog.
  - [x] Add shared command-output fixtures and repair for failed commands with no
    publication: retain native diagnostics, explicitly mark unavailable output,
    disable declarative transfers and preserve bounded reads/digest verification.
    The first full gate exposed the missing projection; native output rejection is correct.
    Preserve previews for existing output descriptors that omit the optional marker;
    the existing result-owner fixtures cover this compatibility boundary.
  - [x] Add browser/deployed fixtures for saved versus unsaved stock edits,
    graph execution, failure/repair, source navigation, offline reopening and
    independently editable project copies; preserve all Login acceptance checks.
  - [x] Run the complete Studio release gate, update presentation evidence and
    audit shared-package ownership before committing the completed example.
  - Evidence: all 52 Studio release tasks pass serially, including 137 Studio
    browser cases, 25 shared preview/registry cases, 32 deployed schema examples,
    online/offline workers, shell/theme/forced-color checks, the complete Login
    and Supplier lifecycle, automated accessibility and the clean seven-package
    consumer. Native checks pass: 106 adapter unit cases, 12 graph-authoring cases,
    three Supplier pipeline cases and seven existing Login/Amsterdam cases.
    All 616 shared unit cases, shared/Studio lint and type checks pass. The standalone
    shared type check uses the temporary generated CLI schema prerequisite described
    above; that source is removed afterward and excluded from delivery.
  - The initial gates exposed absent-output diagnostics loss and a preview
    compatibility regression for legacy descriptors without the availability flag;
    both are repaired. Two workbench cases also timed out in the parallel run.
    All 85 workbench cases pass serially with the original assertions and timeouts.
    The complete Login live-view saved-dependency/offline/copy sequence passes.
  - Two clean builds match 270 files / 176 declared assets, SHA-256
    `9dcd5c53671456d377cde10ff21a87fea20beca3f94b577217e37ed32e500577`.
    Audit: domain rules stay in the example's native templates; constructed-data
    export and result identities stay in shared Rust packages; bounded output
    reading stays in `cem-elements`. Studio only wires the shared reader and data
    field into its existing host and declarative results. Existing CEM alerts/actions
    provide failure and disabled states without local CSS or token defaults. Primary
    HTML remains the preview/copy/download target; artifact selection is next below.
    Generated screenshots and Vitest attachments remain excluded from delivery.
- [x] Determine whether the selected presentation configurations require
  parameter-map authoring; if so, complete its native fixtures and Save/history/
  recovery/offline Run work below. Otherwise retain it in the later graph-editor queue.
  Decision: Login and Supplier pipeline use saved resources and native primary/named
  artifact inputs; neither requires literal parameter-map authoring. Keep that
  authoring feature in the later graph-editor queue.
- [x] Implement the requested classic IDE workspace, following the supplied
  IntelliJ screenshot and VS Code's explorer/editor/panel organization.
  - [x] Audit existing shell, retained editor, resource navigation and preview;
    identify reusable declarative layout and existing interaction capabilities.
  - [x] Fixture and implement a shared declarative workspace with named explorer,
    editor, preview and tools regions; retain DOM/focus, responsive reflow,
    inherited theme spacing and five-theme paint in colocated plays, plus
    forced colors through the deployed fixture.
  - [x] Fixture and implement shared `cem-textarea fit="fill"` sizing, retained
    input/selection, narrow widths and a bounded playground/gallery example.
  - [x] Fixture and implement current-file editing beside an independent preview;
    move command/configuration/diagnostic tools below, keep draft guards and focus.
  - [x] Fixture and implement directory/file branches, saved-project selection,
    collection-grouped predefined project selection and explicit creation/save.
  - [x] Add component playground/gallery/docs and planned Figma inventory, update Studio design and acceptance
    fixtures, verify source/deployment-bundle delivery and visually inspect the running Studio.
  - Verification: 87 Studio workbench cases, 11 shell cases and 29 colocated
    workspace/media-preview/textarea plays pass. Typecheck, lint, the 55-component
    style and planned Figma inventories, and Studio UI classification pass.
    Graph-emitted static output is deterministic (274 files); the deployed Supplier
    flow passes native Run, saved/draft edits, failure/repair, offline reload and
    independent-copy checks with the new layout and disclosure navigation.
  - Visual inspection: a 1440×1000 desktop holds the three-column workspace inside
    the viewport; the source control fills its 592px-wide region. At 900/390/320px
    regions reflow without horizontal page overflow. Editor and preview remain
    visible through tool-tab changes; forced colors retain four readable regions.
    Generated screenshots and Vitest attachments remain excluded from delivery.
  - CEM audit: generic layout and bounded textarea/media sizing live in declarative
    `cem-components`; project/catalog/folder projection stays in Studio CEM-ML.
    Host JavaScript only binds existing service views and commands. The shell
    authors its workspace, retaining DOM, selection and focus through theme and
    status updates. Existing theme tokens own paint/spacing/typography; inherited
    inset overrides, all five themes and Canvas/CanvasText are verified. No new
    JavaScript UI capability or public token family is introduced. Resizable panes,
    editor tabs and physical-device/spoken-AT acceptance remain later work.
- [x] Replace the overlapping Studio forms with the requested URL-driven navigator.
  - [x] Fixture shared opt-in tree branch activation, preserving default disclosure and keyboard behavior.
  - [x] Model samples, saved projects, opened files, folder children and selection in application data; fixture URL startup, history, invalid routes and guarded file admission.
  - [x] Render a matching window/page heading, theme control, left action/tree column, selected folder/file body and status bar; remove obsolete shell scaffolding.
  - [x] Wire Create/Clone/Delete to durable services, preserve resource identities and drafts, and verify real navigation, theme retention and responsive geometry.
  - [x] Update design/docs and run relevant shared, Studio and deployed-browser checks.
  - Verification: 91 Studio workbench + 11 shell browser cases and 17 shared
    workspace/tree plays pass. Studio lint/typecheck, UI classification and the
    55-component style/planned Figma inventories pass. `verify:navigation` passes
    the graph-emitted app with URL startup/history, no-parameter Samples recovery,
    directory listings, project/resource Create/Clone/Delete, guarded Save/reload,
    matching heading/window title, five themes, inherited inset overrides,
    1440/900/390/320px geometry, forced colors, native provenance URL/range and
    offline Supplier Run. Static output is deterministic: 181 assets / 275 files.
  - CEM audit: shared browser/workbench layout remains in declarative
    `cem-workspace`; the opt-in branch activation substrate lives in
    `cem-elements`. Studio trees/resources/selection are application data;
    declarations own icons, count/status formatting and visible composition.
    URL/history/window-title and repository orchestration remain host services.
    Navigation updates retain editor ownership and drafts; stale sample loads do
    not replace a newer selection. Screenshots/attachments remain excluded.
- [x] Remove storage/install/update scaffolding from the requested Studio layout and fix stale local previews.
  - [x] Fixture a browser controlled by an old cache-first worker; verify the current layout loads without clearing projects or draft journals.
  - [x] Serve local preview assets under a content-derived URL prefix while keeping the root page/project URLs and browser origin stable.
  - [x] Remove unused shell status/control bindings and verify the rendered localhost page, shell regression cases and preview delivery.
  - Evidence: 11 shell cases, lint/typecheck, emitted-output verification and
    `verify:preview-reload` pass. The browser fixture deliberately keeps a stale
    cache-first controlling worker and poisoned prior-build bootstrap response;
    after a revision change it loads the new layout, preserves the root project/
    file URL and persisted bytes, and recovers the unsaved draft. Existing cache
    entries are retained. The actual localhost:8080 server now uses the revisioned
    preview handler; its Samples layout renders without storage/install controls.
- [x] Add shared selection of a generated graph artifact for preview, copy and
  download, so the supplier's JSON/YAML outputs are as convenient to inspect as its
  primary HTML output. Reuse native artifact identities and shared declarative
  controls; any reusable JavaScript capability belongs in `cem-elements`.
- [x] Deliver the planned ordinary Markdown + CEM-ML site example with Hugo-inspired
  folders, using [the site roadmap](../roadmap.md#studio-example-markdown--cem-ml-site-project)
  and the same projects/resources/configurations workspace.
- [ ] Diagnose the existing `DOM Stylesheet Adoption / Readiness And Native Diagnostics` held-worker timeout without weakening readiness assertions. The stylesheet ownership work reproduces the 30-second timeout with the unchanged `6f1c8dca` registry as well as the repair, both alone and in the sequential browser group; the other two DOM-adoption cases pass. Keep this separate from completed replacement/collection ownership checks.
- [ ] Audit the five pre-existing strict schema-definition diagnostics in the Studio project package (`id` pattern datatype and `path`/`reference-path` prefix facets); preserve its native portable project contract. The explicit-entry work verifies the same baseline diagnostics before/after and introduces no additional one.
- [ ] Migrate the older monolithic delivery script from retired Home/dropdown/storage
  selectors to current navigator actions; retain its broader engine/provider cases
  separately from the focused `verify:navigation` layout gate. Keep this helper
  acceptance work behind the presentation functionality above.

Do not introduce dedicated example screens. Shared components remain
declarative in `cem-components`, reusable JavaScript behavior belongs in
`cem-elements`, and Studio UI remains CEM-ML. Alert the user before introducing a
new shared JavaScript UI capability. Physical-device/browser/AT acceptance and
unrelated helper work remain independently open, behind presentation functionality.

## Declarative UI Architecture Correction

### Remaining declarative UI migration

Current inventory: **55 canonical / 0 legacy components / 0 legacy authored
code files**. Follow the [declarative UI principle](declarative-ui-principle.md)
and [component development pattern](component-development-pattern.md).

- [x] **Migrate cem-datepicker**, starting with a native
      fixture for its recorded template-brace defect. Preserve keyboard, value,
      form and reset contracts in colocated stories and source/packed galleries;
      add generic runtime capability where declaration support is missing.
  - [x] Fixture: compile and render the canonical datepicker module natively,
        covering named day/week templates, blank edge cells and boolean output;
        retain the recorded brace regression before component cutover.
  - [x] Fixture and implement shared calendar-input behavior for exact native
        owners, bounded Gregorian dates, modal draft/Apply focus and event order,
        composition guards, reset, reconnect and worker/fallback execution.
  - [x] Transfer all seven legacy assertions into colocated declaration stories;
        add boolean presence, boundary years and five-theme coverage.
  - [x] Fixture: consecutive parent renders must update a child's inert authored
        input payload during full reconciliation, retaining native owner identity,
        live/default value, labels, form names and component-owned output in both
        worker and fallback execution; fix the shared projection boundary.
  - [x] Ship scoped token CSS, property controls and five-theme gallery; verify
        source/packed/bundle consumers, forced colors and package gates, then
        retire the legacy owner and transfer state/parity evidence.
  - Evidence: native module regression, 19 declaration-registry unit checks,
    22 shared calendar/input/select/patch plays, 17 datepicker/autocomplete plays,
    22 existing workflow/primitive regressions and source/packed/bundle/forced-color
    integration pass. See the [migration record](cem-components-baseline-2026-09-27.md#datepicker-migration--2026-10-11)
    for the full-reconciliation payload fix and verification boundaries.
- [x] Migrate cem-timepicker and cem-slider, preserving native editor/range
      ownership, value, form, reset and interaction contracts.
  - [x] Fixture: transfer the seven timepicker and six slider legacy browser
        cases into colocated declaration stories without dropping assertions.
  - [x] Fixture: shared time-input and range-input capabilities must retain exact
        native owners, silent reset/reconnect, presence semantics, composition
        guards and strict malformed-payload rejection in worker and fallback.
  - [x] Fixture: scoped CSS must style a projected native input's browser thumb
        while still excluding projected element descendants and nested DCE
        interiors. Reproduce pointer hit-testing, then fix the native scope
        wrapper and compatibility emitter with native/browser regression checks.
  - [x] Fixture: a parent changing a slider between single and range modes must
        update conditional authored input payload, retain the existing input,
        and preserve native values/forms in worker and fallback execution.
  - [x] Implement shared capabilities and canonical scoped declarations; ship
        property pages, five-theme galleries and bundle examples, then verify
        source/packed consumers, forced colors, state/parity and package gates.
  - Evidence: all 13 transferred component plays, four shared worker/fallback
    plays, six native CSS regressions, 46 runtime unit checks and 22 existing
    workflow/primitive regressions pass. Source/packed/bundle, forced-color and
    package gates pass. See the [migration record](cem-components-baseline-2026-09-27.md#timepicker-and-slider-migration--2026-10-12)
    for the scoped native-thumb fix and verification boundaries.
- [x] Migrate navigation and interactive content owners, including cem-nav,
      cem-tabs, cem-stepper, cem-tree, cem-list and cem-chip; resolve their
      recorded presence/focus failures through the adopted owner contracts.
  - [x] Fixture: transfer every tabs, stepper and tree contract assertion to
        colocated stories; retain navigation/content presence, hover, focus,
        active, disclosure, selection and passive-mode assertions.
  - [x] Fixture and implement reusable tab-panel, workflow-panel, tree and
        navigation capabilities with exact owners, stable relationships,
        manual focus/activation, nested-owner isolation and reconnect coverage
        in worker and fallback execution.
  - [x] Fixture: presence-only collapsible/expanded, selectable and
        checkable/checked attributes must work with empty and false-valued
        attributes; live changes retain native focus and state without events.
  - [x] Ship all six canonical scoped declarations, property pages and
        five-theme galleries; retire legacy owners and transfer state/parity
        evidence, then verify source/packed/bundle, forced colors and regressions.
  - Evidence: 34 transferred component plays, four shared worker/fallback plays,
    19 declaration-contract unit checks and 17 remaining primitive/workflow
    regression cases pass. Source/packed/bundle, five forced-color gates,
    architecture, styles, state/parity, lint, typecheck and package checks pass.
    The three pre-existing legacy card/workflow loading/empty failures remain.
    See the [migration record](cem-components-baseline-2026-09-27.md#navigation-and-content-migration--2026-10-12).
- [x] Migrate `cem-surface`, `cem-text`, `cem-stack`, `cem-grid`, `cem-divider`
      and `cem-card` to canonical declarations, resolving loading/empty failures.
  - [x] Fixture: transfer the three existing card/surface state cases and every
        divider assertion to colocated stories before changing their behavior.
  - [x] Fixture: busy/empty/decorative presence includes empty and false-valued
        spellings; live changes retain owners, projected children, edited form
        values and focus in worker and fallback execution. Formatting containers
        stay passive and never infer or inherit workflow state.
  - [x] Fixture: verify stack gap variants and responsive auto/equal-track grid
        geometry using production declarations and bounded width changes.
  - [x] Define token-based scoped layout/typography/surface styling and stable
        public parts; preserve divider spacing, orientation and forced colors.
  - [x] Ship six property pages, five-theme galleries and bundle examples;
        retire legacy definitions/tests, transfer coverage evidence and verify
        source/packed delivery, state/parity, styles, types and regressions.
  - Evidence: all 21 layout/content plays and 34 navigation/content regressions
    pass together. Source/packed/bundle delivery, divider forced colors,
    architecture, styles, state/parity, package, lint and typechecks pass.
    See the [migration record](cem-components-baseline-2026-09-27.md#layout-and-content-migration--2026-10-12).
- [x] Audit the completed layout/content set against CEM theme principles.
  - [x] Fixture: reproduce the grid's misuse of a reading maximum as a layout
        minimum and the missing data numeral features; verify inherited theme
        overrides for all six owners across the five theme modes.
  - [x] Give intrinsic grid minimums a canonical D1 token; consume complete D6
        data typography, reading optical sizing and D4 work-layer surface separation
        without local defaults.
  - [x] Verify source-table generation/manifest, theme contrast and forced
        colors, component plays, source/packed delivery and document the audit.
  - Evidence: 61 component/regression plays pass, including six five-mode
    inheritance audits. Theme Phase 13 (source tables, manifests, generators,
    modes, forced colors, accessibility smoke and cross-spec checks), component
    source/packed/bundle delivery, styles, package, lint and fresh typechecks pass.
    The [audit](../packages/cem-components/docs/layout-content-contract.md#cem-principle-audit--2026-10-12)
    records corrected semantic mappings and the boundary of browser evidence.
- [x] Migrate disclosure/data owners: `cem-expansion`, `cem-table`,
      `cem-sort-header` and `cem-paginator`, resolving the 11 recorded failures.
  - [x] Fixture: transfer all 15 existing expansion/sort/paginator cases to
        colocated stories without dropping assertions; cover passive table
        semantics and application-owned data ordering.
  - [x] Fixture and implement generic disclosure, sort-action and pagination
        capabilities: exact owners, presence attributes, stable focused nodes,
        nested-table isolation, silent author changes, reconnect and both
        worker/fallback execution. Cover fractional invalid page sizes.
  - [x] Ship canonical scoped declarations, four property pages and five-theme
        galleries; verify source/packed/bundle consumers and forced colors,
        then retire legacy owners and transfer state/parity evidence.
  - [x] Fixture and audit inherited semantic tokens in five modes, target/focus
        geometry, data typography, accessibility and passive table ownership;
        record CEM principle conformance and refresh the remaining suite.
  - [x] Fixture: a stylesheet begin must reacquire its compiled source after
        bounded worker artifact-cache eviction. Reproduce the bundled catalog
        failure without enlarging the cache, then verify worker/fallback and
        existing generation cancellation/release contracts.
  - Evidence: all 15 transferred cases, the passive table case, four five-mode
    inheritance audits and three shared worker/fallback lifecycle plays pass.
    Source/packed/bundle delivery, all three forced-color contracts, styles and
    package gates pass. The audit enforced D2 target/guard floors; a reproduced
    worker cache-eviction defect is fixed with 68 passing runtime unit checks.
    See the [CEM principle audit](../packages/cem-components/docs/disclosure-data-contract.md).
- [x] Close the full component gate before the next migration set. The configured
      package gate and all 59 dependencies pass; the canonical cohort is
      **242/242 across 39 files**, and the remaining legacy suite is **23/23
      across five files**. All assertions and deadlines remain intact.
  - [x] Fixture: parent updates retain the explicit payload envelope's patch
        identity and reconcile its unwrapped inert source. Worker/fallback plays
        preserve edited input values and exact input/dialog owners while changing
        parent attributes and inserting/removing an optional toggle.
  - [x] Fixture: Escape from a hovered tooltip surface followed by authored
        `trigger-for` rebinding restores focus interest on the alternate launcher.
        Hidden surfaces release stale pointer interest even without pointerleave.
  - [x] Fixture: a newer native radio choice survives an older child publication.
        Capture from render request time; preserve the same native owners and
        update defaults without overwriting live user state. Cover full plans,
        worker patches, DOM fragments and worker/fallback runtimes. Fifty rapid
        navigation presence cycles pass, as do both affected property gates.
  - [x] Fixture: captured picker lifecycle failures retain their native owners.
        Native disabled availability, dialog close/focus and commit events now
        complete independently of async publication. Matching retained time
        choices update active/selected state before showing; changed choices
        await publication.
  - [x] Refresh the offline component/Figma inventory against all 53 current
        declarations and executable states. Parse primary reference tables
        separately from detailed attribute tables, retaining duplicate checks.
        All 53 external component reviews remain planned.
  - [x] Fix inferred browser-test task dependencies so runtime builds complete
        before tests import their artifacts. The configured full gate proves
        the ordering and all five browser/spec targets pass.
  - [x] Re-run the complete canonical cohort and configured package gate, plus
        102 runtime unit checks, focused regression plays, fresh types and lint.
        Lint retains 115 component / 15 runtime warnings, with no new warnings
        and zero errors. Record the CEM principle audit in the
        [aggregate repair evidence](cem-components-baseline-2026-09-27.md#aggregate-lifecycle-repairs--2026-10-12).
  - [x] Refresh offline Figma token metadata from all five actual mode files:
        272 variables (64 COLOR / 124 FLOAT / 84 STRING). Preserve the historical
        230-variable manual review and pending external refresh.
- [ ] Conditional diagnostic follow-up: if slider property-page root readiness
      fails again, use the new failure capture to attribute it before changing
      behavior. One initial aggregate timeout remains unexplained; the unchanged
      source/packed gate, 20 reloads under load and final full gate pass. Passing
      repeats do not establish a cause or a specific slider fix.
- [ ] Diagnose the isolated suggestions-controller baseline failures reproduced on 2026-10-20 against committed HEAD: ProductionFieldsExternalSourcesAndTrustedAncestorResume (child visibility) and PlatformCloseCancelsPendingAndPreservesEditorRouting (missing committed native row map). Keep assertions and deadlines intact; distinguish these from checkpoint capture acceptance.
- [ ] Conditional diagnostic follow-up: if datepicker/suggestions failures
      recur during concurrent browser runs, capture their focus, dialog and
      processing state before changing behavior. Running the canonical cohort
      alongside the package gate produced two 30-second play timeouts and two
      following focus/open failures (262/266); the standalone cohort passes
      266/266 with unchanged deadlines and assertions. No runtime cause is
      attributed. Preserve this distinction in subsequent acceptance reports.
- [x] Migrate the feedback/status set: `cem-toast`, `cem-progress`,
      `cem-progress-spinner`, `cem-skeleton` and `cem-alert`.
  - [x] Fixture and implement reusable progress-status preparation in
        `cem-elements`: presence-selected mode, finite range normalization,
        clamping, retained owners, no input handlers/events, worker/fallback.
  - [x] Fixture and migrate all five declarations with colocated plays for
        native progress, live status/urgent feedback, hidden placeholders,
        projected content, retained identity and inherited theme overrides.
        Port every spinner assertion before retiring its legacy suite.
  - [x] Fixture and ship property controls, five-theme galleries, source,
        bundled and packed consumers, forced colors and reduced motion.
  - [x] Retire the five legacy declarations and spinner behavior/spec; update
        package/catalog/state/parity/Figma evidence, run aggregate verification
        and record the CEM principle audit and remaining limits.
        Final evidence: 266/266 canonical plays across 44 files; 25/25 focused
        feedback/runtime plays; 18/18 remaining legacy checks across four files;
        full 60-task package gate; final source/packed/bundle feedback gate;
        fresh types and lint (115 component / 15 runtime existing warnings, no
        new warnings or errors). See the [feedback/status contract and CEM audit](../packages/cem-components/docs/feedback-status-contract.md)
        and [verification evidence](cem-components-baseline-2026-09-27.md#feedbackstatus-migration--2026-10-12).
- [x] Finish every remaining legacy member as
      `src/components/<cem-tag>/<cem-tag>.xhtml`, with embedded once-per-declaration
      scoped CEM-token CSS and colocated CSF Next `.stories.ts` play tests.
      Move missing reusable behavior into `cem-elements`, ship property pages
      and five-theme galleries, and verify source, bundle and packed consumers.
      Reduce both legacy component and authored-code counts to zero.
  - [x] Fixture and migrate the nine passive/inert owners (`cem-option`,
        `cem-option-group`, `cem-badge`, `cem-avatar`, `cem-media-preview`,
        `cem-tree-item`, `cem-app-bar`, `cem-tab`, `cem-step`): payload consumption,
        accessible identity/media/landmark semantics, retained nodes, worker and
        fallback, inherited tokens and five-theme/forced-color delivery.
  - [x] Preserve all remaining workflow and substrate-harness assertions in
        colocated component plays or shared runtime tests before deleting the
        legacy suites. Fixture source, bundle and isolated-package consumers.
  - [x] Fixture and fix shared choice-list rendering when every option is
        disabled: an empty active-option ID must not become the selector `#`.
  - [x] Retire the registry, installer, unused option copy and package-owned
        tooling/configuration; update consumers and Nx gates. Enforce zero
        legacy components and zero authored code except colocated stories.
  - [x] Verify Studio XHTML resource loading and its static asset manifest;
        replace the stale 31-schema tour assertion with the canonical runtime
        schema count and retain the one-example-per-package invariant.
  - [x] Fixture: await the nested icon before capturing icon-button hover/active
        baselines. The aggregate failure captured incomplete icon markup; retain
        every exact markup, geometry, focus, event and runtime-state assertion.
  - [x] Run canonical stories after the full package delivery prerequisites,
        audit CEM principles, and record completion evidence and remaining limits.
  - Evidence: **318/318 plays across 54 files** (316 canonical / two shared
    substrate), all 55 package prerequisites, source/packed/bundle and forced
    colors, refreshed architecture/style/state/parity/Figma/package gates,
    fresh runtime types and lint pass. Lint retains 115 component / 15 runtime
    warnings, with no new warnings or errors. Studio's five shell tests, static
    output (98 assets / 188 files) and UI classification checks pass. Package
    publication contains 296 files and zero JavaScript/TypeScript. The
    [CEM principle audit](../packages/cem-components/docs/passive-payload-contract.md#cem-principle-audit)
    and [completion record](cem-components-baseline-2026-09-27.md#passivepayload-migration-and-legacy-retirement--2026-10-12)
    preserve exact ownership, semantic token mappings and remaining manual
    browser/device/AT limits. External Figma review remains 0 completed / 53
    planned; Studio's subsequent visible UI migration is recorded below.
- [x] Re-establish the remaining legacy suite baseline after layout/content
      cutover without weakening assertions: **27 passing / 11 failing / 38 total**
      across all eight remaining specs (five passing files, three failing).
      The original historical baseline remains in the
      [failure inventory](cem-components-baseline-2026-09-27.md).
- [x] Migrate remaining Studio and Site visible DOM construction, UI listeners
      and state projection to XHTML/CEM-ML, retaining JavaScript only for non-UI
      services and host adapters.
  - [x] Migrate Studio application shell markup, interaction bindings and state
        projection to XHTML/CEM-ML; reuse shared select/action/status components.
  - [x] Fixture and implement a generic `cem-elements` host-service view binding:
        explicit command allowlist, synchronous invocation for browser gestures,
        scalar event values, state updates, nested-view isolation, retained focus,
        errors and disposal in worker/fallback execution. No app-owned DOM handlers.
  - [x] Fixture the declarative shell with theme persistence, install/update and
        file-provider availability, native action invocation, retained workbench
        content, disposal and production module-map delivery. Preserve old checks.
        Rapid repeat activations must not duplicate pending host operations.
  - [x] Close Studio's deployed runtime module graph over the migrated shared
        capabilities; fixture every relative JavaScript import against declared
        output assets so omitted modules fail before browser startup.
  - [x] Add a focused deployed-shell gate for production CSS, token inheritance,
        forced colors and offline reload, while retaining the full worker/Feature
        Tour gate and recording its independent schema-validation failure.
  - [x] Mount declaration owners so native stylesheet ownership can publish;
        authorize generated style elements in Studio CSP, retain strict script,
        style-attribute and preview policies, and verify actual deployed styles.
  - [x] Native fixture: module-prelude selected attributes must reflect exactly
        like unwrapped declarations, preserve authored defaults and source data,
        and exclude body/named-template bindings; fix before shell theme cutover.
  - [x] Audit the shell against CEM principles, verify Studio build/browser gates,
        fixture deployed five-mode styling, inherited token overrides and forced
        colors, and record workbench and Site follow-up boundaries.
  - [x] Fixture supported shell action intents; replace retired quiet/secondary
        variants with contextual/alternate and mark confirmed replacement
        destructive under the canonical CEM action vocabulary.
  - [x] Update Studio's incompatible-replacement confirmation to canonical modal
        dialog opening, accessible heading and `cem-close`; retain the existing
        replacement/cancellation browser fixture and assert browser modality.
  - [x] Reconcile Feature Tour fixture counts with all 32 registered schema
        packages, including `cem-data-island`; preserve exact counts and byte/hash
        checks in output verification.
  - [x] Repair native validation of the existing `cem-data-island` complete-instance
        example, then rerun the full Studio Feature Tour/static-worker gate.
        The native command now reports zero errors/warnings; both the public
        engine and package-example path use the selected model's namespace claims.
    - [x] Add native fixtures for data-island adapter/model admission, the complete
          example, required attributes and kind values, unknown parts/foreign
          namespaces, and namespace claims without a ready selected model; retain
          source coordinates and HTML payload checks. Cover the schema-package
          example-validation path as well as the public engine command.
  - [x] Migrate the Studio workbench's source, command, result and diagnostic views
        and their interaction/state bindings. First finish any reusable control
        or composite in `cem-components`, with JavaScript capability ownership in
        `cem-elements`; replace remaining retired action intents during cutover.
    - [x] Fixture shared host-command native form snapshots in worker/fallback:
          successful scalar entries, repeated names, disabled fields, submitters,
          nested-view isolation, synchronous invocation and formdata-event revocation.
    - [x] Fixture and implement declarative form-control selection requests in
          worker/fallback: explicit focus, retained caret on unrelated renders,
          repeated navigation, invalid/disabled/composing requests and disposal.
    - [x] Add a canonical textarea navigation example and colocated fixture;
          document the reusable selection attributes in its contract/playgrounds
          and fixture the five new declarative property controls.
    - [x] Migrate source editing, status and operation controls to a CEM-ML view;
          fixture save/reload, native form selections, duplicate pending commands,
          diagnostic navigation, focus retention and deployed asset delivery.
          Verify all five themes, inherited input/action tokens and forced colors.
          Command and result cutovers are completed below.
    - [x] Migrate command editing, semantic changes, apply/run targeting and
          replacement confirmation. Add any missing shared native-surface close
          binding before removing application listeners; retain revision guards.
      - [x] Fixture shared host-command surface dismissal in worker/fallback:
            exact owner, nested-surface isolation, cancellation and disposal.
      - [x] Fixture retained command/target edits, semantic-change text,
            pending-command exclusion, native replacement dismissal and declaration
            delivery; verify themes, inherited tokens and forced colors for the
            migrated command view.
    - [x] Migrate result previews, execution/expected/trace/graph views and
          diagnostic/provenance navigation. Build any reusable viewer in the
          component library first; preserve preview isolation and exact output.
      - [x] Extend canonical media-preview with text, isolated document and
            message modes; fixture exact text, deny-all sandbox, retained owners,
            property controls, inherited tokens and forced colors. Keep media
            payload/caption compatibility and update its gallery/contract.
        - [x] Accept native formatter whitespace before fenced style content in
              the architecture scanner while retaining its CSS/token checks.
        - [x] Verify new media-preview controls and exact text/document modes in
              source and isolated package galleries, including forced colors.
      - [x] Fixture declarative workbench results and navigation: exact transfers,
            duplicate pending operations, stable previews/focus, expected/trace/
            graph/report values, empty states, disposal and production delivery.
        - [x] Normalize native trace events into bounded path/value records;
              fixture raw event shapes and verify deployed trace/graph row counts.
        - [x] Audit neighboring workbench array counts; fixture zero validation
              diagnostics and empty/multiple semantic changes in their labels.
  - [x] Replace Site's retired installer imports and migrate interactive examples
        and search UI through the shared declarations and host-service boundary.
    - [x] Load canonical component declarations through `cem-elements`; retire
          Site's primitive installer and legacy runtime aliases.
    - [x] Migrate token filtering, action examples and native output presentation
          to CEM-ML. Build any useful missing viewer/control in the library first.
    - [x] Migrate search fields, result lists, status and navigation bindings;
          keep index loading/matching and URL transport in non-UI services.
    - [x] Fixture Site interaction/search and static delivery, then audit all
          five themes, inherited overrides, keyboard focus and forced colors.
      - [x] Fixture canonical Site deployment: explicit runtime asset closure,
            standalone XHTML loading, scoped styles and no retired imports.
      - [x] Fixture token filtering, retained field focus/value, action count and
            exact literal rendered HTML in the shared media-preview component.
      - [x] Fixture search phrase ranking, empty/multiple/literal queries, native
            form submission, URL preservation and the complete no-JS index.
      - [x] Fixture production themes, inherited surface/action tokens, keyboard
            focus and forced colors; retain route/provenance/determinism gates.


Site migration evidence (2026-10-12):

- `examples-view.xhtml` and `search-view.xhtml` own visible composition, bindings
  and state presentation. Token filtering and counting use declarative slices;
  the views reuse canonical field, action, surface and media-preview components.
  Search matching and URL transport remain host services. The native build's
  complete semantic index supplies bootstrap control data and remains available
  without JavaScript; imported source JSON stays inside the native CEM boundary.
- Bootstrap snapshots precede component upgrade. Host updates publish fresh
  state values, preserving edited native owners and focus. The shared text preview
  displays exact browser-serialized greeting paragraph HTML, with hydration
  metadata outside the example output. No app-owned UI listeners, DOM generators,
  visible writes, CSS or component behavior modules remain in Site services.
- The production graph declares **96 assets per interactive route**, including
  the complete `cem-elements` module graph, worker/WASM, standalone component
  XHTML and both app views. Retired installer/runtime aliases are removed. Named
  action/select XHTML transport, selection and scoped CSS pass. Component catalog
  provenance now requires canonical declarations. Eight stale reference fragments
  and a runtime README heading-level gap are corrected at their owning sources.
- `cem-site:verify` passes its full dependency chain: examples, search, production
  and provenance gates; **26 routes**, **25 searchable documents**, **245 files**,
  and two byte-identical clean builds
  (`bb8a635d608aab72263925d6d8b5e617dda0c3eb28366644e2662ad2fcf0f4e3`).
  Checks retain exact/literal output, case/empty filters, edited child values,
  owner/focus retention, phrase ranking, empty/multiple/literal searches, native
  Enter submission, unrelated URL/history state, no-JS index and all fragment
  targets. Site lint passes with no warnings or errors.
- Both view conversions pass native tabular reparse and native runtime rendering;
  search also passes a compiled-template **25 → 1 → 25 → 1** result sequence.
  Standalone CLI schema diagnostics still flag runtime vocabulary/whitespace
  annotations; formatter success is not a claim of complete schema validation.
  Deployed runtime diagnostics are clean.
- CEM audit: named palette samples use shared surfaces with paired D0 foregrounds;
  filtering/count/result statuses retain text cues. Existing D1 inset, D2 coupling,
  D3 shape, D5 boundary/focus and D6 reading/script roles remain theme-owned.
  All five theme modes, inherited palette/inset/script-font/action-radius values,
  the action's coupled border geometry and forced-color indicators pass. No new
  tokens, local defaults or alternate palette families are introduced. Production
  keyboard and naming checks pass in Chromium; physical-device, other-browser and
  spoken AT acceptance remain open. Presentation context remains deferred.

Studio results/preview migration evidence (2026-10-12):

- `workbench-layout.xhtml` and `workbench-results.xhtml` own result/status,
  execution, expected, trace, graph, report and diagnostic/provenance composition.
  The static layout mounts independent source, command and result views; state
  updates retain child owners, focus and unchanged document `srcdoc`. Shared host
  commands invoke exact copy/download and source navigation, with synchronous
  pending guards and disposal. Workbench JavaScript now owns services and bounded
  data transport; its visible DOM generators and UI listeners are retired.
- Canonical `cem-media-preview` now supplies explicit text, document and message
  modes alongside its compatible media/caption slots. Text preserves literal
  values and whitespace. Documents retain a fixed empty sandbox, opaque origin,
  no-referrer policy and deny-all CSP. No new JavaScript capability was necessary.
  Studio's legacy preview renderer, CSP export and preview stylesheet are retired;
  byte budgets, UTF-8 validation, exact transfers and redaction remain services.
- Native trace events now become at most **100** path/value records. A 105-event
  fixture verifies first/last displayed values and the bound. Declarative empty
  states and counts select array members, fixing blank lists/tables and singleton
  container counts in metadata, validation and command labels. Record text keeps
  booleans/nulls and literal markup; truncation preserves Unicode code points.
- Verification: media-preview **8/8** colocated worker/fallback/theme/control
  plays; Studio shell **7/7**, workbench **15/15** and preview **5/5** browser
  fixtures. Component build, source/bundle/isolated-package passive galleries,
  Studio typecheck/lint and runtime typecheck pass. Component lint has no errors
  and retains **115** existing warnings. Native tabular reparse is byte-identical.
  The architecture scanner accepts native formatter spacing before CSS fences
  while retaining its token and ownership checks.
- Final deployed `verify:static-worker` passes: **32/32** Feature Tour examples,
  all **5** portable workbenches with rendered trace/graph counts matching service
  records, exact copy/download, production themes/overrides/forced colors,
  keyboard reachability, reduced motion, sandbox hardening and offline reload.
  Output has **168** declared assets / **258** files; two clean builds are
  byte-identical (`53dab1ee54f5beb5938b1ca0064dd14fad0854a6b1634fc5cecc983a3ac109d8`).
- CEM audit: native figure/preview semantics and canonical action intents carry
  meaning; modes and record/preview limits bound variation. D0 comfort/text,
  D1 inset/related gaps, D3 media bend, D5 boundary/focus/zebra and the complete
  D6 script typography role stay theme-owned and inherit container overrides.
  The text viewport's `60vb` cap is a structural scrolling limit; iframe height
  retains its native default. All five themes and forced colors pass; no new
  token defaults, aliases, motion or application styling are introduced. The
  isolated document remains outside the host theme. Automated evidence is
  Chromium; physical-device and spoken AT acceptance remain open below.
- Site's canonical loading, examples and search are now complete (see the Site
  evidence above). Studio delivery planning and browser/device/AT acceptance
  remain independent work; presentation and its resume context stay preserved.

Studio command/application migration evidence (2026-10-12):

- `workbench-command.xhtml` owns command editing/status, semantic-change tables,
  target selection, apply/run actions and replacement confirmation. It composes
  canonical textarea, text-field, select, action, tabs, table, alert and dialog
  declarations. Application DOM listeners, command markup generators and direct
  editor/target value writes are removed. Host services retain revision guards,
  publish drafts before asynchronous reads and exclude duplicate pending work;
  older grammar results cannot overwrite newer edits. View data excludes full
  command previews, execution records and committed result bytes.
- Shared `mountCemView` admits `cem-close` only from the bound surface itself.
  Nested dismissal cannot cancel an ancestor task, cancelled close requests do
  not deliver commands, and disposal revokes delivery. The application checks
  that confirmation remains pending before handling dismissal. Native dialog
  close and Escape preserve the repository's replacement authorization boundary.
- Native tabular output reparses byte-identically. Named templates and select
  variables avoid formatter recursion/fencing limits; the CLI retains legacy
  HTML-policy warnings for template vocabulary. Browser compilation passes.
  Semantic changes retain missing/string/JSON distinctions and truncate at 239
  Unicode characters plus an ellipsis without splitting surrogate pairs.
- Shared host-view worker/fallback plays **4/4**, workbench browser checks
  **13/13**, shell **7/7**, runtime/Studio types and lint pass (15 existing runtime
  warnings; no Studio warnings). Component build prerequisites pass. Fixtures
  cover retained editors/carets/modal owners, out-of-order validation, exact
  semantic text, duplicate copy/apply-run, native close/Escape and disposal.
- Full `verify:static-worker` passes: **32/32** native/browser schema examples,
  all five portable workbenches, **167 assets / 257 files**, two byte-identical
  clean builds and online/offline worker/WASM execution. Final build hash:
  `b41f904fa90e41b60a252cd048f788398e30022db7ef4b871d19f8c685865706`.
  Deployed command XHTML and shared host adapter match their source/build bytes.
- CEM audit: existing controls retain canonical D0/D1/D2/D2c/D3/D4/D5/D6/D7
  ownership; copy/apply are alternate, reset/cancel contextual, apply-run/use-new
  primary and replacement destructive. Required labels, textual status cues,
  modal focus and explicit replacement consent remain intact. No application CSS,
  component JavaScript, token defaults or parallel token families were added.
  Five deployed themes, inherited input-background/action-radius overrides,
  coupling borders, forced colors and reduced motion pass. Evidence is Chromium;
  physical-device and AT acceptance remain open. Result/preview/diagnostic
  cutover is now recorded above; Site is next and presentation stays deferred.

Studio workbench source/operation migration evidence (2026-10-12):

- `workbench-controls.xhtml` owns source editing, workbench selection, status,
  revision labels, save/reload and operation forms. The host forwards service
  data and commands through `mountCemView`; result bytes/graphs/command previews
  are excluded from this view's state. Native forms retain programmatic select
  values. A synchronous pending guard prevents duplicate service operations.
- Shared host commands capture frozen successful string form values, retain
  repeated names and submitter data, and reject files or controls from another
  view. Formdata-event disposal revokes delivery. Text selection lives in the
  shared form-control capability, with UTF-16 ranges, explicit focus and one
  consumption per request change; unrelated renders and reconnects do not replay
  navigation. The canonical textarea contract, property controls, inspectable
  gallery example and colocated stories demonstrate the same capability.
- Native tabular conversion/reparse passes. Workbench browser checks **12/12**,
  shell **7/7**, shared host/form-control worker/fallback plays **5/5**, and
  canonical textarea plays **12/12**, including all five new property controls.
  Runtime/Studio type checks and runtime/component/Studio lint pass; lint retains
  the existing 15 runtime / 115 component warnings. The component build passes
  its architecture/state/style gates and emits the updated property/gallery pages.
  No app/component-local interaction code or new theme CSS was added.
- Final `verify:static-worker` passes: **32/32** native/browser schema examples,
  all five portable workbenches, **166 assets / 256 files**, two byte-identical
  clean builds, and worker/WASM commands online/offline. The final build hash is
  `c2ed5d4bde3e39eedaef478c040bb812abbe308ac32ab0bf8665d7ddbc6b4854`.
- CEM audit: existing controls retain canonical D0/D1/D2/D2c/D3/D5/D6 ownership;
  save/run are primary, parse/inspect alternate and reload contextual. Live
  status retains text cues, native labels/forms/validation remain intact, and
  explicit selection respects disabled/composing editors. All five deployed
  themes, inherited input-background/action-radius overrides, coupling borders,
  forced colors and reduced motion pass. No defaults or parallel token families
  were introduced. Evidence is Chromium; physical-device and AT acceptance
  remain open. Command/application and result/preview/diagnostic cutovers are
  now recorded above, including retirement of the remaining legacy UI.

Studio shell migration evidence (2026-10-12):

- `shell.xhtml` owns markup, bindings, theme classes and status projection;
  `shell.js` supplies non-UI services through shared `mountCemView`. No new widget
  or component-local behavior was introduced. Canonical action intents replace
  shell quiet/secondary aliases; confirmed replacement uses destructive intent.
- Native module-prelude selection fixtures **6/6**, existing attribute render
  regressions **11/11**, shared host-view worker/fallback contracts **2/2**,
  shell browser fixtures **7/7**, workbench browser regressions **11/11**.
  Host operations retain trusted browser activation and reject duplicate pending
  operations; native owners, edits, caret and focus survive state projection.
- Studio output verification passes with **165 declared assets / 255 files**.
  Two clean graph builds are byte-identical. Relative import closure includes
  all migrated runtime capabilities. The focused
  `verify:shell-delivery` gate passes against production headers, service-worker
  control and offline reload. All 23 UI classifications still pass; Studio and
  runtime type checks and lint pass (15 existing runtime lint warnings).
- CEM audit: semantic action/status choices use existing components and canonical
  D0/D1/D2/D2c/D3/D5/D6 tokens; no defaults, private aliases or new token families.
  All five modes render actual component CSS. Inherited container inset and
  public action-radius overrides reach controls; the action retains its D2
  coupling border. Forced colors, non-color status cues and keyboard-reachable
  preview checks pass. This is Chromium evidence, not a new browser/AT parity claim.
- Declaration owners are now connected for native CSS publication. Studio CSP
  explicitly permits generated style elements; inline script, general eval,
  style attributes and the separate preview policy remain restricted. The
  complete `verify:static-worker` dependency chain now passes after the native
  data-island repair below. Workbench and Site migration, broader Material
  functionality and the preserved presentation backlog remain open.

Data-island validation repair evidence (2026-10-12):

- Registered the data-island CEM adapter identity and native structural model;
  bound its four declared diagnostics to the shared field-contract reporting
  behavior. The unchanged complete-instance example now has **0 errors and
  0 warnings**, down from 41 errors and 16 warnings.
- Namespace admission uses only the selected ready model's package claims.
  Unknown schemas, foreign namespaces and pending models retain rejection;
  original AST names/source maps remain intact. Both engine and package-example
  validation use captured lexical names. Embedded HTML retains its policy checks,
  including acceptance of the standard `slot` attribute.
- New native fixtures **6/6**, namespace disposition **4/4**, runtime validation
  **5/5**, completion **4/4**, and semantic rules **71/71**, including the new
  package-example fixture. Native tests cover missing attributes, invalid kinds,
  unknown parts, namespace admission, source coordinates and embedded HTML.
- Full Studio `verify:static-worker` passes on the final rebuilt artifacts:
  **32/32** Feature Tour examples, all **5** workbenches, **165** declared assets /
  **255** files, two byte-identical clean builds, and browser worker/WASM commands
  online and offline. Production theme modes, inherited overrides, forced colors,
  reduced motion, keyboard reachability and preview hardening all pass.
- CEM audit: this repair keeps validation in the shared native schema layer;
  it introduces no application UI behavior, token defaults or theme aliases.
  Validation remains bounded to package namespace claims and local-name structural
  contracts. Per-element domain pairing and execution of the named lifecycle
  policies (part order/cardinality, dataset consistency, host state and payload
  delegation) are not added by this repair. Browser evidence remains Chromium;
  physical-device and assistive-technology acceptance stays open below.

### Native-surface delivery follow-ups

- [ ] Verify both pinned [ARIA profiles](cem-element-aria-reference-profile.md)
      in the documented browser/assistive-technology matrix. Record actual
      browser/OS/AT versions, accessibility relation exposure, ordered/repeated
      details targets and error-message announcements. Keep untested combinations
      explicit and the draft profile opt-in. Single-relation accessibility APIs
      may expose only the first target; exported DOM sequences must stay intact.

## Autocomplete and suggestions design for CEM inputs

The shared controller, declarative field composition, native profiles and
host-integrated fixtures are implemented. Remaining acceptance uses the
[suggestions acceptance record](../packages/cem-components/docs/suggestions-acceptance.md)
and its repeatable device procedures; automated input does not establish
physical-device or spoken assistive-technology coverage.

- [ ] Complete physical touch/pen, real keyboard/IME, screen-reader and mobile
      viewport runs for both field providers. Record browser/OS/device/AT versions
      and exact outcomes. Verify tap preserves the editor/caret, pan scrolls,
      terminal IME keys neither select nor submit, and active-row/status
      announcements preserve the field's label/help. Run the plain overflow-box
      touch baseline: the automated gesture did not scroll it in this environment,
      and a wheel-scroll pass does not substitute for touch-pan evidence.
- [ ] Record supported-browser native datalist selection, filtering,
      accessibility and device evidence before claiming additional-profile
      acceptance. Demonstrate numeric option selection stores/submits `1` rather
      than its label One; cover email/URL sanitization, range/step mismatches,
      source withdrawal, physical IME and mobile keyboard/AT. Headed Chromium
      and Firefox probes recorded value/FormData and withdrawal, but did not
      establish visible picker selection; numeric Arrow Down/Enter produced `0`.
      WebKit probes remain blocked by missing host libraries. A connected list
      alone is not selection evidence; unsupported native UI preserves ordinary
      editing without a custom popup fallback.
- [ ] Record supported-device Back/dismiss-gesture evidence before advertising
      platform close support. Use the host-enabled acceptance procedure; the
      ordinary local gallery supplies no platform-close authority. Verify one
      accepted request dismisses the admitted child, preserves text and parent
      state, and releases the watcher so normal navigation resumes. Existing
      Chromium protocol evidence does not establish physical Back/gesture support.
- [ ] **Only when requested:** design field-owned label/submission conversion,
      multiline/email-multiple token completion and full surface-provider
      adoption. Carry explicit value/provenance, selection/history, native
      semantics and relationship grants into each separate contract. Labels must
      not rewrite numeric values; token edits must preserve caret, separators and
      composition; one provider owns each adopted surface.

## Browser failures observed during placement verification

Preserve meaningful assertions and existing story/global deadlines. Capture
failing lifecycle state before changing readiness or runtime behavior; passing
repeats alone do not attribute a historical failure.

- [ ] Fixture: attribute the historical aggregate-only data-table/set-URL and
      native suggestions pointer-tap deadlines recorded on 2026-10-08. Establish
      isolated controls, then capture the failing step under load. The separate
      pan-opening readiness defect and provider story split are complete; the
      final aggregate passed 487/487. Retain every tap, pan-cancellation and
      outside-focus assertion. See the
      [archived investigation](archive/todo-snapshot-2026-10-10.md#browser-failures-observed-during-placement-verification).
- [ ] Investigate runtime aggregate instability on recurrence. The historical
      action-cutover run passed 311/312 stories: `Payload Css Readiness And
      Hydration` expected one style but observed zero; its complete 12-story
      file passed in isolation. Later reader-admission fixture corrections do
      not attribute a post-settlement missing style. A full playground run on
      2026-10-10 also hit the gallery sample-readiness deadline once; all 19
      galleries passed in isolation and the complete source/packed rerun passed.
      Use the verifier's component, URL, missing-host and runtime diagnostics;
      preserve the readiness assertion and reproduce before changing behavior.

## Immediate: Legacy Demo Case Coverage

The static readiness inventory and reproduced scoped-CSS, external-src, retry,
NPM and string-demo fixture corrections are complete. Remaining work is traced
recurrence attribution, not wholesale replacement of frame waits. See the
[current boundary inventory](browser-stabilization-review.tmp.md#reconciliation-of-the-remaining-readiness-inventory).
The approved sequencing permits component migration while these historical
investigations remain open.

- [ ] Attribute the source-loaded cell-overrides stock sample's historical
      initial-warning timeout on traced recurrence. Retain its diagnostic target
      and current deadline. After refreshing packaged WASM, all 32 concurrent
      helper/real-component probes passed on 2026-10-08; warning publication took
      5.65–9.35 seconds under overlapping load without reproducing the failure.
- [ ] Capture declaration/render/worker state on recurrence of the location
      samples' 180-frame wait for both readers. Focused overlapping load passes
      without attributing the historical failure. Preserve exact per-card output
      and resource predicates; inspect authored instances and available lifecycle
      signals before changing the fixture. See the
      [NPM/location evidence](browser-stabilization-review.tmp.md#npm-and-location-readiness-under-overlapping-load).
- [ ] Capture HTTP's historical 300-frame initial article-count timeout using
      the existing opt-in lifecycle/per-card tracing, which rethrows the original
      error. Distinguish declaration/library startup, current rendering and
      future HTTP completion before choosing a correction. If startup polling
      is premature, await existing lifecycle settlement before checking each
      authored instance; preserve resource assertions and the 30-second story
      deadline. Traced follow-up workloads pass without reproducing the failure;
      see the [HTTP observation](browser-stabilization-review.tmp.md#http-startup-observation-without-a-reproduced-failure).

## Deferred Roadmap Work

The Edge/SSR host fixtures belong to Phase 3.5 after the browser substrate is
stable. Moving `@epa-wg/custom-element` into the monorepo and deciding final
legacy XSLT preservation belong to Phase 3.6. Swift/Xcode plus Kotlin/Compose
compile gates remain Phase 8. Live Figma UI Kit and prototype work is deferred
until final [Phases 10 and 11](../roadmap.md#phase-10---figma-ui-kit). Their
checklists live only in the roadmap. Finish and verify `cem-elements` and the
declarative `cem-components` migration first, then the remaining non-Figma phases
through Phase 9 release governance.

## Deferred work: Declarative Amsterdam 2026 CEM presentation

- [x] Include `packages/cem-components/playgrounds` and its assets in the demo
      copy; the component index linked to pages previously omitted from deployment.
      Refreshed `/home/suns/cem-bin`; verified 20 gallery links, 30 action-gallery
      import-map targets, and rendered action buttons without missing resources.

- [x] Export CSV to XML through the retained typed data tree. Verified namespace
      declarations, escaped cells, and browser coloring. All 55 native conversion
      tests and 18 browser tests passed, plus lint/typecheck. Refreshed the packaged
      demo and verified CSV → XML on ports 8767 and 8768.

- [x] Append YML to JSON and CSV to CEM-ML playground examples. Verified URL format selection and colored conversion output for both examples.
- [x] Add CSV-to-CEM-ML export through the shared typed import and CEMT writer.
      Bind the imported data namespace for valid CEM-ML output; preserve string
      cells and header policy. Four native CSV tests, 18 Chromium stories, lint
      and typecheck passed. Verified colored output on both source and packaged
      sites. `yarn copy:demo` refreshed `/home/suns/cem-bin`; fixed the elements
      build to avoid recursively compiling temporary CLI-generated sources.

- [x] Refresh the packaged demo site with `yarn copy:demo`. Fixed the components
      build to consume Nx-built dependency declarations instead of recursively
      compiling the CLI's removed temporary command-schema source. Copied 470
      files and generated 7 in `/home/suns/cem-bin`; Chromium verified the
      homepage link, all four playground examples, and reciprocal gallery links
      against the packaged shared WASM runtime without browser/request errors.

- [x] Append the Markdown cem-ml playground example; verify Markdown and embedded CEM-ML SVG render together and the shared link restores the source. Build and 18 Chromium tests passed; served-page check confirmed link restoration and inline SVG rendering.

- [x] Add playground title/description fields and named Hello HTML, Hello
      CEM-ML and login CEM-ML links. Include metadata in shared URLs and append
      each copied link to the list; verify preset rendering and URL restoration.
      All three preset links restore and render. Copied links retain metadata,
      source and formats; clipboard fallback also appends the link. Login forms
      render outside the playground controls form. Build, lint, typecheck,
      Storybook build and 18 Chromium tests passed; served-page checks covered
      clipboard success, URL restoration, append order and mobile layout.

- [x] Add a CEM demo playground with editable, colored source, format selection,
      native WASM parsing/conversion, an inline HTML or colored text preview, and
      shareable source/input/output URL parameters. Cover editing, conversion,
      diagnostics and URL round trips with browser interaction checks.
      Delivered `packages/cem-demo-element/demo/playground.html`: Auto input,
      12 textual formats, HTML default output, and clipboard fallback. Build,
      lint, typecheck, Storybook build and all 16 Chromium tests passed. Served
      page checks confirmed URL restore, clipboard success, native multiline
      editing, colors and mobile layout. Unsupported conversions show native
      diagnostics; Auto is a syntax hint with an explicit format override.

Presentation preparation started 2026-10-10 and was deferred at the user's
request on 2026-10-11. On 2026-10-17 the user prioritized Studio functionality
and availability of the presentation's sample set. The active sample checklist
below takes precedence over device acceptance and helper work; the remaining
deck, website, release and rehearsal actions here remain preserved. Read the
[presentation handoff](presentation/declarative-amsterdam-2026-handoff.md) after
clearing chat context; it records the starting material, implementation
checkpoint and return path. The existing
[session proposal](declarative.amsterdam.2026.md) is the starting brief. The
[working presentation document](declarative-amsterdam-2026/presentation.md)
records the 2026-10-11 program/discussion review, current agenda and
[presentation minutes](declarative-amsterdam-2026/presentation.md#presentation-minutes).
The user requested adoption-focused framing on October 10 and alignment with
the public agenda on October 11. Use the pitch to explain why the promised
document activities matter; preserve the login walkthrough, platform subjects,
and six proposal intervals. Correspondence and release checks remain open.
The [presentation plan](declarative-amsterdam-2026/presentation-plan.md) owns
ideas, preliminary design, and adoption planning; the
[platform pitch](declarative-amsterdam-2026/cem-platform-pitch.md) owns the
audience-facing message. This file remains the sole execution checklist.

- [x] Write the CEM manifesto and platform pitch around audience outcomes,
      declarative embeddable applications, cross-language data neutrality and
      evolutionary architecture. Ground the security story in slide 6 of the
      supplied DCE deck; distinguish current controls from complete vendor
      isolation. Record schema inventory, demonstrations and claim evidence.
      Saved in [manifesto](declarative-amsterdam-2026/cem-manifesto.md), [platform pitch](declarative-amsterdam-2026/cem-platform-pitch.md)
      and [evidence record](declarative-amsterdam-2026/platform-evidence.md).
      Documentation/source audit only; new demonstrations and benchmarks remain
      unimplemented and unverified.
- [x] Review EPA-concept and DWA as idea sources, not implementation guides.
      Incorporated independent ownership/releases, explicit effects, multiple
      presentation contexts and a proposed version-adapter proof into the
      manifesto/pitch; recorded source links and non-adopted assumptions in the
      evidence record. No implementation contracts or fixtures changed.
- [x] Compare the pitch opening with the official
      [Java EE introduction](https://docs.oracle.com/javaee/7/tutorial/overview.htm).
      Rewrite the opening and spoken pitch around the target market, delivery
      and integration costs, security/performance goals, and CEM's distinctive
      embeddability and independent evolution proposition.
- [x] Broaden the pitch beyond web applications to data processing services and
      pipelines with multiple sources and outputs. Align the manifesto, CLI
      and library use cases, audience priorities, and proposed demonstration;
      record the supported engine/graph entry points and host responsibilities.
- [x] Ground the enterprise transformation-service pitch in the Intel/SOA and
      appliance precedent. Explain format and location independence through
      adapter/resolver boundaries; add a proposed location-change proof and
      state extensible, customizable connectors as a platform capability,
      separately from format adapters and appliance comparisons.
- [x] Reframe security around runtime insulation of each vendor's application,
      pipeline stage, and connector. Promote the six layers (Memory, Storage,
      CPU, DOM, CSS, Network) to a main pitch promise; distinguish declarations,
      host grants, and enforced boundaries. Explain nested safeguards and
      budgets, with per-layer evidence and observable demonstration criteria.
- [x] Replace the engine's implementation-starting-point paragraph with the
      full SDLC proposition: source/type selection, schema reuse, extension, or creation,
      module reuse/development, composition, debugging, testing, packaging,
      browser/server/CI execution, operation, and evolution. Connect these
      stages through shared contracts and diagnostics; align the proposed demo.
- [x] Separate the internal presentation plan from the audience-facing platform
      pitch. Preserve audience strategy, proof design, adoption criteria and
      preparation requirements in the plan; rewrite the pitch as public copy
      and update document references to reflect their distinct purposes.
- [x] Tighten the full platform pitch to the user's concise, easy-to-scan style.
      Preserve its message and capability boundaries; record this style in the
      presentation plan for slides, website, and spoken copy.
- [x] Align schema choices across the pitch, manifesto, plan, and evidence:
      pick, extend, or create your own schemas. Retain all three in future copy.
- [x] Commit and push the existing preparation documents before agenda changes
      (`33e18c99` on `origin/develop`). Recheck the official program and discussion
      #24, then map their subjects into the current 60-minute run. Preserve all
      six intervals, the login lifecycle, formatting/reporting and provenance,
      and named platform modules. Reconcile plan, index, and handoff references.
      Source and documentation review on 2026-10-11 passed; validated 11 segments,
      60 minutes, all six proposal boundaries, and 292 local links/anchors.
      Demonstrations, releases, slides, and rehearsal remain unverified.
- [x] Adopt [discussion #24](https://github.com/EPA-WG/cem/discussions/24) as
      the stable resource link for slides, QR codes, handouts, and the website.
      Record Studio as not ready; keep it in the platform overview as work in
      progress and add its access link when available. Slide drafting can proceed.
- [x] Generate a slide-code preview from the existing login source with the
      native CLI's tabular formatter and the existing CEM HTML colorer. Saved
      the [preview workflow](declarative-amsterdam-2026/code-preview.md), standalone
      HTML, full screenshot, and code-only image. Chromium confirms exact source
      insertion, 61 colored tokens, and no clipping. Direct CLI HTML output
      currently stays plain; the preview uses the syntax demo's CEM WASM colorer.
      Studio is not required. Sasha approved the formatting and colors on
      2026-10-11 for the Google Slides presentation.
- [x] Create the native Google Slides deck with editable narrative text,
      diagrams, timed speaker notes and the approved code colors. Generate and
      verify presentation-only login, query and diagnostic assets from existing
      examples; record exact commands and any demo gaps. Import the local deck,
      inspect its rendered slides, and save the delivery link and generation files.
      Delivered 2026-10-11: 24 main slides, four reference slides, 28 speaker
      notes and a native-deck PDF. See [deck delivery](declarative-amsterdam-2026/deck-delivery.md).
      Verified schema failure/repair and equivalent query source spans; full
      interactive tutorial, participant installation and rehearsal remain open.
- [ ] Apply the canonical CEM theme light palette throughout the slide deck.
      Keep physical slide 2 (the original title slide) in dark mode as contrast.
      Use CEM dark mode for terminal output samples. Preserve semantic syntax
      colors and verify their contrast. The new opening teaser uses light mode.
- [x] Test native animation import for the opening: 2-second fades at 0, 12
      and 24 seconds. The converted one-slide deck retains editable groups,
      all three fades and exact delays in its exported animation timeline.
      Static rendering passed. See [opening animation](declarative-amsterdam-2026/opening-animation.md).
- [ ] Copy the prepared animated opening into the main deck and verify the
      26-second sequence in Slideshow. Import currently requires one starting
      click; try After previous in the Motion panel for automatic slide-entry
      playback, then recheck timing. The live deck remains unchanged.
- [x] Synchronize Sasha's native slide additions and ordering: preserve the DCE
      promise diagram and embedding edits, renumber every physical slide,
      reconcile timing and documentation, and verify the rendered deck.
      Reviewed 2026-10-11: 30 slides, 26 timed plus four optional references;
      all rendered pages checked. The 60-minute run and six-minute Q&A remain.
      See the [native order](declarative-amsterdam-2026/native-slide-order.md).
- [x] Develop the opening thought → DNA → CEM metaphor in the user-edited
      first slide, add speaker talking points, and verify its native rendering.
      Saved [opening idea and notes](declarative-amsterdam-2026/opening-teaser.md).
      Used canonical CEM light colors, kept slide 2 dark, and retained the
      60-minute slot with 25 main slides plus four reference slides.
- [ ] Use the presentation plan to map the manifesto and platform pitch into
      the presentation website and slides, following the current presentation
      minutes. Use familiar side-by-side representations and visible outcomes;
      prioritize XML/XSLT developers, web UI developers and platform/integration
      owners. Keep the published activities and 60-minute slot; use broader
      pipeline and security examples as companions.

- [ ] Verify the selected exercise against current CEM implementation and
      resolve remaining delivery requirements. Published title, slot, audience,
      activities, and working minutes are recorded in the presentation document.
      Gmail history, session-specific submission requirements, participant
      releases, and the complete demonstration path remain unresolved.
- [x] Develop the timed narrative and slide outline around one document's
      validation, inspection, source-linked diagnostics, conversion,
      query/transformation and light-DOM rendering. Connect the exercise to the
      wider CEM platform and distinguish implemented capabilities from plans.
      Initial outline completed October 10; revised October 11 into the current
      minutes, preserving public commitments with adoption-focused framing.
- [ ] Prepare a reproducible tutorial and presenter demo with source files,
      exact commands, expected results, an intentional error and repair, and a
      presenter-led fallback. Reuse current examples and native capabilities;
      add an explicit fixture action here before any new fixture work.
- [ ] Produce the presentation, speaker notes, diagrams and participant setup
      instructions. Check the proposed Node/Linux installation paths against
      actual available releases, and record requirements for any other platforms
      included in the session. Prepare resource entries for discussion #24;
      add verified links as materials become available. Use that stable URL
      throughout the deck; Studio remains a future participant route.
- [ ] Rehearse the timed walkthrough and verify every shown command, example,
      output and demo path. Check slide readability and the fallback flow;
      record the tested versions and remaining limitations with the materials.
- [ ] Finalize the presentation and supporting files, record their locations
      and verification in the handoff, and archive completed presentation actions.
      Then continue with **CEM Studio** below. An outline alone does not complete
      this set.

## CEM Studio delivery planning

Studio/Site migration is complete above. Keep shared component functionality ahead
of app composition; presentation preparation remains deferred.

- [x] Review CEM Studio's current implementation against the
      [Studio contract](cem-studio.md) and
      [package status](../packages/cem-studio/README.md). Define its next delivery
      milestone and add the remaining implementation and verification steps here.
  - Reviewed 2026-10-12: repository, search, import/export, command Apply and
    file-provider services exist, but startup/workbench descriptors remain
    Feature Tour-bound. Import changes the provider controller's project without
    opening it in the workbench. At review, the shared host bridge lacked tree activation
    value delivery. These are source-review findings, not new runtime test results.
    Definition checks: milestone links/anchors and changed-file whitespace pass;
    implementation and browser acceptance remain open below.

### Next milestone: user-owned project workspace

Deliver the [bounded project lifecycle](cem-studio.md#next-delivery-milestone-user-owned-project-workspace)
in dependency order. Each fixture step precedes its implementation; existing
Feature Tour, revision, native-command and offline evidence remains mandatory.

- [x] **Shared event-to-command bridge first.**
  - [x] Fixture: bind `cem-tree-activate` declaratively to a host service and
        deliver only its validated string value from the exact component owner.
        Cover keyboard/pointer activation, malformed detail, undeclared events,
        nested views/owners, disabled/cancelled events, revocation and disposal;
        retain native action gesture-stack and form/close regressions.
  - [x] Implement the bounded bridge in `cem-elements`, document its scalar
        request/trust contract, and pass focused shared-runtime browser stories.
        Do not add a Studio tree event listener or arbitrary event-detail handoff.
  - Evidence (2026-10-12): both new canonical-tree worker/fallback fixtures first
    failed on missing delivery, then passed with the shared bridge. The combined
    host-view, navigation-content, form-control and canonical tree suite passes
    **19 Chromium plays**, including native pointer/Enter/Space, focus without
    selection, exact owner isolation, malformed payload rejection, capture-time
    disposal, synchronous native action delivery and truthful custom-event trust.
    `cem-elements:build` and `verify-package` pass (276 packed files, clean import);
    `cem-elements:lint` passes with 15 existing warnings outside the edited files.
    Component `verify-tree-forced-colors` and `verify-gallery-navigation` pass,
    including source/isolated-package properties, bundles and five-theme galleries.
  - CEM review: activation remains a semantic request; shared native owners retain
    hierarchy, keyboard/focus and application-owned selection. The bridge permits
    one documented scalar event, adds no styling/token defaults or visual variants,
    and preserves existing tree theme/forced-color behavior. Shared guarded-dialog
    workflow evidence is recorded below; broader device/browser/AT acceptance remains open.
- [x] **Prove shared project-navigation controls before app adoption.**
  - [x] Fixture: reconciliation must preserve an explicit surface focus return
        and never rebind a removed control's render identity to another produced
        instance. Fix focus-snapshot ownership in the shared DOM projection.
  - [x] Fixture: removing a conditional dialog returns focus to its retained
        entry destination only when it still owned focus; cover explicit opt-out,
        removed destinations and focus moved elsewhere. Keep this lifecycle in
        the shared native-surface capability.
  - [x] Fixture: settling a mounted view must include descendant renders caused
        by ancestor payload updates, so retained tree selection and editor values
        are current before host callers continue. Fix the shared settlement order.
  - [x] Fixture: library workflows for list/action selection, tree expand versus
        leaf activation, admitted versus cancelled selection, Save/Discard/Cancel
        dialog focus return, retained tabs/panes and empty/loading/error states.
        Keep all reusable interaction in shared runtime capabilities.
  - [x] Close any discovered control gaps in `cem-elements` and canonical
        `cem-components` declarations first. Update contracts, colocated plays,
        property/full galleries, source/packed delivery and Studio's control
        classification; retain the pinned Material subset boundaries.
  - [x] Verify semantic intents, five themes, forced colors, inherited token
        overrides, compact layout and keyboard ownership without app CSS patches.
  - Evidence (2026-10-12): the shipped
    [project-navigation declaration](../packages/cem-components/playgrounds/project-navigation.xhtml)
    and [contract](../packages/cem-components/docs/project-navigation-contract.md)
    compose passive lists/actions, a named resource region, tree, retained tabs,
    textarea and conditional modal decisions. Worker/fallback plays cover dirty
    navigation, delayed/failed Save, Discard, Cancel, native Escape, focus without
    selection, retained caret and explicit empty/loading/error states.
  - Shared fixes: mounted-view settlement now waits for ancestors before their
    descendants; native-surface v6 retains a source-free dialog's focused entry
    and restores eligible conditional removal; DOM focus reconciliation respects
    an explicit transfer and cannot borrow another produced owner's control ID.
    Focus opt-out, missing destinations, moved focus and same-owner replacement
    have direct regression evidence. A named resource region isolates transient
    status content from the retained workbench. No Studio UI behavior was added.
  - Verification: **41 distinct Chromium plays** pass across host-view,
    navigation-content, native-surface, projection focus, DOM merge, form-control
    and canonical tree/dialog/tabs suites; **19 declaration-shape unit tests**
    pass, including the corrected existing progress-status capability inventory.
    `cem-elements:build`, `verify-package` (276 files, clean import) and `lint`
    pass; lint retains 15 existing warnings and adds none. Component declarative,
    catalog/state-matrix prerequisites and `verify-gallery-navigation` pass from
    source and isolated package archives, including the exported workflow,
    properties, bundles and five-theme galleries. Studio UI classification passes
    for all 23 rows; relevant rows now link this shared evidence.
  - CEM review: Save/Discard/Cancel retain primary/destructive/contextual intent;
    the composition adds no CSS, token defaults or variant families. Its native
    controls own keyboard/focus while scalar services own admission. Both delivery
    paths verify a 360px viewport, retained selection, inherited spacing and focus
    token overrides, all five themes and system-color focus under forced colors
    with reduced motion. The pinned Material subset boundaries are unchanged.
    Production project transactions, stale completions and broader device/browser/AT
    acceptance remain in their following checklist items. Presentation work stays
    deferred with its existing resume context preserved.
- [x] **Generalize the project session and workbench.**
  - [x] First deliver the session admission service before navigation wiring.
        Fixture: independent source Save/Discard/Cancel and command
        Apply/Discard/Cancel, both dirty, explicit replacement confirmation,
        persistence conflict/failure, editing during Save/Apply, duplicate requests,
        cancelled/failed candidate opening and late results. Defer discards until
        selection commits; saving must not depend on native validation succeeding.
        Keep each candidate workbench isolated until its durable identity is admitted.
        Include delayed native previews/projections so background results cannot
        replace a newer command draft or an applied baseline, and source persistence
        stays independent of native preview or validation failure; recheck and
        Apply the retained command after native preview recovers; recheck admission
        when both drafts are reverted while a decision is pending.
        Verify graph-emitted and isolated-package session exports, including an
        empty session without UI side effects; reject mismatched project/page/resource
        candidates and allow retry after an initial open failure.
  - [x] Refresh Studio's dependency fixture for the migrated shared library:
        distinguish published dependency routes from workspace demo/story routes,
        assert the exact CLI and component-gallery runtime paths, and retain the
        isolated consumer's single installed runtime check. Pack the gallery's
        demo dependency with Yarn's normal workspace-protocol conversion. Refresh
        the import-map CSP hash for the added session export.
        Fixture: required installed files must match the verified deployment/types;
        include emitted build outputs directly in the package cache inputs so a
        source change cannot reuse a stale tarball.
  - Admission slice evidence (2026-10-12): `createCemStudioProjectSession` is
    exported from the root and `/project-session`, with typed snapshots and
    transition decisions. It admits isolated workbenches only after exact
    identity checks and independent draft decisions. Discard consent is deferred
    until successful admission; failed/cancelled opens and newer edits retain
    drafts. Cancelled transactions may finish durably but cannot navigate.
    `saveSource()` separates persistence from native validation, Apply establishes
    its committed command baseline, and late previews/projections retain newer
    edits. The contract is in `docs/cem-studio.md#project-session-admission-service`.
  - Verification: `@epa-wg/cem-studio:verify:consumer` and its prerequisite gates
    pass. All 33 workbench Chromium cases pass (18 admission cases), alongside
    repository, provider, native Feature Tour, preview and shell regressions;
    lint, typecheck and all 23 UI classifications pass. Deployment verifies 169
    declared assets / 259 files, repeat-build determinism and native worker/WASM
    operation online and offline. The isolated consumer verifies both session
    exports, an empty session, current archive bytes/types and one installed
    runtime. The dependency audit asserts exactly three published and nine
    workspace routes. A failing archive-byte fixture exposed and verified the
    fix for the package target's missing direct build-output cache dependency.
  - CEM review: this slice adds service orchestration only, with no visible DOM,
    UI listeners, CSS, token defaults or variation families. Save/Apply,
    Discard/Cancel, conflict and pending states remain distinct scalar data for
    shared CEM controls. Emitted shell/workbench checks pass all five themes,
    inherited token overrides, forced colors/reduced motion and offline reload.
    This initial service slice preceded the resolver/dialog integration recorded
    below. Physical device/AT acceptance and presentation work retain their
    existing deferred context.
  - [x] Fixture: two non-seed projects, nested entries, empty project, missing
        selection, supported/unsupported resources and a command-created page;
        resolve project-owned commands/dependencies without a seed catalog.
        Cover native command-created page reopening/execution, input identity
        independent of resource order, exact binary downloads, and deployed
        project selection with optional Feature Tour expectation metadata.
  - [x] Shared select prerequisite: a changed nonempty `value-request` token
        reapplies the authored value even when it is unchanged. Fixture: preserve
        selection without a new token, reset without synthetic events, retain
        the focused control and cancel stale popup preview. Advance the shared
        capability identity with this contract. Studio uses it to
        restore admitted selection after rejected navigation; keep a stable
        CEM stack around conditional guard surfaces.
  - [x] Integrate the admission service with project-owned workbenches and shared
        native dialogs. Fixture: source Save/Discard/Cancel and command
        Apply/Discard/Cancel, both drafts dirty, cancelled dialog, failed save,
        stale Apply, duplicate requests and late completions during navigation.
        Keep drafts on failure; commit selection only after guarded admission.
        Exercise mounted native dialog Save/Apply failures, successful persistence,
        Escape focus return, and cancellation while a candidate opens.
  - [x] Select project/page/resource by durable identity, retain exact revision
        and stale-result authority, and adapt Feature Tour metadata as optional
        sample data. Implement draft/transition guards before wiring navigation;
        keep selected identity/preferences outside portable exports.
  - Resolver/navigation evidence (2026-10-12): project workbenches load durable
    page/resource identities, exact revisions and declared dependencies without
    requiring a seed catalog. Native CLI command resources are decoded through
    the shared grammar, select their input independently of resource order and
    execute exact persisted bytes. Empty and read-only resources remain selectable;
    original binary bytes are downloadable. Optional sample expectations and
    legacy portable scenarios retain Feature Tour compatibility.
    Startup now uses the guarded session adapter and project-record selector.
    Source and command decisions, incompatible replacement, failure, cancellation
    and late candidate disposal preserve the admitted workbench. Nested page labels
    retain parent context; refreshed records expose newly created command pages.
  - Shared prerequisite: `choice-select` v2 adds explicit `value-request` replay
    without synthetic input/change events or stale popup commitment. Studio's
    stable `cem-stack` composition preserves the launcher while conditional
    dialogs appear, and cancellation restores both admitted value and focus.
    Replacement has one modal owner during guarded Apply.
  - Verification: all 40 Studio workbench Chromium cases pass, including seven
    project-record/mounted-navigation cases; all 13 select stories and 19
    declaration/capability unit cases pass. Native deployment checks pass all 32
    schema-package examples, portable operations and online/offline recovery.
    `@epa-wg/cem-studio:verify:consumer` passes all 48 prerequisite/consumer
    tasks: lint/typecheck, all 23 UI classifications, 169 declared deployment
    assets / 259 files, deterministic builds, exact dependency routes, current
    installed archive bytes/types and a single installed runtime. The consumer
    verifies both new workbench factories as well as the session exports.
    Explicit Parse prepares the command editor for enabled-state theme checks;
    no load-time command is inferred to satisfy an old fixture.
  - CEM review: shared select/dialog/action/editor/status controls own interaction
    and presentation. Studio adds repository/session orchestration and CEM-ML
    composition, with no application UI listeners, styling, token defaults or
    parallel variation families. Five themes, inherited token overrides, system
    colors, reduced motion and keyboard/focus behavior pass deployed checks.
    Home, retained explorer/search panes, local selection recovery and full
    provider coordination remain below; broader device/AT acceptance and the
    deferred presentation resume context are unchanged.
- [x] **Deliver Home, project creation and import.**
  - [x] Fixture: deployed bundled starter passes native validation and exports one
        source resource; empty Home and unavailable storage remain readable.
  - [x] Fixture: mounted shared Home controls create/list/open projects through draft
        guards; cancelled pickers, duplicate archives, invalid names and failed writes
        retain the active draft and repository contents.
  - [x] Fixture: list/open existing projects, create from a validated bundled
        starter with a fresh project identity, and import a distinct archive;
        duplicate identities, invalid input and quota failure leave existing data
        unchanged. Confirm that an imported project becomes selectable/openable.
  - [x] Compose Home from shared fields, lists, actions and status controls;
        use existing validated repository transactions and native format boundaries.
        Present empty/storage/error states and preserve existing user copies at boot.
  - Implemented: persistent declarative Home, repository-owned project choices,
    UUID starter creation, create-only archive import, explicit refresh and guarded
    Open project. Successful creation/import leaves the current draft in place.
    Browser picker cancellation and file-read failures settle without hanging.
  - CEM review: existing shared card, select, text-field, action and alert controls
    own semantics and styling; no app UI listeners, local CSS, private token defaults
    or new behavior were needed. Deployed theme/inherited-override, forced-color
    and offline checks pass. Broader device/AT acceptance remains below.
  - Fixtures: 42 Chromium workbench cases pass; mounted Home covers empty/storage
    states, create/import/open, duplicate/invalid/quota failures and cancellation.
    The deployed native starter exports one source and no inferred command pages.
  - Final evidence: `yarn nx run @epa-wg/cem-studio:verify:consumer` passes all
    48 prerequisite/consumer tasks, including lint/types, all UI classifications,
    deterministic 259-file output, native worker/online/offline regressions and a
    clean seven-package consumer with one runtime copy. `git diff --check` passes.
- [x] **Deliver explorer, search and retained workbench panes.**
  - [x] Fixture: shared tab host events deliver only validated owner values; repeated
        diagnostic/result navigation reveals the retained source pane before applying
        its caret/focus request, without leaking nested tab events.
  - [x] Shared hash fixture: version binary-bearing content addresses, digest actual
        repeated artifact bytes once per call, preserve nonbinary v1 addresses,
        detect byte mutations and make cloned/shared buffers content-equivalent.
  - [x] Shared scale fixture: render a 128-resource tree without serialization
        failure; preserve every identity and expansion/selection semantics.
  - [x] Shared prerequisite fixture: bind a host service to an authored declarative
        child without replacing it; retain identity/caret through parent renders,
        reject foreign/wrong-tag/already-owned targets and revoke disposed commands.
        Prove worker/fallback behavior before Studio retained-pane adoption.
  - [x] Fixture: mounted recursive tree uses stable whitespace-safe keys, separates
        parent expansion/focus from admission, retains committed selection on Cancel,
        and preserves source/command nodes and caret across retained pane switches.
  - [x] Fixture: bounded deterministic repository search admits exact result identities,
        rejects stale completions, reports empty/error states, and refreshes newly
        applied command pages without losing drafts. Verify compact/expanded viewports.
  - [x] Fixture: nested keyboard navigation, stable opaque entity keys,
        parent expansion without navigation, focus without selection, deterministic
        search/result activation, and caret/selection retention across pane changes.
  - [x] Compose canonical tree, local-view tabs and existing editor/result views;
        bind all navigation through the project session. Reuse the bounded search
        service and project-owned command Apply/current/existing/new page paths.
        Initial compact and expanded layouts both use retained local-view panes.
  - Implemented: recursive repository explorer, stable encoded record keys, guarded
    tree/search admission, deterministic 50-result search with stale-completion
    rejection, and navigation refresh after durable workbench Save/Apply. Shared
    tabs retain Source/command and Results at compact and expanded widths;
    diagnostic/provenance actions reveal Source before requesting caret/focus.
  - Shared prerequisites: host views bind authored direct children with revocable
    service ownership; validated own `cem-tab` values cross the host bridge.
    Binary-bearing content addresses use `stable-json-fnv1a64-binary-v2`, hashing
    actual artifact bytes once per buffer per call. Nonbinary v1 keys are unchanged;
    old binary v1 cache entries require fresh publication, not silent reinterpretation.
    Native typed artifacts and their validation/authority remain intact.
  - Layer evidence: 45 Studio Chromium workbench cases, 8 worker/fallback host-view
    stories, 9 shared tree stories (including 128-resource expansion), 5 projection/
    content-address unit cases, 49 processing engine/host unit cases and 58 edge/SSR
    unit cases pass. Shared lint passes with 15 existing warnings and no new warnings.
  - CEM review: canonical tree/content and tabs/navigation semantics are preserved,
    including transparent resting tree paint, independent focus/selection, retained
    node identity and service-admitted navigation. No application CSS, private token
    defaults or application UI listeners were added. Deployed checks cover all five
    themes, inherited tree/control overrides, forced colors/reduced motion, retained
    editor/caret at 390px and 1280px, and offline reload. Broader device/AT acceptance
    and resizable simultaneous panes remain separately open.
  - Final delivery evidence: `yarn nx run @epa-wg/cem-studio:verify:consumer`
    passes all 48 tasks, including lint/types, 23 UI classifications, deterministic
    170-asset/260-file output, native worker/Feature Tour regressions and a clean
    seven-package consumer with one runtime copy. `git diff --check` passes.
- [x] **Finish recovery and active-project provider coordination.**
  - [x] Fixture: recover only admitted identities; missing/trashed records fall back,
        cancelled navigation keeps the committed identity and exact saved bytes.
  - [x] Fixture: repository invalidation refreshes explorer/search/Trash without
        replacing drafts; stale revisions fail Save and restore preserves bytes.
  - [x] Fixture: provider status and picker completion cannot retarget a changed
        active project; cancellation/denial retain archive fallback access.
  - [x] Fixture: update activation blocks unresolved source/command drafts,
        navigation/provider work and edits made during asynchronous persistence.
  - [x] Fixture: exact bytes and last committed selection after offline reload,
        multi-tab revision conflict, missing/trashed selection recovery,
        export/trash/restore and search invalidation, active provider target,
        picker cancellation/denial, archive fallback and safe-update persistence.
  - [x] Coordinate the session, shell/update and provider services; display
        conflicts without overwriting drafts, keep browser permissions explicit,
        and expose durable-save versus validation outcomes separately. Explicit
        Save/Apply is this slice's durability boundary; autosave remains later work.
  - Admission stores only local project/page/resource identity; reload reopens
    IndexedDB bytes. Missing/trashed identities fall back with a recovery alert.
    Repository notifications refresh navigation/search without replacing drafts;
    Home Trash/Restore remains reversible and revision-checked. Trashed resource
    writes are rejected until restore; stale saves retain their draft and conflict.
  - Provider actions capture the active target; late picker/status results cannot
    retarget the session. Cancellation and denial retain explicit archive fallback.
    Update activation waits for Home/navigation/provider work and both draft kinds;
    Save/Apply or reset must resolve drafts before activation. No implicit command
    Apply or draft autosave was added.
  - Build-order fix: shell delivery now depends on determinism, just like the
    full worker delivery check, so it cannot read output during a clean rebuild.
  - Evidence: 46 workbench and 9 shell browser cases pass, including actual Home
    Trash/Restore actions, multi-instance change delivery, retained drafts, revision
    conflicts, byte-exact export/recovery and stale provider completion. Deployed
    output reopens a non-default second project offline with the matching provider
    target, then preserves native Feature Tour/command regressions.
  - CEM audit: shared select/action/alert semantics, destructive Trash intent and
    named status text; no app-owned UI listeners, CSS or private theme defaults.
    Emitted checks retain five themes, inherited overrides, forced colors,
    reduced motion, keyboard behavior and compact/expanded layouts. Complete
    two-project UI lifecycle and physical browser/device/AT acceptance remain open
    below/above; draft autosave and structural editing remain subsequent work.
  - Final verification: `yarn nx run @epa-wg/cem-studio:verify:consumer` passes
    all 48 tasks (41 cached), including lint/types, 23 UI classifications,
    deterministic 170-asset/260-file output and the isolated seven-package consumer
    with one runtime copy. Final output hash:
    `3f189ae4094f0c9dd7e7ed3afb06f5c78db8c5877d41f52225ae72924b014bab`.
    `git diff --check` passes.
- [x] **Close deployed acceptance and CEM principle review.**
  - [x] Fixture: drive two user-owned projects through native CEM controls for
        create/import, source save, command-page Apply/run, guarded navigation,
        export, Trash/Restore and offline reopen with exact persisted bytes.
  - [x] Fixture: constrain shared `cem-select` long resource labels to a narrow
        container without losing accessible names, option access or return focus.
  - [x] Fixture: inspect the emitted Chromium accessibility tree, unique IDREFs,
        required control names, modal keyboard containment/return, retained caret,
        visible keyboard focus and 320/390/1280px reflow; retain five-theme,
        inherited-override, forced-color and reduced-motion delivery checks.
  - [x] Fixture: run the complete two-project lifecycle from graph-emitted Studio
        output online and offline, including command-created pages and retained
        input/focus; preserve Feature Tour native-command/artifact regressions.
  - [x] Include all new declarations/assets in the deployment graph, verify the
        isolated package consumer, run focused layer checks and Studio `check`,
        then record exact evidence and limits. Complete the semantic/token,
        five-theme/forced-color, keyboard and compact-layout audit before closure.
  - Deployed evidence: the dedicated `verify:workspace-delivery` gate drives the
    real file picker/download and CEM controls across two user-owned projects.
    Source Save/validation, native command-page Apply/search/run, cancelled and
    discarded navigation, exact UTF-8 backup, Trash/Restore, offline Save/Apply/run
    and reload all pass. The first project's saved bytes remain unchanged.
  - Chromium 148.0.7778.96 on Linux exposes 46 named interactive controls with
    unique IDs and valid label/help/control references. Modal Tab may visit browser
    chrome but never the inert background; Escape restores the select launcher
    while editor identity, draft and caret remain intact. Keyboard zebra focus
    is visible; page width equals 320/390/1280px at each tested viewport.
  - Shared fix: `cem-select` grid/control minimum sizing now permits its existing
    ellipsis to contain long selected resource labels. The new 240px fixture first
    reproduced 799px overflow, then passes with all 14 select stories, preserving
    option access, accessible naming and return focus. No application CSS or UI
    behavior workaround, theme defaults or parallel tokens were introduced.
  - CEM review: semantic action/status roles, bounded shared-control geometry,
    public token ownership and inherited theme behavior remain intact. Five-theme,
    inherited-override, forced-color and reduced-motion delivery gates remain in
    the aggregate check. `workspace-delivery.json` records the browser and scope;
    release evidence requires and references its lifecycle/accessibility results.
  - Limit: this closes automated Chromium deployed acceptance only. It does not
    claim WCAG certification, physical touch/pen/IME or spoken screen-reader
    acceptance; the independent device/browser/AT checklists stay open above.
  - Final evidence: `yarn nx run @epa-wg/cem-studio:check` passes all 51 tasks
    (32 cached), including 46 workbench cases, 9 shell cases, 32 native Feature Tour
    samples, 23 UI classifications, deterministic 170-asset/260-file output,
    clean seven-package consumer with one runtime copy and release evidence.
    The separate shared select suite passes all 14 stories. Output hash:
    `08cf4e2d5ccecd14e86d576b84c1f2dcd2032c7691e2b457cc589f4f56aba667`.
    `git diff --check` passes. Draft recovery/autosave is the next implementation
    milestone; structural editing follows. Presentation context remains deferred.

- [x] **Local draft autosave and recovery** — journal source and command text by
  admitted project/page/resource identity; never Save, Apply, or Run implicitly.
  - [x] Add browser fixtures for reload recovery, exact Unicode/empty drafts,
    baseline conflicts, corrupt/quota storage, discarded navigation, cancelled
    admission, and independent project journals; verify no repository mutation.
  - [x] Provide shared CEM alert/action recovery status, backup download and
    explicit removal of blocked recovery records; retain failed writes in memory.
  - [x] Verify deployed reload recovery and conflict controls, Studio checks and
    CEM principle review.

  - Evidence: `yarn nx run @epa-wg/cem-studio:check` passes all 51 tasks
    (35 cached), including 51 workbench cases and deployed offline recovery,
    backup download and conflict clearing. Release evidence requires
    `workspace-delivery.json`'s `draftRecovery` flag. Clean seven-package consumer
    retains one runtime copy; 170 assets / 260 files remain deterministic.
    SHA-256: `6f0d6e559bbec8773f27236d75eaa6e71ddac4bd1d8349968c2c8b2d2782eb2d`.
  - CEM review: existing declarative alerts/actions express status and destructive
    clearing with accessible names and native focus. No app CSS, tokens, DOM
    listeners or component-local behavior were added. Inherited theme, five-theme,
    forced-color, reduced-motion and deployed reflow checks pass in the aggregate.
  - Bounded contract: immediate local recovery is separate from project Save/Apply
    and exports; records are capped at 1,000,000 UTF-16 code units. Storage failures
    expose current in-memory backups. Conflicts do not merge automatically;
    simultaneous cross-tab writes are not transactional. Pending targets without
    command-text changes and backup import/merge remain outside this slice.
    Physical device/spoken AT acceptance and deferred presentation context remain
    independently open. `git diff --check` passes.

- [x] **Structural project editing and undo** — native-validated, revision-checked
  project/page rename, subproject creation, subtree copy/move, sibling reorder,
  resource creation/copy; bounded session undo/redo without implicit Save/Apply.
  - [x] Add repository browser fixtures for order/export round trips, copy byte
    identity, hierarchy rejection, stale revisions, Trash and monotonic undo.
  - [x] Add workbench fixtures for clean/dirty admission, history isolation and
    divergent revisions; add deployed shared-control editing/undo acceptance.
  - [x] Extend shared host-view fixtures with a field named `id`; read form
    identity through attributes to prevent native named-control clobbering.
  - [x] Verify aggregate Studio gates and record CEM principle review and limits.

  - Evidence: `yarn nx run @epa-wg/cem-studio:check` passes all 51 tasks
    (41 cached), including 53 workbench cases, 15 repository cases and the deployed
    offline structure/edit/undo/redo/reload scenario. All 8 shared host-view stories
    pass with an `id`-named field in worker and fallback paths. Release evidence
    now requires `workspace-delivery.json`'s `structuralEditing` flag.
  - The new replace path exposed an existing IndexedDB key-cursor deletion bug;
    replacement now uses value cursors and rejects cursor callback failures.
    Form identity capture reads the attribute rather than a clobberable property.
  - Ordered native manifest arrays survive IndexedDB through private order
    metadata; portable exports gain no schema fields. Command Apply retains order.
    Undo/redo is capped at 20 snapshots / 8 MiB per project per session; divergent
    revisions disable replay. In-flight typing remains behind guarded admission.
  - CEM review: shared cards/fields/selects/textareas/alerts/actions own semantics,
    geometry, names and focus; no app CSS, public token overrides or UI listeners
    were introduced. Five-theme, inherited overrides, forced colors, reduced motion
    and deployed 320/390/1280px reflow gates pass. Clean seven-package consumer
    retains one runtime copy. Deterministic 170-asset/260-file SHA-256:
    `66cc9d77108471111a06b8ceeb443c03ee1f0c3317b8e5d9faf82508317e894a`.
    `git diff --check` passes. Physical-device/spoken AT acceptance and deferred
    presentation context remain open. Project resource rename now preserves native
    identity and resolver addresses; no authored-reference rebinding is required.

- [x] **Typed operation forms** — shared controls for native-metadata scalar,
  enum and boolean options on parse/inspect/convert/query/transform/trace drafts.
  Preserve unedited arguments and explicit Apply/Run; reject stale field submits.
  - [x] Add browser fixtures for metadata choices/defaults, literal quoting,
    unchanged repeated options, invalid choices, stale updates, no execution and
    packaged helper exports.
  - [x] Add shared tabs narrow-container coverage for wrapping labels without
    page overflow, retaining accessible selection and keyboard behavior.
  - [x] Verify deployed bounded option selection (one active editor), draft editing and explicit Apply/Run, plus
    aggregate Studio gates and CEM theme/accessibility review.

  - Evidence: `yarn nx run @epa-wg/cem-studio:check` passes all 51 tasks
    (33 cached), including 55 workbench cases, deployed option selection/Set/Reset,
    unchanged project revision before Apply, explicit Run and offline lifecycle.
    All 7 shared tabs stories pass. Release evidence requires `typedOperationForms`;
    clean seven-package consumption verifies the public helpers and one runtime copy.
  - One option editor is rendered at a time. Rendering every option concurrently
    exceeded the shared scheduler's bounded queue and left result tables stale;
    bounded declarative composition fixes that without changing scheduler limits
    or weakening the original settled trace/graph row assertions.
  - CEM review: native metadata owns grammar/choices/defaults; shared CEM controls
    own names, focus, geometry and theme styling. UI selection is a declaration-local
    slice. Shared tabs wrap at narrow widths while preserving token-derived targets
    and keyboard order. No app CSS, token defaults or UI listeners were added.
    Inherited overrides, five-theme, forced-color, reduced-motion and deployed
    320/390/1280px reflow gates pass. Deterministic 170-asset/260-file SHA-256:
    `9c94c66eea46f1c6d5bb9e0be99c4575d6ae74788e64171b06195ee217c74149`.
  - Limits: forms edit prepared scalar/enum/boolean options on six operations;
    coupled/repeated options remain in the literal editor. Switching options drops
    unsubmitted field input; Set enters the guarded/recoverable command draft.
    Existing Apply support remains parse/inspect. Physical-device/spoken AT
    acceptance and deferred presentation context remain open. `git diff --check` passes.

- [x] Retire path-rebinding and browser paint-proof prerequisites for Studio
  tree operations. Project resources retain their stable IDs and resolver addresses;
  names and paths are dataset metadata. See the identity correction checklist above.
- [x] **Grouped operation and graph authoring** — extend native-backed configuration
  editing and persistence, using shared CEM controls before app composition.
  - [x] Shared form-group foundation: ship declarative `cem-fieldset` before Studio adoption.
    - [x] Fixture: verify native legend/group naming, label/help projection, retained controls and unique description references; test fieldset disabling, native submission/reset and keyboard behavior with shared form controls, rendering the group in worker/fallback execution.
    - [x] Implement the shared XHTML declaration with inherited semantic spacing, typography and palette tokens; cover all five themes, inherited overrides and narrow/RTL layout in colocated stories, plus forced colors in source/package browser checks.
    - [x] Add property/full galleries, a bundled form-group example, catalog/index/accessibility documentation and source/isolated-package verification.
    - [x] Adopt it in Studio's existing operation-option form, retain command-view slice ownership and existing host services, and verify typed Set/Reset, stale guards and declaration-only UI composition.
    - Shared evidence: six colocated fieldset stories pass; the fieldset/field/select/action
      regression cohort passes 48 plays. Source/built import checks pass six pages;
      `verify-cem-component-playgrounds.mjs --fieldset-only` passes the property page,
      gallery and bundle from repository source and isolated package archives, including
      template metadata, original resource bases and duplicate style/registration checks.
      Build, package verification (304 files), declarative/state/catalog gates, Figma
      inventory (54 primitives, external reviews still planned), and Studio's 23 UI
      classifications pass. Component lint reports zero errors and 136 warnings;
      changed JavaScript/tooling lint is clean.
    - Studio evidence: `@epa-wg/cem-studio:check --parallel=1` passes all 51 tasks,
      including 95 browser tests (57 workbench), 32 native/deployed Feature Tour
      cases, deterministic 171-asset/261-file deployment, clean consumer/package
      checks, online/offline worker execution, two-project lifecycle, recovery and
      synchronized release evidence. The new mounted test proves native group help,
      pending disable, duplicate-activation prevention, retained group identity and
      typed Set/Reset without persistence or implicit Run. Two existing navigation
      cases timed out during concurrent browser verification; serial execution
      passed without increasing timeouts or changing assertions.
    - CEM audit: neutral comfort palette, boundary stroke, surface bend, container inset,
      related/group gaps and UI typography consume canonical D0/D1/D3/D5/D6 endpoints
      without local defaults or new token families. Inherited gap/inset/bend/font-size
      overrides (27px/23px/7px/19px), all five themes, narrow 300px groups, 360px pages,
      LTR/RTL and forced Canvas/CanvasText paint pass. Native legend/help references,
      retained values/focus/caret, first-legend exemption, nested disabling and explicit
      child state pass; nested controls retain interaction and focus ownership.
      No new production JavaScript behavior or runtime capability is needed.
    - Verification limits: physical-device and spoken AT acceptance remain open.
      The existing global `verify-cem-components-styles.mjs` audit stops on an XML-escaped
      nested `&amp;` selector in unchanged `cem-icon-button.xhtml`; its extractor/parser
      needs maintenance independently of this fieldset's browser theme evidence.
  - [x] Add native-backed atomic grouped option editing and persistence without implicit Apply/Run; retain unchanged arguments, stable resource identities and stale revision guards.
    - [x] Fixture: validate a whole option batch once, including coupled alternative replacement, literal/repeated-value retention, invalid/duplicate/unsupported edits and all-Keep no-ops.
    - [x] Fixture: reject stale/foreign or pending batches, retain the previous draft after preview failure, ignore superseded asynchronous results, and persist grouped changes only through explicit Apply and draft recovery.
    - [x] Compose bounded grouped editors declaratively with shared fieldsets/selects/fields/actions, native Keep/Set/Reset form capture and one visible editor; verify retained staging, native reset and pending disabling.
    - [x] Verify deployed grouped Set, one-transaction Apply, reload/offline persistence, clean package exports, existing single-option behavior and CEM theme/accessibility contracts before delivery.
    - Evidence: `@epa-wg/cem-studio:check --parallel=1` passes all 51 tasks (35
      cached), including 103 browser tests / 65 workbench cases. Ten focused
      acceptance cases cover the new batch/group contracts and existing field
      guards. The real command service persists both changed options in one Apply,
      reopens them with stable resource identities and leaves original resource
      records/bytes untouched. Deployed source/package delivery proves grouped Set,
      no revision change before Apply, online/offline reopening and recovery.
      Release evidence requires `groupedOperationForms`; clean consumer checks
      verify both new public metadata/batch helpers and one runtime copy.
    - CEM audit: one active editor, at most three grouped options; fieldsets,
      selects, fields, textareas, stacks and actions own native semantics, geometry,
      focus and theme paint. Labels, grouping, Keep/Set/Reset controls and bindings
      are CEM-ML. No app CSS, token defaults, DOM listeners or new runtime capability
      were added; JavaScript changes are non-UI command validation/orchestration.
      Native schema groups/choices/defaults remain authoritative. Shared five-theme,
      inherited-token, forced-color and deployed 320/390/1280px accessibility/reflow
      gates pass. Lint and staged whitespace checks pass. Deterministic deployment
      remains 171 assets / 261 files; SHA-256:
      `2f95434dd547a030b74826eff0415e17caa53054b0b253222c54bee32bd6128b`.
    - Bounds: Keep-only batches preserve exact command text; other edits use the
      shared canonical serializer, which may normalize quoting/order and materialize
      defaults. Apply retains its parse/inspect bounds. Graph/configuration authoring,
      physical-device/spoken AT acceptance and global style-audit maintenance remain
      independently open. Generated screenshots and Vitest attachments are excluded
      from the source commit.
  - [x] Add bounded graph/configuration authoring through shared declarative controls and native validation; verify persistence, undo/recovery and offline execution.
    - [x] Fixture: native AST-based graph attribute description/editing validates the entire candidate, retains graph edges and unrelated values, rejects unsupported/duplicate/invalid edits and round-trips literal Unicode/quotes without evaluating expressions.
    - [x] Fixture: exercise the exported authoring protocol in both Node/browser WASM packages, including malformed protocol rejection and exact no-op bytes.
    - [x] Fixture: declarative node/attribute selection stages a guarded source draft; verify native reset, failed/stale/pending edits, 20-snapshot/8 MiB UTF-8 history bounds during edits and replay, explicit Save and no implicit Run.
    - [x] Fixture: deployed native worker lifecycle saves a graph edit, reopens exact bytes, recovers an unsaved graph draft and runs the committed graph offline with stable resource identities.
    - [x] Update public native/Studio API documentation, UI ownership and release evidence; audit inherited themes, forced colors and narrow layout.
    - Evidence: `@epa-wg/cem-studio:check --parallel=1` passes all 51 tasks (35
      cached), including 106 browser tests / 68 workbench cases, 32 native/deployed
      Feature Tour examples, online/offline CLI workers, package/release evidence
      and clean seven-package consumption with one runtime copy. Three focused
      graph acceptance cases pass, including UTF-8 memory bounds on undo/redo.
      Native authoring (3), formatter (12) and graph parser (24) cases pass;
      direct Node/browser WASM export tests (2) exercise the same authoring protocol.
      The native formatter now chooses a representable quote delimiter instead
      of inventing backslash escapes; graph edits reject a failed literal round-trip.
    - CEM audit: node/attribute selection, form composition, Set/Reset and history
      actions are CEM-ML. Shared fieldsets/selects/text fields/actions/layouts own
      native form semantics, focus, spacing, typography and comfort/state palettes.
      Selection slices stay at their declaration owner outside projected layouts.
      Native metadata distinguishes text from enum choices; no application CSS,
      token defaults, DOM listeners or new runtime UI capability were introduced.
      Deployed five-theme paint, inherited 23px container inset, forced colors,
      named controls/ARIA references and 320/390/1280px reflow pass.
    - Persistence evidence: Set/Undo/Redo leave project revision unchanged;
      explicit Save commits exact graph bytes. Reload/recovery and offline Run
      preserve input and command records. Release evidence requires
      `graphConfigurationAuthoring`; both module maps declare the direct WASM
      dependency. A Chromium probe reproduced accessibility-CDP teardown clearing
      offline emulation while Playwright retained its cached setting; the harness
      now forces a transition and asserts offline state before and after Run.
      Lint and whitespace checks pass. Deterministic output: 171 assets / 261 files,
      SHA-256 `c1d739feae1c6718efa67e095606833c13ffc78af10735ee57229fff84ac1095`.
    - Bounds: one named-node scalar attribute at a time; 100 nodes / 1 MiB of
      structured source. Topology creation, IDs, parameters, aliases and expressions
      remain in source. Generated anonymous-node identities can prevent safe
      canonical edits; add authored IDs or use source. Literals using both quote
      delimiters remain unsupported by the structured editor. Draft history is
      session-local and bounded to 20 snapshots / 8 MiB; Save retains it, reload
      resets it. Physical-device/spoken AT acceptance and the global style audit
      remain independently open. Generated screenshots/attachments are excluded.
      The old parse/inspect-only Apply note above was stale: repository Apply
      already accepts all six portable operations; the current docs correct it.
- [x] Maintain the global component style audit: decode canonical XHTML CSS text
  and support retained native nesting before asserting action state selectors.
  Preserve the semantic token/state checks; the unchanged icon-button's
  XML-escaped `&amp;` selectors now pass through the shared native readers.
  - [x] Add audit fixtures for native XML entity/CDATA decoding, multiple static
    style blocks, nested selectors and media ancestry, and malformed sources.
  - [x] Add negative action-state fixtures proving missing states, wrong theme
    endpoints, extra geometry, duplicate declarations and literal paint still fail.
  - Implementation: use the shared CLI XML/XPath lifecycle to export only the
    selected template's decoded text, then the native CEM-ML compiler to collect
    every static stylesheet. PostCSS inspection resolves nested matching paths
    and retains all media ancestors without changing delivered CSS. Forced-color
    and reduced-motion contracts require their exact media conditions. The Nx
    gate builds both shared dependencies, hashes their outputs and runs fixtures.
  - CEM audit: retain all exact paint/geometry/keyframe assertions; recognize the
    existing D0/D5 zebra family and explicitly require the tabs' already-authored
    shrink/wrap geometry. Theme endpoints, inherited tokens, public ownership and
    the five-mode/forced-color implementation are unchanged. This tooling work
    adds no Studio or component UI JavaScript and makes no new rendered/AT claim.
  - Evidence: `@epa-wg/cem-components:verify-style-contract` passes all 12 tasks,
    including 9 regression tests and the 54-primitive / 479-token audit.
    `verify-declarative` confirms 54 declarations and zero authored component
    JavaScript/TypeScript; tooling ESLint and whitespace checks pass. Generated
    screenshots and Vitest attachments remain excluded from delivery.
- [ ] Extend native graph authoring to node/edge creation and parameter maps;
  preserve authored IDs and resource references, validate each candidate and
  verify transactional persistence/history/recovery/offline execution. Add
  fixtures before implementation and shared controls before Studio composition.
  - [x] Add native fixtures and named-node creation with schema-backed fields,
    unique authored IDs and preservation of existing resource identities.
    - [x] Add native creation fixtures for schema-derived parents/fields, every
      operation kind, literal/ID preservation, duplicate/invalid requests and
      source/node bounds before implementing the shared authoring API.
    - [x] Extend both Node and browser WASM export fixtures with creation metadata,
      successful named-node creation and duplicate-ID rejection.
    - [x] Add Studio browser fixtures for native creation forms, reset/busy/stale
      guards, reversible drafts and unchanged durable project records before Save.
    - [x] Extend deployed acceptance to create/save/reload/recover/run a named
      graph branch offline, preserving input/command resource identities and
      checking shared controls across themes, inherited tokens and forced colors.
    - Implementation: the existing native AST authoring API now accepts one
      mutually exclusive `create` request, appends under a schema-approved root
      or named parent, and describes canonical scalar fields/choices. Existing
      graph nodes and edges, parameter bindings and resource paths must survive
      formatting unchanged. Studio composes parent/kind selectors and one native
      creation form from shared controls; services only orchestrate native
      validation, guarded draft staging and the existing history/repository path.
      No new UI JavaScript capability or application CSS was needed.
    - Evidence: 7 native authoring tests, both Node/browser WASM export checks and
      6 focused browser graph checks pass. Deployed `graphNodeCreation` acceptance
      proves reset, Create, Undo/Redo, exact Save/reload, unsaved creation recovery
      and explicit offline Run producing both export artifacts. Input/command
      resource records remain identical. Both graph forms pass all five themes
      and forced colors; creation consumes an inherited 23px inset. Automated AX
      relationships and 320/390/1280px reflow checks pass with no overflow.
      The full Studio `check` passes all 51 tasks, including 108 browser tests,
      32 native/deployed Feature Tour examples, online/offline workers, release
      evidence and a clean seven-package consumer with one runtime copy. Tooling
      ESLint, Rust formatting and whitespace checks pass. Deterministic output:
      171 assets / 261 files, SHA-256
      `5aea77f3bb6aae79f9e2b045331819ee5f0e01b82a16c5aec262c1b1e1e7414c`.
      Generated screenshots and Vitest attachments are excluded from delivery.
    - Bounds: append one named node, 1–256 UTF-8 ID bytes without surrounding
      whitespace, 100 nodes / 1 MiB source; no existing-ID change or node move.
      Aliases, expressions, parameter/`with:*` maps and join modes needing new
      secondary bindings remain source-only. Mixed anonymous-node identities
      that cannot survive formatting are rejected. Physical-device/spoken AT
      acceptance remains open.
  - [x] Add native fixtures and edge creation/editing with candidate validation,
    cycle diagnostics and declarative Studio controls.
    - [x] Add native parent-move, input override/reset and named-secondary-edge
      fixtures, including cycles, invalid roles/names, opaque direct-export
      requirements, exact no-ops and preservation
      of IDs, parameters, unrelated edges and resource paths before implementation.
    - [x] Extend Node/browser WASM fixtures and Studio browser form fixtures for
      edge operations, reset, stale/foreign/pending guards and reversible drafts.
    - [x] Extend deployed edge acceptance for Save/reload, recovery and offline
      execution with stable resource records, theme/forced-color/inheritance and
      accessibility/reflow checks; require the resulting release evidence.
    - Implementation: the native AST authoring API accepts one `edge` operation
      for schema-approved subtree parent moves, explicit input override/reset,
      or one named `with:*` binding. Native graph validation rejects cycles and
      invalid conditional joins. Candidate comparison preserves every node value
      and the complete edge multiset, including separate labels sharing a source;
      exact no-ops retain original bytes. Studio composes selectors, binding lists
      and one active edge form in CEM-ML using existing shared controls. Services
      only guard and stage native candidates through the existing draft history.
      No new UI JavaScript capability, application CSS or token default was needed.
    - Evidence: all 11 native authoring tests, both Node/browser WASM export
      checks and 8 focused browser graph checks pass. Deployed `graphEdgeAuthoring`
      proves reset, staged parent moves, Undo/Redo, exact Save/reload, unsaved edge
      and creation recovery, and explicit offline Run producing three export
      artifacts. Input/command resource records remain identical. All three graph
      forms pass five themes, forced colors and inherited 23px inset checks;
      automated AX relationships and 320/390/1280px reflow checks pass.
      The full Studio `check` passes all 51 tasks, including 110 browser tests,
      32 native/deployed Feature Tour examples, online/offline workers, release
      evidence and a clean seven-package consumer with one runtime copy. ESLint,
      Rust formatting and whitespace checks pass. Deterministic output remains
      171 assets / 261 files, SHA-256
      `89befb466e38f88ffddeaaa5a8b1f8ab597dca223a1627a36f766b9dde8be005`.
      Opaque imports in acceptance each retain a direct export, preserving the
      native contract; moving away their only export is a native regression case.
      Generated screenshots and Vitest attachments remain excluded from delivery.
    - Bounds: sources and targets need authored IDs, with a named lexical parent
      or run root; mixed anonymous-node graphs that change identity are rejected.
      Binding names use 1–128 ASCII name characters starting with a letter or
      underscore. Existing 100-node / 1 MiB source and 20-snapshot / 8 MiB history
      limits remain. IDs, aliases, expressions and parameter-map values remain
      source-only. Physical-device/spoken AT acceptance remains independently open.
  - [ ] Add native fixtures and parameter-map authoring, then verify Save,
    history, recovery and explicit offline Run for the expanded graph editor.

## Component gallery source-link maintenance (after Studio presentation work)

- [ ] Migrate remaining extensionless canonical-source `cem-module-url` references to full declaration paths under the folder import-map convention. Add source/isolated-package fixtures without assuming a template has only id/type attributes. The current artifact-tree increment repairs and verifies cem-media-preview; a filtered passive-component gate keeps that verification scoped while this older bulk maintenance remains separate.
