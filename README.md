# md-patch

CLI tool for declarative, idempotent Markdown block patching.

## Overview

`mdp` (md-patch) provides a structured way to modify Markdown files through **semantic addressing** (path + heading + block_index) rather than fragile line numbers. Designed with AI agents in mind, it supports:

- **Declarative operations**: append, replace, delete
- **Safety-first design**: fingerprint regex validation for destructive operations
- **Atomic batch operations**: YAML-configured multi-file patches
- **Non-destructive workflow**: dry-run mode (`plan`) before applying (`apply`)
- **Offset-based editing**: preserves original formatting (no AST round-trip)

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
| `plan` command | Dry-run mode showing diff without applying |
| Atomic writes | Temp file + rename for crash safety |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error / file not found |
| 2 | Heading not found |
| 3 | Fingerprint mismatch |
| 4 | Ambiguous heading match |

## Agent Integration

Designed for AI agent workflows:

```bash
# Agent checks what would change
mdp plan operations.yaml --format json

# Agent validates then applies
mdp apply operations.yaml --force --format json
```

JSON output enables programmatic error handling and decision making.

## License

MIT
