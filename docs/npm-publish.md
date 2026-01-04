# NPM Publishing Workflow

This document describes the complete workflow for releasing packages to npm in the `@epa-wg/cem` monorepo.

## Overview

The repository uses a **develop 🠊 main 🠊 publish** workflow:

- **`develop`** branch: Active development and feature work
- **`main`** branch: Release-ready code
- **GitHub Actions**: Automated publishing to npm

## Development Workflow

### 1. Working on the Develop Branch

All development work should happen on the `develop` branch:

```bash
# Ensure you're on develop and up to date
git checkout develop
git fetch origin
git pull origin develop

# Make your changes
# ... edit files ...

# Commit with conventional commit messages
git add .
git commit -m "fix: description of bug fix"
# or
git commit -m "feat: description of new feature"

# Push to develop
git push origin develop
```

**Important:** Use [Conventional Commits](https://www.conventionalcommits.org/) format:

- `fix:` - Patch version bump (0.0.x)
- `feat:` - Minor version bump (0.x.0)
- `feat!:` or `BREAKING CHANGE:` - Major version bump (x.0.0)

### 2. Create Pull Request to Main

When ready for release, create a PR from `develop` to `main`:

1. **Via GitHub UI:**
    - Go to https://github.com/EPA-WG/cem/pulls
    - Click "New Pull Request"
    - Base: `main` 🠈 Compare: `develop`
    - Create the PR

2. **Or via GitHub CLI:**
   ```bash
   gh pr create --base main --head develop --title "Release: merge develop to main" --body "Preparing for release"
   ```

3. **Review and Merge:**
    - Wait for CI checks to pass
    - Review the changes
    - Merge the PR (use "Merge commit" or "Squash and merge")

## Release Process

### 3. Prepare and Publish Release (Local)

After the PR is merged to `main`, prepare the release locally:

```bash
# Switch to main and update
git checkout main
git fetch origin
git pull origin main

# Run the release preparation script
yarn publish:prepare
```

**What `yarn publish:prepare` does:**

1. 🔙 Restores `workspace:*` protocols for local dependencies
2. 📦 Runs `nx release` - bumps versions based on conventional commits
3. ❓ Generates/updates CHANGELOG.md files
4. 🔄 Replaces `workspace:*` with semantic versions (e.g., `^0.0.5`)
5. 🔒 Updates `yarn.lock`
6. ✏️ Amends the release commit
7. 🏷️ Creates git tag (e.g., `0.0.5`)
8. ⬆️ Pushes commit and tag to GitHub

**Example output:**

```
🚀 Starting release preparation...
🔙 Restoring workspace protocol for release...
✓ Workspace protocol restoration complete
📦 Running Nx release...
@epa-wg/cem 
✍️  New version 0.0.5 written to manifest: package.json
...
✅  Release preparation complete!
🎉  Ready to publish via CI/CD
```

### 4. Validate GitHub Actions Publish

After pushing, GitHub Actions automatically publishes to npm:

1. **Monitor the workflow:**
    - Go to https://github.com/EPA-WG/cem/actions/workflows/publish.yml
    - Find the workflow run for your tag (e.g., `0.0.5`)

2. **Check the status:**
    - ✅ **Success** - Packages published to npm
    - L **Failure** - Check logs for errors

3. **Verify on npm:**
   ```bash
   npm view @epa-wg/cem-theme version
   npm view @epa-wg/cem-components version
   ```

   Or visit:
    - https://www.npmjs.com/package/@epa-wg/cem-theme
    - https://www.npmjs.com/package/@epa-wg/cem-components

## Post-Release: Sync Develop Branch

After a successful release, the `main` branch is ahead of `develop`. Sync them:

### Option 1: Merge Main into Develop (Recommended)

```bash
# Switch to develop
git checkout develop

# Fetch latest changes
git fetch origin

# Merge main into develop
git merge origin/main

# Push updated develop
git push origin develop
```

This preserves the complete history and is the safest approach.

### Option 2: Rebase Develop onto Main

If you want a linear history and comfortable with rebasing:

```bash
# Switch to develop
git checkout develop

# Fetch latest changes
git fetch origin

# Rebase develop onto main
git rebase origin/main

# Force push (only if no one else is working on develop!)
git push origin develop --force-with-lease
```

**Warning:** Only use rebase if:

- You're the only one working on `develop`, or
- You've coordinated with your team

### Option 3: Reset Develop to Main

If develop has no unique commits and should match main exactly:

```bash
# Switch to develop
git checkout develop

# Reset to match main
git reset --hard origin/main

# Force push
git push origin develop --force-with-lease
```

**Warning:** This discards all commits on `develop` not in `main`.

## Troubleshooting

### Release Failed - Tag Already Exists

If the release script fails because the tag exists:

```bash
# Delete local and remote tag
git tag -d 0.0.5
git push origin :refs/tags/0.0.5

# Run publish:prepare again
yarn publish:prepare
```

### Wrong Version Number

If you need to change the version:

```bash
# Manually edit version in package.json files
# Then commit and create tag manually
git add package.json packages/*/package.json
git commit -m "chore(release): publish 0.0.5"
git tag 0.0.5
git push origin main --tags
```

### GitHub Actions Publish Failed

Check the logs at: https://github.com/EPA-WG/cem/actions/workflows/publish.yml

Common issues:

- **NPM_ACCESS_TOKEN expired** - Update secret in GitHub settings
- **Version already published** - Can't republish the same version
- **Build failed** - Fix code issues and create a new release

### Develop Branch Has Merge Conflicts

After syncing from main:

```bash
git checkout develop
git fetch origin
git merge origin/main

# If conflicts occur:
# 1. Resolve conflicts in your editor
# 2. Stage resolved files
git add .

# 3. Complete the merge
git commit

# 4. Push
git push origin develop
```

## Release Checklist

- [ ] All changes committed to `develop`
- [ ] PR from `develop` to `main` created and reviewed
- [ ] PR merged to `main`
- [ ] Checked out `main` and pulled latest
- [ ] Ran `yarn publish:prepare` locally
- [ ] Verified GitHub Actions workflow succeeded
- [ ] Verified packages on npm
- [ ] Synced `develop` branch with `main`
- [ ] Continued development on `develop`

## Conventional Commits Reference

Version bumps are determined by commit messages:

| Commit Type        | Example                         | Version Bump  |
|--------------------|---------------------------------|---------------|
| `fix:`             | `fix: correct button alignment` | 0.0.x (patch) |
| `feat:`            | `feat: add dark mode`           | 0.x.0 (minor) |
| `feat!:`           | `feat!: redesign API`           | x.0.0 (major) |
| `BREAKING CHANGE:` | See below                       | x.0.0 (major) |

**Example with breaking change:**

```
feat: redesign API

BREAKING CHANGE: The old API has been removed.
Use the new `createTheme()` function instead.
```

## Scripts Reference

### `yarn publish:prepare`

Prepares and pushes a release (versions, changelog, tag).

**Location:** `./tools/scripts/publish-prepare.sh`

**Steps:**

1. Restores workspace protocols
2. Bumps versions via `nx release`
3. Replaces workspace protocols with semantic versions
4. Updates lockfile
5. Amends commit and recreates tag
6. Pushes to remote

### Helper Scripts

- **`tools/scripts/restore-workspace-protocol.cjs`**
  Converts semantic versions back to `workspace:*`

- **`tools/scripts/replace-workspace-protocol.cjs`**
  Converts `workspace:*` to semantic versions (e.g., `^0.0.5`)

- **`tools/scripts/sync-release-version.cjs`**
  Synchronizes versions across packages (called by Nx)

## Additional Resources

- [Nx Release Documentation](https://nx.dev/recipes/nx-release)
- [Conventional Commits](https://www.conventionalcommits.org/)
- [Semantic Versioning](https://semver.org/)
- [GitHub Actions Workflows](../.github/workflows/)
