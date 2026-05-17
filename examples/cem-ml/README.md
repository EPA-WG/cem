# Canonical CEM-ML Fixtures

This directory is the canonical fixture surface for `cem-ml`. These `.cem` files use
the curly-brace syntax from `docs/cem-ml-syntax.md`.

The existing `examples/semantic/*.html` files remain the secondary HTML parity surface.
Parser, validation, transform, and snapshot tests should keep both fixture sets aligned:
the CEM-ML fixture is the source-shape canonical, while the HTML fixture proves adapter
parity.

Fixture pairs:

| Canonical CEM-ML | HTML parity |
|------------------|-------------|
| `login.cem` | `../semantic/login.html` |
| `registration.cem` | `../semantic/registration.html` |
| `profile.cem` | `../semantic/profile.html` |
| `assets-list.cem` | `../semantic/assets-list.html` |
| `message-thread.cem` | `../semantic/message-thread.html` |
