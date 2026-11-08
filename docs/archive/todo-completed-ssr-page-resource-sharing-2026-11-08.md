# Native SSR page resource sharing — 2026-11-08

This increment extends the native parent-first worklist with explicit page-wide
resource coordination. It reuses the shared stylesheet coordinator, importer,
module resolution, managed emitter, canonical island and native application core.
No transport, component/application workaround, JSON tree handoff or HTML reparse
is added. The passive page API retains its existing admission boundary.

The first participant closes a declaration's CSS through its original operation
and explicit resolver authority. The page shares those exact retained records
through a lease bound to the compiled native declaration owner. Later siblings
and nested instances neither refetch nor republish declaration CSS. Private and
named shared scopes keep their ownership contracts. Instance CSS retains separate
owners, identities and resource tickets. Existing ranges and style suffixes
remap through every subsequent native application.

Pending tickets stay opaque, ordered and stable. Foreign, consumed and previous
participant tickets cannot replace or retire current work. Aggregate roots,
import/URL references and delivered bytes use the page's remaining allowance
across every participant. Import validation and native failure locations remain
shared. Cancellation, transport/import failure, exhaustion and observed resolver
revision changes retire the staged page without publishing intermediate output.
Restoring an old resolver fingerprint cannot revive refused work.

Original, copied and projected CEM-ML source envelopes remain literal and
non-executing. Format handling stays in the shared importer. Authored mapping
keeps its original event path; projected source and instance CSS keep conservative
producer maps and no invented authored event/line coordinates. A bounded weak
source pool shares parsed views only for identical native producer ownership,
text, maps, metadata and compatible limits within the original operation. Each
host retains a distinct canonical/source lease. Identical text from different
origins does not share a parsed view; cache hits recheck operation, limits and the
actual source type. Weak entries avoid cycles through applied page ownership.

Twelve native cases cover declaration reuse and single publication, scoped and
nested membership, independent instance tickets/styles, copied/projected source
leases, shared parsed views with distinct canonical owners, source-origin and
limit/type separation, late-participant aggregate exhaustion, foreign/stale
request refusal, transport failure maps, cancellation, resolver revision latching
and final native owner retirement. The lifetime fixture follows retained stylesheet
handles as well as the final page tree; an expired emission lease does not free
source memory while a caller still owns its native handle.

The admitted profile remains one body registry of autonomous inline declarations.
Modules/scoped registries stay refused. Quiescence remains `Partial(Incomplete)`;
precise CPU and final declaration-version, complete-island, identity/policy and
owned-range proof precede CLI/export and browser hydration adoption.

Exact source/artifact hashes and gate results are in the
[verification receipt](ssr-page-resource-sharing-2026-11-08.json).

- [x] Expanded source-admission gate: **284 native checks** across default and
      actual stripped profiles, including **12/12** page resource cases,
      **11/11** passive-page cases and **14/14** single-participant CSS cases
      per profile.
- [x] Standalone stripped SSR suite **109/109**; component receiver **17/17**;
      shared receiver units **19/19 per profile**.
- [x] Existing browser slot profiles **4/4**, host/island publication profiles
      **4/4**, complete single-participant observations **19/19** and passive
      page observations **3/3** match, with no divergences or external requests.
      These are regressions; the native resource API does not enable browser
      SSR adoption.
- [x] Paired runtime packaging, Clippy with existing warnings, focused Rustfmt
      and optimized WASM template/CSS consumers pass.
- [x] Optimized core: **23,996,553 bytes**, SHA-256
      `8f9e8475586e3b432870e0dd9e8fab62b4d774694a8e0827c3e56c91344e3136`,
      below the 26,214,400-byte ceiling, with **28 imports/175 exports**.
      Packaged core matches; named JS/WASM ABI and default features are unchanged.
- [x] Frozen presentation SHA-256 remains
      `bbcb37da0a36491e5c021a5cf0c67304cc41f001a98247fc5c101ed6c45a9864`.
