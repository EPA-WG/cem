# Native SSR readiness proof — 2026-11-08

This increment adds an opaque native `ReadyInlinePage` for the bounded inline
SSR profile. A settled candidate requires a complete current execution/policy
context, supported protocol tuple, canonical declaration versions, complete
islands, live source/CSS leases, exact owned ranges and measured native CPU.
Generic page artifacts keep their partial state and disabled hydration flag;
CLI/export and browser adoption remain separate integration work.

Native islands carry declarationVersion independently of host-owned version
input. Exact SemVer validation rejects malformed author declarations and the
compatibility predicate follows browser caret/prerelease/build rules. A missing
version cannot prove ready. The legacy preparation fixture now uses 1.0.0 so its
existing provenance and refusal checks still reach their intended boundary.

Proof checks use retained native owners and the shared island validator,
projection, render plan and stylesheet leases. Canonical input never comes from
rendered output. Descendant commits are validated separately while each parent
keeps its produced node identities and expected source subtree. Declaration
styles, instance suffixes, attributes, islands, markers, active inventory, unique
IDs and resource/byte/depth totals are checked against actual native evidence.
Replaced trees, opaque or stale context, cancellation and resolver revisions
refuse fresh observations. Fingerprint restoration cannot revive a revoked lease.

The original operation now owns a cumulative CPU ledger. Synchronous phases use
OS thread CPU counters, separate from the active wall deadline; transport wait
and caller work outside admitted phases are excluded. Totals survive new handles
and sequential threads. Guard overlap, foreign controls, incomplete guards,
missing clocks and exhausted budgets refuse receipts. Linux counters are tested;
Windows GetThreadTimes bindings are implemented but not executed on this Linux
host. Other targets, including browser WASM, cannot issue readiness without an
available native CPU clock. CPU resolution follows the operating system, with
aggregate rounding up only at the millisecond observation boundary.

Four CPU cases, seven readiness integration cases and three focused version,
literal-island and range-witness cases cover the new contract. Resource waiting
longer than the CPU allowance succeeds under the separate wall allowance;
incompatible/missing/malformed versions, unsupported protocols, duplicate IDs,
changed contexts, copied trees, stale resolver grants and tampered/grafted content
fail closed. Existing page, resource, receiver and browser gates remain regression
coverage; the new native proof does not enable browser SSR adoption.

Exact source/artifact hashes and gate results are in the
[verification receipt](ssr-readiness-proof-2026-11-08.json).

- [x] Expanded source-admission gate: **312 native checks** across default and
      actual stripped profiles, including **4/4** CPU cases, **7/7** readiness
      integration cases and **3/3** focused version/island/range cases per profile.
- [x] Standalone stripped SSR suite **116/116**; component receiver **17/17**;
      shared receiver units **19/19 per profile**.
- [x] Existing browser slot profiles **4/4**, host/island publication profiles
      **4/4**, complete single-participant observations **19/19** and passive
      page observations **3/3** match, with no divergences or external requests.
- [x] Paired runtime packaging, Clippy with existing warnings, focused Rustfmt
      and optimized WASM template/CSS consumers pass.
- [x] Optimized core: **23,967,363 bytes**, SHA-256
      `57897af7ffc4a8ed07ea774ea36ff01a30a169c598dfda06e6500005b2fad902`,
      below the 26,214,400-byte ceiling, with **28 imports/175 exports**.
      Packaged core matches; named JS/WASM ABI and default features are unchanged.
- [x] Frozen presentation SHA-256 remains
      `bbcb37da0a36491e5c021a5cf0c67304cc41f001a98247fc5c101ed6c45a9864`.
