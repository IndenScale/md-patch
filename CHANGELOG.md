# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] - 2026-02-20

### Initial Release

This is the first official release of `md-patch`, a declarative, idempotent Markdown block patching tool designed for AI agents and automation workflows.

#### Core Features

- **Semantic Addressing Model**: `file → heading_path → block_index` three-layer addressing
  - Heading path supports nested headings: `"# Title ## Subtitle ### Section"`
  - Block index for precise positioning within sections
  - Disambiguation for duplicate headings via full path

- **Idempotent Operations**: All operations (append/replace/delete) can be safely repeated
  - Append: Detects if content already exists
  - Replace: No-op if content is identical
  - Delete: No-op if target already removed

- **Safety-First Design**:
  - **Fingerprint validation**: Regex-based content verification (cannot be bypassed)
  - **Force flag**: Explicit authorization for destructive operations
  - **Automatic backups**: `.bak` files created before modifications
  - **Atomic writes**: Temp file + rename pattern for crash safety

- **Terraform-Style Workflow**:
  - `mdp plan <config.yaml>`: Preview changes (dry-run mode)
  - `mdp apply <config.yaml>`: Apply batch operations
  - Unified diff output for human review

- **Programmatic Integration**:
  - JSON output format with structured error information
  - Semantic exit codes (0=success, 2=heading not found, 3=fingerprint mismatch, 4=ambiguous heading)
  - `is_noop` field for detecting idempotent results

#### Commands

- `mdp patch`: Single-file single-operation patching
- `mdp apply`: Batch operations from YAML configuration
- `mdp plan`: Dry-run preview of batch operations

#### Supported Block Types

- Paragraphs
- Headings (levels 1-6)
- Code blocks (fenced)
- Lists (ordered/unordered)
- Block quotes
- Tables
- HTML blocks
- Thematic breaks

#### Technical Highlights

- **Offset-based editing**: Preserves original formatting (no AST round-trip)
- **LCS diff algorithm**: Unified diff output similar to git
- **Rust implementation**: Single binary, zero runtime dependencies
- **Cross-platform**: Linux, macOS, Windows

#### Installation Methods

```bash
# Quick install (Linux/macOS)
curl -fsSL https://raw.githubusercontent.com/IndenScale/md-patch/main/install.sh | sh

# Cargo
cargo install md-patch

# From source
git clone https://github.com/IndenScale/md-patch.git
cd md-patch && cargo build --release
```

---

## Release Notes Format

### Types of changes

- `Added` for new features
- `Changed` for changes in existing functionality
- `Deprecated` for soon-to-be removed features
- `Removed` for now removed features
- `Fixed` for any bug fixes
- `Security` in case of vulnerabilities

[Unreleased]: https://github.com/IndenScale/md-patch/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/IndenScale/md-patch/releases/tag/v0.1.1
