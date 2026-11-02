# Prepared resource entry for discussion #24

This is a reviewable draft for the existing resource discussion. It has not been
posted. All links below were read on 2026-11-01; reference pages pin `712ff2e4`.
Keep the distribution statement until an actual public download/host is verified.

---

## Login exercise resources

- [Participant setup: local Studio kit](https://github.com/EPA-WG/cem/blob/712ff2e45b89d159abba7b8447ab53e5efb8ae68/docs/declarative-amsterdam-2026/participant-setup.md)
- [Login project: frozen inputs, commands and expected outputs](https://github.com/EPA-WG/cem/blob/712ff2e45b89d159abba7b8447ab53e5efb8ae68/docs/declarative-amsterdam-2026/login-presenter/README.md)
- [Supplier companion: XML/CSV inputs and HTML/JSON/YAML outputs](https://github.com/EPA-WG/cem/blob/712ff2e45b89d159abba7b8447ab53e5efb8ae68/docs/declarative-amsterdam-2026/supplier-presenter/README.md)

The Studio kit is currently supplied by local transfer. These pages document the
exercise and provide readable reference data; no public kit download or hosted
Studio exercise is advertised here. After extraction, the supplied kit runs with
`node start.mjs` using Node built-ins, with no application dependency installation.
The verified environment is Linux x64, Node 24.16.0 and Chromium 148.0.7778.96.
Install Node/browser and warm the offline cache before the session.

The required exercise uses an independent copy of Login in the shared Studio
workspace: edit, validate and repair, inspect/format, compare HTML/CEM semantic
fields, query/transform, and show the light-DOM view. Supplier is a companion;
its saved checkpoints also provide an offline fallback. Keep saved projects and
drafts, and use the kit's same origin when reopening work.

Use disposable values in the local Login example. Entered passwords or one-time
codes block recovery until explicitly cleared; they are not exported in
checkpoints or project bundles. Automatic whole-Studio recovery stays disabled.

Bookmark [Resources and updates](https://github.com/EPA-WG/cem/discussions/24)
for subsequent delivery information.
