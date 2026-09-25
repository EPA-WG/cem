# Maintained parser investigation (2026-09-25)

This isolated research crate reuses the 51 unchanged, licensed WPT cases in
`../wpt-url-parse.json`. JSON is explicitly a fixture input boundary. Neither
candidate is registered with CEM-QL. The nested workspace and lockfile keep
research dependencies out of production manifests and the root lockfile.

## Findings

| Candidate | Native selected cases | Browser-WASM evidence |
| --- | --- | --- |
| Published `url` 2.5.8 | 43/51 match (existing test) | Not newly tested alone |
| Upstream `url` at `00a6ce58d02f4e0d43c5ca0702c0bedb8b1ebf3a` | 44/51 match | Combined probe builds; execution gate blocked by imports |
| `ada-url` 4.0.0 | 51/51 match | Combined probe builds; execution gate blocked by imports |

Upstream fixes the file IPv6 host/drive case 664. Cases 138 (drive separator),
782/786/787/788 (blob origins), and 839/921 (empty punycode labels) still differ.
The native executable asserts the counts as characterization, not conformance.
The selected subset does not establish full WHATWG coverage, CEM-specific base
validation, setter behavior, integration parity, performance, or binary size.

Ada is a maintained Rust binding to bundled C++20. Its published source revision
is `af35063623f2c93bb337728d03c03644b47e75c9`. Its build script explicitly supports
`wasm32-unknown-unknown` using a WASI SDK plus upstream compatibility shims.
The upstream CI at that revision tests `wasm32-wasip1` with SDK 33; that is a
different target from the browser target required here.

An initial build failed because this machine had no `/opt/wasi-sdk`. Installing
SDK 34.0 **only in `/tmp`** made both debug and release builds succeed. The
strict no-host-import gate failed before executing the cases:

- Debug: `wasi_snapshot_preview1.clock_time_get`, `fd_prestat_get`,
  `fd_prestat_dir_name`.
- Release: `wasi_snapshot_preview1.clock_time_get`.

These are observations of the combined artifact, not attribution to a particular
library. No fake clock, WASI host, JavaScript URL fallback, or local dependency
patch was added. Browser-WASM semantic parity remains unproven. SDK 33 was not
probed; SDK selection/link behavior requires follow-up.

## Decision pending

Recommend pursuing Ada with a pinned C++/WASI toolchain **if that additional
build dependency is acceptable**. First isolate and remove the residual host
imports using supported upstream configuration, then execute the same vectors
in browser-compatible WASM and test CEM base validation and setters. Do not
adopt Ada in production until those gates pass. If the project requires a
Rust-only toolchain, the alternative is authorizing ownership of parser-level
fixes to `url`; an update to current upstream alone is insufficient.

This investigation does not authorize either production dependency strategy.

## Reproduce

From the repository root, with Rust's `wasm32-unknown-unknown` target installed:

```sh
cargo run --locked --manifest-path packages/cem_ql/fixtures/url/parser-probe/Cargo.toml --target-dir /tmp/cem-parser-probe-target
WASI_SDK=/tmp/wasi-sdk-34.0-x86_64-linux cargo build --locked --release --manifest-path packages/cem_ql/fixtures/url/parser-probe/Cargo.toml --lib --target wasm32-unknown-unknown --target-dir /tmp/cem-parser-probe-target
node packages/cem_ql/fixtures/url/parser-probe/check-wasm.mjs /tmp/cem-parser-probe-target/wasm32-unknown-unknown/release/cem_url_investigation.wasm
```

The final command is intentionally a failing adoption gate for the recorded
SDK 34 setup. Omit `--release` and use the `debug` artifact to reproduce the
three debug imports. The packed exported return value counts upstream failures
in the upper 16 bits and Ada failures in the lower 16 bits.

Sources:

- [Pinned rust-url source](https://github.com/servo/rust-url/tree/00a6ce58d02f4e0d43c5ca0702c0bedb8b1ebf3a)
- [Ada 4.0.0 crate](https://crates.io/crates/ada-url/4.0.0)
- [Pinned Ada build script](https://github.com/ada-url/rust/blob/af35063623f2c93bb337728d03c03644b47e75c9/build.rs)
- [Pinned Ada CI](https://github.com/ada-url/rust/blob/af35063623f2c93bb337728d03c03644b47e75c9/.github/workflows/ci.yml)
- [WASI SDK 34 release](https://github.com/WebAssembly/wasi-sdk/releases/tag/wasi-sdk-34)
