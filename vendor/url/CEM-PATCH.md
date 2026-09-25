# CEM-QL URL compatibility patch

This is a maintained, scoped copy of the published Rust `url` 2.5.8 crate.
The private Cargo package is named `cem-url` (library name `url`) to avoid
Nx conflating it with the registry package. Only CEM-QL selects it through an explicit path dependency. There is no global
`[patch.crates-io]`; CEM-ML and other registry consumers keep the published
crate. CEM-QL's `url_unpatched` dev dependency retains the original candidate
baselines. This directory is an excluded, independent Cargo workspace.

## Provenance and review surface

- [Published crate](https://crates.io/crates/url/2.5.8)
- Crate archive SHA-256: `ff67a8a4397373c3ef660812acab3268222035010ab8680ec4215f38ba3d0eed`
- Upstream source revision: `d6ea13c5f8e7e6e627f6390161b3e185bda5e5ce`, path `url`.
- Original MIT/Apache licenses, source, README, original manifest and tests are
  retained. WPT-derived fixtures also retain the [WPT license](tests/LICENSE-WPT.md).
- [CEM-PARSER.patch](CEM-PARSER.patch) is the complete runtime source delta.
  It covers file/opaque parsing in `parser.rs` and scheme transitions in `lib.rs`;
  no other upstream runtime source file changes.
- `Cargo.toml` renames the package to private `cem-url` (`publish = false`)
  and gains an independent workspace declaration. The local
  `Cargo.lock` pins standalone dependency-test resolution.
- `tests/expected_failures.txt` removes exactly the 23 now-passing cases listed
  in [CEM-FIXED-CASES.txt](CEM-FIXED-CASES.txt). The original file-path fix did
  not change expected outputs. The later opaque-space refresh below updates
  eight historical cases from the newer pinned WPT expectations.
  This is an upstream historical baseline, distinct from CEM's newer pinned WPT
  fixtures; a green historical harness does not establish full conformance.

## Behavior

Preserve leading empty file path segments instead of unconditionally trimming
them. Route empty/localhost file authorities through path-start and explicitly
consume one initial path separator. This retains ordinary `file:///` roots
while preserving additional separators and enabling first-segment drive-letter
normalization. Removing the trim alone is insufficient: upstream `issue_197`
and CEM's href round-trip checks detect the resulting doubled-root regression.
Those tests pass with the complete patch.

CEM's setter cases pathname[21–23] and parse case 138 now pass. The original
registry candidate evidence stays unchanged. File host/drive loss (parse 664),
empty-punycode labels and the remaining setter gaps are still recorded.

## Validation and maintenance

```sh
cargo test --locked --manifest-path vendor/url/Cargo.toml --test unit --test url_wpt --target-dir /tmp/cem-url-vendor-target
cargo test --locked -p cem-ql --test url_setters --test url_setter_candidate --test url_parse --test url_origin --test url_params --test url_parse_candidate --target-dir dist/target/cem_ql
```

All 66 dependency unit tests, the historical WPT harness (with the documented
fixture refresh), and 34 focused CEM URL tests pass. The CEM matrix
matches 48/51 parse cases and 271/277 component setter cases; remaining exact
baselines remain asserted. No shared evaluator/type or CEM-ML source changed.

When upgrading, compare this patch against upstream, rerun both dependency and
CEM fixture sets, and remove the path override when a release preserves these
behaviors. Do not repurpose this fork for unrelated URL changes or distribute
it as an unmodified crates.io release. CEM-QL publication must account for the
local patch: Cargo packages resolve path+version dependencies from the registry,
so the private `cem-url` package has no registry release. Resolve that
packaging boundary before publishing; workspace/native/WASM builds use the
patched path today.

The CEM-QL Nx WASM build passes with the private parser dependency. This is
compilation verification; registered query execution parity remains pending.
No full workspace/package test suite or global task ran.

## Opaque boundary spaces and protocol transitions

The follow-up patch encodes a SPACE as `%20` when its next preprocessed code
point is `?` or `#` in an opaque path. Internal spaces, pre-encoded spaces,
ordinary trailing-input trimming and hierarchical paths keep their semantics.
Clearing query/fragment then preserves the encoded final space. This implements
[the opaque path state](https://url.spec.whatwg.org/#cannot-be-a-base-url-path-state)
at parsing time, not as an output repair after data has been lost.

The bundled historical setter fixtures expected the older trimming behavior.
Exactly search[10–13] and hash[16–19] now use the expectations from the unchanged
CEM-pinned WPT file at revision `c48d58747e1f211527fb695fd60548a997fae617` (same
case indices, href and new_value). Source SHA-256 remains documented in
[SETTERS.md](../../packages/cem_ql/fixtures/url/SETTERS.md). No CEM-pinned WPT
expectation changed, and no new expected-failure waiver was added. The two
assertions in the renamed `test_preserve_boundary_spaces_from_opaque_path`
unit test now assert `data:space  %20` after clearing three-space seeds.

The `set_scheme` guard now tests credentials/port rather than mere authority
presence for transitions to file. Equal file-scheme assignments succeed;
empty-host file URLs cannot transition to another scheme. This preserves
normalized/equal setter outcomes and permits eligible HTTP(S)-to-file updates.
CEM's separate host adapter retains partial hostname effects while reporting
invalid port subcomponents; no dependency host-parser patch was introduced.
