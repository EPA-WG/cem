# Local Registry

This directory contains the Verdaccio configuration used by the root Nx `local-registry` target.

## Target

```bash
yarn nx run @epa-wg/cem:local-registry
```

The target is defined in the root `package.json` under `nx.targets.local-registry` and uses:

- executor: `@nx/js:verdaccio`
- port: `4873`
- config: `.verdaccio/config.yml`
- storage: `tmp/local-registry/storage`

## Purpose

Use this target for local package publish/install smoke tests before publishing the CEM packages publicly. The registry
allows local anonymous publish, unpublish, and install operations, and proxies requests to the public npm registry when
a
package is not available locally.

## Important

Do not remove `.verdaccio/config.yml` unless the root `local-registry` target and the Verdaccio dev dependency are also
removed or replaced. The target references this config path directly and will fail if the file is missing.
