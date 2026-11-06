# Retained native browser slot input — 2026-11-06

Native browser rendering now supplies retained native nodes to `datadom.slots`.
The earlier named-slot mismatch is closed: all 14 shared native/browser
observations match in full, with no documented divergences. Ordinary record
insertion still produces `cem.ql.render.expression_type`.

`cem_ml::import::dom::DomCapture` maps primitive observations of actual DOM nodes
to a typed retained CEM tree. JavaScript walks the existing canonical payload,
including nested template contents, without creating a JSON DOM, serializing
HTML or implementing a CEM value encoder. The native artifact selects the real
payload container and retains its children, source parents and source keys.
Runtime `cem-dom://` provenance has unknown authored ranges/lines; browser output
uses `dom-canonical` fidelity. The original inert DOM nodes stay in their island.

The importer admits decoded text, comments, namespaced elements and attributes
under byte/node/depth limits, rejects duplicate attributes and invalid parents,
and poisons failed captures. Browser and SSR use the same ordered native slot
index; SSR still runs its original stricter closure/domain admission first.
Native authoritative attribute sequences supply slot names. The browser omits
legacy `datadom.slots` metadata from native rendering data, then binds opaque
CEMV through the existing native-value channel. Existing legacy payload/snapshot
codecs and DOM-template lowering remain separate compatibility paths.

Four additive WASM exports manage capture sessions. Successful finish, failed
append and explicit abandonment release their native owners/workspace permits.
Open sessions participate in the existing `native-values` retirement census;
there is no new inventory manifest/profile. The adapter also disposes temporary
value imports after rendering. Captured payload input is transient and is not
saved by the named hydration or edge snapshot export codecs. Payload mutation
recaptures current input under the existing render/queue guards. Projected
payload styles keep their existing stylesheet owner, while nested template
styles remain inert.

A browser run using the previous built adapter reproduced the original named
slot `expression_type` diagnostic. Native tests establish opaque transport,
owner release, repeated insertion, literal decoded values, source keys and
parents, comments/default-slot indexing, and continued record refusal. The new
browser fixture verifies source DOM identity, repeated named slots, mutation and
slot reassignment, inert templates, CSS ownership, source fidelity, lowered
capture bounds, abandonment and failed-capture cleanup. Forced fallback profiles
allow only their expected startup warning; actual worker execution must dispatch
without fallback. Worker-loss replay also forwards the opaque native payload.

`yarn nx run cem-elements:test:native-slots` depends on the paired build and runs
native capture/slot tests, native observation export, actual stripped slot tests,
focused processing units, four slot profiles, four existing host/island
publication profiles and shared native/browser observations. With an already
prepared paired build, `--excludeTaskDependencies --skip-nx-cache` runs the same
checks without rebuilding dependencies.

- [x] Native DOM capture: **4/4**, in default and actual stripped profiles.
- [x] Native slot transport/record refusal: **2/2**, default and stripped.
- [x] Default SSR render regressions: **17/17**; actual stripped SSR suite:
      **58/58** (2 slot, 11 host commit, 15 capture, 13 preparation, 17 render).
- [x] Processing/worker-loss replay units: **87/87**.
- [x] Browser native-slot profiles: **4/4**; previous publication profiles:
      **4/4**; shared native/browser observations: **14/14 complete matches**.
- [x] Production package build, focused ESLint, native Clippy, focused Rust
      formatting and existing optimized WASM template/CSS consumers pass.
- [x] Optimized default WASM: **24,219,668 bytes**, below **26,214,400**;
      SHA-256 `2add1f853d95c49968d44821db166ff6d1c22b0a2a2d997376f6abdd3f104a49`.
      Optimization preserves all named descriptors: 28 imports, 175 exports,
      including the four new capture functions. Browser/core assets are paired.
- [x] Frozen presentation checksum remains
      `bbcb37da0a36491e5c021a5cf0c67304cc41f001a98247fc5c101ed6c45a9864`.

Exact source and artifact hashes and browser observations are recorded in the
[verification receipt](browser-native-slots-2026-11-06.json).

SSR remains `Partial(Incomplete)`. This increment does not admit additional SSR
source/foreign/CSS domains, prove nested/resource settling, enable adoption,
change default features or publish a deployment. Next: shared source/domain and
stylesheet admission, bounded coordination, then native completion proof before
CLI/export and hydration adoption.
