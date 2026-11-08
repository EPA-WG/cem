# Native SSR stylesheet coordination — 2026-11-08

This increment coordinates declaration and instance CSS for the bounded
single-inline-DCE profile. It adds an explicit stylesheet preparation API while
preserving the ordinary resource-free admission path. Deferred stylesheet work
cannot render until the original operation has admitted a complete current batch.

The native coordinator uses existing retained stylesheet owners, shared CSS
import closures, module URL resolution and managed emission. Transport receives
one opaque native ticket and supplies response bytes, final URI and MIME.
Repeated pending observation is stable; foreign or duplicate delivery cannot
replace current work. Shared import validates redirect bases, MIME, integrity,
cycles and placement. Native import failure locations survive terminal cleanup.
The coordinator performs no implicit file/network reads or format parsing.

All roots and import/URL occurrences share one page resource ceiling. All
imported bytes share the page allowance remaining after the original source.
Workspace and retained source/arena memory use the original operation's permits;
resource sidecars preserve original parser owners, semantic identities and maps.
The original active clock is observed without renewal. This synchronous profile
conservatively uses the stricter wall/CPU cap as an elapsed bound, including
pending transport; that bound is not reported as measured CPU consumption.

A ready native batch binds its original tree, host, instance, operation and
resolver revision. Private CSS uses the produced tag. Shared CSS requires
matching declaration-owned membership. Native host/island staging accepts the
CSS context marker only through that closed proof; authored reserved writes
remain refused. An observed resolver change cannot be undone by restoring its
old fingerprint.

A shared native transaction now supports independent element edits with replace,
append or preserve modes. Duplicate and overlapping targets are refused.
Declaration CSS appends outside the original source template while preserving
its source IDs and child maps. Instance CSS remains outside the render range.
Host attributes, canonical island state and both CSS owners publish together.
Operation, elapsed allowance, resolver freshness and stylesheet liveness are
checked again after application. The applied tree retains all native leases.

Native fixtures were added before extending admission. The controlled import
cases verify parser ownership, maps, final-tree memory lifetime and lowered
bounds/cancellation. The atomic edit case verifies original child IDs and order,
duplicate/overlap refusal and cleanup. Fourteen coordinator cases cover imports
and redirect-relative URLs, pending/readiness gates, opaque request authority,
stale delivery, original-operation replay, MIME/integrity/transport failures,
aggregate bytes/resources, memory admission, scoped membership, reserved metadata
refusal, elapsed limits, resolver revisions and last-owner release. Testing found
a terminal declaration workspace retained by the prepared input; terminal
retirement now drops that input along with the CSS candidate owners.

- [x] Expanded source-admission gate: **234 native checks** across default and
      actual stripped profiles, including **14/14** coordinator cases, **2/2**
      controlled-import cases and **10/10** tree-application cases per profile.
- [x] Shared receiver units **19/19 per profile**; native component receiver
      **17/17**; standalone stripped SSR suite **86/86**.
- [x] Browser slot profiles **4/4**, host/island publication profiles **4/4**,
      and all **19/19 existing complete native/browser observations** match,
      with no divergences or external requests. These remain regressions;
      the new native coordinator does not enable browser SSR adoption.
- [x] Paired build, Clippy with existing warnings, focused Rustfmt and optimized
      WASM template/CSS consumer gates pass.
- [x] Optimized default core: **23,999,276 bytes**, SHA-256
      `c9dcb33944e395da70a9d4427e69a5521d49c7e8482800b86a47a729a42d6ba3`,
      below the 26,214,400-byte ceiling, with **28 imports/175 exports**.
      Packaged core matches; no named JS/WASM ABI or default feature change.
- [x] Frozen presentation SHA-256 remains
      `bbcb37da0a36491e5c021a5cf0c67304cc41f001a98247fc5c101ed6c45a9864`.

Exact source/artifact hashes and named JSON observations are in the
[verification receipt](ssr-stylesheet-coordination-2026-11-08.json).

CSS readiness still does not prove page completion. SSR remains
`Partial(Incomplete)` and hydration remains gated. The next topic is nested
lifecycle coordination under the original operation, followed by final version,
island, identity/policy and output-range proof, then CLI/export and browser
hydration wiring.
