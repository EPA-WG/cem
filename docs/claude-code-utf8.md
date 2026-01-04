# UTF-8 Encoding for Claude Code Generated Files

This document specifically addresses how to ensure Claude Code generates properly UTF-8 encoded Markdown and other text
files.

## The Problem

When Claude Code generates files using the Write tool, certain characters (especially box drawing, arrows, and symbols)
could potentially be corrupted if not properly handled. The original `docs-generation.md` had encoding issues with
control characters appearing instead of proper UTF-8.

## The Solution: Multi-Layer Validation

### Layer 1: Write Tool Behavior (Verified Working ✓)

**Test Results**: Claude Code's Write tool correctly handles UTF-8

```bash
# Test performed:
# - Created file with box drawing: ├── │ └──
# - Created file with arrows: → ← ↑ ↓
# - Created file with symbols: ✓ ✗ •
# - Created file with emoji: 🚀 📦 ✅ ❌

# Result: All characters correctly encoded as UTF-8
```

**Location of test**: `/tmp/utf8-test.md`

**Verification**:

```bash
file -i /tmp/utf8-test.md
# Output: text/plain; charset=utf-8
```

### Layer 2: Pre-Commit Hook (Automatic Validation)

**Location**: `.git/hooks/pre-commit`

**What it does**:

- Automatically runs before EVERY commit
- Validates all staged `.md`, `.js`, `.ts`, and other text files
- Prevents commits with encoding issues

**How it protects against Claude Code errors**:

```bash
# If Claude Code generates a file with encoding issues:
git add problematic-file.md
git commit -m "Add documentation"

# Output:
# 🔍 Validating UTF-8 encoding of staged files...
# ❌ UTF-8 validation failed for the following files:
#   - problematic-file.md
#
# Commit rejected! ← Claude Code cannot commit broken files
```

**Testing**:

```bash
# Manually test the hook:
git add your-file.md
.git/hooks/pre-commit
```

### Layer 3: CI Validation (GitHub Actions)

**Location**: `.github/workflows/ci.yml`

**Added step**:

```yaml
- name: Validate UTF-8 encoding
  run: yarn validate:utf8
```

**How it protects**:

- Runs on every push and pull request
- Catches any files that bypassed the pre-commit hook
- Prevents merging PRs with encoding issues
- **Fails the build if Claude Code generated files with bad encoding**

### Layer 4: Manual Validation

**Command**:

```bash
yarn validate:utf8
```

**What it checks**:

- All Markdown files in the repository
- All JavaScript/TypeScript files
- All YAML configuration files
- Returns exit code 1 if ANY file has encoding issues

**When to run**:

- After Claude Code generates multiple files
- Before creating a pull request
- When you see strange characters in diffs

### Layer 5: Git Attributes (Prevention)

**Location**: `.gitattributes`

**Content**:

```gitattributes
*.md text working-tree-encoding=UTF-8
*.js text working-tree-encoding=UTF-8
*.ts text working-tree-encoding=UTF-8
# ... all text file types
```

**Purpose**:

- Instructs Git to handle these files as UTF-8
- Prevents Git from corrupting encoding during operations
- Normalizes line endings to LF

### Layer 6: EditorConfig (Editor-Level)

**Location**: `.editorconfig`

**Content**:

```ini
[*]
charset = utf-8
```

**Purpose**:

- Ensures editors create files as UTF-8 by default
- Applies to manual edits AND Claude Code operations

## Workflow: How Claude Code Files are Protected

```
┌─────────────────────────────────────────────────────────────┐
│ 1. Claude Code generates file with Write tool               │
│    └─> Write tool creates UTF-8 file ✓                      │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ↓
┌─────────────────────────────────────────────────────────────┐
│ 2. User requests: "commit these changes"                    │
│    └─> Claude Code runs: git add + git commit               │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ↓
┌─────────────────────────────────────────────────────────────┐
│ 3. Pre-commit hook runs automatically                       │
│    ├─> Validates UTF-8 encoding                             │
│    ├─> If VALID: Commit succeeds ✓                          │
│    └─> If INVALID: Commit blocked ✗                         │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ↓ (if commit succeeded)
┌─────────────────────────────────────────────────────────────┐
│ 4. Push to GitHub                                           │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ↓
┌─────────────────────────────────────────────────────────────┐
│ 5. GitHub Actions CI runs                                   │
│    ├─> Runs: yarn validate:utf8                             │
│    ├─> If VALID: Build continues ✓                          │
│    └─> If INVALID: Build fails ✗                            │
└─────────────────────────────────────────────────────────────┘
```

## What This Means for Claude Code Users

### ✅ Automatic Protection

You don't need to:

- Manually check encoding after Claude Code generates files
- Worry about committing files with encoding issues
- Remember to run validation commands

The system automatically:

- Validates before every commit
- Validates in CI/CD
- Blocks bad files from being merged

### 🔍 When to Manually Check

Run `yarn validate:utf8` if:

- You see strange characters in file diffs (control chars, \x92, U+FFFD, etc.)
- Claude Code generated many files at once
- You want to verify before creating a PR

### 🛠️ How to Fix Issues

If validation fails:

```bash
# 1. Identify the problematic file(s)
yarn validate:utf8

# 2. Check what's wrong
file -i path/to/problematic-file.md

# 3. Option A: Ask Claude Code to regenerate the file
# "Please recreate docs/example.md with proper UTF-8 encoding"

# 4. Option B: Manually fix in editor
# - Open in VS Code (or UTF-8 capable editor)
# - Check for control characters or strange symbols
# - Re-save with UTF-8 encoding

# 5. Option C: Use conversion tool
iconv -f ISO-8859-1 -t UTF-8 file.md > file.md.utf8
mv file.md.utf8 file.md

# 6. Verify the fix
yarn validate:utf8
```

## Testing the Protection

### Test 1: Verify Write Tool Creates UTF-8

```bash
# Check the test file created earlier
file -i /tmp/utf8-test.md
# Expected: text/plain; charset=utf-8

cat /tmp/utf8-test.md
# Should show: ├── │ └── → ✓ ✗ (not control chars)
```

### Test 2: Verify Pre-Commit Hook Works

```bash
# Create a file with bad encoding (simulate)
echo -e "Test \x92 Bad byte" > /tmp/bad.md
cp /tmp/bad.md docs/test-bad.md

# Try to commit it
git add docs/test-bad.md
git commit -m "Test bad encoding"

# Expected: Commit blocked by pre-commit hook
# Clean up
rm docs/test-bad.md
```

### Test 3: Verify Validation Script

```bash
yarn validate:utf8
# Expected: ✅ All files are properly UTF-8 encoded
```

## Current Status

All 28 text files in the repository are validated as proper UTF-8:

```
📊 Checked 28 files
✅ All files are properly UTF-8 encoded
```

### Files Covered

- **Markdown**: `*.md` in `packages/`, `docs/`, root
- **JavaScript**: `*.js`, `*.mjs`, `*.cjs` in `tools/`
- **TypeScript**: `*.ts`, `*.mts`, `*.cts` in `packages/`
- **Config**: `*.yml`, `*.yaml` in `.github/workflows/`
- **Excludes**: `node_modules/`, `dist/`, `.nx/` (generated files)

## Summary: Claude Code UTF-8 Assurance

| Layer             | Protection                | Status         |
|-------------------|---------------------------|----------------|
| Write Tool        | Generates UTF-8 correctly | ✅ Verified     |
| Pre-commit Hook   | Blocks bad commits        | ✅ Installed    |
| CI Validation     | Blocks bad PRs            | ✅ Configured   |
| Git Attributes    | Prevents corruption       | ✅ Configured   |
| EditorConfig      | Editor defaults           | ✅ Configured   |
| Manual Validation | `yarn validate:utf8`      | ✅ Available    |

**Result**: Claude Code-generated files are protected by 6 layers of UTF-8 validation and enforcement.
