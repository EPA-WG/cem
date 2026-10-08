# Retained suggestion sources and asynchronous results

Status: adopted design, 2026-10-07, under the user's instruction to continue
with recommended options. Completes the data-lifecycle design action in
[todo.md](todo.md#autocomplete-and-suggestions-design-for-cem-inputs).
Runtime delivery and fixtures follow that action.

Suggestions consume a complete bounded native option/group sequence at a
consumer lifecycle stage. Preserve the original source owner, scopes and node
identity. Reuse the [attachment adapters](cem-suggestions-attachment-design.md#retained-source-adapters)
and [interaction contract](cem-suggestions-interaction-design.md). Loading is a
shared CEM resource/query responsibility, not a component URL attribute or an
application UI handler. The component never parses response formats or turns a
document into JavaScript records.

## Source snapshot and readiness

Static sources use native `options` or the exclusive inert options template.
No source means ready-empty. An explicit pending native binding is pending,
not an empty fallback. Adapt one source family per revision, including all
groups, labels, values and availability flags. Reject the whole invalid revision
with attributed diagnostics instead of dropping malformed rows. Distinct
original nodes may have duplicate stored values; repeated placement of the
same original node in one session is ambiguous and rejected. Neither string
equality nor row position establishes identity or initializes selection.

The first adapter accepts already-materialized scalar source attributes.
An upstream native query/template prepares dynamic `value`/`label` attributes;
the adapter does not evaluate their authored expression/reference slots.
This does not resolve descendant content references or change original owners.

Add these planned scalar coordination inputs alongside the native `options`:

| Input | Contract |
| --- | --- |
| `options-state` | `ready` default, `pending` or `failed`; an unresolved native binding cannot be promoted by this scalar. |
| `options-revision` | Producer's nonempty result revision; required for explicit asynchronous/external coordination, not authority or a source identity. |
| `options-query-revision` | Nonnegative safe integer echo of the requesting session's query revision; required for query-dependent external results. |
| `options-policy` | `vocabulary` or `page`; default vocabulary for local/none modes, page for external mode. |
| `options-error` | Optional nonempty plain-text failure feedback. Default `Suggestions are unavailable.`; details remain attributed diagnostics. |

The source nodes and these inputs publish as one prepared lifecycle snapshot,
with editor/attachment/query/opening/source/placement revisions rechecked before
activation. Do not admit torn combinations of a new node sequence and old query
metadata. Native binding preparation retains its original grants and traversal
bounds; markup revision strings cannot grant source or placement access.

Pending/failed replacement may retain the last complete source plan for
inspection. It cannot commit stale-query rows or open a failed/empty plan.
Clear preview and active-descendant when eligibility is lost. A ready, valid
replacement can recover without resetting text, focus, defaults or constraints.
Do not replace the native editor or replay a source selected flag.

## Query and resource ownership

The reserved native `datadom.slices.suggestions` view exposes the settled query,
its monotonically increasing session-local query revision and readiness/status.
Queries become observable after native editing settlement; composing edits do
not initiate suggestion requests until the shared IME route settles them. Public
setters/reset/restore close and invalidate the current request without replaying
a user opening intent. A native query/binding consumer reads that view through
an explicitly admitted relationship; it does not inspect copied JS snapshots.

For local contains/prefix modes, adapt the complete vocabulary once per source
revision and apply the pinned shared Unicode matching rules. For external mode,
a declarative native query or shared loader supplies already selected rows and
echoes the originating query revision. Do not filter those rows again. `none`
supplies query-independent rows and does not invent remote request semantics.
There is no first-profile debounce or minimum character count; a producer may
use the shared loader's explicit scheduling policy without adding UI behavior.

The [shared loader](cem-data-loader-plan.md) owns transport, content-type decoding,
native import, resource cancellation, source attribution and bounds. It already
publishes scheduled/in-progress/loaded/failed metadata and a retained document
binding. A native query maps its imported vocabulary to the accepted option/group
family; it may produce an explicit native derived view with source edges, rather
than record normalization. Coordinate scheduled/in-progress as pending, a complete
loaded result as ready, and failure as failed. HEAD/no-document is not an options
vocabulary. The declaration supplies loader/query bindings; no `options-url`,
response JSON parsing or second autocomplete fetch implementation is added.

Native capability-view publication and its outbound query relationship require
a shared runtime extension before advertising asynchronous composition. Pass
the retained view and lifecycle metadata through the existing typed/binary
processing boundary, with worker/fallback and source grants intact. Do not
implement a component-specific DOM event payload containing response records.
The implementation action includes that extension and atomic result admission.

### Native session transport

Adopted 2026-10-07:
executable source capture uses a separate CEMB-backed capability session;
CEMV remains the materialized presentation boundary. A session imports its
original sources and lexical capture once, prepares explicit runtime inputs,
directed grants and effective request/destination bounds, then retains its
ordered native selection. The default consumer resolves selection reference
chains, while descendant references remain authored until another explicit
consumer evaluates them. Owning selection containment does not add reference
depth. An incomplete or denied selection cannot activate as ready-empty.

Complete the selected forest's namespace names through the existing lifecycle
stage. The resulting native query view retains the original source handles;
another session can complete the same source differently without changing it.
Namespace preparation has one bounded work allowance across source forests.
It does not expand general descendant references or grant new crossings.

Label templates receive the retained selected native input in the session's
frame. Explicit presentation queries may export CEMV values for render/binding
consumers. An export containing executable source capture still fails with its
native diagnostic; projecting a presentation value does not transfer source
authority. Original source edges stay inside the native session rather than
becoming serialized JavaScript records.

Processing-host protocol v21 provides prepare/view/render/release operations
with immutable source revisions and root-owned session identities. Source leases
survive query changes; immutable publications capture query revisions and
consumer leases check attachment/query eligibility before and after async work. Cancellation, release and root disposal fence in-flight preparation and
release native owners. Default capacity is 64 live/preparing sessions per engine;
the embedding engine may configure its capacity. Issued session identities are
bounded and cannot be reused during that engine's lifetime. Views and label
templates use explicit value/byte/index bounds as well as native execution
limits. These handles and leases never enter durable data-island state.

Startup fallback may prepare a new native session through the same code. Loss
of a worker invalidates its live sessions: views or label work cannot silently
reload old grants into fallback. Resume/retry obtains fresh source authority and
a new session identity. The shared transport and versioned native source adapter
are implemented. An immutable session prepares its option/group plan once;
subsequent native frames apply filtering and row/group label templates while
retaining original source/content edges. Adapter preparation returns scalar
counts, consumer identity and attributed diagnostics. Explicit scalar queries
can export CEMV; exporting the live derived view fails because CEMV cannot retain
its source/content edges. Native label frames bind the reserved view locally.
Canonical component render/diff frames consume host-admitted outbound bindings
on the original publication owner, with one live owner per frame. Original
source-to-row placement grants now use exact nonserializable row handles.
A bounded scalar control response supplies each row’s commit value, current query
eligibility and source availability; it never exports the original source tree.
Opaque original-source identities are session-local and stable across queries.
Publication/consumer release invalidates its rows immediately, while source
replacement, worker loss and root disposal also revoke retained commit proofs.
Provider relationships activate only after the host maps the exact rows to
committed DOM shells and grants both directions. Native rendering now consumes
`suggestion-row` on an option shell and returns bounded placement metadata.
The value must be one exact row from the current publication, directly or through
an already-evaluated native reference. Source nodes, scalar handles, prior-view
rows and duplicate row placements are rejected. The annotation is removed before
DOM/CEMV export. The runtime binds the metadata to exact committed elements under
the current render attempt; neither saved IDs nor row positions restore it.
The production declaration remains open pending the shared adapter for the adopted
[local host authorization](cem-suggestions-attachment-design.md#local-host-authorization).
Explicit host opt-in may authorize native capture of the attachment's local inert
options template; it does not authorize foreign references inside that template.
Capture retains original lexical bindings and effective bounds. Query filtering
reuses the captured source session; source replacement or authorization loss
invalidates it and any dependent publication/commit proof. The opt-in is host
policy, excluded from source attributes and durable resume state.

Request replacement/disconnect asks the shared resource owner to cancel its
obsolete work. Cancellation may not stop an already finishing transport; every
completion must still match the requesting attachment/editor/query revision and
opening generation. Late results may be retained by the resource owner for
inspection but cannot reopen a dismissed session, change its text or steal focus.
An accepted later query can open only under its own qualifying interaction.

## Display pages and committed provenance

An accepted commit retains the original candidate/source owner and stored value
at the field's commit revision. Two equal-value candidates are separate accepted
identities. Public edits, reset/restore and equal-value setters clear proof under
the interaction contract; filtering or display-page turnover does not infer or
restore it.

With `options-policy=page`, removal from the displayed result means only that
the candidate is absent from this page. Its accepted proof survives while its
original owner/authority remains valid. A new page may contain another equal-value
node without transferring proof to it. Pending/failed requests also do not revoke
an accepted proof merely because they lack rows.

With `options-policy=vocabulary`, a complete ready valid replacement is
authoritative for this attachment. If it no longer contains the exact original
committed candidate, or marks that candidate hidden/disabled, revoke proof without
rewriting the field. A pending/invalid replacement cannot silently revoke proof
from the last complete vocabulary. A wholly new owner with equal strings is a
new vocabulary; recommit is required in constrained mode. A source grant/owner
revocation invalidates proof immediately for either policy. Never treat the
policy string as a grant or an authorization bypass.

Local display filtering does not change the complete vocabulary and never
invalidates proof by itself. Constrained validity depends on this retained proof,
not on the current filtered page or label. Optional empty and native required
retain their existing interaction/field rules. Proof is transient and is not
restored as serialized authority on hydration.

## Loading, failure and accessible feedback

Readiness belongs to the suggestions view and listbox, not the field's `busy`
attribute. Set `aria-busy=true` on the controlled listbox while preparing its
result; remove it after ready/failure. Keep typing, selection, native validity
and focus available during loading. Pending/failed plans have no eligible commit
rows even if an older visual result is retained.

Produce a declaration-owned noninteractive `part=status` region outside the
listbox, with `role=status`/polite live semantics and atomic plain text. It is
not an option, active-descendant or a new focus stop. Its default states are:

| State | Default feedback |
| --- | --- |
| Pending after an eligible focus/edit/opening intent | `Loading suggestions.` |
| Ready with eligible matches | `1 suggestion available.` or `N suggestions available.` |
| Ready-empty/no-match/all-disabled | `No suggestions available.` |
| Failed | `options-error`, or the default failure message. |

Expose feedback only for the current focused/qualifying session. Deduplicate
unchanged status text within its current query revision; arrow preview relies
on the listbox's native accessible relationship rather than announcing its label
again from the status region. Cancel/dismiss clears session feedback and blocks
late stale announcements. On failure keep text and constrained/native validity;
do not overwrite the field's error/help message with a resource error. An
application may explicitly bind field-owned busy/error policy through its own
declarative contract, but loading does not do so implicitly.

Localization is declaration-owned: custom declarations can provide status text
from the native view without replacing its listbox semantics or loading controller.
Announce honest state/count, including all-disabled readiness. Native screen-reader
fixtures must assess polite announcement behavior; status markup alone is not
interoperability evidence. The [ARIA status role](https://www.w3.org/TR/wai-aria-1.2/#status)
supplies the semantic baseline.

## Lifecycle and verification

Source preparation and publication are bounded, cancelable lifecycle work;
keyboard/pointer commit only reads the already admitted plan. Keep the original
native owner until selected/proof/view consumers release it. Disposal releases
resource bindings, placement mappings, query subscriptions and native view
leases. Resume reacquires views/owners/grants and begins collapsed, with no
replayed pending request or restored proof.

The adjacent implementation/fixture actions must cover duplicate values and
repeated node identity; atomic metadata/node revisions; pending versus ready-empty;
failure/invalid revision recovery; source/placement revocation; complete vocabulary
versus display-page turnover; external query echoes and stale completion after
dismissal; resource replacement/disconnect; final IME queries; focused status
counts/failure and unchanged field busy/error; independent consumers and
worker/fallback/hydration. These are acceptance scenarios, not tests claimed by
this design document.
