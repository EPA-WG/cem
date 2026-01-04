# UTF-8 Encoding Guidelines

This document describes how UTF-8 encoding is enforced throughout the CEM project to ensure consistent, error-free text file handling.

## Why UTF-8 Matters

UTF-8 encoding is critical for:
- **International Characters**: Proper display of non-ASCII characters (é, ñ, 中, etc.)
- **Markdown Features**: Box drawing characters (├──, │, └──), arrows (→), checkmarks (✓)
- **Emoji Support**: Modern documentation often includes emoji (🚀, ✅, ❌)
- **Cross-platform Compatibility**: Consistent file handling across Windows, macOS, and Linux
- **Git Operations**: Avoiding encoding corruption in version control

## Encoding Safeguards

### 1. EditorConfig (`.editorconfig`)

Ensures all editors use UTF-8 by default:

```ini
[*]
charset = utf-8
```

**Supported by**: VS Code, WebStorm, Sublime Text, Vim, Emacs, and [many others](https://editorconfig.org/#pre-installed)

### 2. Git Attributes (`.gitattributes`)

Enforces UTF-8 encoding in Git operations:

```gitattributes
# Auto detect text files and normalize line endings to LF
* text=auto eol=lf

# Explicitly mark text files as UTF-8
*.md text working-tree-encoding=UTF-8
*.js text working-tree-encoding=UTF-8
*.ts text working-tree-encoding=UTF-8
*.json text working-tree-encoding=UTF-8
# ... (all text file types)
```

**Purpose**: Prevents Git from corrupting UTF-8 files during checkout/commit operations.

### 3. Markdown Compilation Script

The `tools/scripts/compile-markdown.mjs` explicitly specifies UTF-8:

```javascript
// Reading Markdown files
const content = await readFile(srcPath, 'utf-8');

// Writing XHTML files
await writeFile(distPath, xhtml, 'utf-8');
```

### 4. UTF-8 Validation Script

**Location**: `tools/scripts/validate-utf8.mjs`

**Run manually**:
```bash
yarn validate:utf8
```

**What it checks**:
- ✓ Files can be decoded as UTF-8 without errors
- ✓ No invalid UTF-8 byte sequences (no U+FFFD replacement characters)
- ✓ No null bytes or problematic control characters
- ✓ Files re-encode to identical bytes (no data loss)

**Coverage**:
- All `*.md` files in `packages/`, `docs/`, and root
- All `*.js`, `*.mjs`, `*.cjs`, `*.ts`, `*.mts`, `*.cts` files
- All `*.yml`, `*.yaml` workflow files
- Excludes: `node_modules/`, `dist/`, `.nx/`

### 5. Git Pre-commit Hook

**Location**: `.git/hooks/pre-commit`

**Automatic validation**: Runs before every commit

```bash
git commit -m "Your message"
# 🔍 Validating UTF-8 encoding of staged files...
# ✅ All staged files are properly UTF-8 encoded
```

**If validation fails**:
```
❌ UTF-8 validation failed for the following files:
  - packages/example/README.md

To fix:
  1. Open the files in an editor that supports UTF-8
  2. Check for invalid characters or encoding issues
  3. Re-save with UTF-8 encoding

Or run: yarn validate:utf8
```

**Override** (not recommended):
```bash
git commit --no-verify -m "Skip validation"
```

### 6. CI/CD Integration

**GitHub Actions** (`.github/workflows/ci.yml`):

```yaml
- name: Validate UTF-8 encoding
  run: yarn validate:utf8
```

- Runs on every push and pull request
- Prevents merging files with encoding issues
- Provides early feedback in PR checks

## Best Practices for Developers

### When Creating Files

1. **Set your editor to UTF-8**:
   - VS Code: `"files.encoding": "utf8"` (default)
   - WebStorm: Settings → Editor → File Encodings → UTF-8
   - Sublime: `"default_encoding": "UTF-8"`

2. **Verify EditorConfig support**:
   - VS Code: [EditorConfig extension](https://marketplace.visualstudio.com/items?itemName=EditorConfig.EditorConfig)
   - Most modern editors have built-in support

3. **Use proper characters**:
   ```
   ✓ Use: ├── │ └── → ✓ ✗
   ✗ Avoid: ASCII art alternatives or copy-paste from binary sources
   ```

### When Using Claude Code

Claude Code's Write tool automatically generates UTF-8 files. The safeguards ensure:
- Box drawing characters render correctly
- Arrows and symbols are preserved
- Emoji and international characters work
- No binary/control character corruption

### When Reviewing Pull Requests

Check for encoding issues:

```bash
# Review changed files
git diff main...feature-branch

# Validate encoding
yarn validate:utf8
```

The pre-commit hook and CI will catch most issues automatically.

## Troubleshooting

### Issue: "Invalid UTF-8 byte sequences"

**Cause**: File contains bytes that aren't valid UTF-8

**Fix**:
```bash
# Option 1: Re-save in your editor with UTF-8 encoding

# Option 2: Convert with iconv
iconv -f ISO-8859-1 -t UTF-8 file.md > file.md.utf8
mv file.md.utf8 file.md

# Option 3: Use Python to fix
python3 << 'EOF'
with open('file.md', 'rb') as f:
    content = f.read()

# Decode with error handling
text = content.decode('utf-8', errors='replace')

# Write back as UTF-8
with open('file.md', 'w', encoding='utf-8') as f:
    f.write(text)
EOF
```

### Issue: "Content changes when re-encoded"

**Cause**: File claims to be UTF-8 but has encoding inconsistencies

**Fix**: Same as above - re-save or convert to proper UTF-8

### Issue: Box drawing characters look wrong

**Symptoms**:
```
├── src/      (correct)
\x1c\x00 src/ (wrong - showing control chars)
+-- src/      (wrong - ASCII fallback)
```

**Fix**:
1. Ensure terminal/editor supports UTF-8
2. Check font supports box drawing characters
3. Verify file is actually UTF-8 (run `yarn validate:utf8`)

### Issue: Git shows encoding changes but content looks the same

**Cause**: File has mixed line endings or encoding markers

**Fix**:
```bash
# Normalize line endings
git add --renormalize .
```

## Implementation Checklist

When adding new text files to the project:

- [ ] File created with UTF-8 encoding
- [ ] `.editorconfig` applies to file type
- [ ] `.gitattributes` includes file extension
- [ ] `validate-utf8.mjs` pattern includes file type (if applicable)
- [ ] Pre-commit hook will validate file
- [ ] CI will catch encoding issues

## References

- [UTF-8 Specification (RFC 3629)](https://tools.ietf.org/html/rfc3629)
- [EditorConfig Documentation](https://editorconfig.org/)
- [Git Attributes Documentation](https://git-scm.com/docs/gitattributes)
- [Unicode Box Drawing Characters](https://en.wikipedia.org/wiki/Box-drawing_character)

## Summary

UTF-8 encoding is enforced at multiple levels:

1. **Editor** → EditorConfig ensures files are created as UTF-8
2. **Git** → `.gitattributes` prevents corruption during version control
3. **Pre-commit** → Hook validates before commits
4. **CI/CD** → GitHub Actions validates in pull requests
5. **Runtime** → Scripts explicitly use UTF-8 encoding

This multi-layered approach ensures UTF-8 compliance throughout the development lifecycle.
