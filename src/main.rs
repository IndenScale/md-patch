use anyhow::{bail, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

mod config;
mod output;
mod parser;
mod patch;

use config::{load_config, OperationConfig};
use output::OutputFormat;
use patch::{PatchOperation, PatchResult};

/// CLI tool for declarative, idempotent Markdown block patching
#[derive(Parser)]
#[command(name = "mdp")]
#[command(about = "Declarative, idempotent Markdown block patching tool")]
#[command(version)]
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
        #[arg(short, long)]
        fingerprint: Option<String>,

        /// Force execution of destructive operations
        #[arg(short, long)]
        force: bool,

        /// Output format
        #[arg(short, long, value_enum, default_value = "diff")]
        format: OutputFormat,
    },

    /// Apply patches from YAML configuration file
    Apply {
        /// Configuration file path
        config: PathBuf,

        /// Force execution of destructive operations
        #[arg(short, long)]
        force: bool,

        /// Output format
        #[arg(short, long, value_enum, default_value = "diff")]
        format: OutputFormat,
    },

    /// Preview changes without applying (dry-run)
    Plan {
        /// Configuration file path
        config: PathBuf,

        /// Output format
        #[arg(short, long, value_enum, default_value = "diff")]
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
        std::process::exit(1);
    }
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

            match result {
                PatchResult::Applied { new_content, diff } => {
                    if force {
                        std::fs::write(&file, new_content)?;
                    }
                    output::print_result(&diff, format, force);
                }
                PatchResult::DryRun { diff } => {
                    output::print_result(&diff, format, false);
                    if !force {
                        println!("\n(Run with --force to apply changes)");
                    }
                }
            }
        }

        Commands::Apply {
            config,
            force,
            format,
        } => {
            let operations = load_config(&config)?;
            apply_batch(operations, force, format)?;
        }

        Commands::Plan { config, format } => {
            let operations = load_config(&config)?;
            apply_batch(operations, false, format)?;
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

fn apply_batch(operations: Vec<OperationConfig>, force: bool, format: OutputFormat) -> Result<()> {
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
                // Write to temp file first (atomic write)
                let temp_file = file.with_extension("md.tmp");
                std::fs::write(&temp_file, new_content)?;
                std::fs::rename(&temp_file, file)?;
            }
        }
    }

    // Output results
    for (file, result) in &all_results {
        match result {
            PatchResult::Applied { diff, .. } | PatchResult::DryRun { diff } => {
                all_diffs.push(format!("--- {} ---\n{}", file.display(), diff));
            }
        }
    }

    let combined_diff = all_diffs.join("\n");
    output::print_result(&combined_diff, format, force);

    if !force {
        println!("\n(Run with --force to apply changes)");
    }

    Ok(())
}
