use anyhow::{bail, Result};
use regex::Regex;

/// Represents a block of content within a Markdown file
#[derive(Debug, Clone)]
pub struct Block {
    pub start: usize,      // Start offset in bytes
    pub end: usize,        // End offset in bytes
    pub content: String,   // Full content including delimiters
    pub block_type: BlockType,
}

#[derive(Debug, Clone)]
pub enum BlockType {
    Paragraph,
    Heading { level: u8 },
    CodeBlock { lang: Option<String> },
    List { ordered: bool },
    BlockQuote,
    Table,
    Html,
    ThematicBreak,
}

/// Represents a section under a heading
#[derive(Debug)]
pub struct Section {
    pub heading: String,
    pub heading_level: u8,
    pub heading_start: usize,
    pub heading_end: usize,
    pub blocks: Vec<Block>,
}

/// Parse markdown content and find all sections
pub fn parse_sections(content: &str) -> Result<Vec<Section>> {
    let mut sections = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    
    let mut current_section: Option<Section> = None;
    let mut i = 0;
    let mut current_offset = 0;

    while i < lines.len() {
        let line = lines[i];
        let line_start = current_offset;
        let line_end = current_offset + line.len();
        
        // Check if this is a heading
        if let Some(caps) = Regex::new(r"^(#{1,6})\s+(.+)$").unwrap().captures(line) {
            let hashes = caps.get(1).unwrap().as_str();
            let level = hashes.len() as u8;
            let heading_text = format!("{} {}", hashes, caps.get(2).unwrap().as_str());

            // Close previous section
            if let Some(section) = current_section.take() {
                sections.push(section);
            }

            // Start new section
            current_section = Some(Section {
                heading: heading_text,
                heading_level: level,
                heading_start: line_start,
                heading_end: line_end,
                blocks: Vec::new(),
            });
        } else if let Some(ref mut section) = current_section {
            // Parse block in this section
            if let Some((block, next_i)) = parse_block(&lines, i, current_offset)? {
                section.blocks.push(block);
                // Adjust current_offset for next iteration
                let lines_consumed = next_i - i;
                for j in 0..lines_consumed {
                    current_offset += lines[i + j].len() + 1; // +1 for newline
                }
                i = next_i;
                continue;
            }
        }

        current_offset += line.len() + 1; // +1 for newline
        i += 1;
    }

    // Don't forget the last section
    if let Some(section) = current_section {
        sections.push(section);
    }

    Ok(sections)
}

/// Find a section by heading path, supporting nested headings
/// heading_path: ["# Parent", "## Child", "### GrandChild"]
/// 从第一个 heading 开始，逐级向下查找
pub fn find_section<'a>(sections: &'a [Section], heading_path: &[String]) -> Result<&'a Section> {
    if heading_path.is_empty() {
        bail!("Heading path cannot be empty");
    }

    // 第一级：找到所有匹配的顶级 heading
    let first_heading = heading_path[0].trim();
    let first_level = first_heading.chars().take_while(|&c| c == '#').count() as u8;
    
    let candidates: Vec<&Section> = sections
        .iter()
        .filter(|s| s.heading.trim() == first_heading)
        .collect();

    if candidates.is_empty() {
        bail!("Heading not found: {}", first_heading);
    }

    // 如果只找一级，但有多个匹配，报错提示歧义
    if heading_path.len() == 1 {
        if candidates.len() > 1 {
            bail!(
                "Multiple sections found for heading '{}'. Please provide a more specific path like '# Parent ## {}'.",
                first_heading, 
                first_heading.trim_start_matches('#').trim()
            );
        }
        return Ok(candidates[0]);
    }

    // 多级路径：需要按顺序找到匹配的嵌套结构
    // 由于 sections 是按文档顺序排列的，我们可以利用这一点
    let mut current_section = candidates[0];
    let mut section_idx = sections.iter().position(|s| s.heading == current_section.heading).unwrap();

    for i in 1..heading_path.len() {
        let target_heading = heading_path[i].trim();
        let target_level = target_heading.chars().take_while(|&c| c == '#').count() as u8;

        // 从当前 section 之后开始查找
        let mut found = false;
        for (idx, section) in sections.iter().enumerate().skip(section_idx + 1) {
            let section_level = section.heading.chars().take_while(|&c| c == '#').count() as u8;
            
            // 如果遇到同级的 heading，说明已经离开了当前 section 的范围
            if section_level <= first_level {
                break;
            }
            
            // 匹配目标 heading
            if section.heading.trim() == target_heading {
                current_section = section;
                section_idx = idx;
                found = true;
                break;
            }
        }

        if !found {
            bail!("Subheading not found: {}", target_heading);
        }
    }

    Ok(current_section)
}

/// Get a block by index within a section
pub fn get_block(section: &Section, index: usize) -> Result<&Block> {
    if index >= section.blocks.len() {
        bail!(
            "Block index {} out of range (section has {} blocks)",
            index,
            section.blocks.len()
        );
    }
    Ok(&section.blocks[index])
}

/// Parse a block starting at the given line
fn parse_block(
    lines: &[&str],
    start: usize,
    start_offset: usize,
) -> Result<Option<(Block, usize)>> {
    if start >= lines.len() {
        return Ok(None);
    }

    let line = lines[start].trim();

    // Skip empty lines
    if line.is_empty() {
        return Ok(None);
    }

    // Code block
    if line.starts_with("```") {
        return parse_code_block(lines, start, start_offset);
    }

    // Table
    if line.contains('|') {
        return parse_table(lines, start, start_offset);
    }

    // Block quote
    if line.starts_with('>') {
        return parse_block_quote(lines, start, start_offset);
    }

    // List
    if Regex::new(r"^([-*+]|\d+\.)\s").unwrap().is_match(line) {
        return parse_list(lines, start, start_offset);
    }

    // HTML block
    if line.starts_with('<') && !line.starts_with("<!--") {
        return parse_html_block(lines, start, start_offset);
    }

    // Thematic break
    if Regex::new(r"^([-*_]){3,}\s*$").unwrap().is_match(line) {
        let end_offset = start_offset + lines[start].len();
        return Ok(Some((
            Block {
                start: start_offset,
                end: end_offset,
                content: lines[start].to_string(),
                block_type: BlockType::ThematicBreak,
            },
            start + 1,
        )));
    }

    // Default: paragraph
    parse_paragraph(lines, start, start_offset)
}

fn parse_code_block(
    lines: &[&str],
    start: usize,
    start_offset: usize,
) -> Result<Option<(Block, usize)>> {
    let first_line = lines[start];
    let lang = first_line
        .trim_start_matches('`')
        .trim()
        .to_string();
    let lang = if lang.is_empty() { None } else { Some(lang) };

    let mut end = start + 1;
    let mut content = first_line.to_string();
    let mut current_offset = start_offset + first_line.len() + 1;

    while end < lines.len() {
        content.push('\n');
        content.push_str(lines[end]);
        
        if lines[end].trim() == "```" {
            current_offset += lines[end].len();
            break;
        }
        current_offset += lines[end].len() + 1;
        end += 1;
    }

    Ok(Some((
        Block {
            start: start_offset,
            end: current_offset,
            content,
            block_type: BlockType::CodeBlock { lang },
        },
        end + 1,
    )))
}

fn parse_table(
    lines: &[&str],
    start: usize,
    start_offset: usize,
) -> Result<Option<(Block, usize)>> {
    let mut end = start;
    let mut content = String::new();
    let mut current_offset = start_offset;

    while end < lines.len() {
        let line = lines[end];
        if !line.contains('|') && !line.trim().is_empty() {
            break;
        }
        if !content.is_empty() {
            content.push('\n');
            current_offset += 1;
        }
        content.push_str(line);
        current_offset += line.len();
        end += 1;

        // Empty line ends the table
        if line.trim().is_empty() {
            break;
        }
    }

    Ok(Some((
        Block {
            start: start_offset,
            end: current_offset,
            content,
            block_type: BlockType::Table,
        },
        end,
    )))
}

fn parse_block_quote(
    lines: &[&str],
    start: usize,
    start_offset: usize,
) -> Result<Option<(Block, usize)>> {
    let mut end = start;
    let mut content = String::new();
    let mut current_offset = start_offset;

    while end < lines.len() {
        let line = lines[end];
        if !line.starts_with('>') && !line.trim().is_empty() {
            break;
        }
        if !content.is_empty() {
            content.push('\n');
            current_offset += 1;
        }
        content.push_str(line);
        current_offset += line.len();
        end += 1;
    }

    Ok(Some((
        Block {
            start: start_offset,
            end: current_offset,
            content,
            block_type: BlockType::BlockQuote,
        },
        end,
    )))
}

fn parse_list(
    lines: &[&str],
    start: usize,
    start_offset: usize,
) -> Result<Option<(Block, usize)>> {
    let first_line = lines[start];
    let ordered = first_line.trim().chars().next().unwrap().is_ascii_digit();

    let mut end = start;
    let mut content = String::new();
    let mut current_offset = start_offset;

    while end < lines.len() {
        let line = lines[end];
        
        // Check if this is a new list item or continuation
        let is_list_item = Regex::new(r"^([-*+]|\d+\.)\s").unwrap().is_match(line.trim());
        let is_indented = line.starts_with("  ") || line.starts_with("\t") || line.trim().is_empty();

        if !is_list_item && !is_indented && !line.trim().is_empty() {
            break;
        }

        if !content.is_empty() {
            content.push('\n');
            current_offset += 1;
        }
        content.push_str(line);
        current_offset += line.len();
        end += 1;
    }

    Ok(Some((
        Block {
            start: start_offset,
            end: current_offset,
            content,
            block_type: BlockType::List { ordered },
        },
        end,
    )))
}

fn parse_html_block(
    lines: &[&str],
    start: usize,
    start_offset: usize,
) -> Result<Option<(Block, usize)>> {
    let mut end = start;
    let mut content = String::new();
    let mut current_offset = start_offset;
    let mut tag_stack = 0;

    // Simple HTML block parsing - just grab until we hit an empty line
    // or close the initial tag
    while end < lines.len() {
        let line = lines[end];
        if line.trim().is_empty() && tag_stack == 0 {
            break;
        }

        if !content.is_empty() {
            content.push('\n');
            current_offset += 1;
        }
        content.push_str(line);
        current_offset += line.len();

        // Very naive tag counting
        if line.contains('<') && !line.contains("</") {
            tag_stack += 1;
        }
        if line.contains("</") {
            tag_stack -= 1;
        }

        end += 1;
    }

    Ok(Some((
        Block {
            start: start_offset,
            end: current_offset,
            content,
            block_type: BlockType::Html,
        },
        end,
    )))
}

fn parse_paragraph(
    lines: &[&str],
    start: usize,
    start_offset: usize,
) -> Result<Option<(Block, usize)>> {
    let mut end = start;
    let mut content = String::new();
    let mut current_offset = start_offset;

    while end < lines.len() {
        let line = lines[end];
        if line.trim().is_empty() {
            break;
        }
        // Stop at certain block-starting patterns
        if line.starts_with("```") 
            || line.starts_with("#") 
            || line.starts_with(">")
            || Regex::new(r"^([-*+]|\d+\.)\s").unwrap().is_match(line)
            || Regex::new(r"^([-*_]){3,}\s*$").unwrap().is_match(line)
        {
            break;
        }

        if !content.is_empty() {
            content.push('\n');
            current_offset += 1;
        }
        content.push_str(line);
        current_offset += line.len();
        end += 1;
    }

    if content.is_empty() {
        Ok(None)
    } else {
        Ok(Some((
            Block {
                start: start_offset,
                end: current_offset,
                content,
                block_type: BlockType::Paragraph,
            },
            end,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_heading() {
        let content = "# Title\n\nSome paragraph.\n\n## Subtitle\n\nMore text.";
        let sections = parse_sections(content).unwrap();
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].heading, "# Title");
        assert_eq!(sections[0].blocks.len(), 1);
        assert_eq!(sections[1].heading, "## Subtitle");
    }

    #[test]
    fn test_parse_code_block() {
        let content = "# Title\n\n```rust\nfn main() {}\n```\n";
        let sections = parse_sections(content).unwrap();
        assert_eq!(sections[0].blocks.len(), 1);
        assert!(matches!(sections[0].blocks[0].block_type, BlockType::CodeBlock { .. }));
    }
}
