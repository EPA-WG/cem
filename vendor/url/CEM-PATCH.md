# CEM-QL file-path preservation patch

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
  No other upstream source file changes.
- `Cargo.toml` renames the package to private `cem-url` (`publish = false`)
  and gains an independent workspace declaration. The local
  `Cargo.lock` pins standalone dependency-test resolution.
- `tests/expected_failures.txt` removes exactly the 23 now-passing cases listed
  in [CEM-FIXED-CASES.txt](CEM-FIXED-CASES.txt). No expected output was changed.
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
empty-punycode labels and the other setter gaps are not changed by this patch.

## Validation and maintenance

```sh
cargo test --locked --manifest-path vendor/url/Cargo.toml --test unit --test url_wpt --target-dir /tmp/cem-url-vendor-target
cargo test --locked -p cem-ql --test url_setters --test url_setter_candidate --test url_parse --test url_origin --test url_params --test url_parse_candidate --target-dir dist/target/cem_ql
```

All 66 upstream unit tests, the historical WPT harness (after removing only
fixed expected failures), and 31 focused CEM URL tests pass. The CEM matrix
matches 48/51 parse cases and 263/277 component setter cases; remaining exact
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
