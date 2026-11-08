# SSR foreign payload and CSS closure admission — 2026-11-08

This increment admits passive SVG/MathML payloads and resource-free instance CSS
for the bounded one-inline-DCE native SSR profile. SSR remains
`Partial(Incomplete)` and cannot enable hydration adoption.

Foreign content enters through the original native HTML owner. The shared import
boundary now applies HTML's case-sensitive foreign-name adjustments, including
SVG `linearGradient`/`viewBox` and MathML `definitionURL`. The shared semantic
profile preserves expanded names and permits a bounded passive element set;
unknown namespaces, active elements and external resources remain refused.
XML language/space attributes and local XLink fragments retain namespace identity.
SVG title/description and MathML text containers require markup-free text/comments.
That guard also refuses markup-shaped RCDATA from the current HTML tokenizer;
foreign HTML integration is deferred rather than interpreted differently by SSR.

Explicit inert HTML payload templates can supply plain style blocks. CSS parsing
and original-byte mapping stay in `cem_ml::import`; downstream consumers receive
retained typed trees. The native CSS source owner holds the original HTML tree
and the original operation's memory permit. Blocks retain authored maps and
source-selection identity, with bounded bytes, values and depth. This initial
mapping profile refuses carriage-return normalization and style attributes.
Nested HTML templates remain inert and are not scanned for active CSS.

The adapter uses the existing retained instance stylesheet owner, resource
closure, identity derivation and managed emitter. The profile grants no resolver
context or transport authority. Imports, URL references and unsupported managed
syntax fail admission. Ready styles use native scope boundaries and owned
keyframe names. Page resource ceilings bound occurrence count, capped at 64.

Canonical islands keep authored CSS as inert source. Admitted source styles are
removed from projected slots, and managed styles form a host-local suffix after
`cem-render-end`. One native application publishes that suffix with canonical
island state and host attributes. The applied tree retains the owner; releasing
its final owner invalidates outstanding emission handles. Original-operation
checks, cancellation and lowered memory/stack limits remain effective.

Native fixtures were added before extending admission. They cover expanded
names/casing, namespace-qualified attributes, source maps and original-owner
identity, slot projection, suffix placement, owned keyframes, inert nested
styles, sensitive/resource/active-profile refusal, original-operation replay,
cancellation, bounds and retained-owner cleanup. Shared observations cover
foreign payloads and complete CSS source/render/suffix parity. A browser CSS
probe checks that the emitted rule applies inside its host and stays local.

The CSS browser case deliberately forces main-thread processing and asserts its
single expected worker-startup fallback diagnostic. Other diagnostics remain
strictly checked.

- [x] Source-admission gate: **200 native checks** across default and actual
      stripped profiles, including **10/10** new domain cases per profile and
      **13/13** retained stylesheet regressions per profile.
- [x] Shared island receiver units: **19/19 per profile**; native component
      receiver regressions: **17/17**.
- [x] Browser slot profiles **4/4**, publication profiles **4/4**, and **19/19**
      complete native/browser observations with no divergences or external
      requests. Managed CSS application and host locality probe passes.
- [x] Paired production build, Clippy (existing warnings), focused Rustfmt and
      browser-script syntax checks, optimized WASM template/CSS consumers.
- [x] Optimized default core: **23,969,473 bytes**, SHA-256
      `72c465c592e54bf31aff761dd65d3a3f675d4ffbe75798e5a87db2429a47400a`;
      below 26,214,400 bytes, with **28 imports/175 exports**. Packaged core
      matches; no named ABI or default feature change.
- [x] Frozen presentation SHA-256 remains
      `bbcb37da0a36491e5c021a5cf0c67304cc41f001a98247fc5c101ed6c45a9864`.

Exact source/artifact hashes and named JSON test observations are in the
[verification receipt](ssr-foreign-css-admission-2026-11-08.json).

The next generic topic is coordination: declaration CSS and resource-bearing
closures, explicit resolver/loader authority, multiple participants and bounded
nested settling. Completion proof must then validate versions, islands,
identity/policy and owned output ranges before CLI/export or hydration adoption.
