# CEM native platform packages

Phase 8 turns the generated token files into installable, compile-checked iOS and Android package layouts without
changing the Markdown token specifications as the source of truth.

## Supported toolchains

| Platform | Supported compile contract | CI host |
| -------- | -------------------------- | ------- |
| iOS | Xcode 16.4, Swift 6.1 in language mode 6, iOS 15.0+ | GitHub `macos-15` with Xcode 16.4 selected |
| Android | AGP 9.2.0, Gradle 9.4.1, JDK 17, Kotlin/Compose 2.3.21, Compose BOM 2026.08.00, compileSdk 37 | GitHub `ubuntu-latest` with JDK and Android SDK |

These are reproducible validation pins, not promises that consumers must remain on exactly one patch forever. Newer
toolchains are supported only after the native CI gates prove them. Android uses AGP's built-in Kotlin support; the
Compose compiler plugin remains pinned to the Kotlin version.

## Generate and consume

Generate all native artifacts through Nx:

```bash
yarn nx run @epa-wg/cem-theme:build:token-platforms
```

The iOS directory is a Swift Package. Add
`packages/cem-theme/dist/lib/token-platforms/ios/` as a local package and `import CEMTokens`, or copy its root
`CEMTokens.swift` compatibility file into an app target.

The Android directory is a complete Gradle project with a `cem-tokens` library and a Compose `sample` consumer. Copy
the library module into an existing build, or retain the standalone `values/`, `values-night/`, and `compose/`
compatibility files.

Neither package manifest reads the generator, workspace source tree, or repository dependencies. The host compile
scripts copy the generated directory to a temporary clean-consumer location before building it.

## Native component guidance

All 49 public CEM primitives retain their canonical `cem-*` web name. Native implementations use derived guidance
names (`CEM<Name>` for SwiftUI and `Cem<Name>` for Compose), but those names describe equivalent components rather
than generated UI widgets. The public component catalog remains authoritative for component membership and token
families.

Apply these rules to every native adapter:

- Map each catalog state explicitly. Preserve enabled, pressed/active, selected, expanded, loading, invalid,
  indeterminate, and empty meaning with native state APIs and non-color cues. Pointer-only hover is optional on touch
  devices; visible keyboard focus is not.
- Resolve color through the generated mode constants or Android resources. Do not translate a semantic CEM state to a
  hard-coded native color. Native/system and high-contrast behavior may override paint while preserving meaning.
- Resolve typography size and weight from generated tokens, then let product adapters opt into Dynamic Type or Android
  font scaling according to their content policy. Phase 8 does not silently change the canonical nominal scale.
- Preserve native control ownership, accessible names, labels, help/error relationships, roles, value/state
  announcements, reading order, reduced motion, and the CEM minimum target size. Disabled and readonly remain distinct.
- Keep platform-specific interaction mechanics native. Match CEM semantics and token families; do not reproduce web
  DOM structure in SwiftUI or Compose.

The machine-readable source is `examples/native/component-guidance.json`. Its verifier derives one mapping for every
catalog primitive and rejects missing names, states, token families, accessibility categories, or uncovered component
states.

## Parity and validation

The Phase 8 web, SwiftUI, and Compose button/card fixtures share 14 canonical tokens covering color, space, shape,
type, weight, and minimum target size. This is the credential-free visual-parity boundary. Live reviewed Figma canvas
parity remains Phase 10 and is not claimed by these checks.

Run the portable Phase 8 gate anywhere Node and the workspace dependencies are available:

```bash
yarn nx run @epa-wg/cem-theme:verify:phase8
```

Supported native hosts additionally run:

```bash
yarn nx run @epa-wg/cem-theme:compile:ios-platform
yarn nx run @epa-wg/cem-theme:compile:android-platform
```

The first command requires macOS/Xcode. The second works on Linux AMD64 or another supported host when JDK 17,
Gradle 9.4.1, and Android SDK 37 are installed. Local static validation remains truthful when those toolchains are
absent; it never substitutes syntax inspection for the host compile evidence.
