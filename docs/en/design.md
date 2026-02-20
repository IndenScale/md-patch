# Design Document

Deep dive into `md-patch` design philosophy and implementation details.

## Core Design Goals

### 1. Designed for AI Agents

Traditional text editing tools use line numbers, which are too fragile for AI.
`md-patch` uses **semantic addressing**:

```text
❌ Fragile: Insert at line 42
✓ Robust: Append after "## API" section
```

### 2. Safety First

Destructive operations require explicit confirmation:

- **Fingerprint**: Verify you're modifying the intended content
- **Force Flag**: Confirm you accept the risk of changes
- **Automatic Backup**: Save a copy before making changes

### 3. Idempotency

Running the same operation multiple times should produce the same result:

```bash
# First execution
mdp patch -f doc.md -H "## Features" --op append -c "- X" --force
# → Content appended

# Second execution (same command)
mdp patch -f doc.md -H "## Features" --op append -c "- X" --force
# → Detected as already exists, no-op
```

## Architecture Overview

```text
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   CLI Input │────▶│   Parser    │────▶│   Locator   │
└─────────────┘     └─────────────┘     └──────┬──────┘
                                                │
                                                ▼
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Output    │◀────│   Engine    │◀────│   Validator │
│  Formatter  │     │  Executor   │     │             │
└─────────────┘     └─────────────┘     └─────────────┘
```

### Component Responsibilities

| Component | Responsibility |
|-----------|----------------|
| CLI Input | Argument parsing, validation |
| Parser | Markdown parsing, block identification |
| Locator | Heading path resolution, block index location |
| Validator | Fingerprint matching, safety checks |
| Engine | Apply patches, atomic writes |
| Formatter | Generate diff/JSON/text output |

## Addressing Model Deep Dive

### Document Structure

Markdown documents are viewed as a **section tree**:

```markdown
# Document Title                    ← Level 1

## Chapter 1                        ← Level 2

Content block 1

Content block 2

### Section 1                       ← Level 3

Content block 3

## Chapter 2                        ← Level 2

Content block 4
```

### Heading Path Resolution

Heading path is the route from root to target:

```rust
// Input
heading: "# Document Title ## Chapter 1"

// Parsed as
[
    Heading { level: 1, text: "Document Title" },
    Heading { level: 2, text: "Chapter 1" }
]
```

### Block Index

Content under each section is divided into **blocks**:

```markdown
## Section

[Block 0] First paragraph of text...

[Block 1] Second paragraph of text...

[Block 2] - List item 1
          - List item 2
          - List item 3

[Block 3] ```code
          fn main() {}
          ```
```

Block boundaries are defined by:

- Empty lines between paragraphs
- Standalone elements like lists, code blocks
- Headings (as section boundaries)

## Safety Mechanisms

### Fingerprint Validation

Fingerprint is **regex validation** of content:

```bash
# Only replace if content matches "Old version.*"
mdp patch -f doc.md -H "## API" --op replace \
  -c "New version" \
  --fingerprint "Old version.*"
```

Validation flow:

```text
1. Locate target block
2. Extract current content
3. Apply fingerprint regex
4. Match? → Continue operation
   No match? → Exit code 3
```

### Force Flag

Force is **permission confirmation**, not validation:

| Scenario | Result |
|----------|--------|
| Has fingerprint, matches | ✓ Success |
| Has fingerprint, no match | ✗ Fail (force ineffective) |
| No fingerprint, has force | ✓ Success (accept risk) |
| No fingerprint, no force | ✗ Fail (force required) |

### Backup Strategy

Atomic write flow:

```text
1. Read original file → memory
2. Create backup file → file.bak
3. Write temp file → file.tmp.XXXX
4. Atomic rename → file.tmp.XXXX → file
```

If step 3 fails, original file is unaffected.
If step 4 fails, backup file can be used for recovery.

## Idempotency Implementation

### Append Idempotency

Check if content already exists before appending:

```rust
fn apply_append(content: &str, block: &Block, new_content: &str) -> Result<String> {
    // Idempotency check
    if content[block.start..block.end].contains(new_content) {
        return Ok(content.to_string()); // No-op
    }
    // ... perform append
}
```

### Replace Idempotency

Don't write if replacement content is the same:

```rust
fn apply_replace(content: &str, block: &Block, new_content: &str) -> Result<String> {
    let current = &content[block.start..block.end];
    if current.trim() == new_content.trim() {
        return Ok(content.to_string()); // No-op
    }
    // ... perform replace
}
```

### Delete Idempotency

No-op if target block doesn't exist.

## Output Format Design

### Diff Format

Uses unified diff format:

```diff
--- a/file.md
+++ b/file.md
@@ -10,3 +10,7 @@
 ## Section
 
+New content 1
+
+New content 2
+
 Original content
```

### JSON Format

Structured output for programmatic integration:

```json
{
  "success": true,
  "applied": true,
  "is_noop": false,
  "changes": [...]
}
```

### Agent Integration Pattern

```bash
# 1. Agent plans changes
plan_result=$(mdp plan patches.yaml --format json)

# 2. Parse plan result, decide on action
if echo "$plan_result" | jq -e '.success' > /dev/null; then
    # 3. Apply changes
    apply_result=$(mdp apply patches.yaml --force --format json)
    
    # 4. Verify result
    if echo "$apply_result" | jq -e '.is_noop' > /dev/null; then
        echo "No changes needed"
    else
        echo "Changes applied"
    fi
fi
```

## Design Trade-offs

### Offset-based vs AST

| Approach | Pros | Cons |
|----------|------|------|
| **Offset-based** (current) | Preserves original formatting | Limited support for complex structures |
| AST | Structural correctness | Loses original formatting |

`md-patch` chose offset-based because for Agent tools, preserving user
formatting is more important than enforcing structural correctness.

### Fingerprint vs Hash

| Approach | Pros | Cons |
|----------|------|------|
| **Regex fingerprint** (current) | Flexible, tolerates small changes | May false positive |
| Content hash | Exact match | Fails on any change |

Chose regex fingerprint to support scenarios like version number changes.

### Single-file vs Batch

`patch` command for interactive use, `apply` for batch processing.
Separated design makes arguments simpler for each command.

## Future Extensions

### Possible Enhancements

1. **Conditional Operations**: `when: content matches "..."`
2. **Template Support**: Variable substitution in content
3. **Plugin System**: Custom operation types
4. **Concurrent Batch Operations**: Parallel processing of multiple files

### Compatibility Considerations

- YAML format version control
- Command-line interface stability
- JSON output field backward compatibility
