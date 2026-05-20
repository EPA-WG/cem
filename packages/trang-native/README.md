# @epa-wg/trang-native

Native (GraalVM-compiled) Trang for RELAX NG / DTD / XSD conversion —
**no JRE required** on consumer machines.

Wraps upstream [`relaxng/jing-trang`](https://github.com/relaxng/jing-trang)
and ships per-platform binaries built by GitHub Actions. Used by the
`cem_ml` schema-emit verification fixtures (AC-S-2 RELAX NG compact
round-trip) and available to any consumer that wants Trang without
installing Java.

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

## Building from source

You only need this section if you're cutting a new release or hacking
on the native-image config.

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

# 3. Build the native binary for the current host.
yarn nx run @epa-wg/trang-native:build

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
