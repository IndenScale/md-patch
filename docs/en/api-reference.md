# API Reference

Complete reference for `mdp` command-line interface.

## Command Overview

```text
mdp <COMMAND> [OPTIONS]
```

| Command | Description |
|---------|-------------|
| `patch` | Apply a single patch operation |
| `apply` | Apply patches from YAML configuration file |
| `plan` | Preview changes without applying (dry-run) |
| `help` | Print help message |

---

## `mdp patch`

Execute a patch operation on a single Markdown file.

### Usage

```text
mdp patch [OPTIONS] --file <FILE> --heading <HEADING> --op <OPERATION>
```

### Required Arguments

| Argument | Short | Description |
|----------|-------|-------------|
| `--file` | `-f` | Target Markdown file path |
| `--heading` | `-H` | Heading path (e.g., `"# Title ## Subtitle"`) |
| `--op` | `-o` | Operation type: `append`, `replace`, `delete` |

### Optional Arguments

| Argument | Short | Description |
|----------|-------|-------------|
| `--index` | `-i` | Block index (default: 0) |
| `--content` | `-c` | Content to append or replace |
| `--fingerprint` | `-p` | Fingerprint regex for validation |
| `--force` | none | Confirm destructive operation |
| `--no-backup` | none | Skip creating `.bak` backup |
| `--format` | `-F` | Output format: `text`, `diff`, `json` |

### Examples

#### Append Content

```bash
# Basic append
mdp patch -f doc.md -H "## Features" --op append -c "- New feature" --force

# Append to specific index
mdp patch -f doc.md -H "## Features" -i 1 --op append -c "More content" --force

# Multi-level headings
mdp patch -f doc.md -H "# Document ## Chapter ### Section" --op append \
  -c "Section content" --force
```

#### Replace Content

```bash
# Safe replace with fingerprint
mdp patch -f doc.md -H "## API" --op replace \
  -c "New version" \
  --fingerprint "Old version.*" \
  --force

# Without fingerprint (force only)
mdp patch -f doc.md -H "## API" --op replace \
  -c "New version" \
  --force
```

#### Delete Content

```bash
# Safe delete - validate content
mdp patch -f doc.md -H "## Deprecated" --op delete \
  --fingerprint "This feature is deprecated" \
  --force
```

#### JSON Output

```bash
mdp patch -f doc.md -H "## Section" --op append -c "content" --force -F json
```

Output:

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

---

## `mdp apply`

Apply batch patches from a YAML configuration file.

### Usage

```text
mdp apply [OPTIONS] <CONFIG_FILE>
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<CONFIG_FILE>` | YAML configuration file path |

### Options

| Option | Description |
|--------|-------------|
| `--force` | Confirm all destructive operations |
| `--no-backup` | Skip creating backup files |
| `--format <FORMAT>` | Output format: `text`, `diff`, `json` |

### YAML Configuration Format

```yaml
operations:
  - file: path/to/file.md
    heading:
      - "# Level 1 Heading"
      - "## Level 2 Heading"
    index: 0
    operation: append|replace|delete
    content: "Content to insert"
    fingerprint: "Validation regex"
```

### Field Descriptions

| Field | Required | Description |
|-------|----------|-------------|
| `file` | Yes | Target file path (relative or absolute) |
| `heading` | Yes | Heading path array |
| `index` | No | Block index (default: 0) |
| `operation` | Yes | Operation type: `append`, `replace`, `delete` |
| `content` | Conditional | Required for `append` and `replace` |
| `fingerprint` | No | Content validation regex |

### Example Configuration

```yaml
operations:
  # Append operation
  - file: docs/guide.md
    heading:
      - "# User Guide"
      - "## Installation"
    index: 0
    operation: append
    content: "\n### Install from Source\n\n```bash\ngit clone ...\n```"

  # Replace operation
  - file: docs/api.md
    heading:
      - "# API Reference"
    index: 1
    operation: replace
    content: "New API description"
    fingerprint: "Old API.*"

  # Delete operation
  - file: docs/legacy.md
    heading:
      - "# Deprecated Features"
    index: 0
    operation: delete
    fingerprint: "This feature is no longer maintained"
```

### Execution

```bash
# Basic apply
mdp apply patches.yaml --force

# JSON output
mdp apply patches.yaml --force --format json

# No backups
mdp apply patches.yaml --force --no-backup
```

---

## `mdp plan`

Preview changes without actually applying them (dry-run mode).

### Usage

```text
mdp plan [OPTIONS] <CONFIG_FILE>
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<CONFIG_FILE>` | YAML configuration file path |

### Options

| Option | Description |
|--------|-------------|
| `--format <FORMAT>` | Output format: `text`, `diff`, `json` |
|                     | (default: `diff`) |

### Examples

```bash
# View diff
mdp plan patches.yaml

# JSON format preview
mdp plan patches.yaml --format json

# Text summary
mdp plan patches.yaml --format text
```

### Diff Output Example

```diff
--- a/docs/guide.md
+++ b/docs/guide.md
@@ -10,6 +10,10 @@
 
 ## Installation
 
+### Install from Source
+
+```bash
+git clone ...
+```
+
 Install using package manager.
```

---

## Exit Codes

| Code | Constant | Meaning |
|------|----------|---------|
| 0 | `EXIT_SUCCESS` | Success (including no-op) |
| 1 | `EXIT_ERROR` | General error (file not found, permission, etc.) |
| 2 | `EXIT_HEADING_NOT_FOUND` | Specified heading path not found |
| 3 | `EXIT_FINGERPRINT_MISMATCH` | Fingerprint validation failed |
| 4 | `EXIT_AMBIGUOUS_HEADING` | Ambiguous heading match (multiple matches) |

### Usage in Scripts

```bash
#!/bin/bash

mdp patch -f doc.md -H "## Section" --op append -c "content" --force
exit_code=$?

if [ $exit_code -eq 0 ]; then
    echo "✓ Patch applied successfully"
elif [ $exit_code -eq 3 ]; then
    echo "✗ Fingerprint mismatch, operation cancelled"
    exit 1
else
    echo "✗ Error: exit code $exit_code"
    exit 1
fi
```

---

## Output Formats

### Text Format

Human-readable status summary:

```text
✓ Applied: doc.md ## Section [append]
✓ No-op: doc.md ## Other [append] (content already exists)
```

### Diff Format

Unified diff output for viewing specific changes:

```diff
--- a/file.md
+++ b/file.md
@@ -5,6 +5,8 @@
 
 ## Section
 
+Newly appended content
+
 Original content
```

### JSON Format

Structured output for programmatic processing:

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

#### JSON Field Descriptions

| Field | Type | Description |
|-------|------|-------------|
| `success` | boolean | Whether operation succeeded (no errors) |
| `applied` | boolean | Whether any changes were applied |
| `is_noop` | boolean | Whether it was a no-op (idempotent) |
| `changes` | array | List of change details |
| `changes[].file` | string | File path |
| `changes[].operation` | string | Operation type |
| `changes[].heading` | string | Heading path |
| `changes[].index` | number | Block index |
| `changes[].status` | string | Status: `applied`, `noop`, `dry-run` |

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `MDP_NO_COLOR` | Disable colored output |
| `MDP_BACKUP_DIR` | Backup file directory (default: same as target file) |
