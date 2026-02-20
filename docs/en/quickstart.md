# Quick Start

Get started with `mdp` in minutes.

## Installation

### Build from Source

```bash
# Clone the repository
git clone https://github.com/IndenScale/md-patch.git
cd md-patch

# Build release version
cargo build --release

# Install to system path
cp target/release/mdp ~/.local/bin/
```

### Verify Installation

```bash
mdp --version
```

## Your First Patch

Create a test document:

```bash
cat > hello.md << 'EOF'
# Hello World

## Introduction

This is the introduction.

## Features

- Basic feature
EOF
```

### 1. Append Content

Append a new feature to the "## Features" section:

```bash
# Dry-run - see what will happen
mdp patch -f hello.md -H "## Features" --op append -c "- New feature" --format diff

# Actually apply
mdp patch -f hello.md -H "## Features" --op append -c "- New feature" --force
```

### 2. Replace Content

Replace the "## Introduction" section:

```bash
mdp patch -f hello.md -H "## Introduction" \
  --op replace \
  -c "This is the updated introduction." \
  --fingerprint "This is the introduction." \
  --force
```

### 3. Delete Content

Delete the entire "## Features" section:

```bash
mdp patch -f hello.md -H "## Features" \
  --op delete \
  --fingerprint "- Basic feature" \
  --force
```

## Batch Operations

For multiple file modifications, use a YAML configuration file.

Create `batch-patch.yaml`:

```yaml
operations:
  - file: hello.md
    heading:
      - "# Hello World"
      - "## Introduction"
    index: 0
    operation: append
    content: "\nMore introduction content."

  - file: hello.md
    heading:
      - "# Hello World"
      - "## Features"
    index: 0
    operation: replace
    content: "- Feature A\n- Feature B"
    fingerprint: "- New feature"
```

Execute batch operation:

```bash
# Preview
mdp plan batch-patch.yaml

# Apply
mdp apply batch-patch.yaml --force
```

## Addressing Explained

### Heading Path Format

A heading path is the route from document root to target section:

```markdown
# Document Title

## Chapter 1

### Section 1

Content 1

### Section 2

Content 2

## Chapter 2

Content 3
```

Addressing examples:

| Target | Heading Path | Block Index |
|--------|--------------|-------------|
| Content after Chapter 1 | `# Document Title ## Chapter 1` | 0 |
| Section 1 | `# Document Title ## Chapter 1 ### Section 1` | 0 |
| Section 2 | `# Document Title ## Chapter 1 ### Section 2` | 0 |
| Chapter 2 | `# Document Title ## Chapter 2` | 0 |

### Block Index

Block index indicates position within a section, starting from 0:

```markdown
## Section

[Block 0] First paragraph

[Block 1] Second paragraph

[Block 2] - List item 1
          - List item 2
```

- Index 0 points to "First paragraph"
- Index 1 points to "Second paragraph"
- Index 2 points to the entire list

## Safety Best Practices

### 1. Always Use `plan` First

```bash
mdp plan config.yaml --format diff
```

### 2. Use Fingerprint for Destructive Operations

```bash
# Good practice - validate content before replacing
mdp patch -f doc.md -H "## API" \
  --op replace \
  -c "New API" \
  --fingerprint "Old API name" \
  --force
```

### 3. Check Exit Codes

```bash
mdp patch -f doc.md -H "## Section" --op append -c "content" --force
exit_code=$?

case $exit_code in
  0) echo "Success" ;;
  1) echo "Error: General error" ;;
  2) echo "Error: Heading not found" ;;
  3) echo "Error: Fingerprint mismatch" ;;
  4) echo "Error: Ambiguous heading match" ;;
esac
```

### 4. Keep Backups

Backup `.bak` files are created automatically. To restore:

```bash
cp doc.md.bak doc.md
```

## Next Steps

- Read [API Reference](api-reference.md) for all commands and options
- Read [Design Document](design.md) for design principles
