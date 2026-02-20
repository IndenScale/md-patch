# md-patch

CLI tool for declarative, idempotent Markdown block patching.

## Overview

`mdp` (md-patch) provides a structured way to modify Markdown files through
**semantic addressing** (path + heading + block_index) rather than fragile line
numbers. Designed with AI agents in mind, it supports:

- **Declarative operations**: append, replace, delete
- **Safety-first design**: fingerprint regex validation for destructive operations
- **Atomic batch operations**: YAML-configured multi-file patches
- **Non-destructive workflow**: dry-run mode (`plan`) before applying (`apply`)
- **Offset-based editing**: preserves original formatting (no AST round-trip)
- **Automatic backups**: `.bak` files created before destructive operations
- **Idempotency**: duplicate operations are detected and reported as no-ops

## Installation

```bash
# From source
git clone https://github.com/IndenScale/md-patch.git
cd md-patch
cargo build --release

# Binary will be at target/release/mdp
cp target/release/mdp ~/.local/bin/
```

## Quick Start

### Single Patch Operation

```bash
# Append content after a specific block (dry-run)
mdp patch -f doc.md -H "## Getting Started" -i 0 --op append -c "New content here"

# Replace with fingerprint validation
mdp patch -f doc.md -H "# API" -i 1 --op replace \
  -c "New description" \
  --fingerprint "old.*pattern" \
  --force

# Delete a block
mdp patch -f doc.md -H "## Deprecated" -i 0 --op delete \
  --fingerprint "deprecated.*section" \
  --force

# Skip backup creation
mdp patch -f doc.md -H "## Section" --op replace -c "New" --force --no-backup
```

### Batch Operations via YAML

Create `patch.yaml`:

```yaml
operations:
  - file: docs/api.md
    heading:
      - "# API Reference"
    index: 0
    operation: append
    content: "\nNew endpoint documentation."

  - file: docs/auth.md
    heading:
      - "# Authentication"
      - "## OAuth2"
    index: 0
    operation: replace
    content: "Use PKCE flow for all applications."
    fingerprint: "implicit.*flow"
```

Run:

```bash
# Preview changes
mdp plan patch.yaml

# Apply changes
mdp apply patch.yaml --force

# Apply without creating backups
mdp apply patch.yaml --force --no-backup
```

## Addressing Model

```text
file → heading_path → block_index
       (e.g., "# Title ## Subtitle")  (0-based)
```

- **Heading path**: Space-separated headings from top-level to target
- **Block index**: Position within the section (0 = first block after heading)

## Safety Features

| Feature | Description |
|---------|-------------|
| `--force` | Required for replace/delete operations |
| `--fingerprint` | Regex validation before destructive changes |
|                 | (cannot be bypassed by --force) |
| `--no-backup` | Skip creating `.bak` backup files |
| `plan` command | Dry-run mode showing diff without applying |
| Atomic writes | Temp file + rename for crash safety |
| Automatic backups | `.bak` files created before any file modification |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success (including noop) |
| 1 | General error / file not found |
| 2 | Heading not found |
| 3 | Fingerprint mismatch |
| 4 | Ambiguous heading match |

## JSON Output

For programmatic integration:

```bash
mdp patch -f doc.md -H "## Section" --op append -c "New" --force -F json
```

Output format:

```json
{
  "success": true,
  "applied": true,
  "is_noop": false,
  "changes": [
    {
      "file": "doc.md",
      "operation": "append",
      "heading": "## Section",
      "index": 0,
      "status": "applied"
    }
  ]
}
```

Status values:

- `"applied"` - Changes were successfully applied
- `"noop"` - No changes needed (idempotent operation)
- `"dry-run"` - Preview mode, no changes made

## Agent Integration

Designed for AI agent workflows:

```bash
# Agent checks what would change
mdp plan operations.yaml --format json

# Agent validates then applies
mdp apply operations.yaml --force --format json

# Check if operation was noop (idempotent)
mdp patch -f doc.md -H "## Section" --op append -c "Existing" --force -F json | jq '.is_noop'
```

JSON output enables programmatic error handling and decision making.

## Design Philosophy

### Force vs Fingerprint

`--force` and `--fingerprint` serve different purposes:

- **Fingerprint**: Content validation ("Is this the right block?")
  - Fingerprint mismatch = exit code 3
  - **Cannot** be bypassed by `--force` (it's a locator, not a permission)
  
- **Force**: Execution permission ("I accept the risk")
  - Required for destructive operations without fingerprint
  - Bypasses safety check but **not** fingerprint validation

```bash
# ✓ Works: fingerprint validates content
mdp patch ... --fingerprint "TODO.*" --force

# ✓ Works: force accepts risk without fingerprint  
mdp patch ... --force

# ✗ Fails: fingerprint mismatch (force doesn't help)
mdp patch ... --fingerprint "WRONG" --force  # exit code 3
```

### Backup Strategy

By default, `mdp` creates `.bak` backups before any file modification:

1. Original content copied to `file.bak`
2. Changes written to temp file
3. Temp file atomically renamed to target

Use `--no-backup` to skip backup creation (e.g., in CI/CD with read-only filesystems).

## License

MIT
