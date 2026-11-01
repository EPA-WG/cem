# Worker/fallback application checkpoint handoff — 2026-11-01

- [x] Add a native-authored application/bootstrap fixture over original source
  snapshots. WASM restores receiver, scalar, module, input and dataset trees
  without invoking a source parser. Native embedding code supplies scopes/grants.
- [x] Validate node/scalar receivers and an explicit native executable-root
  invocation before/after physical fresh-generation handoff. Unavailable raw
  typed receiver ingress remains gated by the existing typed-profile policy.
- [x] Add the application WASM backend to the existing shared byte transport;
  retain original Memory/Worker identity, opaque native tokens and byte limits.
- [x] Exercise original token/refusal, reentrance, expired owners, sealed journal,
  stale checkpoint, foreign target, late credential and aggregate-budget paths
  in real workers and owned fallback heaps.

`createApplicationSourceWasmBackend` reuses the shared source resource protocol
and native generation group. No application-local bridge or JavaScript AST is
introduced. The Rust-only library provisioning API remains unchanged; the test
embedding alone exposes fixture provisioning. It supplies fresh real bootstrap,
registry, compilation and receiver owners in every replacement heap.

The physical fixture has immutable native-authored input data owned by its
bootstrap, separate from source replay. Real application input restoration and
qualified rendering are the following reduced-ABI work. No external action is
replayed during bootstrap. The sealed acknowledged-operation journal survives;
unknown outcomes block handoff. Private source bytes/pins join the existing
4 MiB aggregate checkpoint budget, including the four-owner budget fixture.
Credentials block preparation and late binding; old/foreign targets refuse.

Verification: the native fixture, shared source protocol tests, 36 independent
WASM reader/ABI checks and all 12 physical application handoff scenarios passed.
The explicit `cem_ql:test:application-resource-wasm` target authors native data,
builds the full opt-in probe, generates the original owned factory and exact
checkpoint store, then runs acceptance. Raw probe binaries are not release assets.
Default component admission remains gated. Generated artifacts are excluded.
