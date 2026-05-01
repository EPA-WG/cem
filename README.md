# CEM - Consumer Semantic Material Theme and custom-element components library

A theme system and custom-element component library for building declarative, no-JavaScript web applications.

CEM reinterprets Google’s [Material Design Guidelines](https://material.io/design) through a consumer-first
lens—focusing on how users perceive and interact with interfaces, rather than how designers construct them.

It is implemented as a combination of:

* [AI instructions](packages/cem-theme/src/lib/tokens) AI instructions for generating and adapting themes
* CSS design tokens and stylesheets
* Web Components for use in fully declarative applications (no JS required)

The result is a system where consumer semantics drive UI behavior and appearance, enabling consistent, accessible, and
maintainable interfaces.

[![npm version](https://badge.fury.io/js/%40epa-wg%2Fcem-theme.svg)](https://badge.fury.io/js/%40epa-wg%2Fcem-theme)
[![Downloads](https://img.shields.io/npm/dm/@epa-wg/cem-theme.svg)](https://www.npmjs.com/package/@epa-wg/cem-theme)
[![License](https://img.shields.io/npm/l/@epa-wg/cem-theme.svg)](./LICENSE)

# Figma design library

The CEM UI Kit is the Figma-native design library for CEM tokens, foundations, components, patterns, and QA fixtures.
Its Tokens page contains the native Figma Variables collection and visual token demos generated from the same source
artifacts as the CSS generator pages.

- [CEM UI Kit Tokens page](https://www.figma.com/design/vLZUzjS7xHACjXgYLA9vtD/CEM-UI-Kit?node-id=2-24&t=QQwTKeMg0v9dTQ10-1)
- [Figma token workflow](packages/cem-theme/docs/token-figma.md)

# Project documentation

- [Documentation index](docs/index.md)
- [Roadmap](roadmap.md)
- [Todo](docs/todo.md)
- [CEM theme package](packages/cem-theme/README.md)
- [CEM components package](packages/cem-components/README.md)
- [Token export architecture](packages/cem-theme/docs/token-export.md)

# Run locally

```bash
yarn start
```

# New Nx Repository

<a alt="Nx logo" href="https://nx.dev" target="_blank" rel="noreferrer"><img src="https://raw.githubusercontent.com/nrwl/nx/master/images/nx-logo.png" width="45"></a>

✨ Your new, shiny [Nx workspace](https://nx.dev) is ready ✨.

[Learn more about this workspace setup and its capabilities](https://nx.dev/nx-api/js?utm_source=nx_project&utm_medium=readme&utm_campaign=nx_projects)
or run `npx nx graph` to visually explore what was created. Now, let's get you up to speed!

## Generate a library

```sh
npx nx g @nx/js:lib packages/pkg1 --publishable --importPath=@my-org/pkg1
```

## Run tasks

To build the library use:

```sh
npx nx build pkg1
```

To run any task with Nx use:

```sh
npx nx <target> <project-name>
```

These targets are
either [inferred automatically](https://nx.dev/concepts/inferred-tasks?utm_source=nx_project&utm_medium=readme&utm_campaign=nx_projects)
or defined in the `project.json` or `package.json` files.

[More about running tasks in the docs &raquo;](https://nx.dev/features/run-tasks?utm_source=nx_project&utm_medium=readme&utm_campaign=nx_projects)

## Versioning and releasing

To version and release the library use

```
npx nx release
```

Pass `--dry-run` to see what would happen without actually releasing the library.

[Learn more about Nx release &raquo;](https://nx.dev/features/manage-releases?utm_source=nx_project&utm_medium=readme&utm_campaign=nx_projects)
