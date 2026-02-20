use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

mod config;
mod output;
mod parser;
mod patch;

use config::{load_config, OperationConfig};
use output::{OutputFormat, OperationInfo};
use patch::{PatchOperation, PatchResult};

/// CLI tool for declarative, idempotent Markdown block patching
#[derive(Parser)]
#[command(name = "mdp")]
#[command(about = "Declarative, idempotent Markdown block patching tool")]
#[command(version)]
#[command(after_help = "EXAMPLES:
    # 在章节后添加内容 (block_index = 0 表示 heading 后的第一个内容块)
    mdp patch -f doc.md -H '## 功能特性' -i 0 --op append -c '新增功能说明'

    # 使用嵌套 heading 路径解决歧义 (用空格分隔多个 heading)
    mdp patch -f doc.md -H '# 父标题 ## 子标题' -i 0 --op append -c '内容'

    # 安全替换内容（带 fingerprint 验证）
    mdp patch -f doc.md -H '## API' -i 0 --op replace \\
        -c '新文档' -p '旧内容.*模式' --force

    # 删除内容块
    mdp patch -f doc.md -H '## 已弃用' -i 0 --op delete -p '待删除.*内容' --force

    # 批量操作
    mdp plan patches.yaml     # 预览更改
    mdp apply patches.yaml --force   # 应用更改

ADDRESSING MODEL:
    file → heading_path → block_index
    
    - Heading path: 空格分隔的 heading 路径，用于精确定位
      例如: '# Title ## Subtitle' 表示 Title 下的 Subtitle
    - Block index: heading 后第 N 个内容块，0-based
      0 = heading 后的第一个内容块（段落、代码块等）
")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Apply a single patch operation
    Patch {
        /// Target file path
        #[arg(short, long)]
        file: PathBuf,

        /// Heading path (e.g., "# Title" or "# Title ## Subtitle")
        #[arg(short = 'H', long)]
        heading: String,

        /// Block index within the heading section (0-based)
        #[arg(short, long, default_value = "0")]
        index: usize,

        /// Operation type
        #[arg(short, long, value_enum)]
        op: OperationType,

        /// Content to insert/replace (not needed for delete)
        #[arg(short, long)]
        content: Option<String>,

        /// Fingerprint regex for safety check
        #[arg(short = 'p', long)]
        fingerprint: Option<String>,

        /// Force execution of destructive operations
        #[arg(long)]
        force: bool,

        /// Skip creating backup files (.bak)
        #[arg(long)]
        no_backup: bool,

        /// Output format
        #[arg(short = 'F', long, value_enum, default_value = "diff")]
        format: OutputFormat,
    },

    /// Apply patches from YAML configuration file
    Apply {
        /// Configuration file path
        config: PathBuf,

        /// Force execution of destructive operations
        #[arg(long)]
        force: bool,

        /// Skip creating backup files (.bak)
        #[arg(long)]
        no_backup: bool,

        /// Output format
        #[arg(short = 'F', long, value_enum, default_value = "diff")]
        format: OutputFormat,
    },

    /// Preview changes without applying (dry-run)
    Plan {
        /// Configuration file path
        config: PathBuf,

        /// Output format
        #[arg(short = 'F', long, value_enum, default_value = "diff")]
        format: OutputFormat,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum OperationType {
    /// Append content after the target block
    Append,
    /// Replace the target block content
    Replace,
    /// Delete the target block
    Delete,
}

impl From<OperationType> for patch::Operation {
    fn from(op: OperationType) -> Self {
        match op {
            OperationType::Append => patch::Operation::Append,
            OperationType::Replace => patch::Operation::Replace,
            OperationType::Delete => patch::Operation::Delete,
        }
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        let exit_code = classify_error(&e.to_string());
        std::process::exit(exit_code);
    }
}

/// 根据错误信息分类返回退出码
fn classify_error(error_msg: &str) -> i32 {
    if error_msg.contains("Fingerprint mismatch") {
        3
    } else if error_msg.contains("Multiple sections found") || error_msg.contains("Ambiguous") {
        4
    } else if error_msg.contains("Heading not found") || error_msg.contains("Subheading not found") {
        2
    } else if error_msg.contains("file") || error_msg.contains("path") || error_msg.contains("not found") {
        1
    } else {
        1
    }
}

/// 原子写入文件：先备份（可选），再写临时文件，最后重命名
fn atomic_write(file: &PathBuf, content: &str, no_backup: bool) -> Result<()> {
    // 如果文件存在且不是禁止备份，先创建备份
    if !no_backup && file.exists() {
        let backup_path = file.with_extension("bak");
        std::fs::copy(file, &backup_path)
            .with_context(|| format!("Failed to create backup: {}", backup_path.display()))?;
    }

    let temp_file = file.with_extension("md.tmp");
    std::fs::write(&temp_file, content)?;
    std::fs::rename(&temp_file, file)?;
    Ok(())
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Patch {
            file,
            heading,
            index,
            op,
            content,
            fingerprint,
            force,
            no_backup,
            format,
        } => {
            // Validate content requirement
            let content = match op {
                OperationType::Delete => None,
                _ => match content {
                    Some(c) => Some(c),
                    None => bail!("Content is required for append/replace operations"),
                },
            };

            let operation = PatchOperation {
                file: file.clone(),
                heading_path: parse_heading_path(&heading)?,
                block_index: index,
                operation: op.into(),
                content,
                fingerprint,
            };

            let content_str = std::fs::read_to_string(&file)?;
            let result = patch::apply_operation(&content_str, &operation, force)?;

            let op_info = OperationInfo {
                file: file.clone(),
                heading: heading.clone(),
                index,
                operation: format!("{:?}", op).to_lowercase(),
            };

            match result {
                PatchResult::Applied { new_content, diff, is_noop } => {
                    atomic_write(&file, &new_content, no_backup)?;
                    output::print_result_with_info(&diff, format, true, Some(op_info), is_noop);
                }
                PatchResult::DryRun { diff, is_noop } => {
                    output::print_result_with_info(&diff, format, false, Some(op_info), is_noop);
                    if !force {
                        println!("\n(Run with --force to apply changes)");
                    }
                }
            }
        }

        Commands::Apply {
            config,
            force,
            no_backup,
            format,
        } => {
            let operations = load_config(&config)?;
            apply_batch(operations, force, format, no_backup)?;
        }

        Commands::Plan { config, format } => {
            let operations = load_config(&config)?;
            apply_batch(operations, false, format, true)?;
        }
    }

    Ok(())
}

fn parse_heading_path(path: &str) -> Result<Vec<String>> {
    // Parse heading path like "# Title ## Subtitle" into ["# Title", "## Subtitle"]
    // Split by heading markers and reconstruct
    let mut headings = Vec::new();
    let mut current = String::new();
    let mut in_heading = false;
    
    for word in path.split_whitespace() {
        if word.starts_with("#") && !word.chars().skip(1).any(|c| c != '#') {
            // Save previous heading if exists
            if !current.is_empty() {
                headings.push(current.trim().to_string());
            }
            // Start new heading
            current = word.to_string();
            in_heading = true;
        } else if in_heading {
            current.push(' ');
            current.push_str(word);
        }
    }
    
    // Don't forget the last heading
    if !current.is_empty() {
        headings.push(current.trim().to_string());
    }
    
    if headings.is_empty() {
        bail!("Invalid heading path format. Expected: '# Title ## Subtitle ...'");
    }

    Ok(headings)
}

fn apply_batch(operations: Vec<OperationConfig>, force: bool, format: OutputFormat, no_backup: bool) -> Result<()> {
    let mut all_diffs = Vec::new();
    let mut all_results = Vec::new();

    // First pass: validate all operations
    for op_config in &operations {
        let content = match std::fs::read_to_string(&op_config.file) {
            Ok(c) => c,
            Err(e) => {
                bail!("Failed to read {}: {}", op_config.file.display(), e);
            }
        };

        let operation = PatchOperation {
            file: op_config.file.clone(),
            heading_path: op_config.heading.clone(),
            block_index: op_config.index,
            operation: op_config.operation.into(),
            content: op_config.content.clone(),
            fingerprint: op_config.fingerprint.clone(),
        };

        match patch::apply_operation(&content, &operation, force) {
            Ok(result) => {
                all_results.push((op_config.file.clone(), result));
            }
            Err(e) => {
                bail!(
                    "Operation failed for {} (heading: {:?}): {}",
                    op_config.file.display(),
                    op_config.heading,
                    e
                );
            }
        }
    }

    // If all validations pass and force is enabled, apply all changes atomically
    if force {
        for (file, result) in &all_results {
            if let PatchResult::Applied { new_content, .. } = result {
                atomic_write(file, new_content, no_backup)?;
            }
        }
    }

    // Output results
    for (file, result) in &all_results {
        match result {
            PatchResult::Applied { diff, .. } | PatchResult::DryRun { diff, .. } => {
                all_diffs.push(format!("--- {} ---\n{}", file.display(), diff));
            }
        }
    }

    let combined_diff = all_diffs.join("\n");
    // Batch 操作暂简单处理，不传递 is_noop
    output::print_result(&combined_diff, format, force, false);

    if !force {
        println!("\n(Run with --force to apply changes)");
    }

    Ok(())
}
