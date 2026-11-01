# Imported/native-rule and custom-profile transport — 2026-11-01

- [x] Add complete custom result-profile source transport with fresh native
  reconstruction, independent pins, original shared arenas, canonical framing,
  malformed/truncated/oversized data, cancellation and nested bounds.
- [x] Capture actual imported query/native bindings with their original profile
  and code owners. Relink against fresh native consumer fields/scopes; refuse
  foreign profiles, unavailable/foreign/dropped code and stale owner authority.
- [x] Exercise separate shared rule/profile byte endpoints in actual independent
  WASM heaps, workers and owned-fallback handoff. Preserve original tokens,
  complete census, sealed journals, byte budgets and credential refusal.

## Delivered behavior

`CEMRP1` carries complete original profile graphs, lexical captures, identities,
schema selectors and namespace bindings. Verification restores fresh native CEM
arenas and compiles shared value contracts without a source parser or implicit
schema catalog. Identical repeated source claims share one restored arena;
conflicting claims refuse. Independent packet hashes and the existing semantic
profile fingerprint must match. The profile ceiling is 16 sources, 16,384
aggregate nodes checked before restoration, 4 MiB of encoded data, depth 64 and
one million verification checks, with original source/range/lexical limits.

`CEMRH1` separately frames unchanged CEMQD1/CEMNR1 bytes and their CEMRP1 profile.
It retains exact optional local datatype selectors and direct/imported origin.
Imported origin must name the exact leaf artifact and contains no datatype
selector. Shared-URI leaf/profile claims must agree completely. Existing formats
and deployment sidecars retain their previous semantics. No callback, context,
consumer expression result, scope grant, evaluated value or request budget is
serialized. The whole packet is bounded to 4 MiB; metadata remains 4 KiB and leaf
limits remain 64 KiB.

`FreshNativeRuleCodeOwner` holds actual Rust validator code, its exact identity,
control and actual generation, with a counted Registry lease. Host installations
borrow the original owner weakly and compare its pointer as well as its declared
identity. Equal-name/hash callbacks cannot substitute the installed owner.
Missing, foreign, retired, dropped or cancelled owners fence capture/execution;
callback retirement after producing success also refuses publication. Native
signatures/serde and validator host adapters are exposed only through the opt-in
query-source runtime on WASM; default builds remain unchanged.

Imported `QueryProfile`/`NativeProfile` registrations borrow actual profile/code
owners. Legacy registrations remain available but cannot invent transport proof
from matching QNames. Capture checks the actual live module/source/behavior and
original owner receipts before/after export. Fresh CEMDP1 linking compares exact
leaf pins and uses its actual original consumer arenas, scopes and reference
resolution. It still performs ordinary crossing, kind and dependency checks.
Native tests validate original fresh consumer node identity and refuse subsequent
capture and consumer publication after the original profile owner drops.

`RuleSourceEndpoint` and its Rust-only installed WASM adapter reimport complete
profiles and leaves on fresh heaps. Native code remains an independently supplied
original owner. The three explicit `ruleSource*V1` byte operations use original
opaque object tokens. Direct local datatypes validate through the existing native
engine and cumulative invocation budget. Imported leaves without a local consumer
remain guarded, recapturable registries; standalone evaluation refuses. Complete
WASM consumer/receiver installation stays in the CEMAPPD4 workstream.

Shared JS only forwards bounded bytes/control pins and checks original physical
identity. `createRuleSourceWasmBackend` selects the explicit native rule ABI.
Trusted endpoint limits may reach 4 MiB; legacy queries retain their 69,643-byte
default. Each worker checks its own backend's limit. Packet/pin bytes remain
charged alongside checkpoints in the unchanged aggregate 4 MiB recovery ceiling.
Native-generated rule packets of 74,669–101,448 bytes exercise the larger path;
unit protocol doubles separately cover invalid limits and 100,000-byte copying.
No Studio-specific or component UI JavaScript is added.

Deployment/code/profile/source/registry/invocation owners retain their actual
native ledger roles. Retired owners stay counted until actual drop. Original
cleanup and deployment uninstall work behind closed admission, followed by full
36-registry v2 census/freeze and actual physical retirement. Fresh targets require
new original bootstrap owners and atomic shared binding publication. Old aliases
remain retired; unknown external journal outcomes require reconciliation.

Entered passwords/OTPs still block recovery until explicitly cleared. Automatic
Studio recovery remains off; five-second recovery deadlines and 4 MiB checkpoints
are unchanged. Imported leaf checkpoint islands restore no consumer DOM/UI.
Source/code fingerprints are agreement, not linked-binary omission evidence.
Full CEMAPPD4 consumer transport, generated app bootstraps and reduced live-control
admission remain in the [active TODO](../todo.md#component-application-wasm-profile-separate-target).

See the [contract](../cem-native-schema-capsules.md#imported-rule-and-custom-profile-transport),
[profile codec](../../packages/cem_ql/src/query_validator_capsule/profiles/transport.rs),
[rule codec](../../packages/cem_ql/src/rule_source_transport.rs),
[endpoint](../../packages/cem_ql/src/rule_source_endpoint.rs),
[code owner](../../packages/cem_ql/src/structural_receiver/native/rule_owner.rs),
[native imported cases](../../packages/cem_ql/tests/datatype_dependency_capsule/rules.rs)
and [physical handoff](../../packages/cem_ql/tests/query-resource-wasm.mjs).

## Verification

- The profile codec failed its missing-API test before implementation. All 310
  selected native cases passed: 291 query/registration/datatype/envelope/structural
  cases (seven new cases) plus 19 inventory/admission/generation cases. Focused
  profile/imported cases passed again after tightening pre-restoration sizing.
  Default, component baseline and opt-in WASM checks passed.
- Both actual WASM targets rebuilt and passed: existing query-source coverage plus
  custom two-source profiles and direct/imported query/native leaves in independent
  heaps and a real worker. Tests cover original/copied/foreign tokens, changed
  pins, code retirement, input canaries, source recapture and native roots.
- All 40 physical handoff scenarios passed through the exact shared group/store:
  five variants × worker/fallback × resume/stale-checkpoint/late-credential/
  foreign-target cases, with complete v2 census and native-sealed journals.
  Imported leaves assert standalone refusal and retain no invented consumer.
- All 44 Node lifecycle/resource/audit/factory cases and existing actual owned-
  generation/memory-budget WASM drivers passed. The shared package build and
  clean import verification passed (374 files, 24,139,260 default WASM bytes).
  Runtime JS/types were refreshed without rebuilding unchanged default WASM.
  The 59-component/24-capability source audit was refreshed and checked.
- All 24 selected browser checks passed: four shared retirement/ownership/resume
  cases, 18 component rendering/generation/attribute cases, and complete Studio
  Login recovery in worker/fallback modes (six Studio cases filtered). Native
  compilation and metadata finished before isolated browser acceptance. Saved
  projects and drafts were not cleared; entered-credential refusal remains covered.
- Focused Rust formatting, Node syntax, diff checks and all 366 local documentation
  targets passed. Completed fixture work is archived, with no completed checkboxes
  in the active TODO. Generated screenshots, Vitest attachments, private WASM
  probes and build output remain excluded from the commit.
