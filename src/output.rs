use clap::ValueEnum;
use colored::Colorize;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum OutputFormat {
    /// Unified diff format
    #[default]
    Diff,
    /// JSON format
    Json,
    /// Short summary
    Short,
}

/// 操作信息，用于 JSON 输出
#[derive(Debug, Clone)]
pub struct OperationInfo {
    pub file: PathBuf,
    pub heading: String,
    pub index: usize,
    pub operation: String,
}

/// 成功操作的 JSON 输出
#[derive(Serialize)]
struct JsonSuccessOutput {
    success: bool,
    applied: bool,
    is_noop: bool,
    changes: Vec<Change>,
}

/// 详细变更信息
#[derive(Serialize)]
struct Change {
    file: String,
    operation: String,
    heading: String,
    index: usize,
    status: String,
}

/// 错误 JSON 输出（Agent 可解析）
#[derive(Serialize)]
pub struct JsonErrorOutput {
    success: bool,
    error: ErrorDetail,
}

#[derive(Serialize)]
pub struct ErrorDetail {
    /// 错误类型代码
    pub code: String,
    /// 人类可读的错误信息
    pub message: String,
    /// 错误发生的上下文
    pub context: Option<ErrorContext>,
    /// 建议的修复操作
    pub suggestion: Option<String>,
}

#[derive(Serialize)]
pub struct ErrorContext {
    pub file: Option<String>,
    pub heading: Option<String>,
    pub index: Option<usize>,
    pub fingerprint: Option<String>,
}

/// 打印错误（支持 JSON 格式）
pub fn print_error(
    error: &anyhow::Error,
    format: OutputFormat,
    exit_code: i32,
    file: Option<&PathBuf>,
    heading: Option<&str>,
    index: Option<usize>,
) {
    match format {
        OutputFormat::Json => {
            let (code, message, suggestion) = classify_error_detail(error, exit_code);
            let error_output = JsonErrorOutput {
                success: false,
                error: ErrorDetail {
                    code: code.to_string(),
                    message: message.to_string(),
                    context: Some(ErrorContext {
                        file: file.map(|p| p.to_string_lossy().to_string()),
                        heading: heading.map(|s| s.to_string()),
                        index,
                        fingerprint: extract_fingerprint_from_error(error),
                    }),
                    suggestion: suggestion.map(|s| s.to_string()),
                },
            };
            eprintln!("{}", serde_json::to_string_pretty(&error_output).unwrap());
        }
        _ => {
            // 文本格式错误已经在 main 中打印
        }
    }
}

/// 分类错误并返回 (code, message, suggestion)
fn classify_error_detail(error: &anyhow::Error, exit_code: i32) -> (&'static str, String, Option<&'static str>) {
    let msg = error.to_string();
    match exit_code {
        2 => (
            "heading_not_found",
            msg.clone(),
            Some("Verify the heading exists or use nested path like '# Parent ## Child'"),
        ),
        3 => (
            "fingerprint_mismatch",
            msg.clone(),
            Some("The target block content has changed. Update fingerprint or verify the block index"),
        ),
        4 => (
            "ambiguous_heading",
            msg.clone(),
            Some("Multiple sections match. Use full path like '# Parent ## TargetHeading'"),
        ),
        _ => {
            if msg.contains("file") || msg.contains("not found") {
                ("file_not_found", msg, Some("Verify the file path exists"))
            } else {
                ("general_error", msg, None)
            }
        }
    }
}

/// 从错误信息中提取 fingerprint
fn extract_fingerprint_from_error(error: &anyhow::Error) -> Option<String> {
    let msg = error.to_string();
    // 尝试提取 fingerprint pattern
    if let Some(start) = msg.find("pattern '") {
        let rest = &msg[start + 9..];
        if let Some(end) = rest.find('\'') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

pub fn print_result(diff: &str, format: OutputFormat, applied: bool, is_noop: bool) {
    print_result_with_info(diff, format, applied, None, is_noop);
}

pub fn print_result_with_info(
    diff: &str,
    format: OutputFormat,
    applied: bool,
    op_info: Option<OperationInfo>,
    is_noop: bool,
) {
    match format {
        OutputFormat::Diff => print_diff(diff, is_noop),
        OutputFormat::Json => print_json(diff, applied, op_info, is_noop),
        OutputFormat::Short => print_short(diff, applied, is_noop),
    }
}

fn print_diff(diff: &str, is_noop: bool) {
    if is_noop {
        println!("{}", "(No changes - content already up to date)".dimmed());
        return;
    }

    for line in diff.lines() {
        if line.starts_with('+') && !line.starts_with("+++") {
            println!("{}", line.green());
        } else if line.starts_with('-') && !line.starts_with("---") {
            println!("{}", line.red());
        } else if line.starts_with("@") {
            println!("{}", line.cyan());
        } else {
            println!("{}", line);
        }
    }
}

fn print_json(_diff: &str, applied: bool, op_info: Option<OperationInfo>, is_noop: bool) {
    let (file, operation, heading, index) = match op_info {
        Some(info) => (
            info.file.to_string_lossy().to_string(),
            info.operation,
            info.heading,
            info.index,
        ),
        None => ("unknown".to_string(), "unknown".to_string(), "unknown".to_string(), 0),
    };

    let status = if is_noop {
        "noop"
    } else if applied {
        "applied"
    } else {
        "dry-run"
    };

    let output = JsonSuccessOutput {
        success: true,
        applied,
        is_noop,
        changes: vec![Change {
            file,
            operation,
            heading,
            index,
            status: status.to_string(),
        }],
    };

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

fn print_short(diff: &str, applied: bool, is_noop: bool) {
    if is_noop {
        println!("{}", "No changes".dimmed());
        return;
    }

    let additions = diff.lines().filter(|l| l.starts_with('+') && !l.starts_with("+++")).count();
    let deletions = diff.lines().filter(|l| l.starts_with('-') && !l.starts_with("---")).count();

    let status = if applied {
        "Applied".green()
    } else {
        "Planned".yellow()
    };

    println!("{}: +{} -{}", status, additions, deletions);
}

#[allow(dead_code)]
pub fn format_diff(old: &str, new: &str, filename: &str) -> String {
    // 如果内容相同，返回空 diff
    if old == new {
        return format!("--- a/{0}\n+++ b/{0}\n", filename);
    }

    let mut diff = format!("--- a/{0}\n+++ b/{0}\n", filename);

    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    // Find approximate change location
    let mut start = 0;
    while start < old_lines.len()
        && start < new_lines.len()
        && old_lines[start] == new_lines[start]
    {
        start += 1;
    }

    let mut old_end = old_lines.len();
    let mut new_end = new_lines.len();
    while old_end > start
        && new_end > start
        && old_lines[old_end - 1] == new_lines[new_end - 1]
    {
        old_end -= 1;
        new_end -= 1;
    }

    // Output context and changes
    let context_start = start.saturating_sub(3);

    diff.push_str(&format!(
        "@@ -{},{1} +{},{2} @@\n",
        context_start + 1,
        old_end.saturating_sub(context_start),
        new_end.saturating_sub(context_start)
    ));

    // Context before
    for i in context_start..start {
        diff.push_str(&format!(" {}\n", old_lines[i]));
    }

    // Deletions
    for i in start..old_end {
        diff.push_str(&format!("-{}\n", old_lines[i]));
    }

    // Additions
    for i in start..new_end {
        diff.push_str(&format!("+{}\n", new_lines[i]));
    }

    // Context after
    for i in old_end..(old_end + 3).min(old_lines.len()) {
        diff.push_str(&format!(" {}\n", old_lines[i]));
    }

    diff
}
