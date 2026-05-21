# @epa-wg/trang-native

Native (GraalVM-compiled) Trang for RELAX NG / DTD / XSD conversion —
**no JRE required** on consumer machines.

Wraps upstream [`relaxng/jing-trang`](https://github.com/relaxng/jing-trang)
and ships per-platform binaries built by GitHub Actions. Used by the
`cem_ml` schema-emit verification fixtures (AC-S-2 RELAX NG compact
round-trip) and available to any consumer that wants Trang without
installing Java.

## Purpose

Trang is the reference RELAX NG converter (RNG ↔ RNC ↔ XSD ↔ DTD), but it
ships as a Java tool — invoking it normally requires a JDK or JRE on every
developer machine and CI runner. For the `cem_ml` test suite, Trang is the
**parity oracle**: the schema emitter writes `*.rng` / `*.rnc` and a
round-trip through Trang proves the two forms agree. Forcing every
contributor (and every CI job) to install a JDK just to run that one check
is heavyweight and fragile.

`@epa-wg/trang-native` solves that by shipping Trang as a self-contained,
ahead-of-time-compiled (GraalVM `native-image`) executable. Consumers get a
single binary that starts in milliseconds and has no Java runtime
dependency.

## Principles

1. **No JDK on the consumer side.** The binary is fully self-contained.
   Building it from source needs GraalVM + Ant; using it does not.
2. **Only the host platform's binary is installed.** The npm package
   itself contains no native code — at install time, `postinstall` detects
   `process.platform` × `process.arch` and downloads exactly one archive.
   No fat package, no multi-platform tarball.
3. **Per-platform binaries live in GitHub Releases, not in npm.** Each
   `trang-native-v<version>` GitHub Release carries one archive per
   supported target plus a `SHA256SUMS` manifest. This keeps the npm tarball
   tiny and lets us re-cut binaries (e.g. for a new architecture) without
   republishing to npm.
4. **Integrity is verified.** The postinstaller checks the downloaded
   archive's SHA-256 against the manifest before extracting.
5. **Opt-out is always available.** `TRANG_NATIVE_SKIP_DOWNLOAD=1`,
   `TRANG_NATIVE_BINARY=/abs/path`, and pre-staged binaries (for
   air-gapped builds) all bypass the network step.
6. **Pinned upstream.** `upstream.json` records the exact `jing-trang`
   commit each release was built from; rebuilds are reproducible.

## Role in `cem_ml`

`cem_ml`'s schema-emit verification fixtures (AC-S-2) use Trang to validate
that the emitter's RELAX NG XML and compact outputs are semantically
equivalent to each other and to a known-good oracle. The cem-ml test
harness depends on `@epa-wg/trang-native` as a dev-dependency, so the moment
`yarn install` finishes on a developer or CI machine, the platform-correct
`trang` binary is already on disk and the parity tests run with no further
setup. Only one binary — the host's — is fetched, so the install cost is a
few MB rather than tens of MB.

## Supported platforms

| Platform        | Target triple        | Status |
| --------------- | -------------------- | ------ |
| Linux x86_64    | `linux-x86_64`       | Tier A (CI server + dev) |
| Linux arm64     | `linux-aarch64`      | Tier A (Graviton / Ampere / RPi server) |
| Windows x86_64  | `windows-x86_64`     | Tier A (Windows dev workstations) |
| macOS arm64     | `macos-aarch64`      | Tier A (Apple Silicon dev workstations) |

Other platforms (Windows arm64, macOS x86_64, FreeBSD, …) are not
shipped today; consumers on unsupported platforms get a clear
postinstall warning and can fall back to a system-installed `trang`.

## Install

```bash
npm install --save-dev @epa-wg/trang-native
# or
yarn add --dev @epa-wg/trang-native
```

The `postinstall` hook reads `process.platform` + `process.arch`,
downloads the matching archive from the
[`trang-native-v<version>` GitHub Release](https://github.com/EPA-WG/cem/releases),
verifies its SHA-256 checksum, and extracts the binary into
`node_modules/@epa-wg/trang-native/bin/native/`. The package's `bin`
entry points to a tiny Node shim (`bin/trang.mjs`) so consumers can call
`npx trang …` or any `package.json` script.

### Skipping the download

| Environment                     | Effect                                                                 |
| ------------------------------- | ---------------------------------------------------------------------- |
| `TRANG_NATIVE_SKIP_DOWNLOAD=1`  | Postinstall is a no-op. Use when the binary is provided out-of-band.    |
| `TRANG_NATIVE_BINARY=/abs/path` | Shim invokes this path instead of `bin/native/`. Use for system Trang.  |
| `TRANG_NATIVE_REQUIRE_PREBUILT=1` | `nx run @epa-wg/trang-native:build` fails instead of compiling from source when no local/release binary is available. Use in CI. |
| Workspace dev install           | Postinstall auto-skips when the script is not under `node_modules/`.    |

### Offline / air-gapped use

Pre-stage the matching archive at `bin/native/<target>/trang(.exe)`
before `npm install`; the postinstall script's existence check finds it
and skips the download.

## Usage

CLI:

```bash
npx trang schema.rnc schema.rng    # compact → XML
npx trang schema.rng schema.rnc    # XML → compact
npx trang schema.rng schema.xsd    # RELAX NG → XSD
npx trang schema.rng schema.dtd    # RELAX NG → DTD
```

Programmatic (Node):

```js
import { runTrang } from '@epa-wg/trang-native';

const result = await runTrang(['schema.rnc', 'schema.rng']);
if (result.status !== 0) {
  console.error(result.stderr);
  process.exit(result.status);
}
```

The shim simply `execFile`s the native binary with the arguments
forwarded verbatim. Trang's own CLI documentation applies — see
[upstream docs](https://relaxng.org/jclark/trang.html).

## How `nx build` resolves the binary

`nx run @epa-wg/trang-native:build` is the **smart** entry point. A
GraalVM `native-image` compile takes minutes, so it is avoided
whenever a prebuilt binary can be obtained. `scripts/acquire-binary.mjs`
resolves a host-platform binary in this order:

1. **Local** — if `build/native/<triple>/` already holds a binary
   whose `metadata.json` records the current `package.json` version,
   it is reused as-is. Zero work.
2. **Release** — otherwise the matching `trang-native-v<version>`
   GitHub Release archive is downloaded, SHA-256 verified against its
   sidecar, and extracted. This is the *same* artifact a published-npm
   consumer's `postinstall` pulls — "npm release" and "git release"
   are the one Release.
3. **From source** — only if neither of the above yields a binary does
   it run the full `fetch-source → build-jar → build-native` chain,
   which requires GraalVM + Ant on the host. CI should set
   `TRANG_NATIVE_REQUIRE_PREBUILT=1` so this path is never taken by
   ordinary validation jobs.

A `native-image` rebuild therefore happens **only** when:

- the package **version was bumped** — no Release exists for the new
  version yet, so step 2 misses and the source build runs (you then
  cut the release, after which every machine resolves via step 1/2); or
- a **force** is requested — `TRANG_NATIVE_FORCE_BUILD=1` or passing
  `--force` to the script. Pair force with `--skip-nx-cache` so Nx
  does not replay a cached result:

  ```bash
  TRANG_NATIVE_FORCE_BUILD=1 yarn nx run @epa-wg/trang-native:build --skip-nx-cache
  ```

Nx caching reinforces this: `package.json` is a `build` input, so a
version bump invalidates the cache; an unchanged version replays the
cached output without re-running the resolver at all.

To compile unconditionally regardless of any available binary, use the
explicit `build-from-source` target (see below).

## Building from source

You only need this section if you're cutting a new release, hacking on
the native-image config, or running on a platform with no prebuilt
binary.

Prerequisites:

- GraalVM JDK 21+ with `native-image` on PATH (or `GRAALVM_HOME` set)
- Apache Ant 1.10+
- A target-platform host (GraalVM does NOT cross-compile; you build
  Linux x86_64 on Linux x86_64, macOS arm64 on Apple Silicon, etc.)

```bash
# 1. Fetch upstream Trang at the pinned ref (upstream.json).
yarn nx run @epa-wg/trang-native:fetch-source

# 2. Build trang.jar via Ant.
yarn nx run @epa-wg/trang-native:build-jar

# 3. Compile the native binary for the current host. Unlike `build`,
#    `build-from-source` always invokes native-image — it never
#    consults a local or released binary.
yarn nx run @epa-wg/trang-native:build-from-source

# 4. Package into a release archive.
yarn nx run @epa-wg/trang-native:package
```

To target a specific platform explicitly (still requires you to be on
that platform's host):

```bash
yarn nx run @epa-wg/trang-native:build:linux-x86_64
yarn nx run @epa-wg/trang-native:build:linux-aarch64
yarn nx run @epa-wg/trang-native:build:windows-x86_64
yarn nx run @epa-wg/trang-native:build:macos-aarch64
```

The release workflow at `.github/workflows/trang-native-release.yml`
runs a matrix across the four `runs-on` targets, uploads all archives
+ a `SHA256SUMS` file to a GitHub Release tagged
`trang-native-v<version>`.

### Reflection config

GraalVM's `native-image` requires reflection metadata for any Java
classes loaded dynamically. Trang's reflection surface is small and
captured in [`reflect-config/`](reflect-config/). To regenerate (run
on a host with GraalVM + Ant):

```bash
# Run Trang under the native-image-agent against a representative
# fixture to capture the reflection it actually uses.
GRAALVM_AGENT=1 node scripts/build-native.mjs --capture-reflect
```

The agent writes JSON config files into `reflect-config/.agent-scratch/`;
review and merge into `reflect-config/reflect-config.json` before
committing.

## License

This package's wrappers are BSD-2-Clause (matches upstream Trang). See
[LICENSE](LICENSE) and [NOTICE](NOTICE).
