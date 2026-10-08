# Declarative suggestions attachment and capability

Status: adopted design, 2026-10-07, under the user's instruction to continue
with recommended options. This completes the attachment/slots/capability item in
[todo.md](todo.md#autocomplete-and-suggestions-design-for-cem-inputs).
The public names below are selected contracts, not implemented exports.
Keyboard/event/history and selection policy are adopted in the
[interaction design](cem-suggestions-interaction-design.md).
Static/async sources and coordination inputs are adopted in the
[source-data lifecycle design](cem-suggestions-data-design.md).

Add a non-form-associated `cem-suggestions` declaration requesting the shared
`suggestions` capability. It binds one existing field provider, produces one
manual native listbox and consumes retained option nodes. It implements the
[composition contract](cem-suggestions-composition-design.md) and
[popup-service profile](cem-suggestions-popup-design.md); it creates no input,
form value, dropdown trigger or second surface controller. The same capability
may serve another conforming XHTML/CEM-ML declaration.

## Public attachment and precedence

Provide two exclusive editor forms: one direct `slot="editor"` provider, or
an explicit `editor-for` relationship. There is no first-descendant, nearest
field or generated-input fallback. Missing or conflicting editors leave the
field's normal editing available and the attachment inactive with a diagnostic.

These HTML illustrations describe the future API; they are not runnable
fixtures or conversions of an existing declaration.

```html
<cem-suggestions label="Fruit suggestions">
  <cem-field slot="editor" name="fruit" label="Fruit"></cem-field>
  <template slot="options">
    <data value="apple">🍎 Apple</data>
    <data value="strawberry">🍓 Strawberry</data>
  </template>
</cem-suggestions>
```

The layout composition projects the existing field. Its `name`, value, label,
help, native hints and constraints stay on that field; the suggestions host
does not proxy them. An external attachment keeps the field in place:

```html
<section interaction-scope>
  <cem-text-field interaction-name="fruit" name="fruit"
                  label="Fruit"></cem-text-field>
  <cem-suggestions editor-for="@fruit" label="Fruit suggestions">
    <template slot="options">
      <data value="apple">🍎 Apple</data>
    </template>
  </cem-suggestions>
</section>
```

The native reference form is `@editor-for={#field}`, designating the original
field/provider node. It needs no context ID or compatibility scope marker.
The existing nearest-`interaction-scope` lookup is an explicit convenience for
`@local-name` strings; it does not become a CEM AST reference or issue grants.
This new endpoint accepts neither CSS selectors nor bare ID/URL strings.

| Input | Contract |
| --- | --- |
| `editor-for` | Exactly one verified editor provider, exclusive with `slot="editor"`; native singleton projection and the exact-provider endpoint adapter are implemented. Shared controller/placement integration is implemented; production composition remains open. |
| `options` | Native node/reference sequence of option/group roots, exclusive with `slot="options"`; a data input, never an ID relationship, URL, JSON string or selector. |
| `filter` | `contains` default, `prefix`, `external` or `none`; ownership and matching below. |
| `filter-by` | Local modes only: `label` default, `value`, or both tokens; match either key independently. |
| `label` | Nonempty plain-text listbox name, default declaration text `Suggestions`; does not rename the editor. A custom declaration may instead supply a valid native naming relationship. |
| `require-selection`, `selection-message` | Opt-in commit-provenance validity and its field-owned message, as defined in the interaction design; omitted requirement keeps free text. |
| `options-state`, `options-revision`, `options-query-revision`, `options-policy`, `options-error` | Atomic readiness/query coordination and page/vocabulary proof policy in the source-data lifecycle design; these strings never grant authority. |
| `placement`, `fallback`, `boundary`, `overflow` | Shared logical fitting inputs, with the defaults and admitted boundaries in the popup-service profile. |

Unknown filter modes/tokens diagnose an inactive attachment rather than choosing
a different matching algorithm. Explicit `filter-by` with `external` or `none`
is a configuration conflict. Native input `autocomplete` stays field-owned;
this component does not reuse that attribute to choose its filtering policy.

No writable `open`, mirrored `value`, submitted `name`, generated trigger,
focus-target or return-focus API is added to this host. Its manual presentation,
actual-editor anchor and retained focus are requirements of the first profile.
The [interaction policy](cem-suggestions-interaction-design.md#preview-commit-and-opening)
opens ready eligible results on fresh focus/edit or an admitted arrow request,
without a minimum character threshold. Constrained selection uses the planned
inputs above; none of these selected APIs is implemented by this document.

## Slots, parts and presentation ownership

| Hook | Selected contract |
| --- | --- |
| `slot="editor"` | One direct projected provider exposing exactly one owned, supported native editor; no arbitrary wrapper or nested-control discovery. |
| `slot="options"` | One direct inert HTML template containing the source option/group roots. Its contents are captured at the native CEM boundary, not projected as live options. |
| `slot="option"` | One direct inert `type="text/cem-ml"` template customizing label content for each row, with one native `suggestion` parameter. It does not replace the row shell. |
| `slot="group-label"` | One direct inert CEM-ML template customizing a group heading, with one native `group` parameter. It does not own filtering or create another group/listbox. |
| `part="surface"` | Exactly one direct component-owned `role="listbox"` element with `popover="manual"`, stable through opening and source updates. |
| `part="group"`, `part="group-label"` | Component-owned group container and its named heading, when the source has groups. |
| `part="option"`, `part="option-label"`, `part="option-value"` | Component-owned row shell, label content and stored-value hint when that differs from the label. |

The field retains all its own parts/slots. Do not stamp suggestions parts on
the field's internal control. Meaningful default children, multiple templates
for one slot, or an explicit input plus its corresponding slot are conflicts;
formatting whitespace/comments are harmless. A template does not count as an
editor, and an explicitly empty `options` sequence does not select slot defaults.
No source is a ready empty set, distinct from a pending explicit native input.

The first profile owns the full listbox shell. There is no `slot="menu"` or
`slot="surface"` adoption path that could conceal a menu controller or a second
native visibility owner. The legacy `input`/`menu` slots remain legacy inputs;
migration maps an existing field to `editor` and data to `options` explicitly.
Full surface-provider adoption is a separate extension if needed.

Default label rendering uses the adapter's display label and admitted static
content. Explicit native option labels take precedence over their text.
Noninteractive rich content is allowed; templates cannot introduce focus stops,
interactive controls, autofocus, independent option roles/IDs or form owners.
Validate their materialized output before enabling the session. The declaration
retains the role, state and relationship shells and always shows the stored-value
hint when label and value differ. The option's accessible name must communicate
that distinction as well. A custom label cannot conceal what commit will insert.
Paint and scoped styles stay declaration/theme-owned.

Compile label templates through the normal native CEM-ML template entry points,
supplying their parameter through an explicit native input frame. Preserve each
template's original captured bindings and scope policy. Outer projection keeps
their contents inert; no string interpolation, implicit reference evaluation or
bindings installed inside the field are substitutes for this consumer stage.

## Provider binding and reference authority

The editor provider supplies its current editor identity/revision, eligibility,
value notifications, owned attribute-claim lease and synchronous commit boundary.
These are shared-runtime extensions to `form-control`, not APIs already exposed
by its [current implementation](../packages/cem-elements/src/lib/form-control-capability.ts).
The suggestions capability rejects a bare input or a provider that only happens
to contain one. Additional native-editor adapters require their own contract.

Resolve `editor-for` through the bounded native
[reference-to-ID consumer](cem-element-reference-ids-design.md), then validate
the provider role. Add its allowlist/marker and lifecycle observation explicitly;
do not broaden the existing button/link `interactionControl` adapter. A local
slot identifies only its exact projected provider and does not waive admission
for separately produced placements. Cross-scope sources and foreign placements
require the existing [host-issued grants](cem-element-granted-placements-design.md).
Markup, adjacency, a slot name and a matching ID do not grant authority.

Field-owned writes and ARIA changes are applied by the field provider under its
lease. The suggestions producer never patches or installs event/slice bindings
inside another producer's output. The listbox producer owns its IDs and row
placements; the field producer owns its editor's claims. The host must admit
any foreign relationships needed in both directions before activation. This
requires coordinated provider/publication integration, including local compound
composition where the editor and listbox have different output owners. A host
without the necessary admissions leaves the attachment inactive, not partially
bound. Resume acquires fresh leases and grants; neither serialization nor local
convenience lookup supplies them.

### Local host authorization

Adopted 2026-10-08: the embedding host may explicitly opt a runtime into local
suggestions composition. The shared runtime then prepares native source sessions
and issues the necessary property-specific placement grants for eligible local
attachments. The default remains unconfigured/inactive. This is a host API
policy, never an authored attribute, slot, data-island value or serialized grant.
The explicit host preparation hooks remain available for other compositions.

Local admission is confined to an attachment managed by that runtime, its one
direct projected `slot="editor"` provider managed by the same runtime, its one
direct inert `slot="options"` template (or the already-defined absent-source
empty set), and its own committed listbox/row shells. Capture the template through
the shared native CEM import boundary with original lexical bindings and effective
scope policy; do not reconstruct source records from rendered options. Keep that
source session across query changes, and replace it when the source changes.
The local adapter must reject conflicting explicit inputs or ambiguous slots;
it never falls back from an invalid explicit input to local defaults.

The opt-in authorizes only that local source capture and these exact placement
relationships: suggestions producer to editor provider for `editor-for`, editor
producer to listbox for `aria-controls`, and editor producer to current native
row placements for `aria-activedescendant`. The two directions use the existing
coordinator and provider leases. Source handles, committed placement metadata,
provider identity and current revisions must all agree before activation. A
shared DOM ancestor, equal ID or same runtime does not admit an arbitrary external
endpoint. Explicit external inputs and foreign source references still require
host-prepared native inputs and their own directed grants; the local adapter
cannot invent grants for references discovered inside a captured template.

Host revocation, attachment disconnect, endpoint/source replacement and owner
loss invalidate affected preparation, publications, placement grants and provider
claims. Check the current authorization before and after asynchronous work so
late completion cannot reactivate a revoked attachment. Reauthorization or resume
starts collapsed and obtains fresh capture/admission and leases; saved IDs and
snapshots restore no authority. Query changes alone retain the source session
and replace only the immutable publication and its consumer leases.

This policy is adopted; the shared opt-in adapter and its fixtures remain an
[implementation action](todo.md#autocomplete-and-suggestions-design-for-cem-inputs).
Existing explicit host hooks remain the implemented route until that action is
complete. Production XHTML composition follows the adapter.

Only one suggestions attachment may lease an editor and one controller may own
the listbox. Competing claims suspend suggestions for that editor with a stable
conflict diagnostic; mounting order cannot choose a winner. Rebinding first
closes and releases the previous binding. An invalid explicit endpoint never
falls back to a slotted or nearby field. Readonly/disabled transitions, reset,
restore, disconnect and authority loss follow the composition/popup contracts.

## Retained source adapters

Adapt the admitted roots by their expanded names in the native consumer. Use
one source family per revision: canonical `cem-option`/`cem-option-group`, legacy
HTML `data`, or HTML `option`/`optgroup`. Mixing families or recursively searching
an arbitrary document is not automatic adaptation. A declarative native query
may deliberately map another vocabulary before supplying `options`.

| Source | Stored string | Display label and grouping |
| --- | --- | --- |
| `cem-option` | Required explicit `value`, including empty | Nonempty explicit `label` or text-derived label; admitted static children supply rich content. |
| Legacy `data` | Required explicit `value`, including empty | Text-derived label/rich content; no implicit value from that label. A nonempty explicit label may override display. |
| HTML `option` | Explicit `value` verbatim; otherwise HTML-aware text with alternative-image text excluded | Nonempty explicit label, otherwise HTML-aware display text including alternative-image text. |
| `cem-option-group` / `optgroup` | No candidate value | Required nonempty group label, direct options of its own family, source order and inherited disabled/hidden state; nested groups are rejected. |

The HTML adapter follows the
[option label/value and text algorithms](https://html.spec.whatwg.org/multipage/form-elements.html#the-option-element):
concatenate text in tree order, omit script subtrees, then strip/collapse ASCII
whitespace for the missing-value fallback. Comments do not contribute text and
inline element boundaries do not insert spaces. A different label never replaces
the text-derived value. Explicit empty values are distinct from missing values.
Use the same text collection rules for canonical/data fallback labels; rich
image-only labels need alternative text or an explicit nonempty label.

An empty or whitespace-only display label is an invalid candidate, even when its
stored value is validly empty. This adapter requires named options; it does not
insert unnamed select placeholders. Diagnose an invalid source revision before
activation, rather than silently choosing another family or publishing a
partially adapted list. Source `selected` does not initialize the field or
session selection; ignore it with a compatibility diagnostic and keep the
field's authored value.
Boolean availability follows attribute presence, including `disabled="false"`.

The first native adapter requires materialized scalar `value` and `label`
attributes, including group labels and image `alt` text. It rejects expression
or reference value slots instead of evaluating them. Prepare dynamic scalars
with an upstream native query/template before source admission. Availability
and `selected` use presence alone; their operands are not evaluated. References
inside candidate content remain authored for a later explicit consumer.

Keep the original source nodes, owners, group relationships and content handles.
The [existing record normalizer](../packages/cem-elements/src/lib/choice-options.ts)
is evidence for legacy behavior, not a new ingress: it computes native fallback
from label and uses values as identities. Neither rule is adopted. Browser code
may receive scalar commit strings and opaque handles as consumer results; it
must not replace the source tree with arrays of option records.
Duplicate string values and async replacement follow the [source-data design](cem-suggestions-data-design.md).
Value equality or a filtered array index never
identifies a candidate. Repeating the same native source node as two rows is
ambiguous; require distinct source nodes rather than choosing one placement.

## Filtering ownership and native view

| Mode | Matching owner | Editor `aria-autocomplete` |
| --- | --- | --- |
| `contains` | Shared native consumer: substring of label by default | `list` |
| `prefix` | Shared native consumer: prefix of label by default | `list` |
| `external` | Explicit upstream declarative query/loader supplies query-dependent roots; no second local text filter | `list` |
| `none` | All admitted roots remain independent of the query; eligibility still applies | `none` |

This separation follows
[WAI-ARIA's autocomplete distinction](https://www.w3.org/TR/wai-aria-1.2/#aria-autocomplete).
It is a stable description of the behavior, not a loading/match-count indicator.
External mode must publish results for the provider's captured query revision;
readiness and stale-result acceptance follow the source-data lifecycle design.

Local matching uses the current whole editor value as query. Strip/collapse
ASCII whitespace and apply Unicode full default case folding to both query and
the selected plain-text keys, then compare codepoint sequences. Pin the shared
Unicode data version in the native implementation and capability identity so
worker/fallback/SSR agree. This adopts
[Unicode default case folding](https://www.unicode.org/reports/tr18/#Default_Loose_Matches)
for search, not locale-sensitive sorting, accent removal, fuzzy matching or
value normalization. It does not change stored strings or field equality.
No additional Unicode normalization is implicit. Empty query matches all
admitted roots; matching both label and value tests either key, without joining
them into a synthetic string. Preserve source/group order; there is no ranking.

The implemented native adapter pins full default folding to Unicode 17.0.0,
using the common and full mappings from the licensed
[Unicode folding data](../packages/cem_ql/src/suggestions/CaseFolding-17.0.0.txt).
The consumer identity includes that version.

Hidden candidates do not display. Disabled candidates may display with disabled
semantics but cannot become active or commit. A group is hidden when no direct
option remains displayed after source visibility and matching. Group labels do
not make otherwise unmatched options match. Clearing/replacing results clears
any active reference before its row is hidden, removed or becomes ineligible;
an otherwise valid active source handle may survive a filter refresh. It never
causes a field-value change or a new automatic commit.

Supply a capability-owned native presentation view through
`datadom.slices.suggestions`, reserved against authored slice writes. This needs
an explicit shared native-binding extension to the current scalar behavior
context. The view contains scalar query/revision/expanded state and derived
native group/row nodes with label, stored value, match/availability flags and
native edges back to the original sources/content. A row's `source` is its
original candidate identity; it is not a DOM target. Active/committed source
handles remain distinct from the current produced row placement.

| Native view element | Scalar attributes | Native relationships/structure |
| --- | --- | --- |
| `suggestions` root | `query`, `query-revision`, `expanded`, `eligible-count`, `unicode-version` | Source-order group and ungrouped option children. |
| `group` | `label`, `disabled`, `hidden` | `source` identifies the original group; children are its option view nodes. |
| `option` | `label`, `value`, `matched`, `disabled`, `hidden`, `active`, `committed` | `source` identifies the original candidate; `content` retains its admitted child content sequence. |

These elements belong to the explicitly versioned consumer view schema;
source edges preserve their original schemas/scopes. Active and committed flags
describe distinct session states. The [interaction policy](cem-suggestions-interaction-design.md#preview-commit-and-opening)
assigns `aria-selected` to the active preview, independently of committed identity.
Custom label templates receive the corresponding native row/group, not a
JavaScript object or the live editor. Queries use native children/attributes
and explicit source edges, without object-shaped shorthand. No context root ID
or executable selector string is introduced.

This view is derived presentation data, not a replacement AST or a shared target
list on the authored reference. Independent sessions over one source have
independent views and bindings. Live active/open/lease state is transient and
must be rebuilt on resume; durable field state remains with its existing owner.
Do not serialize that view as authority or persist browser handles in an island.

For ordinary filtering, retain the complete source-order row sequence and hide
unmatched rows/groups. Do not build a shortened positional loop whose IDs could
move to other source nodes. Retain one row placement per source identity in the
session. Source insertion, deletion or reorder requires explicit re-admission
of row mappings, and clearing active claims before any positional reuse. The
runtime's current positional rendering is not evidence of keyed source identity;
the implementation task must supply this consumer mapping and verification.
Each produced row is a distinct native output element. Its placement ID, rather
than an ID on the inert source option, is the active-descendant target.

## Accessibility and lifecycle handoff

The provider claims `role="combobox"`, `aria-controls`, `aria-expanded`,
`aria-haspopup="listbox"` and the mode's `aria-autocomplete` on the actual editor.
`aria-controls` identifies the exact listbox even while collapsed; actual native
visibility determines expanded state. Claim `aria-activedescendant` only for a
current, visible, eligible row within that controlled listbox while the editor
has focus. Reveal that row within the popup's scroll region without scrolling
the page or moving input focus. These relationships follow the
[combobox/active-descendant contract](https://www.w3.org/TR/wai-aria-1.2/#combobox).
They do not require `aria-owns` or DOM reparenting.

Preserve the field's accessible name, help/error links and native attributes.
Existing incompatible role/autocomplete/active-descendant owners conflict;
unrelated controls tokens remain intact. The lease tracks only its own tokens
and writes, releasing them without restoring older values over new authored
state. Group labels name their own groups. The host itself has no combobox/menu
role. Option state belongs to the session, not authored `selected` flags;
the interaction design specifies preview/selection presentation.

Prepare sources, filtering, label templates and native row mappings in a bounded
consumer lifecycle stage with the original bindings and scope policies. Apply
the same limits across reference slots; do not evaluate an unavailable source
or fetch data during a commit key event. Before activation/publication, recheck
query, editor, source and placement revisions together. Pending replacement may
retain a still-valid committed plan under the existing publication contract,
but stale query candidates cannot commit. Authority loss invalidates claims
immediately. Dismissal/rebinding invalidates old opening generations.

Opening/navigation/dismissal never own a value write. Only the suggestions
session requests a synchronous field-provider commit after checking current
source and placement eligibility; the field updates value/submission/validity
and closes the popup before value-event observers. Keyboard/pointer/IME
arbitration, event/history reconciliation and constrained validity follow the
interaction design. Loading/status announcements follow the source-data lifecycle design.

## Alternatives and delivery

| Alternative | Decision and tradeoff |
| --- | --- |
| Add suggestions behavior separately to every field declaration | One capability plus provider contract avoids duplicated editing/form implementations; the optional composer does not become a field. |
| One implicit nearby input, or slot plus external endpoint | Require one exclusive endpoint; ambiguous or invalid bindings fail predictably. |
| Upstream filtering for every source | Retain an explicit external mode; a bounded local default covers the audited static/grouped intent without application UI handlers. |
| Filter every upstream result again | Reject implicit double filtering; query-dependent results use external mode. |
| Whole menu/surface substitution | Keep label templates inside owned shells for the first profile; richer adoption needs separate semantic/visibility/authority evidence. |
| Existing choice/autocomplete record normalization | Implement retained native adapters; copied records and value-as-identity would break the selected source and reference contracts. |

Public XHTML composition still requires the adopted local authorization adapter
and source/readiness/label wiring. Native-render row-to-source metadata is delivered. The shared editor-for,
provider, native source/view, placement and capability prerequisites are delivered.
Complete the canonical component, colocated stories, catalog/slot documentation and
source/installed galleries. The [active actions](todo.md#autocomplete-and-suggestions-design-for-cem-inputs)
record those gaps and adjacent verification scenarios. The public suggestions
declaration and complete acceptance matrix remain open. The shared controller
and declarative `suggestions` capability are implemented through a trusted host
preparation hook; they do not infer source-to-shell authority from markup.

Native consumer-frame publication now retains the live view and original source
edges directly, rejecting authored `suggestions` aliases or slice values before
binding. The provider's package-private attribute lease checks current editor
revision, exact unique endpoints, native visibility and focused eligible rows.
It applies scalar claims atomically, merges only its own controls token, and
releases without overwriting newer authored relationships. Render reconciliation
preserves active provider claims that the declaration does not replace. The
manual-listbox delegate can share that exact verified provider lease.

These APIs require a trusted current-publication hook; endpoint pointers and
IDs cannot establish original source-to-row placement authority. Cross-component
outbound publication and exact host-issued grants in both directions are
implemented. The placement coordinator admits opaque native row objects,
committed editor/listbox/row endpoints and property-specific grants. It checks
all relationships before reserving omitted endpoint IDs; IDs never issue grants.
Revocation, conflicting endpoints and disconnected targets expire admissions.
The controller activates provider attributes and commits only through a current
admission. Original source objects survive query filtering independently of
publication rows; releasing the source revokes commit proofs.

The current CEMV channel rejects live views containing native source edges. Live
publications now stay on their original processing owner. An immutable publication
admits transient consumer leases whose render jobs route there with independently
captured scalar control frames. Consumers do not rebuild the native view or source
owners. Consumer and publication revision checks run before and after processing;
release or worker loss cannot revive an old lease. Retained views, control bytes
and retired identities share bounded session capacity. They are excluded from
serialized resume authority. The processing protocol is versioned for these jobs.

The host supplies each consumer's current-admission hook. Canonical component
frames now acquire outbound leases through the runtime's `nativeSuggestionsInputs`
hook and route ordinary compile/render/diff execution to the original publication
owner. They retain the consuming declaration's module/function context, native
materialized inputs, CSS/DOM publication and source-map metadata. Compilation
bindings remain fixed for the declaration, as on the ordinary retained path.
The processing protocol is v21. A second live binding is rejected; execution
across multiple live owners needs a separate design. Retained HTTP document handles
belonging to another owner cannot ride this channel; supply materialized inputs.
These leases are excluded from snapshots and reacquired on subsequent frames.

The separate host-owned placement coordinator establishes DOM placement grants
in both directions and integrates them with provider claims. The declarative
capability requires `suggestionsControllerInputs` from its runtime host; a missing
hook, invalid endpoint or copied/resumed handle leaves it inactive with a diagnostic.
The hook supplies bounded native preparation and exact committed shells. It is
transient and cannot be reconstructed from a data island. Automatic mapping from
native rendering to constructed row shells is implemented. A declaration annotates
its owned option shell with `@suggestion-row={#row}`, where `row` is the exact
current native view row (a direct row value is also accepted). The consumer strips
that attribute from output and returns a bounded source-handle/placement map.
The binding accepts only its exact render result and preserves native source order.
`runtime.renderedSuggestionsFor(instance)` exposes the captured committed elements
and opaque rows with a current-admission check; it supplies no grants by itself.
A newer render attempt, disconnect or native owner loss expires that mapping.
Production composition still requires implementation of the adopted
[local host authorization](#local-host-authorization) policy in the active checklist. `editor-for` now has native singleton
projection, a property-specific placement grant, and an exact-provider endpoint
adapter. The adapter admits typed endpoints, scoped local names or a single direct
editor slot, rejects ambiguous/conflicting/unsupported editors, and grants no
authority from lookup alone.

Source sessions retain source revision and owner identity across queries. Each
immutable publication captures its query revision and its own current-eligibility
hook; consuming leases additionally capture the receiving instance revision.
Superseding a query invalidates its publication and consumer results without
reimporting sources or releasing unrelated publications. Source replacement or
owner loss invalidates the entire session. Query revisions do not grant authority
and need not be globally ordered across independent consumers.
