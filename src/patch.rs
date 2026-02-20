use anyhow::{bail, Result};
use regex::Regex;
use std::path::PathBuf;

use crate::parser::{find_section, get_block, parse_sections, Block};

#[derive(Clone, Copy, Debug)]
pub enum Operation {
    Append,
    Replace,
    Delete,
}

impl From<crate::config::OperationType> for Operation {
    fn from(op: crate::config::OperationType) -> Self {
        match op {
            crate::config::OperationType::Append => Operation::Append,
            crate::config::OperationType::Replace => Operation::Replace,
            crate::config::OperationType::Delete => Operation::Delete,
        }
    }
}

#[derive(Debug)]
pub struct PatchOperation {
    pub file: PathBuf,
    pub heading_path: Vec<String>,
    pub block_index: usize,
    pub operation: Operation,
    pub content: Option<String>,
    pub fingerprint: Option<String>,
}

pub enum PatchResult {
    Applied { new_content: String, diff: String },
    DryRun { diff: String },
}

pub fn apply_operation(
    content: &str,
    operation: &PatchOperation,
    force: bool,
) -> Result<PatchResult> {
    // Parse the markdown to find sections and blocks
    let sections = parse_sections(content)?;

    // Find the target section
    let section = find_section(&sections, &operation.heading_path)?;

    // Get the target block
    let block = get_block(section, operation.block_index)?;

    // Validate fingerprint if provided (for Replace/Delete)
    if let Some(ref fingerprint) = operation.fingerprint {
        let regex = Regex::new(fingerprint)?;
        if !regex.is_match(&block.content) {
            bail!(
                "Fingerprint mismatch: expected pattern '{}' not found in block content",
                fingerprint
            );
        }
    }

    // Require fingerprint for destructive operations without --force
    match operation.operation {
        Operation::Replace | Operation::Delete if operation.fingerprint.is_none() && !force => {
            bail!(
                "Destructive operation requires --force flag or fingerprint for safety. \
                 Provide a fingerprint pattern to verify the target block."
            );
        }
        _ => {}
    }

    // Generate the new content
    let new_content = match operation.operation {
        Operation::Append => apply_append(content, block, operation.content.as_deref())?,
        Operation::Replace => apply_replace(content, block, operation.content.as_deref())?,
        Operation::Delete => apply_delete(content, block)?,
    };

    // Generate diff - clean filename for display (remove leading ./ or /)
    let filename = operation.file.to_string_lossy();
    let clean_filename = filename.trim_start_matches("./").trim_start_matches('/');
    let diff = generate_diff(content, &new_content, clean_filename);

    if force {
        Ok(PatchResult::Applied { new_content, diff })
    } else {
        Ok(PatchResult::DryRun { diff })
    }
}

fn apply_append(content: &str, block: &Block, new_content: Option<&str>) -> Result<String> {
    let insert_content = match new_content {
        Some(c) => c,
        None => bail!("Append operation requires content"),
    };

    // 幂等性检查：如果内容已存在，直接返回原内容
    let block_and_after = &content[block.start..];
    if block_and_after.contains(insert_content) {
        return Ok(content.to_string());
    }

    let before = &content[..block.end];
    let after = &content[block.end..];

    // 确保追加内容前有换行，且与后续内容有适当分隔
    let insert_with_newline = format!("\n{}\n", insert_content);

    Ok(format!("{}{}{}", before, insert_with_newline, after))
}

fn apply_replace(content: &str, block: &Block, new_content: Option<&str>) -> Result<String> {
    let replacement = match new_content {
        Some(c) => c,
        None => bail!("Replace operation requires content"),
    };

    let before = &content[..block.start];
    let after = &content[block.end..];

    Ok(format!("{}{}{}", before, replacement, after))
}

fn apply_delete(content: &str, block: &Block) -> Result<String> {
    let before = &content[..block.start];
    let after = &content[block.end..];

    // Clean up extra newlines that might result from deletion
    let result = format!("{}{}", before, after);
    
    // Remove consecutive blank lines caused by deletion
    let cleaned = Regex::new(r"\n{3,}")?.replace_all(&result, "\n\n");
    
    Ok(cleaned.to_string())
}

fn generate_diff(original: &str, modified: &str, filename: &str) -> String {


    // Simple line-based diff
    let original_lines: Vec<&str> = original.lines().collect();
    let modified_lines: Vec<&str> = modified.lines().collect();

    let mut diff = format!("--- a/{}\n+++ b/{}\n", filename, filename);

    // Use a simple LCS-based diff
    let lcs = compute_lcs(&original_lines, &modified_lines);

    let mut i = 0;
    let mut j = 0;
    let mut lcs_idx = 0;

    while i < original_lines.len() || j < modified_lines.len() {
        if lcs_idx < lcs.len() {
            if i < original_lines.len() 
                && j < modified_lines.len()
                && original_lines[i] == modified_lines[j]
                && original_lines[i] == lcs[lcs_idx]
            {
                // Unchanged line
                diff.push_str(&format!(" {}\n", original_lines[i]));
                i += 1;
                j += 1;
                lcs_idx += 1;
            } else if i < original_lines.len() 
                && (lcs_idx >= lcs.len() || original_lines[i] != lcs[lcs_idx])
            {
                // Deleted line
                diff.push_str(&format!("-{}\n", original_lines[i]));
                i += 1;
            } else {
                // Added line
                diff.push_str(&format!("+{}\n", modified_lines[j]));
                j += 1;
            }
        } else if i < original_lines.len() {
            // Remaining deletions
            diff.push_str(&format!("-{}\n", original_lines[i]));
            i += 1;
        } else {
            // Remaining additions
            diff.push_str(&format!("+{}\n", modified_lines[j]));
            j += 1;
        }
    }

    diff
}

fn compute_lcs<'a>(a: &[&'a str], b: &[&'a str]) -> Vec<&'a str> {
    let m = a.len();
    let n = b.len();
    
    if m == 0 || n == 0 {
        return Vec::new();
    }

    // Use dynamic programming for LCS
    let mut dp = vec![vec![0; n + 1]; m + 1];

    for i in 1..=m {
        for j in 1..=n {
            if a[i - 1] == b[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    // Backtrack to find LCS
    let mut lcs = Vec::new();
    let mut i = m;
    let mut j = n;

    while i > 0 && j > 0 {
        if a[i - 1] == b[j - 1] {
            lcs.push(a[i - 1]);
            i -= 1;
            j -= 1;
        } else if dp[i - 1][j] > dp[i][j - 1] {
            i -= 1;
        } else {
            j -= 1;
        }
    }

    lcs.reverse();
    lcs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_append() {
        let content = "# Title\n\nFirst paragraph.\n\nSecond paragraph.\n";
        let block = Block {
            start: 10,
            end: 27,
            content: "First paragraph.".to_string(),
            block_type: crate::parser::BlockType::Paragraph,
        };
        
        let result = apply_append(content, &block, Some("New content")).unwrap();
        assert!(result.contains("First paragraph.\nNew content"));
    }

    #[test]
    fn test_apply_replace() {
        let content = "# Title\n\nOld content.\n\nOther text.\n";
        let block = Block {
            start: 10,
            end: 23,
            content: "Old content.".to_string(),
            block_type: crate::parser::BlockType::Paragraph,
        };
        
        let result = apply_replace(content, &block, Some("New content.")).unwrap();
        assert!(result.contains("New content."));
        assert!(!result.contains("Old content."));
    }

    #[test]
    fn test_apply_delete() {
        let content = "# Title\n\nDelete me.\n\nKeep me.\n";
        let block = Block {
            start: 10,
            end: 21,
            content: "Delete me.".to_string(),
            block_type: crate::parser::BlockType::Paragraph,
        };
        
        let result = apply_delete(content, &block).unwrap();
        assert!(!result.contains("Delete me."));
        assert!(result.contains("Keep me."));
    }
}
